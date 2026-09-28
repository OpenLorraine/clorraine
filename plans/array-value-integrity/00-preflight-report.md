# Preflight Report: Array Value Integrity

> **Status**: ✅ PREFLIGHT PASSED — all 8 findings resolved (5 major, 3 minor); iteration 2 verified
> **Iteration**: 2 (re-scan after accepted fixes)
> **Artifact**: Implementation plan at `plans/array-value-integrity/` (8 documents, uncommitted working tree)
> **Codebase Grounded**: 20+ source/spec files examined; ~45 references verified; 14+ live probes with `target/debug/clc`
> **Scope**: Strict (no `--explore-scope`); audit target = the plan directory only
> **Last Updated**: 2026-09-28 14:05

> **SAME-SESSION REVIEW**: This artifact was created in the current session
> (same session lineage). Same-agent bias risk is elevated. The scan used a
> separate challenger subagent for the CRITICAL/MAJOR batch; consider a
> fresh-session review before execution for maximum independence.

## Codebase Context Summary

**Repository:** The Clorraine compiler (Rust workspace, std-only; `clc` + `clorraine` driver)
**Tech Stack:** Rust (compiler), C99 (one emitted translation unit per program, with the selected profile shim prepended), Clorraine (`.clo` sources); pinned Docker SDK toolchain
**Architecture:** `lex → parse → load → resolve → sema → own → ir → codegen`; the `own` pass owns typing/lowering/ownership and emits `ir::Statement`/`Value`; codegen renders C. Fixed arrays are one `TyKind::Array { element, extents }`; by-value fixed-array returns use a hidden trailing out-parameter.
**Key Files Examined:** `compiler/src/own/loops.rs`, `own/statements.rs`, `own/expr.rs`, `own/call_check.rs`, `own/place.rs`, `own/lower.rs`, `codegen/function.rs`, `codegen/calls.rs`, `codegen/guards.rs`, `codegen/expr.rs`, `codegen/types.rs`, `codegen/drop.rs`, `types/mod.rs`, `profile.rs`, `compiler/runtime/shim_*.c`, `compiler/tests/element_loop_integrity_spec_test.rs`, `compiler/tests/freestanding_spec_test.rs`, `spec/02`, `03`, `04`, `06`, `08`, `14`, `17`.
**Key Observations:**
- Emitted C must compile warning-free under `-Wall -Wextra -Werror` (host tests: `element_loop_integrity_spec_test.rs:116`); the freestanding legs add `-ffreestanding -fno-builtin -nostdlib`.
- The plan's core design is sound: the byte-copy store, the materialize lift, the `len` outer-extent rule, and the E10026 helper are all implementable in the cited functions.
- Five code paths around fixed-array values were not covered by the original plan; each was reproduced live and is now covered by an accepted fix.

**Reference Verification:** ~45 mapped — all now resolve (after the PF-008 corrections).

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 0 | — |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 0 | — |
| 4 | Completeness Gaps | 3 | 🟠 resolved |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 1 | 🟠 resolved |
| 7 | Testability | 0 | — |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 1 | 🟡 resolved |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 1 | 🟡 resolved |
| 13 | Codebase Alignment | 5 (shared) | 🟠 resolved |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 5 | all resolved |
| MINOR | 3 | all resolved |
| OBSERVATION | 0 | — |

---

## Findings

### PF-001: Fixed-array Call assignments bypass the target drop and the save path [🟠 MAJOR]

**Dimension:** 4 Completeness Gap · 13 Codebase Alignment
**Location:** `01-requirements.md` R2/R3; `03-01` "New Functions" (store path)
**Codebase Evidence:** `codegen/function.rs:336-353` routes a fixed-array `Call` RHS to `emit_call(..., Some(target))` and returns before `emit_assign` (:577-612). Probes: `s = mk();` → `cl_..._mk(s);` with no drop (leak); `s = dup(borrow s);` → `cl_..._dup(s, s);` with no save and aliasing.
**The Problem:** R2/R3 require drop-first/save for these forms; the only realistic self-referencing array RHS is a call, and it bypasses the whole path.
**Recommended resolution:** Option A (call into a fresh temporary — the discarded-call pattern `function.rs:466-489` — then lower as the fixed-array `Statement::Move`), plus spec cases.
**User Decision:** Resolved — User accepted recommendation: Option A. Applied: R2/R3 updated; `03-01` store path now specifies the call-into-temporary path and the same-place no-op; ST-25/ST-26 added; tasks 1.2.1/1.3.1 extended.
**Challenger:** converged (refined the mechanism to temp-then-Move, avoiding the alias-unsound call-with-destination=target shape).

