//! `Core` 本体と `CoreConfig`。
//!
//! 守る不変条件:
//! - I7 `apply` は複製した State だけを変え、Err なら元の State を一切変えない。
//! - I8 `apply` は State と Command だけから決まる（時刻・乱数・反復順の不定なコレクションを使わない）。
//! - I1〜I6・I9 は各 handler と `arrange` が保ち、`check_invariants` で検査できる。

use crate::application::command::Command;
use crate::application::effect::Applied;
use crate::application::view::StateView;
use crate::application::{arrange, diff, handlers};
use crate::domain::state::State;
use crate::error::CoreError;
use crate::layout::Layout;
use crate::policy::Policy;

/// `Core` の設定。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreConfig {
    /// ワークスペースの数（1〜32。範囲外は `Core::new` が丸める）。
    pub workspace_count: u8,
}

impl CoreConfig {
    /// ワークスペース数を指定して作る。
    #[must_use]
    pub const fn new(workspace_count: u8) -> Self {
        Self { workspace_count }
    }
}

impl Default for CoreConfig {
    /// 既定はワークスペース 9 個。
    fn default() -> Self {
        Self::new(9)
    }
}

/// Contrast の決定的な状態遷移エンジン。
#[derive(Debug, Clone)]
pub struct Core<P: Policy, L: Layout> {
    /// 現在の状態。
    state: State,
    /// 新しい窓の配置の判断。
    policy: P,
    /// Tiled 窓の配置計算。
    layout: L,
}

impl<P: Policy, L: Layout> Core<P, L> {
    /// 空の状態（出力・窓なし、ワークスペース `workspace_count` 個）で作る。
    #[must_use]
    pub fn new(config: CoreConfig, policy: P, layout: L) -> Self {
        Self {
            state: State::new(config.workspace_count),
            policy,
            layout,
        }
    }

    /// Command を 1 つ適用し、シェルが実行すべき Effect 列を返す。
    ///
    /// # Errors
    /// 仕様 6 節の `CoreError`。Err のとき状態は変わらない（I7）。
    pub fn apply(&mut self, command: Command) -> Result<Applied, CoreError> {
        let mut next = self.state.clone();
        let from_handler = handlers::dispatch(&mut next, &self.policy, command)?;
        arrange::arrange(&mut next, &self.layout)?;
        let effects = diff::effects(&self.state, &next, from_handler);
        self.state = next;
        Ok(Applied { effects })
    }

    /// 読み取り専用の状態。
    #[must_use]
    pub const fn view(&self) -> StateView<'_> {
        StateView::new(&self.state)
    }

    /// 不変条件 I1〜I6・I9 を検査する。
    ///
    /// # Errors
    /// 最初に見つかった違反。
    #[cfg(any(test, kani, feature = "invariants"))]
    pub fn check_invariants(&self) -> Result<(), crate::invariant::InvariantViolation> {
        crate::invariant::check(&self.state)
    }

    /// 状態の複製（I7 の比較用）。
    #[cfg(any(test, kani, feature = "invariants"))]
    #[must_use]
    pub fn snapshot(&self) -> crate::invariant::StateSnapshot {
        crate::invariant::StateSnapshot::new(self.state.clone())
    }
}
