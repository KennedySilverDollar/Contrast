//! DRM バックエンド（フェーズ1）: libseat の session と udev で DRM の出力を列挙してログに出す。
//!
//! モード設定（modeset）も描画も KD_GRAPHICS への切り替えもしない。デバイスは session 経由で開くが、
//! DRM master も取らず（`DrmDeviceFd` / `DrmDevice` を使わない）、connector と CRTC を読むだけで閉じる。
//! 列挙が終わったらすぐ正常終了する。

use std::fmt;
use std::io;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::time::Duration;

use smithay::backend::{
    session::{Event as SessionEvent, Session, libseat::LibSeatSession},
    udev::{UdevBackend, primary_gpu},
};
use smithay::reexports::{
    calloop::EventLoop,
    drm::{
        self,
        control::{self, Device as ControlDevice, connector::State},
    },
    rustix::fs::OFlags,
};

/// DRM バックエンドの起動・列挙の失敗。
#[derive(Debug)]
pub enum DrmError {
    /// libseat の session を開けなかった（seat が使えない、logind / seatd に繋がらない、など）。
    Session(String),
    /// udev で seat のデバイスを列挙できなかった。
    Udev(String),
    /// イベントループの作成・登録に失敗した。
    EventLoop(String),
    /// seat に DRM デバイスはあったが、どれも開いて読めなかった（session が active でない、など）。
    NoReadableDevice {
        /// session が active だったか。
        active: bool,
    },
}

impl fmt::Display for DrmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DrmError::Session(e) => write!(
                f,
                "DRM バックエンド: seat を開けません（{e}）。\
                 グラフィカルなセッションの外の TTY（Ctrl+Alt+F3 など）で、ログインしたユーザーとして実行してください。\
                 別のコンポジタが動いている VT や ssh からは seat を取れません"
            ),
            DrmError::Udev(e) => write!(
                f,
                "DRM バックエンド: udev でデバイスを列挙できません（{e}）"
            ),
            DrmError::NoReadableDevice { active } => write!(
                f,
                "DRM バックエンド: DRM デバイスを開けませんでした（session active={active}）。\
                 このセッションが seat の有効なセッションではない可能性があります。\
                 グラフィカルなセッションの外の TTY（Ctrl+Alt+F3 など）にログインして実行してください"
            ),
            DrmError::EventLoop(e) => {
                write!(
                    f,
                    "DRM バックエンド: イベントループの初期化に失敗しました（{e}）"
                )
            }
        }
    }
}

impl std::error::Error for DrmError {}

/// session 経由で開いた DRM デバイス。master は取らず、読み取り専用の ioctl だけに使う。
struct Card(OwnedFd);

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl drm::Device for Card {}
impl ControlDevice for Card {}

/// DRM の出力を列挙してログに出し、終わったら戻る。
pub fn enumerate() -> Result<(), DrmError> {
    let (mut session, notifier) =
        LibSeatSession::new().map_err(|e| DrmError::Session(e.to_string()))?;
    let seat = session.seat();
    tracing::info!(seat = %seat, active = session.is_active(), "drm: session opened");

    let mut event_loop: EventLoop<()> =
        EventLoop::try_new().map_err(|e| DrmError::EventLoop(e.to_string()))?;
    event_loop
        .handle()
        .insert_source(notifier, |event, _, _| match event {
            SessionEvent::ActivateSession => tracing::info!("drm: session active"),
            SessionEvent::PauseSession => tracing::info!("drm: session inactive"),
        })
        .map_err(|e| DrmError::EventLoop(e.to_string()))?;

    let primary = primary_gpu(&seat).map_err(|e| DrmError::Udev(e.to_string()))?;
    let udev = UdevBackend::new(&seat).map_err(|e| DrmError::Udev(e.to_string()))?;
    let mut paths: Vec<PathBuf> = udev.device_list().map(|(_, p)| p.to_path_buf()).collect();
    paths.sort();
    if paths.is_empty() {
        tracing::warn!(seat = %seat, "drm: no DRM device on this seat");
    }

    let mut readable = 0usize;
    for path in &paths {
        let is_primary = primary.as_deref() == Some(path.as_path());
        tracing::info!("drm: device={} primary={is_primary}", path.display());
        match list_device(&mut session, path) {
            Ok(()) => readable = readable.saturating_add(1),
            Err(e) => tracing::warn!("drm: device={} skipped: {e}", path.display()),
        }
    }

    // 開いた直後に届いている session イベントがあればログに出す。
    if let Err(e) = event_loop.dispatch(Duration::ZERO, &mut ()) {
        tracing::warn!("drm: session event dispatch failed: {e}");
    }
    if !paths.is_empty() && readable == 0 {
        return Err(DrmError::NoReadableDevice {
            active: session.is_active(),
        });
    }
    Ok(())
}

/// 1 つの DRM デバイスを session 経由で開き、connector と CRTC をログに出して閉じる。
fn list_device(session: &mut LibSeatSession, path: &Path) -> Result<(), String> {
    let flags = OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK;
    let fd = session.open(path, flags).map_err(|e| e.to_string())?;
    let card = Card(fd);
    let result = log_resources(&card).map_err(|e| e.to_string());
    if let Err(e) = session.close(card.0) {
        tracing::warn!("drm: close {} failed: {e}", path.display());
    }
    result
}

/// connector（名前・状態・モード・物理サイズ）と CRTC をログに出す。
fn log_resources(card: &Card) -> io::Result<()> {
    let res = card.resource_handles()?;
    for &handle in res.connectors() {
        let info = card.get_connector(handle, false)?;
        let name = format!("{}-{}", info.interface().as_str(), info.interface_id());
        let status = match info.state() {
            State::Connected => "connected",
            State::Disconnected => "disconnected",
            State::Unknown => "unknown",
        };
        let preferred = info
            .modes()
            .iter()
            .find(|m| m.mode_type().contains(control::ModeTypeFlags::PREFERRED))
            .map_or_else(|| "none".to_string(), format_mode);
        let size = info
            .size()
            .map_or_else(|| "unknown".to_string(), |(w, h)| format!("{w}x{h}"));
        tracing::info!(
            "drm: connector={name} status={status} modes={} preferred={preferred} size_mm={size}",
            info.modes().len()
        );
        for mode in info.modes() {
            tracing::info!("drm: mode connector={name} {}", format_mode(mode));
        }
    }
    for &crtc in res.crtcs() {
        tracing::info!("drm: crtc={}", u32::from(crtc));
    }
    Ok(())
}

/// モードを `WxH@Hz` の形にする。
fn format_mode(mode: &control::Mode) -> String {
    let (w, h) = mode.size();
    format!("{w}x{h}@{}", mode.vrefresh())
}
