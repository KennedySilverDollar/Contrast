//! 出力（モニタ）: `Output` と `OutputName`。
//!
//! 守る不変条件:
//! - I3 各出力は表示中ワークスペースをちょうど 1 つ持つ（`shown` は常に値を持つ）。
//!   2 つの出力が同じワークスペースを表示しないことは `State` の操作が保証する。
//! - D4 作業領域は論理矩形と同じ。

use alloc::string::String;

use crate::domain::geometry::Rect;
use crate::domain::id::{OutputId, WorkspaceIndex};

/// 出力の名前（例: `"winit"`、`"DP-1"`）。core は表示用に保持するだけ。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputName(String);

impl OutputName {
    /// 名前 `name` を作る。
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// 文字列として返す。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// core が管理する出力 1 つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// ID。
    pub id: OutputId,
    /// 名前。
    pub name: OutputName,
    /// 論理矩形。
    pub geometry: Rect,
    /// 表示中のワークスペース（I3）。
    pub shown: WorkspaceIndex,
}

impl Output {
    /// 作業領域。D4 により論理矩形と同じ。
    #[must_use]
    pub const fn work_area(&self) -> Rect {
        self.geometry
    }
}
