//! P2 の Kani 仕様ハーネス。ID の払い出しと KeyMap の全単射（I6）。
use crate::domain::id::{ExternalKey, IdGen, IdKind, KeyMap, WindowId};
use crate::error::CoreError;

/// IdGen::next は任意の開始値でパニックせず、成功時は開始値をそのまま返す。
/// 失敗は開始値が u64::MAX のときだけで、そのときは IdExhausted。
#[kani::proof]
fn idgen_next_never_panics() {
    let start: u64 = kani::any();
    let mut g = IdGen::<WindowId>::starting_at(start);
    match g.next() {
        Ok(id) => kani::assert(id.raw() == start && start != u64::MAX, "開始値を返す"),
        Err(e) => kani::assert(
            e == CoreError::IdExhausted && start == u64::MAX,
            "枯渇だけが失敗",
        ),
    }
}

/// 連続 2 回の発行は厳密に増え、ちょうど 1 だけ進む。
#[kani::proof]
fn idgen_is_strictly_monotone() {
    let start: u64 = kani::any();
    let mut g = IdGen::<WindowId>::starting_at(start);
    if let (Ok(a), Ok(b)) = (g.next(), g.next()) {
        kani::assert(b.raw() == a.raw() + 1, "1 ずつ増える");
    }
}

/// 3 回以内の insert / remove の任意列で、全単射が保たれ、
/// Err を返した操作は状態を変えない。
#[kani::proof]
#[kani::unwind(5)]
fn keymap_ops_preserve_bijection() {
    let mut g = IdGen::<WindowId>::new();
    let (Ok(w0), Ok(w1), Ok(w2)) = (g.next(), g.next(), g.next()) else {
        return;
    };
    let pool = [w0, w1, w2];
    let mut m = KeyMap::<WindowId>::new();

    for _ in 0..3 {
        let is_insert: bool = kani::any();
        let k: u8 = kani::any();
        kani::assume(k < 3); // キーを小さい集合に絞り、衝突を起こしやすくする
        let key = ExternalKey::new(u64::from(k));
        let before = m.clone();

        if is_insert {
            let i: usize = kani::any();
            kani::assume(i < 3);
            let id = pool[i];
            match m.insert(key, id) {
                Ok(()) => {
                    kani::assert(m.id_of(key) == Some(id), "insert 後に key→id");
                    kani::assert(m.key_of(id) == Some(key), "insert 後に id→key");
                }
                Err(_) => kani::assert(m == before, "Err なら不変"),
            }
        } else {
            match m.remove_by_key(key) {
                Ok(id) => {
                    kani::assert(m.id_of(key).is_none(), "remove 後に key が消える");
                    kani::assert(m.key_of(id).is_none(), "remove 後に id が消える");
                }
                Err(_) => kani::assert(m == before, "Err なら不変"),
            }
        }
        kani::assert(m.is_bijective(), "全単射（I6）");
    }
}
