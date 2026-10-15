# Current State: Conditional, Projection, and Update Residue II

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

Batch I extended the conditional/projection machinery and left the sites this batch's rows
extend. The planning baseline is the batch-I close-out commit `a462084`; the line numbers below
are that tree.

- **Root binders and carriers** (`compiler/src/own/lower.rs`):
  `bind_guarded_projection_root` (:3429), `own_ownerless_conditional_arms` (:3695),
  `hoist_ownerless_conditional_arms` (:4003), `bind_ownerless_projection_root` (:3122),
  `bind_view_projection_root` (:3210), `adopt_view_backings` (:3300),
  `carried_view_owners` (:2727), `consume_conditional_projection` (:2181),
  `distribute_conditional_projection` (:1660), `roots_in_conditional` (:1745),
  `materialize_owned_argument` (:4199), `consume_literal_sources` (:4124).
- **Discard and literal materializers** (`lower.rs`): `defer_discarded_arm` (:1165),
  `materialize_discarded_field` (:1232), `field_aliases_owned_storage` (:1290),
  `discard_binds_owner` (:1330), `bind_discarded_owner` (:1341).
- **Fixed-array root** (`lower.rs`): `bind_fixed_array_root` (:1590) — the batch-I phase-5
  correction binds each arm's root in the store/return/iterable gates and distributes a nested
  conditional's leaves; the element-read base is the residual gap (`DEF-306`).
- **Update pin** (`lower.rs`): `contains_update` (:4553), `pin_update_value` (:1457) — the
  batch-I phase-6 work pins the assignment target/RHS, the literal operands, the dot receiver,
  the loop condition (through the per-iteration `Materialize` fill), and the owning argument;
  the guarded `&&`/`||`/`?:` operand keeps the deferred spelling (`DEF-308`).
- **View provenance and owners** (`compiler/src/own/place.rs`): `view_provenance` (:605),
  `storage_atoms` (:700), `collect_view_owners` (:1028), `call_provenance` (:1190),
  `view_is_mixed` (:1207), `call_view_source` (:1250).
- **Store and return judgments** (`compiler/src/own/statements.rs`): `prepare_store` (:412) with
  the `LiteralViews` destination treatment, `check_return` (:1253); the escaping-view store
  judgment is `check_escaping_view_store` (`own/expr.rs` :1291) and the element-store rule sites
  are `own/expr.rs` :564/:885/:940; `mark_read_conditional_move` is `own/expr.rs` :353.
- **Call-argument owners** (`own/call_check.rs`): `argument_owner` (:680).
- **Fixed-array emission** (`compiler/src/codegen`): `fixed_array_field_needs_stores`
  (`function.rs` :1542), `emit_fixed_array_return_arm` (`cleanup.rs` :751), the per-arm
  destination store in `function.rs`/`guards.rs`, and the deferred fill
  (`function.rs` `has_deferred_fill` :2406).
- **Verified follow-up status** (AR-3): `DEF-278`/`289`/`296`/`301` fixed; `DEF-306`'s
  store/return/iterable distribution and `DEF-308`'s plain loop-condition sibling fixed; all
  other in-scope reds confirmed, including the critical `DEF-303` (segfault) and the majors
  `DEF-286`, `288`, `294`, `295`, `310`.

### Relevant Files

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `compiler/src/own/lower.rs` | Root binders, carriers, discards, fixed-array root, update pin | The residual root bindings, the discard/literal gates, the read-base distribution, the update-pin trigger, the raw-deref store lowering, the fill pin |
| `compiler/src/own/place.rs` | Provenance, owner walk, call-view source | The cross-frame backing propagation, the container element loans |
| `compiler/src/own/statements.rs` | Store preparation, return judgment, `LiteralViews` | The carrier-escape element loans, the all-null atoms, the report dedupe |
| `compiler/src/own/expr.rs` | Element-store rule, move marks, escaping store | The runtime-index element store, the mark alignment, the null-atom acceptance |
| `compiler/src/own/loops.rs` | Loop-condition and `for`-iterable lowering | The loop-condition root binding and the receiver pin |
| `compiler/src/codegen/*` | Fixed-array destinations, literal/guarded emission, arm shapes, return shapes | The conditional field values, the call-filled literal fill, the `String[2]` return arms, the guarded fallback, the element gate |
| `compiler/tests/conditional_projection_residue_*.rs` | The batch-I suite pair | Append additions only (AR-12) |

