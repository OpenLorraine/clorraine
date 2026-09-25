# Preflight Report: Clorraine v1 language completion

> **Status**: ✅ PREFLIGHT PASSED — all 9 findings resolved (3 major, 4 minor, 2 observations; 1 refuted)
> **Iteration**: 3 (targeted re-check after external changes landed)
> **Artifact**: Implementation plan at `plans/clorraine-v1-language-completion/` (10 documents)
> **Codebase Grounded**: 18 source/spec files examined, 40+ references verified
> **Last Updated**: 2026-09-25 08:58

> ⚠️ SAME-SESSION REVIEW: This artifact was created in the current session.
> Same-agent bias risk is elevated. The CRITICAL/MAJOR batch was challenged by
> one independent subagent, which refuted one finding and confirmed two. Consider
> a fresh-session review before execution for maximum independence.

## Codebase Context Summary

**Tech Stack:** Rust workspace (std-only, no crates), emitting one C99
translation unit; Clorraine `.clo` stdlib; validated in the Docker SDK container
(`make check` / `make verify`).

**Architecture:** staged pipeline `lex` → `parse` → `load` → `resolve` → `sema`
→ `own` → `ir` → `codegen`, with `types`/`consts` layers. Generic *functions* are
monomorphized (`compiler/src/ir/mono.rs`, `compiler/src/codegen/program.rs`);
generic *structs* drop their type arguments; lambdas are staged; the volatile
intrinsics lower to `0`.

**Key Files Examined:** `compiler/src/codegen/types.rs`,
`compiler/src/codegen/program.rs`, `compiler/src/ir/mono.rs`,
`compiler/src/types/table.rs`, `compiler/src/profile.rs`,
`compiler/src/stdlib.rs`, `compiler/src/sema/expr.rs`, `compiler/src/sema/mod.rs`,
`compiler/src/ast/expr.rs`, `compiler/src/token.rs`,
`compiler/tests/compile_fail_spec_test.rs`, `stdlib/std.amiga.graphics.clo`,
`spec/{06,07,14,17}-*.md`, `spec/decisions.md`, `spec/check.sh`,
`spec/appendix-{amiga,stdlib}.md`, `spec/tests/staged/**`.

**Reference Verification:** every file:line citation in `02-current-state.md` and
`03-0*.md` was checked; all resolved. Ledger IDs (G1-G8, L1-L5, T7, P7, P11, P13,
N2, X6, M2, M10, GF22, A5a, MOD-29) resolve to `spec/decisions.md` and the
chapter rule tables. No phantom references.

## Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 2 | 🟠 |
| 3 | Logical Contradictions | 1 | 🟠 |
| 4 | Completeness Gaps | 2 | 🟡 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 0 | — |
| 7 | Testability | 2 | 🟡 |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 1 | 🔵 |
| 13 | Codebase Alignment | 2 | 🟠 |

(Dimensions with no findings: 2, 8, 9, 10 — none detected.)

## Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 3 | all resolved |
| MINOR | 4 | all resolved |
| OBSERVATION | 2 | 1 resolved, 1 not-a-defect |

---

## Findings

### PF-001: Capturing closure with an explicit `func(...)` type argument [🟠 MAJOR]

**Dimension:** Ambiguities / Logical Contradictions
**Location:** `00-index.md` §Quick Reference (line 81); `03-02-lambdas.md` Code Example 2 (line 130)
**Codebase Evidence:** `spec/06-functions.md:211` ("a capturing lambda used where
`func(...)` is required is E10400"); `spec/17-codegen.md:265` ("only capture-less
lambdas match `func(...)`"); AR-10 in `00-ambiguity-register.md`.

**The Problem:** The examples wrote `apply<i32, func(i32): i32>(1, [base] ...)`,
supplying an explicit concrete `func(i32): i32` type argument for the
callable-bound `F` while passing a capturing closure. AR-10 allows a capturing
closure only *at the generic callable-bound parameter*; an explicit `func(...)`
argument makes the position a concrete func value, which is E10400 (ST-18).
The examples therefore contradicted the plan's own Key Decision ("a capturing
closure fills a generic `F: func(...)`; **not** a concrete `func(...)` type") and
ST-12's expected success.

**Options:** A) A capturing closure is accepted only when `F` is inferred; fix the
examples to omit the explicit `F` and state the rule. B) Allow it and define a new
typing rule.

**Recommendation:** Option A — matches the frozen spec and ST-18, and is the
smaller change.

**User Decision:** Resolved — user authorized applying the recommended plan-doc fixes ("Continue with 2"); independent challenger CONFIRMED-REAL.
**Fix:** Both examples now leave `F` to inference; `03-02-lambdas.md` gained
Example 4 (explicit `func(...)` → E10400); AR-10 and ST-12 state the rule.

