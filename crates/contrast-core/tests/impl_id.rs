//! id の実装テスト: 仕様テストが触れていない境界と、proptest による I6 の確認。

/// `expect` を補助関数でも使えるよう、テスト一式を `cfg(test)` モジュールに入れる。
#[cfg(test)]
mod id {
    use std::collections::BTreeSet;

    use contrast_core::{
        CoreError, ExternalKey, IdGen, IdKind, KeyMap, LayoutError, OutputId, WindowId,
        WorkspaceId, WorkspaceIndex,
    };
    use proptest::prelude::*;

    /// テスト用のキーを作る。
    fn key(n: u64) -> ExternalKey {
        ExternalKey::new(n)
    }

    /// 1000 回払い出しても重複がなく、raw は 1..=1000 を順に埋める。
    #[test]
    fn idgen_thousand_ids_are_unique_and_dense() {
        let mut g = IdGen::<WindowId>::new();
        let mut seen = BTreeSet::new();
        for expected in 1..=1000u64 {
            let id = g.next().expect("上限から遠い");
            assert_eq!(id.raw(), expected);
            assert!(seen.insert(id), "重複なし");
        }
        assert_eq!(seen.len(), 1000);
    }

    /// `WorkspaceId` の払い出し器も 1 から始まる。
    #[test]
    fn idgen_workspace_starts_at_one() {
        let mut g = IdGen::<WorkspaceId>::new();
        assert_eq!(g.next().map(IdKind::raw), Ok(1));
        assert_eq!(g.next().map(IdKind::raw), Ok(2));
    }

    /// `starting_at(0)` は raw = 0 を払い出せる（テスト用の入口なので値を制限しない）。
    #[test]
    fn idgen_starting_at_zero_issues_zero() {
        let mut g = IdGen::<OutputId>::starting_at(0);
        assert_eq!(g.next().map(IdKind::raw), Ok(0));
        assert_eq!(g.next().map(IdKind::raw), Ok(1));
    }

    /// `u64::MAX - 1` までは払い出せて、その後は枯渇が続く。
    #[test]
    fn idgen_last_issuable_value_is_max_minus_one() {
        let mut g = IdGen::<OutputId>::starting_at(u64::MAX - 2);
        assert_eq!(g.next().map(IdKind::raw), Ok(u64::MAX - 2));
        assert_eq!(g.next().map(IdKind::raw), Ok(u64::MAX - 1));
        for _ in 0..3 {
            assert_eq!(g.next(), Err(CoreError::IdExhausted));
        }
    }

    /// `Default` は `new()` と同じ。
    #[test]
    fn defaults_match_new() {
        assert_eq!(IdGen::<WindowId>::default(), IdGen::<WindowId>::new());
        assert_eq!(KeyMap::<WindowId>::default(), KeyMap::<WindowId>::new());
    }

    /// `from_raw` と `raw` は往復する。
    #[test]
    fn id_raw_roundtrip() {
        for raw in [0, 1, 42, u64::MAX] {
            assert_eq!(OutputId::from_raw(raw).raw(), raw);
            assert_eq!(WindowId::from_raw(raw).raw(), raw);
            assert_eq!(WorkspaceId::from_raw(raw).raw(), raw);
        }
    }

    /// `WorkspaceIndex` は u8 の全域で往復し、順序は値の順。
    #[test]
    fn workspace_index_full_range() {
        for n in 0..=u8::MAX {
            assert_eq!(WorkspaceIndex::new(n).get(), n);
        }
        assert!(WorkspaceIndex::new(0) < WorkspaceIndex::new(u8::MAX));
    }

    /// 同じキーの登録と削除を繰り返しても、len は 0 と 1 を往復し、古い ID は残らない。
    #[test]
    fn keymap_repeated_insert_remove_same_key() {
        let mut g = IdGen::<WindowId>::new();
        let mut m = KeyMap::<WindowId>::new();
        let mut old = Vec::new();
        for _ in 0..100 {
            let w = g.next().expect("上限から遠い");
            m.insert(key(1), w).expect("キーは空いている");
            assert_eq!(m.len(), 1);
            assert_eq!(m.remove_by_key(key(1)), Ok(w));
            assert_eq!(m.len(), 0);
            assert!(m.is_empty());
            old.push(w);
        }
        assert!(old.iter().all(|&w| m.key_of(w).is_none()));
    }

    /// 同じ (key, id) の 2 回目の登録も `DuplicateKey`。
    #[test]
    fn keymap_same_pair_twice_is_duplicate() {
        let w = WindowId::from_raw(1);
        let mut m = KeyMap::<WindowId>::new();
        m.insert(key(1), w).expect("空の表");
        assert_eq!(m.insert(key(1), w), Err(CoreError::DuplicateKey));
        assert_eq!(m.len(), 1);
    }

