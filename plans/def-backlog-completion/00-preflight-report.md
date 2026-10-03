# Preflight Report: Def Backlog Completion

> **Status**: ✅ PREFLIGHT PASSED — all 43 findings resolved (30 iteration 1, 11 iteration 2, 2 iteration 3)
> **Iteration**: 3 (bounded verification complete; iteration-3 minors PA-801/PA-802 fixed and re-checked)
> **Artifact**: Implementation plan at `plans/def-backlog-completion/` (11 files incl. register and this report)
> **Codebase Grounded**: 5 independent cluster audits, 1 challenger, 3 verification audits, and 5 prototype builds of the planned duty; all seven reproductions re-run at `d06d451`
> **Last Updated**: 2026-10-03 09:48

> ⚠️ **SAME-SESSION REVIEW**: this plan was created in the current session; same-agent bias is elevated.
> Mitigation: the 13-dimension scan ran as five fresh-context `preflight-auditor` dispatches plus one
> independent critical/major challenger; all findings are codebase-grounded.

### Codebase Context Summary

**Tech Stack:** Rust (std-only workspace), C99 emission; frozen specification under `spec/`; `clc` + `clorraine` binaries; Docker-pinned toolchain.
**Architecture:** `own/` (classify, lower, duties) → `ir/` (with `mono`) → `codegen/` (types, expr, calls, guards, cleanup).
**Key Files Examined:** `own/{place,statements,call_check,expr,lower,call,loops,generics,instance_check,liveness}.rs`, `own.rs`, `ir/{mono,place}.rs`, `ir.rs`, `codegen/{types,expr,cleanup,function,calls,guards}.rs`, `codegen.rs`, `types/mod.rs`, `sema/{expr,call}.rs`, `compiler/tests/residual_defect_completion_{spec,impl}_test.rs`, `spec/{03,06,14}`.
**Reference Verification:** all plan file:line citations spot-checked; the only phantom reference is the missing `03-03` document (PF-001).

### Summary by Dimension

| # | Dimension | Findings | Highest |
|---|-----------|----------|---------|
| 1 | Ambiguities | 3 | 🔴 |
| 2 | Implicit Assumptions | 5 | 🟠 |
| 3 | Logical Contradictions | 4 | 🟠 |
| 4 | Completeness Gaps | 6 | 🔴 |
| 5 | Dependency Issues | 2 | 🔴 |
| 6 | Feasibility Concerns | 4 | 🟠 |
| 7 | Testability | 6 | 🔴 |
| 8 | Security Blind Spots | 3 | 🟠 |
| 9 | Edge Cases | 5 | 🟠 |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 2 | 🟡 |
| 12 | Consistency | 5 | 🟡 |
| 13 | Codebase Alignment | 5 | 🟠 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 2 | all resolved |
| MAJOR | 10 | all resolved |
| MINOR | 15 | all resolved |
| OBSERVATION | 3 | all resolved |

**Challenger:** one independent challenger reviewed the critical/major batch before recommendations
were recorded; it converged on 10/12 and strengthened PF-006 (guard-aware materializer) and PF-009
(the aggregate-literal form is a distinct defect; filing recommended).

---

### PF-001: DEF-62's design document `03-03` is missing [🔴 CRITICAL]

**Dimension:** Completeness / Phantom Reference
**Location:** `plans/def-backlog-completion/03-03-do-while-condition-release.md` (absent); referenced at `00-index.md:60`, `01-requirements.md:40`, `02-current-state.md:113`, `07-testing-strategy.md:50-53`, `99-execution-plan.md:133,150`
**Codebase Evidence:** `ls` confirms the file does not exist; no git history (untracked folder). Phase 2 task 2.2.1 points at `03-03 §Proposed Changes`.
**The Problem:** The DEF-62 design (append algorithm, body-block reachability, newest-first ordering, builder mutator, emitter fallback trigger) has no authoritative document; the spec author would have to invent it, breaking "expectations come from the spec".
**Options:** (A) author `03-03` from AR-4 + 02 Gap 4 with the verified mechanics (recommended); (B) delete the references and inline the design into `02`/`99`.
**Recommendation:** Option A. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-002: The new `E10034` duty rejects two existing tests the plan never updates 🔴 CRITICAL

