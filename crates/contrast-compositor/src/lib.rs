//! Contrast の Smithay シェル（nested プロトタイプ）。
//!
//! ホストの Wayland 上に winit の窓を開き、その中でクライアントを受け付ける。
//! 配置・キーバインド・背景色は `contrast_policy::Policy` に問い合わせる。

mod backend;
mod handler;
mod input;
mod render;
mod shell;
mod state;

use smithay::reexports::{calloop::EventLoop, wayland_server::Display};

use contrast_policy::Policy;
pub use state::Contrast;

/// コンポジタを起動し、終了まで戻らない。
pub fn run(policy: Box<dyn Policy>) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop: EventLoop<Contrast> = EventLoop::try_new()?;
    let display: Display<Contrast> = Display::new()?;
    let mut state = Contrast::new(&mut event_loop, display, policy);

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
