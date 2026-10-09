# Preflight Report: Borrow/Generic Integrity Implementation Plan

> **Status**: ✅ PREFLIGHT PASSED — all 17 findings resolved (1 🔴, 6 🟠, 9 🟡, 1 🔵)
> **Iteration**: 2 (fixes applied and verified — see §Iteration 2)
> **Artifact**: implementation plan at `plans/borrow-generic-integrity/` — 10 documents, HEAD `823e008`, tree `4e93c84a17c8b26d76190c395f23b23eea08447d`
> **Codebase Grounded**: ~20 compiler/test files examined; every file:line reference in the plan verified; 13 execution probes run with `target/debug/clc`
> **Last Updated**: 2026-10-09 21:01
>
> ⚠️ **SAME-SESSION REVIEW**: this artifact was created in the current session. Same-agent
> bias is elevated. Countermeasures: the 13-dimension scan ran as five independent auditor
> clusters, every contested claim was re-verified with execution probes, and one independent
> design challenger reviewed the CRITICAL/MAJOR batch blind to the lead's picks.

## Codebase Context Summary

**Tech Stack:** Rust (std-only workspace), C99 emission, `target/debug/clc` host compiler + `cc`/ASan probes.
**Architecture:** front end (lex/parse/load/resolve/sema) → ownership/lowering (`own/`: bindings, duties, instance audits, narrowing) → IR (`ir/`, `ir/mono.rs` duty records) → codegen (`codegen/`: signatures, places, calls, guards). The frozen spec (`spec/`) is the oracle; the standard library is embedded.
**Key files verified:** `own.rs` (param declaration, `unique_ir_name`, places), `own/expr.rs` (`check_assign`), `own/place.rs` (`move_place`, `mark_moved_if_owning`), `own/instance_check.rs` (duty audits), `own/call_check.rs` (`W10001`), `own/call.rs` (`lower_call`, inference), `own/lower.rs` (`value_may_lend`, `lowered_lend_source`, `take_move_out_places`), `codegen.rs`/`codegen/{types,function,expr,program}.rs`, `types/keys.rs`, `types/table.rs`, `sema/ops.rs`, `sema/call.rs`.
**Probes:** `p_a` (inferred `raw u8?` warning), `p_b` (nested nullable function instance → duplicate `cl_pick__Wrap__String` + `struct ClWrap__String` redefinition), `p_c` (shadowed written call), `p_d` (nullable-slice indexing → `E10010`), `p_f` (local element self-store → `E10026`), `p_f2` (non-nullable lend element self-store → `E10034`), `p_g` (`str == str` accepted), `p_h` (`String?[]` → `E20000`), `p_j1`/`p_j2` (concrete nullable store accepted; generic at `String?` rejected `E10034`).

### Summary by Dimension

| # | Dimension | Findings | Highest |
|---|-----------|----------|---------|
| 1 | Ambiguities | 2 (PF-012, PF-017) | 🟡 |
| 2 | Implicit Assumptions | — (folded into 13) | — |
| 3 | Logical Contradictions | 2 (PF-001, PF-005) | 🔴 |
| 4 | Completeness Gaps | 2 (PF-006, PF-014) | 🟠 |
| 5 | Dependency Issues | 1 (PF-013) | 🟡 |
| 6 | Feasibility Concerns | 1 (PF-009) | 🟡 |
| 7 | Testability | 3 (PF-008, PF-009, PF-015) | 🟡 |
| 8 | Security Blind Spots | — (memory-safety covered by PF-005/PF-007) | — |
| 9 | Edge Cases | 3 (PF-002, PF-005, PF-006) | 🟠 |
| 10 | Scope Creep Indicators | — (checked clean) | — |
| 11 | Ordering & Sequencing | 2 (PF-004, PF-007) | 🟠 |
| 12 | Consistency | 4 (PF-004, PF-010, PF-011, PF-012) | 🟠 |
| 13 | Codebase Alignment | 6 (PF-001, PF-002, PF-003, PF-007, PF-013, PF-016) | 🔴 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| 🔴 CRITICAL | 1 | resolved (user ruling A) |
| 🟠 MAJOR | 6 | resolved (PF-003/PF-005 user rulings; PF-002/004/006/007 delegated) |
| 🟡 MINOR | 9 | resolved (delegated) |
| 🔵 OBSERVATION | 1 | resolved (delegated) |

