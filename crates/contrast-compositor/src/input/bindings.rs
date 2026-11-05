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
            Action::SwitchWorkspace(n) => {
                contrast_core::Action::SwitchWorkspace(WorkspaceIndex::new(n))
            }
            Action::MoveToWorkspace(n) => {
                contrast_core::Action::MoveToWorkspace(WorkspaceIndex::new(n))
            }
        };
        self.core_action(core_action);
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
