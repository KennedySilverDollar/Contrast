//! P2 の追加 Kani ハーネス。仕様ハーネスが `WindowId` だけで確かめている I6 を、
//! `OutputId` / `WorkspaceId` と、枯渇後の継続・`len` の増減まで広げる。
use crate::domain::id::{ExternalKey, IdGen, IdKind, KeyMap, OutputId, WorkspaceId};
use crate::error::CoreError;

/// `IdGen<OutputId>` は上限付近から何回呼んでも、成功値は厳密に増え、
/// 一度 `IdExhausted` を返したら以後も `IdExhausted` のまま。
#[kani::proof]
#[kani::unwind(5)]
fn idgen_output_exhaustion_is_sticky() {
    let start: u64 = kani::any();
    kani::assume(start >= u64::MAX - 2);
    let mut g = IdGen::<OutputId>::starting_at(start);
    let mut last: Option<u64> = None;
    let mut exhausted = false;
    for _ in 0..4 {
        match g.next() {
            Ok(id) => {
                kani::assert(!exhausted, "枯渇後に成功しない");
                kani::assert(id.raw() != u64::MAX, "u64::MAX は払い出さない");
                if let Some(prev) = last {
                    kani::assert(id.raw() > prev, "厳密に増える");
                }
                last = Some(id.raw());
            }
            Err(e) => {
                kani::assert(e == CoreError::IdExhausted, "失敗は枯渇だけ");
                exhausted = true;
            }
        }
    }
    kani::assert(exhausted, "上限付近から 4 回呼べば必ず枯渇する");
}

/// `IdGen<WorkspaceId>` は `new()` で raw = 1 から始まり、`starting_at(u64::MAX)` は即座に枯渇する。
#[kani::proof]
#[kani::unwind(2)]
fn idgen_workspace_bounds() {
    let mut fresh = IdGen::<WorkspaceId>::new();
    kani::assert(fresh.next().map(IdKind::raw) == Ok(1), "最初の ID は 1");
    let mut full = IdGen::<WorkspaceId>::starting_at(u64::MAX);
    kani::assert(full.next() == Err(CoreError::IdExhausted), "上限では枯渇");
    kani::assert(full.next() == Err(CoreError::IdExhausted), "枯渇は続く");
}

/// 任意の key と id について、空の `KeyMap<OutputId>` への insert と remove_by_key は
/// 往復で元に戻り、途中も全単射を保つ。
#[kani::proof]
#[kani::unwind(2)]
fn keymap_output_roundtrip() {
    let key = ExternalKey::new(kani::any());
    let id = OutputId::from_raw(kani::any());
    let mut m = KeyMap::<OutputId>::new();
    kani::assert(m.insert(key, id) == Ok(()), "空の表への登録は成功する");
    kani::assert(m.is_bijective() && m.len() == 1, "登録後も全単射");
    kani::assert(m.remove_by_key(key) == Ok(id), "登録した ID が返る");
    kani::assert(m == KeyMap::new(), "削除後は空の表に戻る");
}

/// `KeyMap<OutputId>` への 3 回以内の任意の操作列で、全単射が保たれ、
/// `len` は成功した insert で 1 増え、成功した remove で 1 減り、Err では変わらない。
#[kani::proof]
#[kani::unwind(5)]
fn keymap_output_ops_preserve_bijection_and_len() {
    let mut g = IdGen::<OutputId>::new();
    let (Ok(o0), Ok(o1), Ok(o2)) = (g.next(), g.next(), g.next()) else {
        return;
    };
    let mut m = KeyMap::<OutputId>::new();

    for _ in 0..3 {
        let k: u8 = kani::any();
        kani::assume(k < 3); // 衝突が起きやすいようキーを小さい集合に絞る
        let key = ExternalKey::new(u64::from(k));
        let before = m.clone();

        let ok = if kani::any() {
            let i: u8 = kani::any();
            kani::assume(i < 3);
            let id = match i {
                0 => o0,
                1 => o1,
                _ => o2,
            };
            let r = m.insert(key, id);
            kani::assert(
                r.is_ok() || r == Err(CoreError::DuplicateKey),
                "insert の失敗は DuplicateKey だけ",
            );
            if r.is_ok() {
                kani::assert(Some(m.len()) == before.len().checked_add(1), "成功で 1 増える");
            }
            r.is_ok()
        } else {
            let r = m.remove_by_key(key);
            kani::assert(
                r.is_ok() || r == Err(CoreError::UnknownKey),
                "remove の失敗は UnknownKey だけ",
            );
            if r.is_ok() {
                kani::assert(Some(m.len()) == before.len().checked_sub(1), "成功で 1 減る");
            }
            r.is_ok()
        };
        if !ok {
            kani::assert(m == before, "Err なら不変");
        }
        kani::assert(m.is_bijective(), "全単射（I6）");
    }
}
