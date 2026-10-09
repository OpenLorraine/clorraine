# Ambiguity Register: Borrow-Store Integrity

> **Status**: ✅ GATE PASSED — all 14 items resolved
> **Last Updated**: 2026-10-10 00:34

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| AR-1 | Scope | Which defects this plan fixes | Store-path batch (DEF-100/101/102/105/106) / write path + capture memory / closure family | **User:** chose the store-path batch — DEF-100, DEF-101, DEF-102, DEF-105, DEF-106. | ✅ Resolved |
| AR-2 | Technical | DEF-100: how the parameter-typed lend self-assign is judged per instance | Extend `RequiresStorableSlot` with a kind flag / a new `RequiresSelfAssign` duty record | **AI — delegated by --auto-design (eligible: compiler mechanism).** Record the self-assign site at the lend-store site and judge it in the existing per-instance audit beside `check_storable_slot_requirements`, reusing the established `ir::Requires*` duty pattern. See notes. | ✅ Resolved (delegated) |
| AR-3 | Behavioral | DEF-101: the concrete `borrow T?` direct return — move out or reject | Move out (empty the caller's slot, like the accepted `let local = v` store) / reject `E10034` (like the generic instance audit) | **User:** chose move out — the callee transfers the slot's value to its caller and empties the slot (`__cl_ret = (*v); (*v) = NULL;`), matching the accepted `let local = v` store path; the generic instance audit stays unchanged. | ✅ Resolved |
| AR-4 | Behavioral | DEF-102: the runtime-indexed element self-assign on a nullable out-slot | Keep the conservative guard (any same-root runtime pair rejected; pin + document) / refine to index-expression identity (accept `xs[i] = xs[j]`) | **User:** chose the conservative guard — any same-root runtime-indexed element pair stays rejected (`E10020`); the plan pins it with spec tests and documents the conservative comparison. | ✅ Resolved |
| AR-5 | Technical | DEF-105: where and how the element-store null check lands | Mirror the read path (the store target's element base must be non-null-narrowed, `E10010`) / a runtime trap | **AI — delegated by --auto-design (eligible: compiler mechanism).** Mirror the read path: the element-store base must be non-null-narrowed, reported with the existing `E10010` message; narrowed bases stay accepted. See notes. | ✅ Resolved (delegated) |
| AR-6 | Technical | DEF-106: how the constant-index element store gains its bounds trap | Reuse the read path's `E11211` trap statement before the store / route stores through `StorePointer` | **AI — delegated by --auto-design (eligible: compiler mechanism).** Emit the same `E11211` trap statement the read path emits, before the store, for constant-index element targets with a dynamic bound; fixed-array in-range constants keep the semantic-layer check and stay unchanged. See notes. | ✅ Resolved (delegated) |
| AR-7 | Behavioral | DEF-100: the diagnostic per substitution | Owning non-nullable → `E10034` (as the concrete spelling) · nullable → `E10020` (the concrete out-slot self-assign) · duplicable/view/unknown → accepted | **AI — delegated by --auto-design (eligible: conformance to the confirmed concrete rules).** Each substitution reproduces exactly what the concrete spelling of the same store reports today. See notes. | ✅ Resolved (delegated) |
| AR-8 | Scope | DEF-101 projected forms (`return xs[0]`, `return xs[i]` through a slot) | Include in the fix / leave to the existing rules | **AI — delegated (eligible: verified current behavior).** Both are already rejected `E10026` (the v1 element-move limit); the fix covers the direct whole-slot return and pins the projected rejections. See notes. | ✅ Resolved (delegated) |
| AR-9 | Scope | Non-nullable `borrow` view element self-assign (`xs[i] = xs[i]` on `borrow xs: i32[]`) | Include / leave unchanged | **AI — delegated (eligible: verified current behavior).** Accepted today and harmless for the out-slot rule's scope; the self-assign carve-out applies to out-slot (nullable-borrow) roots only. Out of scope; a guard test keeps it accepted. See notes. | ✅ Resolved (delegated) |
| AR-10 | Scope / process | Whether `spec/` may change | Keep `spec/` byte-identical (project rule) / add clarifications | **Project rule + user precedent (borrow-generic-integrity AR-10).** `spec/` stays byte-identical; every fix conforms the implementation to the frozen rules. | ✅ Resolved |
| AR-11 | Technical | Verify commands and test layout | `make check` per phase, ASan probes for the memory phases, `make verify` final; suites `borrow_store_integrity_{spec,impl}_test.rs` | **User:** confirmed the verify convention (make check per phase + make verify final). Test-file names follow the repo convention; internal. | ✅ Resolved |
| AR-12 | Sequencing | Phase structure for the execution plan | 3 phases by surface (self-assign rules; out-slot return; view-store guards) / one phase per defect | **AI — delegated by --auto-design (eligible: implementation sequencing).** P1 = DEF-100 + DEF-102 (the self-assign rule family); P2 = DEF-101 (the out-slot return); P3 = DEF-105 + DEF-106 (the view-store guards). See notes. | ✅ Resolved (delegated) |
| AR-13 | Process | Phase-review dispatch for this change surface | correctness + memory-safety auditors (ownership/store changes) and the emission-integrity auditor (P2/P3 codegen/emission changes) | **AI — delegated (eligible: process routing per AGENTS.md).** Dispatch per the repository's change-surface matrix. See notes. | ✅ Resolved (delegated) |
| AR-14 | Naming | Plan folder and any new internal names | `borrow-store-integrity`; duty/helpers named per the `Requires*`/`emit_*` patterns | **AI — delegated (eligible: internal naming, zero external surface).** Folder `plans/borrow-store-integrity/`; new internal entities follow the existing naming patterns. See notes. | ✅ Resolved (delegated) |

## Resolution Notes

**AR-2 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler mechanism; stays within the confirmed DEF-100 behavior (mirror the concrete spelling); no reserved consequence
Objective: judge the parameter-typed lend self-assign after substitution, with one report per site
Decision: record the site through the established per-instance duty pattern so the audit judges the substituted type; mechanism detail (flag on an existing record vs a small new record) is owned by 03-01
Evidence: the concrete `borrow String` spelling reports `E10034` at the store site (probe p100b); the concrete `borrow String?` spelling reports `E10020` (pinned by borrow-generic-integrity); `borrow i32` stays accepted (probes p100c/p100d); the generic instance at String is accepted today and emitted `cl_copySelf__String(ClString slot)` whose body frees the caller's handle (`FreeString(slot)`) then self-assigns it — ASan use-after-free and `free(): double free` (probe p100a)
Rejected alternatives: a declaration-site rejection (would reject the duplicable instances the concrete spelling accepts)
Strongest counterargument: a duty adds a record; the concrete path already reports directly, so the generic could instead be pushed through the same declaration-site rule — rejected because the type is still a parameter there and the judgment must wait for the instance
Confidence: High — the concrete spelling is pinned and the substitution machinery is unchanged
Hardening: in-context layers only (medium stakes: conformance fix; no independent challenger required)
Policy version: 1
Root invocation ID: plans-borrow-store-integrity-20261010
Reopen triggers: a concrete spelling that behaves differently from the mapped substitution; a spec change to OW-16/D9
```

**AR-5 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler mechanism; conforms to the existing read-path rule and the T8/D9 null rules; no reserved consequence
Objective: make a write through a possibly-null view obey the same non-null requirement as a read
Decision: the element-store target's base must be non-null-narrowed; the existing `E10010` ("value '…' may be null here") is reported at the store, mirroring the read path's check at ExprKind::Index
Evidence: the read form `return i32(xs[0])` on `u8[]?` is rejected `E10010` (probe p105b); the store form `xs[0] = 1` is accepted and emits `((*xs)).ptr[0] = 1u;` with no test — SEGV on a null view (probe p105a, plain 139 / ASan SEGV write); inside `if (xs != null)` the store is accepted and works (probe p105c, exit 7) and must stay accepted
Rejected alternatives: a runtime null trap (the read path reports at compile time, not run time; the hardening would diverge)
Strongest counterargument: rejecting a previously-accepted program is a behavior change — but it is the same safety correction the read path already enforces, and the accepted program segfaults
Confidence: High
Hardening: in-context layers only
Policy version: 1
Root invocation ID: plans-borrow-store-integrity-20261010
Reopen triggers: a spec rule that admits null dereferences on stores; a read-path change away from E10010
```

**AR-6 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler mechanism; conforms to the existing bounds-trap rule and the read/runtime-index store paths; no reserved consequence
Objective: make `xs[0] = v` trap exactly as `xs[i] = v` and `xs[0]` already do when the index is out of range
Decision: emit the read path's `E11211` trap statement before a constant-index element store when the base has a dynamic bound (slice/`str` views); fixed-array bases keep the semantic-layer check (no guard) and runtime-index stores keep their pointer guard
Evidence: `if (xs != null) { xs[0] = 1; }` on `slice(d, 2, 2)` (empty non-null view) is accepted, writes out of bounds (probe p106a: plain exit 0, ASan stack-buffer-overflow WRITE); the read form and the runtime-index store form both emit `if ((i < 0) || …) cl_trap("E11211", …)` (probes p106b/p106c, exit 134 with the trap message); the in-bounds constant store works (p106d, exit 9) and must stay working
Rejected alternatives: routing constant-index stores through `StorePointer` (broad emission churn across accepted shapes and pinned suites)
Strongest counterargument: adding a trap line changes emission for every slice element store — intended, and covered by shape pins and the corpus/emission audits
Confidence: High
Hardening: in-context layers only
Policy version: 1
Root invocation ID: plans-borrow-store-integrity-20261010
Reopen triggers: a spec rule that makes constant slice stores unchecked; a read-path guard-spelling change
```

**AR-6 (preflight clarification, 2026-10-10):** the `Guards` statement is constructed only when at
least one selected level's base has a dynamic bound (a fixed-array-only chain constructs no
statement, so fixed-array emission and `for`-step headers stay unchanged), and it covers every
constant-index store form (plain, compound, update, row). The original "the emitter cannot
recompute the per-level base types" premise was incorrect (`codegen/expr.rs` walks per-level place
types); an emitter-side derivation was considered and rejected because the checker has one
insertion point per assignment path while the store forms fan out across several emitter arms, and
the guard then reuses the read path's guard-value recursion verbatim. See the preflight report
PF-005/PF-007.

**AR-7 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: conformance to the confirmed concrete rules (AR-2's mapping); no new language surface
Objective: one substitution map for the DEF-100 self-assign judgment
Decision: owning non-nullable substitution → E10034 (the concrete `borrow String` spelling, probe p100b); nullable substitution → E10020 (the concrete `borrow String?` out-slot rule, pinned); duplicable/view/unknown substitution → accepted (probes p100c/p100d)
Evidence: the three concrete spellings above; the concrete `borrow T?` classification is OutSlot while the generic declaration stays Param, so the duty must emulate the per-instance classification
Rejected alternatives: one diagnostic for all substitutions (would diverge from a pinned concrete spelling)
Strongest counterargument: two diagnostics on one generic site could confuse — but each instance reports once, matching its concrete spelling
Confidence: High
Hardening: in-context layers only
Policy version: 1
Root invocation ID: plans-borrow-store-integrity-20261010
Reopen triggers: a change to the concrete out-slot self-assign diagnostic
```

**AR-7 (preflight clarification, 2026-10-10):** the audit checks nullability before the
duplicable/view skips — every nullable `borrow` concrete spelling reports `E10020`, including
view, `raw`, and `cstring` substitutions — while a non-nullable duplicable/view/unknown
substitution stays accepted; one site reports once (the first offending instance in instantiation
order). See the preflight report PF-003/PF-008.

**AR-8 (delegated):** verified in probes p101d/p101e — `return xs[0]` and `return xs[i]` through a `String[]?` slot are rejected `E10026` ("partial moves are limited to top-level fields in v1"). The plan pins both rejections as guards; no fix work.

**AR-9 (delegated):** verified in probe q102e — `xs[i] = xs[i]` on a non-nullable `borrow xs: i32[]` is accepted and emits a plain self-copy with the read's guard. The out-slot self-assign carve-out does not apply; no change, guard test only.

**AR-11 (preflight clarification, 2026-10-10):** the in-container `RDC_ASAN=1` full-suite run is a
non-gating convenience (timing-flaky, matching the previous cycles); the per-case host ASan probe
(emit, `cc -fsanitize=address -no-pie`, run) is the phase memory oracle. See the preflight report
PF-013.

**AR-12 (delegated):** P1 groups the two self-assign judgments (DEF-100's parameter-typed lend root and DEF-102's out-slot element root) because they share the comparison/classification machinery; P2 is the return emission alone; P3 groups the two store-guard changes that both sit in the assignment-target emission path (null requirement and bounds trap).

**AR-13 (delegated):** per `AGENTS.md` — P1 (ownership rules, a lend language-rule change) → correctness + memory-safety + semantics reviewers; P2 (return emission) and P3 (store emission) → correctness + memory-safety plus the emission-integrity auditor (base/head emission diff, strict-build hunt). Packets include the blast-radius step for the changed construct. *(Preflight correction 2026-10-10: the P1 list now matches 99's phase lenses and the AGENTS.md matrix; see PF-012.)*

**AR-14 (delegated):** plan folder `plans/borrow-store-integrity/` (flat layout, the repository convention); tests `compiler/tests/borrow_store_integrity_spec_test.rs` and `_impl_test.rs`; new internal names follow `Requires*` duty and `emit_*` emitter patterns.

**Scan completeness (12 categories):** Feature gaps — covered by the per-defect requirements (01) and components (03). Behavioral — AR-3, AR-4, AR-5, AR-7. Scope — AR-1, AR-8, AR-9, AR-10. Technical — AR-2, AR-5, AR-6, AR-11, AR-12. Edge cases — covered in 03 (nested element targets, narrowed bases, projected returns, duplicable/nullable/view substitutions) and pinned in 07. Integration points — the duty audit (`own/instance_check.rs`), the checker (`own/expr.rs`, `own/place.rs`), the emitter (`codegen/function.rs`, `codegen/cleanup.rs`, `codegen/guards.rs`), the instance-collection walks (`own/generics.rs`); owned by 03. Data & state — no new data model or state; the existing place/duty state only. Security & compliance — memory safety is the security surface for a compiler; AR-11's ASan probes cover it. Non-functional — determinism (byte-identical re-emission) follows the existing suite convention; no performance surface. UX & presentation — all diagnostics reuse existing codes/messages (`E10010`, `E10020`, `E10034`, `E11211`); no new user-facing text. Stakeholder conflicts — none. Naming — AR-14.
