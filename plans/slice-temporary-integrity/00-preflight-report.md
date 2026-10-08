# Preflight Report: slice-temporary-integrity

> **Status**: ✅ PREFLIGHT PASSED — all 18 findings resolved (1 critical, 7 major, 10 minor); fixes applied and verified (iteration 2; PF-018 fix applied)
> **Iteration**: 1 (first scan) → 2 (fix verification)
> **Artifact**: Implementation plan at `plans/slice-temporary-integrity/` (10 documents)
> **Artifact revision**: audited `f2de87651d74a3ce6e978fa7f343768a26e5977871088abfb302b73df4bd8d72`; post-fix `f0c46fc9c84589d5090aad3d7a84b93b8ccf7615e7bf5dc1429f71abba0e24ef` (iteration 2)
> **Codebase Grounded**: lead recon + 5 fresh-context auditor clusters + 1 independent challenger; 49/49 cited line ranges resolve; behavior claims verified with the built `clc` and host ASan probes
> **Last Updated**: 2026-10-08 09:15
> **Scope mode**: strict · **Normal mode** (no auto-design / auto-effort / explore tokens)

⚠️ **SAME-SESSION REVIEW**: This artifact was created earlier in the current session, by the same
model family. Same-agent bias risk is elevated; the scan used fresh-context auditor dispatches and
an independent challenger to counteract it. Consider a human domain expert review before execution
given the type-model/ownership stakes.

## Codebase Context Summary

**Tech Stack:** Rust (std-only workspace), emitting C99 for `host`, `hosted-amiga`, and
`freestanding`; embedded Clorraine standard library; pinned Docker SDK toolchain.
**Architecture:** staged compiler (`lex`/`parse`/`load`/`resolve`/`sema`/`own`/`ir`/`codegen`);
ownership lowering in `compiler/src/own/`, C emission in `compiler/src/codegen/`; specification
tests are the immutable oracle under `spec/` and `compiler/tests/*_spec_test.rs`.
**Key Files Examined:** `compiler/src/own/lower.rs`, `compiler/src/own/call.rs`,
`compiler/src/own/statements.rs`, `compiler/src/own/expr.rs`, `compiler/src/own/loops.rs`,
`compiler/src/types/{mod,keys,table}.rs`, `compiler/src/codegen/{types,program,calls,guards,function,drop,expr}.rs`,
`compiler/tests/{def_backlog_closeout,array_value_integrity,call_value_integrity,runtime_value_integrity,ownership_drop,def_backlog_completion}_*_test.rs`,
`spec/04`, `spec/08`, `spec/14`, `spec/17`.
**Key Observations:**
- The plan's four design threads all build on real existing machinery; the defects and their
  root-cause citations were verified.
- The v1 `E10026` partial-move limit is *shape-based*, not type-based, and the plan's mandatory
  ST cases collide with it (PF-001, PF-009).
- Two pre-existing memory-safety defects surfaced beyond the filed batch: a generic borrow-view row
  move (PF-001 evidence) and an eager-take-temp loop adoption (PF-003 evidence).

**Reference Verification:** 49/49 cited line ranges resolve; all named helpers/symbols exist; the
one broken reference is `99-execution-plan.md`'s quality-checklist link (PF-016).

## Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 1 | 🟡 |
| 2 | Implicit Assumptions | 2 | 🟠 |
| 3 | Logical Contradictions | 1 | 🔴 |
| 4 | Completeness Gaps | 2 | 🟠 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 0 | — |
| 7 | Testability | 2 | 🟠 |
| 8 | Security Blind Spots | 0 | — (clean, evidence recorded) |
| 9 | Edge Cases | 2 | 🟠 |
| 10 | Scope Creep Indicators | 0 | — (clean) |
| 11 | Ordering & Sequencing | 1 | 🟡 |
| 12 | Consistency | 2 | 🟡 |
| 13 | Codebase Alignment | 4 | 🟠 |

## Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 1 | resolved — fix applied & verified |
| MAJOR | 7 | resolved — fixes applied & verified |
| MINOR | 10 | 10 resolved (fixes applied) |
| OBSERVATION | 0 | — |

---

## CRITICAL Findings

### PF-001: Mandatory ST cases contradict the unchanged `E10026` element-move limit 🔴 CRITICAL

