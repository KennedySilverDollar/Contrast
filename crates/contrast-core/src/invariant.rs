//! 不変条件 I1〜I6 の検査（テストと Kani から呼ぶ）。I7・I8 は状態機械テストで確かめる。

use alloc::vec::Vec;

use crate::domain::id::{OutputId, WindowId, WorkspaceIndex};
use crate::domain::state::State;
use crate::domain::window::WindowState;

/// 見つかった不変条件の違反。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvariantViolation {
    /// I1: 窓がちょうど 1 つのワークスペースに属していない。
    I1(WindowId),
    /// I2: 順序列に重複か、存在しない窓がある。
    I2(WorkspaceIndex),
    /// I3: 出力の表示ワークスペースが範囲外か、2 つの出力で重なっている。
    I3(OutputId),
    /// I4: フォーカスが存在しない窓か、表示されていない窓を指している。
    I4(WindowId),
    /// I5: 表示中の Tiled 窓が作業領域からはみ出すか、重なっている。
    I5(WindowId),
    /// I6: ExternalKey と ID の対応が全単射でない。
    I6,
}

/// I7 の比較用の状態の複製。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSnapshot(State);

impl StateSnapshot {
    /// 状態の複製から作る。
    pub(crate) const fn new(state: State) -> Self {
        Self(state)
    }
}

/// I1〜I6 を順に検査し、最初の違反を返す。
///
/// # Errors
/// 違反があればその種類。
pub fn check(state: &State) -> Result<(), InvariantViolation> {
    check_i1_i2(state)?;
    check_i3(state)?;
    check_i4(state)?;
    check_i5(state)?;
    check_i6(state)
}

/// I1・I2: 所属と順序列の整合。
fn check_i1_i2(state: &State) -> Result<(), InvariantViolation> {
    for ws in &state.workspaces {
        let mut seen: Vec<WindowId> = Vec::with_capacity(ws.windows.len());
        for id in &ws.windows {
            let belongs = state
                .windows
                .get(id)
                .is_some_and(|w| w.workspace == ws.index);
            if seen.contains(id) || !belongs {
                return Err(InvariantViolation::I2(ws.index));
            }
            seen.push(*id);
        }
        if ws.focus_history.iter().any(|id| !ws.windows.contains(id)) {
            return Err(InvariantViolation::I2(ws.index));
        }
    }
    for w in state.windows.values() {
        let count = state
            .workspaces
            .iter()
            .filter(|ws| ws.windows.contains(&w.id))
            .count();
        let in_own = state
            .workspace(w.workspace)
            .is_some_and(|ws| ws.windows.contains(&w.id));
        if count != 1 || !in_own {
            return Err(InvariantViolation::I1(w.id));
        }
    }
    Ok(())
}

/// I3: 各出力の表示ワークスペースは範囲内で、互いに異なる。
fn check_i3(state: &State) -> Result<(), InvariantViolation> {
    let mut shown: Vec<WorkspaceIndex> = Vec::with_capacity(state.outputs.len());
    for o in state.outputs.values() {
        if state.workspace(o.shown).is_none() || shown.contains(&o.shown) {
            return Err(InvariantViolation::I3(o.id));
        }
        shown.push(o.shown);
    }
    Ok(())
}

/// I4: フォーカスは None か表示中の窓。
fn check_i4(state: &State) -> Result<(), InvariantViolation> {
    match state.focus {
        Some(id) if !state.is_window_visible(id) => Err(InvariantViolation::I4(id)),
        _ => Ok(()),
    }
}

/// I5: 表示中の Tiled 窓は作業領域内に収まり、互いに重ならない。
fn check_i5(state: &State) -> Result<(), InvariantViolation> {
    for o in state.outputs.values() {
        let area = o.work_area();
        let Some(ws) = state.workspace(o.shown) else {
            continue;
        };
        let mut placed: Vec<(WindowId, crate::domain::geometry::Rect)> = Vec::new();
        for id in &ws.windows {
            let Some(w) = state.windows.get(id) else {
                continue;
            };
            if w.state != WindowState::Tiled {
                continue;
            }
            let Some(rect) = w.rect else {
                return Err(InvariantViolation::I5(*id));
            };
            let inside = area.intersect(&rect) == Some(rect);
            let overlaps = placed.iter().any(|(_, r)| r.intersect(&rect).is_some());
            if !inside || overlaps {
                return Err(InvariantViolation::I5(*id));
            }
            placed.push((*id, rect));
        }
    }
    Ok(())
}

/// I6: ExternalKey と ID の対応が全単射で、登録済みの実体と一致する。
fn check_i6(state: &State) -> Result<(), InvariantViolation> {
    let windows_ok = state.window_keys.is_bijective()
        && state.window_keys.len() == state.windows.len()
        && state
            .windows
            .keys()
            .all(|id| state.window_keys.key_of(*id).is_some());
    let outputs_ok = state.output_keys.is_bijective()
        && state.output_keys.len() == state.outputs.len()
        && state
            .outputs
            .keys()
            .all(|id| state.output_keys.key_of(*id).is_some());
    if windows_ok && outputs_ok {
        Ok(())
    } else {
        Err(InvariantViolation::I6)
    }
}
