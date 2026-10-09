# Current State: Borrow-Store Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The ownership pass (`compiler/src/own/`) checks assignments, returns, and calls over the typed IR;
the emitter renders stores by releasing the target's old value first. Generic bodies defer
substitution-dependent judgments to per-instance duties (`ir::Requires*` records) audited in
`own/instance_check.rs`. Null narrowing is tracked per place path (`own/place.rs narrowing`,
`own/flow.rs PlaceState.narrowed`); the **read** path reports `E10010` for a nullable base used
without a narrowing, while the **write** path does not. Element writes with a constant index lower
to a `Place` with `Projection::Element(Some(n))` and render directly (`xs[0] = v`); element writes
with a run-time index lower to `Statement::StorePointer` over an `AddressOf(Index)` value whose
guard pass emits the `E11211` bounds trap. Returns of owned places are emptied by
`Emitter::emit_arm_source_moves` (`codegen/function.rs`) in the result-variable and
block-local branches of `Emitter::emit_return` (`codegen/cleanup.rs`); the direct-return branch
(no result variable, no releases) returns without emptying the source.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/expr.rs` | `check_assign`, `check_update`, `check_lend_target_store` | DEF-100 duty recording; DEF-102 conservative-comparison doc; DEF-105 walker call |
| `compiler/src/own/place.rs` | `place_target`, projections, narrowing | Reference for the exact self-assign test and the narrowing set |
| `compiler/src/own.rs` | `declare_param`, duty recorders (re-export) | New `RequiresSelfAssign` record (AR-2) |
| `compiler/src/ir/mono.rs`, `compiler/src/ir.rs` | Duty record types, their re-export, and the `Statement` enum | `RequiresSelfAssign` definition and accessor; the `Statement::Guards` variant |
| `compiler/src/own/instance_check.rs` | Per-instance audits | New self-assign audit: `E10034` / `E10020` mapping (AR-7) |
| `compiler/src/own/generics.rs` | Instance-collection walks over `Statement` | Walk arm for `Statement::Guards` (guards join instance collection) |
| `compiler/src/codegen/cleanup.rs` | `emit_return` branches | DEF-101: direct branch empties a caller-storage source |
| `compiler/src/codegen/function.rs` | Statement emission, `pointer_params`, place rendering | DEF-101 detection; DEF-106 target guard emission |
| `compiler/src/codegen/guards.rs` | `emit_bounds_guard`, `bounds_guard_condition` | Reused by DEF-106; no change expected |
| `compiler/tests/borrow_store_integrity_spec_test.rs`, `_impl_test.rs` | New suites (AR-11) | All ST cases and shape pins |

### Code Analysis

- **DEF-100.** `check_lend_target_store` (`own/expr.rs:639`) judges a whole-binding store into a
  `Param ∧ Borrow` slot: it first defers to `lend_moves_judged`, which returns true when any
  source is a lend place (the predicate covers `Mode::None | Const | Borrow`) — and for the
  generic `slot = slot` the source *is* that same slot, so the site is skipped. The concrete
  `borrow String` spelling reports `E10034` through the lend-move path instead (`bind_value` →
  `BindingEffect::Move` → `move_place`, `own/place.rs:214`); the `owns_drop` tail in
  `check_lend_target_store` is unreachable for this shape because the deferral returns first. The
  concrete `borrow String?` spelling is an `OutSlot` and reports `E10020` in the first branch. The
  generic declaration stays `Param`, so nothing judges it after substitution: the emitted instance
  takes the handle by value, releases it, and self-assigns it (probe p100a).
- **DEF-102.** The `OutSlot` branch compares `source_projections == projections`. A run-time index
  produces `Projection::Element(None)` on both sides (`own/place.rs:929` `element_index`), so any
  same-root run-time pair compares equal and is rejected `E10020`; only literal indices
  (`Element(Some(n))`) distinguish. The filed "unguarded" premise does not reproduce at the plan
  baseline (probes p102a, q102b–q102d).
- **DEF-105.** `eval`'s index walk (`own/expr.rs:281-289`) reports `E10010` when an index base is
  nullable and not narrowed. The assignment path (`check_assign`, `check_update`) only runs
  `check_place_raw` over the target's projections; no null requirement is applied, so a store
  through a possibly-null view is accepted and dereferences null at run time (probe p105a).
- **DEF-106.** A slice-typed constant-index store renders `((*xs)).ptr[0] = v;` with no guard;
  `bounds_guard_condition` (`codegen/guards.rs:920`) already computes the right condition for a
  slice base, and the read path and the `StorePointer` path already emit it as an
  `if (…) cl_trap("E11211", …);` statement.
- **DEF-101.** `emit_return` (`codegen/cleanup.rs:373`) has three value paths: fixed-array, the
  result-variable/ladder path (which calls `emit_arm_source_moves`, emptying the source), and the
  direct branch (`None` result and no releases) which emits `return {lowered};` with no emptying.
  The out-slot deref `(*v)` is only reachable through the direct branch when no cleanup is needed
  (probe p101f confirms even with unrelated locals); the emitted function has no ladder because
  the out-slot parameter owns nothing the callee must release.

## Verified Reproductions (plan baseline)

All five defects and the boundary shapes were reproduced at the plan baseline with
`target/debug/clc` and the host C compiler under AddressSanitizer where the failure is a memory
error (probe batches 1–6, 2026-10-10; scratch files under the session temporary directory are not
retained).

| # | Repro (source shape) | Observed behavior |
| - | -------------------- | ----------------- |
| DEF-100 | `func copySelf<T>(borrow slot: T): void { slot = slot; }` called as `copySelf<String>(borrow s)`; the concrete `copySelf(borrow slot: String)`; the concrete `same(borrow p: i32)` and `same<i32>(borrow p)` | Generic `String` instance: accepted; emitted `cl_copySelf__String(ClString slot)` frees the parameter handle (`FreeString(slot); slot = NULL; slot = slot;`) — plain run `free(): double free detected` (exit 134), ASan heap-use-after-free in `cl_string_len` + double free. Concrete `String`: rejected once `E10034` at the store. Concrete/generic `i32`: accepted and runs (exit 1 / value survives) |
| DEF-101 | `func cid(borrow v: String?): String? { return v; }` with `let r: String? = cid(borrow s)`; the projected forms `return xs[0]` / `return xs[i]` through a `String[]?` slot; the generic `gid<T>(borrow v: T): T` at `String?` | Direct form: accepted; emitted `ClString cl_probe_p101a_cid(ClString *v) { … return (*v); }` — no emptying; caller and result release one handle (plain exit 134 `free(): double free`, ASan `attempting double-free`). The store form `let local: String? = v;` emits `local = (*v); (*v) = NULL;` and runs ASan-clean (exit 3). Projected forms: rejected `E10026` (with the narrowing in place; the unnarrowed spelling also reports the read path's `E10010` — the v1 element-move limit). Generic: rejected `E10034` (the borrow-generic-integrity ST-22 rule) |
| DEF-102 | `if (xs != null) { xs[i] = xs[i]; }`, `xs[0] = xs[0]`, `xs[i] = xs[j]`, `xs[0] = xs[i]`, `xs[i] = xs[0]`, `xs[1] = xs[0]` on `i32[]?` slots (i, j locals); the non-nullable `borrow xs: i32[]` variant | `xs[i] = xs[i]` and `xs[i] = xs[j]`: rejected once `E10020` (the comparison treats any two `Element(None)` projections as equal). `xs[0] = xs[0]`: rejected `E10020`. `xs[1] = xs[0]`, `xs[0] = xs[i]`, `xs[i] = xs[0]`: accepted; the constant pair emits `((*xs)).ptr[1] = ((*xs)).ptr[0];` with the read's guard and runs clean. The non-nullable Param form is accepted and emits a plain self-copy (out of the out-slot carve-out) |
| DEF-105 | `func f(borrow xs: u8[]?): i32 { xs[0] = 1; return 0; }` called with a null view; the narrowed `if (xs != null) { xs[0] = 1; }`; the read `return i32(xs[0])`; the non-null view store; a struct-field-view variant | Null store: accepted; emitted `((*xs)).ptr[0] = 1u;` — plain SEGV (exit 139), ASan SEGV WRITE to address 0. Narrowed store: accepted; runs (exit 7). Read form: rejected `E10010` ("value 'xs' may be null here"). Field-view variant: inexpressible (view-typed fields are `E10205`) |
| DEF-106 | `if (xs != null) { xs[0] = 1; }` on `slice(d, 2, 2)` (empty non-null view); the read form; the runtime-index store form; the in-bounds constant store on `slice(d, 0, 2)` | Constant store: accepted; emitted `((*xs)).ptr[0] = 1;` with no guard — plain exit 0, ASan stack-buffer-overflow WRITE of 4 bytes. Read and runtime-index store: accepted; run traps `error E11211: index out of range` (exit 134). In-bounds constant store: accepted; runs (exit 9) |

## Gaps Identified

### Gap 1: The parameter-typed lend self-assign is never judged (DEF-100)

**Current Behavior:** the generic instance at an owning type is accepted; the callee frees the
caller's handle and self-assigns it (ASan use-after-free, double free).
**Required Behavior:** the owning non-nullable instance reports `E10034` and the nullable instance
reports `E10020`, exactly as the concrete spellings do; duplicable/view instances stay accepted.
**Fix Required:** record the exact whole-binding self-assign site when the lend root's type still
mentions a parameter and judge it per instance in the existing audit structure (03-01; AR-2, AR-7).

### Gap 2: The direct slot return never empties the caller's slot (DEF-101)

**Current Behavior:** `return v` emits `return (*v);`; the caller's slot and the returned handle
alias one allocation and both release it (ASan double free).
**Required Behavior:** the return transfers the value and empties the slot, exactly as the
accepted `let local = v` store does; the caller observes a null slot.
**Fix Required:** the direct-return branch empties a caller-storage source through the existing
zeroing step before control leaves (03-02; AR-3).

### Gap 3: Element stores skip the read path's view guards (DEF-105, DEF-106)

**Current Behavior:** a store through a possibly-null view dereferences null (no `E10010`), and a
constant-index store into a slice emits no `E11211` trap (out-of-bounds write).
**Required Behavior:** the store target obeys the same null-narrowing requirement as a read, and a
constant-index slice store traps exactly as the read and runtime-index store paths do.
**Fix Required:** mirror the read path's nullable-base check over write targets, and emit the
bounds-trap statement before constant-index element stores with a dynamic bound (03-03; AR-5,
AR-6).

### Gap 4: The runtime-index projection comparison is conservative, not exact (DEF-102)

**Current Behavior:** any two runtime-indexed projections on the same out-slot root compare equal,
so distinct indices are rejected as a self-assign; the filed "unguarded" premise does not
reproduce.
**Required Behavior:** per the user's ruling, the conservative guard stays; the plan documents it
and pins the accepted and rejected pairs (AR-4).
**Fix Required:** documentation touch + specification/implementation pins only (03-01).

## Dependencies

### Internal Dependencies

- The per-instance audit pattern (`RequiresNotLend`, `RequiresDuplicable`, `RequiresOutSlotPlace`,
  `RequiresStorableSlot`) in `ir/mono.rs` + `own/instance_check.rs` (DEF-100).
- The read path's null check (`check_nonnull_expr`) and the narrowing state (DEF-105).
- The bounds guard pair (`bounds_guard_condition`, `emit_bounds_guard`) and the statement-target
  place rendering (DEF-106).
- `emit_arm_source_moves` / `emit_move_zero_place` and the `pointer_params` set (DEF-101).

### External Dependencies

- None. The container's `cc` and AddressSanitizer are the existing verification tools (AR-11).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The DEF-100 duty reports twice for one site | Low | Medium | Record only when the root still mentions a parameter and the value is the exact same binding; the concrete spelling reports through `move_place` while the self-source is skipped in `apply_binding_effect`, so no shape reports twice; the audit reports once per site (first offending instance) |
| The DEF-101  detection misses a projected caller-storage source | Low | Medium | Scope to the direct whole-slot return (projected returns are `E10026`); pin the guard tests |
| The DEF-105 walker over-reports on read-only or narrowed shapes | Medium | Medium | Mirror the read path's conditions exactly (`nullable`, non-raw, narrowing state, `unsafe` gating); the narrowed and non-null guards are ST cases |
| The DEF-106 guard changes fixed-array emissions | Low | Medium | `bounds_guard_condition` returns `None` for in-range fixed-array constants; the emission diff review covers the rest (AR-13) |
| Byte-identical pins break from the new trap line in slice stores | Expected | Low | Intended diff; the shape pins in 07 are written to the new emission |
