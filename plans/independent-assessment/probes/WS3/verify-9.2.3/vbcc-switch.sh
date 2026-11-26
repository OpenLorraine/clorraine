#!/usr/bin/env bash
# Verifier re-measurement of ASM-WS3-007: vbcc compile-only time vs switch size,
# plus a control (same arm count as an if-else chain) and an -O1 contrast.
set -uo pipefail
ulimit -f 1048576
C=build/tmp/assessment/baseline/clc
D=build/tmp/assessment/verify/9.2.3/vbcc-switch; mkdir -p $D
LOG=$D/times.log; : > $LOG
gen() { # kind n
python3 - "$1" "$2" "$D" <<'PY'
import sys
kind=sys.argv[1]; n=int(sys.argv[2]); d=sys.argv[3]
hdr="module Probe.Shape;\n\nimport { println } from std.io;\n\nfunc f(n: i32): i32 {\n    let r: i32 = 0;\n"
if kind=="switch":
    body="    switch (n) {\n" + "".join(f"        case {i}:\n            r = {i*7%1000};\n            break;\n" for i in range(n)) + "        default:\n            r = -1;\n            break;\n    }\n"
else:
    body="".join(f"    {'if' if i==0 else '} else if'} (n == {i}) {{\n        r = {i*7%1000};\n" for i in range(n)) + "    } else {\n        r = -1;\n    }\n"
open(f"{d}/{kind}{n}.clo","w").write(hdr+body+"    return r;\n}\n\nfunc main(): i32 {\n    println($\"{f(1)}\");\n    return 0;\n}\n")
PY
}
for spec in switch:0 switch:250 switch:500 switch:1000 ifchain:500 ifchain:1000; do
  k=${spec%%:*}; n=${spec##*:}
  gen $k $n
  $C --profile hosted-amiga -o $D/$k$n.c $D/$k$n.clo || { echo "$k-$n clc failed" >> $LOG; continue; }
  for opt in -O2 -O1; do
    [ "$opt" = -O1 ] && [ "$k$n" != switch1000 ] && [ "$k$n" != switch500 ] && continue
    s=$(date +%s%N); timeout 600 vc +aos68k -c99 $opt -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include -c -o $D/$k$n.o $D/$k$n.c > $D/$k$n$opt.log 2>&1; rc=$?; e=$(date +%s%N)
    echo "vbcc $opt $k-$n: rc=$rc $(( (e-s)/1000000 ))ms lines=$(wc -l < $D/$k$n.c)" >> $LOG
  done
done
s=$(date +%s%N); m68k-amigaos-gcc -mcrt=nix20 -m68000 -fwrapv -std=c99 -O2 -c -o $D/g.o $D/switch1000.c; e=$(date +%s%N)
echo "gcc -O2 switch-1000: $(( (e-s)/1000000 ))ms" >> $LOG
cat $LOG
