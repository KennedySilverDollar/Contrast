//! コンポジタ全体の状態と、Wayland ソケットの用意。

use std::{ffi::OsString, rc::Rc, sync::Arc};

use smithay::{
    backend::{
        allocator::{Format, dmabuf::Dmabuf},
        renderer::element::solid::SolidColorBuffer,
    },
    desktop::{PopupManager, Space, Window},
    input::{Seat, SeatState, keyboard::Keycode},
    reexports::{
        calloop::{EventLoop, Interest, LoopSignal, Mode, PostAction, generic::Generic},
        wayland_server::{
            Display, DisplayHandle,
            backend::{ClientData, ClientId, DisconnectReason},
        },
    },
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        dmabuf::{DmabufFeedback, DmabufGlobal, DmabufState},
        output::OutputManagerState,
        selection::{data_device::DataDeviceState, primary_selection::PrimarySelectionState},
        shell::{
            wlr_layer::WlrLayerShellState,
            xdg::{XdgShellState, decoration::XdgDecorationState},
        },
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use contrast_policy::{ConfigLoader, ConfigStore, Policy, PortPlacement};

use crate::ConfigSource;

use crate::shell::{ContrastCore, Shell};

/// dmabuf をレンダラーにインポートできるか試す関数（できれば `true`）。
pub type DmabufImporter = Box<dyn FnMut(&Dmabuf) -> bool>;

/// コンポジタの状態。calloop のイベントループが `&mut` で持ち回る。
pub struct Contrast {
    /// 起動時刻（frame コールバックの時刻に使う）。
    pub start_time: std::time::Instant,
    /// クライアント向けに公開したソケット名。
    pub socket_name: OsString,
    /// Display のハンドル。
    pub display_handle: DisplayHandle,
    /// 窓と出力を置く 2 次元平面。
    pub space: Space<Window>,
    /// イベントループを止めるための信号。
    pub loop_signal: LoopSignal,
    /// 配置・キーバインドの問い合わせ先。
    /// core の配置判断（`PortPlacement`）とも共有する。
    pub policy: Rc<dyn Policy>,
    /// 現在の設定（`policy` の実体）。リロードで差し替える。
    pub store: Rc<ConfigStore>,
    /// リロードで設定を読み直す読み込み器。
    pub config_loader: Box<dyn ConfigLoader>,
    /// 設定ファイルのパス。
    pub config_path: Option<std::path::PathBuf>,
    /// 配置・フォーカスを決める状態遷移エンジン。
    pub core: ContrastCore,
    /// core の ExternalKey とシェル側の窓・出力の対応。
    pub shell: Shell,
    /// 枠の帯のバッファ（帯 1 本に 1 つ。フォーカス窓の 4 本、続けてフォーカス外の窓の分）。Id を保って描画の差分追跡を効かせる。
    pub border_buffers: Vec<SolidColorBuffer>,
    /// キーバインドとして握りつぶした押下中のキー（離したときも素通ししない）。
    pub suppressed_keys: Vec<Keycode>,

    pub xdg_shell_state: XdgShellState,
    pub xdg_decoration_state: XdgDecorationState,
    pub compositor_state: CompositorState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Contrast>,
    pub data_device_state: DataDeviceState,
    pub primary_selection_state: PrimarySelectionState,
    pub layer_shell_state: WlrLayerShellState,
    /// キーボードフォーカスを持っているレイヤー面（無ければ core のフォーカス窓）。
    pub layer_focus: Option<smithay::reexports::wayland_server::protocol::wl_surface::WlSurface>,
    pub popups: PopupManager,
    pub seat: Seat<Self>,
    /// linux-dmabuf の状態。グローバルはレンダラーができてから `enable_dmabuf` で作る。
    pub dmabuf_state: DmabufState,
    /// 公開した linux-dmabuf のグローバル（`enable_dmabuf` の後）。
    pub dmabuf_global: Option<DmabufGlobal>,
    /// クライアントの dmabuf をレンダラーにインポートできるか試す関数（バックエンドが入れる）。
    pub dmabuf_importer: Option<DmabufImporter>,
    /// VT を切り替える関数（DRM バックエンドだけが入れる）。キーボードをコンポジタが持つので、これが無いと VT を移れない。
    pub vt_switch: Option<Box<dyn FnMut(i32)>>,
    /// クライアントが指定した（または既定の）カーソル。DRM バックエンドが描く。
    pub cursor_status: smithay::input::pointer::CursorImageStatus,
    /// 起動時に決まった窓の並べ方。リロードでは変わらない（core を作り直さないため）。
    pub layout_mode: contrast_core::LayoutMode,
    /// libinput のデバイス（設定のリロードで再適用するために覚える。DRM バックエンドだけが入れる）。
    pub input_devices: Vec<smithay::reexports::input::Device>,
    /// キー処理の中で見つかった VT 切り替えの要求（キー処理が終わってから実行する）。
    pub pending_vt: Option<i32>,
    /// 窓の矩形の補間（`Layout.animationMs`）。
    pub anim: crate::anim::Animator,
    /// 最後に入力（キー・ポインタ）があった時刻。`Cursor.hideAfterInactiveMs` でカーソルを隠すのに使う。
    pub last_input: std::time::Instant,
    /// winit のホストの窓でカーソルを隠している状態（変わったときだけホストに伝える）。
    pub host_cursor_hidden: bool,
}

impl Contrast {
    /// 各 Wayland グローバルを作り、ソケットをイベントループに登録する。
    pub fn new(
        event_loop: &mut EventLoop<Self>,
        display: Display<Self>,
        store: Rc<ConfigStore>,
        source: ConfigSource,
    ) -> Self {
        let policy: Rc<dyn Policy> = store.clone();
        let dh = display.handle();

        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let xdg_decoration_state = XdgDecorationState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&dh);
        let primary_selection_state = PrimarySelectionState::new::<Self>(&dh);
        let layer_shell_state = WlrLayerShellState::new::<Self>(&dh);
        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "contrast");
        add_keyboard(&mut seat);
        seat.add_pointer();

        let socket_name = Self::init_wayland_listener(display, event_loop);
        let layout = policy.layout_settings();
        let mut core_config = contrast_core::CoreConfig::new(policy.workspace_count())
            .with_layout(layout.mode)
            .with_preset_column_widths(layout.preset_column_widths.clone())
            .with_preset_window_heights(layout.window_height_presets.clone());
        if let Some(width) = layout.default_column_width {
            core_config = core_config.with_default_column_width(width);
        }
        let layout_mode = layout.mode;
        let core = ContrastCore::new(
            core_config,
            PortPlacement::new(Rc::clone(&policy)),
            contrast_core::Columns::new(),
        );

        Self {
            start_time: std::time::Instant::now(),
            socket_name,
            display_handle: dh,
            space: Space::default(),
            loop_signal: event_loop.get_signal(),
            policy,
            store,
            config_loader: source.loader,
            config_path: source.path,
            core,
            shell: Shell::default(),
            border_buffers: Default::default(),
            suppressed_keys: Vec::new(),
            xdg_shell_state,
            xdg_decoration_state,
            compositor_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            primary_selection_state,
            layer_shell_state,
            layer_focus: None,
            popups: PopupManager::default(),
            seat,
            dmabuf_state: DmabufState::new(),
            dmabuf_global: None,
            dmabuf_importer: None,
            vt_switch: None,
            pending_vt: None,
            anim: crate::anim::Animator::default(),
            last_input: std::time::Instant::now(),
            host_cursor_hidden: false,
            input_devices: Vec::new(),
            layout_mode,
            cursor_status: smithay::input::pointer::CursorImageStatus::default_named(),
        }
    }

    /// zwp_linux_dmabuf_v1 を `formats`（レンダラーが受け付けるもの）で公開する。`importer` は、バッファがレンダラーに
    /// インポートできるかを試す。できなければクライアントにエラーを返す。
    pub fn enable_dmabuf(
        &mut self,
        formats: impl IntoIterator<Item = Format>,
        feedback: Option<DmabufFeedback>,
        importer: DmabufImporter,
    ) {
        let formats: Vec<Format> = formats.into_iter().collect();
        tracing::info!(
            formats = formats.len(),
            feedback = feedback.is_some(),
            "dmabuf: enabled"
        );
        self.dmabuf_importer = Some(importer);
        // v4（フィードバックつき）は、クライアントの Mesa が描画デバイスを知るのに要る。無ければ v3。
        let global = match &feedback {
            Some(f) => self
                .dmabuf_state
                .create_global_with_default_feedback::<Self>(&self.display_handle, f),
            None => self
                .dmabuf_state
                .create_global::<Self>(&self.display_handle, formats),
        };
        self.dmabuf_global = Some(global);
    }

    /// 空いている `wayland-N` でソケットを開き、Display と一緒にイベントループへ入れる。
    fn init_wayland_listener(display: Display<Self>, event_loop: &mut EventLoop<Self>) -> OsString {
        let listening_socket = ListeningSocketSource::new_auto().expect("failed to open socket");
        let socket_name = listening_socket.socket_name().to_os_string();
        let handle = event_loop.handle();

        handle
            .insert_source(listening_socket, |stream, _, state| {
                let client = state
                    .display_handle
                    .insert_client(stream, Arc::new(ClientState::default()))
                    .expect("failed to insert client");
                tracing::info!(client = ?client.id(), "client connected");
            })
            .expect("failed to init the wayland socket source");

        handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    // SAFETY: Display はこのソースが持ち続け、ここで drop しない。
                    unsafe {
                        display
                            .get_mut()
                            .dispatch_clients(state)
                            .expect("dispatch_clients failed");
                    }
                    Ok(PostAction::Continue)
                },
            )
            .expect("failed to insert display source");

        socket_name
    }
}

