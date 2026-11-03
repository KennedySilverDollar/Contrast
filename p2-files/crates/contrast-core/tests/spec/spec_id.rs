//! P2 仕様: ID の払い出しと ExternalKey ⇔ ID の対応表。
//! 公開 API だけで書く（feature や cfg に依存しない）。
use contrast_core::{CoreError, EntityId, ExternalKey, IdGen, IdKind, KeyMap, OutputId, WindowId, WorkspaceIndex};

fn key(n: u64) -> ExternalKey {
    ExternalKey::new(n)
}

/// 対応表が内部で矛盾していないことを、公開 API だけで確認する。
fn assert_consistent(map: &KeyMap<WindowId>, keys: &[u64], ids: &[WindowId]) {
    let mut forward = 0;
    for &k in keys {
        if let Some(id) = map.id_of(key(k)) {
            forward += 1;
            assert_eq!(map.key_of(id), Some(key(k)), "key→id→key が一致する");
        }
    }
    for &id in ids {
        if let Some(k) = map.key_of(id) {
            assert_eq!(map.id_of(k), Some(id), "id→key→id が一致する");
        }
    }
    assert_eq!(map.len(), forward, "len は key 側の件数と一致する");
}

// ---- IdGen ----

#[test]
fn idgen_starts_at_one_and_increases_by_one() {
    let mut g = IdGen::<WindowId>::new();
    let a = g.next().unwrap();
    let b = g.next().unwrap();
    let c = g.next().unwrap();
    assert_eq!((a.raw(), b.raw(), c.raw()), (1, 2, 3));
    assert!(a < b && b < c, "ID は発行順に大小が付く");
}

#[test]
fn idgen_never_issues_the_same_id_twice() {
    let mut g = IdGen::<OutputId>::new();
    let mut seen = Vec::new();
    for _ in 0..100 {
        let id = g.next().unwrap();
        assert!(!seen.contains(&id));
        seen.push(id);
    }
}

#[test]
fn idgen_exhaustion_is_an_error_and_stays_one() {
    let mut g = IdGen::<WindowId>::starting_at(u64::MAX - 1);
    assert_eq!(g.next().unwrap().raw(), u64::MAX - 1);
    assert_eq!(g.next(), Err(CoreError::IdExhausted));
    assert_eq!(g.next(), Err(CoreError::IdExhausted));
}

#[test]
fn idgen_at_the_very_limit_is_exhausted_immediately() {
    let mut g = IdGen::<WindowId>::starting_at(u64::MAX);
    assert_eq!(g.next(), Err(CoreError::IdExhausted));
}

// ---- 値型 ----

#[test]
fn entity_id_distinguishes_kinds() {
    let mut wg = IdGen::<WindowId>::new();
    let mut og = IdGen::<OutputId>::new();
    let w = EntityId::Window(wg.next().unwrap());
    let o = EntityId::Output(og.next().unwrap());
    assert_ne!(w, o);
    assert_eq!(w, w);
}

#[test]
fn workspace_index_roundtrip() {
    assert_eq!(WorkspaceIndex::new(7).get(), 7);
}

// ---- KeyMap ----

#[test]
fn keymap_insert_and_lookup_both_ways() {
    let mut g = IdGen::<WindowId>::new();
    let w = g.next().unwrap();
    let mut m = KeyMap::<WindowId>::new();
    assert!(m.is_empty());
    m.insert(key(10), w).unwrap();
    assert_eq!(m.id_of(key(10)), Some(w));
    assert_eq!(m.key_of(w), Some(key(10)));
    assert_eq!(m.len(), 1);
    assert!(!m.is_empty());
}

#[test]
fn keymap_unknown_lookups_are_none() {
    let mut g = IdGen::<WindowId>::new();
    let w = g.next().unwrap();
    let m = KeyMap::<WindowId>::new();
    assert_eq!(m.id_of(key(1)), None);
    assert_eq!(m.key_of(w), None);
}

