#!/usr/bin/env bash
# Verifier probe (new finding): vbcc's 128-nested-block limit vs emitted else-if
# chains and nested source blocks. Emits hosted-amiga C with the pinned clc and
# compiles it with vbcc (-O2) and m68k GCC (-O2), compile only.
set -uo pipefail
ulimit -f 1048576
C=build/tmp/assessment/baseline/clc
D=build/tmp/assessment/verify/9.2.3/vbcc-nest; mkdir -p $D
gen() { # kind n
python3 - "$1" "$2" "$D" <<'PY'
import sys
kind=sys.argv[1]; n=int(sys.argv[2]); d=sys.argv[3]
hdr="module Probe.Nest;\n\nimport { println } from std.io;\n\nfunc f(n: i32): i32 {\n    let r: i32 = 0;\n"
if kind=="ifchain":
    body="".join(f"    {'if' if i==0 else '} else if'} (n == {i}) {{\n        r = {i*7%1000};\n" for i in range(n)) + "    } else {\n        r = -1;\n    }\n"
else:  # nested plain blocks
    body="    " + "{ " * n + "r = n + 1; " + "} " * n + "\n"
open(f"{d}/{kind}{n}.clo","w").write(hdr+body+"    return r;\n}\n\nfunc main(): i32 {\n    println($\"{f(1)}\");\n    return 0;\n}\n")
PY
}
for spec in ifchain:100 ifchain:124 ifchain:125 ifchain:126 ifchain:127 ifchain:128 blocks:120 blocks:124 blocks:125 blocks:126 blocks:127; do
  k=${spec%%:*}; n=${spec##*:}
  gen $k $n
  if ! $C --profile hosted-amiga -o $D/$k$n.c $D/$k$n.clo > $D/$k$n.clc.log 2>&1; then echo "$k-$n: clc rejected: $(head -1 $D/$k$n.clc.log)"; continue; fi
  timeout 300 vc +aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include -c -o $D/$k$n.o $D/$k$n.c > $D/$k$n.vbcc.log 2>&1; v=$?
  timeout 300 m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -c -o $D/$k$n.g.o $D/$k$n.c > $D/$k$n.gcc.log 2>&1; g=$?
  echo "$k-$n: vbcc rc=$v $(grep -m1 -o 'too many ([0-9]*) nested blocks' $D/$k$n.vbcc.log) gcc rc=$g"
done
