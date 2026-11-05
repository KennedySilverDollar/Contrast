//! winit バックエンド: ホストの窓を 1 つの出力として扱う。

use smithay::{
    backend::{
        renderer::{damage::OutputDamageTracker, gles::GlesRenderer},
        winit::{self, WinitEvent, WinitGraphicsBackend},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::EventLoop,
    utils::{Physical, Size, Transform},
};

use contrast_core::Rect;

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
    state.output_added("winit", output_rect(mode.size));
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
                state.output_changed(output_rect(size));
            }
            WinitEvent::Input(event) => state.process_input_event(event),
            WinitEvent::Redraw => {
                crate::render::render_frame(state, &mut backend, &output, &mut damage_tracker);
                backend.window().request_redraw();
            }
            WinitEvent::CloseRequested => state.loop_signal.stop(),
            _ => {}
        })?;

    Ok(())
}

/// 出力の論理矩形（原点 (0, 0)、拡大率 1）。
fn output_rect(size: Size<i32, Physical>) -> Rect {
    Rect::new(0, 0, size.w, size.h).expect("output size is positive")
}
