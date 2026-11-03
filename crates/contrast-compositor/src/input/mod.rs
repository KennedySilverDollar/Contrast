//! 入力: winit からのキー・ポインタイベントを処理し、キーバインドを意味操作に変える。

pub mod action;
pub mod debug;

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    input::{
        keyboard::{FilterResult, ModifiersState, xkb},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, SERIAL_COUNTER},
    desktop::WindowSurfaceType,
};

use contrast_policy::{Action, Mods};

use crate::state::Contrast;

/// smithay の修飾キー状態をポリシーの `Mods` に変える。
fn to_mods(m: &ModifiersState) -> Mods {
    Mods {
        alt: m.alt,
        shift: m.shift,
        ctrl: m.ctrl,
        logo: m.logo,
    }
}

impl Contrast {
    /// 座標の下にある surface と、その surface の原点の位置。
    fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space.element_under(pos).and_then(|(window, location)| {
            window
                .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                .map(|(s, p)| (s, (p + location).to_f64()))
        })
    }

    /// 座標の下の窓にフォーカスを移す。窓の外（空き領域）なら何もしない。
    pub fn focus_at(&mut self, pos: Point<f64, Logical>) {
        let Some(window) = self.space.element_under(pos).map(|(w, _)| w.clone()) else {
            return;
        };
        let id = self
            .windows
            .iter()
            .find(|m| m.window == window)
            .map(|m| m.id);
        tracing::info!(
            "click focus window={} x={} y={}",
            crate::handler::fmt_id(id),
            pos.x,
            pos.y
        );
        self.focus_window(Some(&window));
    }

    /// winit からの入力イベントを 1 つ処理する。
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event, .. } => self.on_key(
                event.key_code(),
                event.state(),
                Event::time_msec(&event),
            ),
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.space.outputs().next().expect("one output").clone();
                let output_geo = self.space.output_geometry(&output).expect("mapped output");
                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
                let serial = SERIAL_COUNTER.next_serial();
                let pointer = self.seat.get_pointer().expect("seat has pointer");
                let under = self.surface_under(pos);
                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerButton { event, .. } => {
                let pointer = self.seat.get_pointer().expect("seat has pointer");
                let serial = SERIAL_COUNTER.next_serial();
                let state = event.state();
                if state == ButtonState::Pressed && !pointer.is_grabbed() {
                    self.focus_at(pointer.current_location());
                }
                pointer.button(
                    self,
                    &ButtonEvent {
                        button: event.button_code(),
                        state,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerAxis { event, .. } => {
                let source = event.source();
                let mut frame = AxisFrame::new(event.time_msec()).source(source);
                for axis in [Axis::Horizontal, Axis::Vertical] {
                    let discrete = event.amount_v120(axis);
                    let amount = event
                        .amount(axis)
                        .unwrap_or_else(|| discrete.unwrap_or(0.0) * 15.0 / 120.0);
                    if amount != 0.0 {
                        frame = frame.value(axis, amount);
                        if let Some(d) = discrete {
                            frame = frame.v120(axis, d as i32);
                        }
                    }
                    if source == AxisSource::Finger && event.amount(axis) == Some(0.0) {
                        frame = frame.stop(axis);
                    }
                }
                let pointer = self.seat.get_pointer().expect("seat has pointer");
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }
    }

    /// キー 1 回分。キーバインドに当たれば握りつぶして実行し、離したときも素通ししない。
    fn on_key(&mut self, keycode: xkb::Keycode, key_state: KeyState, time: u32) {
        let serial = SERIAL_COUNTER.next_serial();
        let keyboard = self.seat.get_keyboard().expect("seat has keyboard");
        let action = keyboard.input::<Option<Action>, _>(
            self,
            keycode,
            key_state,
            serial,
            time,
            |state, mods, handle| {
                if key_state == KeyState::Released {
                    if let Some(i) = state.suppressed_keys.iter().position(|k| *k == keycode) {
                        state.suppressed_keys.remove(i);
                        return FilterResult::Intercept(None);
                    }
                    return FilterResult::Forward;
                }
                let Some(sym) = handle.raw_latin_sym_or_raw_current_sym() else {
                    return FilterResult::Forward;
                };
                let name = xkb::keysym_get_name(sym);
                match state.policy.resolve_key(to_mods(mods), &name) {
                    Some(action) => {
                        tracing::info!(key = %name, ?action, "keybinding");
                        state.suppressed_keys.push(keycode);
                        FilterResult::Intercept(Some(action))
                    }
                    None => FilterResult::Forward,
                }
            },
        );
        if let Some(Some(action)) = action {
            self.run_action(action);
        }
    }
}
