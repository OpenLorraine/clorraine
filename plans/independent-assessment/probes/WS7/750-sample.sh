#!/usr/bin/env bash
# WS7-06 hotspot attribution: sample the call stack of the pinned release `clc` with
# gdb (host `perf` is unavailable: perf_event_paranoid is 4). gdb runs `clc` as its
# child, a helper sends SIGINT to it every second, and each stop records the top 14
# frames. The counts of the innermost compiler frames attribute the time. Attribution
# evidence only (confidence `probable`); no pass/fail.
# Usage (host, repo root): 750-sample.sh <clc> <input.clo> <out.log> [samples]
set -uo pipefail
clc=$1; src=$2; log=$3; n=${4:-12}
cmds=(-ex "handle SIGINT stop print nopass" -ex run)
for _ in $(seq 1 "$n"); do cmds+=(-ex "bt 14" -ex continue); done
( for _ in $(seq 1 "$n"); do sleep 1; pkill -INT -f "^$clc --profile host -o /dev/null $src" || true; done ) &
gdb -q -batch "${cmds[@]}" --args "$clc" --profile host -o /dev/null "$src" > "$log" 2>&1
wait
grep -E "^#[0-9]+ " "$log" | sed -E 's/^#[0-9]+ +(0x[0-9a-f]+ in )?//; s/ \(.*//' \
    | grep -E "clorraine|clc" | sort | uniq -c | sort -rn | head -25
