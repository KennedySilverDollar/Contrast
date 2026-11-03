//! OutputRemoved: 出力を除く。表示していたワークスペースは未表示になり、窓は残る（D3）。
//!
//! フォーカスがそのワークスペースにあれば、残る出力のうち OutputId 最小の表示ワークスペースの
//! 順序列先頭へ移し、なければ None にする（I4）。

use alloc::vec::Vec;

use crate::application::effect::Effect;
use crate::domain::id::ExternalKey;
use crate::domain::state::State;
use crate::error::CoreError;

/// 出力を削除する。
///
/// # Errors
/// key が未登録なら `UnknownKey`。
pub fn handle(state: &mut State, key: ExternalKey) -> Result<Vec<Effect>, CoreError> {
    let id = state.output_keys.remove_by_key(key)?;
    state.outputs.remove(&id);
    let focus_lost = state.focus.is_some_and(|f| !state.is_window_visible(f));
    if focus_lost {
        let next = state
            .outputs
            .values()
            .next()
            .and_then(|o| state.workspace(o.shown))
            .and_then(|ws| ws.windows.first().copied());
        state.set_focus(next);
    }
    Ok(Vec::new())
}
