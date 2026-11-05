# contrast-core 仕様 v0.1

contrast-core は Contrast の純粋な中核クレート。Smithay・I/O・CDL のどれにも依存しない
決定的な状態遷移エンジンで、構成は「関数型コア／命令型シェル」。
Smithay 側（シェル）は Wayland イベントを `Command` に翻訳して渡し、
core が返す `Effect` を順に実行するだけにする。

公開 API の中心は全関数 `Core::apply(Command) -> Result<Applied, CoreError>` の 1 本。
不変条件は常に成り立ち、Err を返した Command は状態を一切変えない。core は決してパニックしない。

> 本書は Contrast 仕様 v0.6 と食い違う場合、v0.6 を正として本書を直す。

## 1. 責務と非責務

責務:
- 出力（モニタ）・ワークスペース・窓の所有関係の管理
- レイアウト計算（各窓の矩形の決定）
- フォーカスとナビゲーション
- 意味操作 `Action`（キーバインド解決後）の解釈
- ポリシーへの問い合わせ（初期配置、浮動判定など）と結果の適用

core に入れないもの: Wayland プロトコル、バッファ、描画、DRM / libinput（contrast-backend-smithay）、
CDL のパースと評価（contrast-policy-cdl）、キーシンボル→Action 変換、設定ファイル読み込み、
ログ出力、時刻、乱数。

## 2. クレート構成と依存方向

```text
contrast (bin) ──┬──> contrast-policy-cdl ──> cdl（別リポジトリ）
                 │            │
                 │            v
                 ├──────> contrast-core   （依存クレート 0, no_std）
                 │            ^
                 │            │
                 └──> contrast-backend-smithay ──> smithay
```

矢印は「依存する」向き。contrast-core はどこにも依存しない。

## 3. ドメインモデル

| 型 | 意味 | 主な属性 |
| --- | --- | --- |
| `Output` / `OutputId` | 物理または仮想モニタ | 論理矩形、作業領域、表示中ワークスペース |
| `Workspace` / `WorkspaceId` / `WorkspaceIndex` | 窓の集合と並び | インデックス、窓の順序列、フォーカス履歴 |
| `Window` / `WindowId` | トップレベル窓 | `WindowMeta`、状態（Tiled / Floating / Fullscreen）、浮動時の矩形 |
| `Point` / `Size` / `Rect` | 論理座標の幾何 | i32 座標、幅と高さは 1 以上 |
| `ExternalKey` | シェル側の識別子 | 不透明な u64。core は ID と対応づけるだけ |
| `EntityId` | 登録結果 | `Output(OutputId)` / `Window(WindowId)` |
| `Command` / `Action` | core への入力 | 5 節参照 |
| `Effect` | core からの出力 | 5 節参照 |

- ID は u64 の newtype。`IdGen` が単調増加で払い出し、再利用しない。上限到達は `Err(IdExhausted)`。
- コレクションは反復順が決定的な `BTreeMap` / `BTreeSet` / `Vec` だけを使う。
- `WindowMeta { app_id: Option<String>, title: Option<String>, is_dialog: bool, requested_size: Option<Size> }`
- `CoreConfig { workspace_count: u8 }`（1〜32、既定 9）。範囲外は `Core::new` が 1〜32 に丸める。

### ID と対応表（P2 で確定）

- `IdKind` トレイト（`from_raw` / `raw`）を 3 つの ID 型（OutputId / WindowId / WorkspaceId）が実装する。
- `IdGen<T>::new()` は raw = 1 から払い出す。`starting_at(n)` はテストと Kani 用。
  現在値が `u64::MAX` のとき `Err(IdExhausted)`（以後も同じ）。
- `KeyMap<T>` は key と id の双方向表。`insert` は key か id の重複で `Err(DuplicateKey)`、
  `remove_by_key` は未登録で `Err(UnknownKey)`。Err のとき表は変わらない。

## 4. 不変条件

