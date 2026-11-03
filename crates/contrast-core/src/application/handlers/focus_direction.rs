//! FocusDirection: フォーカス窓の矩形中心から見てその方向にある、最も近い表示中の窓へ移る（D10）。
//!
//! 距離は中心間のマンハッタン距離、同距離は WindowId 最小。候補なしは状態不変で `Ok`。

use alloc::vec::Vec;

use crate::application::command::Direction;
use crate::application::effect::Effect;
use crate::application::handlers::focused;
use crate::domain::geometry::Point;
use crate::domain::state::State;
use crate::error::CoreError;

/// 方向にフォーカスを移す。
///
/// # Errors
/// フォーカスが None なら `NoFocusedWindow`。
pub fn handle(state: &mut State, direction: Direction) -> Result<Vec<Effect>, CoreError> {
    let id = focused(state)?;
    let Some(from) = state
        .windows
        .get(&id)
        .and_then(|w| w.rect)
        .map(|r| r.center())
    else {
        return Ok(Vec::new());
    };
    let target = state
        .windows
        .values()
        .filter(|w| w.id != id && state.is_window_visible(w.id))
        .filter_map(|w| w.rect.map(|r| (w.id, r.center())))
        .filter(|(_, c)| is_toward(from, *c, direction))
        .min_by_key(|(wid, c)| (manhattan(from, *c), *wid))
        .map(|(wid, _)| wid);
    if target.is_some() {
        state.set_focus(target);
    }
    Ok(Vec::new())
}

/// `to` が `from` から見て `direction` の側にあるか。
const fn is_toward(from: Point, to: Point, direction: Direction) -> bool {
    match direction {
        Direction::Left => to.x < from.x,
        Direction::Right => to.x > from.x,
        Direction::Up => to.y < from.y,
        Direction::Down => to.y > from.y,
    }
}

/// 2 点のマンハッタン距離。
fn manhattan(a: Point, b: Point) -> u64 {
    u64::from(a.x.abs_diff(b.x)).saturating_add(u64::from(a.y.abs_diff(b.y)))
}
