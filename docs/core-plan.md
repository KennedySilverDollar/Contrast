# contrast-core 制作手順書とテスト方法

9 フェーズ、合計およそ 3 週間で v0.1.0 を切る。各フェーズは
「人間が仕様テストと Kani ハーネスを手で書く → Claude Code が実装して通す →
`scripts/verify.sh --full` が緑 → 差分を手でレビューしてマージ」の順で進める。

## 1. 1 フェーズの回し方

1. `git switch -c core/pN-<名前>` でフェーズ用ブランチを切る。
2. 下の「人間が先に書くもの」を `tests/spec/` と `src/verification/spec/` に書いてコミットする。
3. `scripts/cc-phase.sh N` で Claude Code に `prompts/pN.md` を渡す。
4. `scripts/verify.sh --full` を自分の手で実行し、緑を確認する。
5. `docs/questions.md` を確認し、回答と `docs/core-spec.md` の暫定決定を更新する。
6. 差分を読み、`docs/decisions/pN.md` に判断理由を残してからマージする。

`cc-phase.sh` が終了コード 2（保護パスの変更検出）で止まったら、差分を見て扱いを自分で決める。
スクリプトは何も戻さない。

## 2. フェーズ一覧

| フェーズ | 目安 | 主な変更先 | 完了条件 |
| --- | --- | --- | --- |
| P0 足場 | 1日・手書き | リポジトリ全体 | 空の contrast-core で `verify.sh --full` が緑 |
| P1 幾何 | 1〜2日 | domain/geometry.rs | 全演算に Kani ハーネス。オーバーフローでパニックせず、分割が元の矩形を重なりなく覆う |
| P2 ID と対応表 | 1日 | domain/id.rs | 挿入・削除の任意列で I6（全単射）を Kani で証明 |
| P3 エンティティと不変条件 | 2日 | domain/*, invariant.rs, error.rs | I1〜I6 それぞれに「壊した状態を検出する」否定テスト |
| P4 Core と生存管理 | 3日 | application/*, layout/mod.rs, policy/mod.rs | 状態機械テスト 10,000 ケースで I1〜I8 |
| P5 レイアウト | 2日 | layout/columns.rs | 窓 4 枚以下で I5 を Kani で証明、64 枚までプロパティテスト |
| P6 フォーカスと Action | 3日 | application/handlers/* | Action ごとの仕様テストと状態機械テストへの Action 追加が緑 |
| P7 ポリシーポート | 1日 | policy/* | 任意のポリシー出力でも不変条件が壊れない |
| P8 硬化と API 凍結 | 2〜3日 | テスト追加のみ | 生存ミュータント 0（残すものは理由を記録）、行カバレッジ 95%、`v0.1.0` タグ |

## 3. P0 足場チェックリスト（手書き）

- [ ] Cargo workspace に `crates/contrast-core` を作る（`[lib]` のみ、依存 0）
- [ ] `rust-toolchain.toml` に stable と `x86_64-unknown-none` ターゲット、rustfmt、clippy、llvm-tools
- [ ] flake の devShell に cargo-kani、cargo-mutants、cargo-llvm-cov、cargo-semver-checks
- [ ] `crates/contrast-core/Cargo.toml` の `[lints]` に仕様 8 節の clippy 設定、`[features] invariants = []`
- [ ] `clippy.toml` に `allow-unwrap-in-tests = true` と `allow-expect-in-tests = true`
- [ ] `lib.rs` に `#![no_std]`、`extern crate alloc;`、`#![forbid(unsafe_code)]`、`#[cfg(any(kani))] mod verification;`
- [ ] `src/verification/` の `mod.rs`、`spec/mod.rs`、`extra/mod.rs`、`tests/spec/main.rs` を置く（キット同梱）
- [ ] 本リポジトリ同梱の CLAUDE.md、`.claude/settings.json`、`scripts/`、`prompts/`、`docs/` を配置
- [ ] `.gitignore` に `.cc-logs/` と `mutants.out*/`
- [ ] `scripts/*.sh` に実行権限を付ける
- [ ] CI（verify / kani / mutants-coverage の 3 ジョブ）
- [ ] `main` ブランチを作り、以降のフェーズはここから切る

## 4. 人間が先に書くもの（各フェーズ）

Claude Code はこれらを編集できない。実装の方向を決める唯一の仕様なので、ここを先に固める。

| フェーズ | `tests/spec/` | `src/verification/spec/` |
| --- | --- | --- |
| P1 | spec_geometry.rs: Size::new の下限、contains / intersect の具体例、split の余り配分の具体例 | geometry.rs: split_columns / split_rows の分割性、intersect の対称性 |
| P2 | spec_id.rs: IdGen の単調性、DuplicateKey、UnknownKey | id.rs: 操作 3 回以内の任意列で全単射 |
| P3 | spec_invariant.rs: I1〜I6 それぞれを壊した State を作り、該当 variant が返ること | なし |
| P4 | spec_lifecycle.rs: 出力・窓の追加削除の具体例と Effect 列、spec_state_machine.rs（状態機械テスト本体） | なし |
| P5 | spec_layout.rs: 1〜3 枚の配置の具体例、AreaTooSmall | layout.rs: 窓 4 枚以下で I5 |
| P6 | spec_actions.rs: D10〜D14 の具体例 | なし |
| P7 | spec_policy.rs: Placement の各フィールドが反映される例 | なし |
| P8 | なし（生存ミュータントを殺すテストは Claude Code が impl_* に足す） | なし |

## 5. テスト方法

| 層 | 置き場所 | 書く人 | 道具 | 実行タイミング | 保証すること |
| --- | --- | --- | --- | --- | --- |
| 仕様テスト | tests/spec/ | 人間 | cargo test | verify | 仕様どおりの具体例 |
| 状態機械テスト | tests/spec/spec_state_machine.rs | 人間 | proptest-state-machine | verify 256件 / full 10,000件 | 任意の Command 列で I1〜I8 |
| 実装テスト | tests/impl_*.rs, src 内 `#[cfg(test)]` | Claude Code | cargo test | verify | 境界値と回帰 |
| Kani 証明 | src/verification/spec, extra | 両方 | cargo kani | full | 有界入力でのパニック不在と性質 |
| no_std ビルド | なし | なし | cargo build --target x86_64-unknown-none | verify | std 依存が混入していない |
| 層検査 | scripts/check-layering.sh | 人間 | grep | verify | モジュール依存の方向 |
| ミューテーション | なし | なし | cargo-mutants | full は差分、P8 は全体 | テストが実装の変化を検出できる |
| カバレッジ | なし | なし | cargo-llvm-cov | P8, CI | 行カバレッジ 95% 以上 |

unsafe を forbid しているので Miri は使わない。

### 5.1 Kani ハーネスの書き方（P1 の例）

入力は `kani::any()` で全域から取り、`kani::assume` で個数だけを絞る。
パニックとオーバーフローは Kani が自動で検査するので、性質だけを `kani::assert` で書く。

```rust
// crates/contrast-core/src/verification/spec/geometry.rs
#[cfg(kani)]
mod harness {
    use crate::domain::geometry::Rect;

    /// split_columns は元の矩形を隙間・重なりなく n 列に分ける。
    #[kani::proof]
    #[kani::unwind(6)]
    fn split_columns_partitions_rect() {
        let (x, y, w, h): (i32, i32, i32, i32) = kani::any();
        let n: usize = kani::any();
        kani::assume((1..=4).contains(&n));
        let Some(r) = Rect::new(x, y, w, h) else { return };
        match r.split_columns(n) {
            Err(_) => kani::assert(r.width() < n as i32, "失敗は幅不足のときだけ"),
            Ok(parts) => {
                kani::assert(parts.len() == n, "n 個に分かれる");
                let mut cursor = r.x();
                for p in parts.iter() {
                    kani::assert(p.x() == cursor, "隙間なく隣接する");
                    kani::assert(p.y() == r.y() && p.height() == r.height(), "高さは元と同じ");
                    cursor = p.right();
                }
                kani::assert(cursor == r.right(), "右端が一致する");
            }
        }
    }
}
```

状態全体の不変条件は状態空間が大きく Kani では重いので、状態機械テストに任せる。

### 5.2 状態機械テスト（P4 で導入）

単純な参照モデル（窓とワークスペースの所属、フォーカスの有無だけを持つ）と本物の `Core` に
同じ Command 列を与え、毎ステップ後に次を確認する。

1. `check_invariants()` が Ok（I1〜I6）。
2. `apply` が Err のとき、呼ぶ前の `snapshot()` と状態が等しい（I7）。
3. 同じ列を新しい `Core` にもう一度流し、Effect 列が完全一致する（I8）。
4. 窓の所属・フォーカスの有無が参照モデルと一致する。

Command の生成器は、存在しない `ExternalKey` への操作を 2 割混ぜ、エラー経路も必ず通るようにする。
P6 で Action を生成器に加える。

### 5.3 CI（GitHub Actions）

- `verify`: push ごと。`scripts/verify.sh`
- `kani`: PR ごと。`model-checking/kani-github-action` で `cargo kani -p contrast-core`
- `mutants-coverage`: PR ごと。差分ミュータントと `cargo llvm-cov -p contrast-core --fail-under-lines 95`

### 5.4 NixOS（devhost）での注意

`cargo kani setup` は動的リンクされた CBMC などの配布バイナリを取得するため、
NixOS ではそのままでは動かない可能性が高い。devhost で `programs.nix-ld.enable = true;` を
有効にするか、Kani だけ CI に任せる。

## 6. 未決事項

- [ ] 仕様 v0.6 と本書・`core-spec.md` の突き合わせ。特にレイアウト方式（均等列は仮の参照実装）
- [ ] nested プロトタイプ（約 3 週間）との順序
- [ ] devhost で Kani を動かすか（nix-ld か CI 専任か）
- [ ] 暫定決定 D1（ワークスペースの共有方式）と D8（浮動窓の配置規則）の確定
- [ ] 窓数の想定上限（複製して差し替える更新方式の性能前提）
- [ ] no_std を将来も維持するか
