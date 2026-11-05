//! ワークスペース: 窓の順序列とフォーカス履歴。
//!
//! 守る不変条件:
//! - I2 順序列に重複がない（`push_window` は重複を足さない）。列中の ID が存在する窓を指すことは
//!   `State` の操作が保証し、`invariant.rs` が検査する。
//! - フォーカス履歴は順序列の部分集合で、重複がない（最新が末尾）。

use alloc::vec::Vec;

use crate::domain::column::Column;
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
    /// 列の並び（左から右）。`Scrolling` のときだけ使い、`Equal` では空のまま（D36）。
    pub columns: Vec<Column>,
    /// ビューの左端（作業領域の左端から数えた x、0 以上。`Scrolling` のみ、D39）。
    pub view_x: i32,
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
            columns: Vec::new(),
            view_x: 0,
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
        self.remove_from_columns(id);
        let position = self.windows.iter().position(|w| *w == id)?;
        self.windows.retain(|w| *w != id);
        Some(position)
    }

    /// 窓 `id` がある列の位置 `(列, 列の中の行)`。どの列にもなければ `None`。
    #[must_use]
    pub fn column_of(&self, id: WindowId) -> Option<(usize, usize)> {
        self.columns.iter().enumerate().find_map(|(c, col)| {
            col.windows
                .iter()
                .position(|w| *w == id)
                .map(|row| (c, row))
        })
    }

    /// 窓 `id` を列から除く。空になった列も除く。
    pub fn remove_from_columns(&mut self, id: WindowId) {
        for col in &mut self.columns {
            col.windows.retain(|w| *w != id);
        }
        self.columns.retain(|col| !col.windows.is_empty());
    }

    /// 列 `from` を位置 `to` へ移す（間の列は 1 つずつずれる）。範囲外なら何もしない。
    pub fn move_column(&mut self, from: usize, to: usize) {
        if from >= self.columns.len() || to >= self.columns.len() || from == to {
            return;
        }
        let column = self.columns.remove(from);
        self.columns.insert(to, column);
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
