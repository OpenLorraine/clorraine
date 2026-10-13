# Current State: Conditional, Projection, and Update Residue

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

Batch H built and reviewed the conditional/projection machinery this batch's rows extend:

- **Arm roots and carriers** (`compiler/src/own/lower.rs`): `bind_ownerless_projection_root`
  (:2331) binds an ownerless call/literal root of a consumed projection; `bind_view_projection_root`
  (:2419) binds a view projection's backing; `own_ownerless_conditional_arms` (:2778) and
  `hoist_ownerless_conditional_arms` (:3077) are the conditional-arm binders; the discard path uses
  `defer_discarded_arm` (:1019), `field_aliases_owned_storage` (:1069), `discard_binds_owner`
  (:1109), and `bind_discarded_owner` (:1120); `materialize_owned_argument` (:3223) is the
  per-argument binder.
- **View provenance and owners** (`compiler/src/own/place.rs`): `view_provenance` (:605) judges a
  view expression's storage (the Conditional arm returns `None` when the arms disagree unless a
  local-backed arm keeps its provenance, the batch-H phase-5 rule); `collect_view_owners` (:826)
  gathers a view expression's owners for the loan record; `call_provenance` (:966) rides a
  view-returning call's argument derivation; `compiler/src/own/call_check.rs:680` `argument_owner`
  maps a view argument to its owning binding.
- **Returns and stores** (`compiler/src/own/statements.rs`): `check_return` (:1163) judges a view
  return's derivation and records its loans; `prepare_store` (:357) prepares a value-into-place
  store (the ownerless-root binding, the view-backing binding, and the batch-H phase-5
  fixed-array root branch).
- **Fixed arrays**: `bind_fixed_array_root` (`lower.rs:1279`) binds an ownerless projection root
  before a fixed-array copy; the codegen destination-store shape serves the direct spelling.
- **Update pin** (`lower.rs`): `contains_update` (:3506) detects a nested update; `pin_update_value`
  (:1234) binds an operand or argument to a statement temporary at its lowering position, with the
  `defer_move_outs`/`in_loop_condition` exclusions and the duplicable-or-update-leaf safety rule.
- **Judgments** (`compiler/src/own/expr.rs`): the element-store rule reports `E10033` at :564,
  :885, and :940; `compiler/src/own/liveness.rs:350` reports the escape (`E10030`).

### Relevant Files

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `compiler/src/own/lower.rs` | Arm binders, carriers, discards, fixed-array root, update pin | Per-instance duty, fresh-root bindings, guarded carrier, literal roots, pin triggers |
| `compiler/src/own/place.rs` | Provenance and the owner walk | The call-wrapped owner walk (every arm) |
| `compiler/src/own/call_check.rs` | Argument owners | The call-wrapped argument owner |
| `compiler/src/own/statements.rs` | Store preparation and the return judgment | The per-destination store judgment; the `str[N]` return guard; the fixed-array arm route |
| `compiler/src/own/expr.rs` | Element-store and mutability judgments | The scoped element-store acceptance and the widening gate |
| `compiler/src/own/liveness.rs` | The escape report | Unchanged (`DEF-279` parked — AR-2) |
| `compiler/src/codegen/*` | The fixed-array destination shape, arm spellings | The conditional-arm destination and the `for`-iterable projection |
| `compiler/tests/conditional_projection_residue_*.rs` | New suites | Created by this plan |

## Gaps Identified

Each gap names the row, the observed behavior on an accepted program, the required behavior, and
the site. The batches' road-map rows carry the reproductions; this table is the site map.

### P1 — Arm roots and per-instance duty

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-257` | A generic arm projection (`(c ? a : mkPair<T>(x, y)).v`) copies without a move-out → UAF under ASan | The projection is judged per instance like the concrete twin | `own/lower.rs` arm recorders, `own/expr.rs` projected-source collector, `own/call_check.rs` judgment |
| `DEF-258` | `let s: String = c ? a.v : mkBox().v;` never releases the fresh root's sibling field | The fresh arm's root binds through the arm-fill machinery | `lower.rs` `hoist_ownerless_conditional_arms`/`own_ownerless_conditional_arms` |
| `DEF-259` *(incidentally closed — AR-23)* | `(c ? a : Box { v: …, w: … }).v` leaked the literal's sibling in the pre-batch-H behavior | Green at the planning baseline; the program stays a green control | `lower.rs` `bind_ownerless_projection_root` |
| `DEF-260` | A guarded `take` of a conditional projection double-frees under ASan | The guarded position routes through the eager per-arm carrier | `lower.rs` `materialize_owned_argument`/`defer_discarded_arm` |
| `DEF-264` | `H { call: addOne, s: mkBox().v }.call(5)` in a guarded position leaks the projection root's sibling | The struct-literal field's projection root binds | `lower.rs` `bind_guarded_projection_root`/`bind_ownerless_projection_root` |

### P2 — View derivation and owners

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-261` | `bytes((c ? b : d).v)` (parameter/parameter) records no owner and the two-arm spelling collects only the first arm → use-after-free; the local-arm spelling already reports `E10031` | The call path recurses through the view-returning call's argument and collects *every* arm's owners, like the direct walk | `place.rs` `collect_view_owners`/`call_provenance` |
| `DEF-266` | `use([mk().v])` and `Holder { names: [mk().v] }` leak 11 B per element | The argument and field positions bind or retain the element backings | `lower.rs` view-element lowering, `own/expr.rs` the field check |
| `DEF-267` | `func f(s: String): str[1] { return [s]; }` reports a false `E10034`; `return [h.v]` gains an extra `E10036` | The return spelling runs through the view-element lowering (single-view parity) | `statements.rs` `check_return` |
| `DEF-268` *(parked — AR-2)* | `return c ? x : h.v;` (disagreeing arms, both parameters) is accepted | Parked: the batch-H suite pins the conditional twin accepted, so whether the disagreement must report `E10036` needs a ruling | `statements.rs` `check_return`, `place.rs` `view_provenance` |
| `DEF-279` *(parked — AR-2)* | The same return reports `E10035` and `E10030` at one span | Parked: the oracle pins exactly one `E10030` for a shape with a local arm; the dedupe needs a ruling | `liveness.rs:350`, the return's loan record |

