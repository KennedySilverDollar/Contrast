//! OutputChanged: 出力の論理矩形を更新する（再配置は `Core::apply` が行う）。

use alloc::vec::Vec;

use crate::application::effect::Effect;
use crate::domain::geometry::Rect;
use crate::domain::id::ExternalKey;
use crate::domain::state::State;
use crate::error::CoreError;

/// 出力の論理矩形を変える。
///
/// # Errors
/// key が未登録なら `UnknownKey`。
pub fn handle(
    state: &mut State,
    key: ExternalKey,
    geometry: Rect,
) -> Result<Vec<Effect>, CoreError> {
    let id = state.output_keys.id_of(key).ok_or(CoreError::UnknownKey)?;
    let output = state.outputs.get_mut(&id).ok_or(CoreError::UnknownKey)?;
    output.geometry = geometry;
    Ok(Vec::new())
}
