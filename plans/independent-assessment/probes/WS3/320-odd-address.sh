#!/usr/bin/env bash
# WS3-05: build 320-odd-address.c for the 68000 and run it under vamos (default
# CPU 68000) and under vamos -C 68020. Usage: 320-odd-address.sh <out-dir>
set -uo pipefail
here=$(dirname "$0"); out=$1; mkdir -p "$out"
m68k-amigaos-gcc -std=c99 -mcrt=nix13 -m68000 -O0 -o "$out/odd" "$here/320-odd-address.c" || exit 1
echo "== vamos 68000"; timeout 60 vamos "$out/odd" 2>&1 | tail -3; echo "rc=${PIPESTATUS[0]}"
echo "== vamos 68020"; timeout 60 vamos -C 68020 "$out/odd" 2>&1 | tail -3; echo "rc=${PIPESTATUS[0]}"
