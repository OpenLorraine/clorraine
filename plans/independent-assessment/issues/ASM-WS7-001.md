<!-- title: [performance] Lower module-level array fills without one store per element -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:perf,tier:core,tier:advanced,performance -->
<!-- parent: R12 -->

## Summary
A demo or game that declares a module-level buffer with a fill (a 16 KB sample buffer, a 10 KB bitplane, a lookup table of a constant) gets about 8 bytes of 68000 code per element and runs that code at startup; at 64 KB the image no longer fits the A500, and the C build takes minutes. A zero fill pays the same cost although the static is already zero.

| Field | Value |
|---|---|
| Finding | ASM-WS7-001 · DEF-596 |
| Severity / Priority / Rank | Major / P1 / 15 of 58 |
| Domain / Lens | perf / A500 (measured); A1200 shipped and host (the same emission) |
| Confidence / Verdict | verified / confirmed (Major, verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Measurement — probe `probes/WS7/740-fill-emission.sh`, command `make -C docker exec CMD="bash plans/independent-assessment/probes/WS7/740-fill-emission.sh build/tmp/assessment/baseline/clc build/tmp/assessment/WS7/fill"`, A500 release line (`-mcrt=nix13 -m68000 -O2`, section GC, strip), output checked under `vamos` ("1" in every case). Module-level `u8` fill: 1,024 → 7,216 C lines, GCC 3.0 s, `.text` 25,932 B; 16,384 → 22,576 lines, 38.4 s, `.text` 154,460 B; 65,536 → 71,728 lines, 163.1 s, `.text` 547,676 B (image 613,612 B > 512 KB). The same fill as a function local: 6,193 lines and about 2.2 s at every size, `.text` 23,384 B at 65,536. A 614,400-element module fill (also with `[; 0]`) emits a 22 MB unit that GCC did not finish in 30 minutes (`records/WS7-runs.log`). Expected (`740-fill-emission.expected`: spec/08 §2 fill semantics; 03-01 §Severity C-compile limit about 32 s): size-independent C and GCC within 32 s.

Phase 9 notes: emitter site traced by the verifier: `compiler/src/codegen/program.rs:1974`, `:2058-2085`; related to ASM-WS4-004 (both lower module initializers as run-time stores; different shapes and remedies).

Probe `plans/independent-assessment/probes/WS7/740-fill-emission.sh` (baseline `e24a8752`):

```sh
#!/usr/bin/env bash
# WS7-06 / WS3-11: emitted C size and m68k GCC compile time of a non-zero fill literal
# `[; 1]` for u8 arrays of 1,024, 16,384, and 65,536 elements, at module level and as a
# function local, on the A500 release line.
# Usage (SDK container, repo root): 740-fill-emission.sh <clc> <out-dir>
set -uo pipefail
clc=$1; out=$2; mkdir -p "$out"
flags=(-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s)
for n in 1024 16384 65536; do
  for where in module local; do
    src="$out/$where-$n.clo"
    if [ "$where" = module ]; then
      printf 'module Probe.Main;\n\nimport { println } from std.io;\n\nlet big: u8[%d] = [; 1];\n\nfunc main(): i32 {\n    println($"{big[%d]}");\n    return 0;\n}\n' "$n" "$((n - 1))" > "$src"
    else
      printf 'module Probe.Main;\n\nimport { println } from std.io;\n\nfunc main(): i32 {\n    let big: u8[%d] = [; 1];\n    println($"{big[%d]}");\n    return 0;\n}\n' "$n" "$((n - 1))" > "$src"
    fi
    "$clc" --profile hosted-amiga-13 -o "$out/$where-$n.c" "$src" > "$out/$where-$n.clc.log" 2>&1 || { echo "FAIL $where-$n: clc"; continue; }
    lines=$(wc -l < "$out/$where-$n.c"); bytes=$(wc -c < "$out/$where-$n.c")
    start=$(date +%s%N)
    if timeout 600 m68k-amigaos-gcc "${flags[@]}" -o "$out/$where-$n" "$out/$where-$n.c" -lamiga -lm > "$out/$where-$n.gcc.log" 2>&1; then
      ms=$(( ($(date +%s%N) - start) / 1000000 ))
      outp=$(vamos "$out/$where-$n" 2>/dev/null | tr -d '\r')
      echo "$where-$n: C $lines lines $bytes B; gcc ${ms} ms; output '$outp'"
    else
      echo "$where-$n: C $lines lines $bytes B; gcc FAILED or timed out (600 s)"
    fi
  done
done
```

## Root cause
the module initializer expands the fill into per-element stores instead of a loop (or, for zero, nothing beyond the static `{0}`) as the function-local lowering does.

Location: module-initializer lowering of `let g: T[N] = [; v];` (emitted into the module's init function as `cl_<module>_<name>[i] = v;` for every `i`, after a static `= {0}`; the exact emitter site was not traced), at e24a8752

## Recommended fix
lower a module-level fill like the local one (a loop, or `memset` for a byte fill); emit nothing for a zero fill; for a constant fill of a `const`-able value consider a static initializer.

## Acceptance criteria
- The behavior the specification requires: size-independent C and GCC within 32 s (source: `740-fill-emission.expected`: spec/08 §2 fill semantics; 03-01 §Severity C-compile limit about 32 s)
- A regression test that fails without the fix and passes with it
- The measured target on the stated lens, with the same lens and flags as the evidence (net 68000 instructions under `vamos` or `.text` bytes; compile time for the compile-time items)

## Verification
- AGENTS.md gates: `make check`; `make bench` and `make bench-compare` (and `make bench-compile` for compile time)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
ASM-WS4-004 (module tables occupy RAM twice); DEF-309 (the fill expression evaluated per element — a different, fixed defect); WS3-11. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS7-performance.md`.
