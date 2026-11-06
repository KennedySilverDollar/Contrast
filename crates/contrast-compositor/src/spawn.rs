//! 子プロセスの起動。`Startup`（起動時に一度だけ）と `spawn` のキーバインドが共有する。
//!
//! 子の環境変数には、`Environment` の組を足し、最後に `WAYLAND_DISPLAY`（このコンポジタのソケット）を置く
//! （`Environment` で上書きさせない）。コンポジタ自身の環境は変えない。

use std::ffi::OsStr;
use std::process::Command;

use crate::state::Contrast;

/// `command`（空白で区切った先頭がプログラム、残りが引数）から起動するコマンドを作る。空なら `None`。
pub(crate) fn build_command(
    command: &str,
    socket: &OsStr,
    environment: &[(String, String)],
) -> Option<Command> {
    let mut words = command.split_whitespace();
    let program = words.next()?;
    let mut cmd = Command::new(program);
    cmd.args(words);
    for (name, value) in environment {
        cmd.env(name, value);
    }
    cmd.env("WAYLAND_DISPLAY", socket);
    Some(cmd)
}

impl Contrast {
    /// `command` をこのコンポジタのソケットにつないで起動する。失敗はログに出すだけ。`kind` はログ用の起動元。
    pub(crate) fn spawn_command(&self, command: &str, kind: &'static str) {
        let environment = self.policy.environment();
        let Some(mut cmd) = build_command(command, &self.socket_name, &environment) else {
            return;
        };
        match cmd.spawn() {
            Ok(mut child) => {
                tracing::info!(kind, command, pid = child.id(), "spawned");
                // ゾンビを残さないよう、別スレッドで終了を待つ。
                std::thread::spawn(move || child.wait());
            }
            Err(err) => tracing::warn!(kind, command, %err, "spawn failed"),
        }
    }

    /// `Startup` のコマンドを、並びの順に起動する（Wayland のソケットを公開したあとに一度だけ呼ぶ）。
    /// 失敗しても残りを続ける。リロードでは呼ばない。
    pub(crate) fn run_startup(&self) {
        let commands = self.policy.startup_commands();
        tracing::info!(count = commands.len(), "startup: running commands");
        for command in &commands {
            self.spawn_command(command, "startup");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// コマンドの環境変数の指定（`env_remove` は含まない）。
    fn envs(cmd: &Command) -> Vec<(String, String)> {
        cmd.get_envs()
            .filter_map(|(k, v)| {
                Some((
                    k.to_string_lossy().into_owned(),
                    v?.to_string_lossy().into_owned(),
                ))
            })
            .collect()
    }

    #[test]
    fn program_and_arguments_are_split_on_whitespace() {
        let cmd = build_command("ironbar -c  /a.toml", OsStr::new("wayland-9"), &[]);
        let cmd = cmd.expect("command");
        assert_eq!(cmd.get_program(), "ironbar");
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, ["-c", "/a.toml"]);
    }

    #[test]
    fn empty_command_is_none() {
        assert!(build_command("", OsStr::new("w"), &[]).is_none());
        assert!(build_command("   ", OsStr::new("w"), &[]).is_none());
    }

    #[test]
    fn environment_is_passed_and_the_socket_wins() {
        let env = vec![
            ("QT_IM_MODULE".to_owned(), "fcitx5".to_owned()),
            ("WAYLAND_DISPLAY".to_owned(), "evil".to_owned()),
        ];
        let cmd = build_command("foot", OsStr::new("wayland-9"), &env).expect("command");
        let got = envs(&cmd);
        assert!(got.contains(&("QT_IM_MODULE".to_owned(), "fcitx5".to_owned())));
        // 同じ名前は後の指定が勝つ（Command は最後の指定を持つ）。
        let wayland: Vec<_> = got.iter().filter(|(k, _)| k == "WAYLAND_DISPLAY").collect();
        assert_eq!(
            wayland,
            [&("WAYLAND_DISPLAY".to_owned(), "wayland-9".to_owned())]
        );
    }

    #[test]
    fn the_compositor_environment_is_not_touched() {
        let before = std::env::var_os("CONTRAST_SPAWN_TEST_VAR");
        let env = vec![("CONTRAST_SPAWN_TEST_VAR".to_owned(), "x".to_owned())];
        let _ = build_command("true", OsStr::new("w"), &env);
        assert_eq!(std::env::var_os("CONTRAST_SPAWN_TEST_VAR"), before);
    }

    /// 実際に子を起動して、環境変数が子に届くことを確かめる（`sh` と `printenv` がある環境）。
    #[test]
    fn the_child_sees_the_environment() {
        let env = vec![("QT_IM_MODULE".to_owned(), "fcitx5".to_owned())];
        let mut cmd = build_command("sh -c", OsStr::new("wayland-9"), &env).expect("command");
        let out = cmd
            .arg("printf '%s %s' \"$QT_IM_MODULE\" \"$WAYLAND_DISPLAY\"")
            .output();
        if let Ok(out) = out {
            assert_eq!(String::from_utf8_lossy(&out.stdout), "fcitx5 wayland-9");
        }
    }
}
