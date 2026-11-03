contrast-core のフェーズ N を実装する。N は次の行で指定する。

フェーズ: N = （ここに番号）

# 手順
1. 次を順に読む: CLAUDE.md、docs/core-spec.md、docs/core-plan.md の 2 節と 4 節、
   prompts/common.md、prompts/pN.md、docs/questions.md。
2. 前提を確認する。満たさなければ、何も変更せず、不足を報告して止まる。
   - 現在のブランチが core/pN-* である。
   - 作業ツリーがきれい（未コミットの変更がない）。
   - docs/core-plan.md 4 節の「人間が先に書くもの」が、そのフェーズの分だけ
     tests/spec/ と src/verification/spec/ に存在し、コミット済みである。
3. prompts/pN.md の「やること」を、CLAUDE.md の規則を守って実装する。
   小さく書き、scripts/verify.sh を回し、直す、を繰り返す。小さい単位でコミットしてよい（core(pN): 要約）。
4. 仕様が曖昧、または仕様テストと prompts/pN.md が食い違うときは、推測で埋めない。
   docs/questions.md に書き、その項目に依存しない作業だけを続ける。
5. 完了前に scripts/verify.sh --full を実行する。Kani が環境の都合で動かない場合は、
   その旨を報告し、動くものだけを緑にして、Kani の結果は未確認と書く。
6. CLAUDE.md の「報告の形式」で報告して終わる。次のフェーズには進まない。
