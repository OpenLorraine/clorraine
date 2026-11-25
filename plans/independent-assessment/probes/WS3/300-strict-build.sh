#!/usr/bin/env bash
# WS3-01 strict-flag build of the Phase 0 emission (compile only).
# Usage (SDK container, repo root): 300-strict-build.sh <emit-dir> <out-dir> [<seeded.c>]
# For every accepted cell under <emit-dir>/<program>/<profile>/a.c it compiles with
#   -std=c99 -pedantic -Wall -Wextra -Wconversion -Wsign-conversion -Wcast-align
#   -Wshadow -Wstrict-prototypes
# plus the profile's base flags (host: cc; Amiga: m68k-amigaos-gcc with its -mcrt line;
# freestanding: its no-libc line), -O2, -c. Every warning is recorded as
# <profile> <origin> <flag>, where origin is "clo" when the reported file is a .clo
# source (code the compiler emitted for a Clorraine statement, mapped by #line) and
# "c" when it is the translation unit itself (runtime shim or unmapped helpers).
# Output: <out-dir>/warnings.tsv (one line per warning), <out-dir>/summary.tsv
# (counts per profile/origin/flag), <out-dir>/errors.tsv (any compile failure).
# With a third argument, only that file is compiled with the host line (red check).
set -uo pipefail
emit=$1; out=$2; seeded=${3:-}
mkdir -p "$out"
ulimit -f 1048576
strict=(-std=c99 -pedantic -Wall -Wextra -Wconversion -Wsign-conversion -Wcast-align -Wshadow -Wstrict-prototypes -O2 -c)
build() { # profile file log
  case "$1" in
    host) timeout 600 cc "${strict[@]}" -o /dev/null "$2" > "$3" 2>&1 ;;
    hosted-amiga) timeout 600 m68k-amigaos-gcc -mcrt=nix20 -m68000 -fwrapv "${strict[@]}" -o /dev/null "$2" > "$3" 2>&1 ;;
    hosted-amiga-13) timeout 600 m68k-amigaos-gcc -mcrt=nix13 -m68000 -fwrapv "${strict[@]}" -o /dev/null "$2" > "$3" 2>&1 ;;
    freestanding) timeout 600 m68k-amigaos-gcc -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv "${strict[@]}" -o /dev/null "$2" > "$3" 2>&1 ;;
  esac
}
record() { # profile program log
  grep -E "warning: |error: " "$3" | sed -E 's/^([^:]+):[0-9]+:[0-9]+: (warning|error): .*\[(-W[^]]+)\]$/\1\t\2\t\3/' | \
    awk -F'\t' -v p="$1" -v g="$2" 'NF==3 { o = ($1 ~ /\.clo$/) ? "clo" : "c"; print p "\t" g "\t" o "\t" $2 "\t" $3 }' >> "$out/warnings.tsv"
}
: > "$out/warnings.tsv"; : > "$out/errors.tsv"
if [ -n "$seeded" ]; then
  build host "$seeded" "$out/seeded.log"; rc=$?
  record host seeded "$out/seeded.log"
  echo "seeded rc=$rc warnings=$(wc -l < "$out/warnings.tsv")"
  exit 0
fi
for c in "$emit"/*/*/a.c; do
  profile=$(basename "$(dirname "$c")"); program=$(basename "$(dirname "$(dirname "$c")")")
  log="$out/${program}.${profile}.log"
  if ! build "$profile" "$c" "$log"; then printf '%s\t%s\n' "$profile" "$program" >> "$out/errors.tsv"; fi
  record "$profile" "$program" "$log"
done
cut -f1,3,4,5 "$out/warnings.tsv" | sort | uniq -c | sort -rn > "$out/summary.tsv"
echo "strict-build: $(wc -l < "$out/warnings.tsv") warnings, $(wc -l < "$out/errors.tsv") failed builds"
