//! バックエンド。ホストの Wayland 上の winit 窓と、実機の DRM（フェーズ1: 出力の列挙のみ）を持つ。

pub mod drm;
pub mod winit;

use std::fmt;

/// 使うバックエンド。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// ホストの Wayland 上の winit 窓（開発用）。
    Winit,
    /// 実機の DRM。
    Drm,
}

/// バックエンドを決められなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectError {
    /// `CONTRAST_BACKEND` が `winit` / `drm` のどちらでもない。
    Unknown(String),
    /// 未指定で、ホストの Wayland / X11 も無い。
    NoHost,
}

impl fmt::Display for SelectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectError::Unknown(v) => write!(
                f,
                "CONTRAST_BACKEND={v} は不明です。winit か drm を指定してください"
            ),
            SelectError::NoHost => write!(
                f,
                "WAYLAND_DISPLAY も DISPLAY も無いため winit は使えません。\
                 DRM（実機）を使うなら CONTRAST_BACKEND=drm を指定してください"
            ),
        }
    }
}

impl std::error::Error for SelectError {}

/// 環境変数の値からバックエンドを選ぶ。DRM は `drm` を明示したときだけ有効。
///
/// `choice` は `CONTRAST_BACKEND`、`has_host_display` は `WAYLAND_DISPLAY` か `DISPLAY` があるか。
pub fn select(choice: Option<&str>, has_host_display: bool) -> Result<Kind, SelectError> {
    match choice {
        Some("winit") => Ok(Kind::Winit),
        Some("drm") => Ok(Kind::Drm),
        Some(other) => Err(SelectError::Unknown(other.to_string())),
        None if has_host_display => Ok(Kind::Winit),
        None => Err(SelectError::NoHost),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_choice_wins() {
        assert_eq!(select(Some("drm"), true), Ok(Kind::Drm));
        assert_eq!(select(Some("winit"), false), Ok(Kind::Winit));
    }

    #[test]
    fn unset_needs_a_host_display() {
        assert_eq!(select(None, true), Ok(Kind::Winit));
        assert_eq!(select(None, false), Err(SelectError::NoHost));
    }

    #[test]
    fn unknown_value_is_an_error() {
        assert_eq!(
            select(Some("x11"), true),
            Err(SelectError::Unknown("x11".to_string()))
        );
    }
}
