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

### Q1 (p1) `cfg(kani)` が clippy の `-D warnings` で `unexpected_cfgs` になる
- 該当箇所: `crates/contrast-core/Cargo.toml` / `scripts/verify.sh` / core-spec 5 節・core-plan 5.1
- 疑問: 仕様は `#[cfg(kani)]` でハーネスを置く前提だが、`cfg(kani)` が check-cfg に未登録のため
  `cargo clippy ... -D warnings` が `unexpected cfg condition name: kani` で失敗する。
  解消には Cargo.toml の `[lints.rust]`、build.rs、`#[allow]` のいずれかが要るが、
  どれも CLAUDE.md で禁止されている。
- 候補: 人間が contrast-core/Cargo.toml の `[lints.rust]` に
  `unexpected_cfgs = { level = "warn", check-cfg = ['cfg(kani)'] }` を足す（推奨）。
- 止めた作業: `src/verification/extra/geometry.rs` は作成済みだが、lib.rs から `mod` 宣言していない
  （宣言すると verify.sh が赤になる）。Kani ハーネスの実行。
- 回答:

### Q2 (p1) P1 の仕様ファイルが存在しない
- 該当箇所: `crates/contrast-core/tests/spec/spec_geometry.rs`、`crates/contrast-core/src/verification/spec/geometry.rs`
- 疑問: prompts/p1.md が「通す」対象とする仕様テストと仕様ハーネスがリポジトリにない。
  また `tests/spec/` 配下は cargo が自動検出しないため、Cargo.toml に `[[test]]` 登録が要る（人間の作業）。
  `verification/spec` を読み込む `mod` 宣言（`verification/mod.rs` 等）をどちらが書くかも未定。
- 候補: 人間が両ファイルと `[[test]] name = "spec_geometry"`、`path = "tests/spec/spec_geometry.rs"` を追加し、
  `verification` モジュールの宣言者を決める。
- 止めた作業: 仕様テスト・仕様ハーネスに対する合格確認。
- 回答:

### Q3 (p1) 未確定の幾何の細部（暫定実装済み）
- 該当箇所: core-spec 3 節 `Point` / `Size` / `Rect`、prompts/p1.md、CLAUDE.md 規則 6
- 疑問:
  1. `Rect::center` の丸め。暫定で `(x + width / 2, y + height / 2)`（切り捨て）。D10 の距離計算に影響する。
  2. `Size::new` の引数型。暫定で `i32`（`Rect` の座標と揃え、負値は `None`）。`u32` の可能性もある。
  3. `Point` のフィールドを公開した（`pub x`, `pub y`。`#[non_exhaustive]` なので外部からは `Point::new` で構築）。
     `Size` / `Rect` は不変条件があるので非公開フィールドとアクセサ。
  4. `domain/geometry.rs` が `crate::error::LayoutError` を use している。p1.md の指示どおりだが、
     CLAUDE.md 規則 6「domain は crate 内の他モジュールを use しない」と字面上は衝突する
     （check-layering.sh は error を禁止していない）。
- 候補: 仕様テストで確定させる。異なれば実装を合わせる。
- 止めた作業: なし（暫定実装で進め、仕様テスト到着時に合わせる）。
- 回答:
