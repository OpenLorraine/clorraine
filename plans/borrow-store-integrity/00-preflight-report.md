# Preflight Report: Borrow-Store Integrity

> **Status**: ✅ PREFLIGHT PASSED — all 17 findings resolved (6 major, 11 minor; 16 delegated under auto-design, 1 reserved ruling recorded). Iteration 2 verified.
> **Iteration**: 1 (first scan)
> **Artifact**: implementation plan at `plans/borrow-store-integrity/` (9 documents plus this report), scanned at HEAD `8a47422`, plan tree `40ce47eeee743737d650b62a7b37aa698953d676`
> **Mode**: `--auto-design` active (eligible technical resolutions delegated and recorded; one reserved scope question — PF-004)
> **Scope**: strict (no `--explore-scope`)
> **Codebase Grounded**: ~30 compiler files examined; every plan `file:line` reference checked; 30+ probes run with `target/debug/clc` + host `cc`/AddressSanitizer; independent challenger on the CRITICAL/MAJOR batch
> **Last Updated**: 2026-10-10 01:06

## Codebase Context Summary

**Tech Stack:** Rust (std-only workspace), C99 emission (one translation unit per program), Clorraine `.clo` sources; `target/debug/clc` + host `cc`/ASan probes; `make check`/`make verify` per AGENTS.md.
**Architecture:** front end (`lex`, `parse`, `load`, `resolve`, `sema`) → ownership and lowering (`own/`: bindings, duties, per-instance audits, narrowing, places, loops, statements) → typed IR (`ir.rs`, `ir/mono.rs` duty records) → codegen (`codegen/`: signatures, places, calls, guards, cleanup ladder). The ownership pass records substitution-blind duties (`ir::Requires*`) that `own/instance_check.rs` audits per recorded instance; the frozen `spec/` is the oracle.
**Key files examined:** `own/expr.rs` (`check_assign`, `check_update`, `check_lend_target_store`, `lend_moves_judged`, `check_nonnull_expr`, `emit_assigned`, `emit_indexed_store`, `indexed_target`), `own/place.rs` (`move_place`, `mark_moved_if_owning_from`, `read_place`, `element_index`), `own/statements.rs` (`check_expr_statement`, `bind_value`, `prepare_binding_effect`, `apply_binding_effect`), `own/instance_check.rs` (all five audits), `own/generics.rs` (two `Statement` walks), `own/loops.rs` (`for_step_is_structured`), `own.rs` (bindings, `maybe_owns`, duty plumbing, `check()`), `ir.rs` (`Statement`), `codegen/cleanup.rs` (`emit_return`), `codegen/function.rs` (`pointer_params`, `emit_arm_source_moves`, `emit_move_zero_place`, statement emit, `unread_locals`), `codegen/guards.rs` (`emit_bounds_guard`, `bounds_guard_condition`), `codegen/control.rs` (for-step text), `codegen/expr.rs` (`place`, `place_base_type`), `codegen/drop.rs`; tests `borrow_generic_integrity_{spec,impl}_test.rs`, `borrow_view_and_shadowing_impl_test.rs`, `run_fault_spec_test.rs`, `determinism_spec_test.rs`; `spec/03`, `spec/04`, `spec/06`, `spec/14`, `spec/16`, `spec/17`, `spec/decisions`, `spec/02`.

### Summary by Dimension

| # | Dimension | Findings | Highest |
|---|-----------|----------|---------|
| 1 | Ambiguities | PF-003, PF-008 | 🟠 |
| 2 | Implicit Assumptions | PF-003, PF-007, PF-017 | 🟠 |
| 3 | Logical Contradictions | PF-001 | 🟠 |
| 4 | Completeness Gaps | PF-002, PF-004, PF-010, PF-012 | 🟠 |
| 5 | Dependency Issues | PF-011 | 🟡 |
| 6 | Feasibility Concerns | PF-005 | 🟠 |
| 7 | Testability | PF-001, PF-002, PF-006, PF-009, PF-010, PF-013, PF-014, PF-015 | 🟠 |
| 8 | Security Blind Spots | PF-002, PF-004 | 🟠 |
| 9 | Edge Cases | PF-005, PF-006, PF-016 | 🟠 |
| 10 | Scope Creep Indicators | PF-007 | 🟡 |
| 11 | Ordering & Sequencing | PF-001 | 🟠 |
| 12 | Consistency | PF-015, PF-016, PF-017 | 🟡 |
| 13 | Codebase Alignment | PF-002, PF-003, PF-004, PF-005, PF-006, PF-007, PF-011, PF-017 | 🟠 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| 🔴 CRITICAL | 0 | — |
| 🟠 MAJOR | 6 | PF-001…PF-006; five delegated, PF-004 reserved (user decision pending) |
| 🟡 MINOR | 11 | PF-007…PF-017; delegated |
| 🔵 OBSERVATION | 0 | — |

