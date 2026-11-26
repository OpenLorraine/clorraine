<!-- title: [bug] Emit else-if chains without one nested C block per arm -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:codegen,tier:core,bug -->
<!-- parent: R11 -->

## Summary
A program with a long `else if` dispatch (a command parser, an opcode or key table) cannot be built on the vbcc leg; it also rules out the `if`-chain workaround for ASM-WS3-007. Build-time failure, no wrong binary.

| Field | Value |
|---|---|
| Finding | ASM-WS3-008 · DEF-608 |
| Severity / Priority / Rank | Major / P1 / 27 of 58 |
| Domain / Lens | codegen / vbcc leg of hosted-amiga (`vc +aos68k -c99 -O2 -cpu=68000`); the m68k GCC and host legs accept the same unit |
| Confidence / Verdict | verified / confirmed (Major, verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — `probes/WS3/verify-9.2.3/vbcc-nest.sh` (committed after the verifier's run), command `make -C docker exec CMD="bash plans/independent-assessment/probes/WS3/verify-9.2.3/vbcc-nest.sh"`: clc accepts every N; vbcc compiles 100, 124–126 arms; at 127 arms vbcc ends with `Segmentation fault (core dumped)`; at 128, 500, and 1,000 it fails with `error 9 … too many (128) nested blocks`; m68k GCC compiles every N. The emitted C nests one block per arm. A control with 127 nested plain source blocks compiles (plain blocks are flattened). C99 §5.2.4.1 guarantees only 127 nesting levels of blocks; 127 arms reach depth 128.

Probe `plans/independent-assessment/probes/WS3/verify-9.2.3/vbcc-nest.sh` (baseline `e24a8752`):

```sh
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
  timeout 300 vc +aos68k -c99 -O2 -cpu=68000 -I/opt<licensed Amiga media> -c -o $D/$k$n.o $D/$k$n.c > $D/$k$n.vbcc.log 2>&1; v=$?
  timeout 300 m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -c -o $D/$k$n.g.o $D/$k$n.c > $D/$k$n.gcc.log 2>&1; g=$?
  echo "$k-$n: vbcc rc=$v $(grep -m1 -o 'too many ([0-9]*) nested blocks' $D/$k$n.vbcc.log) gcc rc=$g"
done
```

## Root cause
each `else` arm opens a new C block instead of `} else if (…) {` at the same depth.

Location: `compiler/src/codegen/control.rs:421-442` (`emit_conditional` emits each false branch inside a new `} else {` block), at e24a8752

## Recommended fix
emit `} else if (cond) {` when the false branch is a single conditional with no hoisted declarations or cleanup; otherwise fall back to the existing `goto` form past a fixed depth (`control.rs:415`); add a vbcc build golden with a 200-arm chain.

## Acceptance criteria
- The behavior stated under Recommended fix, checked against the cited spec section or autodoc
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; `make asan` and `make ubsan`; `make corpus-amiga` (m68k GCC and vbcc legs)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, emission-integrity auditor

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
ASM-WS3-007; ASM-WS1-004. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS3-codegen.md`.
