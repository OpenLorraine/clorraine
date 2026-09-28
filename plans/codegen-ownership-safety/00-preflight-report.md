# Preflight Report: Codegen Ownership Safety

> **Status**: ✅ PASSED — all 17 findings resolved (16 accepted and applied; PF-003 withdrawn after challenge)
> **Iteration**: 2 (re-scan after fixes)
> **Previous Iteration**: 17 findings (2 critical, 1 major, 12 minor, 2 observations) — all resolved
> **This Iteration**: 0 new findings
> **Artifact**: implementation plan (`plans/codegen-ownership-safety/`, 8 documents) — content hash `9a2d0e03121d2498` (iteration 1 revision: `8c16e6718bba7c4e`)
> **Codebase Grounded**: ~35 source/test files examined, 61 references mapped; live compiler + clang ASan + gcc probes
> **Last Updated**: 2026-09-27
>
> ⚠️ SAME-SESSION REVIEW: this plan was created in the current session. Same-agent bias risk is
> elevated; the scan used 5 independent auditor clusters plus one challenger for the blocking
> findings. Consider a fresh-session audit for the next iteration.
>
> **Scope**: strict (no optional expansion); **Mode**: normal.

### Codebase Context Summary

**Tech Stack:** Rust workspace (std-only), C99 emitted TU, Clorraine sources; container toolchain via `make -C docker exec`; host-only clang ASan.
**Architecture:** AST → `resolve` → `sema` → ownership walk (`own/`, produces typed IR with `PlaceBase` bases, bindings, scopes, cleanup ladder) → `codegen/` (C99) → shims.
**Key Files Examined:** `own/loops.rs`, `own/statements.rs`, `own/place.rs`, `own/flow.rs`, `own/lower.rs`, `own.rs`, `ir/place.rs`, `ir.rs`, `codegen/function.rs`, `codegen/expr.rs`, `codegen/guards.rs`, `codegen/control.rs`, `codegen/types.rs`, `codegen.rs`, `tests/codegen_run_spec_test.rs`, `tests/freestanding_spec_test.rs`, `spec/{03,08,10,16}`, `runtime/shim_freestanding.c`.
**Key Observations:**
- Normal scope exit does not truncate bindings (`own.rs:356`); truncation happens only in speculative rollback (`own.rs:478/:522/:552`), so a binding-index-based `ir_name` is stable and unique across sibling scopes.
- Captures are `BindingKind::Param` with `base = EnvField` (`own/lower.rs:459-469`) and were never in `Function.locals`; the 02 risk row about captures is phantom.
- `Emitter::empty_state` maps `NULL` to `0` on hosted-amiga/freestanding (`codegen.rs:323-332`), so the proposed raw temps initialize safely on freestanding.
- A pre-existing, unfiled defect: reading an owned array element as a view argument materializes an owned temporary without moving the slot (`own/lower.rs:657` `materialize_owned_argument` skips `Place` but not `Index`) and double-frees (`vlen(arr[0])`; probes `build/defs/read1.c`, `read2.c`, ASan).
- A pre-existing, unfiled defect: nested element loops hang because `for_step_block` (`codegen/control.rs:407-420`) picks the enclosing body block as the step; the test harness has no timeout.

### Summary by Dimension

| # | Dimension | Findings |
|---|-----------|----------|
| 1 | Ambiguities | 3 |
| 2 | Implicit Assumptions | 1 |
| 3 | Logical Contradictions | 1 |
| 4 | Completeness Gaps | 4 |
| 5 | Dependency Issues | 1 |
| 6 | Feasibility Concerns | 1 |
| 7 | Testability | 6 |
| 8 | Security Blind Spots | 0 |
| 9 | Edge Cases | 3 |
| 10 | Scope Creep Indicators | 1 |
| 11 | Ordering & Sequencing | 2 |
| 12 | Consistency | 6 |
| 13 | Codebase Alignment | 4 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 2 | 2 pending |
| MAJOR | 1 | 1 pending |
| MINOR | 12 | 12 pending |
| OBSERVATION | 2 | 2 pending |

---

### PF-001: Element-read double free blocks the element ST oracle [🔴 CRITICAL]