    /// 端の値（0 と `u64::MAX`）もキーとして使える。
    #[test]
    fn keymap_accepts_extreme_keys() {
        let mut m = KeyMap::<OutputId>::new();
        m.insert(key(0), OutputId::from_raw(1)).expect("空の表");
        m.insert(key(u64::MAX), OutputId::from_raw(2))
            .expect("別のキーと別の ID");
        assert_eq!(m.id_of(key(0)), Some(OutputId::from_raw(1)));
        assert_eq!(m.id_of(key(u64::MAX)), Some(OutputId::from_raw(2)));
        assert_eq!(m.len(), 2);
    }

    /// `LayoutError` は `From` で `CoreError::Layout` に包まれる。
    #[test]
    fn layout_error_converts() {
        assert_eq!(
            CoreError::from(LayoutError::AreaTooSmall),
            CoreError::Layout(LayoutError::AreaTooSmall)
        );
    }

    /// 8 variant すべての表示文字列は空でなく、互いに異なる。`core::error::Error` としても使える。
    #[test]
    fn core_error_display_is_distinct_and_error_trait() {
        let all = [
            CoreError::UnknownKey,
            CoreError::DuplicateKey,
            CoreError::NoFreeWorkspace,
            CoreError::WorkspaceIndexOutOfRange,
            CoreError::NoFocusedWindow,
            CoreError::IdExhausted,
            CoreError::GeometryOverflow,
            CoreError::Layout(LayoutError::AreaTooSmall),
        ];
        let texts: BTreeSet<String> = all.iter().map(ToString::to_string).collect();
        assert!(texts.iter().all(|t| !t.is_empty()));
        assert_eq!(texts.len(), all.len());
        let e: &dyn core::error::Error = &CoreError::IdExhausted;
        assert!(e.source().is_none());
    }

    /// 表に対する 1 操作。
    #[derive(Debug, Clone)]
    enum Op {
        /// key と id の raw 値で登録する。
        Insert(u64, u64),
        /// key の raw 値で削除する。
        Remove(u64),
    }

    /// 衝突が起きやすいよう、key と id を小さい範囲から選ぶ。
    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            (0u64..6, 0u64..6).prop_map(|(k, i)| Op::Insert(k, i)),
            (0u64..6).prop_map(Op::Remove),
        ]
    }

    proptest! {
        /// 任意の操作列で、len と逆引きが参照モデル（key → id の BTreeMap）と一致し、
        /// Err を返した操作は表を変えない（I6）。
        #[test]
        fn keymap_matches_reference_model(ops in prop::collection::vec(op(), 0..64)) {
            let mut m = KeyMap::<WindowId>::new();
            let mut model = std::collections::BTreeMap::<u64, u64>::new();
            for o in ops {
                let before = m.clone();
                match o {
                    Op::Insert(k, i) => {
                        let dup = model.contains_key(&k) || model.values().any(|&v| v == i);
                        let r = m.insert(key(k), WindowId::from_raw(i));
                        if dup {
                            prop_assert_eq!(r, Err(CoreError::DuplicateKey));
                            prop_assert_eq!(&m, &before);
                        } else {
                            prop_assert_eq!(r, Ok(()));
                            model.insert(k, i);
                        }
                    }
                    Op::Remove(k) => {
                        let r = m.remove_by_key(key(k));
                        match model.remove(&k) {
                            Some(i) => prop_assert_eq!(r, Ok(WindowId::from_raw(i))),
                            None => {
                                prop_assert_eq!(r, Err(CoreError::UnknownKey));
                                prop_assert_eq!(&m, &before);
                            }
                        }
                    }
                }
                prop_assert_eq!(m.len(), model.len());
                for (&k, &i) in &model {
                    prop_assert_eq!(m.id_of(key(k)), Some(WindowId::from_raw(i)));
                    prop_assert_eq!(m.key_of(WindowId::from_raw(i)), Some(key(k)));
                }
                #[cfg(feature = "invariants")]
                prop_assert!(m.is_bijective());
            }
        }

        /// 任意の開始値から 2 回払い出すと、成功値は 1 ずつ増え、失敗は枯渇だけ。
        #[test]
        fn idgen_two_steps_from_any_start(start in any::<u64>()) {
            let mut g = IdGen::<WorkspaceId>::starting_at(start);
            let a = g.next();
            let b = g.next();
            match (a, b) {
                (Ok(a), Ok(b)) => prop_assert_eq!(a.raw().checked_add(1), Some(b.raw())),
                (Ok(a), Err(e)) => {
                    prop_assert_eq!(a.raw(), u64::MAX - 1);
                    prop_assert_eq!(e, CoreError::IdExhausted);
                }
                (Err(e1), Err(e2)) => {
                    prop_assert_eq!(start, u64::MAX);
                    prop_assert_eq!((e1, e2), (CoreError::IdExhausted, CoreError::IdExhausted));
                }
                (Err(_), Ok(_)) => prop_assert!(false, "枯渇後に成功した"),
            }
        }
    }
}