Challenger: one independent design challenger reviewed the CRITICAL/MAJOR batch. Converged on
PF-001, PF-002, PF-003, PF-006; diverged on PF-005's mechanism (preferred an emitter-side guard
derivation; reconciled to the gated `Statement::Guards` — see PF-005); added the row-store form
(PF-002), the concrete-spelling repro (PF-004), and the R1 text correction (PF-003).

---

## Findings

### PF-001: The P2 red-phase expectation is wrong — ST-9 also fails at baseline [🟠 MAJOR]

**Dimension:** 3 (Logical Contradictions) / 7 (Testability) / 11 (Ordering & Sequencing)
**Location:** `99-execution-plan.md` Phase 2 task 2.1.2 ("expected: ST-8 fails; ST-9 – ST-12 pass as green pins"); `07-testing-strategy.md` ST-9 row
**Codebase Evidence:** the guarded arm `func cid(borrow v: String?): String? { if (v != null) { return v; } return null; }` takes the same direct return branch as ST-8 (`codegen/cleanup.rs:498-502`, no source emptying). Baseline probe (three runs, plain and ASan): accepted, plain exit 134 `free(): double free detected`, ASan `attempting double-free`.
**The Problem:** ST-9 is red at baseline, exactly like ST-8; the phase's red-phase gate as written treats its failure as unexpected, inviting a blocked task or a test edit against the immutable-oracle rule.

**Options:** (A) correct 2.1.2 to "ST-8 and ST-9 fail; ST-10 – ST-12 pass", and annotate ST-9 "red at baseline" in 07 (the guarded arm is the second return shape the P2 fix must cover); (B) split ST-9 into a null-slot green clause and a non-null guarded-arm red case — more churn, same outcome.

**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-002: DEF-106 does not cover the constant-index compound and update stores [🟠 MAJOR]

**Dimension:** 4 (Completeness Gaps) / 7 (Testability) / 8 (Security — memory safety) / 13 (Codebase Alignment)
**Location:** `01-requirements.md` R5; `03-03-view-store-guards.md` §Proposed Changes DEF-106 ("the constant-index element path"); `07-testing-strategy.md` ST-18–ST-21; `99-execution-plan.md` task 3.2.2
**Codebase Evidence:** the constant-index store reaches emission through `check_assign` (`own/expr.rs:569-575` → `emit_assigned` → `emit_store`/`emit_compound_store`) and through `check_update` (`:744-758` → `emit_compound_store`); only the runtime-index path (`StorePointer`) carries a guard. Baseline probes on `slice(d, 2, 2)` (empty non-null view): `xs[0] += 1`, `xs[0]++`, and the row store `rows[0] = [...]` are accepted, emit no `cl_trap("E11211", …)`, and reproduce ASan `stack-buffer-overflow` writes; the read and runtime-index forms trap.
**The Problem:** R5's text ("a constant-index element store … emits the `E11211` bounds trap") and the filed defect ("the constant-index store path does not" guard) cover every constant-index store form. The design names only "the constant-index element path" and no ST case exercises `+=`/`++`/row stores, so an implementation that patches only the plain-store site satisfies the design and the ST set while the same defect class remains live.

**Options:** (A) cover every constant-index store form (plain and compound in `check_assign`; the constant update path in `check_update`; the row-valued store) with the same guard and add ST cases (`+=` and `++` on the empty view trap; an in-bounds compound store stays accepted); (B) narrow R5 to plain assignment stores and file the compound/update forms as a new defect — knowingly ships a memory-safety hole the filed defect already describes.

**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-003: DEF-100's nullable-substitution precedence diverges from every concrete nullable spelling [🟠 MAJOR]

**Dimension:** 1 (Ambiguities) / 2 (Implicit Assumptions) / 13 (Codebase Alignment)
**Location:** `03-01-self-assign-rules.md` §New Functions/Methods ("skips a duplicable, view, or unknown substitution; reports `E10020` … for a nullable substitution"); `01-requirements.md` R1; `00-ambiguity-register.md` AR-7
**Codebase Evidence:** the concrete out-slot branch (`own/expr.rs:646-662`) tests only `BindingKind::OutSlot` (every nullable `borrow` parameter — `own.rs:785-790`) and reports `E10020` on an exact self-assign. Probes: concrete `borrow str?`, `u8[]?`, `raw i32?`, `cstring?` self-assigns all report exactly `E10020`; the generic instances at those substitutions are accepted at baseline. `duplicable()` is true for `str`/`raw`/`cstring` and slices, so a skip-first audit accepts all of them.
**The Problem:** R1/AR-7 promise that each substitution reproduces the concrete spelling; the implemented order breaks that for nullable views/raw/cstring — two implementers produce different accept/reject sets, and one reading leaves nullable-view self-assigns accepted while the concrete spelling rejects them.