**Dimension:** 13 Codebase Alignment (Impact Blindness), 7 Testability
**Location:** `07-testing-strategy.md` ST-1 – ST-7 (post-loop `vlen(arr[0])` / `vlen(ps[0].name)` observations)
**Codebase Evidence:** `own/lower.rs:657` (`materialize_owned_argument` skips `ValueKind::Place` but not `Index`); probes `build/defs/read1.c`, `build/defs/read2.c` double-free under ASan; `vlen` of an owned local is clean (`build/defs/read5.clo`)
**The Problem:** reading an owned array element as a view argument lowers to an owned temporary without moving/zeroing the slot, so the array cleanup and the temporary free the same allocation. ST-1 – ST-7 as written would still abort after the scoped DEF-13 fix. Different root cause from DEF-13 (expression materialization, not loop lowering); 02's `f13`/`f13r` evidence is clean because those programs do not read elements.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Redesign element ST cases to observe duplicable state (struct + scalar fields / globals); file the read-path defect as a new DEF | Keeps AR-1 scope; case can be red for DEF-13 and green after | R1 string write-through verified structurally (impl/ASan), not by reading the element |
| B | Expand this plan to fix element-read materialization | Full runtime oracle | AR-1 amendment; third design; larger risk |
| C | Fix the read-path defect first in a separate plan, then execute | Cleanest oracle eventually | Blocks this plan now |

**Recommendation:** Option A — the challenger reproduced the defect and confirmed it is a distinct root cause under the same scope rule used to exclude DEF-15; option C is the fallback if the user wants the stronger oracle first. · **Confidence:** High · **Hardening:** challenger converged; diverged only to name option C as the alternative.
**Related:** AR #1 (scope), AR #10 (same exclusion pattern).
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-002: Nested element loops hang on a separate unfiled defect [🔴 CRITICAL]

**Dimension:** 6 Feasibility, 7 Testability, 9 Edge Cases, 11 Ordering
**Location:** `07-testing-strategy.md` ST-6; `03-01-borrow-element-alias.md` testing requirements (nested); `99-execution-plan.md` 1.3.1
**Codebase Evidence:** `codegen/control.rs:407-420` (`for_step_block`); emitted inner loop `for (; (t1 < 2); t2 = ..., t1 = 0u)` resets its own index; reproduced with nested copy loops (no borrow/drop) and with ST-6 (`exit 124` under timeout); `codegen_run_spec_test.rs:154` has no timeout
**The Problem:** nested element loops never advance; ST-6 cannot turn green, the nested implementation-test item is unachievable, and RED verification would hang `cargo test` rather than fail.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Replace ST-6 with two sequential element loops; drop the nested impl item; file the nested-loop step defect as a new DEF; add a harness timeout | Keeps AR-1 scope; still exercises alias refresh and multiple aliases | Loses nested alias/scope-floor interaction coverage |
| B | Include the `control.rs` step-selection fix in this plan | Keeps ST-6 | Scope expansion; new root cause; new design + specs |

**Recommendation:** Option A — the challenger verified the hang is general (copy loops too) and that ST-6 is impossible in scope; add the harness timeout as cheap protection. · **Confidence:** High · **Hardening:** challenger converged and proposed the timeout addition.
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-003: (WITHDRAWN) Freestanding raw-temp `NULL` initializer [—]

**Dimension:** 8/13 · **Location:** freelist candidate reported by one auditor.
**Codebase Evidence:** `Emitter::empty_state` (`codegen.rs:323-332`) replaces `NULL` with `0` for `HostedAmiga | Freestanding`; direct reproduction emits `i32 *p = 0;` and compiles clean under `-ffreestanding -nostdlib -Werror`. **The finding is false; no action.** (Coverage gap retained as PF-005.)

### PF-004: ST-3 is not red today [🟠 MAJOR]

**Dimension:** 7 Testability
**Location:** `07-testing-strategy.md` ST-3; `99-execution-plan.md` 1.1.2 (RED gate)
**Codebase Evidence:** `own/statements.rs:530` (`check_break` runs the write-back before cleanup); loop-only probe `build/defs/st3loop.clo` is ASan-clean and exits 0 today; the full ST-3 abort comes from PF-001's read path
**The Problem:** assign-then-`break` in the first iteration is repaired by the write-back, so the case passes for the wrong reason and cannot satisfy the "fail for the defect reason" gate.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Reshape ST-3 so a write survives one iteration and the `break` happens in a later one; expected exit unchanged | Genuinely red; keeps break coverage | No longer pins break-in-first-iteration (already covered by the existing run case) |
| B | Keep ST-3 as a green guard and document RED exceptions | No edit | Violates the stated RED gate |

**Recommendation:** Option A — challenger independently verified and converged. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-005: No off-host coverage of the new construct [🟡 MINOR]

