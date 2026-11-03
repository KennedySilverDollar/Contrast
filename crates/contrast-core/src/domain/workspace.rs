//! ワークスペース: 窓の順序列とフォーカス履歴。
//!
//! 守る不変条件:
//! - I2 順序列に重複がない（`push_window` は重複を足さない）。列中の ID が存在する窓を指すことは
//!   `State` の操作が保証し、`invariant.rs` が検査する。
//! - フォーカス履歴は順序列の部分集合で、重複がない（最新が末尾）。

use alloc::vec::Vec;

use crate::domain::id::{WindowId, WorkspaceId, WorkspaceIndex};

/// ワークスペース 1 つ。D1 により全出力で共有され、数は増減しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    /// ID。
    pub id: WorkspaceId,
    /// 0 始まりのインデックス。
    pub index: WorkspaceIndex,
    /// 窓の順序列（タイル配置の並び順）。
    pub windows: Vec<WindowId>,
    /// フォーカス履歴。末尾が最新。
    pub focus_history: Vec<WindowId>,
}

impl Workspace {
    /// 空のワークスペースを作る。
    #[must_use]
    pub const fn new(id: WorkspaceId, index: WorkspaceIndex) -> Self {
        Self {
            id,
            index,
            windows: Vec::new(),
            focus_history: Vec::new(),
        }
    }

    /// 順序列の末尾に窓を足す。既にあれば何もしない（I2）。
    pub fn push_window(&mut self, id: WindowId) {
        if !self.windows.contains(&id) {
            self.windows.push(id);
        }
    }

    /// 順序列と履歴から窓を除き、順序列で占めていた位置を返す。
    pub fn remove_window(&mut self, id: WindowId) -> Option<usize> {
        self.focus_history.retain(|w| *w != id);
        let position = self.windows.iter().position(|w| *w == id)?;
        self.windows.retain(|w| *w != id);
        Some(position)
    }

    /// フォーカス履歴の最新を `id` にする。
    pub fn record_focus(&mut self, id: WindowId) {
        self.focus_history.retain(|w| *w != id);
        self.focus_history.push(id);
    }

    /// フォーカス履歴の最新。なければ順序列の先頭。どちらもなければ `None`（D12）。
    #[must_use]
    pub fn preferred_focus(&self) -> Option<WindowId> {
        self.focus_history
            .last()
            .or_else(|| self.windows.first())
            .copied()
    }
}
