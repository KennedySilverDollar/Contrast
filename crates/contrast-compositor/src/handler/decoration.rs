//! xdg-decoration: クライアントの要求にかかわらず ServerSide を返す。
//!
//! コンポジタ側は装飾を描かないので、結果としてタイトルバーも枠も無く、
//! 窓内容だけがタイルいっぱいに表示される。

use smithay::{
    delegate_xdg_decoration,
    reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode,
    wayland::shell::xdg::{ToplevelSurface, decoration::XdgDecorationHandler},
};

use crate::state::Contrast;

impl Contrast {
    /// toplevel の装飾モードを ServerSide にして、初回 configure 後なら即座に伝える。
    fn force_server_side(&mut self, toplevel: &ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ServerSide);
        });
        if toplevel.is_initial_configure_sent() {
            toplevel.send_pending_configure();
        }
        let id = self
            .index_of_surface(toplevel.wl_surface())
            .map(|i| self.windows[i].id);
        tracing::info!("decoration mode=ServerSide window={}", fmt_id(id));
    }
}

/// ログ用に窓の通し番号を書く（窓が無い・未管理なら `none`）。
pub fn fmt_id(id: Option<u64>) -> String {
    id.map_or_else(|| "none".to_string(), |id| id.to_string())
}

impl XdgDecorationHandler for Contrast {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        self.force_server_side(&toplevel);
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, _mode: Mode) {
        self.force_server_side(&toplevel);
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        self.force_server_side(&toplevel);
    }
}

delegate_xdg_decoration!(Contrast);