**Dimension:** Completeness / Dependency
**Location:** `00-ambiguity-register.md` AR-6; `99-execution-plan.md` task 1.2.7
**Codebase Evidence:** `compiler/tests/residual_defect_completion_spec_test.rs:445-466` and `_impl_test.rs:527-560` instantiate `apply<T>(x: T): T { return x; }` via `outer<String>`; the new duty reports `E10034` at that return. AR-6's claim "the specification suites never pinned the bug" is false.
**The Problem:** Phase 1's green step cannot pass without changing a spec-suite program, which AGENTS.md treats as an immutable oracle; the plan names only the stale element pin.
**Options:** (A) change `apply` to `take x: T` in both programs (their original assertions about lambda specialization and determinism survive — verified that the concrete lambda form still compiles), list both files in task 1.2.7, and record the user ruling that adapts a program whose previously accepted shape is now an error (recommended); (B) exempt concrete-typed lends (contradicts R1 and the concrete spelling).
**Recommendation:** Option A, with the explicit user ruling recorded in AR-6. **User Decision:** Resolved — user accepted recommendation (2026-10-03)
**Iteration 2 evidence (reopens PF-002):** the first migration text named two programs and assumed the
lifted lambda body needed no change. A prototype build of the duty showed the correct inventory is
four programs, and that under the literal (scope-A) implementation the two lambda programs migrate
by changing `apply<T>` to `take x: T` only (the lambda bodies then keep their lend-parameter use and
the lambda-shape checking stays a pre-existing boundary, outside this plan's charter — the user
ruled scope A, 2026-10-03). Task 1.2.7 now lists all four programs.

---

### PF-003: ST-16 expects `42` from a program that returns `41` [🟠 MAJOR]

**Dimension:** Testability / Contradiction
**Location:** `07-testing-strategy.md:59`; `03-04-closure-c-spelling.md:81-84`
**The Problem:** `outer<i32>(41)` with the identity lambda `[x](y: T) => y` returns 41; an immutable oracle expecting 42 fails a correct implementation.
**Options:** (A) set the expectation to 41 (recommended); (B) change the lambda to increment.
**Recommendation:** Option A. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-004: ST-11's "unchanged" claim is false for the constant-index store [🟠 MAJOR]

**Dimension:** Consistency / Testability
**Location:** `07-testing-strategy.md:44`; `03-02-move-out-integrity.md:49-54,89`
**Codebase Evidence:** `let s: T = mk()[0]` currently copies and releases both slots (ASan UAF); the fix turns it into a move. Shape and run both change.
**Options:** (A) split ST-11 into a take-shape regression plus a constant-index store/return case expecting the move-out and a clean run (recommended); (B) narrow "unchanged" to the `Element(Some(0))` rendering only.
**Recommendation:** Option A. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-005: The DEF-60 example is illegal under DEF-61's own fix and never produces the move-out [🟠 MAJOR]

**Dimension:** Consistency / Completeness
**Location:** `03-02-move-out-integrity.md:99-105`
**Codebase Evidence:** `takeElement<T>(x: T) { sink<T>(x); }` passes a default lend to a take position — exactly what ST-4 now rejects; `takeElement<T>(mk()[i])` passes to a lend, so no take and no move-out is generated.
**Options:** (A) make it `take x: T`; (B) inline `sink<T>(mk()[i])` directly (the verified repro). **Challenger:** B.
**Recommendation:** Option B (matches the reproduction and exercises the fixed path). **User Decision:** Resolved — user accepted recommendation (2026-10-03)
**Iteration 2 evidence (reopens PF-005):** the first fix pass left the old example in place; the
example now inlines `sink<T>(mk()[i])` as accepted.

---

### PF-006: The index-materialization site cannot emit as written [🟠 MAJOR]

**Dimension:** Feasibility / Implicit Assumption
**Location:** `03-02-move-out-integrity.md:40-45,82`; `99-execution-plan.md` task 1.2.4
**Codebase Evidence:** `take_move_out_places(&self, name, args: &[Value])` (`own/call.rs:346`) cannot emit `Statement::Assign` or rewrite the argument; `value_place` is `&self`; the discarded-call path (`own/statements.rs:180`) is a second move-out producer. **Challenger correction:** the helper must mirror `materialize_take_place` (`own/lower.rs:691-715`) and wrap in `ValueKind::Materialize` when `defer_move_outs > 0`, and recurse through `Member` chains.
**Options:** (A) materialize inside `materialize_owned_argument`'s take block (`own/lower.rs:1193`, a `&mut` producer both callers reach) — recommended; (B) make `take_move_out_places(&mut self)` and rewrite `call.args` at both callers.
**Recommendation:** Option A with the guard-aware/member-chain corrections; update 03-02 and task 1.2.4 to name `own/lower.rs`. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-007: `Projection::ElementTemp` blast radius omits four exhaustive matches [🟠 MAJOR]