---

### PF-002: The array-copy helper claim is false for freestanding and not spec-authorized [🟠 MAJOR]

**Dimension:** 6 Feasibility · 13 Codebase Alignment
**Location:** AR-4; `01` Compatibility; `02` Dependencies; `03-01` byte copy
**Codebase Evidence:** `cl_memcpy` exists in `shim_host.c:73` and `shim_hosted_amiga.c:171`, not in `shim_freestanding.c` (plain `memcpy` at :29); `cl_memcpy` unused anywhere; `spec/17-codegen.md:351` limits the move lowering to `cl_zero`; `spec/17` §13 guarantees a shim `memcpy` on every profile.
**The Problem:** The stated mechanism breaks the freestanding profile and adds an unauthorized `cl_` helper.
**Recommended resolution:** Option A: emit the C runtime `memcpy`; correct AR-4, `01`, `02`, `03-01`, and `00-index`.
**User Decision:** Resolved — User accepted recommendation: Option A. Applied: AR-4 rewritten (with the correction noted); all `cl_memcpy` references in the plan replaced; compatibility/dependency/perf text updated.
**Challenger:** converged (plain `memcpy`; wrapper to the freestanding shim only as an alternative).

---

### PF-003: E10026 coverage is incomplete versus R4 "in any form" [🟠 MAJOR]

**Dimension:** 4 Completeness Gap · 13 Codebase Alignment
**Location:** `03-02` "Element moves" table; `01` R4; `07` ST-19 – ST-21
**Codebase Evidence:** `s = mk()[0]` and `Holder { s: mk()[0] }` accepted → invalid C; `h.s = mk()[0]` and `h.s = a[0]` accepted although `spec/03` §4 requires E10026. Missing paths: `own/expr.rs:48` (`consume_owned_place`), the assignment path `own/expr.rs:291-365`, and field-target assignment.
**The Problem:** R4's "any form" was not implemented or tested.
**Recommended resolution:** Option A: centralize the check at the shared consume points (store preparation, aggregate consume point) plus `take`/return, including field targets; add spec cases.
**User Decision:** Resolved — User accepted recommendation: Option A. Applied: `03-02` site table and error table updated; R4 lists every move form; ST-27 – ST-30 added; tasks 3.1.1/3.2.2/3.3.1 extended.
**Challenger:** converged (additionally flagged the field-target holes, verified live).

---

### PF-004: Borrow row views need a pointer-to-array C type [🟠 MAJOR]

**Dimension:** 6 Feasibility · 13 Codebase Alignment
**Location:** `03-01` "Integration Points"; `99` task 2.2.1
**Codebase Evidence:** `own/loops.rs:404` types the address temp as `Ty::raw(element)`; `codegen/types.rs:109-123` spells `raw` as `c_type(inner) *name`; `c_type` of a fixed array returns the element type → `i32 *` vs `i32 (*)[3]`, fatal under `-Werror` (ST-11).
**The Problem:** Phase 2 as specified could not compile the borrow row form.
**Recommended resolution:** Option A: extend `declarator`/`c_type` for `raw` of a fixed array to `T (*tmp)[M]`; add an implementation test.
**User Decision:** Resolved — User accepted recommendation: Option A. Applied: `03-01` Proposed Changes/Integration Points/Testing updated; `02` file table, `00-index` related files, tasks 2.2.1/2.3.1 extended.
**Challenger:** converged (small blast radius; pointer-to-first-element loses nested extents).

---

### PF-005: Materialized owning array temporaries are never released [🟠 MAJOR]