**Options:** (A) test nullability before the duplicable/view/unknown skips (mirroring the concrete out-slot branch and the sibling audits, e.g. `own/instance_check.rs:197`), amend R1's sentence to "a non-nullable duplicable, view, or unknown instance stays accepted", and add pins (nullable view substitution red; non-nullable view substitution green); (B) keep skip-first acceptance and amend R1/AR-7 to record the divergence — requires a behavior ruling and leaves the stated conformance purpose unmet.

**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-004: A conditional-arm store through a lend root is accepted and use-after-frees the caller [🟠 MAJOR] — RESERVED (scope)

**Dimension:** 8 (Security — memory safety) / 4 (Completeness)
**Location:** not covered by any plan document; adjacent to `03-01` (the deferral logic the plan edits)
**Codebase Evidence:** `func st(borrow slot: String, pick: bool): void { let other: String = stringOf("bbbb"); slot = pick ? slot : other; }` is accepted; with `pick == false` the emitted conditional store frees the caller's handle (`ClString __cl_save_1 = slot; … else { slot = other; other = NULL; } if (__cl_save_1) { FreeString(__cl_save_1); }`) and the caller's later read is an ASan heap-use-after-free (generic `st<T>` at `String` identical; plain exit 134). Root cause: `lend_moves_judged` (`own/expr.rs:713-736`) defers whenever **any** source is a lend, so the target-side store rule never fires for this shape. The plan's new duty records only exact whole-binding self-assigns, so the defect survives the batch. The concrete spelling is affected, not only the generic instance.
**The Problem:** a newly discovered, verified memory-safety defect adjacent to DEF-100's construct. It is outside the five filed defects and the confirmed batch scope, and the plan neither fixes nor mentions it.

**Options:** (A) strict scope: document the residual as a known limitation in `03-01` (with the minimal repro) and file a new backlog defect (next free id `DEF-113` in `plans/00-roadmap.md`), to be fixed by a later plan; (B) expand this batch to judge conditional stores into lend roots — new design plus ST cases, a scope expansion requiring explicit user approval; (C) take no action.

**Recommendation:** Option A — the batch's confirmed scope is the five filed defects (AR-1); the residual gets a durable, honest record and a filed defect, not a silent gap.
**Resolution:** Resolved — User chose Option A (file + document residual): the limitation note lives in `03-01` §Known Adjacent Limitation and `01` Won't Have, and `DEF-113` is filed in `plans/00-roadmap.md`.

---

### PF-005: `Statement::Guards` is constructed for fixed-array targets; the byte-identity claim is false and the for-step/marker surfaces change [🟠 MAJOR]

**Dimension:** 6 (Feasibility Concerns) / 9 (Edge Cases) / 13 (Codebase Alignment)
**Location:** `03-03-view-store-guards.md` §Proposed Changes DEF-106 ("fixed-array bases reduce to `bounds_guard_condition == None` and emit nothing, so their emission stays byte-identical"); `01-requirements.md` §Compatibility
**Codebase Evidence:** `for_step_is_structured` (`own/loops.rs:802-813`) accepts only `Assign`/`Nop` in a for-step; a `Guards` statement in the step block degrades the emitted C `for` header to the label/goto form even when no trap line is emitted. The planned read-analysis arm can also remove the `-Werror` silencer marker (`codegen/function.rs:2199-2260`) while the fixed-array emission adds no read, which fails the strict build (`-Werror=unused-but-set-variable`). `codegen/control.rs:504-516` renders only step `Assign`s into the header.
**The Problem:** the statement's presence — not its emission — changes accepted-program output (loop shape) and can break the strict build; the compatibility claim cannot hold as written.

**Options:** (A) construct the `Guards` checks only when at least one level's base has a dynamic bound (a checker-side gate mirroring `bounds_guard_condition`), so fixed-only chains construct no statement — byte-identity, for headers, and markers are preserved (a dynamic-bound store in a for-step still degrades to the flat loop form, as any non-`Assign` step statement does, and the claim should say so); (B) keep constructing the statement and teach the step-structuring and liveness consumers about it — wider churn and still not byte-identical.

**Recommendation:** Option A. The checker knows every level's type, so the gate is one type test; it removes the entire fixed-array surface from the change.
**Mechanism note (challenger divergence):** the independent challenger preferred an emitter-side guard derivation (no IR variant; all store forms covered at the emission funnel) and flagged 03-03's premise "the emitter's `Place` carries only the final type" as false (`codegen/expr.rs:117-226` walks per-level place types). Reconciled: keep the gated `Statement::Guards` — the constant store forms funnel through several emitter arms (`Statement::Assign`, `Statement::Move`, the row store) while the checker has exactly two insertion points; the guard rides the read path's proven `ValueKind::Index` guard recursion verbatim instead of newly synthesized values; and AR-6 records the statement mechanism. The false premise is corrected either way, and the emitter-side alternative is recorded as considered.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-006: The DEF-105 walker double-reports `E10010` on update statements [🟠 MAJOR]