**Dimension:** Codebase Alignment / Impact Blindness
**Location:** `03-02-move-out-integrity.md` §Integration Points; task 1.2.3
**Codebase Evidence:** adding a variant breaks `own/expr.rs:847-860`, `own/place.rs:151-183`, `own/place.rs:250-253`, `own.rs:817-821` (the last must map to `ty.iterated_element()`). **Challenger correction:** no `ir.rs` re-export is needed (`pub use place::{...}` exports wholesale); `narrowing` and `has_runtime_element` are `matches!` sites, not compile breaks; derives still hold (`TempId` is integral).
**Options:** (A) add explicit arms and list the four sites in the map/task (recommended); (B) wildcard arms (disfavored by convention).
**Recommendation:** Option A. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-008: Closure stamping is specified with a condition that cannot be true yet [🟠 MAJOR]

**Dimension:** Feasibility / Ordering
**Location:** `03-04-closure-c-spelling.md:40-50`; task 3.2.2
**Codebase Evidence:** `own/generics.rs:489` walks the enclosing body (recording the closure argument) before `:494-508` records the lifted specializations; the emitter's `has_lifted_specialization` reads that list, so nothing would be stamped.
**Options:** (A) reorder the `record_lifted` block before `walk_function` (recommended); (B) use the order-independent `lifted_needs_specialization` predicate; (C) post-pass stamping with a new mutator.
**Recommendation:** Option A. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-009: The aggregate-literal consume still moves the wrong slot and is a distinct defect [🟠 MAJOR]

**Dimension:** Security / Edge Cases
**Location:** `03-02-move-out-integrity.md:118-124` (the claim that `Element(None)` keeps a safe fallback)
**Codebase Evidence:** `Box<T> { v: mk()[i] }` emits `__cl_tmp1 = __cl_tmp0[0]; __cl_tmp0[0] = NULL;` and segfaults; the `mk(__cl_tmp0)` call is never emitted on that path. The constant-index form is already `E10026`.
**Options:** (A) extend the index helper to `consume_place` (auditor's pick; the challenger shows the helper alone does not fix the root-materialization gap); (B) file a new backlog defect and document the limitation (challenger's pick — the failure is distinct).
**Recommendation:** Option B (file as a new backlog defect, like DEF-63; scope stays DEF-56 – DEF-62). **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-010: Memory-cleanliness expectations have no scheduled, observable verification [🟠 MAJOR]

**Dimension:** Testability
**Location:** `07-testing-strategy.md:18-20,40-53`; tasks 1.1.2, 2.1.2, 1.3.1, 2.3.1, 3.3.1
**Codebase Evidence:** `RDC_ASAN` exists only in the impl suite (`residual_defect_completion_impl_test.rs:259-269`) and is not named by any execution task; a pure leak exits 0 on a plain run, so the claimed red phase is unobservable; only task 4.1.2 guarantees ASan.
**Options:** (A) add `RDC_ASAN`-gated `run_memory` impl tests for the memory-only cases, give ST-13/15 concrete exit codes, and document the red-phase procedure (acceptance red for DEF-56/61, glibc abort where it occurs, host ASan probe for DEF-59/62) — recommended, adopting the challenger's wording; (B) restate those ST expectations as program-visible and delegate leak claims to the probes.
**Recommendation:** Option A. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-011: "All seven reproductions ASan-clean" is impossible for the two rejection fixes [🟠 MAJOR]

