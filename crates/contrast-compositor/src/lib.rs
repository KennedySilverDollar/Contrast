//! Contrast の Smithay シェル（nested プロトタイプ）。
//!
//! ホストの Wayland 上に winit の窓を開き、その中でクライアントを受け付ける。
//! 配置・キーバインド・背景色は `contrast_policy::Policy` に問い合わせる。

mod backend;
mod handler;
mod input;
mod reload;
mod render;
mod shell;
mod state;

use std::path::PathBuf;
use std::rc::Rc;

use calloop::signals::{Signal, Signals};
use smithay::reexports::{calloop::EventLoop, wayland_server::Display};

use contrast_policy::{ConfigLoader, ConfigStore};
pub use state::Contrast;

/// 設定の再読み込みに使う読み込み器と設定ファイルのパス。
pub struct ConfigSource {
    /// 設定ファイルを読む読み込み器。
    pub loader: Box<dyn ConfigLoader>,
    /// 設定ファイルのパス。決められなかったときは `None`（リロードは何もしない）。
    pub path: Option<PathBuf>,
}

/// コンポジタを起動し、終了まで戻らない。`store` が配置・キーバインド・見た目の問い合わせ先。
///
/// SIGUSR1 と `reload-config` アクションで、`source` から設定を読み直して `store` を差し替える。
///
/// バックエンドは環境変数 `CONTRAST_BACKEND=winit|drm` で選ぶ。未指定で `WAYLAND_DISPLAY` か `DISPLAY` が
/// あれば winit、無ければエラー。`drm` のときは出力を列挙してログに出し、すぐ正常終了する（フェーズ1）。
pub fn run(store: Rc<ConfigStore>, source: ConfigSource) -> Result<(), Box<dyn std::error::Error>> {
    let choice = std::env::var("CONTRAST_BACKEND").ok();
    let has_host_display = ["WAYLAND_DISPLAY", "DISPLAY"]
        .iter()
        .any(|k| std::env::var_os(k).is_some_and(|v| !v.is_empty()));
    if backend::select(choice.as_deref(), has_host_display)? == backend::Kind::Drm {
        backend::drm::enumerate()?;
        return Ok(());
    }

    let mut event_loop: EventLoop<Contrast> = EventLoop::try_new()?;
    // SIGUSR1 はスレッドを作る前にブロックして calloop で受ける（既定の動作は終了）。
    let signals = Signals::new(&[Signal::SIGUSR1])?;
    event_loop
        .handle()
        .insert_source(signals, |_, _, state| state.reload_config())?;
    let display: Display<Contrast> = Display::new()?;
    let mut state = Contrast::new(&mut event_loop, display, store, source);

    backend::winit::init_winit(&mut event_loop, &mut state)?;
    input::debug::init(&mut event_loop);

    tracing::info!(
        socket = ?state.socket_name,
        "listening: WAYLAND_DISPLAY={}",
        state.socket_name.to_string_lossy()
    );

    event_loop.run(None, &mut state, |_| {})?;
    tracing::info!("event loop stopped, exiting");
    Ok(())
}
