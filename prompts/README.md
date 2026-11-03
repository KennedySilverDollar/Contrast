# prompts/

Claude Code に渡すフェーズ別プロンプト。`scripts/cc-phase.sh N` が
`common.md` と `pN.md` を連結して `claude -p` に渡す。

- P0（足場）はプロンプトなし。`docs/core-plan.md` の 3 節を手で進める。
- 各フェーズの前に、`docs/core-plan.md` の 4 節「人間が先に書くもの」を
  `tests/spec/` と `src/verification/spec/` に書いてコミットしておく。
- 対話で進めたいときは、`claude` を起動して
  「prompts/common.md と prompts/pN.md を読んで、その指示に従って」と伝えればよい。
