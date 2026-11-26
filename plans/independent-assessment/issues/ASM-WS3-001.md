<!-- title: [bug] Escape C99 keywords and header macro names used as identifiers -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:codegen,tier:core,tier:advanced,bug -->
<!-- parent: R3 -->

## Summary
An accepted program emits C that no compile-matrix leg accepts. Natural C89-keyword names (`char`, `int`, `long`, `short`) are already escaped (a test pins `int`: `codegen_ownership_impl_test.rs::a_borrow_element_may_spell_a_c_keyword`); the failing names are `restrict`, `_Bool`, and macro-like names such as `EOF`, `TRUE`, `NULL`, `errno`, `stdout` — plausible in a parser or a port of C code. The failure is closed (no wrong binary runs) but the program cannot be built. (Impact narrowed in task 8.2.1.)

| Field | Value |
|---|---|
| Finding | ASM-WS3-001 · DEF-589 |
| Severity / Priority / Rank | Critical / P0 / 8 of 58 |
| Domain / Lens | codegen / all |
| Confidence / Verdict | verified / confirmed (Critical, verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — probes `probes/WS3/340-c-keywords-as-names.clo` and `341-header-macro-names.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS3 --clc build/tmp/assessment/baseline/clc --sanitize --only 340 341"`. Expected (spec/01 §2-§3: `int`, `char`, `long`, `register`, `TRUE`, `NULL`, `EOF`, `stdout`, `errno` are legal Clorraine identifiers; spec/17 §7: "Local: source name if it is a valid, unique C identifier; otherwise a stable suffix"; §1: one valid C99 unit): both programs build and print 121 and 120. Observed on host, hosted-amiga, and hosted-amiga-13, every build line: the C compiler rejects the unit. **Corrected in task 8.2.1** (re-reading the emitted C of 340): the C89 keywords and `inline` are escaped (`i32 cl_int = 0;`, `cl_char`, `cl_long`, `cl_short`, … `cl_goto`, `cl_inline`, `cl_union`); the unescaped names are the C99 keywords `restrict` and `_Bool` (`i32 restrict = 0;`, `i32 _Bool = 0;` → `expected identifier or '(' before '=' token`, `expected expression before 'restrict'`) and, in 341, the header and NDK macros `NULL`, `EOF`, `errno`, `stdout`, `TRUE`, `FALSE`, `INT_MAX`, `CHAR_BIT`, `EXIT_SUCCESS`, `MEMF_CHIP`, `BYTE`, `APTR`, `ULONG` (`lvalue required as left operand of assignment` and related errors). The diagnostics are reported at `stdlib/std.io.clo` lines (ASM-WS3-005).

Probe `plans/independent-assessment/probes/WS3/340-c-keywords-as-names.clo` (baseline `e24a8752`):

```
module Probe.Main;

import { println } from std.io;

func register(auto: i32): i32 {
    return auto + 1;
}

func main(): i32 {
    let int: i32 = 1;
    let char: i32 = 2;
    let signed: i32 = 3;
    let unsigned: i32 = 4;
    let long: i32 = 5;
    let short: i32 = 6;
    let double: i32 = 7;
    let float: i32 = 8;
    let extern: i32 = 9;
    let typedef: i32 = 10;
    let goto: i32 = 11;
    let restrict: i32 = 12;
    let inline: i32 = 13;
    let union: i32 = 14;
    let _Bool: i32 = 15;
    let total: i32 = int + char + signed + unsigned + long + short + double + float + extern + typedef + goto + restrict + inline + union + _Bool;
    println($"{register(total)}");
    return 0;
}
```

## Root cause
the emitter's escape list covers the C89 keywords (and `inline`) but not the C99 keywords `restrict` and `_Bool`, nor the macro and global names the included headers and the shim define.

Location: local naming in `compiler/src/codegen/function.rs` (locals keep their source spelling; spec/17 §7) at e24a8752

## Recommended fix
escape locals (and parameters) whose spelling is a C99 keyword, a reserved name, or a name defined by the included headers or the shim (a fixed deny-list per profile), using the existing stable-suffix scheme; add run goldens for `char`/`int`/`long` locals and for `NULL`/`EOF`/`errno`.

## Acceptance criteria
- The behavior the specification requires: both programs build and print 121 and 120 (source: spec/01 §2-§3: `int`, `char`, `long`, `register`, `TRUE`, `NULL`, `EOF`, `stdout`, `errno` are legal Clorraine identifiers; spec/17 §7: "Local: source name if it is a valid, unique C identifier; otherwise a stable suffix"; §1: one valid C99 unit)
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; `make asan` and `make ubsan`; `make corpus-amiga` (m68k GCC and vbcc legs)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, emission-integrity auditor

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
ASM-WS3-002; spec/17 §7; DEF-112 and DEF-138 (collisions with compiler-generated locals, fixed). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS3-codegen.md`.