#[test]
fn keymap_duplicate_key_is_rejected_and_changes_nothing() {
    let mut g = IdGen::<WindowId>::new();
    let (w1, w2) = (g.next().unwrap(), g.next().unwrap());
    let mut m = KeyMap::<WindowId>::new();
    m.insert(key(1), w1).unwrap();
    let before = m.clone();
    assert_eq!(m.insert(key(1), w2), Err(CoreError::DuplicateKey));
    assert_eq!(m, before);
    assert_eq!(m.key_of(w2), None);
}

#[test]
fn keymap_duplicate_id_is_rejected_and_changes_nothing() {
    let mut g = IdGen::<WindowId>::new();
    let w1 = g.next().unwrap();
    let mut m = KeyMap::<WindowId>::new();
    m.insert(key(1), w1).unwrap();
    let before = m.clone();
    assert_eq!(m.insert(key(2), w1), Err(CoreError::DuplicateKey));
    assert_eq!(m, before);
    assert_eq!(m.id_of(key(2)), None);
}

#[test]
fn keymap_remove_by_key_returns_id_and_clears_both_directions() {
    let mut g = IdGen::<WindowId>::new();
    let w = g.next().unwrap();
    let mut m = KeyMap::<WindowId>::new();
    m.insert(key(5), w).unwrap();
    assert_eq!(m.remove_by_key(key(5)), Ok(w));
    assert_eq!(m.id_of(key(5)), None);
    assert_eq!(m.key_of(w), None);
    assert_eq!(m.len(), 0);
}

#[test]
fn keymap_remove_unknown_key_is_an_error_and_changes_nothing() {
    let mut g = IdGen::<WindowId>::new();
    let w = g.next().unwrap();
    let mut m = KeyMap::<WindowId>::new();
    m.insert(key(1), w).unwrap();
    let before = m.clone();
    assert_eq!(m.remove_by_key(key(99)), Err(CoreError::UnknownKey));
    assert_eq!(m, before);
    // 削除済みキーの 2 回目の削除も UnknownKey
    m.remove_by_key(key(1)).unwrap();
    assert_eq!(m.remove_by_key(key(1)), Err(CoreError::UnknownKey));
}

#[test]
fn keymap_allows_reusing_a_key_with_a_fresh_id_after_removal() {
    let mut g = IdGen::<WindowId>::new();
    let (w1, w2) = (g.next().unwrap(), g.next().unwrap());
    let mut m = KeyMap::<WindowId>::new();
    m.insert(key(1), w1).unwrap();
    m.remove_by_key(key(1)).unwrap();
    m.insert(key(1), w2).unwrap();
    assert_eq!(m.id_of(key(1)), Some(w2));
    assert_eq!(m.key_of(w1), None, "古い ID は戻らない");
}

#[test]
fn keymap_stays_consistent_over_a_mixed_sequence() {
    let mut g = IdGen::<WindowId>::new();
    let ids: Vec<WindowId> = (0..6).map(|_| g.next().unwrap()).collect();
    let keys = [1u64, 2, 3, 4];
    let mut m = KeyMap::<WindowId>::new();
    let _ = m.insert(key(1), ids[0]);
    let _ = m.insert(key(2), ids[1]);
    let _ = m.insert(key(2), ids[2]); // 重複キー
    let _ = m.insert(key(3), ids[1]); // 重複 ID
    let _ = m.remove_by_key(key(1));
    let _ = m.insert(key(1), ids[3]);
    let _ = m.remove_by_key(key(9)); // 未登録
    let _ = m.insert(key(4), ids[4]);
    assert_consistent(&m, &keys, &ids);
    assert_eq!(m.len(), 3);
}

#[test]
fn keymap_works_for_output_ids_too() {
    let mut g = IdGen::<OutputId>::new();
    let o = g.next().unwrap();
    let mut m = KeyMap::<OutputId>::new();
    m.insert(key(7), o).unwrap();
    assert_eq!(m.id_of(key(7)), Some(o));
}

// ---- エラー ----

#[test]
fn core_errors_have_display_text() {
    for e in [
        CoreError::UnknownKey,
        CoreError::DuplicateKey,
        CoreError::NoFreeWorkspace,
        CoreError::WorkspaceIndexOutOfRange,
        CoreError::NoFocusedWindow,
        CoreError::IdExhausted,
        CoreError::GeometryOverflow,
    ] {
        assert!(!format!("{e}").is_empty());
    }
}
