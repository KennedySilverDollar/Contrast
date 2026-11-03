//! 意味操作 `Action` の実行。

use contrast_policy::Action;

use crate::state::Contrast;

impl Contrast {
    /// ポリシーが返した意味操作を実行する。
    pub fn run_action(&mut self, action: Action) {
        match action {
            Action::Spawn(program) => self.spawn(&program),
            Action::CloseFocused => {
                if let Some(index) = self.focused_index() {
                    let m = &self.windows[index];
                    tracing::info!(window = m.id, "close requested");
                    m.window.toplevel().expect("wayland toplevel").send_close();
                }
            }
            Action::FocusPrev => self.focus_step(-1),
            Action::FocusNext => self.focus_step(1),
            Action::SwitchWorkspace(n) => self.switch_workspace(n),
            Action::Quit => {
                tracing::info!("quit requested");
                self.loop_signal.stop();
            }
        }
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

    /// 表示中の窓の並びで、フォーカスを `delta` だけ（端で折り返して）動かす。
    fn focus_step(&mut self, delta: isize) {
        let visible = self.visible_windows();
        if visible.is_empty() {
            return;
        }
        let current = self
            .focused_index()
            .and_then(|i| visible.iter().position(|w| *w == self.windows[i].window));
        let len = visible.len() as isize;
        let next = match current {
            Some(i) => (i as isize + delta).rem_euclid(len) as usize,
            None => 0,
        };
        self.focus_window(Some(&visible[next]));
    }

    /// 表示するワークスペースを切り替える（M4）。
    fn switch_workspace(&mut self, n: u8) {
        tracing::info!(workspace = n + 1, "switch workspace: not implemented yet");
    }
}
