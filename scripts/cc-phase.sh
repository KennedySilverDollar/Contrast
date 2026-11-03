#!/usr/bin/env bash
# 使い方: scripts/cc-phase.sh <フェーズ番号>
# 非破壊: ファイルの削除・巻き戻し・push は一切行わない。
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

phase="${1:?フェーズ番号を指定 (例: 1)}"
prompt="prompts/p${phase}.md"
protected='^(crates/contrast-core/tests/spec/|crates/contrast-core/src/verification/spec/|CLAUDE\.md$|\.claude/|docs/core-spec\.md$|docs/core-plan\.md$|scripts/|\.github/|.*Cargo\.toml$)'

[[ -f "$prompt" ]] || { echo "プロンプトがない: $prompt" >&2; exit 1; }

branch="$(git rev-parse --abbrev-ref HEAD)"
[[ "$branch" == core/p${phase}-* ]] || { echo "ブランチ名が core/p${phase}-* でない: $branch" >&2; exit 1; }

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "未コミットの変更がある。先にコミットして。" >&2; exit 1
fi

base="$(git rev-parse HEAD)"
mkdir -p .cc-logs
log=".cc-logs/p${phase}-$(date +%Y%m%d-%H%M%S).json"

claude -p "$(cat prompts/common.md "$prompt")" \
  --permission-mode acceptEdits \
  --max-turns 80 \
  --output-format json > "$log"
echo "ログ: $log"

changed="$( { git diff --name-only "$base" HEAD; git status --porcelain | cut -c4-; } | sort -u )"
if grep -E "$protected" <<<"$changed"; then
  echo "警告: 保護パスが変更された（上記）。自動では戻さない。差分を確認して判断して。" >&2
  exit 2
fi

scripts/verify.sh --full
echo "OK: base=$base 以降の差分をレビューしてからマージして。"
