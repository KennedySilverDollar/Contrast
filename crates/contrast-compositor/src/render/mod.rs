//! 1 フレームの描画: 背景色で塗り、Space 上の窓を重ねる。

use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::WinitGraphicsBackend,
    },
    output::Output,
    utils::Rectangle,
};

use crate::state::Contrast;

/// 出力 1 枚分を描いて提出し、クライアントに frame コールバックを送る。
pub fn render_frame(
    state: &mut Contrast,
    backend: &mut WinitGraphicsBackend<GlesRenderer>,
    output: &Output,
    damage_tracker: &mut OutputDamageTracker,
) {
    let size = backend.window_size();
    let damage = Rectangle::from_size(size);
    let background = state.policy.background();

    {
        let (renderer, mut framebuffer) = backend.bind().expect("failed to bind backend");
        smithay::desktop::space::render_output::<
            _,
            WaylandSurfaceRenderElement<GlesRenderer>,
            _,
            _,
        >(
            output,
            renderer,
            &mut framebuffer,
            1.0,
            0,
            [&state.space],
            &[],
            damage_tracker,
            background,
        )
        .expect("render_output failed");
    }
    backend.submit(Some(&[damage])).expect("submit failed");

    state.space.elements().for_each(|window| {
        window.send_frame(
            output,
            state.start_time.elapsed(),
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        )
    });

    state.space.refresh();
    state.popups.cleanup();
    let _ = state.display_handle.flush_clients();
}
