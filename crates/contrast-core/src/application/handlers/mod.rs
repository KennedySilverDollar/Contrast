//! ユースケースごとの handler（1 ファイル 1 ユースケース）。
//!
//! handler は複製された `State` を `&mut` で受け、Registered / RequestClose だけを返す。
//! 再配置と残りの Effect は `Core::apply` が行う。Err を返したら複製は捨てられる（I7）。

mod output_added;
mod output_changed;
mod output_removed;
mod window_mapped;
mod window_meta_changed;
mod window_unmapped;

use alloc::vec::Vec;

use crate::application::command::Command;
use crate::application::effect::Effect;
use crate::domain::id::WorkspaceIndex;
use crate::domain::state::State;
use crate::domain::window::WindowState;
use crate::error::CoreError;
use crate::policy::Policy;

/// Command を対応する handler に振り分ける。
///
/// # Errors
/// 各 handler の `CoreError`。
pub fn dispatch<P: Policy>(
    state: &mut State,
    policy: &P,
    command: Command,
) -> Result<Vec<Effect>, CoreError> {
    match command {
        Command::OutputAdded {
            key,
            name,
            geometry,
        } => output_added::handle(state, key, name, geometry),
        Command::OutputRemoved(key) => output_removed::handle(state, key),
        Command::OutputChanged { key, geometry } => output_changed::handle(state, key, geometry),
        Command::WindowMapped { key, meta } => window_mapped::handle(state, policy, key, meta),
        Command::WindowUnmapped(key) => window_unmapped::handle(state, key),
        Command::WindowMetaChanged { key, meta } => window_meta_changed::handle(state, key, meta),
        // C2 で実装する。
        Command::WindowFocusRequested(_) | Command::Action(_) => Ok(Vec::new()),
    }
}

/// `index` が範囲内か確かめる。
///
/// # Errors
/// `workspace_count` 以上なら `WorkspaceIndexOutOfRange`。
fn check_index(state: &State, index: WorkspaceIndex) -> Result<(), CoreError> {
    if index.get() < state.workspace_count() {
        Ok(())
    } else {
        Err(CoreError::WorkspaceIndexOutOfRange)
    }
}

/// D9: ワークスペース `index` の全画面窓のうち `except` 以外を元の状態に戻す。
fn demote_other_fullscreen(
    state: &mut State,
    index: WorkspaceIndex,
    except: crate::domain::id::WindowId,
) {
    let ids = state
        .workspace(index)
        .map(|ws| ws.windows.clone())
        .unwrap_or_default();
    for id in ids {
        if id == except {
            continue;
        }
        let fullscreen = state
            .windows
            .get_mut(&id)
            .filter(|w| w.state == WindowState::Fullscreen);
        if let Some(w) = fullscreen {
            w.state = w.restore_state;
        }
    }
}
