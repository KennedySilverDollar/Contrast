//! 不変条件 I1〜I6・I9 の検査（テストと Kani から呼ぶ）。I7・I8 は状態機械テストで確かめる。

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
    /// I9: 同じワークスペースに Fullscreen の窓が 2 枚以上ある。
    I9(WorkspaceIndex),
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

/// I1〜I6、I9 を順に検査し、最初の違反を返す。
///
/// # Errors
/// 違反があればその種類。
pub fn check(state: &State) -> Result<(), InvariantViolation> {
    check_i1_i2(state)?;
    check_i3(state)?;
    check_i4(state)?;
    check_i5(state)?;
    check_i6(state)?;
    check_i9(state)
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

/// I9: 同じワークスペースの Fullscreen の窓は高々 1 枚（D26）。
fn check_i9(state: &State) -> Result<(), InvariantViolation> {
    for ws in &state.workspaces {
        let fullscreen = ws
            .windows
            .iter()
            .filter_map(|id| state.windows.get(id))
            .filter(|w| w.state == WindowState::Fullscreen)
            .count();
        if fullscreen > 1 {
            return Err(InvariantViolation::I9(ws.index));
        }
    }
    Ok(())
}

/// 壊した State が、その不変条件の variant を返すことの確認（I1〜I9）。
#[cfg(test)]
mod tests {
    use super::{InvariantViolation, check};
    use crate::domain::geometry::Rect;
    use crate::domain::id::{ExternalKey, IdKind, OutputId, WindowId, WorkspaceIndex};
    use crate::domain::output::{Output, OutputName};
    use crate::domain::state::State;
    use crate::domain::window::{Window, WindowMeta, WindowState};

    /// 試験用の作業領域。
    fn area() -> Option<Rect> {
        Rect::new(0, 0, 100, 100)
    }

    /// 窓 `raw` を Tiled・ワークスペース 0・作業領域全体で足す（キーも登録する）。
    fn add_window(state: &mut State, raw: u64, window_state: WindowState) -> Option<()> {
        let id = WindowId::from_raw(raw);
        let index = WorkspaceIndex::new(0);
        let mut w = Window::new(id, WindowMeta::default(), window_state, index);
        if window_state == WindowState::Tiled {
            w.rect = area();
        }
        state.windows.insert(id, w);
        state.workspace_mut(index)?.push_window(id);
        state.window_keys.insert(ExternalKey::new(raw), id).ok()
    }

    /// 全不変条件を満たす State: 出力 1 つがワークスペース 0 を表示し、窓 1 が Tiled でフォーカス。
    fn valid() -> Option<State> {
        let mut state = State::new(2);
        let oid = OutputId::from_raw(1);
        let output = Output {
            id: oid,
            name: OutputName::new("test"),
            geometry: area()?,
            shown: WorkspaceIndex::new(0),
        };
        state.outputs.insert(oid, output);
        state.output_keys.insert(ExternalKey::new(1000), oid).ok()?;
        add_window(&mut state, 1, WindowState::Tiled)?;
        state.focus = Some(WindowId::from_raw(1));
        Some(state)
    }

    /// 壊す前の State は検査を通る。
    #[test]
    fn valid_state_passes() -> Result<(), &'static str> {
        let state = valid().ok_or("setup")?;
        assert_eq!(check(&state), Ok(()));
        Ok(())
    }

    /// I1: どのワークスペースの順序列にもない窓。
    #[test]
    fn detects_i1() -> Result<(), &'static str> {
        let mut state = valid().ok_or("setup")?;
        state
            .workspace_mut(WorkspaceIndex::new(0))
            .ok_or("ws")?
            .windows
            .clear();
        assert_eq!(
            check(&state),
            Err(InvariantViolation::I1(WindowId::from_raw(1)))
        );
        Ok(())
    }

    /// I2: 順序列の重複。
    #[test]
    fn detects_i2() -> Result<(), &'static str> {
        let mut state = valid().ok_or("setup")?;
        let index = WorkspaceIndex::new(0);
        state
            .workspace_mut(index)
            .ok_or("ws")?
            .windows
            .push(WindowId::from_raw(1));
        assert_eq!(check(&state), Err(InvariantViolation::I2(index)));
        Ok(())
    }

    /// I3: 表示ワークスペースが範囲外。
    #[test]
    fn detects_i3() -> Result<(), &'static str> {
        let mut state = valid().ok_or("setup")?;
        let oid = OutputId::from_raw(1);
        state.outputs.get_mut(&oid).ok_or("output")?.shown = WorkspaceIndex::new(9);
        assert_eq!(check(&state), Err(InvariantViolation::I3(oid)));
        Ok(())
    }

    /// I3: 2 つの出力が同じワークスペースを表示している。
    #[test]
    fn detects_i3_when_two_outputs_share_a_workspace() -> Result<(), &'static str> {
        let mut state = valid().ok_or("setup")?;
        let second = OutputId::from_raw(2);
        let output = Output {
            id: second,
            name: OutputName::new("second"),
            geometry: area().ok_or("area")?,
            shown: WorkspaceIndex::new(0),
        };
        state.outputs.insert(second, output);
        state
            .output_keys
            .insert(ExternalKey::new(1001), second)
            .map_err(|_| "key")?;
        assert_eq!(check(&state), Err(InvariantViolation::I3(second)));
        Ok(())
    }

    /// I4: フォーカスが存在しない窓を指す。
    #[test]
    fn detects_i4() -> Result<(), &'static str> {
        let mut state = valid().ok_or("setup")?;
        let ghost = WindowId::from_raw(99);
        state.focus = Some(ghost);
        assert_eq!(check(&state), Err(InvariantViolation::I4(ghost)));
        Ok(())
    }

    /// I5: Tiled 窓が作業領域からはみ出す。
    #[test]
    fn detects_i5() -> Result<(), &'static str> {
        let mut state = valid().ok_or("setup")?;
        let id = WindowId::from_raw(1);
        state.windows.get_mut(&id).ok_or("window")?.rect = Rect::new(0, 0, 200, 100);
        assert_eq!(check(&state), Err(InvariantViolation::I5(id)));
        Ok(())
    }

    /// I6: 窓のない ID がキー表にある。
    #[test]
    fn detects_i6() -> Result<(), &'static str> {
        let mut state = valid().ok_or("setup")?;
        state
            .window_keys
            .insert(ExternalKey::new(2), WindowId::from_raw(77))
            .map_err(|_| "key")?;
        assert_eq!(check(&state), Err(InvariantViolation::I6));
        Ok(())
    }

    /// I9: 同じワークスペースに Fullscreen の窓が 2 枚。
    #[test]
    fn detects_i9() -> Result<(), &'static str> {
        let mut state = valid().ok_or("setup")?;
        state
            .windows
            .get_mut(&WindowId::from_raw(1))
            .ok_or("window")?
            .state = WindowState::Fullscreen;
        add_window(&mut state, 2, WindowState::Fullscreen).ok_or("setup")?;
        assert_eq!(
            check(&state),
            Err(InvariantViolation::I9(WorkspaceIndex::new(0)))
        );
        Ok(())
    }
}
