//! contrast-core への薄い翻訳層。
//!
//! - Wayland のイベント（窓の map/unmap、出力の追加・サイズ変更、キーバインド、クリック）を `Command` にする。
//! - `Core::apply` が返す `Effect` を Smithay の操作として実行する。
//! - `ExternalKey` はここで払い出し、`ExternalKey` ⇔ Smithay の `Window` の対応を持つ。
//!
//! 配置・フォーカスの判断は core だけが行う。ここに残すのは見た目の都合だけ:
//! タイルからフォーカス枠の幅だけ内側に縮めて窓を置くこと（枠の描画は `render`）。

use std::collections::BTreeMap;

use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::SERIAL_COUNTER,
    wayland::{compositor::with_states, shell::xdg::XdgToplevelSurfaceData},
};

use contrast_core::{
    Columns, Command, Core, CoreError, Effect, EntityId, ExternalKey, IdKind, OutputName, Point,
    Rect, WindowId, WindowMeta, WindowState,
};

use contrast_policy::{Policy, PortPlacement};

use crate::handler::fmt_id;
use crate::state::Contrast;

/// コンポジタが使う Core の型。窓の配置は `contrast_policy::Policy` に問い合わせる。
pub type ContrastCore = Core<PortPlacement<dyn Policy>, Columns>;

/// core とシェル側の対応。
#[derive(Default)]
pub struct Shell {
    /// 次に払い出す ExternalKey の raw 値。
    next_key: u64,
    /// 窓の ExternalKey ⇔ Smithay の窓（core に登録済みのもの）。
    pub windows: BTreeMap<ExternalKey, Window>,
    /// 作られたが最初の commit をまだ受けていない窓（core には未登録）。
    pub pending: Vec<Window>,
    /// 出力の ExternalKey（winit の出力は 1 つだけ）。
    pub output_key: Option<ExternalKey>,
}

impl Shell {
    /// 新しい ExternalKey を払い出す。
    fn issue_key(&mut self) -> ExternalKey {
        self.next_key += 1;
        ExternalKey::new(self.next_key)
    }
}

/// ログ用の窓番号（WindowId の raw 値）。
fn raw(id: WindowId) -> u64 {
    id.raw()
}

/// Smithay の窓のトップレベル surface。
fn surface_of(window: &Window) -> WlSurface {
    window
        .toplevel()
        .expect("only wayland toplevels are managed")
        .wl_surface()
        .clone()
}

impl Contrast {
    // ---- 対応表の参照 ----

    /// surface に対応する窓の ExternalKey。
    pub fn key_of_surface(&self, surface: &WlSurface) -> Option<ExternalKey> {
        self.shell
            .windows
            .iter()
            .find(|(_, w)| &surface_of(w) == surface)
            .map(|(k, _)| *k)
    }

