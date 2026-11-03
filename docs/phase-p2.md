# P2 作業ガイド: ID と対応表

目的: `IdGen`（ID の払い出し）と `KeyMap`（ExternalKey ⇔ ID の対応表）を作り、不変条件 I6 を Kani で証明する。
所要の目安は 1 日。P3（エンティティ）が使う土台なので、ここで API を固める。

## 1. 前提チェック（全部 ✓ になってから始める）
- [ ] 点検ブランチ（chore/audit-fix）の内容を確認し、`main` にマージした。
- [ ] `main` で `scripts/verify.sh` が緑。`rustfmt` と `clippy` が devhost で使える（flake を直した）。
- [ ] GitHub の CI が緑（最新コミットの ✗ が消えた）。
- [ ] P1（幾何）が `main` に入っている。`docs/questions.md` に未回答の質問がない。
- [ ] `.claude/settings.json` が存在する（`ls -a .claude`）。無ければ `scripts/cc-phase.sh` ではなく、対話で `prompts/run-phase.md` を使う。

## 2. 仕様への追記（自分で `docs/core-spec.md` を編集する）
Claude Code は `core-spec.md` を編集できない。P2 の API を、3 節の末尾に次の内容で追記する。

```markdown
### ID と対応表（P2 で確定）
- `IdKind` トレイト（`from_raw` / `raw`）を 3 つの ID 型（OutputId / WindowId / WorkspaceId）が実装する。
- `IdGen<T>::new()` は raw = 1 から払い出す。`starting_at(n)` はテストと Kani 用。
  現在値が `u64::MAX` のとき `Err(IdExhausted)`（以後も同じ）。
- `KeyMap<T>` は key と id の双方向表。`insert` は key か id の重複で `Err(DuplicateKey)`、
  `remove_by_key` は未登録で `Err(UnknownKey)`。Err のとき表は変わらない。
```

## 3. 手順
1. ブランチを切る。
   ```
   git switch main
   git pull
   git switch -c core/p2-id
   ```
2. 仕様ファイルを置く（このキットの `p2-files/crates/contrast-core/` から、同じ相対パスへコピー）。
   - `tests/spec/spec_id.rs`（新規）
   - `src/verification/spec/id.rs`（新規）
3. 既存の 2 ファイルに 1 行ずつ足す（自分で編集）。
   - `crates/contrast-core/tests/spec/main.rs` に `mod spec_id;`
   - `crates/contrast-core/src/verification/spec/mod.rs` に `pub mod id;`
4. 仕様テストを読み、意図と違う箇所があれば直す。特に確認する点:
   - `IdGen::new()` の最初の ID が 1 でよいか。`u64::MAX` を払い出さない（枯渇）でよいか。
   - `KeyMap::insert` が「key か id の片方でも重複したら `DuplicateKey`」でよいか。
   - ID を `IdKind::from_raw` で外から作れる設計でよいか（`IdGen` 以外が ID を作れてしまう）。
     気になるなら、トレイトを sealed にする案を `docs/questions.md` に書いて、実装前に決める。
5. 仕様をコミットする（この時点で実装が無いので、ビルドは通らなくて構わない）。
   ```
   git add crates/contrast-core/tests/spec crates/contrast-core/src/verification/spec docs/core-spec.md
   git commit -m "spec(p2): id and keymap specs"
   ```
6. Claude Code を回す。`.claude/settings.json` がある場合:
   ```
   scripts/cc-phase.sh 2
   ```
   ない場合は、通常モードの `claude` に `prompts/run-phase.md` を貼り、N = 2 にして実行する。
7. 終了コードを見る。`0` ならレビューへ。`2` は保護パスの変更なので、`git diff` で確認して自分で判断する。
   それ以外は `.cc-logs/` のログを見る。

## 4. レビューチェックリスト（マージ前）
- [ ] `git diff --stat main...HEAD` に、`tests/spec/` と `verification/spec/` の変更が**含まれていない**（自分のコミット以外）。
- [ ] `error.rs` の CoreError が仕様 6 節の 8 variant だけ。余計な variant を足していない。
- [ ] `IdGen::next` が `checked_add` で、`u64::MAX` で `Err(IdExhausted)`。
- [ ] `KeyMap::insert` が、**両方を確認してから**書き込んでいる（片方だけ書いて Err を返す経路がない）。
- [ ] 本体コードに `unwrap`、`expect`、`panic!`、添字アクセス、`HashMap` がない。
- [ ] `lib.rs` の再エクスポートが `prompts/p2.md` の一覧と一致。
- [ ] `scripts/verify.sh --full` を自分で実行して緑。Kani のハーネス（`idgen_*`、`keymap_*`）が実際に走っている。
- [ ] `docs/questions.md` に質問が増えていれば、回答を書く。
- [ ] `docs/decisions/p2.md` に判断を 3〜5 行残す（ID の最初の値、枯渇の定義、重複の扱いなど）。
- [ ] `main` にマージする。

## 5. 次: P3（エンティティと不変条件検査）
P3 の前に、自分で `tests/spec/spec_invariant.rs` を書く（I1〜I6 をそれぞれ壊した `State` を作り、
該当する違反が返ることを確認する）。`State` を組み立てるテスト用ビルダの形（`StateBuilder`）を、先に決めておくこと。
P3 の仕様ファイルの下書きが必要になったら依頼する。
