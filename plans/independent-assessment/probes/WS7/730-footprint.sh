#!/usr/bin/env bash
# WS7-02: static footprint of every program the A500 profile accepts, built with the
# A500 shipped release line, against the 512 KB machine.
# Usage (SDK container, repo root): 730-footprint.sh <clc> <out-dir> [--seeded]
# Programs: the 31 hosted-amiga-13 cells the Phase 0 emission accepted
# (findings/WS0-emission-record.tsv). --seeded builds a C program with a 600 KB static
# array, which must be reported over budget (a Clorraine module-level fill of that size
# does not build in time, see probe 740).
set -uo pipefail
clc=$1; out=$2; seeded=${3:-}
rec=plans/independent-assessment/findings/WS0-emission-record.tsv
flags=(-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s)
mkdir -p "$out"
progs=$(awk -F'\t' 'NR>1 && $2=="hosted-amiga-13" && $3=="yes" {print $1}' "$rec")
if [ "$seeded" = --seeded ]; then
    printf 'static unsigned char big[614400];\nint main(int argc, char **argv) { (void)argv; big[argc] = 1; return big[7]; }\n' > "$out/seeded.c"
    m68k-amigaos-gcc "${flags[@]}" -o "$out/seeded" "$out/seeded.c" -lamiga -lm
    read -r t d b _ < <(m68k-amigaos-size "$out/seeded" | tail -1)
    tot=$((t + d + b))
    [ "$tot" -gt 524288 ] && echo "OVER seeded.c: $tot bytes exceeds 512 KB"
    exit 0
fi
printf "%-48s %8s %8s %8s %8s %6s\n" program text data bss total pct512
over=0
for p in $progs; do
    n=$(echo "$p" | tr '/' '_' | sed 's/\.clo$//')
    if ! "$clc" --profile hosted-amiga-13 -o "$out/$n.c" "$p" > "$out/$n.clc.log" 2>&1; then
        echo "FAIL $p: clc"; over=$((over + 1)); continue
    fi
    if ! m68k-amigaos-gcc "${flags[@]}" -o "$out/$n" "$out/$n.c" -lamiga -lm > "$out/$n.gcc.log" 2>&1; then
        echo "FAIL $p: gcc"; over=$((over + 1)); continue
    fi
    read -r t d b _ < <(m68k-amigaos-size "$out/$n" | tail -1)
    tot=$((t + d + b)); pct=$((tot * 100 / 524288))
    printf "%-48s %8d %8d %8d %8d %5d%%\n" "$p" "$t" "$d" "$b" "$tot" "$pct"
    [ "$tot" -gt 524288 ] && { echo "OVER $p: $tot bytes exceeds 512 KB"; over=$((over + 1)); }
done
echo "footprint: $over report(s)"
