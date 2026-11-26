<!-- title: [bug] Report read errors from readFile distinctly from end of file -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:stdlib,tier:core,tier:advanced,bug -->
<!-- parent: R10 -->

## Summary
A program cannot tell a failed read from the end of a file: a copier or loader stops early and reports success with truncated data (a bad floppy sector on the Amiga, a removed volume, a directory given as a file). The `std.io` doc comment groups the two ("zero or less means end of file or an error"), but no value is ever negative, so even that reading cannot be acted on. The `file-copier` corpus program follows exactly this loop.

| Field | Value |
|---|---|
| Finding | ASM-WS6-001 · DEF-594 |
| Severity / Priority / Rank | Major / P1 / 13 of 58 |
| Domain / Lens | stdlib / host (reproduced); A500, A1200 shipped (the same `fread` code) |
| Confidence / Verdict | verified / confirmed (Major) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — probe `probes/WS6/617-read-error.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS6 --clc build/tmp/assessment/baseline/clc --sanitize --only 617"`. Expected (`617-read-error.expected`: spec/appendix-stdlib.md §4 — partial IO returns the count moved plus a status; failures are not hidden; C99 7.19.8.1 — end of file and error are told apart only by `feof`/`ferror`): a failing read (a directory opened for reading, `EISDIR`) and a clean end of file report differently ("true true"). Observed on host debug, release, ASan, and UBSan: "true false" — both return 0; the contrast (two clean ends of file) prints "true false" as expected.

Probe `plans/independent-assessment/probes/WS6/617-read-error.clo` (baseline `e24a8752`):

```
module Probe.Main;

import { println, File, createFile, openFile, readFile, MODE_OLDFILE } from std.io;
import { slice } from std.mem;

func readOnce(path: cstring): i32 {
    let f: File? = openFile(path, MODE_OLDFILE);
    if (f == null) {
        return -100;
    }
    let file: File = f;
    let buf: u8[16] = [; 0];
    return readFile(file, slice<u8>(buf, 0, 16));
}

func main(): i32 {
    let made: File? = createFile("build/tmp/assessment/WS6/f617-empty");
    let ok: bool = made != null;
    let failing: i32 = readOnce("build");
    let empty: i32 = readOnce("build/tmp/assessment/WS6/f617-empty");
    let distinct: bool = failing != empty;
    println($"{ok} {distinct}");
    return 0;
}
```

## Root cause
the shim maps the AmigaDOS-style `Read` contract (−1 on error) onto `fread`, which has no error return.

Location: `stdlib/std.io.clo:46-54` (`readFile` returns `Read`'s count), `compiler/runtime/shim_host.c:645-648` and `compiler/runtime/shim_hosted_amiga.c:1259-1262` (`Read` is `fread`, which returns 0 on both end of file and error; `ferror` is never consulted); `writeBytes` / `Write` likewise report a failed write only as a short count, at e24a8752

## Recommended fix
return −1 when `ferror` is set (and clear it), as the AmigaDOS `Read` contract does, and document the −1; or return the spec §4 count-plus-`Status` pair. Same for `Write`.

## Acceptance criteria
- The behavior the specification requires: a failing read (a directory opened for reading, `EISDIR`) and a clean end of file report differently ("true true") (source: `617-read-error.expected`: spec/appendix-stdlib.md §4 — partial IO returns the count moved plus a status; failures are not hidden; C99 7.19.8.1 — end of file and error are told apart only by `feof`/`ferror`)
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; `make asan` and `make ubsan`
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
SQ-WS6-2 (the §4 carrier); ASM-WS6-004 (`std.io` has no appendix table). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS6-stdlib.md`.
