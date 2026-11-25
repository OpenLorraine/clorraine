#!/usr/bin/env bash
# WS6 runner (SDK container, repository root). Bounded; writes under build/tmp only.
#
# Usage: run.sh <clc> <out-dir> main|contrast|seeded
#   main      every .clo probe (host debug/release/ASan/UBSan, hosted-amiga and
#             hosted-amiga-13 under vamos with the leak detector), then the scripts
#   contrast  every .clo probe's contrasting input
#   seeded    the detectors: a mutated appendix copy for 600 and --seed for 640, 650,
#             670, and 620's seeded quadratic workload
# Files created by probes 612 and 651 are removed before each probe run, so a file
# from an earlier run cannot satisfy an expectation.
set -uo pipefail
P=plans/independent-assessment/probes
clc=$1; out=$2; mode=$3
mkdir -p "$out" build/tmp/assessment/WS6
clean() { rm -f build/tmp/assessment/WS6/f612-* build/tmp/assessment/WS6/f651; }
case "$mode" in
main)
    clean
    python3 "$P/WS1/probe.py" "$P/WS6" --clc "$clc" --out "$out/probes" --sanitize --vamos-leaks --jobs 6
    for s in 600-api-conformance 640-kickstart-gating 650-handle-ownership 670-embedding; do
        echo "== $s"; python3 "$P/WS6/$s.py"; echo "rc=$?"
    done
    echo "== 620-scaling"; python3 "$P/WS6/620-scaling.py" "$clc" "$out/scaling"; echo "rc=$?"
    ;;
contrast)
    clean
    python3 "$P/WS1/probe.py" "$P/WS6" --clc "$clc" --out "$out/contrast" --sanitize --vamos-leaks --contrast --jobs 6
    ;;
seeded)
    mkdir -p "$out/seeded"
    sed -e 's/`bufferLen(b: Buffer)`/`bufferLenX(b: Buffer)`/' \
        -e 's/| `bufferNew(size: usize)` | `Buffer?` |/| `bufferNew(size: usize)` | `Buffer` |/' \
        -e 's/`bufferBytes(borrow b: Buffer)`/`bufferBytes(take b: Buffer)`/' \
        spec/appendix-stdlib.md > "$out/seeded/appendix-stdlib.md"
    echo "== 600 seeded"; python3 "$P/WS6/600-api-conformance.py" --appendix "$out/seeded/appendix-stdlib.md" | grep -E "bufferLenX|bufferNew|bufferBytes"
    echo "== 640 seeded"; python3 "$P/WS6/640-kickstart-gating.py" --seed | grep -E "workbench|semaphore"
    echo "== 650 seeded"; python3 "$P/WS6/650-handle-ownership.py" --seed | grep -E "seeded"
    echo "== 670 seeded"; python3 "$P/WS6/670-embedding.py" --seed | grep -E "seeded"
    echo "== 620 seeded"; python3 "$P/WS6/620-scaling.py" "$clc" "$out/scaling-seeded" --seeded-only
    ;;
esac
