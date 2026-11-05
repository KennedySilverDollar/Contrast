//! winit バックエンド: ホストの窓を 1 つの出力として扱う。

use smithay::{
    backend::{
        renderer::{damage::OutputDamageTracker, gles::GlesRenderer},
        winit::{self, WinitEvent, WinitGraphicsBackend},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::EventLoop,
    utils::{Physical, Rectangle, Size, Transform},
};

use contrast_core::Rect;

use crate::render::{RenderError, finish_frame, render_scene};
use crate::state::Contrast;

/// 出力のリフレッシュレート（mHz）。winit では目安でしかない。
const REFRESH: i32 = 60_000;

/// winit の窓を開き、出力を作り、描画とイベント処理をイベントループに登録する。
pub fn init_winit(
    event_loop: &mut EventLoop<Contrast>,
    state: &mut Contrast,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut backend, winit): (WinitGraphicsBackend<GlesRenderer>, _) = winit::init()?;

    let mode = Mode {
        size: backend.window_size(),
        refresh: REFRESH,
    };
    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Contrast".into(),
            model: "Nested".into(),
        },
    );
    let _global = output.create_global::<Contrast>(&state.display_handle);
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);
    state.space.map_output(&output, (0, 0));
    let rect = output_rect(mode.size).ok_or("winit window size is not positive")?;
    state.output_added("winit", rect);
    tracing::info!(size = ?mode.size, "output created");

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    event_loop
        .handle()
        .insert_source(winit, move |event, _, state| match event {
            WinitEvent::Resized { size, .. } => {
                output.change_current_state(
                    Some(Mode {
                        size,
                        refresh: REFRESH,
                    }),
                    None,
                    None,
                    None,
                );
                tracing::info!(?size, "output resized");
                {
                    let mut map = smithay::desktop::layer_map_for_output(&output);
                    let changed = map.arrange();
                    tracing::info!(changed, zone = ?map.non_exclusive_zone(), "layers re-arranged");
                }
                match output_rect(size) {
                    Some(rect) => state.output_changed(rect),
                    None => tracing::warn!(?size, "output size is not positive, layout skipped"),
                }
            }
            WinitEvent::Input(event) => state.process_input_event(event),
            WinitEvent::Redraw => {
                if let Err(e) = render_winit(state, &mut backend, &output, &mut damage_tracker) {
                    tracing::error!("render: {e}, stopping");
                    state.loop_signal.stop();
                    return;
                }
                backend.window().request_redraw();
            }
            WinitEvent::CloseRequested => state.loop_signal.stop(),
            _ => {}
        })?;

    Ok(())
}

/// winit の窓に 1 フレーム描いて提出し、後始末（frame コールバックなど）をする。
fn render_winit(
    state: &mut Contrast,
    backend: &mut WinitGraphicsBackend<GlesRenderer>,
    output: &Output,
    damage_tracker: &mut OutputDamageTracker,
) -> Result<(), RenderError> {
    let damage = Rectangle::from_size(backend.window_size());
    {
        let (renderer, mut framebuffer) = backend
            .bind()
            .map_err(|e| RenderError::Bind(e.to_string()))?;
        render_scene(state, renderer, &mut framebuffer, output, damage_tracker)?;
    }
    backend
        .submit(Some(&[damage]))
        .map_err(|e| RenderError::Submit(e.to_string()))?;
    finish_frame(state, output);
    Ok(())
}

/// 出力の論理矩形（原点 (0, 0)、拡大率 1）。大きさが正でなければ `None`。
fn output_rect(size: Size<i32, Physical>) -> Option<Rect> {
    Rect::new(0, 0, size.w, size.h)
}
