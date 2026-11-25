#!/usr/bin/env bash
# WS3-09 determinism probe (expectation in 360-determinism.expected).
# Usage (SDK container, repo root): 360-determinism.sh <release-clc> <debug-clc> <out-dir>
# For three programs, emits hosted-amiga C: (a) twice with the release clc from the
# repository root with a relative path, (b) once with the debug clc, (c) once from a
# different working directory with the equivalent relative path, (d) once with an
# absolute path. Reports byte equality of each against (a) and any absolute path,
# date, or time string found in the output.
set -uo pipefail
rel=$1; dbg=$2; out=$3; mkdir -p "$out"
root=$(pwd)
for src in spec/tests/corpus/string-parser/program.clo benchmarks/container/program.clo examples/amiga/boing.clo; do
  n=$(echo "$src" | tr '/.' '__')
  "$rel" --profile hosted-amiga -o "$out/$n.a.c" "$src"
  "$rel" --profile hosted-amiga -o "$out/$n.a2.c" "$src"
  "$dbg" --profile hosted-amiga -o "$out/$n.b.c" "$src"
  (cd "$(dirname "$src")" && "$root/$rel" --profile hosted-amiga -o "$root/$out/$n.c.c" "$(basename "$src")")
  "$rel" --profile hosted-amiga -o "$out/$n.d.c" "$root/$src"
  for v in a2 b c d; do
    if cmp -s "$out/$n.a.c" "$out/$n.$v.c"; then r=identical; else r="differs ($(diff "$out/$n.a.c" "$out/$n.$v.c" | grep -c '^<') lines)"; fi
    echo "$src $v: $r"
  done
  echo "$src absolute paths in (d): $(grep -c "\"$root" "$out/$n.d.c")"
  echo "$src date/time strings in (a): $(grep -cE '20[0-9]{2}-[0-9]{2}-[0-9]{2}|[0-9]{2}:[0-9]{2}:[0-9]{2}' "$out/$n.a.c")"
done