---

### PF-002: `setColor` exported by both `std.amiga.custom` and `std.amiga.graphics` [🔵 OBSERVATION]

**Dimension:** Codebase Alignment (Redundancy)
**Location:** `03-03-volatile-mmio.md` module example
**Codebase Evidence:** `compiler/src/stdlib.rs:161` and
`stdlib/std.amiga.graphics.clo:29` export `setColor`; but the name is
spec-mandated for the custom wrapper (`spec/appendix-amiga.md:170`) and the
graphics wrapper (`spec/appendix-stdlib.md:196`).

**The Problem:** Two standard modules export `setColor` with different
signatures/semantics.

**Decision:** Not-a-defect. Exports are module-scoped; duplicate *import binds* are
E11103 and qualified access is available (`spec/10-modules.md:111-112`); C symbols
are module-mangled. Raised and refuted by the independent challenger. No change.

---

### PF-003: Struct symbol vs P11 "hash dedupe" [🟠 MAJOR]

**Dimension:** Consistency / Codebase Alignment
**Location:** AR-8 (`00-ambiguity-register.md`), R1 (`01-requirements.md`)
**Codebase Evidence:** `spec/decisions.md:224` (P11 "hash dedupe"),
`spec/17-codegen.md:242,255`, `spec/grammar.ebnf.md:531`; actual code
`compiler/src/codegen/program.rs:312,649` uses
`format!("cl_{}__{}", function, args.join("_"))` with no hash.

**The Problem:** The plan claimed conformance to P11 while deciding "No new
hashing is introduced", and its authorized spec-edit set did not reconcile P11 /
Ch 17 §7 / grammar §11 — leaving an unreconciled contradiction with the frozen
spec. The spec's hash applies to *over-long* keys only.

**Options:** A) Reuse the existing canonical-key scheme and scope the
over-long-key hash out explicitly (no frozen-spec edit). B) Implement the hash for
functions and structs. C) Edit P11/§7/§11 (a frozen-spec change needing a ruling).

**Recommendation:** Option A — smallest change; keeps the frozen spec untouched
(AGENTS.md requires a ruling for spec changes) and matches the shipped scheme.

**User Decision:** Resolved — user authorized applying the recommended plan-doc fixes; independent challenger CONFIRMED-REAL (preferred the spec-preserving option).
**Fix:** AR-8 reworded to "reuse the existing deterministic scheme … P11's
over-long-key hash is a separate, pre-existing concern"; `03-04-spec-goldens.md`
states those three sites need no edit.

---

### PF-004: Execution plan under-specifies the `ASSERTED_CODES` update [🟡 MINOR]

**Dimension:** Ordering & Sequencing / Testability
**Location:** `99-execution-plan.md` tasks 1.1.2, 2.1.2
**Codebase Evidence:** `compiler/tests/compile_fail_spec_test.rs:31-42`
(`ASSERTED_CODES`), `:143-154` (`skip_reason`), `:446-450` (the `pending` assertion).

**The Problem:** Removing `EXCLUDED_CODES` entries and `// stage:` annotations
without adding the codes to `ASSERTED_CODES` leaves them "pending" and fails
`skip_report` with a confusing error. `03-04` stated the additions, but the
execution tasks did not.

**Recommendation:** Spell out the additions in the tasks.
**User Decision:** Resolved — applied.
**Fix:** 1.1.2 and 2.1.2 now name the `ASSERTED_CODES` additions explicitly.

---

### PF-005: R5 (cap covers generic structs) had no spec test [🟡 MINOR]

**Dimension:** Testability / Completeness
**Location:** `01-requirements.md` R5; `07-testing-strategy.md` generics cases
**Codebase Evidence:** ST-8 exercised a recursive generic *function* (`nest<T>`),
not a struct.

**The Problem:** R5 extends the `E10305` cap to struct instantiation; no case
exercised it.
**Recommendation:** Add a recursive-generic-struct ST case.
**User Decision:** Resolved — applied.
**Fix:** Added ST-27 (`struct Nest<T> { next: Nest<Wrapper<T>>; }` → E10305) and
wired it into Phase 1 tasks 1.1.1/1.3.1.

---

### PF-006: ST-25 (`E11200` for `std.amiga.custom` on host) had no fixture [🟡 MINOR]

**Dimension:** Completeness / Testability
**Location:** `07-testing-strategy.md` ST-25 and the test-categorization table
**Codebase Evidence:** Existing `E11200_unavailable_module.clo` covers a different
import; `compiler/src/driver.rs:218-232` emits `E11200` for `supports_module`
failures.

**The Problem:** ST-25 is listed under the golden suite but no golden was created.
**Recommendation:** Add the fixture.
**User Decision:** Resolved — applied.
**Fix:** `07-testing-strategy.md` lists `spec/tests/compile-fail/E11200_custom_module.clo`;
`03-04` authorizes it; task 3.1.2 creates it.