**Dimension:** Testability / Scope wording
**Location:** `01-requirements.md:77-79,110-114`; `00-index.md:35-36`; tasks 4.1.2/4.1.3; `99` Success Criterion 5; AR-7/AR-12
**Codebase Evidence:** after the fix, DEF-56/61 emit `E10034` and no C, so there is nothing to build or run.
**Options:** (A) split the close-out runner into five ASan run probes plus two rejection probes (expect `E10034`, no C written), each with an expected signature, and reword the goal/AC4/§Memory Safety/task 4.1.2 (recommended); (B) keep a runnable duplicate instantiation (re-admits the defect).
**Recommendation:** Option A. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-012: R1's "stored" form has no test [🟠 MAJOR]

**Dimension:** Testability / Completeness
**Location:** `01-requirements.md:23-26`; tasks 1.2.1/1.2.2
**Codebase Evidence:** no ST exercises `BindingEffect::MoveIfOwning` (`own/statements.rs:461-481`) or `consume_owned_place` (`own/expr.rs:48-62`). **Challenger correction:** the duty must require `BindingKind::Param` + lend mode, or `for (borrow e in xs) { let b: T = e; }` (`residual_defect_hardening_spec_test.rs:1268`) records a duty and flips a pinned `E10020` to `E10034`.
**Options:** (A) add one ST for a lend stored into a binding/aggregate plus the `BindingKind::Param` predicate rule (recommended); (B) drop "stored" from R1.
**Recommendation:** Option A. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-013: DEF-58 is cited as `AR-8` (the naming entry) [🟡 MINOR]

**Dimension:** Consistency / Traceability
**Location:** `03-04-closure-c-spelling.md:12`; `99-execution-plan.md:190`; register `:17`
**The Problem:** AR-8 decided folder/suite names; no register row resolves the separator rule; `00-index.md:81` already says "AR-3 family".
**Options:** standardize on AR-3 (recommended) or add a register row.
**Recommendation:** Fix the citations; extend AR-3's resolution note with the separator rule. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-014: The `RequiresNotLend` interface is not constructible as specified [🟡 MINOR]

**Dimension:** Implicit Assumption
**Location:** `03-01-generic-lend-duties.md:28-33,45-63,75-78`
**Codebase Evidence:** `mark_moved_if_owning(&mut self, target: PlaceTarget)` has no span/type; the "callers already pass the value" claim is false at 6 of 11 call sites; `Span` is not `Ord`, so a `(function, parameter, span)` dedup key will not compile; `projected_type` (`own.rs:815-824`) resolves projection types.
**Options:** (A) keep `(target, span)`, derive `parameter`/`ty` internally, dedup on `(function, ir_name, span.file, span.start)`, and correct the caller list (recommended); (B) dedup by binding id (loses per-site reporting).
**Recommendation:** Option A. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-015: The one-report rule cites AR-10 (phase ordering) [🟡 MINOR]

**Dimension:** Traceability
**Location:** `03-01-generic-lend-duties.md:113`; `07-testing-strategy.md:35`
**Recommendation:** cite AR-2 and mirror the duplicability audit's break (`own/instance_check.rs:93-95`). **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-016: Close-out re-files DEF-63, which planning already filed [🟡 MINOR]

**Dimension:** Consistency
**Location:** task 4.1.3; `01-requirements.md:115`; AC5; `00-index.md:44-45`; roadmap now carries DEF-63
**Recommendation:** reword to "verify the DEF-63 row (filed at planning) and the corrected DEF-60 remedy; mark DEF-56 – DEF-62 Done". **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-017: ST-17's `outer<str>(stringOf("hi"))` does not build even after the fix [🟡 MINOR]

**Dimension:** Consistency / Oracle
**Codebase Evidence:** compiled at `d06d451`: the emitted C passes `ClString` where `ClStr` is expected (an unrelated implicit-widening gap).
**Recommendation:** ground ST-17 on a `str` local (verified to build) or on `outer<String>`. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-018: ST-17 does not exercise the dedupe collapse it claims to pin [🟡 MINOR]

**Dimension:** Testability / Coverage
**Codebase Evidence:** in ST-17's program the two instances differ in `T`, so `dedupe_arg` already separates them; the collapse shows only when the closure is the sole type argument (`consume<F: func(): i32>` from two enclosing instances — verified).
**Recommendation:** add a closure-only case (two symbols/spellings) or delegate it to an explicitly described impl test and update AR-12 coverage. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-019: `type_key` cannot move while its slice helpers stay in `codegen` [🟡 MINOR]

