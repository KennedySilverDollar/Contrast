//! キーバインドの意味操作（`contrast_policy::Action`）の実行。
//!
//! 窓の配置・フォーカスに関わるものは contrast-core の `Action` に翻訳して `Core` に渡す。
//! 端末の起動と終了は core を通さない。

use contrast_core::{Direction, WorkspaceIndex};
use contrast_policy::Action;

use crate::state::Contrast;

impl Contrast {
    /// ポリシーが返した意味操作を実行する。
    pub fn run_action(&mut self, action: Action) {
        let core_action = match action {
            Action::Spawn(program) => return self.spawn(&program),
            Action::Quit => {
                tracing::info!("quit requested");
                return self.loop_signal.stop();
            }
            Action::ReloadConfig => return self.reload_config(),
            Action::CloseFocused => contrast_core::Action::CloseFocused,
            Action::FocusPrev => contrast_core::Action::FocusDirection(Direction::Left),
            Action::FocusNext => contrast_core::Action::FocusDirection(Direction::Right),
            Action::MoveLeft => contrast_core::Action::MoveWindow(Direction::Left),
            Action::MoveRight => contrast_core::Action::MoveWindow(Direction::Right),
            Action::ToggleFloating => contrast_core::Action::ToggleFloating,
            Action::ToggleFullscreen => contrast_core::Action::ToggleFullscreen,
            Action::WorkspaceNext => return self.step_workspace(1, false),
            Action::WorkspacePrev => return self.step_workspace(-1, false),
            Action::MoveToWorkspaceNext => return self.step_workspace(1, true),
            Action::MoveToWorkspacePrev => return self.step_workspace(-1, true),
            Action::SwitchWorkspace(n) => {
                contrast_core::Action::SwitchWorkspace(WorkspaceIndex::new(n))
            }
            Action::MoveToWorkspace(n) => {
                contrast_core::Action::MoveToWorkspace(WorkspaceIndex::new(n))
            }
        };
        self.core_action(core_action);
    }

    /// 表示中のワークスペースの番号（0 始まり）。出力が無ければ `None`。
    fn shown_workspace_index(&self) -> Option<u8> {
        let view = self.core.view();
        let output = view.output_id(self.shell.output_key?)?;
        view.shown_workspace(output).map(|i| i.get())
    }

    /// 表示中の番号から `delta` だけ離れたワークスペースへ、切り替える（`carry` なら窓も移す）。範囲外は何もしない。
    fn step_workspace(&mut self, delta: i8, carry: bool) {
        let Some(shown) = self.shown_workspace_index() else {
            return;
        };
        let Some(target) = step_index(shown, delta, self.policy.workspace_count()) else {
            tracing::info!(
                shown = shown + 1,
                delta,
                "workspace step: out of range, ignored"
            );
            return;
        };
        let index = WorkspaceIndex::new(target);
        self.core_action(if carry {
            contrast_core::Action::MoveToWorkspace(index)
        } else {
            contrast_core::Action::SwitchWorkspace(index)
        });
    }

    /// `command`（空白で区切った先頭がプログラム、残りが引数）をこのコンポジタのソケットにつないで起動する。
    fn spawn(&self, command: &str) {
        let mut words = command.split_whitespace();
        let Some(program) = words.next() else {
            return;
        };
        match std::process::Command::new(program)
            .args(words)
            .env("WAYLAND_DISPLAY", &self.socket_name)
            .spawn()
        {
            Ok(mut child) => {
                tracing::info!(command, pid = child.id(), "spawned");
                // ゾンビを残さないよう、別スレッドで終了を待つ。
                std::thread::spawn(move || child.wait());
            }
            Err(err) => tracing::warn!(command, %err, "spawn failed"),
        }
    }
}

/// `shown` から `delta` だけ離れた番号。`0..count` を出るなら `None`（循環しない）。
fn step_index(shown: u8, delta: i8, count: u8) -> Option<u8> {
    let n = i16::from(shown).checked_add(i16::from(delta))?;
    if n < 0 || n >= i16::from(count) {
        return None;
    }
    u8::try_from(n).ok()
}

#[cfg(test)]
mod tests {
    use super::step_index;

    #[test]
    fn steps_within_range() {
        assert_eq!(step_index(0, 1, 4), Some(1));
        assert_eq!(step_index(2, -1, 4), Some(1));
        assert_eq!(step_index(2, 1, 4), Some(3));
    }

    #[test]
    fn out_of_range_does_nothing() {
        assert_eq!(step_index(3, 1, 4), None);
        assert_eq!(step_index(0, -1, 4), None);
        assert_eq!(step_index(0, 1, 0), None);
        assert_eq!(step_index(255, 1, 255), None);
    }
}
