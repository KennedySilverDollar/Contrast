//! 動作確認用の stdin コマンド（`CONTRAST_DEBUG_STDIN=1` のときだけ有効）。
//!
//! 画面を見られない環境でキーバインドとクリックの経路を試すためのもの。1 行 1 コマンド:
//! - `key alt+shift+q` … 修飾キーとキーシンボル名をポリシーで解決して実行する
//!   （xkb のキーコード→キーシンボル変換は通らない）
//! - `click X Y` … 論理座標 (X, Y) をクリックしたときのフォーカス移動
//! - `state` … 管理中の窓の一覧をログに出す

use std::io::BufRead;

use smithay::reexports::calloop::{
    EventLoop,
    channel::{self, Event},
};

use contrast_policy::Mods;

use crate::state::Contrast;

/// 環境変数が立っていれば、stdin を読むスレッドとイベントループへの経路を作る。
pub fn init(event_loop: &mut EventLoop<Contrast>) {
    if std::env::var_os("CONTRAST_DEBUG_STDIN").is_none() {
        return;
    }
    let (sender, receiver) = channel::channel::<String>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    event_loop
        .handle()
        .insert_source(receiver, |event, _, state| {
            if let Event::Msg(line) = event {
                state.debug_command(line.trim());
            }
        })
        .expect("failed to insert debug stdin source");
    tracing::info!("debug stdin commands enabled");
}

impl Contrast {
    /// stdin から来た 1 行を実行する。
    fn debug_command(&mut self, line: &str) {
        let mut words = line.split_whitespace();
        match (words.next(), words.next(), words.next()) {
            (Some("key"), Some(combo), None) => {
                let mut mods = Mods::default();
                let mut parts: Vec<&str> = combo.split('+').collect();
                let key = parts.pop().unwrap_or_default();
                for m in parts {
                    match m {
                        "alt" => mods.alt = true,
                        "shift" => mods.shift = true,
                        "ctrl" => mods.ctrl = true,
                        "logo" | "super" => mods.logo = true,
                        other => tracing::warn!(modifier = other, "debug: unknown modifier"),
                    }
                }
                match self.policy.resolve_key(mods, key) {
                    Some(action) => {
                        tracing::info!(combo, ?action, "debug keybinding");
                        self.run_action(action);
                    }
                    None => tracing::info!(combo, "debug key: no binding"),
                }
            }
            (Some("click"), Some(x), Some(y)) => match (x.parse::<f64>(), y.parse::<f64>()) {
                (Ok(x), Ok(y)) => self.focus_at((x, y).into()),
                _ => tracing::warn!(line, "debug: bad click"),
            },
            (Some("state"), None, None) => {
                let focused = self.focused_index().map(|i| self.windows[i].id);
                for m in &self.windows {
                    tracing::info!(
                        window = m.id,
                        workspace = m.workspace + 1,
                        location = ?self.space.element_location(&m.window),
                        "debug state"
                    );
                }
                tracing::info!(
                    active_workspace = self.active_ws + 1,
                    focused = ?focused,
                    "debug state"
                );
            }
            _ => tracing::warn!(line, "debug: unknown command"),
        }
    }
}
