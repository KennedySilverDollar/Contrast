//! ポリシー: 新しい窓の配置先・状態・フォーカスを決める `Policy` トレイトと既定実装。
//!
//! ポリシーは判断だけを返し、状態は変えない。結果の適用は application 層が行う（D6・D7）。

mod default;

use crate::domain::id::WorkspaceIndex;
use crate::domain::window::{WindowMeta, WindowState};

pub use default::DefaultPolicy;

/// 新しい窓の配置の判断。
pub trait Policy {
    /// `meta` の窓をどこにどう置くか決める。
    fn place(&self, meta: &WindowMeta, ctx: &PlacementContext<'_>) -> Placement;
}

/// `Policy::place` の結果。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    /// 配置先。`None` ならフォーカス出力の表示ワークスペース（出力 0 台ならワークスペース 0、D6）。
    pub workspace: Option<WorkspaceIndex>,
    /// 初期状態。
    pub state: WindowState,
    /// 配置先が表示中ならフォーカスを与えるか（D7）。
    pub focus: bool,
}

impl Placement {
    /// 配置の判断を作る。
    #[must_use]
    pub const fn new(workspace: Option<WorkspaceIndex>, state: WindowState, focus: bool) -> Self {
        Self {
            workspace,
            state,
            focus,
        }
    }
}

/// `Policy::place` に渡す読み取り専用の文脈。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacementContext<'a> {
    /// フォーカス出力の表示ワークスペース。出力 0 台なら `None`。
    focused_workspace: Option<WorkspaceIndex>,
    /// ワークスペースの数。
    workspace_count: u8,
    /// 出力の数。
    output_count: usize,
    /// 将来の拡張（状態への参照など）のための寿命。
    marker: core::marker::PhantomData<&'a ()>,
}

impl PlacementContext<'_> {
    /// 文脈を作る。
    #[must_use]
    pub const fn new(
        focused_workspace: Option<WorkspaceIndex>,
        workspace_count: u8,
        output_count: usize,
    ) -> Self {
        Self {
            focused_workspace,
            workspace_count,
            output_count,
            marker: core::marker::PhantomData,
        }
    }

    /// フォーカス出力の表示ワークスペース。
    #[must_use]
    pub const fn focused_workspace(&self) -> Option<WorkspaceIndex> {
        self.focused_workspace
    }

    /// ワークスペースの数。
    #[must_use]
    pub const fn workspace_count(&self) -> u8 {
        self.workspace_count
    }

    /// 出力の数。
    #[must_use]
    pub const fn output_count(&self) -> usize {
        self.output_count
    }
}
