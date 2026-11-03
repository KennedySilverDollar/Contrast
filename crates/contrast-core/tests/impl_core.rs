//! Core の実装テスト: 出力・窓の追加削除、Effect 列の順序、Action の基本動作。
//!
//! 各操作の後に `check_invariants`（I1〜I6）が Ok であることを確かめる。
//! `check_invariants` は `invariants` feature で公開されるので、そのときだけ動かす。

/// `expect` を補助関数でも使えるよう、テスト一式を `cfg(test)` モジュールに入れる。
#[cfg(test)]
#[cfg(feature = "invariants")]
mod core_impl {
    use contrast_core::{
        Columns, Command, Core, CoreConfig, CoreError, DefaultPolicy, Effect, EntityId,
        ExternalKey, OutputId, OutputName, Rect, WindowId, WindowMeta, WindowState,
    };

    /// テスト対象の Core（コンポジタと同じくワークスペース 4 個）。
    type TestCore = Core<DefaultPolicy, Columns>;

    /// 空の Core を作る。
    fn core() -> TestCore {
        Core::new(CoreConfig::new(4), DefaultPolicy::new(), Columns::new())
    }

    /// 矩形を作る。
    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect::new(x, y, w, h).expect("valid rect")
    }

    /// 出力のキー。
    const OUT: ExternalKey = ExternalKey::new(1000);

    /// 窓のキー。
    fn win(n: u64) -> ExternalKey {
        ExternalKey::new(n)
    }

    /// apply して不変条件を確かめ、Effect 列を返す。
    fn apply(core: &mut TestCore, cmd: Command) -> Vec<Effect> {
        let applied = core.apply(cmd).expect("apply ok");
        core.check_invariants().expect("invariants hold");
        applied.effects
    }

    /// 928x1076 の出力を 1 つ足した Core。
    fn core_with_output() -> TestCore {
        let mut c = core();
        apply(
            &mut c,
            Command::OutputAdded {
                key: OUT,
                name: OutputName::new("winit"),
                geometry: rect(0, 0, 928, 1076),
            },
        );
        c
    }

    /// 通常の窓を map する。
    fn map(core: &mut TestCore, n: u64) -> Vec<Effect> {
        apply(
            core,
            Command::WindowMapped {
                key: win(n),
                meta: WindowMeta::default(),
            },
        )
    }

    /// キーから ID を引く。
    fn id(core: &TestCore, n: u64) -> WindowId {
        core.view().window_id(win(n)).expect("registered")
    }

    /// 窓の現在の矩形。
    fn rect_of(core: &TestCore, n: u64) -> Rect {
        core.view()
            .window(id(core, n))
            .and_then(|w| w.rect)
            .expect("has rect")
    }

    #[test]
    fn output_added_shows_workspace_0() {
        let mut c = core();
        let effects = apply(
            &mut c,
            Command::OutputAdded {
                key: OUT,
                name: OutputName::new("winit"),
                geometry: rect(0, 0, 928, 1076),
            },
        );
        let out = c.view().output_id(OUT).expect("output registered");
        assert!(matches!(
            effects.first(),
            Some(Effect::Registered {
                id: EntityId::Output(_),
                ..
            })
        ));
        assert!(
            matches!(effects.get(1), Some(Effect::ShowWorkspace { output, .. }) if *output == out)
        );
        assert_eq!(effects.last(), Some(&Effect::Damage(out)));
        assert_eq!(c.view().shown_workspace(out).map(|i| i.get()), Some(0));
    }

    #[test]
    fn windows_split_columns_and_focus_newest() {
        let mut c = core_with_output();
        map(&mut c, 1);
        assert_eq!(rect_of(&c, 1), rect(0, 0, 928, 1076));
        map(&mut c, 2);
        assert_eq!(rect_of(&c, 1), rect(0, 0, 464, 1076));
        assert_eq!(rect_of(&c, 2), rect(464, 0, 464, 1076));
        map(&mut c, 3);
        assert_eq!(rect_of(&c, 1), rect(0, 0, 310, 1076));
        assert_eq!(rect_of(&c, 2), rect(310, 0, 309, 1076));
        assert_eq!(rect_of(&c, 3), rect(619, 0, 309, 1076));
        assert_eq!(c.view().focus(), Some(id(&c, 3)));
    }

    #[test]
    fn effect_order_is_fixed() {
        let mut c = core_with_output();
        map(&mut c, 1);
        let effects = map(&mut c, 2);
        let (w1, w2) = (id(&c, 1), id(&c, 2));
        let out: OutputId = c.view().output_id(OUT).expect("output");
        assert_eq!(
            effects,
            vec![
                Effect::Registered {
                    key: win(2),
                    id: EntityId::Window(w2)
                },
                Effect::Configure {
                    window: w1,
                    rect: rect(0, 0, 464, 1076),
                    state: WindowState::Tiled
                },
                Effect::Configure {
                    window: w2,
                    rect: rect(464, 0, 464, 1076),
                    state: WindowState::Tiled
                },
                Effect::SetFocus(Some(w2)),
                Effect::Damage(out),
            ]
        );
    }

    #[test]
    fn unmapping_focused_window_focuses_neighbor() {
        let mut c = core_with_output();
        map(&mut c, 1);
        map(&mut c, 2);
        map(&mut c, 3);
        // 末尾の窓を閉じたら、新しい末尾（左隣）へ。
        apply(&mut c, Command::WindowUnmapped(win(3)));
        assert_eq!(c.view().focus(), Some(id(&c, 2)));
        assert_eq!(rect_of(&c, 1), rect(0, 0, 464, 1076));
        assert_eq!(rect_of(&c, 2), rect(464, 0, 464, 1076));
        apply(&mut c, Command::WindowUnmapped(win(2)));
        assert_eq!(c.view().focus(), Some(id(&c, 1)));
        apply(&mut c, Command::WindowUnmapped(win(1)));
        assert_eq!(c.view().focus(), None);
    }

    #[test]
    fn unmapping_unfocused_window_keeps_focus() {
        let mut c = core_with_output();
        map(&mut c, 1);
        map(&mut c, 2);
        apply(&mut c, Command::WindowUnmapped(win(1)));
        assert_eq!(c.view().focus(), Some(id(&c, 2)));
    }

    #[test]
    fn errors_leave_state_unchanged() {
        let mut c = core_with_output();
        map(&mut c, 1);
        let before = c.snapshot();
        let dup = c.apply(Command::WindowMapped {
            key: win(1),
            meta: WindowMeta::default(),
        });
        assert_eq!(dup, Err(CoreError::DuplicateKey));
        assert_eq!(
            c.apply(Command::WindowUnmapped(win(99))),
            Err(CoreError::UnknownKey)
        );
        assert_eq!(
            c.apply(Command::OutputChanged {
                key: win(99),
                geometry: rect(0, 0, 10, 10)
            }),
            Err(CoreError::UnknownKey)
        );
        assert_eq!(c.snapshot(), before);
    }

    #[test]
    fn window_mapped_without_output_is_not_configured() {
        let mut c = core();
        let effects = map(&mut c, 1);
        assert_eq!(effects.len(), 1);
        assert_eq!(c.view().focus(), None);
        let w = c.view().window(id(&c, 1)).expect("window");
        assert_eq!(w.workspace.get(), 0);
        assert!(!w.visible);
        // 出力が来たら表示されて Configure される。
        let effects = apply(
            &mut c,
            Command::OutputAdded {
                key: OUT,
                name: OutputName::new("winit"),
                geometry: rect(0, 0, 800, 600),
            },
        );
        assert!(
            effects.iter().any(
                |e| matches!(e, Effect::Configure { rect: r, .. } if *r == rect(0, 0, 800, 600))
            )
        );
    }

    #[test]
    fn output_resize_reconfigures_windows() {
        let mut c = core_with_output();
        map(&mut c, 1);
        map(&mut c, 2);
        let effects = apply(
            &mut c,
            Command::OutputChanged {
                key: OUT,
                geometry: rect(0, 0, 1000, 500),
            },
        );
        let configures = effects
            .iter()
            .filter(|e| matches!(e, Effect::Configure { .. }))
            .count();
        assert_eq!(configures, 2);
        assert_eq!(rect_of(&c, 2), rect(500, 0, 500, 500));
    }

    #[test]
    fn output_removed_drops_focus_when_no_output_left() {
        let mut c = core_with_output();
        map(&mut c, 1);
        apply(&mut c, Command::OutputRemoved(OUT));
        assert_eq!(c.view().focus(), None);
        assert!(!c.view().window(id(&c, 1)).expect("window").visible);
    }

    #[test]
    fn dialog_floats_centered_at_half_size() {
        let mut c = core_with_output();
        map(&mut c, 1);
        let mut meta = WindowMeta::default();
        meta.is_dialog = true;
        apply(&mut c, Command::WindowMapped { key: win(2), meta });
        let w = c.view().window(id(&c, 2)).expect("window");
        assert_eq!(w.state, WindowState::Floating);
        assert_eq!(w.rect, Some(rect(232, 269, 464, 538)));
        // Tiled 窓は 1 枚分のまま。
        assert_eq!(rect_of(&c, 1), rect(0, 0, 928, 1076));
    }
}