`check_invariants()` が I1〜I6 を検査し、テストと Kani の両方から呼ぶ。I7・I8 は状態機械テストで確認する。

- **I1** 窓はちょうど 1 つのワークスペースに属する。
- **I2** ワークスペースの窓順序列に重複がなく、列中の ID はすべて存在する窓を指す。
- **I3** 各出力は表示中ワークスペースをちょうど 1 つ持ち、1 つのワークスペースが 2 つの出力に同時表示されることはない。
- **I4** フォーカスは None か、いずれかの出力に表示中のワークスペース上の窓を指す。
- **I5** 表示中ワークスペースの Tiled 窓の矩形は、その出力の作業領域内に収まり、互いに重ならない。
- **I6** ID は単調増加で再利用しない。`ExternalKey` と ID の対応は全単射。
- **I7** `apply` が Err を返したとき、状態は呼び出し前と等しい（強い例外保証）。
- **I8** 同じ状態と同じ Command からは、同じ状態と同じ Effect 列が得られる（決定性）。

I7 は「状態を複製して新状態を組み立て、成功時だけ差し替える」実装で満たす。

## 5. 公開 API（v0.1）

```rust
pub struct Core<P: Policy, L: Layout> { /* private */ }

impl<P: Policy, L: Layout> Core<P, L> {
    pub fn new(config: CoreConfig, policy: P, layout: L) -> Self;
    pub fn apply(&mut self, cmd: Command) -> Result<Applied, CoreError>;
    pub fn view(&self) -> StateView<'_>; // 読み取り専用
    #[cfg(any(test, kani, feature = "invariants"))]
    pub fn check_invariants(&self) -> Result<(), InvariantViolation>;
    #[cfg(any(test, kani, feature = "invariants"))]
    pub fn snapshot(&self) -> StateSnapshot; // PartialEq + Debug。I7 の比較用
}

pub struct Applied { pub effects: Vec<Effect> }

#[non_exhaustive]
pub enum Command {
    OutputAdded { key: ExternalKey, name: OutputName, geometry: Rect },
    OutputRemoved(ExternalKey),
    OutputChanged { key: ExternalKey, geometry: Rect },
    WindowMapped { key: ExternalKey, meta: WindowMeta },
    WindowUnmapped(ExternalKey),
    WindowMetaChanged { key: ExternalKey, meta: WindowMeta },
    Action(Action),
}

#[non_exhaustive]
pub enum Action {
    FocusDirection(Direction),
    MoveWindow(Direction),
    SwitchWorkspace(WorkspaceIndex),
    MoveToWorkspace(WorkspaceIndex),
    ToggleFloating,
    ToggleFullscreen,
    CloseFocused,
}

pub enum Direction { Left, Right, Up, Down }

#[non_exhaustive]
pub enum Effect {
    Registered { key: ExternalKey, id: EntityId },
    ShowWorkspace { output: OutputId, workspace: WorkspaceId },
    Configure { window: WindowId, rect: Rect, state: WindowState },
    SetFocus(Option<WindowId>),
    RequestClose(WindowId),
    Damage(OutputId),
}

pub trait Policy {
    fn place(&self, meta: &WindowMeta, ctx: &PlacementContext<'_>) -> Placement;
}

pub struct Placement {
    pub workspace: Option<WorkspaceIndex>, // None = フォーカス出力の表示ワークスペース
    pub state: WindowState,
    pub focus: bool,
}

pub trait Layout {
    fn arrange(&self, area: Rect, windows: &[WindowId], out: &mut Vec<(WindowId, Rect)>)
        -> Result<(), LayoutError>;
}
```

`PlacementContext` は読み取り専用で、フォーカス中のワークスペースインデックス、ワークスペース数、出力数を持つ。

### Effect 列の順序（固定）

