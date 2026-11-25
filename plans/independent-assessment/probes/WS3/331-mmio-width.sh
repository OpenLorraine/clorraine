#!/usr/bin/env bash
# WS3-06 MMIO access-width check (expectation in 331-mmio-width-called.expected).
# Usage (SDK container, repo root): 331-mmio-width.sh <clc> <out-dir>
# Emits 331-mmio-width-called.clo for hosted-amiga and hosted-amiga-13, compiles it to
# assembly with the shipped -O2 line, and prints every instruction of poke() that
# touches one of the probe's absolute addresses or an address register loaded from
# one, so each access can be classified by width and count.
set -uo pipefail
here=$(dirname "$0"); clc=$1; out=$2
mkdir -p "$out"
for p in hosted-amiga:nix20 hosted-amiga-13:nix13; do
  profile=${p%%:*}; crt=${p##*:}
  "$clc" --profile "$profile" -o "$out/$profile.c" "$here/331-mmio-width-called.clo" || exit 1
  m68k-amigaos-gcc -std=c99 -mcrt=$crt -m68000 -fwrapv -O2 -S -o "$out/$profile.s" "$out/$profile.c" || exit 1
  echo "== $profile: poke()"
  awk '/^_cl_Probe_Main_poke:/,/rts/' "$out/$profile.s"
done
