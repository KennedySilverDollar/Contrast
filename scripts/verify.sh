#!/usr/bin/env bash
# 使い方: scripts/verify.sh [--full]   非破壊: 検査のみ
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
full=0; [[ "${1:-}" == "--full" ]] && full=1

echo "== fmt";      cargo fmt --all -- --check
echo "== clippy";   cargo clippy -p contrast-core --all-targets --all-features -- -D warnings
echo "== no_std";   cargo build -p contrast-core --target x86_64-unknown-none
echo "== layering"; scripts/check-layering.sh

if (( full )); then
  echo "== test (10000)"; PROPTEST_CASES=10000 cargo test -p contrast-core --all-features
  echo "== kani";         cargo kani -p contrast-core
  echo "== mutants (diff)"
  diff_file="$(mktemp)"
  git diff main...HEAD > "$diff_file"
  if [[ -s "$diff_file" ]]; then
    cargo mutants -p contrast-core --in-diff "$diff_file"
  else
    echo "main との差分なし: mutants を省略"
  fi
else
  echo "== test";         PROPTEST_CASES=256 cargo test -p contrast-core --all-features
fi
echo "verify: OK"