---

### PF-007: Closure `id` origin and dedupe key unstated [🟡 MINOR]

**Dimension:** Ambiguities / Implicit Assumptions
**Location:** `03-02-lambdas.md` `ClosureTy.id`, `type_key` `closure_<N>`
**Codebase Evidence:** `ExprKind::Lambda` has no id field
(`compiler/src/ast/expr.rs:122-129`); `type_key`
(`compiler/src/codegen/types.rs:131-146`) has no closure case.

**The Problem:** The design introduced a stable site id without saying where it is
assigned or how it stays consistent between type-checking and codegen; a lambda
inside a generic body keyed only by site id could mis-dedupe across instances.
**Recommendation:** Specify id assignment and scope out lambdas inside generic
bodies.
**User Decision:** Resolved — applied.
**Fix:** `03-02-lambdas.md` now states sema assigns `id` as a deterministic
per-program counter in source order, threaded through the IR, and that lambdas
inside a generic body are out of scope.

---

### PF-008: `00-index.md` related-files list inaccurate [🔵 OBSERVATION]

**Dimension:** Consistency
**Location:** `00-index.md` §Related Files
**The Problem:** Listed `stage` (ambiguous) and omitted the files Phase 3/4 touch
(`corpus_spec_test.rs`, `determinism_spec_test.rs`).
**Recommendation:** List the real file names.
**User Decision:** Resolved — applied.
**Fix:** Now `{compile_fail,stage,corpus,determinism}_spec_test.rs`, plus the new
`E11200_custom_module.clo` under Created.

---

### PF-009: Corpus wiring targets the wrong file [🟠 MAJOR]

**Dimension:** Codebase Alignment (Stale Assumption)
**Location:** `99-execution-plan.md` Phase 3 task 3.2.5 and the Phase 3 baseline;
`00-index.md` §Related Files
**Codebase Evidence:** `docker/scripts/corpus-amiga.sh` holds a hardcoded
`PROGRAMS=(file-copier string-parser buffer-container)` list plus explicit
build blocks for `interrupt-counter` and `graphics`; `compiler/tests/corpus_spec_test.rs`
has one test per program. The `Makefile` `corpus-amiga` target only calls that
script — it holds no program list.

**The Problem:** The plan named `Makefile` as the corpus-wiring target. Executing
that task would edit the wrong file, leaving the `volatile-mmio` program unbuilt
and untested, so `make verify` would not exercise the MMIO leg.

**Options:** A) Retarget the task to `docker/scripts/corpus-amiga.sh` and
`compiler/tests/corpus_spec_test.rs`. B) Leave `Makefile`.

**Recommendation:** Option A — matches how every existing corpus program is wired.

**User Decision:** Resolved — user directed the status check ("things have changed"); applied as a targeted post-preflight correction.
**Fix:** Phase 3 baseline and task 3.2.5 now target
`docker/scripts/corpus-amiga.sh` + `compiler/tests/corpus_spec_test.rs`;
`00-index.md` and `03-03-volatile-mmio.md` updated to match.

---

## Iteration 2 Re-scan

- Every fix verified in place; no new contradiction introduced.
- Re-checked the changed sections against code: E10305 (ST-27) and E11200
  (ST-25) messages match the registry templates (`spec/14-diagnostics.md:117,120,167`);
  the corrected examples now match ST-12/ST-18 and AR-10.
- `./spec/check.sh` remains unaffected by these edits (plan docs only).

### Iteration 3 Re-check (external changes)

Between the pass and execution, two concurrent plans landed: PLAN-08 (Amiga
graphics) and PLAN-09 (demo-video toolkit). A targeted re-check of the sections
that depend on the changed files found PF-009 (corpus wiring) and confirmed the
target surfaces are otherwise untouched:

- `compiler/src/sema/expr.rs` still stages lambdas (`E90000`/`E90002`);
  `compiler/src/codegen/calls.rs` still lowers the volatile intrinsics to `0`.
- `spec/14-diagnostics.md`, `spec/check.sh`, and `spec/tests/compile-fail/**` are
  unchanged since recon, so the golden/spec edits remain valid.
- `compiler/src/stdlib.rs` gained the graphics/intuition registrations the plan
  already expected; the new `std.amiga.custom` entry coexists with them.

## Roadmap Sync

Applied. The concurrent sessions have committed, so `plans/00-roadmap.md` was
safe to update: **PLAN-04 → `Plan Preflighted`** (🔬, 2026-09-25).

## Verdict

**✅ PREFLIGHT PASSED — all 9 findings resolved.** No unresolved CRITICAL/MAJOR.
The plan is ready for execution.