**Dimension:** 4 Completeness (coverage), 7
**Location:** `01-requirements.md` Compatibility; `99-execution-plan.md` 3.1.2
**Codebase Evidence:** no `for (borrow ...)` anywhere under `spec/tests/`; `docker/scripts/corpus-amiga.sh:36` builds four programs without one
**The Problem:** R4/AR #13 claim every profile and both C toolchains keep working, but the m68k/vbcc legs never compile the new alias C. (The freestanding initializer itself is safe — see PF-003.)
**Options:** (a) add a build-only freestanding/m68k compile case for a borrow-element loop (pattern `clorraine_driver_spec_test.rs:249`); (b) narrow the R4 wording to host-verified plus existing corpus legs.
**Recommendation:** (a) — small and makes the requirement real. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-006: `ir_name` example contradicts the stated rule [🟡 MINOR]

**Dimension:** 1 Ambiguity, 12 Consistency
**Location:** `03-02-local-identity.md` §Proposed Changes and MOD-16 example; `00-ambiguity-register.md` AR #11
**Codebase Evidence:** `own.rs:604-641` (`id = bindings.len()`; `declare_param` goes through `declare`), `own.rs:356` (no truncation on scope exit)
**The Problem:** under the stated "position in `bindings`" rule, the MOD-16 local is `LIMIT#2` (param 0, `doubled` 1, local 2), not the documented `LIMIT#1`; "live binding table" is undefined because closed scopes stay in the table and must count.
**Options:** (a) fix the examples to `LIMIT#2` and state that every binding declared so far counts; (b) switch to a per-name collision ordinal (needs an AR amendment).
**Recommendation:** (a) — preserves the AR-approved scheme; one-line example fix plus one clarification. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-007: `Function.locals` predicate ambiguous; capture risk row phantom; stale citations [🟡 MINOR]

**Dimension:** 1, 12, 13 (Stale Assumptions)
**Location:** `03-01-borrow-element-alias.md:97-98`, `03-02-local-identity.md:45-46,74-76`, `02-current-state.md:24,145`, AR-2 note
**Codebase Evidence:** `own.rs:303-311` / `own/lower.rs:382-390` filter `kind == Local` only; captures are `BindingKind::Param` (`own/lower.rs:468`); `Binding.base` is at `flow.rs:62`; break/continue hooks at `statements.rs:523`/`:547`
**The Problem:** read literally, `base.is_none()` alone admits params/out-slots into `Function.locals` (duplicate C declarations); the capture risk row describes a hazard that does not exist; three citations drift by 1–7 lines.
**Options:** (a) state the predicate as `kind == BindingKind::Local && binding.base.is_none()`, rewrite the risk row for the real hazard (dropping the kind predicate leaks parameters), fix citations.
**Recommendation:** (a) — the only viable reading; no strawman alternative. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-008: R6 names a symbol that does not exist [🟡 MINOR]

**Dimension:** 12 Consistency
**Location:** `01-requirements.md` R6 and acceptance criterion 3; `99-execution-plan.md` 1.2.3
**Codebase Evidence:** no `store_for_write` in `compiler/src`; real chain `store_for_in_element` (`own/loops.rs:365`) called by `run_for_in_write` (`own/statements.rs:567`)
**The Problem:** R6 lists `store_for_write` (phantom) and omits `run_for_in_write`, so the requirement and its acceptance check are inaccurate.
**Recommendation:** replace `store_for_write` with `run_for_in_write` in R6. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation

### PF-009: `declare_captures` bypasses `declare`; task file list incomplete [🟡 MINOR]

**Dimension:** 4 Completeness, 5 Dependency, 13
**Location:** `03-02-local-identity.md` §Proposed Changes; `99-execution-plan.md` 2.2.1 (files: `own/flow.rs`, `own.rs`)
**Codebase Evidence:** `own/lower.rs:459-475` pushes `Binding` directly (one of only two construction sites)
**The Problem:** adding `ir_name` breaks that literal; `own/lower.rs` is not in 2.2.1, and the spelling semantics for base-overridden captures are unstated (uniqueness would hold by accident).
**Options:** (a) add `own/lower.rs` to 2.2.1 and state captures keep the source name (exempt, never read for places). (b) add a shared helper.
**Recommendation:** (a) — matches the actual contract; the helper is unjustified surface. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-010: 03-01 implementation-test list diverges from 07/99 [🟡 MINOR]