### P3 — Escaping stores and their spellings

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-265` | `names = mk().v;` (a global) and an out-slot write are accepted; the view outlives the backing | The store is rejected per destination kind: `E10033` for the global/field (static owners only), `E10030` for the out-slot | `own/statements.rs` (global stores), `own/expr.rs` the out-slot path |
| `DEF-269` | `g.names = [mk().v]` emits `g.names = { … };` (strict build fails); a `str[1]` capture fails similarly | The accepted element store lowers through the value-into-place path; the capture through its conversion (the fresh-element spelling is rejected by the store judgment) | `own/statements.rs`, `codegen` |
| `DEF-276` | A whole-value view store into a `borrow str?` out-slot is never judged → UAF | The stored value's provenance is judged against the frame it escapes, the handle→view widening included | `own/expr.rs` (the local store check gated on non-empty projections) |
| `DEF-280` | `xs[1] = c ? b.v : a;` (a local array) reports `E10033`; the loop and nested-scope stores (`while (…) { xs[1] = mk().v; }`) are accepted with no loan and read freed storage | Acceptance is scoped to backings whose scope covers the array's, with the binding path's loan; a loop-iteration or nested-block backing reports `E10030` | `own/expr.rs` element-store rule (:564/:885/:940) |

### P4 — Guarded and discarded literal positions

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-262` | A conditional literal receiver in a loop condition double-frees (the field and the literal temporary both own) | One owner per field, as the direct-literal fix established | `lower.rs` the loop-condition walk |
| `DEF-270` | A discarded struct-literal arm mixing a fresh and a place-aliasing field leaks the fresh field, and a later use of the place field reports `E10020` | The fresh fields materialize; the place fields stay inline as reads (the direct `ok ? t : u;` control) | `lower.rs` `defer_discarded_arm`/`field_aliases_owned_storage` |
| `DEF-271` | `c ? mk().v : t;` (discarded) leaks the `mk()` root | The discarded arm binds an ownerless projection root | `lower.rs` `bind_discarded_owner` |

### P5 — Fixed-array conditional arms and projections

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-272` | `c ? a() : b();` (fixed-array results) emits a call without the hidden destination (the deferred fill falls back to a value expression) | Each arm routes through the per-arm destination store | `codegen` the deferred conditional fill and the arm stores, `own/lower.rs` |
| `DEF-275` | `let a: u8[4] = (c ? b1 : b2).xs;` and `for (x in mk().xs)` emit `&(…)` of a non-lvalue (a move-only root reports `lvalue required`) | Each arm's root binds and the projection renders the per-arm address form; the `for` iterable routes through the projection preparation | `lower.rs` `bind_fixed_array_root`, `own/loops.rs` the `for`-iterable lowering |

### P6 — Update order completion

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-273` | `arr[i] = i++;`, `i = i++;`, `i += i++;` fail the strict build (the pinned container rejects the sequence shapes under the suite's flags — AR-24) | The target's location and index parts pin before the right-hand side's update, and the update-containing right-hand side pins at its own source position | `lower.rs` the assignment/compound-assignment lowering |
| `DEF-274` | `P { a: i++, b: i }` and `[i++, i]` leave the C initializer order unspecified (the host orders left-to-right, AR-24) | The update-containing field or element pins at its source position | `lower.rs` literal lowering |
| `DEF-277` | `(i++).get(i)` and `while (i + id(i++) < 3)` fail the strict build (the pinned container rejects the sequence shapes — AR-24) | The dot receiver pins at argument position zero; the loop condition pins through a per-iteration `Materialize` fill | `lower.rs`/`own/loops.rs` the loop condition, `own/call.rs` the call receiver |
| `DEF-278` | `f(g(), i++)` evaluates `g()` after the update (the owning argument is not pinned) | The owning argument binds at its own position | `lower.rs` `pin_update_value`/`materialize_owned_argument` |

## Dependencies

### Internal Dependencies

- The batch-H suites (`conditional_projection_integrity_spec_test.rs`,
  `conditional_projection_integrity_impl_test.rs`) are the regression oracle: they stay green
  through every phase.
- The `make check` suite set (64 binaries) and `make verify`'s Amiga legs gate every phase's green
  state.

### External Dependencies

- None — the workspace stays std-only.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| A fix in the freshly reviewed machinery regresses a batch-H pin | Med | High | The phases' red/green runs include the batch-H suites; the emission auditor diffs base vs head on P3/P5/P6 |
| A guarded-position fix re-introduces the phase-4 double-free class | Med | High | The memory auditor probes every guarded shape on both taken paths and in loops before the phase closes |
| An over-rejection (a safe program rejected) | Med | Med | The semantics reviewer probes the twin spellings; a new rejection needs the frozen rule's citation |
| A per-instance fix changes generic emissions broadly | Low | Med | The corpus differential classifies every delta; the generic spellings have concrete twins to compare |
