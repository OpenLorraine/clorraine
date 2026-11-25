#!/usr/bin/env bash
# WS3-02 vbcc scan. Usage: 301-vbcc-scan.sh <emit-dir> [<seeded.c> <out-dir>]
# Reads the Phase 0 vbcc logs (<emit-dir>/<program>/<profile>/vbcc.log) and reports
# every warning or error line that names the emitted unit (a.c) or a .clo source,
# split from NDK-header noise. With a seeded file, compiles it with the gate's vbcc
# line in the SDK container and reports its warnings (red check).
set -uo pipefail
emit=$1
if [ $# -ge 3 ]; then
  mkdir -p "$3"
  vc +aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include -c -o "$3/seeded.o" "$2" > "$3/seeded.log" 2>&1 || \
    vc +aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include -c -o "$3/seeded.o" "$2" > "$3/seeded.log" 2>&1
  echo "seeded: $(grep -ci 'warning\|error' "$3/seeded.log") diagnostics"; grep -i "warning\|error" "$3/seeded.log" | head -5
  exit 0
fi
total=0
for log in "$emit"/*/hosted-amiga*/vbcc.log; do
  n=$(grep -iE "warning|error" "$log" | grep -cE '"[^"]*(a\.c|\.clo)"|a\.c|\.clo' || true)
  all=$(grep -ciE "warning|error" "$log" || true)
  if [ "$n" -gt 0 ]; then echo "UNIT $log: $n"; grep -iE "warning|error" "$log" | grep -E 'a\.c|\.clo' | head -3; fi
  total=$((total + n)); echo "LOG $log unit=$n all=$all" >> /dev/null
done
echo "vbcc-scan: $total unit diagnostics"