**Dimension:** 3 (Logical Contradictions) / 1 / 7
**Location:** `07-testing-strategy.md` ST-2, ST-3, ST-6, ST-7; `01-requirements.md` R1, R2;
`03-01` Error Handling; `99-execution-plan.md` tasks 1.1.2, 2.1.2; AR-3
**Codebase Evidence:** `compiler/src/own/statements.rs:303-308` (E10026 report),
`compiler/src/own/expr.rs:201-217` (`is_ownerless_element_move`),
`compiler/src/own/lower.rs:927` (`consume_place` maybe-owning gate). Lead reproduction:
`let row: String[2] = slice<String[2]>(mkGrid(), 0, 2)[1];` → `error E10026` (pre-fix); the filed
generic spelling (`storeRows<T>(make: func(): T[2][2], i: usize): T[2]`) compiles and host ASan
reports a double free.
**The Problem:** ST-2/ST-6/ST-7 and the R1/R2 examples are written as concrete spellings that the
deliberate v1 partial-move limit rejects before any typing applies, so they can never go green after
the planned Phase 1/2 changes (which leave that rule untouched). The actual filed reproductions are
the generic/maybe-owning spellings. Additionally, R1 claims a borrow-backed view move reports
`E10026`, which holds only for concrete instantiations: the generic borrow-view form is accepted
today and corrupts (challenger saw double-free/UAF). ST-3 also mixes `String[2]` with `slice<i32[2]>`
(a mismatch that reports `E10200`, not the stated `E10026`).

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Re-author ST-2/ST-6/ST-7 (and the R1/R2 examples) with the filed generic spellings; keep the concrete forms as `E10026` regression pins; correct R1's borrow-view wording; record the generic borrow-view escape as a follow-up defect row | Achieves red→green within the approved scope; matches the actual filings; preserves the oracle | A follow-up defect row is needed outside this plan |
| B | Expand this plan to accept concrete owned-temporary row moves (lift the `E10026` limit for fixed-array rows) and fix the generic borrow-view escape | Makes the concrete spelling work | Language-rule change requiring instantiation-time checking; contradicts AR-3/R1 ("rules unchanged"); disproportionate machinery |

**Recommendation:** Option A — the approved scope (AR-3) keeps the existing element rules; the
generic spelling is the filed defect and yields a genuine red.
**Hardening:** independent challenger — confirmed critical; recommendation A endorsed (the SEGV
detail corrected to double-free; ST-7 may use a duplicable spelling as a justified-pass variant).

**User Decision:** Resolved — User chose Option A (re-author the cases and R1/R2 examples to the
filed generic spellings; concrete forms stay `E10026` regression pins; the generic borrow-view
escape is a follow-up defect outside this plan).

---

## MAJOR Findings

### PF-002: The `E10200` mechanism named in AR-9 / 03-01 does not exist 🟠 MAJOR

**Dimension:** 13 (Codebase Alignment — Stale Assumptions)
**Location:** `03-01-slice-row-views.md` Proposed Changes/Implementation Details; AR-9;
`99-execution-plan.md` task 1.2.1
**Codebase Evidence:** `compiler/src/types/table.rs:155,217` — `Item::Intrinsic` is deliberately
not registered; `compiler/src/sema/mod.rs:541-552`; `compiler/src/types/mod.rs:595-614`
(`implicit_conversion` has no array→slice arm). Probe: `slice<i32>(grid, 0, 2)` with
`grid: i32[2][3]` is accepted today; registering the declared signature also rejects the *agreeing*
call (`slice<i32[3]>(grid, ...)`).
**The Problem:** The plan states the agreement check runs "through the declared intrinsic
signature so the ordinary conversion diagnostic applies"; that path is unreachable, and naive
registration would reject correct calls and change checking for every intrinsic.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Report `E10200` directly from `lower_intrinsic_call` by comparing the explicit type argument with the source's view element; state that intrinsic registration/sema changes are out of scope | Smallest; keeps AR-9's observable outcome; no suite-wide checking change | The diagnostic is hand-raised rather than path-derived |
| B | Register intrinsic signatures and add an array→slice conversion rule | Uses the ordinary path | Legalizes `sink(grid)`; touches sema for all intrinsics; spec-adjacent surface |

**Recommendation:** Option A — verified smallest path; B is a language-surface change outside the
approved scope.
**Hardening:** independent challenger — confirmed; A endorsed; correct ST-5's expected message to
the actual mismatch.

**User Decision:** Resolved — User accepted recommendation: Option A.