## Gaps Identified

Each gap names the row, the observed behavior on an accepted program, the required behavior, and
the site. The roadmap rows carry the reproductions; this table is the site map. Line numbers are
the planning baseline; the executor re-resolves them before editing.

### P1 — View owners, loans, and stores

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-281` | `holdNum(c ? a.n : mkBox().n)` and the return twin leak 11 B per fresh root (ASan confirmed); the read positions `(c ? mk() : b).ys[i]`/`mk().ys[i]` leak 48 B/4 allocs | The fresh root binds for its residual release even when the projected member is duplicable (the selected value stays a copy) | `lower.rs` the carrying gates (`!duplicable \|\| maybe_owns`), `bind_ownerless_projection_root`/`materialize_owned_argument` |
| `DEF-285` | `xs[j] = b.v;` strands the owner (11 B per store) and skips the element loan (`while { xs[j] = mk().v; }` 33 B over three iterations) | The runtime index routes through the constant index's copy/owner/loan path | `own/expr.rs` the element-store rule (:564/:885/:940), `lower.rs` `value_place`/element lowering |
| `DEF-286` | `outer(borrow slot) { let b = …; fill(b.v, borrow slot); }` and the `slot = [p, q]`/`str`-twin spellings are accepted and read freed storage (ASan, byte-identical base/head) | The call-site rule propagates the argument's backing frame so the store rejects (one report; AR-6) | `own/call_check.rs` `argument_owner`, `own/place.rs` the derivation walk, `own/statements.rs`/`own/expr.rs` the out-slot judgment |
| `DEF-287` | A module-global view initialized from a fresh call leaks its owner (11 B) | The fresh owner is adopted or released at module initialization | `own/program.rs`/`own.rs` the initializer owner path, `own/lower.rs` the init-owner registry |
| `DEF-288` | `xs[1] = b.v; return xs;` (and the alias spelling), `xs[1] = b.v; g = xs;` are accepted and read freed storage while the single-view twin reports one `E10035` | The view-like return and the global store's view-like branch consult the binding's element loans as the carrier path does | `own/statements.rs` `check_return` (:1253)/`prepare_store`, `own/place.rs` the loan lookup |
| `DEF-290` | A stale literal-carrier loan survives `h.names = ["AA"]` (or the whole-carrier reassignment), so a later backing write reports a false `E10031` | The literal carrier loans drop on the field/whole overwrite (the constant-index element twin replaces its loan) | `own/statements.rs` the loan record (`LiteralViews`), `own/expr.rs` the write-mutability path |
| `DEF-291` | `slot = c ? null : null;` into a `borrow str?` out-slot reports a false `E10030` (the global twin `E10033`; probe confirmed) | An empty storage-atom set is treated like the scalar `null` case (names no storage) | `own/place.rs` `storage_atoms` (:700), `own/expr.rs` the out-slot store rule |

### P2 — Guarded, discarded, and carrier roots

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-284` | `g ? use(Holder { names: [id(mk().v)] }) : 0` hoists `__cl_owned0 = mk().v` outside the guard, leaks the root residual, and runs the skipped call | The deferred branch covers the call-wrapped position under `defer_move_outs > 0`: the call root binds for its residual release and runs only on the taken path | `lower.rs` the deferred/guarded element path, `codegen/guards.rs` the guarded fill |
| `DEF-292` | `let h: Holder = g ? Holder { names: [mk().v] } : Holder { names: ["zz"] };` leaks the taken arm's fresh element root (11–22 B) | The taken arm's fresh element root binds for the local's frame (the loop-condition/guarded-literal precedent) | `lower.rs` the carrier literal arm (`hoist_ownerless_conditional_arms`/`adopt_view_backings`), `codegen/function.rs` the let-position arm fill |
| `DEF-293` | `c ? i32(mk().v.len()) : t;` leaks 11 B: the arm gate keys on the selected type (owns nothing) | The arm's release need is judged by the root's residual ownership, not only the selected type | `lower.rs` `discard_binds_owner` (:1330), the arm gate |
| `DEF-297` | `[mk().v];` as a bare statement leaks 42 B/4 allocs (the discard machinery covers conditional arms only); at the planning head the filed spelling emits a discard statement the strict build rejects (`(void)({ … });`) — the fix removes it and releases the fresh leaves | The discarded-literal materialization extends to the bare statement spelling | `lower.rs` `defer_discarded_arm` (:1165), the discard path |

