あなたは Contrast リポジトリ全体の点検と修正を行う担当。
目的は「今のリポジトリが、docs/core-spec.md と docs/core-plan.md の設計どおりに動き、CI が緑になる状態にすること」。
新機能の実装はしない。P2 以降のコードは書かない。

# 0. 作業ルール（最優先。例外なし）
1. 最初に `git status --short` と `git branch --show-current` を確認する。main にいるなら
   `git switch -c chore/audit-fix` で作業ブランチを切る。main に直接コミットしない。
2. 破壊的な操作を一切しない。ファイル削除（rm）、`git rm`、`git reset`、`git clean`、`git checkout -- <path>`、
   `git restore`、`git rebase`、`git stash drop`、`git push`、強制上書きのリダイレクト（既存ファイルへの `>`）。
   削除や上書きが必要に見える場合は、実行せず、最後の報告の「要承認の破壊的操作」に
   コマンドを 1 行ずつ分けて書く。複数コマンドをまとめたブロックに削除を混ぜない。
3. 次は読み取り専用。変更しない: flake.nix、flake.lock、deny.toml、rustfmt.toml、README.md、LICENSE、
   docs/architecture.md、docs/cdl-contract.md、crates/contrast-core/tests/spec/、
   crates/contrast-core/src/verification/spec/、p1-spec/。問題を見つけたら報告に書くだけにする。
4. 次は、この作業に限って変更してよい: CLAUDE.md、.claude/、scripts/、.github/、prompts/、
   docs/core-spec.md、docs/core-plan.md、docs/questions.md、rust-toolchain.toml（変更は報告に理由を書く）、
   crates/contrast-core/Cargo.toml、crates/contrast-core/clippy.toml、
   crates/contrast-core/src/（コンパイル・lint・仕様テストを通すための最小修正のみ。公開 API の名前と引数は変えない）、
   crates/contrast-core/tests/impl_*.rs、crates/contrast-core/src/verification/extra/。
5. 仕様テストや Kani 仕様ハーネスが失敗しても、テスト側を変えない。実装側を直せないなら
   docs/questions.md に書き、報告の「要判断」に入れる。
6. 依存クレートを追加しない。`cargo add`、`cargo install`、`rustup component add`、`nix` 系の変更もしない。
   ツールが足りなければ、入れずに報告に「必要なもの」として書く。
7. 修正は種類ごとに別コミットにする。コミットメッセージは `chore(audit): <要約>`。push しない。
8. 推測で直さない。根拠（実行した出力、ファイルの該当行）を示せないものは「要判断」に回す。

# 1. 既知の事実（点検の出発点。今の状態と違えば、今の状態が正）
- ルート Cargo.toml は、パッケージ contrast-wm（edition 2024、src/main.rs）と
  [workspace] members = ["crates/contrast-core"] の両方を持つ。
- crates/contrast-policy/（port.rs、cdl_adapter.rs、static_policy.rs）と
  crates/contrast/src/main.rs は、Cargo.toml が無く workspace にも入っていない。
- crates/contrast-core/src/lib.rs は `mod domain { pub mod geometry; }`、`mod error;`、
  `#[cfg(kani)] mod verification;` を宣言し、Point・Rect・Size・LayoutError を再エクスポートしている。
  domain/id.rs、output.rs、window.rs、workspace.rs、invariant.rs は宣言されていない孤立ファイル。
- p1-spec/crates/contrast-core/ に、仕様ファイルのコピーが残っている。
- .claude/settings.json が git に追跡されていない。
- `scripts/verify.sh` を devhost で実行すると、最初の `cargo fmt` が
  「no such command: fmt」で止まる（rustfmt が未導入）。clippy 以降は未確認。
- GitHub の最新コミットに赤い ✗（CI 失敗）が付いている。原因は未確認。
- docs/core-spec.md の 2 節は、クレートを contrast-policy-cdl と contrast-backend-smithay と書いているが、
  実際は contrast-policy と contrast のようだ（docs/architecture.md で確認する）。

# 2. 点検（まず全部、読み取りと実行だけで行い、結果を表にまとめる）
各項目で、確認したこと・実行したコマンド・結果を残す。

## 2.1 環境
- `which cargo rustc rustfmt cargo-clippy cargo-kani cargo-mutants cargo-llvm-cov` と
  `cargo --version`、`rustc --version`、`rustfmt --version`（失敗なら失敗を記録）。