---

### PF-003: Register-only release on the eager temporary double-frees adoptions and projections 🟠 MAJOR

**Dimension:** 2 (Implicit Assumptions) / 13
**Location:** `03-03-root-and-receiver-binding.md` Proposed Changes item 1; `00-index.md`
Smallest viable design; `99-execution-plan.md` task 3.2.1
**Codebase Evidence:** `compiler/src/own/lower.rs:135-145` (eager branch, bare `Temp`),
`:895-905` (`materialize_value`), `:1074-1095` (`projects_from_place`), `:1151-1156`
(destination-temp owners); `compiler/src/codegen/function.rs:931-1005` (`emit_assign` never fails
to empty a temp — it never empties); pin `runtime_value_integrity_impl_test.rs:170-188`.
Challenger-built counterfactual: register-only double-frees `let r = mkfrom(mka())` and
`len(mkfrom(mka()).a)`; the destination-marked materializer alone fixes the projection case but not
whole-result adoption. It also reproduced an unfiled pre-existing loop double-free
(`while (...) { let r: Pair = mkfrom(mka()); }`).
**The Problem:** "Register exactly like the deferred branch" is not sufficient: the eager temporary
needs destination-temp status and an emptying adoption, or the release double-frees.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Use the destination-temp materializer (`mark_destination_temp` + register) **and** emit an emptying adoption (`AssignThenMoveOut`) when the lowered RHS is a registered destination temp; add spec cases for whole-result adoption, the borrowed projection, and the loop re-assignment | Challenger-built and verified: all four probes clean; also fixes the unfiled loop double-free | Touches `prepare_store` in addition to the root binds |
| B | Keep register-only and make every adopting/projection path empty registered eager temps | Avoids the destination-temp concept | Broader: touches assignment emission and `projects_from_place` consumers |

**Recommendation:** Option A — the smallest correct fix; include the unfiled loop double-free as a
Phase 3 spec case (and consider a roadmap row if it is not fully covered).
**Hardening:** independent challenger — confirmed; option A corrected to the combined fix
(destination mark + releasing adoption), verified by patch.

**User Decision:** Resolved — User accepted recommendation: Option A (combined fix).

---

### PF-004: The member-receiver bind has no position context and preempts consume/loop machinery 🟠 MAJOR

**Dimension:** 13 (Architecture Mismatch / Test Impact)
**Location:** `03-03-root-and-receiver-binding.md` Proposed Changes item 3; `99-execution-plan.md`
task 3.2.2
**Codebase Evidence:** `compiler/src/own/lower.rs:34-37` (Member lowering, no position knowledge),
`:1447-1486` (`bind_intrinsic_argument`), `compiler/src/own/statements.rs:287-347` (`prepare_store`
branch), `compiler/src/own/loops.rs:48-50,315-321` and `:682-770`. Challenger probes:
`let s: String = mkv().a` double-frees and the pinned `s = __cl_owned0.a; __cl_owned0.a = NULL;`
shape disappears; the guarded literal `flag && sink(Box { v: mkv().a, w: 0 })` double-frees
(base leaks); loop-condition pins break (shape change, not a call-frequency change).
**The Problem:** "Adopting positions are untouched" cannot be guaranteed from inside
`lower_value(Member)`; the central bind changes store-branch selection, guarded-literal ownership,
and loop-condition emission.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Bind at read positions only: invoke the receiver bind from the specific read consumers (cast/intrinsic/argument/return reads) and explicitly exclude `consume_place`, guarded positions, and loop-condition lowering; add the sites and pins to the phase list | Smallest correct placement; matches the existing bind call sites | Requires enumerating the read consumers |
| B | Keep the central Member bind and thread explicit position context through lowering | One bind point | Invasive flag threading; the same enumeration problem |
| C | Realize the receiver bind as a statement-scope walk like `materialize_owned_calls` | Reuses a proven pattern | Still position-specific; replaces the mechanism |

**Recommendation:** Option A.
**Hardening:** independent challenger — confirmed; sub-claim (b) narrowed to a shape/pin/contract
break (no semantic call-frequency defect observed).

**User Decision:** Resolved — User accepted recommendation: Option A.

---

### PF-005: Unconditional suffix parenthesization breaks specification pins and R8 stability 🟠 MAJOR