### P3 — Fixed-array and literal emissions

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-298` | `Holder { row: c ? a.row : mkBox().row }` guarded emits `(ClHolder){ (c ? … ) }.row[0]` (`int-conversion`, `missing-braces`) | The conditional value routes through the per-arm destination store in the field-initializer position | `codegen/function.rs` the literal field route, `lower.rs` the field preparation |
| `DEF-299` | `Holder { row: (c ? a : b).row }` emits the same invalid initializer | The per-arm rendering extends to the field-initializer position | `codegen/function.rs`/`lower.rs` `distribute_conditional_projection` |
| `DEF-300` | `g && len(Holder { names: [mk().v] }.names[0]) > 0` emits an incomplete `ClStr` initializer (`missing-field-initializers`) | The field in the guarded position uses the array-elements initializer | `codegen/guards.rs` the guarded literal fill |
| `DEF-302` | A conditional of call-filled fixed-array-field literals (`ok ? Mixed { names: n1(), tag: … } : …`) drops the call arguments (`too few arguments`) | The conditional-of-literals routes through the guards literal-fill work | `codegen/guards.rs` the literal fill, `function.rs` `has_deferred_fill` (:2406) |
| `DEF-303` | `String[2]` conditional-root return `(c ? b1 : b2).xs` emits `__cl_ret = __cl_tmp;` (an array assignment through the C-adjusted parameter) and segfaults at run time (probe: exit 139) | Each arm binds its root in the fixed-array return emitter (the gap between `consume_conditional_projection` Temp arms and `emit_fixed_array_return_arm`) | `codegen/cleanup.rs` `emit_fixed_array_return_arm` (:751), `lower.rs` `consume_conditional_projection` (:2181) |
| `DEF-304` | `g && consume((c ? b1 : b2).names)` and `g && consume(c ? mk1() : mk2())` take the expression-path fallback and emit invalid C (`assignment to expression with array type` / `too few arguments`) | The guarded expression-path fallback routes constructors through the per-arm destination store | `codegen/guards.rs` the fallback, `lower.rs` the guarded preparation |
| `DEF-305` | `Outer { items: [Inner { row: n1() }, …] }` emits `too few arguments`; `fixed_array_field_needs_stores` does not descend `ArrayLit` elements | The gate descends array-literal elements | `codegen/function.rs` `fixed_array_field_needs_stores` (:1542) |
| `DEF-306` (residual) | `(c ? (d ? b1 : b2).ys : b3.ys)[i]` emits `lvalue required as unary '&' operand` (probe confirmed, container and host); the store/return/iterable gates are fixed | The read base distributes through the nested arms like the store/return/iterable gates | `lower.rs` `distribute_conditional_projection` (:1660)/`bind_fixed_array_root` (:1590), the read position |

### P4 — Loop conditions, update order, and store lowering

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-294` | `while (i < 1 && (c ? mkH() : a).call(5) > 0)` fails the host GCC 15 `-Werror=sequence-point` (the container accepts; emission byte-identical to the phase-4 tree) | The receiver position pins like the `DEF-277` pattern; both toolchains measured (AR-10) | `lower.rs` `pin_update_value` (:1457), `own/call.rs` the receiver, `codegen/guards.rs` the deferred fill |
| `DEF-295` | `while (i < k && i32(len(mkH().s)) > 0)` double-frees (probe: run abort 134, ASan double-free) | The projection's root binds once and the member reads from it | `lower.rs` `bind_ownerless_projection_root` (:3122)/the loop-condition walk, `own/loops.rs`, the per-iteration `Materialize` fill |
| `DEF-308` (residual) | `g && f(i++, i)` and `c ? f(i++, i) : 0` emit the unsequenced deferred operand (`-Werror=sequence-point`, container and host); the plain loop-condition sibling is fixed | The guarded operand's parts pin inside the guard path (the per-iteration `Materialize` fill precedent) | `lower.rs` `pin_update_value` (:1457), `codegen/guards.rs`, `own/call.rs` |
| `DEF-309` | `[; i++]` runs the fill expression once per element (three iterations for a three-element array) where spec/04 requires once | The fill pins into a temporary before the element loop | `lower.rs` the array-literal lowering, `codegen/expr.rs` `array_literal` (:1012) |
| `DEF-310` | `unsafe { *q = 9; }` and `*q = i++;` compile and the store emits nothing (probe: exit 254, the write is dropped); the field write through a raw works | The bare raw-deref store target lowers | `own/statements.rs` the store statement lowering, `own/expr.rs` `check_place_raw` (:1976-adjacent), `ir.rs` `Statement::Assign`/`StorePointer` (:299) |

