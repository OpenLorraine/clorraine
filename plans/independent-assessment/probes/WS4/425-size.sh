#!/usr/bin/env bash
# WS4-07 fixed per-program size on the A500 (expectation in 425-size.expected).
# Usage (SDK container, repo root): 425-size.sh <pinned-clc> <plan-folder> <out-dir>
# Builds benchmarks/calibration (Clorraine) and probes/WS0/ref/calibration.c (C), and
# benchmarks/sieve and ref/sieve.c, with the A500 shipped -O2 line without -s, and
# lists the retained text symbols with their sizes (m68k-amigaos-nm --size-sort -S),
# so the fixed runtime cost and what pulls it in can be read.
set -uo pipefail
clc=$1; plan=$2; out=$3; mkdir -p "$out"
flags=(-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections)
for b in calibration sieve; do
  "$clc" --profile hosted-amiga-13 -o "$out/$b.clo.c" "benchmarks/$b/program.clo"
  m68k-amigaos-gcc "${flags[@]}" -Wl,-Map="$out/$b.clo.map" -o "$out/$b.clo" "$out/$b.clo.c" -lamiga -lm
  m68k-amigaos-gcc "${flags[@]}" -o "$out/$b.c" "$plan/probes/WS0/ref/$b.c" -lamiga -lm
  for side in clo c; do
    echo "== $b $side: $(m68k-amigaos-size "$out/$b.$side" | tail -1)"
    m68k-amigaos-nm --size-sort -S "$out/$b.$side" | while read -r a s t n; do case "$t" in t|T) echo "$((16#$s)) $n";; esac; done | sort -rn | head -15
  done
done