- rust-toolchain.toml、flake.nix（読むだけ）、.github/workflows/core.yml が、
  使う Rust のバージョン・コンポーネント・ターゲットで食い違っていないか。
  ルートの edition 2024 に必要な Rust のバージョンと、実際のバージョンの整合。
- rustfmt と clippy が無い原因（Nix の指定漏れか rustup の問題か）を特定し、
  flake.nix を直す場合の差分案を、適用せずにテキストで報告に書く。

## 2.2 ワークスペースとクレート
- `cargo metadata --no-deps --format-version 1` で、workspace の members と各パッケージの
  edition・rust-version・lints を確認する。ルートの contrast-wm と contrast-core で edition が違う点の影響を評価する。
- crates/contrast-core/Cargo.toml が prompts/p0.md の指定（features、lints、dev-dependencies）どおりか。
  ルートの clippy.toml と crates/contrast-core/clippy.toml の重複・衝突。
- crates/contrast-policy と crates/contrast に Cargo.toml が無い件。docs/architecture.md の記述と照らして、
  どう扱うべきかの案を出す（この作業では workspace に追加しない。要判断）。

## 2.3 contrast-core の中身
- `cargo build -p contrast-core`、`cargo build -p contrast-core --target x86_64-unknown-none`、
  `cargo clippy -p contrast-core --all-targets --all-features -- -D warnings`、
  `cargo test -p contrast-core --all-features` を **1 つずつ**実行し、結果を記録する。
  fmt が使えなくても、他の項目は続ける。
- domain/geometry.rs が docs/core-spec.md と prompts/p1.md の要件（Debug・Clone・Copy・PartialEq・Eq、
  checked 算術、半開区間、center の式、split の余り配分）を満たすか、コードを読んで確認する。
- tests/spec/spec_geometry.rs と verification/spec/geometry.rs が、実装の公開 API とシグネチャで
  コンパイルできるか（`cargo test -p contrast-core --test spec --no-run`、
  Kani が無ければ `cargo check -p contrast-core --cfg kani` は使わず、ハーネスのコードを目視で確認）。
- unwrap・expect・panic!・添字アクセス・HashMap/HashSet・unsafe が本体コードに無いか（grep と clippy）。
- 孤立ファイル（id.rs など）の中身が空スタブか実装済みかを確認し、どのフェーズ用かを対応づける。

## 2.4 scripts と CLAUDE.md
- `bash -n scripts/*.sh`、shellcheck があれば実行。
- verify.sh、check-layering.sh、cc-phase.sh のパス・パッケージ名・ターゲットが、実際の構成
  （ルートに contrast-wm があること、crates/contrast-core があること）と合っているか。
  `cargo fmt --all` がルートの contrast-wm と contrast-core を両方対象にしてよいか。
- cc-phase.sh の protected 正規表現が、実際の保護すべきファイル
  （.claude/、CLAUDE.md、Cargo.toml、rust-toolchain.toml、.github/、scripts/、docs/core-spec.md など）を
  漏れなく含むか。settings.json の deny との食い違い。
- CLAUDE.md の記述（コマンド、パス、テストの実行方法 `--test spec`）が、実際と合っているか。

## 2.5 CI
- .github/workflows/core.yml を読み、YAML として正しいか（python3 の yaml か、無ければ目視）。
- 各ジョブの手順を、手元で再現できる形に分解し、1 ステップずつ実行して、赤くなる原因を絞る。
- `gh` コマンドが使えれば、読み取りだけ（`gh run list --limit 5`、`gh run view <id> --log-failed`）で
  失敗ログを取得する。使えなければ、ワークフローから推測した原因と、その根拠を書く。
- kani ジョブがハーネス 0 件でも落ちない設定か。mutants-coverage ジョブは P8 まで `if: false` か。

## 2.6 ドキュメントの整合
- docs/architecture.md と docs/cdl-contract.md を読み、docs/core-spec.md の 2 節（クレート構成）、
  1 節（責務と非責務）、docs/core-plan.md の用語・パスとの食い違いを一覧にする。
