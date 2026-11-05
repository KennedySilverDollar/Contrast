//! 1 フレームの描画: 背景色で塗り、Space 上の窓を重ね、フォーカス窓のタイルの縁に枠を描く。

use std::fmt;
use std::time::Duration;

use smithay::{
    backend::egl::EGLDevice,
    backend::renderer::{
        ImportDma,
        damage::OutputDamageTracker,
        element::{
            Kind, memory::MemoryRenderBufferRenderElement, render_elements,
            solid::SolidColorRenderElement, surface::WaylandSurfaceRenderElement,
        },
        gles::{GlesRenderer, GlesTarget},
    },
    desktop::space::SpaceRenderElements,
    output::Output,
    wayland::dmabuf::{DmabufFeedback, DmabufFeedbackBuilder},
};

use crate::state::Contrast;

/// linux-dmabuf の既定のフィードバック: 描画デバイス（`renderer` の EGL デバイスの render node）と、
/// レンダラーが受け付けるフォーマット。デバイスを特定できなければ `None`（v3 で公開する）。
pub fn dmabuf_feedback(renderer: &GlesRenderer) -> Option<DmabufFeedback> {
    let device = EGLDevice::device_for_display(renderer.egl_context().display()).ok()?;
    let node = device.try_get_render_node().ok().flatten()?;
    let formats = renderer.dmabuf_formats();
    DmabufFeedbackBuilder::new(node.dev_id(), formats.iter().copied())
        .build()
        .ok()
}

/// 描画の失敗。バックエンド（winit / DRM）が受けて、ログに出すか、フレームを諦めるかを決める。
#[derive(Debug)]
pub enum RenderError {
    /// 描画先（フレームバッファ）を束縛できなかった。
    Bind(String),
    /// 要素の描画に失敗した。
    Render(String),
    /// 描き終えたバッファの提出に失敗した。
    Submit(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::Bind(e) => write!(f, "bind failed: {e}"),
            RenderError::Render(e) => write!(f, "render failed: {e}"),
            RenderError::Submit(e) => write!(f, "submit failed: {e}"),
        }
    }
}

impl std::error::Error for RenderError {}

render_elements! {
    /// DRM の `DrmOutput::render_frame` に渡す要素。`render_output` と同じく、枠・カーソル（custom）が前、Space（窓とレイヤー面）が後ろ。
    pub SceneElement<=GlesRenderer>;
    /// 枠の帯と、簡易カーソル。
    Solid=SolidColorRenderElement,
    /// テーマのカーソル。
    Memory=MemoryRenderBufferRenderElement<GlesRenderer>,
    /// クライアントが指定したカーソルの面。
    Surface=WaylandSurfaceRenderElement<GlesRenderer>,
    /// Space の要素（窓・レイヤー面・popup）。
    Space=SpaceRenderElements<GlesRenderer, WaylandSurfaceRenderElement<GlesRenderer>>,
}

/// 出力 1 枚分の要素を前から順に集める。`cursor` があれば最前面。`render_scene` と同じ値・順序。
pub fn scene_elements(
    state: &mut Contrast,
    renderer: &mut GlesRenderer,
    output: &Output,
    cursor: Vec<SceneElement>,
) -> Result<Vec<SceneElement>, RenderError> {
    let borders = border_elements(state);
    let space =
        smithay::desktop::space::space_render_elements(renderer, [&state.space], output, 1.0)
            .map_err(|e| RenderError::Render(e.to_string()))?;
    let mut elements: Vec<SceneElement> =
        Vec::with_capacity(cursor.len() + borders.len() + space.len());
    elements.extend(cursor);
    elements.extend(borders.into_iter().map(SceneElement::Solid));
    elements.extend(space.into_iter().map(SceneElement::Space));
    Ok(elements)
}

/// 出力 1 枚分の絵（背景・窓・レイヤー面・枠）を `framebuffer` に描く。バックエンドには依存しない。
///
/// 描画先の用意（bind）と提出（submit / queue_frame）は呼び出し側のバックエンドが行う。
pub fn render_scene(
    state: &mut Contrast,
    renderer: &mut GlesRenderer,
    framebuffer: &mut GlesTarget<'_>,
    output: &Output,
    damage_tracker: &mut OutputDamageTracker,
) -> Result<(), RenderError> {
    let background = state.policy.background();
    let borders = border_elements(state);
    smithay::desktop::space::render_output::<_, SolidColorRenderElement, _, _>(
        output,
        renderer,
        framebuffer,
        1.0,
        0,
        [&state.space],
        &borders,
        damage_tracker,
        background,
    )
    .map(|_| ())
    .map_err(|e| RenderError::Render(e.to_string()))
}

/// 提出したあとの後始末: クライアントへ frame コールバックを送り、Space・popup・レイヤーを整理する。
pub fn finish_frame(state: &mut Contrast, output: &Output) {
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

/// フォーカス外の枠の太さ。未指定は `border`、指定は 0〜`border` に収める。
fn unfocused_width(border: i32, inactive: Option<i32>) -> i32 {
    inactive.map_or(border, |w| w.clamp(0, border.max(0)))
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
            // 窓の大きさは枠の太さ `b` で決まっている。細い枠は窓に接する側に描く。
            let thin = unfocused_width(b, state.policy.appearance().inactive_border_width);
            let ring = crate::geometry::window_rect(tile, b - thin);
            if let Some(es) = edges(ring, thin) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unfocused_width_defaults_to_the_border_and_is_clamped() {
        assert_eq!(unfocused_width(4, None), 4);
        assert_eq!(unfocused_width(4, Some(2)), 2);
        assert_eq!(unfocused_width(4, Some(9)), 4);
        assert_eq!(unfocused_width(4, Some(-1)), 0);
        assert_eq!(unfocused_width(0, Some(3)), 0);
    }
}
