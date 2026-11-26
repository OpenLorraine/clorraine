<!-- title: [correctness] Accept slice of a str as the standard library specifies -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:stdlib,tier:core,tier:advanced,bug -->
<!-- parent: R10 -->

## Summary
Text code cannot take a substring view without copying (`stringOf` of bytes is not available either: `str` from `u8[]` has no safe conversion), so parsers either copy or work on `u8[]` and lose the `str` type. A valid program per the appendix is rejected.

| Field | Value |
|---|---|
| Finding | ASM-WS6-002 · DEF-603 |
| Severity / Priority / Rank | Major / P1 / 22 of 58 |
| Domain / Lens | stdlib / all |
| Confidence / Verdict | verified / confirmed (Major) (independent verification) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — probe `probes/WS6/612-cstring-of-slice.clo` (`let part: str = slice(full, 0, 36);`), command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS6 --clc build/tmp/assessment/baseline/clc --only 612"`. Expected (spec/appendix-stdlib.md §11: `slice(view, lo, hi)` — "view -> same view type", no allocation; spec/08 §3: the stdlib `slice` helper is a source of views; the expectation file pre-registers a rejection as this finding): the program compiles. Observed on all three profiles: `error E10200: cannot convert '?[]' to 'str' implicitly`.

Probe `plans/independent-assessment/probes/WS6/612-cstring-of-slice.clo` (baseline `e24a8752`):

```
module Probe.Main;

import { println, File, createFile, openFile, MODE_OLDFILE } from std.io;
import { toCString } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let full: str = "build/tmp/assessment/WS6/f612-prefix-TAIL";
    let part: str = slice(full, 0, 36);
    let made: File? = createFile(toCString(part));
    let created: bool = made != null;
    let prefix: File? = openFile("build/tmp/assessment/WS6/f612-prefix", MODE_OLDFILE);
    let whole: File? = openFile("build/tmp/assessment/WS6/f612-prefix-TAIL", MODE_OLDFILE);
    let p: bool = prefix != null;
    let w: bool = whole != null;
    println($"created {created} prefix {p} full {w}");
    return 0;
}
```

## Root cause
`slice` is declared over `T[]` only; `str` is a distinct view type, and the intrinsic has no `str` overload.

Location: `stdlib/std.mem.clo:40` (`intrinsic func slice<T>(v: T[], lo: usize, hi: usize): T[]` — `str` does not unify with `T[]`), the intrinsic's checking in `compiler/src/sema/call.rs:114`, `:341`, `:557`, at e24a8752

## Recommended fix
give `slice` a `str` form (or a `substr(s: str, lo, hi): str` helper) with the same bounds check; at the same time decide SQ-WS6-3 — `toCString` returns the view's pointer without a terminator at the view's end, which is sound today only because every `str` ends at a NUL.

## Acceptance criteria
- The behavior the specification requires: ` — "view -> same view type", no allocation; spec/08 §3: the stdlib `slice` helper is a source of views; the expectation file pre-registers a rejection as this finding): the program compiles (source: spec/appendix-stdlib.md §11: `slice(view, lo, hi)
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; `make asan` and `make ubsan`
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, semantics reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
SQ-WS6-3 (`toCString` of a sub-view); ASM-WS1-003 (the earlier spec-vs-implementation gap class). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS6-stdlib.md`.