1. Registered
2. ShowWorkspace（OutputId 昇順）
3. Configure（WindowId 昇順）
4. SetFocus（フォーカスが変わったときだけ、最大 1 個）
5. RequestClose
6. Damage（OutputId 昇順、重複なし）

Configure は「表示中ワークスペースにあり、矩形か状態が変わった窓」にだけ出す。

## 6. エラー

`CoreError` は `#[non_exhaustive]` の enum。v0.1 で許す variant は次がすべて。

| variant | 発生条件 |
| --- | --- |
| `UnknownKey` | 登録されていない `ExternalKey` への操作 |
| `DuplicateKey` | 登録済み `ExternalKey` の再登録 |
| `NoFreeWorkspace` | 出力追加時に未表示のワークスペースが残っていない |
| `WorkspaceIndexOutOfRange` | `WorkspaceIndex` が `workspace_count` 以上 |
| `NoFocusedWindow` | 対象窓が必要な Action でフォーカスが None |
| `IdExhausted` | ID の上限到達 |
| `GeometryOverflow` | 幾何計算が i32 に収まらない |
| `Layout(LayoutError)` | レイアウト計算の失敗 |

`UnknownKey` はシェル側の競合（閉じかけの窓への操作など）で普通に起こる。シェルはログだけ出して続行する。
`LayoutError` は `AreaTooSmall` のみ。

## 7. 暫定決定（v0.6 との突き合わせまで有効）

実装者は推測で埋めず、ここに従う。変更は人間だけが行う。

- **D1** ワークスペースは全出力で共有するグローバル集合。`Core::new` が `workspace_count` 個を生成し、増減しない。
- **D2** 出力追加時、その出力は「未表示のワークスペースのうち最小インデックス」を表示する。なければ `NoFreeWorkspace`。
- **D3** 出力削除時、その出力が表示していたワークスペースは未表示になる。窓はワークスペースに残る。フォーカスがそこにあれば、残る出力のうち OutputId 最小の表示ワークスペースの順序列先頭の窓へ移り、なければ None。
- **D4** 作業領域は出力の論理矩形と同じ（layer-shell による予約は v0.1 では扱わない）。
- **D5** フォーカス出力 = フォーカス窓のある出力。フォーカスが None なら OutputId 最小の出力。出力 0 台ならなし。
- **D6** 出力 0 台でも `WindowMapped` は成功する。配置先は `Placement.workspace`、None ならワークスペース 0。表示されていないので Configure は出さない。
- **D7** 新しい窓は `Placement.focus` が true で、かつ配置先が表示中ならフォーカスを得る。順序列では末尾に追加する。
- **D8** Floating 窓は I5 の対象外（重なり可）。矩形は `requested_size`（なければ作業領域の半分）を作業領域中央に置き、出力の論理矩形と最低 1px 交差するよう切り詰める。
- **D9** Fullscreen 窓の矩形は出力の論理矩形全体。I5 の対象外。1 ワークスペースに Fullscreen は高々 1 枚で、2 枚目を全画面にすると 1 枚目は元の状態に戻る。
- **D10** `FocusDirection` は、フォーカス窓の矩形中心から見てその方向（Left なら中心 x が小さい側）にある表示中の窓のうち、中心間のマンハッタン距離が最小のもの。同距離は WindowId 最小。候補なしは状態不変で `Ok`（Effect 空）。
- **D11** `MoveWindow` は Tiled 窓の順序列での入れ替え。Left / Up は 1 つ前、Right / Down は 1 つ後と交換。端なら状態不変で `Ok`。Floating / Fullscreen では状態不変で `Ok`。
- **D12** `SwitchWorkspace(i)` はフォーカス出力の表示を i にする。i が別の出力で表示中なら 2 出力の表示を入れ替える。切替後のフォーカスは i のフォーカス履歴の最新、なければ順序列先頭、なければ None。
- **D13** `MoveToWorkspace(i)` はフォーカス窓を i の順序列末尾へ移す。フォーカスは D3 と同じ規則で元のワークスペース内で移る（窓に付いていかない）。
- **D14** `CloseFocused` は `RequestClose` を出すだけ。窓の削除はシェルが `WindowUnmapped` を送ったときに行う。
- **D15** `Damage` は、表示内容（窓の矩形・状態・表示ワークスペース・フォーカス）が変わった出力すべてに出す。

