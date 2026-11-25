#!/usr/bin/env bash
# WS2 sanitizer self-test (seeded defects). Usage (SDK container, repo root):
#   san-control.sh <out-dir>
# Expected (san-control.expected): every seeded mode is reported by its sanitizer
# (non-zero exit and a sanitizer report); the clean mode passes both lines.
set -uo pipefail
here=$(dirname "$0")
out=${1:-build/tmp/assessment/WS2/san-control}
mkdir -p "$out"
cc -std=c99 -O0 -g -fsanitize=address -fno-omit-frame-pointer -no-pie -o "$out/asan" "$here/san-control.c" || exit 2
cc -std=c99 -O0 -g -fsanitize=undefined -fno-sanitize-recover=all -no-pie -o "$out/ubsan" "$here/san-control.c" || exit 2
fail=0
check() {
  local bin=$1 mode=$2 want=$3
  ASAN_OPTIONS=detect_leaks=1 "$out/$bin" "$mode" > "$out/$bin-$mode.out" 2> "$out/$bin-$mode.err"
  local rc=$?
  local report=no
  grep -qE "Sanitizer|runtime error" "$out/$bin-$mode.err" && report=yes
  local got=pass
  if [ "$rc" -ne 0 ] || [ "$report" = yes ]; then got=detected; fi
  if [ "$got" = "$want" ]; then echo "PASS $bin $mode: $got (rc=$rc)"; else echo "FAIL $bin $mode: got $got want $want (rc=$rc)"; fail=1; fi
}
check asan clean pass
check asan leak detected
check asan double-free detected
check asan use-after-free detected
check ubsan clean pass
check ubsan overflow detected
exit $fail
