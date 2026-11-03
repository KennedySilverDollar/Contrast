# prompts/

- `p0.md`: 足場。ガードファイルを置く前に、`claude` を通常モードで起動して貼る。
- `common.md` + `p1.md`〜`p8.md`: フェーズ別。`scripts/cc-phase.sh N` が連結して `claude -p` に渡す。
- `audit.md`: リポジトリ全体の点検と修正。chore/audit-fix ブランチで、通常モードの `claude` に貼る。
- `run-phase.md`: 対話でフェーズを回すときの共通プロンプト。N を埋めて貼る。

各フェーズの前に、`docs/core-plan.md` の 4 節「人間が先に書くもの」を
`tests/spec/` と `src/verification/spec/` に書いてコミットしておく。
フェーズの間に人間の仕様テストが入るので、全フェーズを 1 回のプロンプトでは回さない。