## 8. 依存とコード規約

- `#![no_std]` + `extern crate alloc`。
- `#![forbid(unsafe_code)]`。
- 実行時依存クレートは 0。dev-dependencies は `proptest` と `proptest-state-machine` のみ。
- 算術は `checked_*` / `saturating_*` だけ。clippy の `arithmetic_side_effects`、`unwrap_used`、
  `expect_used`、`panic`、`indexing_slicing`、`missing_docs_in_private_items` を deny。
- テストコードでは `clippy.toml` で `allow-unwrap-in-tests` と `allow-expect-in-tests` を true にする。
- 公開型はすべて `#[non_exhaustive]`（構造体は必要に応じてコンストラクタを用意）、公開項目はすべて doc コメント必須。
- Kani が要求する nightly でもビルドできるよう、新しすぎる言語機能は使わない。

## 9. モジュール構成

依存は上から下への一方向だけ許す（domain は他のどの層も use しない）。

```text
crates/contrast-core/src/
├── lib.rs            公開APIの再エクスポートのみ
├── domain/           エンティティと値オブジェクト
│   ├── geometry.rs   Point, Size, Rect
│   ├── id.rs         各ID, IdGen, ExternalKey, KeyMap
│   ├── output.rs
│   ├── workspace.rs
│   ├── window.rs
│   └── state.rs      State（全エンティティの集約）
├── layout/           Layout trait（mod.rs）と参照実装 columns.rs
├── policy/           Policy trait（mod.rs）と DefaultPolicy
├── application/      Core, Command, Action, Effect
│   └── handlers/     1ファイル1ユースケース
├── verification/     Kani ハーネス（spec/ は人間、extra/ は実装者）
├── invariant.rs      check_invariants, InvariantViolation
└── error.rs          CoreError, LayoutError
```

層の依存方向は `scripts/check-layering.sh` が CI で検査する。

## 10. P3 で確定した挙動（tests/spec/spec_core_*.rs が前提にする）

暫定決定 D1〜D15 を引き継ぎ、実装が決めていた点を仕様にする。違う意図があれば、この節とテストを一緒に直す。