**Dimension:** 9 (Edge Cases) / 13 (Codebase Alignment)
**Location:** `03-03-view-store-guards.md` §Proposed Changes DEF-105 ("runs for every write target (assignment, compound assignment, and update)") and §Implementation Details ("called once near the top of `check_assign` and `check_update`")
**Codebase Evidence:** the statement dispatcher evaluates an update expression first (`own/statements.rs:156-169` → `eval` → the `Index` base walk at `own/expr.rs:281-289`), which already reports the null requirement; baseline probe: unnarrowed `xs[0]++` on `u8[]?` reports exactly one `E10010`. `check_nonnull_expr` pushes unconditionally and the diagnostics collector does not dedupe. No ST covers the update form.
**The Problem:** the designed call in `check_update` adds a second identical report for the same occurrence, breaking the repository's one-report convention (pinned in `borrow_view_and_shadowing_impl_test.rs`); the plan's suites would not catch the regression.

**Options:** (A) call the walker only in `check_assign` (plain and compound), correct 03-03's walk-scope sentence to say the update form is already covered by the operand read walk, and add a pin (exactly one `E10010` for unnarrowed `xs[0]++`); (B) keep the `check_update` call and dedupe against the read path by span/code — needless machinery for a walk that must not run.

**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-007: The DEF-106 necessity argument is factually wrong and the emitter-side alternative was never weighed [🟡 MINOR]

