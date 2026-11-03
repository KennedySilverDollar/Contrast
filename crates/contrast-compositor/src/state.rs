//! コンポジタ全体の状態と、Wayland ソケットの用意。

use std::{ffi::OsString, sync::Arc};

use smithay::{
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
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        shell::xdg::{XdgShellState, decoration::XdgDecorationState},
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use contrast_policy::Policy;

use crate::wm::Managed;

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
    pub policy: Box<dyn Policy>,
    /// 管理中の窓。並び順がそのまま配置の順。
    pub windows: Vec<Managed>,
    /// 次に払い出す窓の通し番号の 1 つ前。
    pub next_id: u64,
    /// 表示中のワークスペース（0 始まり）。
    pub active_ws: u8,
    /// フォーカス順の刻み。
    pub focus_counter: u64,
    /// キーバインドとして握りつぶした押下中のキー（離したときも素通ししない）。
    pub suppressed_keys: Vec<Keycode>,

    pub xdg_shell_state: XdgShellState,
    pub xdg_decoration_state: XdgDecorationState,
    pub compositor_state: CompositorState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Contrast>,
    pub data_device_state: DataDeviceState,
    pub popups: PopupManager,
    pub seat: Seat<Self>,
}

impl Contrast {
    /// 各 Wayland グローバルを作り、ソケットをイベントループに登録する。
    pub fn new(
        event_loop: &mut EventLoop<Self>,
        display: Display<Self>,
        policy: Box<dyn Policy>,
    ) -> Self {
        let dh = display.handle();

        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let xdg_decoration_state = XdgDecorationState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&dh);
        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "contrast");
        seat.add_keyboard(Default::default(), 200, 25)
            .expect("failed to add keyboard");
        seat.add_pointer();

        let socket_name = Self::init_wayland_listener(display, event_loop);

        Self {
            start_time: std::time::Instant::now(),
            socket_name,
            display_handle: dh,
            space: Space::default(),
            loop_signal: event_loop.get_signal(),
            policy,
            windows: Vec::new(),
            next_id: 0,
            active_ws: 0,
            focus_counter: 0,
            suppressed_keys: Vec::new(),
            xdg_shell_state,
            xdg_decoration_state,
            compositor_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            popups: PopupManager::default(),
            seat,
        }
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
