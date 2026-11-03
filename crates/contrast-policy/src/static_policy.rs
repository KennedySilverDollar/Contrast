//! 静的な既定ポリシー。
//!
//! 配置は `Rect::split_columns` による横一列。キーバインドは固定表。

use contrast_core::Rect;

use crate::port::{Action, Mods, Policy};

/// 静的な既定値で答えるポリシー。
#[derive(Debug, Clone)]
pub struct StaticPolicy {
    /// Alt+Return で起動する端末のプログラム名。
    pub terminal: String,
}

impl StaticPolicy {
    /// 端末のプログラム名を指定して作る。
    pub fn new(terminal: impl Into<String>) -> Self {
        Self {
            terminal: terminal.into(),
        }
    }
}

impl Policy for StaticPolicy {
    fn background(&self) -> [f32; 4] {
        [0.12, 0.14, 0.20, 1.0]
    }

    fn border_width(&self) -> i32 {
        3
    }

    fn focus_border_color(&self) -> [f32; 4] {
        [0.40, 0.65, 1.0, 1.0]
    }

    fn workspace_count(&self) -> u8 {
        4
    }

    fn layout(&self, area: Rect, count: usize) -> Vec<Rect> {
        if count == 0 {
            return Vec::new();
        }
        // 幅が窓数に足りないときは全窓を作業領域いっぱいに重ねる（プロトタイプの割り切り）。
        area.split_columns(count)
            .unwrap_or_else(|_| vec![area; count])
    }

    fn resolve_key(&self, mods: Mods, keysym: &str) -> Option<Action> {
        if !mods.alt || mods.ctrl || mods.logo {
            return None;
        }
        let action = match (mods.shift, keysym) {
            (false, "Return") => Action::Spawn(self.terminal.clone()),
            (false, "q") => Action::CloseFocused,
            (true, "q") => Action::Quit,
            (false, "Left") => Action::FocusPrev,
            (false, "Right") => Action::FocusNext,
            (false, "1") => Action::SwitchWorkspace(0),
            (false, "2") => Action::SwitchWorkspace(1),
            (false, "3") => Action::SwitchWorkspace(2),
            (false, "4") => Action::SwitchWorkspace(3),
            _ => return None,
        };
        Some(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alt(shift: bool) -> Mods {
        Mods {
            alt: true,
            shift,
            ..Mods::default()
        }
    }

    #[test]
    fn keybindings() {
        let p = StaticPolicy::new("foot");
        assert_eq!(
            p.resolve_key(alt(false), "Return"),
            Some(Action::Spawn("foot".into()))
        );
        assert_eq!(p.resolve_key(alt(false), "q"), Some(Action::CloseFocused));
        assert_eq!(p.resolve_key(alt(true), "q"), Some(Action::Quit));
        for (key, ws) in [("1", 0), ("2", 1), ("3", 2), ("4", 3)] {
            assert_eq!(
                p.resolve_key(alt(false), key),
                Some(Action::SwitchWorkspace(ws))
            );
            assert!(ws < p.workspace_count());
        }
        assert_eq!(p.resolve_key(alt(false), "5"), None);
        assert_eq!(p.resolve_key(alt(true), "Return"), None);
        assert_eq!(p.resolve_key(Mods::default(), "q"), None);
    }

    #[test]
    fn layout_fills_area() {
        let p = StaticPolicy::new("foot");
        let area = Rect::new(0, 0, 1001, 600).unwrap();
        let rects = p.layout(area, 3);
        assert_eq!(rects.len(), 3);
        assert_eq!(rects.iter().map(Rect::width).sum::<i32>(), 1001);
        assert!(p.layout(area, 0).is_empty());
    }
}
