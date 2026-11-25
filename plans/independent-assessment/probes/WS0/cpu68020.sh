#!/usr/bin/env bash
# Probe: does vamos execute 68020 code with -C 68020, and reject it on 68000?
# Usage (inside the SDK container, repository root): cpu68020.sh <out-dir>
set -uo pipefail
here=$(dirname "$0")
out=${1:-build/tmp/assessment/WS0/cpu68020}
mkdir -p "$out"
ulimit -f 1048576
m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68020 -O2 -o "$out/cpu68020" "$here/cpu68020.c" || exit 1
m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -O2 -o "$out/cpu68000" "$here/cpu68020.c" || exit 1
m68k-amigaos-objdump -d "$out/cpu68020" > "$out/cpu68020.dis"
echo "long multiply/divide in -m68020 build: $(grep -cE 'mul[us]l|mul[us]\.l|div[us]l|div[us]\.l' "$out/cpu68020.dis")"
echo "long multiply/divide in -m68000 build: $(m68k-amigaos-objdump -d "$out/cpu68000" | grep -cE 'mul[us]l|mul[us]\.l|div[us]l|div[us]\.l')"
echo "--- vamos -C 68020 (68020 build):"
timeout 60 vamos -C 68020 "$out/cpu68020" 2>&1 | tail -3; echo "rc=${PIPESTATUS[0]}"
echo "--- vamos default CPU (68020 build):"
timeout 60 vamos "$out/cpu68020" 2>&1 | tail -3; echo "rc=${PIPESTATUS[0]}"
echo "--- vamos default CPU (68000 build, control):"
timeout 60 vamos "$out/cpu68000" 2>&1 | tail -3; echo "rc=${PIPESTATUS[0]}"
