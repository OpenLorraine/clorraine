# Current State: Ownership and Drop Model Completion

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The ownership and borrow model is rule-complete against the frozen chapters 03
and 16 for everything it models; the gaps are an array *shape* mistake, a
missing array drop arm, and one incomplete field check. The relevant code:

- `compiler/src/types/mod.rs:33-76` defines `TyKind::Array { element, extents }`,
  where an omitted extent (`None`) marks a slice and every extent present marks a
  fixed array.
- `compiler/src/borrow.rs:151-186` classifies a type as a view, an elision view,
  and a lend candidate. All three predicates match the whole `TyKind::Array`.
- `compiler/src/types/table.rs:367-385` derives duplicability, recursing into an
  array element for every `TyKind::Array`.
- `compiler/src/own/expr.rs:157-164` evaluates array-literal elements but does
  not consume them; `compiler/src/own/lower.rs:170-175` lowers them without
  `consume_place`. The struct-literal paths do both (`own/expr.rs:151-156`,
  `own/lower.rs:158-168`).
- `compiler/src/codegen/drop.rs:32-63` emits the drop ladder: a handle releases
  through its deleter, a struct recurses into its fields, and everything else is
  a no-op. There is no array arm.
- `compiler/src/sema/mod.rs:266-289` rejects an `Array` whose element is not
  duplicable with `E90000`.