All fixes were applied to the plan documents under the user's instruction and verified in
iteration 2 (see §Iteration 2).

---

## Findings

### PF-001: DEF-77 (P4) — the confirmed mask approach, as planned, emits invalid C [🔴 CRITICAL]

**Dimension:** 3 (Logical Contradictions) / 13 (Codebase Alignment)
**Location:** `03-04-may-lend-callee-identity.md` §Proposed Changes/§New Types; `99-execution-plan.md` task 4.2.1; `00-index.md` Related Files
**Codebase Evidence:** `own.rs:324-332` builds `ir::Param.name` from the AST source name; `own.rs:842-850` masks only `BindingKind::Local`; `own.rs:898` builds places from `binding.ir_name`; `codegen.rs:540-548` emits the signature from source names; `codegen/function.rs:94-98` keys `local_names` by the IR param name; `codegen/expr.rs:581-586` emits the raw callee on lookup miss; `codegen/expr.rs:614-635` matches func bindings by name. A masked parameter therefore emits undeclared identifiers or raw `give#0(...)` — the plan's own ST-17..ST-20 shapes would not build. Two existing impl tests pin the current unmasked spelling for colliding params (`runtime_value_integrity_impl_test.rs:626-643`, `call_value_integrity_impl_test.rs:1354-1372`). The plan says "one condition changes" and lists only `compiler/src/own.rs`; it also claims P1–P4 are independent in code (option A overlaps P2's `codegen/expr.rs`).
**The Problem:** the artifact's confirmed mechanism (AR-3) is under-specified; as written, the P4 execution would fail its own tests/builds. Additional name-resolving consumers beyond the two the plan lists: `take_move_out_places` (`own/lower.rs:99-103`) and `materialized_ty` (`own/lower.rs:649-672`).

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Complete the plumbing under AR-3: carry the masked spelling into `ir::Param`, the C signature, `local_names`, pointer-param tables and closure/func lookups; update the two pinned tests; add the emission-integrity reviewer to P4 | One uniform naming rule (locals already masked); closes **all** name-based consumers by construction; consistent with the repo's existing pattern | Multi-file change (own.rs, codegen.rs, codegen/function.rs, codegen/expr.rs); emitted C changes for colliding params (emission sweep + lens); sanitize-collision edge (`give#0`→`give_0` vs a param named `give_0`) must be handled; two pinned tests updated |
| B | Carry callee-binding identity on the IR value: record on `LoweredCall`/`ValueKind::Call` that the callee was a binding (`lower_call` already knows), and gate every name-based consumer (`value_may_lend`, `collect_lowered_temp_owners`, `take_move_out_places`, `materialized_ty`) with that fact | No emitted-name change; no existing test updates; compiler-enforced exhaustiveness (~9 patterns + 2 constructors); stays inside `ir/own` | Reopens the user-confirmed AR-3 (mechanism change); adds an IR field; every name-based consumer must consult it (or the class re-opens) |

