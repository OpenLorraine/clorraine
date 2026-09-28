# Current State: Array Value Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The type system already represents a fixed multi-extent array as one
`TyKind::Array { element, extents }` whose extents are all `Some`, and a slice
as the same shape with a `None` first extent (`compiler/src/types/mod.rs:217`,
`:291`). The emitter already stores array literals element by element
(`codegen/function.rs:631`), returns fixed arrays through a hidden trailing
out-parameter (`codegen/function.rs:336-353`, `:415`), and drops fixed-array
elements first to last (`codegen/drop.rs:40`). All three profiles reach the C
runtime `memcpy`: host and hosted-amiga through `<string.h>` included by their
shims, and freestanding through the shim's own definition for its `-nostdlib`
link.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/sema/stmt.rs` | Checker element type for `for ... in` | Derive the row type from the remaining extents |
| `compiler/src/own/loops.rs` | Element-loop lowering and element binding | Derive the row type; keep the `len`-based bound |
| `compiler/src/codegen/calls.rs` | `len` intrinsic rendering | Outermost extent for fixed multi-extent arrays; `String` byte count |
| `compiler/src/codegen/function.rs` | Assignment, move, save path, call emission | Whole-array byte copy; save path for arrays |
| `compiler/src/own/statements.rs` | Initializer/store preparation | Reject an element move out of an ownerless temporary (E10026) |
| `compiler/src/own/lower.rs` | Index lowering and operand materialization | Allow fixed-array call results to materialize |
| `compiler/src/codegen/guards.rs` | Guard and materialize emission | Emit a fixed-array call into its temporary |
| `compiler/src/types/mod.rs` | Shared type rules | Add `Ty::iterated_element`, used by both checkers |
| `compiler/src/own/expr.rs` | Aggregate consume points and assignment checking | Reject ownerless element moves (E10026); host the predicate |
| `compiler/src/own/call_check.rs` | Argument ownership effects | Reject ownerless element moves into `take` arguments |
| `compiler/src/own/place.rs` | Place resolution (`place_target`, `:535`) | Reused by the ownerless-element predicate |
| `compiler/src/codegen/types.rs` | C type and declarator spelling | Spell `raw T[M]` as a pointer-to-array for borrow rows |

### Code Analysis

`check_for_in` in both the checker and the ownership pass reads the array's
`element` field directly:

```rust
let element = match &iterable_ty.kind {
    TyKind::Array { element, .. } => (**element).clone(),
    _ => Ty::unknown(),
};
```

For `i32[2][3]` the base element is `i32`, so the loop binds `i32` and the real
row type is lost. The loop bound is `len(iterable)` (`own/loops.rs:489`), and
the `len` intrinsic multiplies every extent of a fixed array
(`codegen/calls.rs:158`), so `for (borrow row in m)` runs six times for a
two-row array. The element selection itself already indexes the C array
(`m[index]`), which yields the row once the types are right.

Statement assignment reaches `emit_assign` (`codegen/function.rs:577`), which
stores with `target = value;`. That is valid for scalars and structs but not
for C arrays; the fixed-array `Statement::Move` path (`codegen/function.rs`,
`Statement::Move` arm) has the same problem.

The index lowering already materializes side-effecting operands
(`own/lower.rs:521`) so the guard and the access read the call once, but that
helper declines fixed arrays because a fixed array cannot be assigned to a C
temporary — exactly the gap the assignment fix removes. The `Materialize`
guard then writes `__cl_tmp = value` (`codegen/guards.rs:100`), which for an
array-returning call must instead use the hidden out-parameter.

## Gaps Identified

### Gap 1: Multi-extent element loops bind the base element (DEF-15)

**Current Behavior:** For `m: i32[2][3]`, `for (row in m) { row[0] }` reports
`E10209: type 'i32' cannot be indexed`, and `for (borrow row in m) { }` runs
six iterations (one per scalar). A slice of rows (`xs: i32[][3]`) fails the
same way. Live probes on 2026-09-28 (host profile): both forms reproduce.

**Required Behavior:** The element is the row (`i32[3]`); the loop runs once
per row (`spec/08` AR-3, section 5, LOOP-1/2/3).

**Fix Required:** Derive the element type by stripping the first extent in
`own/loops.rs` and `sema/stmt.rs`; make `len` of a fixed multi-extent array the
outermost extent (`03-01`, `03-02`).

### Gap 2: Fixed-array value assignment emits invalid C (DEF-22)

**Current Behavior:** `let b: i32[2] = a;`, `let r: i32[3] = m[0];`,
`let c: i32[2] = s.v;`, a whole-array move, and `a = a;` are accepted and then
fail the C compiler with "assignment to expression with array type".

**Required Behavior:** Copy emits a byte copy; move emits a byte copy plus a
zeroed source; reassignment drops the old value first; a self-referencing
assignment saves the old value first (`spec/03` section 3, O3, V8/V9).

**Fix Required:** A fixed-array store path in `codegen/function.rs` used by
assign, move, and the save path; a checker-side E10026 for element moves out
of ownerless temporaries (`03-01`).

### Gap 3: Projections over array-returning calls emit invalid C (DEF-21)

**Current Behavior:** `let x: i32 = mk()[1];`, `vlen(mk()[1])`, and
`let s: String = mk()[0];` are accepted and emit `cl_mk()[1]`; the C compiler
reports "too few arguments to function".

**Required Behavior:** The call runs once into a compiler temporary and the
projection reads or lends the element; moving a non-duplicable element out
stays E10026 (`spec/03` section 4).

**Fix Required:** Lift the fixed-array exclusion in the materialize helpers
and emit the fixed-array call into its temporary (`03-02`).

### Gap 4: `len` is wrong for `String` and fixed multi-extent arrays (DEF-19, AR-6)

**Current Behavior:** `len(s)` for `s: String` compiles and returns the
literal `0` (the intrinsic falls through to `_ => "0"`,
`codegen/calls.rs:166`). `len(m)` for `m: i32[2][3]` returns `6` (the extent
product) instead of `2`.

**Required Behavior:** `String` yields its byte count through the implicit
`str` widening; a fixed multi-extent array yields its outermost element count
(`spec/02` section 9, `spec/04` sections 9 and 11).

**Fix Required:** Add the `String` and outer-extent arms to the `len`
intrinsic (`03-02`).

## Dependencies

### Internal Dependencies

- The C runtime `memcpy` on all three profiles: host and hosted-amiga through
  `<string.h>` included by their shims, and freestanding through the shim's
  own definition (`shim_freestanding.c:29`).
- `emit_drop` / `emit_fixed_array_drop` (`codegen/drop.rs:21`, `:40`) for the
  reassignment drop.
- The hidden out-parameter call path (`codegen/function.rs:415`).
- `ValueKind::Materialize` (`ir.rs:248`) and its guard emission
  (`codegen/guards.rs:100`, `:262`).

### External Dependencies

- None.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The deferred materialize path must stay correct inside short-circuit operands | Medium | High | A spec case pins that a call inside a false `&&` operand does not run; the guard emission chooses a comma-expression form |
| Move semantics for arrays must not double-own or leak | Medium | High | ASan close-out probes plus move/use-after-move spec cases |
| `len` change could alter the element-loop bound for existing single-extent programs | Low | Medium | A regression spec case pins single-extent, slice, and growable-array lengths |
| A fixed-array call reassignment must drop the old value and stay alias-safe | Medium | High | Call into a temporary, then move (task 1.2.1); counting-deleter probes in close-out |
| The row address temporary needs a pointer-to-array C declaration | High | Medium | Extended `declarator`/`c_type` (task 2.2.1); an implementation test pins the declaration |
| Materialized owning temporary lifetime is a recorded limitation (DEF-23) | High | Medium | Limitation documented in `01`, `03-02`, and `07`; follow-up defect filed |
| vbcc/hosted-amiga byte-copy emission differences | Low | Medium | `make verify` covers the cross-toolchain legs |