    /// surface に対応する Smithay の窓。
    pub fn window_of_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.key_of_surface(surface)
            .and_then(|k| self.shell.windows.get(&k).cloned())
    }

    /// surface に対応する窓のログ用番号。
    pub fn log_id_of_surface(&self, surface: &WlSurface) -> Option<u64> {
        self.key_of_surface(surface)
            .and_then(|k| self.core.view().window_id(k))
            .map(raw)
    }

    /// core の WindowId に対応する Smithay の窓。
    fn window_of_id(&self, id: WindowId) -> Option<Window> {
        self.core
            .view()
            .window_key(id)
            .and_then(|k| self.shell.windows.get(&k).cloned())
    }

    /// フォーカス枠を描くべきタイル（表示中のフォーカス窓のもの。全画面なら枠なし）。
    pub fn focused_tile(&self) -> Option<Rect> {
        let view = self.core.view();
        let w = view.window(view.focus()?)?;
        if !w.visible || w.state == WindowState::Fullscreen {
            return None;
        }
        w.rect.map(|tile| self.frame_of(tile, w.state))
    }

    /// フォーカス外の表示中の窓のタイル（全画面は除く）。フォーカス外の枠を描くのに使う。
    pub fn unfocused_tiles(&self) -> Vec<Rect> {
        let view = self.core.view();
        let focus = view.focus();
        view.visible_windows()
            .iter()
            .filter(|w| Some(w.id) != focus && w.state != WindowState::Fullscreen)
            .filter_map(|w| w.rect.map(|tile| self.frame_of(tile, w.state)))
            .collect()
    }

    /// core が窓を並べる領域（今は出力の論理矩形。排他領域はまだ core に渡していない）。出力がなければ `None`。
    fn work_area(&self) -> Option<Rect> {
        let output = self.space.outputs().next()?;
        let geo = self.space.output_geometry(output)?;
        Rect::new(geo.loc.x, geo.loc.y, geo.size.w, geo.size.h)
    }

    /// 表示中の窓に全画面のものがあるか。
    pub fn has_visible_fullscreen(&self) -> bool {
        self.core
            .view()
            .visible_windows()
            .iter()
            .any(|w| w.state == WindowState::Fullscreen)
    }

    /// タイルから、隙間（`gap`・`outerGap`）を引いたフレーム（枠線の外側の矩形）。浮動・全画面は隙間なし。
    pub fn frame_of(&self, tile: Rect, state: WindowState) -> Rect {
        let layout = self.policy.layout_settings();
        if state != WindowState::Tiled || (layout.gap <= 0 && layout.outer_gap <= 0) {
            return tile;
        }
        let bounds = if self.layout_mode == contrast_core::LayoutMode::Scrolling {
            // スクロール列では、タイルが画面の外まで続く。画面（作業領域）の端に接する辺に `outerGap` を引く。
            self.work_area()
        } else {
            let view = self.core.view();
            let tiles = view
                .visible_windows()
                .into_iter()
                .filter(|w| w.state == WindowState::Tiled)
                .filter_map(|w| w.rect);
            crate::geometry::bounds_of(tiles)
        };
        crate::geometry::frame_rect(tile, bounds.unwrap_or(tile), layout.gap, layout.outer_gap)
    }

    /// 表示中の窓の配置をポリシーの今の値（枠の太さなど）で取り直す。設定のリロードのあとに呼ぶ。
    pub fn reapply_layout(&mut self) {
        let placed: Vec<(WindowId, Rect, WindowState)> = self
            .core
            .view()
            .visible_windows()
            .iter()
            .filter_map(|w| w.rect.map(|r| (w.id, r, w.state)))
            .collect();
        for (id, rect, state) in placed {
            self.configure(id, rect, state);
        }
        self.sync_space();
    }

    /// 表示中で、タイル（枠を含む）が点 `(x, y)` を含む窓の ExternalKey。上に描かれている窓を優先する。
    pub fn visible_key_at(&self, x: i32, y: i32) -> Option<ExternalKey> {
        let p = Point::new(x, y);
        let view = self.core.view();
        let mut visible = view.visible_windows();
        visible.sort_by_key(|w| stacking(w.state));
        visible
            .iter()
            .rev()
            .find(|w| w.rect.is_some_and(|r| r.contains(p)))
            .and_then(|w| view.window_key(w.id))
    }

    // ---- Wayland のイベント → Command ----

    /// 出力ができた。
    pub fn output_added(&mut self, name: &str, geometry: Rect) {
        let key = self.shell.issue_key();
        self.shell.output_key = Some(key);
        self.submit(Command::OutputAdded {
            key,
            name: OutputName::new(name),
            geometry,
        });
    }

    /// 出力の大きさが変わった。
    pub fn output_changed(&mut self, geometry: Rect) {
        let Some(key) = self.shell.output_key else {
            return;
        };
        self.submit(Command::OutputChanged { key, geometry });
    }

    /// トップレベル窓ができた。
    ///
    /// core にはまだ送らず、最初の commit（`map_pending`）まで保留する。
    pub fn window_created(&mut self, window: Window) {
        self.shell.pending.push(window);
    }

    /// 保留中の窓の最初の commit。app_id と title をここで確定させて core に WindowMapped を送る。
    ///
    /// xdg-shell ではクライアントが app_id・title を設定してから最初の commit をするので、
    /// この時点で無ければ「無い」と確定する。保留中の窓でなければ何もしない。
    pub fn map_pending(&mut self, surface: &WlSurface) {
        let Some(index) = self
            .shell
            .pending
            .iter()
            .position(|w| &surface_of(w) == surface)
        else {
            return;
        };
        let window = self.shell.pending.remove(index);
        let meta = read_meta(&window);
        tracing::info!(app_id = ?meta.app_id, title = ?meta.title, is_dialog = meta.is_dialog, "window first commit");
        self.window_mapped(window, meta);
    }

    /// 窓を core に登録する。
    fn window_mapped(&mut self, window: Window, meta: WindowMeta) {
        let key = self.shell.issue_key();
        self.shell.windows.insert(key, window);
        if !self.submit(Command::WindowMapped { key, meta }) {
            // core に登録できなかった窓は管理しない（docs/questions.md Q9）。
            self.shell.windows.remove(&key);
        }
    }

    /// トップレベル窓が消えた（クライアントの終了を含む）。
    ///
    /// 最初の commit の前に閉じられた窓は保留から外すだけで、core には何も送らない。
    pub fn window_unmapped(&mut self, surface: &WlSurface) {
        if let Some(index) = self
            .shell
            .pending
            .iter()
            .position(|w| &surface_of(w) == surface)
        {
            self.shell.pending.remove(index);
            tracing::info!("window closed before map");
            return;
        }
        let Some(key) = self.key_of_surface(surface) else {
            return;
        };
        let id = self.core.view().window_id(key);
        tracing::info!("window removed window={}", fmt_id(id.map(raw)));
        if let Some(window) = self.shell.windows.remove(&key) {
            self.space.unmap_elem(&window);
        }
        self.submit(Command::WindowUnmapped(key));
    }

    /// 座標の下の窓へのフォーカス要求（クリック）。窓の外なら何もしない。
    pub fn focus_at(&mut self, x: f64, y: f64) {
        let Some(key) = self.visible_key_at(x.floor() as i32, y.floor() as i32) else {
            return;
        };
        let id = self.core.view().window_id(key);
        tracing::info!("click focus window={} x={} y={}", fmt_id(id.map(raw)), x, y);
        self.submit(Command::WindowFocusRequested(key));
    }

    /// core の Action を適用する。
    pub fn core_action(&mut self, action: contrast_core::Action) {
        self.submit(Command::Action(action));
    }

    /// Command を core に渡し、返った Effect を実行する。成功なら true。
    fn submit(&mut self, command: Command) -> bool {
        match self.core.apply(command.clone()) {
            Ok(applied) => {
                for effect in applied.effects {
                    self.run_effect(effect);
                }
                true
            }
            Err(CoreError::NoFocusedWindow) => {
                tracing::info!(?command, "no focused window, ignored");
                false
            }
            Err(err) => {
                tracing::warn!(?command, ?err, "core rejected command");
                false
            }
        }
    }

    // ---- Effect → Smithay ----

    /// Effect を 1 つ実行する。
    fn run_effect(&mut self, effect: Effect) {
        match effect {
            Effect::Registered {
                id: EntityId::Window(id),
                ..
            } => {
                let ws = self.core.view().window(id).map(|w| w.workspace.get() + 1);
                tracing::info!(
                    "window added window={} workspace={}",
                    raw(id),
                    fmt_id(ws.map(u64::from))
                );
            }
            Effect::Registered {
                id: EntityId::Output(id),
                ..
            } => tracing::info!(output = ?id, "output registered"),
            Effect::ShowWorkspace { output, .. } => {
                let shown = self.core.view().shown_workspace(output);
                tracing::info!(
                    "show workspace={}",
                    fmt_id(shown.map(|i| u64::from(i.get()) + 1))
                );
                self.sync_space();
            }
            Effect::Configure {
                window,
                rect,
                state,
            } => self.configure(window, rect, state),
            Effect::SetFocus(focus) => self.set_focus(focus),
            Effect::RequestClose(id) => {
                tracing::info!("close focused window={}", raw(id));
                if let Some(t) = self
                    .window_of_id(id)
                    .as_ref()
                    .and_then(|w| w.toplevel().cloned())
                {
                    t.send_close();
                }
            }
            Effect::Damage(_) => self.sync_space(),
            _ => tracing::warn!(?effect, "unhandled effect"),
        }
    }

    /// Configure: 窓の大きさと状態をクライアントに伝え、Space 上の位置を合わせる。
    fn configure(&mut self, id: WindowId, tile: Rect, state: WindowState) {
        let Some(window) = self.window_of_id(id) else {
            return;
        };
        let border = self.policy.border_width().max(0);
        let rect = match state {
            WindowState::Fullscreen => tile,
            _ => crate::geometry::window_rect(self.frame_of(tile, state), border),
        };
        let toplevel = window.toplevel().expect("wayland toplevel");
        toplevel.with_pending_state(|s| {
            s.size = Some((rect.width(), rect.height()).into());
            let tiled = [
                xdg_toplevel::State::TiledLeft,
                xdg_toplevel::State::TiledRight,
                xdg_toplevel::State::TiledTop,
                xdg_toplevel::State::TiledBottom,
            ];
            for t in tiled {
                if state == WindowState::Tiled {
                    s.states.set(t);
                } else {
                    s.states.unset(t);
                }
            }
            if state == WindowState::Fullscreen {
                s.states.set(xdg_toplevel::State::Fullscreen);
            } else {
                s.states.unset(xdg_toplevel::State::Fullscreen);
            }
        });
        // 初回 configure は最初の commit で送る（handler::xdg_shell）。
        if toplevel.is_initial_configure_sent() {
            toplevel.send_pending_configure();
        }
        self.space.map_element(window, (rect.x(), rect.y()), false);
        tracing::info!(
            "layout window={} x={} y={} w={} h={} border={} state={:?}",
            raw(id),
            rect.x(),
            rect.y(),
            rect.width(),
            rect.height(),
            border,
            state
        );
    }

    /// キーボードフォーカスを core のフォーカス窓に戻す（レイヤー面が消えたあと）。
    pub fn restore_window_focus(&mut self) {
        let focus = self.core.view().focus();
        self.set_focus(focus);
    }

    /// SetFocus: キーボードフォーカスと activated 状態を合わせる。
    fn set_focus(&mut self, focus: Option<WindowId>) {
        let target = focus.and_then(|id| self.window_of_id(id));
        for w in self.shell.windows.values() {
            w.set_activated(Some(w) == target.as_ref());
            let toplevel = w.toplevel().expect("wayland toplevel");
            if toplevel.is_initial_configure_sent() {
                toplevel.send_pending_configure();
            }
        }
        let keyboard = self.seat.get_keyboard().expect("seat has keyboard");
        keyboard.set_focus(
            self,
            target.as_ref().map(surface_of),
            SERIAL_COUNTER.next_serial(),
        );
        tracing::info!("focus window={}", fmt_id(focus.map(raw)));
    }

    /// 表示中の窓だけを core の矩形で Space に置き、それ以外は外す。
    fn sync_space(&mut self) {
        let border = self.policy.border_width().max(0);
        let view = self.core.view();
        let mut visible = view.visible_windows();
        visible.sort_by_key(|w| stacking(w.state));
        let mut shown = Vec::new();
        for w in &visible {
            let (Some(tile), Some(key)) = (w.rect, view.window_key(w.id)) else {
                continue;
            };
            let Some(window) = self.shell.windows.get(&key).cloned() else {
                continue;
            };
            let rect = match w.state {
                WindowState::Fullscreen => tile,
                _ => crate::geometry::window_rect(self.frame_of(tile, w.state), border),
            };
            shown.push((window, rect));
        }
        let hidden: Vec<Window> = self
            .shell
            .windows
            .values()
            .filter(|w| !shown.iter().any(|(s, _)| s == *w))
            .cloned()
            .collect();
        for w in hidden {
            self.space.unmap_elem(&w);
        }
        for (window, rect) in shown {
            self.space.map_element(window, (rect.x(), rect.y()), false);
        }
    }
}

/// 重なり順（小さいほど下）: Tiled → Floating → Fullscreen。
fn stacking(state: WindowState) -> u8 {
    match state {
        WindowState::Tiled => 0,
        WindowState::Floating => 1,
        _ => 2,
    }
}

/// トップレベルの app_id・title・親の有無から WindowMeta を作る。
fn read_meta(window: &Window) -> WindowMeta {
    let toplevel = window.toplevel().expect("wayland toplevel");
    let (app_id, title) = with_states(toplevel.wl_surface(), |states| {
        let data = states
            .data_map
            .get::<XdgToplevelSurfaceData>()
            .expect("toplevel role data")
            .lock()
            .expect("toplevel data lock");
        (data.app_id.clone(), data.title.clone())
    });
    let mut meta = WindowMeta::default();
    meta.app_id = app_id;
    meta.title = title;
    meta.is_dialog = toplevel.parent().is_some();
    meta
}
