# Ambiguity Register: Borrow/Generic Integrity (DEF-77/78/79/81/82/83)

> **Status**: ✅ GATE PASSED — all 13 items resolved
> **Last Updated**: 2026-10-09 20:23
> **CodeOps Artifact Schema**: 1
> **Auto-design**: root invocation `ad-borrow-generic-integrity-20261009` (policy version 1) —
> eligible technical decisions carry delegated provenance; product behavior, scope, and
> acceptance criteria were decided by the user.

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| AR-1 | Scope | Plan scope, the DEF-78/DEF-79 row corrections, success criteria, plan folder | Confirm as stated / adjust | **User:** confirmed as stated — fix DEF-77/78/79/81/82/83 to the frozen spec's rules; DEF-78 covers the concrete and generic spellings; nullable out-slots and duplicable lends stay legal; OUT: DEF-76/80/90/92–95, new features, unrelated refactors; success: all six repros fixed, new suites green, host ASan clean on the ownership probes, `make verify` all legs; folder `plans/borrow-generic-integrity/`. The filed DEF-78/DEF-79 row notes are corrected at close-out. | ✅ Resolved |
| AR-2 | Behavioral | DEF-78: the rule for a whole-binding store into a lend parameter | Declaration-site + per-instance duty (recommended) / other | **User:** confirmed the recommended rule. Derived predicate: reject when the (substituted) slot is non-nullable and owns storage (non-duplicable, non-view); nullable out-slots and duplicable lends accept. Reuses the existing `E10034` message and help unchanged. | ✅ Resolved |
| AR-3 | Technical | DEF-77: how the may-lend gate makes callee identity unambiguous | Extend the callee-name guard (recommended) / IR callee-kind marker | **User:** confirmed — extend `unique_ir_name`'s callee-masking guard to every binding kind whose IR name can be a lowered callee. | ✅ Resolved |
| AR-4 | Technical | DEF-79: where nullability enters the type key | In `type_key` itself (recommended) / struct-instance key only | **User:** confirmed — fold nullability into `type_key`; keep the existing `Q` suffix; preserve the text-conversion helper spelling by clearing the flag at that consumer. | ✅ Resolved |
| AR-5 | Naming | DEF-78: name and shape of the per-instance store duty | `RequiresStorableSlot` / reuse `RequiresNotLend` with a flag / generalized store-policy enum | **AI — delegated (eligible: internal interface)** → `ir::RequiresStorableSlot { function, parameter, ty, span }`, audited in `own/instance_check.rs`. See notes. | ✅ Resolved (delegated) |
| AR-6 | Technical | DEF-79: disposition of `instance_arg_key` after the key change | Remove (use `type_key`) / keep as a delegation | **AI — delegated (eligible: internal interface, DRY)** → remove it; its three call sites call `type_key` directly. See notes. | ✅ Resolved (delegated) |
| AR-7 | Behavioral | DEF-81: the exact comparison rule for views | View-vs-`null` only (recommended) / general view equality | **User:** confirmed — `==`/`!=` accept a view with `null` in either order; `view == view` stays `E10222`. Derived: the view's own nullability does not restrict the comparison (consistent with the handle path). | ✅ Resolved |
| AR-8 | Behavioral | DEF-82: out-slot self-assignment semantics | Reject with `E10020` (recommended) / no-op | **User:** confirmed — reject `slot = slot` with the existing `E10020` message, the same as locals. Whole-binding coverage; projected self-assign is unreachable for out-slot kinds (`T?` is pointer-like only). | ✅ Resolved |
| AR-9 | Behavioral | DEF-83: which parameters the `W10001` exemption covers | All nullable borrow params (recommended) / `raw?` only | **User:** confirmed — exempt every nullable borrow (out-slot) parameter. | ✅ Resolved |
| AR-10 | Scope / process | Whether `spec/` may change | Keep `spec/` untouched (recommended) / add clarifications | **User:** confirmed — `spec/` stays byte-identical; the plan cites T8, OW-16/FN-7, D9, and the out-param carve-out and conforms the implementation. | ✅ Resolved |
| AR-11 | Sequencing | Phase structure for the execution plan | 4 phases (recommended) / one per defect | **User:** confirmed — P1 DEF-78+82+83, P2 DEF-79, P3 DEF-81, P4 DEF-77. | ✅ Resolved |
| AR-12 | Testing | Verify command and test-file layout | Confirm (recommended) / different | **User:** confirmed — `make check` per phase; host-ASan probes for P1; strict host build check for P2; `make verify` final; suites `compiler/tests/borrow_generic_integrity_spec_test.rs` and `borrow_generic_integrity_impl_test.rs`. | ✅ Resolved |
| AR-13 | Technical | DEF-78: check placement and interaction with the existing lend-move duty | Site rules (see notes) | **AI — delegated (eligible: compiler mechanism)** → whole-binding `Mode::Borrow` parameters only; the check runs in the ownership assignment path before the store is emitted; the per-instance duty is recorded at the same site; at most one `E10034` per assignment site (deduped against the existing lend-move duty); field and element stores keep their existing rules. See notes. | ✅ Resolved (delegated) |

## Resolution Notes

**AR-2 (derived facts):** the storage predicate mirrors `owns_drop` in
`compiler/src/own.rs` (`!duplicable`, not a view, not unknown). Legal-shape pins that must stay
accepted: `borrow i32` stores (warning-only today) and the nullable out-slot forms pinned by the
existing `p3f2`/`p3r26`/`inst-slot` cases.

