//! SwitchWorkspace: フォーカス出力の表示をワークスペース i にする（D12）。
//!
//! i が別の出力で表示中なら 2 出力の表示を入れ替える（I3: 同じワークスペースを 2 出力が表示しない）。
//! 切替後のフォーカスは i のフォーカス履歴の最新、なければ順序列先頭、なければ None（I4）。
//! 出力 0 台、または既に i を表示しているときは状態不変で `Ok`。

use alloc::vec::Vec;

use crate::application::effect::Effect;
use crate::application::handlers::check_index;
use crate::domain::id::WorkspaceIndex;
use crate::domain::state::State;
use crate::error::CoreError;

/// 表示ワークスペースを切り替える。
///
/// # Errors
/// i が範囲外なら `WorkspaceIndexOutOfRange`。
pub fn handle(state: &mut State, index: WorkspaceIndex) -> Result<Vec<Effect>, CoreError> {
    check_index(state, index)?;
    let Some(output) = state.focused_output() else {
        return Ok(Vec::new());
    };
    let Some(current) = state.outputs.get(&output).map(|o| o.shown) else {
        return Ok(Vec::new());
    };
    if current == index {
        return Ok(Vec::new());
    }
    let other = state.output_showing(index);
    if let Some(o) = other.and_then(|id| state.outputs.get_mut(&id)) {
        o.shown = current;
    }
    if let Some(o) = state.outputs.get_mut(&output) {
        o.shown = index;
    }
    let next = state.workspace(index).and_then(|ws| ws.preferred_focus());
    state.set_focus(next);
    Ok(Vec::new())
}
