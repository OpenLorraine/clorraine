#!/usr/bin/env bash
# WS7-06 / WS3-11: emitted C size and m68k GCC compile time of a non-zero fill literal
# `[; 1]` for u8 arrays of 1,024, 16,384, and 65,536 elements, at module level and as a
# function local, on the A500 release line.
# Usage (SDK container, repo root): 740-fill-emission.sh <clc> <out-dir>
set -uo pipefail
clc=$1; out=$2; mkdir -p "$out"
flags=(-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s)
for n in 1024 16384 65536; do
  for where in module local; do
    src="$out/$where-$n.clo"
    if [ "$where" = module ]; then
      printf 'module Probe.Main;\n\nimport { println } from std.io;\n\nlet big: u8[%d] = [; 1];\n\nfunc main(): i32 {\n    println($"{big[%d]}");\n    return 0;\n}\n' "$n" "$((n - 1))" > "$src"
    else
      printf 'module Probe.Main;\n\nimport { println } from std.io;\n\nfunc main(): i32 {\n    let big: u8[%d] = [; 1];\n    println($"{big[%d]}");\n    return 0;\n}\n' "$n" "$((n - 1))" > "$src"
    fi
    "$clc" --profile hosted-amiga-13 -o "$out/$where-$n.c" "$src" > "$out/$where-$n.clc.log" 2>&1 || { echo "FAIL $where-$n: clc"; continue; }
    lines=$(wc -l < "$out/$where-$n.c"); bytes=$(wc -c < "$out/$where-$n.c")
    start=$(date +%s%N)
    if timeout 600 m68k-amigaos-gcc "${flags[@]}" -o "$out/$where-$n" "$out/$where-$n.c" -lamiga -lm > "$out/$where-$n.gcc.log" 2>&1; then
      ms=$(( ($(date +%s%N) - start) / 1000000 ))
      outp=$(vamos "$out/$where-$n" 2>/dev/null | tr -d '\r')
      echo "$where-$n: C $lines lines $bytes B; gcc ${ms} ms; output '$outp'"
    else
      echo "$where-$n: C $lines lines $bytes B; gcc FAILED or timed out (600 s)"
    fi
  done
done