**Dimension:** 7, 12
**Location:** `03-01-borrow-element-alias.md` §Testing Requirements vs `07-testing-strategy.md` impl table and `99-execution-plan.md` 1.3.1
**The Problem:** "replacement read-back within the same iteration" appears in no task; slice wording is unconditional in 03-01 and conditional in 07/99 (slice iteration is expressible — probe `build/defs/st7s.clo` compiles with `import { slice } from std.mem`).
**Options:** (a) add the read-back case to 07/1.3.1 and align the slice wording; (b) delete the extra items from 03-01.
**Recommendation:** (a) — immediate read-back is a distinct property not asserted by ST-1/ST-2 and is cheap. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-011: ST-11 scenario understates the loop bound [🟡 MINOR]

**Dimension:** 1 Ambiguity
**Location:** `07-testing-strategy.md` ST-11
**The Problem:** exit 16 follows only for a two-iteration loop; the bound is not stated, violating the document's own derivability rule.
**Recommendation:** state the two-iteration loop explicitly in the row. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation

### PF-012: Rendering unit test routed to a module that cannot reach codegen [🟡 MINOR]

**Dimension:** 11 Ordering, 13 Convention Violations
**Location:** `99-execution-plan.md` 1.2.1 (`compiler/src/ir/tests.rs`)
**Codebase Evidence:** `ir/tests.rs:1-5` deliberately excludes the C emitter; `place_base`/`place_base_type` are private `Emitter` methods; codegen unit tests live in `codegen.rs:565`, `types.rs:192`
**The Problem:** the stated rendering test cannot be written where the task puts it; the exhaustiveness mitigation is not actually implemented.
**Options:** (a) keep a variant/typing assertion in `ir/tests.rs` and verify rendering in the impl suite / codegen tests (compiler enforces exhaustive matches); (b) add an `Emitter` fixture to codegen.
**Recommendation:** (a) — matches existing conventions and the impl suite already reads emitted C. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-013: Deleting the write-back removes the only `used` marking [🟡 MINOR]

**Dimension:** 4 Completeness, 13 Impact Blindness
**Location:** `03-01-borrow-element-alias.md` step 4; `99-execution-plan.md` 1.2.3; `01-requirements.md` R6
**Codebase Evidence:** `own/loops.rs:372` sets `used = true`; `own/statements.rs:222-230` reports W10000 for unused locals; probe: `for (borrow e in xs) {}` is silent today, `for (e in xs) {}` warns
**The Problem:** after the deletion, an unused borrow element gains W10000 — a user-visible change (and an error under `--deny-warnings`) that AR #16 and the test plan do not mention.
**Options:** (a) keep today's behavior: set `used = true` for the borrow alias in `bind_for_in_element`; (b) accept the warning for consistency with copy elements and pin it with a test.
**Recommendation:** (a) — compatibility-preserving and one line; (b) is defensible but is a diagnostics change needing an explicit ruling. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-015: "make check green after every task" contradicts the RED tasks [🟡 MINOR]

**Dimension:** 12 Consistency
**Location:** `01-requirements.md` acceptance criterion 5; `99-execution-plan.md` Success Criteria 2
**The Problem:** RED tasks intentionally leave the suite failing; the criterion as worded cannot hold.
**Recommendation:** reword to "green after every GREEN task; RED tasks verify by failing for the defect reason". · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation

### PF-016: AR-9 coverage extended beyond its decision text; ST-15 not expressible as stated [🟡 MINOR]

**Dimension:** 10 Scope, 9 Edge Cases, 7
**Location:** `01-requirements.md` R3; `03-02-local-identity.md`; `07-testing-strategy.md` ST-14/ST-15; AR #9
**Codebase Evidence:** AR #9 names local/local, local/parameter, const, defer — not generics or lambda bodies; probe `build/defs/st15b.clo` → `E10400: capturing lambda cannot be used as 'func(...)'`; the supported form is a callable-bound generic (`func apply<T, F: func(T): T>`, pattern `v1_language_spec_test.rs:346-372`) with an explicit `[base]` capture list
**The Problem:** generic/lambda shadowing are good coverage but not traceable to AR #9, and ST-15 as written cannot compile (wrong lambda form).
**Options:** (a) amend AR-9 to name generic instantiations and callable-bound lambda bodies, and spell the supported ST-15 form; (b) drop ST-14/ST-15 and the lambda impl test.
**Recommendation:** (a) — one AR amendment plus a corrected ST-15; the coverage is real and cheap in the callable-bound form. · **Confidence:** Medium (ST-14 confirmed feasible; ST-15 needs the corrected form)
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-017: 03-01 error row about moves through the alias is false [🟡 MINOR]