**Dimension:** Feasibility / Architecture
**Codebase Evidence:** `type_key`'s slice arm calls `slice_element_key` (`codegen/types.rs:302,326-328`), also used by `codegen/calls.rs:268,297`; moving only `type_key` creates a `types` → `codegen` dependency.
**Recommendation:** move `slice_base_element`/`slice_element_key` alongside `type_key` into `types` (both are pure `Ty` functions), preserving names. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-020: The do-while append mechanics are unspecified [🟡 MINOR]

**Dimension:** Completeness / Ordering (content for the restored 03-03)
**Codebase Evidence:** body/condition/exit blocks are allocated before body lowering, so a block-id range is not "the body"; return releases are newest-first (`own.rs:1044-1046`); `Builder` has no release-append mutator (`ir.rs:701`); temps are function-scope hoisted, so references are safe.
**Recommendation:** specify in 03-03: snapshot the pending-release length; collect Return blocks reachable from `body_block` (reuse `liveness::successors`); prepend the new releases newest-first; add the mutator to `compiler/src/ir.rs`; name `ir.rs` in task 2.2.1 and the file maps. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-021: A concrete-typed lend under a maybe-owning position still double-frees [🟡 MINOR]

**Dimension:** Implicit Assumption / Memory Safety
**Codebase Evidence:** `f<T>(v: String): T { return v; }` compiles and ASan-double-frees; the sibling audit's `mentions_type_param` gate (`own/instance_check.rs:68-70`) would skip it if mirrored.
**Recommendation:** keep the written rule (no param-mention early skip), add one ST case, and note the acceptance boundary in R1. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-022: Task ordering produces two non-compiling intermediate states [🟡 MINOR]

**Dimension:** Ordering / Atomicity
**Location:** tasks 1.2.1/1.2.2; 1.2.4/1.2.7
**Recommendation:** define the duty record before recording it (merge or swap 1.2.1/1.2.2) and move the stale-pin update directly after 1.2.4. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-023: Multi-level runtime indices are not covered [🟡 MINOR]

**Dimension:** Edge Cases
**Codebase Evidence:** `mk()[i][j]` emits the read then zeroes `__cl_tmp[0][0]` (double free verified).
**Recommendation:** state the recursion requirement in the helper contract and add one nested-index ST case. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-024: Move-then-reuse reports `E10034` + `E10020`; the concrete form reports one `E10034` [🟡 MINOR]

**Dimension:** Edge / R1 fidelity
**Codebase Evidence:** concrete `reuse(x: String)` → 1×E10034; the generic instance currently → 1×E10020, and after the fix would report both.
**Recommendation:** relax R1 to "reports E10034" and pin the dual report in an ST case (recommended); do not suppress the duplicability duty. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-025: The close-out ASan recipe hangs on two of seven reproductions [🟡 MINOR]

**Dimension:** Feasibility / Testing
**Codebase Evidence:** in-container, the exact DEF-56 and DEF-62 repros never terminate (~40 MB of `AddressSanitizer:DEADLYSIGNAL`); `handle_segv=0` makes them deterministic.
**Recommendation:** add `timeout` and `ASAN_OPTIONS=handle_segv=0` (keep leak detection), or run probes on the host; pin the exact runner options in task 4.1.2. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-026: The duty rules name only field projections; lend element takes segfault [🟡 MINOR]

**Dimension:** Edge / Security
**Codebase Evidence:** concrete `sink(p[0])` reports `E10034`; generic `at<T>(p: T[2]) { sink<T>(p[0]); }` is accepted and segfaults.
**Recommendation:** broaden the Behavior Rule to "field or element projection" and add an ST case; state the `BindingKind::Param` predicate explicitly. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-027: The `borrow T?` out-param exemption is not covered [🟡 MINOR]

**Dimension:** Testability
**Codebase Evidence:** `spec/06-functions.md:141-145` and `spec/14-diagnostics.md:62` exempt the out-param slot from `E10034`.
**Recommendation:** add a behavior-rule line and one case pinning that a generic `borrow T?` reassignment instantiated owning stays accepted. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-028: AR-5 says the store path avoids `Place`, but Change 4 uses one [🔵 OBSERVATION]

**Dimension:** Consistency
**Recommendation:** reword AR-5 to "the store path carries the index in the `ElementTemp` place; the return path renders the lowered value; neither collapses to `[0]`". **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