**Dimension:** 2 (Implicit Assumptions) / 10 (Scope Creep Indicators — the baseline claim) / 13 (Codebase Alignment)
**Location:** `03-03-view-store-guards.md` §Current Architecture ("The emitter's `Place` carries only the final type, so an emission-side bounds guard cannot recompute the per-level base types"); `00-ambiguity-register.md` AR-6; `00-index.md` Minimum-Sufficient Baseline
**Codebase Evidence:** `codegen/expr.rs:117-226` (`place()`, `place_base_type`) already walks per-level place types, and `bounds_guard_condition` takes base/index values the emitter could derive; the IR has no guard-only statement today, so "reuse the read path's trap statement" necessarily meant "add one" — but the stated impossibility is false, and the emitter-side option is absent from AR-6's rejected set.
**The Problem:** the baseline's "no new support surface" claim and AR-6's rationale rest on an incorrect premise; a reviewer consulting the decision record gets a false technical justification (the mechanism itself is defensible and retained — see PF-005's reconciliation).
**Options:** (A) keep the mechanism, rewrite the premise to the true reason (the checker already types every node; the guard reuses the read path's guard-value recursion; two checker insertion points cover all store forms), and record the emitter-side derivation as a considered-and-rejected alternative; (B) switch to the emitter-side design — the challenger's preference, declined for the insertion-surface and value-synthesis reasons in PF-005.
**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design.

---

### PF-008: The multi-instance DEF-100 report contract is ambiguous [🟡 MINOR]

**Dimension:** 1 (Ambiguities)
**Location:** `03-01-self-assign-rules.md` ("breaks after the first offending instance, so one site reports once"); `01-requirements.md` R1 ("one report per site"); `00-ambiguity-register.md` AR-7 ("each substitution reproduces exactly what the concrete spelling of the same store reports today")
**Codebase Evidence:** the existing audits break after the first reporting instance (`own/instance_check.rs:165, 217, 269`); `mono.instances()` is a deterministic vector; a body instantiated at both `String` and `String?` is reachable (probe accepted at baseline). A mixed-kind instantiation yields exactly one diagnostic under break-after-first, while AR-7's wording reads as per-offending-instance.
**The Problem:** two implementers produce different diagnostic sets for the same program; the mixed case is unpinned.
**Options:** (A) keep break-after-first (the repository convention) and state it explicitly in R1/AR-7 ("the first offending instance in instantiation order reports; later offending instances stay silent"); (B) report per offending instance — diverges from the existing pattern.
**Recommendation:** Option A (document; no new ST, because a mixed-kind pin would freeze fragile instance ordering).
**Resolution:** AI — delegated by --auto-design.

---

### PF-009: ST-10's `E10026` expectation needs the narrowed spelling [🟡 MINOR]

**Dimension:** 7 (Testability) / 12 (Consistency)
**Location:** `07-testing-strategy.md` ST-10; `02-current-state.md` DEF-101 row; `00-ambiguity-register.md` AR-8
**Codebase Evidence:** probes: `return v[0]` on `String[]?` unnarrowed → `E10010` (read path) **and** `E10026`; with `if (v != null)` → exactly `E10026`; on a non-nullable `borrow String[]` → `E10034`. ST-10 names no shape.
**The Problem:** an ST-10 written literally cannot turn green under this plan (the read path is unchanged and out of scope), and a test asserting "only E10026" fails on the unnarrowed form.
**Options:** (A) pin the narrowed spelling for the `E10026` green pin and note the unnarrowed form's `E10010` + `E10026` pair in the row; (B) leave the wording.
**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design.

---

### PF-010: Pins promised by AR-4/AR-9/03-01 are absent from the ST table [🟡 MINOR]

**Dimension:** 4 (Completeness Gaps) / 7 (Testability)
**Location:** `03-01-self-assign-rules.md` §Proposed Changes ("pins the rejected pairs (`xs[i] = xs[i]`, `xs[0] = xs[0]`, `xs[i] = xs[j]`) and the accepted pair (`xs[1] = xs[0]`, plus the mixed constant/runtime pairs)"); `00-ambiguity-register.md` AR-4 and AR-9; `01-requirements.md` R1; `07-testing-strategy.md` §P1
**Codebase Evidence:** probes: `xs[0] = xs[0]` → one `E10020`; `xs[0] = xs[i]` and `xs[i] = xs[0]` → accepted, exit 0; non-nullable `borrow i32[]` element self-assign (`xs[i] = xs[i]`) → accepted, exit 0; non-nullable view substitution `copySelf<u8[]>` → accepted, exit 0. None is an ST or impl case.
**The Problem:** the conservative rule's boundaries and the must-stay-accepted clauses (AR-4/AR-9 decisions) have no regression guard; a later audit change could silently flip them.
**Options:** (A) append the missing green pins (and, with PF-003, the nullable-view red pin) to the P1 set as ST-23 – ST-27 and update the range references; (B) trim the promises — contradicts the AR decisions.
**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design.

---

### PF-011: The modification set and blast radius are incomplete [🟡 MINOR]

**Dimension:** 5 (Dependency Issues) / 13 (Codebase Alignment)
**Location:** `00-index.md` Related Files; `02-current-state.md` §Relevant Files (`ir.rs` row); `99-execution-plan.md` tasks 1.2.1/1.2.2/3.2.2; `03-01-self-assign-rules.md` §Integration Points ("drained by the audit chain in `own.rs::check()`")
**Codebase Evidence:** `own/generics.rs:711-751` is an exhaustive `Statement` match with no wildcard — adding `Statement::Guards` fails compilation until an arm is added (walking the checks for instance collection, mirroring the `Assign`/`StorePointer` arms); `own/generics.rs:588-605` (`_ => false`) and `own/instance_check.rs:355-371` (`_ => {}`) silently skip guard values; the DEF-100 duty transfer runs through `Monomorphization::require_self_assign`-style plumbing (`ir/mono.rs:312-329`, `own.rs:128-148`) and the audit runs inside `instance_check::check()` (called at `own.rs:144`), not directly in `own.rs::check()`; `ir.rs` gains both the duty record and the `Statement::Guards` variant.
**The Problem:** the guaranteed `own/generics.rs` change is outside the planned modification set (the strict-profile diff review and acceptance criterion 5 measure against it), the duty-transfer hop is unnamed, and one integration-point sentence misplaces the audit.
**Options:** (A) add `own/generics.rs` (with the walk-arm intent), name the `require_self_assign` hop and the exact audit location, extend `02`'s `ir.rs` row to the `Statement::Guards` definition, and state that `own/statements.rs` is not expected to change (the checker inserts the guard before `emit_assigned`); (B) leave the lists and let the build error surface it.
**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design.

---

### PF-012: No findings ledger or phase-review evidence location is planned [🟡 MINOR]

**Dimension:** 4 (Completeness Gaps — process artifact)
**Location:** `00-index.md` §Document Index; `99-execution-plan.md` Lenses lines and success criterion 7; `00-ambiguity-register.md` AR-13 note
**Codebase Evidence:** the exec-plan skill records each phase-review ruling in the plan's findings ledger (`05-findings.md`); the sibling plan `plans/borrow-generic-integrity/` has one, indexed; this plan names neither a ledger nor a review-evidence location. AR-13's note lists two reviewers for P1 while 99 and 00-index (and the AGENTS.md matrix, since P1 is a lend language rule) include the semantics reviewer.
**The Problem:** the strict-profile audit trail for the three phase reviews has no planned home; the decision record contradicts the operative dispatch.
**Options:** (A) add `05-findings.md` (header + three phase sections), an index row, per-phase review-evidence notes in 99, and fix AR-13's note; (B) record review outcomes inline in 99 only.
**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design.

---

### PF-013: The container `RDC_ASAN` run is scheduled as a phase gate [🟡 MINOR]

**Dimension:** 7 (Testability) / 13 (Codebase Alignment)
**Location:** `99-execution-plan.md` Verify lines for tasks 1.2.4, 2.2.2, 3.2.3; `07-testing-strategy.md` §Testing Overview; `00-ambiguity-register.md` AR-11
**Codebase Evidence:** the mechanism is real (`borrow_generic_integrity_spec_test.rs:256-293` reads `RDC_ASAN` and applies `-fsanitize=address -no-pie`), but the repository's memory-safety-auditor brief and the last two cycles (`defer-integrity` AR-13; `def-backlog-closeout` 02/07) record the in-container full-suite `RDC_ASAN=1` run as timing-flaky and explicitly **not** a gate; the deterministic per-case host probe is the oracle.
**The Problem:** a timing flake during a memory phase can produce a false red at a gate step.
**Options:** (A) keep the container command but mark it non-gating and name the per-case host probes (`RDC_ASAN`-style emit + `cc -fsanitize=address -no-pie` + run) as the phase oracle, citing the precedent; (B) replace the phase verify with host probe commands only.
**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design.

---

### PF-014: ST-18/19/21 do not pin the trap-observation contract [🟡 MINOR]

**Dimension:** 7 (Testability)
**Location:** `07-testing-strategy.md` ST-18/19/21 ("traps `E11211`"); §Test Data ("reuse `borrow_generic_integrity_spec_test.rs` patterns")
**Codebase Evidence:** the repository's trap contract is `run_fault_spec_test.rs:60-116`: assert a non-success/aborted exit plus the exact first stderr line (`error E11211: index out of range`); the cited `assert_exit` helper (`borrow_generic_integrity_spec_test.rs:296-303`) compares `status.code()`, which is `None` for a SIGABRT — a test copied from it can never pass for a trap.
**The Problem:** "traps" is unpinned and the cited helper cannot express the observable.
**Options:** (A) word ST-18/19/21 with the run-fault contract (aborted exit + exact first stderr line; the ASan variant adds "no `AddressSanitizer` output") and cite the run-fault helper; (B) leave the assertion to the implementer.
**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design.

---

### PF-015: ST-22's input is unpinned and the ST ranges disagree across documents [🟡 MINOR]

**Dimension:** 7 (Testability) / 12 (Consistency)
**Location:** `07-testing-strategy.md` ST-22 ("one accepted program of each phase"); `03-03-view-store-guards.md` §Testing Requirements ("ST-13 – ST-21"); `99-execution-plan.md` task 3.2.3 ("the ASan probes for ST-8 and ST-18"); `01-requirements.md` acceptance criterion 1 ("ST-1 – ST-21"); task 3.1.1 ("ST-13 – ST-22")
**Codebase Evidence:** the existing determinism suite names its programs; "accepted" alone permits a program that exercises none of the changed emissions; ST-8 belongs to P2 while 3.2.3 issues ASan probes inside P3.
**The Problem:** three documents state the P3 test boundary differently; the determinism pin can be satisfied without touching a changed emission.
**Options:** (A) name ST-22's programs (e.g. the ST-8 out-slot program, the ST-15 slice program, and one P1 accepted program), fix 03-03's range to "ST-13 – ST-22", and change 3.2.3's probes to the P3 memory cases (ST-15, ST-18, ST-20); (B) delete ST-22 and rely on the per-phase impl re-emission pins.
**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design.

---

### PF-016: The DEF-105 `Unsafe` arm does not bump `unsafe_depth` [🟡 MINOR]

**Dimension:** 9 (Edge Cases) / 12 (Consistency)
**Location:** `03-03-view-store-guards.md` §Proposed Changes DEF-105 ("`ExprKind::Unsafe(inner)`: recurse (the `unsafe_depth` gate already disables the check inside an `unsafe` block, matching the read path)")
**Codebase Evidence:** the read path bumps `unsafe_depth` around the expression wrapper (`own/expr.rs:383-387`); the walker as specified recurses without bumping. Probes: `(unsafe xs[0]).x = 1` and the read counterpart are accepted at baseline; the literal walker would report `E10010` and reject the previously accepted unsafe write.
**The Problem:** the arm's justification is wrong for the expression wrapper, diverging from the read path AR-5 says to mirror.
**Options:** (A) bump `unsafe_depth` around the arm's recursion (true mirror; one line), with an optional acceptance pin; (B) drop the arm — the nested wrapper is reachable, so the walk would diverge instead.
**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design.

---

### PF-017: The concrete `E10034` mechanism attribution is wrong [🟡 MINOR]

**Dimension:** 12 (Consistency) / 13 (Codebase Alignment)
**Location:** `02-current-state.md` §Code Analysis DEF-100 ("falls through to the `owns_drop` tail"); `03-01-self-assign-rules.md` §Proposed Changes ("`borrow String` still reports `E10034` from the `owns_drop` tail") and the risk row ("a concrete fall-through plus the audit")
**Codebase Evidence:** `lend_moves_judged` returns true for the self source (`own/expr.rs:672-674`; the predicate includes `Mode::Borrow`), so the tail is unreachable; the report comes from `check_assign` → `bind_value` → `BindingEffect::Move` (`own/statements.rs:506-514`) → `move_place` (`own/place.rs:210-220`). Probe: one E10034 (identical message, so behavior is unaffected).
**The Problem:** the plan's explained path and its double-report risk argument rest on an unreachable code path; reviewers tracing the "kept path" are pointed at the wrong function.
**Options:** (A) correct both documents to the `bind_value` → `move_place` chain and restate the risk row; (B) leave the narrative.
**Recommendation:** Option A.
**Resolution:** AI — delegated by --auto-design.

---

## Delegated Resolutions (auto-design, policy v1)

Root invocation: `preflight-borrow-store-integrity-20261010-0036`. Eligibility: each resolution stays
within the plan's confirmed goals, the AR decisions it cites, and the confirmed scope; none adds
language surface, diagnostics, dependencies, or support machinery; `spec/` stays untouched.

| Finding | Decision (selected option) | Key evidence | Rejected alternatives | Confidence | Reopen triggers |
|---------|----------------------------|--------------|-----------------------|------------|-----------------|
| PF-001 | Correct the P2 red-phase expectation to "ST-8 and ST-9 fail; ST-10 – ST-12 pass"; annotate ST-9 red in 07 | probes (plain + ASan double-free); `cleanup.rs:498-502` | (B) split ST-9 — churn without benefit | High | a guarded return that does not take the direct branch |
| PF-002 | Cover all constant-index store forms with the guard (plain, compound, update, row) and add ST/trap pins | probes (`+=`/`++`/row OOB); `own/expr.rs:569-575, 744-758, 825-829` | (B) narrow R5 — leaves the filed defect class live | High | a store form that cannot ride the checker insertion points |
| PF-003 | Nullability before the duplicable/view/unknown skips; R1 text precised; nullable-view red pin + non-nullable-view green pin | probes (concrete E10020 on `str?`/`u8[]?`/`raw i32?`/`cstring?`; generic accepted); `own/expr.rs:646-662`; `own.rs:785-790` | (B) keep skip-first and record divergence — unmet conformance purpose | High | a concrete nullable spelling that is accepted |
| PF-005 | Gate `Guards` construction on a dynamic-bound level; fix the byte-identity wording; keep the mechanism, correct the premise | `own/loops.rs:802-813`; `codegen/function.rs:2199-2260`; `codegen/expr.rs:117-226` | (challenger) emitter-side derivation — declined: several emitter arms vs two checker insertion points, synthesized vs reused guard values | High (gate) / Med (mechanism; challenger diverged, reconciled) | a store form the checker cannot see the base type of |
| PF-006 | Walker in `check_assign` only; correct the walk-scope sentence; add the one-report update pin | probe (one E10010 at baseline); `own/statements.rs:156-169` | (B) dedupe by span — needless machinery | High | an update path that stops evaluating its operand |
| PF-007 | Rewrite the false necessity premise; record the emitter-side alternative as considered-and-rejected | `codegen/expr.rs:117-226` | (B) switch mechanisms — see PF-005 | High | an emitter-side derivation becoming materially smaller |
| PF-008 | Document break-after-first ("first offending instance in instantiation order reports") | `own/instance_check.rs:165/217/269`; deterministic `mono.instances()` | (B) per-instance reporting — diverges from convention | High | a user ruling preferring per-instance reports |
| PF-009 | Pin the narrowed `E10026` spelling; note the unnarrowed `E10010`+`E10026` pair | probes | (B) leave the wording — unsatisfiable pin | High | — |
| PF-010 | Append the missing P1 pins (ST-23 – ST-27) and align range references | probes (all green at baseline except the PF-003 red pin) | (B) trim the AR promises — contradicts AR-4/AR-9 | High | — |
| PF-011 | Add `own/generics.rs` + walk arm; name the duty-transfer hop and the audit location; extend the `ir.rs` row; state `own/statements.rs` unchanged | `own/generics.rs:588-605, 711-751`; `own.rs:128-148`; `ir/mono.rs:312-329` | (B) let the build error surface it — incomplete review diff | High | — |
| PF-012 | Add `05-findings.md` + index row + review-evidence notes; fix AR-13's P1 reviewers | sibling plan convention; exec-plan ledger expectation | (B) inline records — no durable ledger | High | — |
| PF-013 | Container `RDC_ASAN` non-gating; host probes are the phase oracle | memory-safety-auditor brief; defer-integrity AR-13; def-backlog-closeout 02/07 | (B) host-only verify — loses the convenience run | High | the flake policy changing |
| PF-014 | Run-fault trap contract wording for ST-18/19/21; cite the run-fault helper | `run_fault_spec_test.rs:60-116`; `borrow_generic_integrity_spec_test.rs:296-303` | (B) leave to the implementer | High | — |
| PF-015 | Name ST-22's programs; fix the ranges; correct 3.2.3's probe list | `determinism_spec_test.rs`; cross-document ranges | (B) delete ST-22 — loses the spec-level determinism pin | High | — |
| PF-016 | Bump `unsafe_depth` around the `Unsafe` arm's recursion | `own/expr.rs:383-387`; probes | (B) drop the arm — leaves a reachable divergence | Med-High | the read path changing its unsafe gate |
| PF-017 | Correct the two attribution sentences and the risk row | `own/expr.rs:672-674`; `own/place.rs:210-220`; probe | (B) leave the narrative — misleading audit trail | High | — |

**Reserved decision (user):** PF-004 — file-and-document (recommended) vs batch scope expansion.

## Iteration 2 — Fix Verification

Applied and verified (bounded re-scan of the unchanged audit target; the artifact changed only
inside `plans/borrow-store-integrity/`, plus the roadmap rows and the new ledger).

**Verified revision:** HEAD `9043219` — plan tree `4f593f8202bd3f47805b78673dd6ffcdd27b1488`
(the applied fixes are the only artifact change). If the artifact's git blob changes from this
revision, consume only a targeted re-check of the changed sections (the staleness rule), not a
set-wide audit.

| Finding | Resolution applied | Where |
|---------|--------------------|-------|
| PF-001 | P2 red expectation corrected; ST-9 annotated red; ST-9 probe added to the phase verify | 99 (2.1.2, 2.2.2), 07 ST-9 |
| PF-002 | Constant-index store forms covered (plain/compound/update/row); ST-29/ST-30 added; tasks and file lists extended | 01 R5, 03-03, 07, 99 (3.1.x/3.2.x/3.3.1), AR-6 note |
| PF-003 | Nullability checked before the skips; R1 precised; ST-27 red pin + ST-26 green pin | 01 R1, 03-01, 07, 99 (1.1.x/1.2.2), AR-7 note |
| PF-004 | User ruling A: limitation note + `DEF-113` filed | 00-index, 01 Won't Have, 03-01 §Known Adjacent Limitation, plans/00-roadmap.md |
| PF-005 | Dynamic-bound construction gate; byte-identity/for-step wording corrected; mechanism premise fixed | 03-03, 01 Compatibility, AR-6 note |
| PF-006 | Walker in `check_assign` only; update pin ST-28; walk-scope sentence corrected | 03-03, 07, 99 (3.2.1) |
| PF-007 | Premise rewritten; emitter-side alternative recorded | 03-03, AR-6 note |
| PF-008 | Break-after-first documented (first offending instance reports) | 01 R1, 03-01, AR-7 note |
| PF-009 | ST-10 rewritten (narrowed `E10026` pin; unnarrowed `E10010` note) | 07, 02 |
| PF-010 | ST-23 – ST-27 appended; range references aligned | 07, 03-01, 99 |
| PF-011 | `own/generics.rs` + walk arm, transfer hop, audit location, `ir.rs` row extended | 00-index, 02, 03-01, 03-03, 99 |
| PF-012 | `05-findings.md` created + index row; AR-13 note fixed | 05-findings, 00-index, AR-13 |
| PF-013 | `RDC_ASAN` marked non-gating; host probe oracle named | 07, 99 (1.2.4/2.2.2/3.2.3), AR-11 note |
| PF-014 | Trap-observation contract worded for ST-18/19/21 | 07 |
| PF-015 | ST-22 programs named; ranges fixed; 3.2.3 probe list corrected | 07, 03-03, 99, 01 AC1 |
| PF-016 | `Unsafe` arm bumps `unsafe_depth` | 03-03 |
| PF-017 | Concrete-E10034 attribution corrected | 02, 03-01 |

Verification evidence:

- Stale-phrase sweep clean: no "ST-13 – ST-21", "ST-1 – ST-21", "owns_drop tail",
  "ST-8 and ST-18", or "open ones pending" in the plan documents (remaining occurrences are this
  report's own finding text).
- ST accounting: 30 cases (ST-1 – ST-30), each with an owning phase; every red-phase list (99
  tasks 1.1.2/2.1.2/3.1.2) matches the per-case annotations (P1: ST-1/ST-3/ST-27; P2: ST-8/ST-9;
  P3: ST-13/ST-14/ST-18/ST-29).
- Task accounting unchanged: 22 tasks, `0/22` progress; phase deliverables updated with the new
  ranges.
- Cross-references re-checked after the edits (AR-6/AR-7/AR-11/AR-13 notes, ST ranges, file
  lists, the `05` index row, the roadmap rows including `DEF-113`).
- No new issues were introduced by the fixes in the bounded scan.

## Verdict

**✅ PREFLIGHT PASSED — all 17 findings resolved** (16 delegated under auto-design, 1 user ruling).
The roadmap rows advance to `Plan Preflighted`; `DEF-113` is filed for the documented residual. The
plan may enter `exec-plan`.