**Recommendation:** **Option A** — after reconciliation with the independent challenger, the uniform-naming invariant (finish the mask that locals already use) closes the whole string-resolution class by construction, including consumers the plan never enumerated; option B remains a legitimate, smaller-blast-radius alternative if reopening AR-3 is preferred. *(Challenger: diverged from the lead's initial lean — challenger chose A, lead initially favored B; reconciled to A on the by-construction argument, presenting both.)*
**User Decision:** Resolved — User chose Option A (complete the plumbing; AR-3 mechanism unchanged). Applied in 03-04, 99 (tasks 4.2.1/4.2.2/4.3.1, lenses, dependencies), 00-index, 02, AR-3.

---

### PF-002: DEF-83 — the W10001 exemption misses inferred nullable substitutions [🟠 MAJOR]

**Dimension:** 13 / 9 (Edge Cases)
**Location:** `03-01-borrow-store-rules.md` §DEF-83; `07-testing-strategy.md` ST-8
**Codebase Evidence:** `own/call_check.rs:78-91` substitutes only explicit type arguments (`zip(type_args)`); the warning arm at `:353-383` uses the argument type and never the parameter; **probe p_a_inferred**: `setIt(p, borrow r)` with `raw u8?` still warns `W10001`; explicit `<raw u8?>` warns too (probe p_a2). AR-9 promises every nullable borrow parameter is exempt.
**The Problem:** the fix as designed consults the bare `T` for inferred calls, so the filed defect recurs for the inferred spelling while ST-8 (concrete only) stays green.
**Options:** (A) substitute the effective parameter type at the warning — reuse the lowering's inference (`infer_call_types`, `own/call.rs:186-198`), ideally computed once in `check_call_expr` and passed down, then warn iff duplicable ∧ ¬view ∧ ¬nullable; add an inferred-instance ST. (B) defer the warning per instance as a duty (new deferred-warning mechanism).
**Recommendation:** Option A (challenger-converged; DRY, guarantees check/emission agreement).
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-003: DEF-79 — nested nullable arguments DO change function-instance symbols (R7 is false) [🟠 MAJOR]

**Dimension:** 13 (Codebase Alignment)
**Location:** `01-requirements.md` R7/Technical Requirements; `03-02-instance-keys.md` §Proposed/§Error Handling/§Testing; `00-ambiguity-register.md` AR-6; `07-testing-strategy.md` ST-10/11
**Codebase Evidence:** `codegen/types.rs:53-59` appends `Q` only for the argument's own flag; `types/keys.rs:39-45` drops nested nullability; **probe p_b_nested**: `pick<Wrap<String>>` and `pick<Wrap<String?>>` both emit `cl_pick__Wrap__String` plus duplicate `struct ClWrap__String` (strict `cc`: redefinition). Under the fold, `pick<Wrap<String?>>` becomes `cl_pick__Wrap__StringQ`. AR-6/03-02 say "three call sites"; there are two (`codegen.rs:627`, `codegen/expr.rs:569`).
**The Problem:** R7 ("function-instance symbol spellings unchanged"), the sweep expectation ("no function-instance symbol changes"), and 03-02's "identical output to today" are unsatisfiable together with R6; the change is exactly the collision the fix must repair.
**Options:** (A) narrow R7 to "top-level nullable arguments stay byte-identical; nested nullable arguments gain `Q` (an expected fixing diff)"; add a nested function-instance pin; correct the call-site count. (B) preserve nested spellings — not viable (duplicate symbol).
**Recommendation:** Option A (challenger-converged).
**User Decision:** Resolved — User confirmed the narrowed R7. Applied in 01 (R7, compatibility), 03-02, 07 (ST-10/ST-11), AR-4/AR-6.

---

### PF-004: One report per assignment site — the dedupe mechanism is under-specified and mis-cited [🟠 MAJOR]

**Dimension:** 13 / 12 / 11
**Location:** `00-ambiguity-register.md` AR-13; `02-current-state.md` risk row; `03-01-borrow-store-rules.md` §"One report per assignment site"; `07-testing-strategy.md` ST-1–ST-4; `99-execution-plan.md` task 1.1.2
**Codebase Evidence:** concrete judges: `move_place` (`own/place.rs:204-215`) reached via `bind_value`/`move_target` (`own/statements.rs:506-520`), and the not-lend duty (`place.rs:299-330` + `instance_check.rs:118-153`); `Diagnostics::dedupe_from` (`diag.rs:131-150`) cannot merge the differing messages ('v' vs 'slot'). ST-3 (concrete both-lend) **already reports exactly one E10034 at baseline** (probes by three clusters) — it is a regression pin, not a red case, so task 1.1.2's expected list is wrong; ST-1/2/4/5 have no competing judge, so the dedupe is un-pinned; no ST covers the generic both-borrow spelling (`copy<T>(borrow v: T, borrow slot: T)` at `String`), where both judgments are deferred duties.
**Options:** (A) recording-time skip in `check_assign`: skip the target-side judgment when the RHS move sources are lend-place moves already judged by the existing path, **mirroring `move_place`'s exact reporting set** (Param ∧ mode ∈ {None, Const, Borrow}; a parameter-typed source already owing a `RequiresNotLend` duty at the same span); extend ST-3 with the generic both-borrow case; fix the red-phase list and the AR-13/risk-row citations. (B) audit-time shared span-keyed reported set across the two audits **plus** a recorded-span guard for the concrete direct report.
**Recommendation:** Option A (challenger-converged; the recording-time skip is the plan's own stated shape and needs less machinery).
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-005: DEF-82 — the "projected self-assign is unreachable" premise is false [🟠 MAJOR]

**Dimension:** 3 / 9
**Location:** `00-ambiguity-register.md` AR-8; `03-01-borrow-store-rules.md` §DEF-82
**Codebase Evidence:** nullable-slice indexing requires narrowing (`E10010`, probe p_d); **P3 in this same batch** makes `if (xs != null) { xs[0] = xs[0]; }` expressible for `T[]?`; concrete owning elements under out-slot roots are stopped by the `E10026` element-move limit (`place.rs:240-248`, probe p_f); non-nullable lend roots by the not-lend duty (probe p_f2: `E10034`); the substituted maybe-owning element under an out-slot root records no duty (`mark_moved_if_owning` skips `BindingKind::OutSlot`, `place.rs:299-330`) and the emitted store is the DEF-82 class (release target, then store source). Baseline execution of the exact shape is blocked by DEF-81, so destructiveness is unverified by execution but consistent across two independent code readings.
**Options:** (A) correct the premise; keep whole-binding coverage; add an execution probe for the narrowed `T[]?` self-assign in P3 plus a decision point. (B) extend the DEF-82 rule to exact projected self-assign on out-slot roots (same-place comparison including projections; `E10020`); note the runtime-indexed `xs[i] = xs[i]` variant remains unguarded. (C) add a per-instance duty for maybe-owning projected moves out of out-slot roots (complete guard; a larger behavioral change).
**Recommendation:** Option B — the batch itself unlocks the shape, the statement is never legitimate, and the same-place comparison already exists for whole bindings (challenger pre-framed B as the likely answer; the smallest safe change).
**User Decision:** Resolved — User chose Option B (extend to exact projected self-assign). Applied in 03-01 (DEF-82), 01 (R4, Won't Have residual), 07 (ST-7), AR-8/AR-13, 99 (tasks 1.2.3 and close-out 4.3.4).

---

### PF-006: The not-lend audit rejects legal nullable substitutions [🟠 MAJOR]

**Dimension:** 4 / 9
**Location:** `01-requirements.md` R3; `03-01-borrow-store-rules.md` §DEF-78 exemptions
**Codebase Evidence:** `String?` is non-duplicable (`types/table.rs:399-429`, Handle → false); `check_not_lend_requirements` has no nullable carve-out (`instance_check.rs:118-153`); **probe p_j1**: concrete `copy(borrow v: String?, borrow slot: String?)` accepted; **p_j2**: generic `copy<T>(borrow v: T, borrow slot: T)` at `T=String?` rejected `E10034` ('v') — while the plan's new storable audit would exempt nullable substitutions, so the generic spelling stays falsely rejected. AR-1 explicitly confirmed "DEF-78 covers the concrete and generic spellings; nullable out-slots and duplicable lends stay legal".
**Options:** (A) extend the not-lend audit's exemption to nullable substitutions, mirroring the out-slot carve-out the concrete paths apply; add an ST. (B) keep the rejection; narrow R3/AR-2 and file a separate defect.
**Recommendation:** Option A (challenger-converged; required for the confirmed generic nullable acceptance to be true).
**Resolution:** AI — delegated by --auto-design (within the confirmed AR-1 sentence on generic spellings; see Delegated Resolutions).

---

### PF-007: The ASan oracle is not executable as specified, and its scope conflicts across documents [🟠 MAJOR]

**Dimension:** 11 / 13
**Location:** `07-testing-strategy.md` §Testing Overview + ST-17; `99-execution-plan.md` verify lines (P1/P4) and task 4.2.2; `01-requirements.md` AC4; `00-ambiguity-register.md` AR-12
**Codebase Evidence:** memory cases are `RDC_ASAN`-gated (`def_backlog_closeout_spec_test.rs:233-245`; `assert_no_asan` is vacuous without it); grep finds no `RDC_ASAN` in `Makefile`/`docker/`; ST-17's defect is leak-only (a plain run exits 0). AR-12/AC4 say P1 only while success criterion 5/task 4.2.2 add P4.
**Options:** (A) name the command in 07 + the P1/P4 verify lines — `make -C docker exec CMD="RDC_ASAN=1 cargo test --test borrow_generic_integrity_spec_test"` (env inside the container command) or the explicit host probe; note the known container flakiness (run probes individually); unify the P1+P4 scope across AR-12/AC4/SC5. (B) leave the generic wording (red phase unreproducible).
**Recommendation:** Option A (challenger-converged).
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-008: ST-20 is under-specified and its natural reading proves nothing [🟡 MINOR]

**Dimension:** 7
**Location:** `07-testing-strategy.md` ST-20; `99-execution-plan.md` task 4.1.2
**Codebase Evidence:** sema skips the free-function path when the name is a lexical binding (`sema/call.rs:56-58`), yet codegen still renders the module symbol for a direct call (probe p_c: `cl_t_demo(i32 give)` body emits `cl_t_give(s)`); wrapping the call in `len(...)` silently discards it (`codegen/calls.rs` `_` arm). The shadowing behavior is pre-existing and inconsistent; ST-20's "reaches the module function" claim is only demonstrable with a **direct** call form.
**Options:** only one — rewrite ST-20 with an explicit direct-call shape and an observable (return value / emitted symbol); note the `len(...)`-wrapped form is degenerate.
**Recommendation:** rewrite as described.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-009: ST-11 specifies an unwritable type (`String?[]`) and an impossible slice-keys claim [🟡 MINOR]

**Dimension:** 7 / 6
**Location:** `07-testing-strategy.md` ST-11; `03-02-instance-keys.md` §Integration Points/§Testing
**Codebase Evidence:** grammar `spec/grammar.ebnf.md:190-197` — `array_element_type` admits no `?`; **probe p_h**: `String?[]` → `E20000`. Slice keys are built from `view_element(ty)` (`codegen/types.rs:71,101,337-352`), never a nullable element.
**Options:** only one — respell the third ST-11 witness as a keyed instance carrying a nullable-slice argument (e.g. `Wrap<String[]?>` vs `Wrap<String[]>`); delete the "slice keys gain Q when the element is nullable" claim from 03-02.
**Recommendation:** as described.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-010: "view == view stays E10222" is false for `str`/`static str`/`cstring` [🟡 MINOR]

**Dimension:** 12
**Location:** `01-requirements.md` R8; `00-ambiguity-register.md` AR-7; `03-03-nullable-view-comparison.md` §Proposed/§Error Handling
**Codebase Evidence:** `sema/ops.rs:443-455` accepts `Str`/`CString`/`StaticStr`; **probe p_g**: `str == str` compiles today; only slices are rejected.
**Options:** only one — reword to "slice-view `==` slice-view stays `E10222`"; optionally note the pre-existing non-slice equality acceptance as an out-of-scope deviation.
**Recommendation:** as described.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-011: Stale cross-references across 02/03-04/AR-13 [🟡 MINOR]

**Dimension:** 12
**Location:** `02-current-state.md` risks/dependencies; `03-04-may-lend-callee-identity.md` §Testing Requirements; `00-ambiguity-register.md` AR-13
**Codebase Evidence:** `02` cites the dedupe as pinned by "ST-1/ST-2" (actually ST-3 + the new generic case), "ST-11" for re-emission (ST-12), "ST-19 … ST-16..18" for the DEF-77 cases (ST-20, ST-17..19), "ST-13 pins the run behavior" (ST-14), and attributes DEF-83 to `own/expr.rs`'s assignment path (it is `own/call_check.rs:353-383`); AR-13 cites "ST-1/ST-4". `03-04` defines ST-18 as the "existing local-collision case" while `07` defines ST-18 as the func-typed-field receiver case.
**Options:** only one — sweep and correct all citations; make ST-18's description identical wherever it appears.
**Recommendation:** as described.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-012: Wording corrections and the recorded `Q`-marker limitation [🟡 MINOR]

**Dimension:** 1 / 12
**Location:** `01-requirements.md` (R2, overview), `00-ambiguity-register.md` AR-8, `00-index.md` (baseline, key decisions), `03-02-instance-keys.md`
**Codebase Evidence:** (a) R2 says "for every owning instantiation" while the audit reports once at the first offending instance (`instance_check.rs:118-153`); (b) the overview says "(DEF-78 – DEF-83) plus DEF-77", double-counting and including excluded DEF-80; (c) AR-8/02 claim a local `s = s;` is rejected — probe: it compiles; `E10020` fires at the later **use**; (d) the minimum-sufficient baseline says "every change lands in an existing function or an existing audit" while the plan adds `RequiresStorableSlot` + its audit (approved under AR-5) — reword to name the approved addition; (e) the `Q` marker collides with user identifiers ending in `Q` (`Wrap<StringQ>` vs `Wrap<String?>` → both `Wrap__StringQ` after the fold; pre-existing on the function path) — record the limitation and add an impl-test note (AR-4 pinned the `Q` spelling).
**Recommendation:** apply all five wording corrections.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-013: P1 file lists understate the change and list an untouched file [🟡 MINOR]

**Dimension:** 5 / 13
**Location:** `99-execution-plan.md` tasks 1.2.1/1.2.2; `00-index.md` Related Files; `02-current-state.md` file table
**Codebase Evidence:** a new duty record requires the `ir.rs` re-export (`compiler/src/ir.rs:44-47`) and own.rs plumbing (checker fields `own.rs:281-293`, constructor `:362-375`, `check` return tuple `:383-391`, collection loop `:117-137`); task 1.2.1 names only `ir/mono.rs` + `own/expr.rs`; `own/place.rs` is listed as Modified but no task/design touches it (its only role is the conditional shared helper).
**Recommendation:** add `compiler/src/ir.rs` + the own.rs plumbing to the tasks; annotate or drop `place.rs`.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-014: AR-1's close-out commitment has no task (plus the AGENTS.md test-map sync) [🟡 MINOR]

**Dimension:** 4
**Location:** `00-ambiguity-register.md` AR-1; `99-execution-plan.md` §Success Criteria / Phase 4 tasks
**Codebase Evidence:** `plans/00-roadmap.md:97-98` still carries the inaccurate DEF-78 ("the concrete spelling is rejected") and DEF-79 ("field C types differ") notes; the two new suites are absent from the AGENTS.md test-naming map; precedent: `plans/defer-integrity/99-execution-plan.md` carries an explicit close-out task.
**Recommendation:** add a close-out task (e.g. 4.3.4) covering the row-note corrections, the AGENTS.md test-map entries, and the roadmap sync.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-015: Implementation-test scope wording claims unobservable pins [🟡 MINOR]

**Dimension:** 7 / 10
**Location:** `99-execution-plan.md` tasks 1.3.1/3.3.1; `07-testing-strategy.md` impl row; `01-requirements.md` R10
**Codebase Evidence:** the CLI has no IR dump (`compiler/src/driver.rs` — emitted C + stderr only), so "duty dedupe" is only observable as a diagnostic count (duplicating ST-1–ST-3) unless stated as a loop-body store case; `3.3.1`'s "E10222 for view-versus-view" duplicates ST-15; R10 promises an impl test "for each fix" while DEF-82/DEF-83 have none named.
**Recommendation:** restate 1.3.1's dedupe check as a loop-body store with exactly one `E10034`; drop the duplicated count/diagnostic items; soften R10's wording.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-016: The specialist-check evidence predates this plan [🟡 MINOR]

**Dimension:** 13
**Location:** `00-index.md` §Specialist Agents
**Codebase Evidence:** `codeops/specialist-check.json` (`checkedAt` 2026-10-09T19:41:29+02:00) lists 23 plans and does not include `borrow-generic-integrity` (created 20:23); AGENTS.md: "re-run `analyze-agents` after new plans land".
**Recommendation:** re-run the check now the plan exists; update the evidence (the outcome is expected `None`).
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

### PF-017: The 12-category scan does not account for AR-11 [🔵 OBSERVATION]

**Dimension:** 1
**Location:** `00-ambiguity-register.md` §Systematic 12-category scan
**Codebase Evidence:** every other resolved item has a category outcome; AR-11 (sequencing) appears in none.
**Recommendation:** add a sequencing row/note.
**Resolution:** AI — delegated by --auto-design (see Delegated Resolutions).

---

## Delegated Resolutions (auto-design, policy v1)

Root invocation: `ad-borrow-generic-integrity-20261009`. Eligibility: each resolution stays
within the plan's confirmed goals, the AR register decisions it cites, and the confirmed scope;
none introduces new language surface, new diagnostics, or new support machinery beyond the
already-approved `RequiresStorableSlot` addition; `spec/` untouched.

| Finding | Decision (selected option) | Key evidence | Rejected alternatives | Confidence | Reopen triggers |
|---------|----------------------------|--------------|-----------------------|------------|-----------------|
| PF-002 | Substitute the effective parameter type at the W10001 decision (reuse the lowering's inference; pass bindings down from the call check); add an inferred-instance ST | probes p_a/p_a2; `call_check.rs:78-91,353-383`; `own/call.rs:186-198` | (B) deferred warning duty — new mechanism, warning semantics unspecified | High | check/lowering disagreement on inferred types |
| PF-004 | Recording-time skip mirroring `move_place`'s exact set + generic both-borrow ST + red-list/citation fixes | probes (ST-3 one report; generic one report); `diag.rs:131-150` cannot merge messages | (B) shared span set + recorded-span guard — more machinery | Med-High | conditional/mixed RHS shapes escaping the skip |
| PF-006 | Extend the not-lend audit's exemption to nullable substitutions; add an ST | probes p_j1/p_j2; `instance_check.rs:118-153`; AR-1 "covers the concrete and generic spellings; nullable out-slots … stay legal" | (B) keep rejection + narrow R3 — contradicts the confirmed generic coverage | High | the exemption accepting a shape the concrete spelling rejects |
| PF-007 | Name the RDC_ASAN command (env inside the container command) in 07 + P1/P4 verify lines; note flake policy; unify P1+P4 in AR-12/AC4/SC5 | `def_backlog_closeout_spec_test.rs:233-245`; no RDC_ASAN in build scripts; challenger-verified `make -C docker exec` env placement | (B) generic wording — unreproducible evidence | High | none observable |
| PF-008 | Rewrite ST-20 with a direct-call observable | probe p_c; `sema/call.rs:56-58` | keep as written — proves nothing | High | — |
| PF-009 | Respell ST-11's slice witness as a keyed instance with a nullable-slice argument; drop the slice-keys claim | probe p_h; grammar; `codegen/types.rs:71,101` | add element nullability — out of scope | High | — |
| PF-010 | Reword to "slice-view `==` slice-view"; note the non-slice deviation | probe p_g; `sema/ops.rs:443-455` | reject `str == str` — separate behavior ruling | High | — |
| PF-011 | Sweep stale citations; make ST-18 identical in 07/03-04 | `02-current-state.md`; `03-04` §Testing | — | High | — |
| PF-012 | Five wording corrections (R2; 01 overview; AR-8 local precedent; baseline addition note; record the `Q` limitation) | probes p_b/def82b; `own.rs:763-787`; AR-4/AR-5 | change the `Q` marker — contradicts AR-4 | High | — |
| PF-013 | Add `compiler/src/ir.rs` + own.rs plumbing to P1 tasks; annotate/drop `place.rs` | `ir.rs:44-47`; `own.rs:281-293,362-391` | define the record outside `ir` — contradicts AR-5 | High | — |
| PF-014 | Add close-out task 4.3.4 (row notes + AGENTS.md test map + roadmap sync) | `plans/00-roadmap.md:97-98`; AGENTS.md map | rely on the exec skill's generic close-out — it does not cover notes | High | — |
| PF-015 | Restate impl-test pins as observable (loop-body store; drop duplicates); soften R10 | `compiler/src/driver.rs` (no IR dump) | — | High | — |
| PF-016 | Re-run `analyze-agents`; update the evidence | `codeops/specialist-check.json` | — | High | — |
| PF-017 | Add the sequencing note | register §scan | — | High | — |

**Reserved decisions (user):** PF-001 → Option A; PF-003 → narrowed R7 confirmed; PF-005 →
Option B. All recorded and applied. All delegated resolutions in the table were likewise
applied in iteration 2.

## Iteration 2 — Fix Verification

Applied and verified (bounded re-scan; the artifact changed only inside the plan folder plus
`codeops/specialist-check.json`):

| Finding | Resolution applied | Where |
|---------|--------------------|-------|
| PF-001 | Option A: mask + parameter-name plumbing; emission lens; independence note | 03-04, 99, 00-index, 02, AR-3 |
| PF-002 | Effective (explicit or inferred) parameter type at the warning; inferred ST case | 03-01, 07 ST-8, 99 task 1.2.4 |
| PF-003 | R7 narrowed to top-level; nested `Q` documented; call-site count corrected | 01, 03-02, 07, AR-4/AR-6 |
| PF-004 | Recording-time skip; ST-3 generic clause; red-phase list fixed | 03-01, 07 ST-3, 99 task 1.1.2, AR-13 |
| PF-005 | Option B: exact projected self-assign; premise corrected; residual filed at close-out | 03-01, 01, 07 ST-7, AR-8/AR-13, 99 |
| PF-006 | Not-lend audit nullable exemption; ST-6 generic-nullable clause | 03-01, 07 ST-6, 99 task 1.2.2 |
| PF-007 | `RDC_ASAN` commands; P1+P4 scope unified; flake note | 07, 99 (1.2.5, 4.1.2, 4.2.2), AR-12, 01 AC4 |
| PF-008–PF-017 | Document corrections as recorded above | 07, 02, 03-02, 03-03, 03-04, 99, register, 00-index, specialist-check state |

Verification evidence:

- Stale-phrase sweep over `plans/borrow-generic-integrity/` is clean (no "three call sites",
  "one condition changes", old red-phase lists, "view == view", stale ST citations, or 0/29
  counters).
- Task accounting: 30 task checkboxes, `0/30` progress, P4 = 8 tasks, total 30 — consistent.
- Cross-references (AR-1–AR-13, ST-1–ST-20, R1–R10, DEF ids, 03-xx links) re-checked after the
  edits; the four red-phase lists match the updated ST-table notes.
- `analyze-agents` re-ran (2026-10-09 21:09) with `None`, now including this plan
  (`codeops/specialist-check.json`).
- No new issues were introduced by the fixes in the bounded scan.

## Verdict

**✅ PREFLIGHT PASSED — all 17 findings resolved.** All rulings and delegated resolutions were
applied to the plan documents and verified by the iteration-2 bounded re-scan. The roadmap rows
advance to `Plan Preflighted`; the plan may enter `exec-plan`.
