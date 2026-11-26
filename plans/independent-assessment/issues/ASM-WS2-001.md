<!-- title: [correctness] Release owned temporaries at the end of their statement -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:ownership,tier:core,tier:advanced,bug -->
<!-- parent: R2 -->

## Summary
Owned temporaries are released at function exit (or when the same statement runs again), not at the end of their statement as spec/03 and spec/17 require. A `File` written through a temporary is still open and unflushed when the same function reopens it, so the read returns 0 bytes instead of 5 on host and hosted-amiga (probe 260). Any program that relies on a temporary's deleter running at statement end — closing, flushing, releasing a lock — behaves wrongly.

| Field | Value |
|---|---|
| Finding | ASM-WS2-001 · DEF-584 |
| Severity / Priority / Rank | Critical / P0 / 3 of 58 |
| Domain / Lens | ownership / all (host verified; the lowering is profile-independent) |
| Confidence / Verdict | verified / upgraded Major→Critical (independent verification) |
| Effort | L |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — probes `probes/WS2/226-call-result-temp.clo`, `227`, `228`, `229`, `246`, `247`, `219`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS2 --clc build/tmp/assessment/baseline/clc --sanitize --only 219 226 227 228 229 246 247"`. Expected (spec/04-expressions.md §3: "Temporaries created anywhere in a statement … live until the end of that statement and drop there in reverse creation order"; spec/17-codegen.md §"temporary … dropped at statement end in reverse creation order"; spec/decisions.md D5, PF-040): `println($"{idOf(tok(7))}"); println("next");` prints `7`, `drop 7`, `next`. Observed in the debug, release, ASan, and UBSan builds: `7`, `next`, `end`, `drop 7` — the temporary is released after every later statement of the scope. Same for argument temporaries (227: `3`, `end`, `drop 2`, `drop 1`), short-circuit and `?:` temporaries (228, 229), and projections of call results (246, 247). In a loop body (219) the condition temporary of iteration *n* drops after the iteration's earlier-declared local, reversing creation order (`drop 0`, `drop 50` instead of `drop 50`, `drop 0`). The emitted C for 248 shows the mechanism: `ClPair __cl_owned0 = {0};` at function scope, released at the `__cl_c1` cleanup label. Memory safety is intact (no sanitizer report).

Probe `plans/independent-assessment/probes/WS2/226-call-result-temp.clo` (baseline `e24a8752`):

```
module Probe.Main;

import { println } from std.io;
import { Tok, tok, idOf, show } from Probe.Tok;

func main(): i32 {
    println($"{idOf(tok(7))}");
    println("next");
    println("end");
    return 0;
}
```

## Root cause
to fix leaks of materialized temporaries (DEF-23, DEF-45), a call result that owns storage is bound to a scope owner whose release the scope ladder emits; there is no per-statement cleanup point, although the code comments state "owns its storage only until the statement ends" (`own/lower.rs:5176`). The user ruling `plans/eval-emission-integrity/00-ambiguity-register.md` AR-B2 extended a temporary to scope end only when a **view binding** keeps it alive; the implementation extends every owned temporary.

Location: `compiler/src/own/lower.rs:5174-5184` (a call result is bound to a scope owner), `compiler/src/own/lower.rs:199-205`, `materialize_owned_argument`; emitted as function-level `__cl_owned<N>` slots released by the scope cleanup ladder, at e24a8752

## Recommended fix
emit a statement-end release for statement-scoped owners (reverse creation order) and keep scope adoption only for temporaries whose view is bound past the statement (the AR-B2 case); add drop-order run goldens for call-argument, template, short-circuit, and projection temporaries.

## Acceptance criteria
- The behavior the specification requires: `println($"{idOf(tok(7))}"); println("next");` prints `7`, `drop 7`, `next` (source: spec/04-expressions.md §3: "Temporaries created anywhere in a statement … live until the end of that statement and drop there in reverse creation order"; spec/17-codegen.md §"temporary … dropped at statement end in reverse creation order"; spec/decisions.md D5, PF-040)
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; `make asan` and `make ubsan`; `make corpus-amiga` (m68k GCC and vbcc legs)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, emission-integrity auditor, semantics reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
DEF-23, DEF-45 (leak fixes that introduced scope owners); AR-B2 (`eval-emission-integrity`); SQ-WS2-1. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS2-ownership.md`.
