//! 設定の入口: 設定ファイルのパスの解決と、読み込み器 `ConfigLoader`。
//!
//! CDL の読み込みはまだ無いので、実装は常に `NotLoaded` を返す `NotImplementedLoader` だけ。
//! CDL が使えるようになったら、`ConfigLoader` を実装した読み込み器を `cdl_adapter` に足す。

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::port::Policy;

/// 設定ファイルのパスを上書きする環境変数。
pub const CONFIG_ENV: &str = "CONTRAST_CONFIG";

/// `$HOME` からの既定の設定ファイルの相対パス。
pub const DEFAULT_RELATIVE_PATH: &str = ".config/contrast/config.cdl";

/// 設定ファイルのパスを決める。
///
/// `CONTRAST_CONFIG`（空でなければ）を優先し、無ければ `$HOME/.config/contrast/config.cdl`。
/// どちらも無ければ `None`。引数は環境変数の値（テストしやすいよう外から渡す）。
pub fn resolve_config_path(
    contrast_config: Option<OsString>,
    home: Option<OsString>,
) -> Option<PathBuf> {
    if let Some(path) = contrast_config.filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }
    home.filter(|h| !h.is_empty())
        .map(|h| PathBuf::from(h).join(DEFAULT_RELATIVE_PATH))
}

/// 読み込みの結果。
pub enum LoadOutcome {
    /// 読み込めた。このポリシーで動く。
    Loaded(Box<dyn Policy>),
    /// 読み込まなかった（読み込み器が未実装など）。静的な設定で動く。
    NotLoaded,
    /// 読み込みか評価に失敗した。静的な設定で動く。
    Failed(String),
}

/// 設定ファイルからポリシーを作る読み込み器。
pub trait ConfigLoader {
    /// ログ用の名前。
    fn name(&self) -> &'static str;

    /// `path` の設定を読み込む。
    fn load(&self, path: &Path) -> LoadOutcome;
}

/// 何も読まない読み込み器（CDL が使えるまでのスタブ）。常に `NotLoaded`。
#[derive(Debug, Clone, Copy, Default)]
pub struct NotImplementedLoader;

impl ConfigLoader for NotImplementedLoader {
    fn name(&self) -> &'static str {
        "not-implemented"
    }

    fn load(&self, _path: &Path) -> LoadOutcome {
        LoadOutcome::NotLoaded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(s: &str) -> Option<OsString> {
        Some(OsString::from(s))
    }

    #[test]
    fn env_overrides_home() {
        assert_eq!(
            resolve_config_path(os("/tmp/x.cdl"), os("/home/u")),
            Some(PathBuf::from("/tmp/x.cdl"))
        );
    }

    #[test]
    fn falls_back_to_home_config() {
        assert_eq!(
            resolve_config_path(None, os("/home/u")),
            Some(PathBuf::from("/home/u/.config/contrast/config.cdl"))
        );
        assert_eq!(
            resolve_config_path(os(""), os("/home/u")),
            Some(PathBuf::from("/home/u/.config/contrast/config.cdl"))
        );
        assert_eq!(resolve_config_path(None, None), None);
    }

    #[test]
    fn stub_never_loads() {
        assert!(matches!(
            NotImplementedLoader.load(Path::new("/nonexistent")),
            LoadOutcome::NotLoaded
        ));
    }
}