### P5 — Diagnostics and marks

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-282` | `return (c ? a : (d ? mkBox() : mkBox2())).v;` reports two identical `E10030` at one span (probe confirmed); `gfieldcall` adds an `E10033` at the store while the callee's return reports `E10036`; a mixed `Pair` array reports two `E10030` at one span | One report per span; a downstream consequence report is withheld where the callee-side judgment already reports (AR-7) | `own/liveness.rs` the generated-owner report, `own/statements.rs` `check_return`, `own/place.rs` the per-backing walk |
| `DEF-283` | The deferred mixed view return prints `borrow of '__cl_owned1' does not live long enough` (probe confirmed) | The generated-owner phrase `a compiler-generated owner` renders (the pinned phrasing) | `own/liveness.rs` `display` (:594) — the deferred path must route through it |
| `DEF-307` | `for (borrow x in (c ? b1 : b2))` with owned elements reports `E10020: 'b1' was moved` at a later use (probe confirmed; the in-place emission is fixed) | The mark aligns with the in-place route (no move mark) | `own/expr.rs` `mark_read_conditional_move` (:353) |

## Dependencies

### Internal Dependencies

- The batch-I residue suites (`conditional_projection_residue_spec_test.rs`,
  `conditional_projection_residue_impl_test.rs`) are the regression oracle; the new cases append
  to them (AR-12).
- The batch-H suites, the `callable_release_integrity` suite (its one-report pin,
  `should_reject_a_mixed_conditional_view_return`), and the `checker_conformance` phrase pin gate
  the phases that touch reports.
- `make check` (the container) and `make verify`'s Amiga legs gate every phase's green state.

### External Dependencies

- None — the workspace stays std-only.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `DEF-286`'s call-site rule over-rejects the accepted parameter/same-frame twins | Med | High | The phase measures the twins first and stops to park the row if the rule cannot be bounded (AR-6) |
| `DEF-294`'s red does not reproduce on the pinned container | High (measured) | Low | Dual-toolchain measurement; the emission pin; the control outcome is a recorded precedent (AR-10) |
| A root-binding fix in the freshly reviewed machinery regresses a batch-I pin | Med | High | Red/green runs include the batch-I suites; the memory auditor probes every new binding on both taken paths and in loops |
| `DEF-282`'s dedupe withholds a report the oracle counts | Med | Med | The oracle pins (`callable_release_integrity` exactly one `E10030`; `checker_conformance`'s phrase) are run before and after; the semantics lens reviews the boundary (AR-7) |
| `DEF-303`'s per-arm return fix changes the fixed-array return shape broadly | Low | High | The emission auditor diffs base vs head over the corpus; the direct return spellings have byte-identity controls (P3) |
| An over-rejection in P1/P2 (the null/loan rows) | Med | Med | The semantics/memory lenses probe the twin spellings; every new rejection needs the frozen-rule citation |