**AR-4 (derived facts):** the text-conversion fallback
(`compiler/src/codegen/expr.rs:771`) clears the flag before calling `type_key`; the runtime
helpers (`cl_text_*` in `compiler/runtime/shim_host.c`) keep their fixed names. The `Q` marker
keeps the function-instance spellings byte-identical (`instance_arg_key` today produces the same
`StringQ` form).

**AR-7 (derived facts):** the narrowing machinery is type-agnostic and needs no view-specific
change — `own/place.rs` (`narrowing`) matches any `x == null` / `!= null` shape and
`own/statements.rs` (`apply_narrowing`) records a narrowed place path; the emitter's
`view_null_test` (`compiler/src/codegen/expr.rs`) already lowers the `.ptr` test. The sema
operator rule is the only blocker.

**AR-8 (derived facts):** reference behavior verified — a local `s = s` is rejected
`E10020: 's' was moved and cannot be used`; the out-slot case is accepted today and empties the
caller's storage (probe exits 1 where the value should survive).

**AR-5 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: internal interface and naming; stays within the confirmed DEF-78 behavior; no reserved consequence
Objective: record the parameter-typed store site so each instantiation is judged by one small, reviewable mechanism
Decision: ir::RequiresStorableSlot { function, parameter, ty, span }, collected in the ownership pass and judged by a new check in own/instance_check.rs alongside the existing duty audits
Evidence: the established duty pattern (RequiresNotLend, RequiresDuplicable, RequiresOutSlotPlace) in compiler/src/ir/mono.rs; the audit structure in own/instance_check.rs
Rejected alternatives: reuse RequiresNotLend with a boolean flag — overloads two different predicates and hides a mode; a generalized store-policy enum — a larger surface with no second use
Strongest counterargument: a new record type where a flag would do; the flag's ambiguity is the reason it loses
Confidence: High — the pattern exists three times over
Hardening: not required — internal, reversible, covered by the P1 correctness and memory-safety reviews
Policy version: 1
Root invocation ID: ad-borrow-generic-integrity-20261009
Reopen triggers: a third store-like duty appears, or review finds the record cannot carry the judgment
```

**AR-6 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: internal interface; implementation mechanism
Objective: one rule for one fact — instance keys include nullability
Decision: remove instance_arg_key; its three call sites use type_key directly
Evidence: compiler/src/codegen/types.rs:53 is a one-line wrapper today; with AR-4 it duplicates the same rule
Rejected alternatives: keep it as a thin delegation — two names for one rule invite drift; keep and special-case — no second behavior remains to preserve
Strongest counterargument: the name documents instance intent at the call sites; the component doc names those sites instead
Confidence: High
Hardening: not required — reversible, reviewed with P2
Policy version: 1
Root invocation ID: ad-borrow-generic-integrity-20261009
Reopen triggers: a future instance key must differ from type_key for a reason beyond nullability
```

**AR-13 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler mechanism; stays within the confirmed DEF-78 rule
Objective: reject the offending store once, at the right stage, without disturbing legal stores
Decision: in the ownership assignment path (own/expr.rs check_assign, whole-binding branch) reject a store to a BindingKind::Param, Mode::Borrow binding when the target type owns storage, and record RequiresStorableSlot when the target type is parameter-typed; field and element stores and unmarked/const parameters keep existing rules (E10024 for read-only writes); the instance audit rejects a duty when the substituted slot is non-nullable and owns storage; one E10034 per assignment site, deduped so a lend-move duty at the same span does not double-report
Evidence: own/expr.rs check_assign (whole-binding branch); own/place.rs move_place (the existing release/store rule, reachable for moves but not stores); own/statements.rs emit_store; the duty recorders at own/place.rs (mark_moved_if_owning, record_requires_duplicable) and the audit at own/instance_check.rs
Rejected alternatives: suppress the target release in the emitter for lend params — accepts a statement the spec forbids (OW-16) and leaves the stored copy unreleased; reject every parameter-typed store at the declaration site — over-rejects the legal nullable instances (p3r26, inst-slot)
Strongest counterargument: the check restates part of move_place's rule at another stage; the two guard different operations (release vs store) that the spec words together, so a shared helper may be warranted if review finds drift
Confidence: Med-High — placement traced through the assignment and emission paths; the dedupe shape is exercised by the ST-1/ST-4 pairs
Hardening: not required — bounded internal mechanism; P1 carries correctness and memory-safety reviews and an ASan probe
Policy version: 1
Root invocation ID: ad-borrow-generic-integrity-20261009
Reopen triggers: a store shape reaches the target release without passing check_assign, or review shows one report per site cannot be achieved
```

### Systematic 12-category scan

| Category | Outcome |
|----------|---------|
| Feature gaps | Items recorded (AR-1, AR-2) |
| Behavioral gaps | Items recorded (AR-2, AR-7, AR-8, AR-9) |
| Scope ambiguities | AR-1, AR-10 |
| Technical unknowns | AR-3, AR-4, AR-6, AR-13 |
| Edge cases | AR-2 (duplicable/nullable boundaries), AR-7 (non-nullable view), AR-8 (whole-binding reach) |
| Integration points | Clear — compiler-internal stages only; the emitted-C ABI/name changes are covered by P2's emission sweep |
| Data & state | Clear — no persisted data, no migration surface |
| Security & compliance | Clear — no input, auth, secrets, or network surface; memory safety is the host-ASan control in P1 |
| Non-functional | Clear — no performance targets; emission determinism stays pinned by the existing re-emission tests |
| UX & presentation | Items recorded (AR-8, AR-9) — diagnostics and the warning behavior; no new message text |
| Stakeholder conflicts | Clear — priorities set by the confirmed scope |
| Naming & terminology | Items recorded (AR-1 folder, AR-5 duty, AR-12 suites; AR-4 marker) |
