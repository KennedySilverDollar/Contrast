//! 1 フレームの描画: 背景色で塗り、Space 上の窓を重ね、フォーカス窓のタイルの縁に枠を描く。

use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker,
            element::{Kind, solid::SolidColorRenderElement},
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
    let borders = border_elements(state);

    {
        let (renderer, mut framebuffer) = backend.bind().expect("failed to bind backend");
        smithay::desktop::space::render_output::<_, SolidColorRenderElement, _, _>(
            output,
            renderer,
            &mut framebuffer,
            1.0,
            0,
            [&state.space],
            &borders,
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
    smithay::desktop::layer_map_for_output(output).cleanup();
    let _ = state.display_handle.flush_clients();
}

/// 帯の位置と大きさ `(x, y, w, h)`。
type Band = (i32, i32, i32, i32);

/// タイルの縁（窓とタイルの間の余白）を埋める 4 本の帯（上・下・左・右）。太さが足りなければ `None`。
fn edges(tile: contrast_core::Rect, b: i32) -> Option<[Band; 4]> {
    if b <= 0 || tile.width() <= b * 2 || tile.height() <= b * 2 {
        return None;
    }
    let (x, y, w, h) = (tile.x(), tile.y(), tile.width(), tile.height());
    Some([
        (x, y, w, b),
        (x, y + h - b, w, b),
        (x, y + b, b, h - 2 * b),
        (x + w - b, y + b, b, h - 2 * b),
    ])
}

/// フォーカス窓の枠（`focusColor`）と、設定があればフォーカス外の窓の枠（`borderColor`）の帯。
fn border_elements(state: &mut Contrast) -> Vec<SolidColorRenderElement> {
    let b = state.policy.border_width();
    let mut bands: Vec<(Band, [f32; 4])> = Vec::new();
    if let Some(tile) = state.focused_tile()
        && let Some(es) = edges(tile, b)
    {
        let color = state.policy.focus_border_color();
        bands.extend(es.into_iter().map(|e| (e, color)));
    }
    if let Some(color) = state.policy.unfocused_border_color() {
        for tile in state.unfocused_tiles() {
            if let Some(es) = edges(tile, b) {
                bands.extend(es.into_iter().map(|e| (e, color)));
            }
        }
    }
    // 帯 1 本につきバッファ 1 つ（Id を保って描画の差分追跡を効かせる）。足りなければ増やす。
    while state.border_buffers.len() < bands.len() {
        state.border_buffers.push(Default::default());
    }
    state
        .border_buffers
        .iter_mut()
        .zip(bands)
        .map(|(buffer, ((ex, ey, ew, eh), color))| {
            buffer.update((ew, eh), color);
            SolidColorRenderElement::from_buffer(buffer, (ex, ey), 1.0, 1.0, Kind::Unspecified)
        })
        .collect()
}