- `compiler/src/sema/items.rs:35-44` rejects a `TyKind::Str` struct field with
  `E10205`.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/types/mod.rs` | Type shapes and predicates | Add `Ty::is_slice` / `Ty::is_fixed_array` |
| `compiler/src/types/table.rs` | Duplicability, size | Slice short-circuit in `duplicable_at` |
| `compiler/src/borrow.rs` | View/elision/nullability classification | Route the view, elision, and nullability predicates through `is_slice`; keep `is_lend_candidate` unchanged |
| `compiler/src/own/expr.rs` | Ownership walk | Consume array-literal elements |
| `compiler/src/own/lower.rs` | IR lowering | `consume_place` array-literal elements |
| `compiler/src/codegen/drop.rs` | Drop ladder | Fixed-array (per dimension) and growable-`Array` arms |
| `compiler/src/codegen.rs` | Function signatures | By-value fixed-array return signature (03-04) |
| `compiler/src/codegen/function.rs` | Returns and array assignment | Out-parameter return (03-04); per-dimension literal emission (03-05); no scratch arm (03-02) |
| `compiler/src/codegen/calls.rs` | Call lowering | Pass the fixed-array result destination (03-04) |
| `stdlib/std.mem.clo` | `Array<T>::set` | Drop the replaced element (R10) |
| `compiler/src/sema/tests.rs` | Unit tests | Update the `E90000` unit test (PF-005) |
| `compiler/src/sema/mod.rs` | Statement/type checks | Remove the `E90000` trigger |
| `compiler/src/sema/items.rs` | Struct checks | Extend `E10205` to slices |
| `compiler/src/sema/expr.rs` | Expression checks | `E10215` guard for a move-only fill |
| `spec/08-arrays-strings.md` | Array chapter | Extend the `AL` fill rule (AR-10) |
| `spec/14-diagnostics.md` | Diagnostics registry | Extend `E10215` text (AR-20) |
| `spec/tests/staged/` | Staged test data | Remove the `E90000` golden and its README row (AR-3) |

### Code Analysis

The three defects were reproduced against the emitted C during planning:

- A fixed-array local of `String` emits `arr[0] = a; arr[1] = b;` and then
  `FreeString(a); FreeString(b);`, and never drops `arr` — two aliases of freed
  handles (defect F1).
- `func pair(): String[2]` is rejected with `E10036` (view return does not derive
  from exactly one parameter), because a fixed array is an elision view (F1).
- `struct Bad { xs: u8[]; }` is accepted and emits `struct ClBad { ClSlice_u8 xs; }`,
  a view stored in a struct (defect F2).
- `Array<Buffer>` is rejected with `E90000` (defect F3).
- `let arr: String[2] = [; a];` emits `arr[0] = a; arr[1] = a;` and then
  `FreeString(a)` — a repeated move-only value (defect F4, AR-10).

## Gaps Identified

### Gap 1: A fixed array is classified as a view (F1)

**Current Behavior:** `is_view`, `is_elided_view`, `is_lend_candidate`, and
`duplicable_at` treat every `TyKind::Array` as a view.
**Required Behavior:** an array with every extent present is an owned aggregate;
an array with an omitted extent is a view (`spec/08-arrays-strings.md` sections
1-3).
**Fix Required:** one `Ty::is_slice` predicate, used by every consumer (R1, R2).

### Gap 2: Owning-element drop glue for arrays is missing (F1, F3)

**Current Behavior:** the drop ladder has no array arm; a fixed array is never
dropped and a growable `Array<move-only>` is staged as `E90000`.
**Required Behavior:** a fixed array drops its elements first-to-last; a growable
`Array<T>` drops its elements first-to-last then frees the buffer
(O2; `spec/appendix-stdlib.md` section 3).
**Fix Required:** an array arm in the drop ladder and the removal of the `E90000`
trigger (R5, R6).

### Gap 3: A slice view field is accepted (F2)

**Current Behavior:** `check_struct` rejects only `TyKind::Str`.
**Required Behavior:** every view-typed field except `static str` is rejected;
`cstring` stays allowed by T4 (`spec/08-arrays-strings.md:289`).
**Fix Required:** extend the check to a slice field (R8).

### Gap 4: A fill literal of a move-only element is unsound (F4)

**Current Behavior:** `[; a]` for `String[2]` repeats the handle and frees it.
**Required Behavior:** the fill form cannot repeat a move-only value; it is
rejected (AR-10).
**Fix Required:** an `E10215` guard in the fill path (R7).

### Gap 5: A fixed-array return cannot be emitted (R3)

**Current Behavior:** the checker rejects `func pair(): String[2]` with `E10036`
(a fixed array is an elision view). If it were allowed, the emitter would spell
the return type with `c_type` (the element type) and C cannot return an array.
**Required Behavior:** the function returns by value (`spec/08` section 6).
**Fix Required:** the by-value lowering in [03-04](03-04-fixed-array-return.md)
(PF-001); classification alone is not enough.

### Gap 6: Nested array-literal emission is invalid C (R4, R5)

**Current Behavior:** `u8[3][2] = [[1,2],[3,4],[5,6]]` emits
`d[0] = { 1, 2 };`, which C rejects.
**Required Behavior:** each dimension assigns its own elements.
**Fix Required:** the per-dimension recursion in
[03-05](03-05-nested-array-literals.md) (PF-003); it unblocks the
multi-dimensional drop test.

### Gap 7: A fixed array can still be nullable (R1)

**Current Behavior:** `is_nullable_capable` (`borrow.rs:203-214`) accepts
`Array{..}`, so `let a: String[2]? = null;` type-checks and emits invalid C.
**Required Behavior:** a fixed array is an owned aggregate and is not nullable.
**Fix Required:** route `is_nullable_capable` through `is_slice` (PF-004).

### Gap 8: Un-staging `Array<move-only>` exposes `set`'s leak (R6)

**Current Behavior:** `xs.set(i, v)` writes `one[0] = v;` and never releases the
old element (`stdlib/std.mem.clo:97-102`), which leaks a handle once a
move-only element is allowed.
**Required Behavior:** `set` replaces element `i`, dropping the old one
(`spec/appendix-stdlib.md:62`).
**Fix Required:** drop the old element in `set` (R10, PF-007).

## Known Pre-existing Issues (recorded, not introduced)

- The runtime bounds guard for a multi-dimensional index flattens `grid[i][0]` to
  `i >= 4` (`codegen/guards.rs:331-337`) while the constant check and C use row
  semantics (`grid[2]` is `E10216`). This plan fixes the drop path's indexing
  (PF-002) and records this guard inconsistency as a separate pre-existing issue
  (PF-012).
- Whole fixed-array assignment (`b = a;`) emits invalid C and stays out of scope
  (PF-009).

## Dependencies

### Internal Dependencies

- `Ty::is_slice` (R1) is used by R2, R5, R8.
- The drop arm (R5, R6) depends on the classification (R1, R2) because
  `needs_drop` consults `is_view`.
- The fill guard (R7) is independent of the others but shares the array-literal
  lowering touched by R4.

### External Dependencies

None. The growable drop loop reuses the existing `cl_buffer_ptr` FFI in the
runtime shims (`compiler/runtime/shim_host.c:119`).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| A fixed-array return needs codegen work beyond classification | High | High | designed in [03-04](03-04-fixed-array-return.md) and given its own tasks; verified by a build-and-run test |
| The multi-dimensional drop needs nested-literal codegen | High | Medium | designed in [03-05](03-05-nested-array-literals.md) and given its own task |
| Removing the `E90000` trigger leaves an untested code | Low | Low | AR-3 keeps it as a documented dormant fallback; `spec/check.sh` keeps it excluded; the `sema/tests.rs` unit test is updated (PF-005) |
| The `set` change alters a standard-library body | Low | Medium | covered by a build-and-run double-free/leak test (R10) |
| Growable `Array<T>` element drop re-evaluates `cl_buffer_ptr` per element | Low | Low | The loop reads the buffer pointer once into a temporary before the loop |
| An array-literal change regresses an existing program | Low | High | `make verify` and the corpus are the regression gate (R9) |