**Dimension:** 13 (Test Impact / Migration & Compatibility)
**Location:** `03-04-element-store-drop-spelling.md` Proposed Changes; `99-execution-plan.md`
tasks 4.2.1, 4.3.1; R8
**Codebase Evidence:** `compiler/src/codegen/drop.rs:86,100` (statement-form appends),
`compiler/src/codegen/function.rs:509` (the only dereference base).
Challenger patch of unconditional grouping: `FreeString(grid[0][0])` →
`FreeString(((grid)[0])[0])`, `if (__cl_owned0.a)` → `if ((__cl_owned0).a)`; breaks two
specification tests (`ownership_drop_spec_test.rs:377-401`,
`def_backlog_completion_spec_test.rs:750-768`) and ~20 implementation assertion sites.
**The Problem:** "Render `({c_name})` at each suffix site" re-spells every struct-field and
fixed-array drop, contradicting R8 and the "specification tests stay untouched" rule.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Parenthesize only when the base is not a primary/postfix expression (the filed path's base `*({pointer_c})`); existing spellings stay byte-identical | Fixes DEF-75; keeps the oracle and pins green | Slightly conditional emission |
| B | Keep unconditional grouping and migrate every affected pin | Uniform spelling | Requires oracle (spec-test) edits without a spec justification |

**Recommendation:** Option A.
**Hardening:** independent challenger — confirmed; A verified by patch (only the dereference base
needs grouping).

**User Decision:** Resolved — User accepted recommendation: Option A.

---

### PF-006: The deferred consume temp is not emptied by the indexed-store adoption (loop double free) 🟠 MAJOR

**Dimension:** 9 (Edge Cases) / 4
**Location:** `03-02-deferred-consume-preparation.md` Proposed Changes/Integration;
`99-execution-plan.md` tasks 2.2.1, 2.3.1
**Codebase Evidence:** `compiler/src/codegen/guards.rs:111-117,317-328` (reassigned-temp drop before
every fill), `compiler/src/codegen/function.rs:496-512` (`StorePointer` owned branch zeroes only
`__cl_arm`), literal consumers empty via `emit_arm_source_moves` (`:1553-1558,1987-1992`).
**The Problem:** In `while (...) { out[i] = mk().a; }`, the second iteration's reassigned-temp drop
frees the value the stored element owns; element cleanup frees it again → double free/UAF. No test
covers the loop form.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Zero the deferred temp at the indexed-store adoption (as the literal/array consumers do) and add a loop-form ASan probe | Closes a live memory-safety hole in an in-scope shape | One extra adoption site |
| B | Prove/pin that the reassigned drop is a no-op for this temp | No emission change | Refuted by the code (the release is registered) |
| C | Declare the loop form out of scope and file it | Smaller plan | Leaves a double free in the DEF-72/73/74 workstream |

**Recommendation:** Option A.
**Hardening:** independent challenger — confirmed; A endorsed (B refuted by code; C rejected).

**User Decision:** Resolved — User accepted recommendation: Option A.

---

### PF-007: Leak-only ST cases have no red observable under the stated verification 🟠 MAJOR

**Dimension:** 7 (Testability)
**Location:** `07-testing-strategy.md` ST-8 – ST-11 and Test Data; `99-execution-plan.md` red-phase
tasks 1.1.2/2.1.2/3.1.2 and Verify lines; `01-requirements.md` acceptance 1
**Codebase Evidence:** `compiler/tests/def_backlog_closeout_spec_test.rs:239-245` (`RDC_ASAN`
gate), `:257-261` (`assert_no_asan` vacuous without the sanitizer). Pre-fix probes: ST-8/9/10/11
pass with correct exit codes under a plain run; host ASan shows the leaks (12/12/10/36+ bytes).
**The Problem:** The plan's phases verify `make check` only, and says the container `RDC_ASAN` path
stays unused; the red phase for R4/R5/R6 cannot be demonstrated as written.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Name the host ASan red/green command in 07 and the red-phase tasks; mark ST-8 – ST-11 ASan-red | Makes the red phase reproducible; matches the prior plans' and the memory auditor's method | Adds a documented command to run |
| B | Require a stronger plain-run observable for each leak case | No sanitizer dependency | Impossible for pure leaks |

**Recommendation:** Option A — the plan already names host ASan as the oracle; only the red-phase
procedure is missing.
**Hardening:** independent challenger — confirmed (claim narrowed accordingly); A endorsed.

**User Decision:** Resolved — User accepted recommendation: Option A.

---

### PF-009: R3's fix surface may not cover the duplicable-element order variant 🟠 MAJOR (medium confidence)

**Dimension:** 4 (Completeness) / 9
**Location:** `01-requirements.md` R3; `07-testing-strategy.md` ST-7;
`03-02-deferred-consume-preparation.md` item 4; `99-execution-plan.md` task 2.2.2
**Codebase Evidence:** `compiler/src/own/lower.rs:927` (consume path returns early for non-owning
values); `compiler/src/codegen/function.rs:1455-1569` (initializer form vs store form);
`emit_assign`'s whole-value guard fill (`:931-1005`). Challenger reproduction: trace 21 with i32
fields (`Box { w: f(), v: make()[i] }`), where nothing is consumed; ST-7's type-free spelling hides
which variant is pinned.
**The Problem:** The order bug also occurs without ownership; a "deferred-consume predicate" may not
select the store form for that field, so ST-7 could stay red (or pass for the wrong variant).

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Add the duplicable-element variant to R3/ST-7 and ensure the fix covers it (force the store form / per-field fill for literals with deferred base fills) | R3's order rule is type-independent; closes the gap | May widen the Phase 2 emission change |
| B | Scope R3 to owning elements and file the duplicable order variant as a follow-up | Smaller phase | Leaves an accepted order reversal under a requirement that names the exact program |

**Recommendation:** Option A (verify with the bounded re-check after the fix).
**Hardening:** independent challenger — raised as an additional risk; medium confidence, bounded
re-verification required during Phase 2.

**User Decision:** Resolved — User accepted recommendation: Option A.

---

## MINOR Findings

### PF-008: Phase 2's review dispatch omits the semantics reviewer 🟡 MINOR

**Dimension:** 4 (Completeness) / 11
**Location:** `99-execution-plan.md` task 2.3.3 (and 3.3.3); Phase 2 lens line
**Codebase Evidence:** `AGENTS.md` phase-review dispatch — evaluation-order changes require the
semantics reviewer; `plans/def-backlog-closeout` used it for the same class; Phase 2's declared lens
is "evaluation-order and statement-temporary contracts".
**The Problem:** The plan omits the reviewer the repository's own contract assigns to this surface.
**Recommendation:** Add `semantics-reviewer` to 2.3.3 (and consider 3.3.3) with the
`spec/04-expressions.md` §3 lens.
**Hardening:** independent challenger — confirmed with **severity downgraded to MINOR** (no direct
technical defect; fix is a one-line dispatch addition).

**User Decision:** Resolved — User accepted the correction.

---

### PF-010: "Every affected program is miscompiled today" / R8 wording is false 🟡 MINOR

**Dimension:** 2 (Stale Assumptions)
**Location:** `00-index.md` Overview; AR-3 resolution note; `07-testing-strategy.md` R8 row;
`01-requirements.md` R8; `02-current-state.md` risks
**Codebase Evidence:** `residual_defect_completion_impl_test.rs:380-402` and
`array_value_integrity_spec_test.rs:569-695` build and run correctly today; only their keys,
typedefs, and emission change under R1 (challenger reproduced exit 7 on the nested-access form).
**The Problem:** The claim mis-states the compatibility picture; the emission sweep must expect
intentional diffs in correct multi-extent programs, not only in miscompiled ones.
**Recommendation:** Reword to "programs whose behavior depends on the view element are wrong; the
rest are retyped", and list the retyped-but-correct pins in the sweep expectations.

**User Decision:** Resolved — User accepted the correction.

---

### PF-011: `zero_view_helper` and the builder spellings are omitted from the one-helper consumer list 🟡 MINOR

**Dimension:** 13 (Impact Blindness)
**Location:** `03-01-slice-row-views.md` Proposed Changes item 2
**Codebase Evidence:** `compiler/src/codegen/types.rs:67-75` (`zero_view_helper` uses
`slice_element_key`), callers `compiler/src/codegen/expr.rs:330-338` (null view) and
`compiler/src/codegen/cleanup.rs:670-677` (empty return); `compiler/src/codegen/calls.rs:288-325`
(builder spellings via `intrinsic_element`).
**The Problem:** A null/empty row view would call a helper keyed by the flat element while the
typedef is row-keyed → strict build/link failure in an untested path.
**Recommendation:** Include `zero_view_helper` (and the builder spellings) in the consumer list;
add nullable-row-view and empty-return implementation tests.

**User Decision:** Resolved — User accepted the correction.

---

### PF-012: ST-case specification precision 🟡 MINOR

**Dimension:** 7 (Testability)
**Location:** `07-testing-strategy.md` ST-1, ST-11, ST-12
**Codebase Evidence:** ST-1 runs correctly pre-fix (accepted, strict clean, exit 7) — a justified
pass, not red evidence; ST-12's first program omits `out`'s element type (only a struct element with
owning fields reproduces the pointer-arrow failure); several rows lack explicit expected exit codes.
**The Problem:** The authoring rule requires concrete input/output pairs and red-phase honesty.
**Recommendation:** Mark ST-1 as a justified pass; state ST-12's element type; add explicit exit
codes to ST-2/6/8/9/11.

**User Decision:** Resolved — User accepted the correction.

---

### PF-013: `view_element` contract left ambiguous 🟡 MINOR

**Dimension:** 1 (Ambiguities)
**Location:** `03-01-slice-row-views.md` New Types/Interfaces
**Codebase Evidence:** `Ty::iterated_element` (`compiler/src/types/mod.rs:375-386`) is the required
computation for both row representations.
**The Problem:** "`== ty.iterated_element()` in the common case" plus "location at the implementer's
discretion" implies an unstated divergence; key, typedef, builder, and recovery must agree exactly.
**Recommendation:** State the contract as exactly `iterated_element()` with `Ty::unknown()` for
non-arrays, or call it directly and drop the helper.

**User Decision:** Resolved — User accepted the correction.

---

### PF-014: Phase 1 step ordering makes Step 1.2's verify red 🟡 MINOR

**Dimension:** 11 (Ordering & Sequencing)
**Location:** `99-execution-plan.md` tasks 1.2.* and 1.3.1
**Codebase Evidence:** the flat-name pins
(`residual_defect_completion_impl_test.rs:398-401`, `array_value_integrity_impl_test.rs:565`,
`compiler/src/codegen/types.rs:469-476`) are migrated in 1.3.1, after 1.2.5's green check.
**The Problem:** `make check` cannot be green at the end of Step 1.2; the green phase is reported
blocked or forced premature.
**Recommendation:** Move the pin migration into Step 1.2 (before 1.2.5), or qualify Step 1.2's
verify line.

**User Decision:** Resolved — User accepted the correction.

---

### PF-015: Expression-form drop builders keep the suffix-precedence hazard 🟡 MINOR

**Dimension:** 9 (Edge Cases, latent)
**Location:** `03-04-element-store-drop-spelling.md` Proposed Changes (names only the statement-form
functions)
**Codebase Evidence:** `compiler/src/codegen/drop.rs:178-222` (`fixed_array_drop_expr`,
`struct_drop_expr`); callers pass only `__cl_tmpN` today (`guards.rs:238,764,786`).
**The Problem:** AR-10's rationale ("the same suffix-precedence hazard") applies to the
expression-form twins; latency depends on today's callers only.
**Recommendation:** Parenthesize both expression-form suffix sites in the same change, or record in
03-04 why the expression form is unreachable with a suffix base.

**User Decision:** Resolved — User accepted the correction.

---

### PF-016: Reference integrity bundle 🟡 MINOR

**Dimension:** 12 (Consistency)
**Location:** `99-execution-plan.md:30` (quality-checklist link); `99-execution-plan.md` Steps 2.2
and 3.2 (`§Implementation Details` refs); `03-01-slice-row-views.md:42`
(`pointer_to_array_type, compiler/src/codegen/types.rs:84-87`)
**Codebase Evidence:** `../../../.opencode/...` resolves outside the repo (prior plans name the
file in backticks: `plans/generic-field-integrity/99-execution-plan.md:31`); 03-02/03-03 have no
"Implementation Details" section (the content is under §Proposed Changes);
`pointer_to_array_type` is defined at `compiler/src/codegen/types.rs:240`, while the typedef field
needs the declarator helper at `:254-267` (lines 84-87 are the usage site).
**The Problem:** One broken link, two dangling section references, and one helper/line citation
drift.
**Recommendation:** Plain-text the checklist name; retarget the section refs to §Proposed Changes;
cite/use the declarator helper.

**User Decision:** Resolved — User accepted the correction.

---

### PF-017: Stale text and provenance bundle 🟡 MINOR

**Dimension:** 12 (Consistency)
**Location:** `00-index.md` Minimum-Sufficient Baseline; `07-testing-strategy.md` ST-4 source
**Codebase Evidence:** the index says "parenthesize the field append" (AR-10 later approved both
suffix sites); the DEF-70 filing is SA-003 + PE-001
(`plans/def-backlog-closeout/99-execution-plan.md:143`), while ST-4 cites SA-001/SA-005 (the
closeout's resolved gate findings).
**The Problem:** Stale text after AR-10 and a mislabeled provenance.
**Recommendation:** Update the smallest-design sentence to both suffix sites; cite SA-003/PE-001 for
the filing.

**User Decision:** Resolved — User accepted the correction.

---

### PF-018: The nested-access pre-fix symptom names the wrong diagnostic 🟡 MINOR

**Dimension:** 2 (Stale Assumptions)
**Location:** `02-current-state.md` Gap 1; `07-testing-strategy.md` ST-4 expected cell
**Codebase Evidence:** The ST-4 program's pre-fix strict build fails with "subscripted value is
neither array nor pointer nor vector" at the view-scalar nested access; the quoted
`-Wint-conversion` appears for a different element spelling (iteration-2 probe).
**The Problem:** The claimed behavior (the nested access mis-types and the strict build fails)
holds; the named diagnostic does not match the ST-4 spelling, so the document pairs a correct
expectation with an inaccurate observation.
**Recommendation:** Describe the symptom generically ("the strict build fails at the mis-typed
nested access; the exact diagnostic depends on the element spelling") in 02 and ST-4.

**User Decision:** Resolved — User accepted the recommendation (wording fix applied).

---

## Iteration 2 (Fix Verification)

> **Method**: one fresh-context auditor re-read the plan set, verified every applied resolution
> against the report, ran pre-fix probes for the re-authored cases, and re-checked cross-document
> consistency and the citation set.

| Result | Count |
|--------|-------|
| Fixes verified clean | 16 |
| Residuals (same finding, fix completed this iteration) | PF-012 (ST-8/ST-9 exit codes) |
| Wording consequences of accepted fixes (applied) | PF-001 (register AR-3 cell), PF-010 (corpus-row scope) |
| New minor findings | PF-018 (resolved — wording fix applied) |
| New contradictions introduced by the edits | none |

Verified evidence highlights: probes reproduced ST-1 (exit 7, justified pass), ST-2 (generic double
free), ST-3 (`E10026`), ST-4 (strict-build failure), ST-5 (accepted pre-fix), ST-6 (picker never
runs), ST-7 (trace 21), ST-12 (pointer-arrow failure), ST-13 (loop double free); task arithmetic
34 = 11+8+8+7; AR/ST references resolve; the host-ASan command is executable as written.

## Clean Dimensions and Verified Claims

- **Dimension 5 (Dependencies): clean.** Helper signatures exist as assumed; no task references a
  not-yet-created entity; no circularity; the stated Phase 2 → Phase 1 link is conservative.
- **Dimension 8 (Security): clean.** No new input/network/persistence surface; the test harness
  spawns processes with argv arrays, never a shell; scratch files are `temp_dir()` + `Drop`.
- **Dimension 10 (Scope creep): clean.** Every change maps to R1–R8 or an approved AR item
  (AR-3/4/5/6/7/8/9/10); no unapproved machinery; task atomicity sound.
- **Complexity Escalation Gate:** not triggered by the plan as written; would trigger only for
  PF-001 option B (concrete-limit change).
- Verified claim groups: slice path, consume path, root/receiver path, drop path, caller
  enumeration, defect provenance, migration/compatibility (no corpus multi-extent slices),
  convention compliance, task arithmetic (33 = 10+8+8+7), spec-first ordering, traceability
  (every defect → requirement → ST → phase task).

## Adversarial Checklist Notes (same-session bias)

- "What did I assume during creation?" — that the concrete spellings were accepted and that the
  named-generic `E10200` probe grounded the intrinsic mechanism. Both were wrong (PF-001, PF-002).
- "What external standard might be violated?" — the frozen specification's array rules were
  re-read (`spec/08:28` supports row typing) and AGENTS.md's review matrix (PF-008).
- "What would a domain expert flag?" — the ambiguous `view_element` contract (PF-013) and the
  ownership-boundary wording (PF-001).