- **D16 Command**: `WindowFocusRequested(ExternalKey)` を Command に加える。表示中のワークスペースの窓ならフォーカスを移す。非表示の窓は状態不変で Ok。未登録は `UnknownKey`。
- **D17 窓が多すぎるとき**: Tiled 窓の数が作業領域の幅（px）を超えると `Layout(AreaTooSmall)`。WindowMapped、OutputChanged（縮小）、ToggleFloating、MoveToWorkspace、SwitchWorkspace、OutputAdded のどれでも同じ。Err のとき状態は変わらない（OutputChanged が失敗したら、出力の矩形は古いまま）。シェルは Err をログに出して続行する。
- **D18 窓の削除とフォーカス**: フォーカス窓の WindowUnmapped は、同じワークスペースの順序列で右隣へ、末尾なら左隣へ、窓がなければ None にする（Floating・Fullscreen も対象）。ほかの窓の削除でフォーカスは変わらない。
- **D19 ワークスペースへの復帰**: 表示に戻った窓の矩形と状態が前回の Configure と同じなら、Configure は出さない（ShowWorkspace・SetFocus・Damage だけ）。シェルは、窓の最後の設定を保持する。
- **D20 浮動矩形（D8 を改める）**: 前回の浮動矩形が作業領域と交差していればそのまま保つ。なければ `requested_size`（なければ作業領域の半分）を作業領域に収まるよう縮め、中央に置く。
- **D21 OutputAdded**: フォーカスを与えない。
- **D22 Action の端**: 次はすべて状態不変で Ok（Effect なし）。Fullscreen の窓への ToggleFloating、表示中のワークスペースへの SwitchWorkspace、出力 0 台での SwitchWorkspace、現在のワークスペースへの MoveToWorkspace、Floating・Fullscreen の窓への MoveWindow、端での MoveWindow、候補のない FocusDirection。MoveWindow の前後は Tiled の窓だけを数える（Left・Up が前、Right・Down が後）。MoveToWorkspace は、インデックスの範囲、次にフォーカスの有無の順に検査する。
- **D23 WindowMetaChanged**: `meta` を差し替えるだけで、再配置・状態の変更・Effect はない。配置の判断は map のときだけ行う。
- **D24 出力の削除**: 窓は残り、表示されていたワークスペースは未表示になる。削除された出力への ShowWorkspace・Damage は出さない。同じキーの再追加は新しい OutputId になり、最小インデックスの未表示ワークスペースを表示する。
- **D25 FocusDirection の候補**: フォーカス窓以外の表示中の全窓（Floating・Fullscreen・他の出力の窓を含む）。フォーカス窓に矩形がなければ状態不変で Ok。
- **D26 不変条件 I9**: 同じワークスペースの Fullscreen の窓は高々 1 枚。`check_invariants` で検査する。
- **D27 I7・I8 の確認**: 任意の Command 列に対するプロパティテスト（`spec_core_properties.rs`）で確認する。確認項目は、毎ステップ I1〜I6 と I9、Err なら状態不変（I7）、同じ列を新しい Core に流すと結果と状態が一致（I8）、Effect の順序、Configure と SetFocus が適用後の状態と一致すること、SetFocus は変わったときだけ出ること。
- **D28 公開 API**: `StateView`・`WindowView`・`InvariantViolation`・`StateSnapshot` は現状の形で確定する。`Point` は公開フィールドと `new` だけを持つ（`x()`・`y()` は作らない）。「公開型は `#[non_exhaustive]`」の対象は、Command・Action・Effect・Placement・WindowMeta などのデータ型で、Core・StateView・IdGen・KeyMap などの振る舞いの型は対象外。`domain` は `crate::error` を use してよい。
- **D29 Damage（D15 の確認）**: フォーカスだけが変わったときも、その出力に Damage を出す。

## 11. 作業領域の余白（Insets。D4 を改める）

layer-shell のバーなどが確保する領域を、出力の作業領域から除く。余白が 0 のときは、D4（作業領域 = 出力の論理矩形）と同じ。

- **D30 Insets**: `Insets { top, right, bottom, left }`（i32、すべて 0 以上）。フィールドは非公開で、`Insets::new(top, right, bottom, left) -> Option<Insets>`（負の値があれば None）、`top()` `right()` `bottom()` `left()`、`Default`（すべて 0）を持つ。
- **D31 Command**: 出力ごとに insets を持つ。OutputAdded の時点では 0。`Command::OutputInsetsChanged { key: ExternalKey, insets: Insets }` で置き換える。未登録のキーは `UnknownKey`。同じ値なら、状態不変で Ok（Effect なし）。
- **D32 作業領域**: 出力の論理矩形から insets を引いた矩形。幅または高さが 1 未満になるときは `Layout(AreaTooSmall)` で、状態は変わらない。これは OutputInsetsChanged と、insets を持つ出力への OutputChanged の両方で検査する。窓が 1 枚もなくても検査する。
- **D33 使う場所**: Tiled の窓の配置、Floating の窓の配置（D8・D20）、不変条件 I5 は、作業領域を使う。Fullscreen の窓は insets を無視して、出力の論理矩形全体を使う（D9 のまま）。
- **D34 再配置**: insets が変わると、窓を並べ直す。Effect は通常の規則（矩形が変わった窓の Configure、Damage）に従う。

