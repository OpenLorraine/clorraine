#!/usr/bin/env bash
# WS5-12: link the freestanding corpus emission with the freestanding build line and
# list undefined symbols (expectation in 520-freestanding-link.expected).
# Usage (SDK container, repo root): 520-freestanding-link.sh <clc> <out-dir>
set -uo pipefail
clc=$1; out=$2; mkdir -p "$out"
"$clc" --profile freestanding -o "$out/fs.c" spec/tests/corpus/freestanding/program.clo || exit 1
m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O2 -c -o "$out/fs.o" "$out/fs.c" || exit 1
echo "undefined symbols in the object:"; m68k-amigaos-nm -u "$out/fs.o"
m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O2 -o "$out/fs" "$out/fs.c" -lg > "$out/link.log" 2>&1; echo "link rc=$?"; head -5 "$out/link.log"