/// キーボードを seat に足す。配列は `CONTRAST_XKB_LAYOUT` / `_VARIANT` / `_MODEL` / `_OPTIONS`、
/// 未指定は xkbcommon の既定。指定が不正でキーマップを作れなければ、警告して既定にする。
fn add_keyboard(seat: &mut Seat<Contrast>) {
    let var = |name: &str| std::env::var(name).unwrap_or_default();
    let (layout, variant, model) = (
        var("CONTRAST_XKB_LAYOUT"),
        var("CONTRAST_XKB_VARIANT"),
        var("CONTRAST_XKB_MODEL"),
    );
    let options = std::env::var("CONTRAST_XKB_OPTIONS").ok();
    let config = smithay::input::keyboard::XkbConfig {
        layout: &layout,
        variant: &variant,
        model: &model,
        options: options.clone(),
        ..Default::default()
    };
    if let Err(e) = seat.add_keyboard(config, 200, 25) {
        tracing::warn!(?e, %layout, %variant, %model, ?options, "xkb: invalid keymap settings, using the default");
        if let Err(e) = seat.add_keyboard(Default::default(), 200, 25) {
            tracing::error!(?e, "xkb: failed to add a keyboard");
        }
    }
}

/// クライアントごとのデータ。
#[derive(Default)]
pub struct ClientState {
    /// wl_compositor のクライアント側状態。
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason) {
        tracing::info!(client = ?client_id, ?reason, "client disconnected");
    }
}