**Dimension:** 9 Edge Cases, 12
**Location:** `03-01-borrow-element-alias.md` §Error Handling ("the existing move rules still reject it")
**Codebase Evidence:** probe `build/defs/movealias.clo` (`consume(take e)` inside a borrow element loop) compiles (`clc=0`); `own/place.rs:185-196` restricts only `BindingKind::Param`, and a borrow element is `BindingKind::Local`
**The Problem:** the row misdocuments the behavior; a reviewer could write the wrong expectation, and the underlying view-move hole is unfiled. No memory-safety regression is introduced by this plan.
**Options:** (a) correct the row to the actual (steal) semantics and file the view-move hole as a separate DEF; (b) add a checker restriction (new language behavior — needs a ruling).
**Recommendation:** (a) — out of AR-1 scope; the row must stop claiming rejection. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation (Option A)

### PF-014: Phase 3 ASan probe task is not reproducible [🔵 OBSERVATION]

**Dimension:** 4 Completeness
**Location:** `99-execution-plan.md` 3.1.1; `07-testing-strategy.md` E2E table
**The Problem:** no commands, no sources (the probes live in gitignored `build/defs/`), no recording location; probes must be redesigned anyway per PF-001.
**Recommendation:** inline the redesigned probe programs and the exact emit/build/run commands in 07 and record results on the 3.1.1 line. · **Confidence:** High
**User Decision:** Resolved — User accepted recommendation

### PF-018: Sanitized-name collision and keyword shadowing untested [🔵 OBSERVATION]

**Dimension:** 9, 7
**Location:** `03-02-local-identity.md` error table; `07-testing-strategy.md` impl list
**The Problem:** `name#1` sanitizes to `name_1`, colliding with a source `name_1`; `unique_local` handles it, but no case pins it, nor keyword-shaped locals.
**Recommendation:** add one impl case combining a shadow (`x`, `x#1`) with a source `x_1` and a keyword-named local. · **Confidence:** Medium (value), High (facts)
**User Decision:** Resolved — User accepted recommendation

---

## Iteration 2 — Verification (2026-09-28)

Fixes applied for every accepted finding; blockers verified:

| Finding | Fix applied | Verification |
| ------- | ----------- | ------------ |
| PF-001 | Element ST-1 – ST-7 redesigned to observe the duplicable `n` field; owned element reads excluded; `DEF-16` filed | Probes `build/defs/pfst/pf1`–`pf7` compile under `clc` + gcc; pf1/pf3–pf7 abort today (134), pf2 returns 3 (wrong value); all expected exits recomputed (8, 6, 8, 5, 16, 14, 10) |
| PF-002 | ST-6 replaced by two sequential loops; nested impl item dropped; harness timeout added to 1.3.1; `DEF-17` filed | Nested hang independently reproduced (exit 124); remaining "nested" references are the exclusion notes only |
| PF-004 | ST-3 reshaped to `break` after the second write (exit 8) | Probe `pf3` aborts today; write-through now spans an iteration |
| PF-005 | Off-host build-only freestanding compile added (1.3.1, checklist, 01 Compatibility) | `Emitter::empty_state` emits `0` on freestanding (`codegen.rs:323`), so the construct compiles |
| PF-006/007/008/009 | Index rule clarified (`LIMIT#2`, closed scopes count); full `Function.locals` predicate stated in 03-01/03-02/99; capture risk row rewritten; citations fixed; R6 symbols fixed; `declare_captures` added to 2.2.1 | Residual grep: no `LIMIT#1`, `flow.rs:55`, `statements.rs:524/545`, `store_for_write`, or "live binding table" outside this report/history |
| PF-010/011/012 | Read-back and slice wording aligned; ST-11 bound stated; rendering-test placement corrected | ST table holds 15 cases; phase task count still 20 |
| PF-013 | Borrow alias marks the binding used in `bind_for_in_element` | Preserves today's W10000 silence |
| PF-014 | E2E probes specify sources, exact commands, and the recording location | — |
| PF-015 | AC5 and Success Criteria reworded for RED tasks | — |
| PF-016 | AR #9 extended; ST-15 rewritten to the callable-bound form | Probe `pf15` compiles (`clc=0`) and fails `-Werror` for the DEF-14 reason (exit 5 expected after) |
| PF-017 | Error row corrected to the observed steal semantics; `DEF-18` filed | `movealias` probe compiles (`clc=0`) |
| PF-018 | Name-collision + keyword-shadow impl case added | — |

Artifact hash after fixes: `9a2d0e03121d2498` (8 plan documents, report excluded).

## Verdict

✅ **PREFLIGHT PASSED** — all 17 findings resolved (16 accepted and applied, PF-003 withdrawn
after independent challenge); no critical, major, minor, or observation finding remains open.
Roadmap advanced to `Plan Preflighted`.