**Dimension:** 4 Completeness Gap · 13 Codebase Alignment
**Location:** `03-02` "Materialize lift"; `07` ST-18; `99` task 4.1.1
**Codebase Evidence:** `own/lower.rs:521-550`; no temp cleanup anywhere (`codegen/function.rs:248-262`); `spec/17` requires statement temporaries to drop; ST-18 leaks the array's untouched elements; `let v: str = mk()[0]` is accepted today; probes run with `detect_leaks=0`.
**The Problem:** The plan introduced the first common owning `Materialize` temp without a release path or a view-lifetime rule.
**Recommended resolution:** Option B: record the limitation, file a follow-up defect, and word the ASan criterion for the configured run (Option A — statement-scoped temp release plus view promotion/rejection — is spec-complete but adds a new ownership surface requiring Complexity Escalation Gate approval).
**User Decision:** Resolved — User accepted recommendation: Option B. Applied: `01` Won't Have + acceptance criterion 3, `03-02` "Known Limitations", `02` risk row, `07` E2E wording; `DEF-23` filed in the roadmap; task 4.1.3 updated.
**Challenger:** diverged — preferred a strengthened Option A; accepted Option B only with the limitation filed and the `spec/17` deviation recorded.

---

### PF-006: ST-8 self-assignment path is under-specified [🟡 MINOR]

**Dimension:** 9 Edge Cases · 13 Codebase Alignment
**Location:** `03-01` store path; `07` ST-8
**Codebase Evidence:** `emit_assign` checks `!emits_drop` first (`function.rs:580-585`) and never reaches the save path for non-droppable arrays; `a = a;` is accepted and emits `a = a;` today; unconditional `memcpy(&a, &a, sizeof a)` is formally UB.
**Recommended resolution:** Option A: same-place right-hand side emits no copy; implementation-test assertion.
**User Decision:** Resolved — User accepted recommendation: Option A. Applied: `03-01` store path and `07` impl-test row updated; task 1.3.1 extended.

---

### PF-007: R1 over-promises call-return iterables [🟡 MINOR]

**Dimension:** 4 Completeness Gap
**Location:** `01` R1; `00-index` Overview
**Codebase Evidence:** `for (x in mk())` accepted, emits `x = cl_..._mk()[i];` (invalid C), untouched by the plan.
**Recommended resolution:** Option A: narrow R1 to place-rooted iterables and file the call form as a follow-up defect.
**User Decision:** Resolved — User accepted recommendation: Option A. Applied: R1 now says "held in a place (local, parameter, or field)"; `01` Won't Have bullet added; `DEF-24` filed in the roadmap.

---

### PF-008: Citation and cross-reference drift [🟡 MINOR]

**Dimension:** 12 Consistency · 13 Codebase Alignment
**Location:** several plan documents
**Codebase Evidence / corrections:** ST-9 source now `spec/06` F2; `03-02` testing range `ST-16 – ST-24, ST-27 – ST-30`; `place_target` cited as `own/place.rs:535`; `02` out-parameter citation `codegen/function.rs:336-353`; related-file lists extended (`types/mod.rs`, `own/expr.rs`, `own/call_check.rs`, `own/place.rs`, `codegen/types.rs`); `function.rs` Move arm citation corrected to `:357`.
**Recommended resolution:** Option A: apply all corrections in one pass.
**User Decision:** Resolved — User accepted recommendation: Option A. Applied.

---

## Iteration 2 — Fix Verification

| Check | Result |
|-------|--------|
| All 8 `User Decision` lines recorded | ✅ |
| No stale `cl_memcpy` references in executable plan content | ✅ (only the AR-4 correction note and this report) |
| ST table complete: 30 cases (ST-1 – ST-30) with contiguous numbering | ✅ (first pass found ST-27 – ST-30 missing from `07`; fixed) |
| Task count and phase table unchanged (25 tasks, 4 phases) | ✅ `grep` count = 25 |
| Residual "three move sites" wording | ✅ found in `03-02` error table; fixed to "every move decision" |
| `cl_memcpy`/task references in `99` consistent with the corrected design | ✅ |
| Roadmap: DEF-15/19/21/22 advanced to `Plan Preflighted 🔬`; DEF-23/DEF-24 filed | ✅ |
| Acceptance criterion 3 and ASan wording match the DEF-23 limitation | ✅ |
| No new dimension findings introduced by the edits | ✅ (re-scan of the changed sections clean) |

## Roadmap Sync

- `DEF-15`, `DEF-19`, `DEF-21`, `DEF-22` → **Plan Preflighted** (🔬), plan link retained.
- New: `DEF-23` (materialized temporary lifetime, 🟠 backlog) and `DEF-24` (element loop over an array-returning call, 🔴 backlog).

## Next Steps

1. Commit the plan directory plus this report (and the roadmap changes) when ready.
2. Run `exec-plan array-value-integrity` with the desired commit mode to execute the 4 phases.
3. A fresh-session preflight is optional but recommended before execution given the same-session review.
