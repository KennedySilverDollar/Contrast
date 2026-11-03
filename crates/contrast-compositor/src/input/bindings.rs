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
            Action::CloseFocused => contrast_core::Action::CloseFocused,
            Action::FocusPrev => contrast_core::Action::FocusDirection(Direction::Left),
            Action::FocusNext => contrast_core::Action::FocusDirection(Direction::Right),
            Action::SwitchWorkspace(n) => {
                contrast_core::Action::SwitchWorkspace(WorkspaceIndex::new(n))
            }
        };
        self.core_action(core_action);
    }

    /// `program` をこのコンポジタのソケットにつないで起動する。
    fn spawn(&self, program: &str) {
        match std::process::Command::new(program)
            .env("WAYLAND_DISPLAY", &self.socket_name)
            .spawn()
        {
            Ok(mut child) => {
                tracing::info!(program, pid = child.id(), "spawned");
                // ゾンビを残さないよう、別スレッドで終了を待つ。
                std::thread::spawn(move || child.wait());
            }
            Err(err) => tracing::warn!(program, %err, "spawn failed"),
        }
    }
}