### PF-029: `03-01` cites "R3 case `i32`" for duplicable acceptance [🔵 OBSERVATION]

**Dimension:** Citation
**Recommendation:** cite R1/ST-3. **User Decision:** Resolved — user accepted recommendation (2026-10-03)

---

## Iteration 2 — Fix Verification and Bounded Fresh Scan

Two fresh audits ran: one verified every PF-001 – PF-030 fix against the report, one re-scanned all
13 dimensions of the updated artifact. Result: 26 fixes verified, 1 not applied, 3 partial, plus 11
new findings (PA-701 – PA-711). All are resolved:

| # | Sev | Finding | Resolution |
|---|-----|---------|------------|
| PA-701 | 🟠 | Reopens PF-002: the named migration assumed the lifted lambda needed no change; under an extended duty scope the lambda body moves its lend | User ruled **scope A** (duty records at the existing maybe-owning call sites; lifted expression bodies keep today's handling); task 1.2.7 now lists the four migrated programs (rdc08, liftdeterminism, `own/tests.rs:939`, `ownership_drop_spec_test.rs:977`) per the prototype inventory; 03-01 records the scope |
| PA-702 | 🟠 | Reopens PF-005: the old DEF-60 example remained | Fixed: 03-02's example is now the inline `sink<T>(mk()[i])` form |
| PA-703 | 🟡 | `99` Phase 3 still cited AR-8 (naming) | Fixed: the phase reference is AR-3 only |
| PA-704 | 🟡 | AR-10 row still said "all seven reproductions ASan-clean" | Fixed: the row now states five run probes + two rejection probes |
| PA-705 | 🟡 | The builder signature named `&[Place]`, not `&[Release]` | Fixed: `append_return_releases(..., releases: &[Release])` with the `pending_release_copies` conversion |
| PA-706 | 🟡 | Phase 3 deliverables omitted ST-27 | Fixed |
| PA-707 | 🟡 | Phase 1 red-phase list omitted ST-25 | Fixed |
| PA-708 | 🟡 | 03-04's example still used `outer<str>(stringOf("hi"))` | Fixed: `outer<String>` |
| PA-709 | 🟡 | ST-13/ST-15 said "the pinned value" instead of concrete exit codes | Fixed: `3` and `2` |
| PA-710 | 🟡 | The stale-pin/migration task remained after the store/return tasks | Fixed: task order is now 1.2.5 pins+migration, 1.2.6 store, 1.2.7 return, 1.2.8 green |
| PA-711 | 🔵 | 03-03's candidate-set wording diverged from AR-4 | Fixed: `body_block` is included in the candidate set with its meaning stated |

**Iteration-2 verdict:** BLOCKED until the two 🟠 findings were resolved; both are now resolved and
the mechanical fixes applied. A bounded iteration-3 verification of the touched sections follows.

---

## Iteration 3 — Bounded Verification

One auditor re-checked the touched sections. All 11 iteration-2 fixes verified; the task-ordering
and citation surfaces are clean. Two new minor findings were found and fixed:

| # | Sev | Finding | Resolution |
|---|-----|---------|------------|
| PA-801 | 🟡 | Task 3.1.2 was glued onto 3.1.1's line, so only 30 of the 31 task checkboxes parsed | Fixed: the two task lines are separate again; 31 task checkboxes parse |
| PA-802 | 🟡 | 03-01 cited "task 1.2.7" after the reorder moved the migration to 1.2.5 | Fixed: the citation now reads 1.2.5 |

**Final verdict: ✅ PREFLIGHT PASSED — all 43 findings resolved.** No critical or major findings
remain; every minor and observation is resolved or explicitly accepted through its recorded user
decision. The artifact is ready for execution.

---

### PF-030: `dedupe_arg` is described as the symbol-key renderer [🔵 OBSERVATION]

**Dimension:** Codebase Alignment (citation precision)
**Codebase Evidence:** C symbols come from `instance_typekey` → `types::type_key` (`codegen.rs:555-558`); `dedupe_arg` feeds `dedupe_key` for diagnostics/obligations.
**Recommendation:** reword AR-3/03-04 to credit `type_key` for the symbol and `dedupe_arg` for the dedupe/diagnostic key. **User Decision:** Resolved — user accepted recommendation (2026-10-03)
