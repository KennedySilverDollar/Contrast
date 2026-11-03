# 仕様への疑問

Claude Code はここに追記する。人間が回答したら「回答:」行を足し、
必要なら `docs/core-spec.md` の「暫定決定」節に反映する。

## 書式
### Q<番号> (pN) <一行要約>
- 該当箇所: <ファイル / 節>
- 疑問: <何が決まっていないか>
- 候補: <考えられる選択肢と影響>
- 止めた作業: <この疑問に依存して保留したもの>
- 回答: （人間が記入）

### Q1 (p1) 仕様ファイル 2 本が rustfmt と clippy を通らず、CI の verify が赤い
- 該当箇所: `crates/contrast-core/tests/spec/spec_geometry.rs`、`crates/contrast-core/src/verification/spec/geometry.rs`
- 疑問: CI（run 41599697717, be9d282）の `cargo fmt --all -- --check` が、spec_geometry.rs の 143・156 行付近、
  verification/spec/geometry.rs の 31・61・86 行付近で差分を出して失敗している（GitHub の check-run 注釈で確認）。
  fmt を直すと次の clippy でも、spec_geometry.rs 5 行の補助関数 `fn rect` の `expect` が
  `clippy::expect_used` で失敗する（`allow-expect-in-tests` は `#[test]` 関数の中だけが対象。手元の clippy 1.99.0 で確認）。
  どちらも人間が書くファイルなので、実装側では直せない。
- 候補:
  - A（推奨）: 人間が rustfmt のある環境で `cargo fmt --all` を実行し、`tests/spec/main.rs` の先頭に
    `#![allow(clippy::unwrap_used, clippy::expect_used)]` を足す（または `fn rect` に `#[cfg(test)]` を付ける）。
    どちらも手元の clippy で通ることを確認済み。
  - B: `scripts/verify.sh` で clippy を 2 回に分け、`--test spec` だけ `-A clippy::expect_used` を付ける。
    fmt の差分は verify.sh 側では除外できない（stable の rustfmt に除外指定が無い）ので、A の fmt は結局必要。
- 止めた作業: なし（仕様ファイルは変更していない）
- 回答: （人間が記入）

### Q2 (p1) domain/geometry.rs が crate::error を use するのは規則 6 に反するか
- 該当箇所: `CLAUDE.md` 規則 6、`docs/core-spec.md` 9 節、`prompts/p1.md` 手順 3・4、`src/domain/geometry.rs` 9 行
- 疑問: CLAUDE.md は「domain は crate 内の他モジュールを use しない」、core-spec 9 節は「domain は他のどの層も use しない」。
  一方 p1.md は split が `LayoutError` を返し、`LayoutError` を `src/error.rs` に置くよう指示している。
  そのため geometry.rs は `use crate::error::LayoutError;` を持つ。`scripts/check-layering.sh` は `crate::error` を禁止していない。
- 候補:
  - A: error.rs を層ではなく共有の型置き場とみなし、CLAUDE.md 規則 6 に「error は除く」と足す（現状のコードのまま）。
  - B: `LayoutError` を domain 側（例: domain/geometry.rs）に移し、error.rs から再エクスポートする。公開パスは変わらない。
- 止めた作業: なし（P1 は済んでいる。P2 の CoreError が LayoutError を包むので、P2 の前に決めたい）
- 回答: （人間が記入）

### Q3 (p1) Point に x()・y() アクセサが無い
- 該当箇所: `prompts/p1.md` 手順 2、`src/domain/geometry.rs` 14〜27 行
- 疑問: p1.md は「Point は `Point::new(x, y)` と `x()` `y()` を持つ」とするが、実装は公開フィールド `x`・`y` だけで、
  メソッドは無い。仕様テストと Kani 仕様ハーネスはどちらも使っていないので、テストは通る。
- 候補:
  - A: `x()`・`y()` を足す（追加のみで既存 API は変わらない）。
  - B: フィールド公開のままとし、p1.md を直す。
- 止めた作業: アクセサの追加（今回の点検は「仕様テストを通すための最小修正」に限るため）
- 回答: （人間が記入）

### Q4 (p0) クレート名と配置が core-spec 2 節と食い違う
- 該当箇所: `docs/core-spec.md` 1 節・2 節、`crates/`
- 疑問: core-spec は `contrast-policy-cdl`・`contrast-backend-smithay`・`contrast (bin)` と書くが、リポジトリには
  `crates/contrast-policy/src/{port,cdl_adapter,static_policy}.rs`、`crates/contrast/src/main.rs`（いずれも空、Cargo.toml なし）と、
  git 未追跡の空ディレクトリ `crates/contrast-compositor/` がある。ルート Cargo.toml も bin パッケージ `contrast-wm` を持つ。
  対応を決めるはずの `docs/architecture.md` は空（0 バイト）で確認できない。
- 候補:
  - A: core-spec 2 節を実際の名前（contrast-policy、contrast、contrast-compositor）に合わせる。
  - B: ディレクトリ側を core-spec の名前に合わせる（移動は人間が行う）。
  - いずれでも、ルートの `contrast-wm` パッケージを残すか（仮想 workspace にするか）も決める。
- 止めた作業: core-spec 2 節の書き換え（意味に関わるため）
- 回答: （人間が記入）

### Q5 (p2) tests/spec/ が clippy を通らず、verify.sh（--full なし）を緑にできない
- 該当箇所: `crates/contrast-core/tests/spec/main.rs`、`tests/spec/spec_id.rs` 16・174〜181 行、`tests/spec/spec_geometry.rs` 5 行
- 疑問: `nix develop -c scripts/verify.sh` が clippy で止まる。`--all-targets --all-features -- -D warnings` の指摘 8 件は
  すべて仕様ファイルにあり、src/ と tests/impl_*.rs には 1 件もない。
  - spec_geometry.rs 5 行 `clippy::expect_used`（補助関数 `fn rect`。Q1 と同じ）
  - spec_id.rs 16 行 `clippy::arithmetic_side_effects`（補助関数 `assert_consistent` の `forward += 1`）
  - spec_id.rs 174〜177・179・181 行 `clippy::indexing_slicing`（`ids[0]` など。`#[test]` 関数の中でも対象）
  P2 の指示では「tests/spec/main.rs と同じ形の crate レベルの `#![allow(...)]`」とあるが、main.rs には現在 `#![allow]` が無い。
  no_std・layering・test（256 ケース）は個別に実行して緑（spec 35 件、impl_id 14 件、impl_geometry 11 件）。
- 候補:
  - A（推奨）: 人間が `tests/spec/main.rs` の先頭に
    `#![allow(clippy::expect_used, clippy::arithmetic_side_effects, clippy::indexing_slicing, reason = "仕様テストは読みやすさを優先する")]` を足す。
  - B: 仕様テスト側を lint に合わせて書き換える（`forward` を `checked_add` で増やす、`ids[0]` を `ids.first()` などに置き換える）。
  - C: verify.sh の clippy を `--all-targets` から、`--test spec` だけ別の lint 設定で走らせる形に分ける（scripts/ は人間が変える）。
- 止めた作業: verify.sh を緑にすること（clippy 以降のステップは個別に確認済み）
- 回答: （人間が記入）
