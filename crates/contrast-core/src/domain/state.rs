//! `State`: 出力・ワークスペース・窓・フォーカス・ID 対応表の集約。
//!
//! ここには状態を読む・小さく書き換える操作だけを置き、ユースケースの判断は
//! `application/handlers/` が行う。
//!
//! 守る不変条件（操作ごとに保つもの。検査は `invariant.rs`）:
//! - I1 / I2 窓の所属（`Window::workspace`）と順序列を `remove_window` などで同時に更新する。
//! - I3 ワークスペース数は固定（D1）。
//! - I4 フォーカスは `set_focus` だけが変え、履歴も同時に更新する。
//! - I6 ID は `IdGen` から払い出し、対応表は `KeyMap` で持つ。

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::domain::id::{IdGen, IdKind, KeyMap, OutputId, WindowId, WorkspaceId, WorkspaceIndex};
use crate::domain::output::Output;
use crate::domain::window::Window;
use crate::domain::workspace::Workspace;

/// ワークスペース数の下限。
pub const MIN_WORKSPACES: u8 = 1;
/// ワークスペース数の上限。
pub const MAX_WORKSPACES: u8 = 32;

/// core の全状態。`Clone` して変更し、成功時だけ差し替える（I7）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// 出力。OutputId 昇順。
    pub outputs: BTreeMap<OutputId, Output>,
    /// ワークスペース。添字がそのまま `WorkspaceIndex`（D1: 数は固定）。
    pub workspaces: Vec<Workspace>,
    /// 窓。WindowId 昇順。
    pub windows: BTreeMap<WindowId, Window>,
    /// フォーカス中の窓。
    pub focus: Option<WindowId>,
    /// 出力の ExternalKey 対応表。
    pub output_keys: KeyMap<OutputId>,
    /// 窓の ExternalKey 対応表。
    pub window_keys: KeyMap<WindowId>,
    /// 出力 ID の払い出し器。
    pub output_ids: IdGen<OutputId>,
    /// 窓 ID の払い出し器。
    pub window_ids: IdGen<WindowId>,
}

impl State {
    /// `workspace_count` 個（1〜32 に丸める）の空ワークスペースを持つ状態を作る。
    #[must_use]
    pub fn new(workspace_count: u8) -> Self {
        let count = workspace_count.clamp(MIN_WORKSPACES, MAX_WORKSPACES);
        let workspaces = (0..count)
            .map(|i| {
                let raw = u64::from(i).saturating_add(1);
                Workspace::new(WorkspaceId::from_raw(raw), WorkspaceIndex::new(i))
            })
            .collect();
        Self {
            outputs: BTreeMap::new(),
            workspaces,
            windows: BTreeMap::new(),
            focus: None,
            output_keys: KeyMap::new(),
            window_keys: KeyMap::new(),
            output_ids: IdGen::new(),
            window_ids: IdGen::new(),
        }
    }

    /// ワークスペースの数。
    #[must_use]
    pub fn workspace_count(&self) -> u8 {
        u8::try_from(self.workspaces.len()).unwrap_or(MAX_WORKSPACES)
    }

    /// インデックスのワークスペース。
    #[must_use]
    pub fn workspace(&self, index: WorkspaceIndex) -> Option<&Workspace> {
        self.workspaces.get(usize::from(index.get()))
    }

    /// インデックスのワークスペース（可変）。
    pub fn workspace_mut(&mut self, index: WorkspaceIndex) -> Option<&mut Workspace> {
        self.workspaces.get_mut(usize::from(index.get()))
    }

    /// ワークスペース `index` を表示している出力。
    #[must_use]
    pub fn output_showing(&self, index: WorkspaceIndex) -> Option<OutputId> {
        self.outputs
            .values()
            .find(|o| o.shown == index)
            .map(|o| o.id)
    }

    /// 窓が表示中のワークスペースにあれば true。
    #[must_use]
    pub fn is_window_visible(&self, id: WindowId) -> bool {
        self.windows
            .get(&id)
            .is_some_and(|w| self.output_showing(w.workspace).is_some())
    }

    /// フォーカス出力（D5）: フォーカス窓のある出力、なければ OutputId 最小の出力。
    #[must_use]
    pub fn focused_output(&self) -> Option<OutputId> {
        self.focus
            .and_then(|id| self.windows.get(&id))
            .and_then(|w| self.output_showing(w.workspace))
            .or_else(|| self.outputs.keys().next().copied())
    }

    /// フォーカスを `focus` にし、その窓のワークスペースの履歴を更新する。
    pub fn set_focus(&mut self, focus: Option<WindowId>) {
        self.focus = focus;
        let Some(id) = focus else { return };
        let Some(index) = self.windows.get(&id).map(|w| w.workspace) else {
            return;
        };
        if let Some(ws) = self.workspace_mut(index) {
            ws.record_focus(id);
        }
    }

    /// 窓を状態から除く。所属していたワークスペースと、順序列での位置を返す。
    pub fn remove_window(&mut self, id: WindowId) -> Option<(WorkspaceIndex, Option<usize>)> {
        let window = self.windows.remove(&id)?;
        let position = self
            .workspace_mut(window.workspace)
            .and_then(|ws| ws.remove_window(id));
        if self.focus == Some(id) {
            self.focus = None;
        }
        Some((window.workspace, position))
    }
}