- docs/core-spec.md の「暫定決定 D1〜D15」と architecture.md の記述が矛盾しないか。
- prompts/*.md が参照するパスや型名が、実際のリポジトリに存在するか（`crates/contrast-core/src/...`）。

## 2.7 p1-spec/ の残骸
- p1-spec/crates/contrast-core/ の各ファイルを、crates/contrast-core/ の対応ファイルと
  `diff` で比較する（読み取りのみ）。同一か、差があるかを報告する。削除は提案するだけ。

# 3. 修正（点検結果のうち、次の条件を満たすものだけ直す）
- 根拠が出力で示せる。
- 4 項で許可されたファイルだけの変更で済む。
- 破壊的操作を含まない。
直す対象の優先順:
1. CI を赤くしている原因（ワークフロー、パス、ターゲット、lint 設定の誤り）。
2. scripts/verify.sh の再現性（ツールが無いときに分かりやすく失敗する、各ステップの見出しが出る）。
3. .claude/settings.json が存在しなければ作る（次の内容。既存があれば内容を比較して差分だけ報告）。
   .gitignore に .claude が含まれていないかも確認する。
4. cc-phase.sh の protected 正規表現と settings.json の deny の食い違い。
5. CLAUDE.md、docs/core-plan.md、docs/core-spec.md の、実際の構成との食い違い
   （クレート名、パス、テスト実行コマンド）。仕様の意味（責務・不変条件・API・暫定決定）は変えない。
   意味に関わる食い違いは、直さず要判断にする。
6. crates/contrast-core のコンパイル・lint・実装テストの失敗（公開 API は変えない）。

.claude/settings.json の内容:

```json
{
  "permissions": {
    "allow": [
      "Bash(cargo build:*)",
      "Bash(cargo test:*)",
      "Bash(cargo clippy:*)",
      "Bash(cargo fmt:*)",
      "Bash(cargo kani:*)",
      "Bash(cargo mutants:*)",
      "Bash(scripts/verify.sh:*)",
      "Bash(scripts/check-layering.sh:*)",
      "Bash(git status:*)",
      "Bash(git diff:*)",
      "Bash(git log:*)",
      "Bash(git add:*)",
      "Bash(git commit:*)",
      "Bash(cargo doc:*)",
      "Bash(cargo llvm-cov:*)"
    ],
    "deny": [
      "Bash(rm:*)",
      "Bash(git reset:*)",
      "Bash(git clean:*)",
      "Bash(git checkout:*)",
      "Bash(git restore:*)",
      "Bash(git rebase:*)",
      "Bash(git stash:*)",
      "Bash(git push:*)",
      "Bash(cargo add:*)",
      "Edit(./crates/contrast-core/tests/spec/**)",
      "Write(./crates/contrast-core/tests/spec/**)",
      "Edit(./crates/contrast-core/src/verification/spec/**)",
      "Write(./crates/contrast-core/src/verification/spec/**)",
      "Edit(./**/Cargo.toml)",
      "Write(./**/Cargo.toml)",
      "Edit(./CLAUDE.md)",
      "Edit(./.claude/**)",
      "Write(./.claude/**)",
      "Edit(./docs/core-spec.md)",
      "Edit(./docs/core-plan.md)",
      "Edit(./scripts/**)",
      "Write(./scripts/**)",
      "Edit(./.github/**)",
      "Write(./.github/**)",
      "Edit(./rust-toolchain.toml)",
      "Write(./rust-toolchain.toml)"
    ]
  }
}
```

# 4. 最後の確認
- `scripts/verify.sh` を実行する。rustfmt や clippy が環境に無くて止まる場合は、止まったステップ名を報告し、
  それ以外のステップを個別に実行して結果を貼る。
- `git diff --stat main...HEAD` と、`git log --oneline main..HEAD` を貼る。

# 5. 報告の形式（この順で、日本語で）
1. 点検結果の表: ID、領域（環境 / ワークスペース / core / scripts / CI / docs / 残骸）、深刻度（高・中・低）、
   根拠（コマンドと出力の要点、またはファイルと行）、対応（修正済み / 提案のみ / 要判断）。
2. 修正したこと: コミットごとに、変更ファイルと理由。
3. CI が赤い原因: 特定できたか、できなければ何が不明か。
4. 要判断: 私が決めるべきこと（選択肢と、推奨とその理由）。
5. 要承認の破壊的操作: コマンドを 1 行ずつ。実行はしない。
6. 環境で必要な変更: flake.nix への差分案（適用しない）と、足りないツール一覧。
7. `scripts/verify.sh` の実行結果（止まった場合はそのステップ名と、残りを個別に実行した結果）。
8. この作業では手を付けなかったもの（読み取り専用の領域、P2 以降）。
