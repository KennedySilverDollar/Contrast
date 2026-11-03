#!/usr/bin/env bash
# contrast-core の層の依存方向を検査する（非破壊）。
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
src=crates/contrast-core/src
fail=0
check() { # $1=層ディレクトリ  $2=禁止する参照の正規表現
  [[ -d "$src/$1" ]] || return 0
  if grep -rnE "$2" "$src/$1"; then
    echo "層違反: $1 から禁止された層を参照" >&2; fail=1
  fi
}
check domain 'crate::(layout|policy|application|invariant|verification)|super::super'
check layout 'crate::(policy|application|invariant)'
check policy 'crate::(layout|application|invariant)'
if grep -rnE '\b(HashMap|HashSet)\b' "$src" ; then
  echo "禁止: HashMap / HashSet" >&2; fail=1
fi
exit "$fail"
