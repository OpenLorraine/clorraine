# Generated-C Changes: Generated-C Optimization

> **Document**: 03-02-generated-c-changes.md
> **Parent**: [Index](00-index.md)

## Overview

Four local changes, each selected by the baseline in
[Current State](02-current-state.md) and bounded by the requirements in
[Requirements](01-requirements.md). None changes the language, the ABI, the
runtime shims, or a frozen golden.

| Change | Owner requirement | Measured baseline |
|---|---|---|
| R4 scalar module `const` read folding | R4 | sieve 3,044,035 → 2,165,514 instructions (−28.9%) |
| R5 compact large one-extent scalar fills | R5 | sieve C 489,491 → 310,413 bytes; `.text` 101,396 → 76,768 |
| R6 non-negative slice/string constant guard | R6 | `s[1]` emits `(1 < 0)`; dead condition |
| R7 constant division/remainder guard | R7 | `x % 2` emits `2 == 0`; dead condition |

## R4 — Scalar Module `const` Folding

### Current behavior

`ExprKind::Name` and `ExprKind::Qualified` lower to `ValueKind::Place` in
`compiler/src/own/lower.rs::lower_value`. A module `const` is a file-scope
mutable `static` assigned in `cl_init_<module>`; every read loads it. In a loop
whose bound is a const, the C compiler cannot constant-propagate the bound, so
the index guards remain.

### Proposed behavior

Before a bare global read becomes a `Place`, resolve the place target. If it is
a bare `PlaceTarget::Global(id, [])`, the global is a `const`, its initializer
folds, and its type is a supported scalar, lower the read to a typed literal.

### Address-taken pre-pass

`ProgramInfo::collect` (`compiler/src/own/program.rs`) gains
`address_taken_consts: BTreeSet<ir::GlobalId>`. A walker visits every
body-bearing item (`Item::Func` and `Item::Comptime`), every global initializer
(`Item::GlobalLet`, `Item::GlobalConst`), and every lambda body. Inside a
body it visits every statement form (`let`, `const`, expression, `if`,
`while`, `do`, three-clause `for`, element `for`, `return`, `break`,
`continue`, `switch`, `defer`, `unsafe`, bare block) and every expression
child (`call` callee and arguments, index, member, unary, binary, assignment,
conditional, cast, raw cast, struct literal, array literal, lambda, unsafe,
intrinsic argument, template placeholder). At each
`ExprKind::Unary { op: AddressOf, operand }`:

- a `Name` operand resolves with
  `init_graph::resolve_name_global(loaded, module, name)`;
- a `Qualified` operand resolves with
  `init_graph::resolve_global(loaded, module, name)`;
- a projected operand resolves its root when the root is a `Name`/`Qualified`;
- the resolved `GlobalRef` becomes `ir::GlobalId::new(module, name)` and is
  inserted when the global is a const.

This is the only guard for the accepted `unsafe &N` form; a walker miss is a
correctness defect, not just a lost optimization. The implementation must be
exhaustive over the AST and is tested with nested addresses in a call
argument, a lambda, a `defer`, and a module initializer. The pass adds one AST
walk at program collection, not a dataflow analysis.

### Scope-gated fold

The fold runs only when all of these hold:

1. The expression is a bare `Name`/`Qualified` with no field or element
   projection.
2. `place_target(expr)` is `PlaceTarget::Global(id, projections)` with
   `projections.is_empty()`.
3. `program.globals[id].is_const` is true.
4. `!program.address_taken_consts.contains(id)`.
5. The global has a recorded const initializer expression.
6. The expression type is a supported scalar: `i8`, `i16`, `i32`, `u8`, `u16`,
   `u32`, `usize`, `isize`, or `bool`. Enums, `raw`, views, floats,
   aggregates, and handles are rejected.
7. The typed fold succeeds: `ConstFolder::integer_in(initializer, &ty)` for an
   integer or `bool`. A `f32`/`f64` const is not folded; the folder evaluates
   floats in `f64` and does not round an `f32` cast at each operation.
8. Every arithmetic operand in the initializer has an integer type of at least
   32 bits. A narrower operation is promoted to `int` in C but wrapped at the
   operand width by the folder, so folding it would disagree with the stored
   value. A direct narrow literal or cast is still eligible.
9. No subexpression has type `f32`/`f64`. An integer const through an `f32`
   cast is not stable because the folder does not round that cast.

A local or parameter that shadows the name resolves to a `Local` target and is
not folded. A field read resolves to a projection and is not folded.

### Declaring-module evaluation

The initializer is evaluated in the module that declares the const:

```rust
let declaring = self.loaded.modules.get(&id.module)?;
let mut folder = ConstFolder::new(&self.sema.table, declaring, self.loaded);
let value = match integer_scalar(&ty) {
    Some(_) => folder.integer_in(initializer, &ty)?,
    None => folder.fold(initializer).ok()?,
};
```

`integer_in` folds with the const's declared integer width, so `1 << 31`
masks/wraps exactly as the runtime does and `1 << 32` masks its count instead
of wrapping the result. This makes `const A = 5; const B = A;` fold in module
`m` even when `B` is read from module `n`. It does not change the pre-existing
extent folder behavior; extents are already folded by the sema/type stage.

### Typed literal construction

| Folded value | IR value | Rendering |
|---|---|---|
| `Int(n)`, `n >= 0` | `ValueKind::Int(n)` with the const type | existing literal suffix/cast |
| `Int(n)`, `n < 0` | `ValueKind::Unary { Neg, Int(-n) }` with the const type | `(i32)(-(u32)1)` and the other signed-wrap forms |
| `Bool(b)` | `ValueKind::Bool(b)` | `0`/`1` |
| `Float(f)` (f32/f64) | no fold | the place stays; the folder does not round f32 arithmetic at each step |
| a narrow arithmetic initializer | no fold | the place stays; its C storage promotes to `int`, the folder wraps at the operand width |
| `Str` | no fold | the place stays |

The value's `ty` is the expression's checked type; a surrounding cast or
narrowing stays in the IR. The negative form uses the existing unary
negation's wrapping rule, so `i32::MIN` and `i8::MIN` render correctly.

### Storage boundary

The declaration and `cl_init_<module>` assignment are unchanged. The symbol is
still emitted, so an accepted `&N` still has storage. `lower_global` (module
initializers) is unchanged; R4.7 excludes it.

### Rejected alternatives

| Alternative | Reason |
|---|---|
| Make the const `static const` / remove the initializer | Separate storage/init/ABI-adjacent change; breaks accepted raw writes |
| Fold every const read including module initializers | No measured need; separate lowering path |
| Use the caller's module for the folder | Wrong for imported initializer chains |
| Fold aggregate/string consts | Different lowering; no measured need |
| Reject `&N` with a new diagnostic | Language/diagnostic change, out of scope |

## R5 — Compact Large One-Extent Scalar Fills

### Current behavior

`compiler/src/codegen/function.rs::emit_array_assign_to` emits one store per
covered slot for a fill. `u8[8192] = [; 1]` produces 8,192 statements.

### Proposed behavior

Add one named constant:

```rust
const FILL_COMPACT_THRESHOLD: usize = 16;
```

After the prefix stores, let `covered = extent - prefix`. When
`covered >= FILL_COMPACT_THRESHOLD` and the element is a scalar (one extent,
statement form), emit one of:

- **`u8`/`i8` constant fill** (at least 16 covered slots):
  `memset(&(target.text[prefix]), value, covered);` The indexed address is
  valid for an array object, an adjusted parameter, and a hidden return slot.
  The element is one byte by definition, so no `sizeof` is needed. A prefix
  starts the `memset` at the first covered slot, not at index zero.
- **any other scalar fill** (at least 64 covered slots, including a
  non-constant `u8`/`i8` fill and `bool`): a C block with one
  `usize` index local and one `for` loop from `prefix` to `extent`, storing
  the existing lowered value. For an impure fill or a store-needing literal,
  the loop copies the existing once-evaluated temporary; for a pure fill, the
  loop repeats the pure expression, exactly as the unrolled form did. The
  higher threshold keeps a 16–63-slot non-byte fill unrolled, because the
  loop is slower than the unrolled stores at that size.

`emit_array_assign_to` takes an explicit `compact` flag. `emit_array_assign`
sets it true only for a plain local, adjusted parameter, or return slot; it is
false for a field projection (which may name a `volatile` member), a
`volatile` module array, and a `DerefTemp` loop alias. The materialized
fixed-array fills, conditional arms, struct-literal fields, and the shared
self-read path pass false, so those forms keep their current spelling. A fill
whose expression reads a named binding takes the shared self-read path and
stays unrolled. A non-scalar element (including a simple struct literal that
falls through to the generic fill) never uses the compact path. Below the
threshold, the existing spelling is unchanged.

### Invariants

- The fill expression is evaluated once when it has effects. An impure fill
  (a call or an intrinsic such as `readVolatile8`) is evaluated once into the
  existing `__cl_fill` temporary and the loop copies it. A pure fill has no
  effect to repeat; a pure volatile *place* read is repeated per element,
  exactly as the unrolled form did.
- The prefix stores, move-outs, and cleanup order are unchanged.
- The loop local is `self.unique_local("__cl_fill")`, so it cannot collide
  with a source name or another generated name.
- The loop is one C statement inside the source statement's `#line` mapping;
  the cleanup ladder is unaffected.
- The target can be an array object, an adjusted array parameter, or a hidden
  fixed-array return slot. For a parameter, the byte count is
  `covered * sizeof(element)`, never `sizeof(pointer)`; the `u8`/`i8` memset
  path needs no size.
- No nested-row, struct-literal, array-copy, or deferred expression form is
  changed.

### Rejected alternatives

| Alternative | Reason |
|---|---|
| Compact every fill, including nested rows | No baseline; the nested path already uses a row temp and one `memcpy` per row |
| `memset` for `bool` | `bool` is not byte-sized on the target |
| A runtime fill helper | New runtime surface and a call per fill |
| Leave the fill unrolled | 24.3% of `.text` and 36.6% of C bytes on the measured program |

## R6 — Non-Negative Slice/String Constant Guard

### Current behavior

`compiler/src/codegen/render.rs::bounds_condition` omits the `< 0` half when
the index type is unsigned. A non-negative `i32` literal into a slice or
`str` is typed signed, so it still emits `(1 < 0)`. A fixed-array in-range
constant is already unguarded in `bounds_guard_condition`.

### Proposed behavior

In `compiler/src/codegen/guards.rs::bounds_guard_condition`, when the index is
signed, call a shared `constant_guard_integer(index)`. If it is `Some(n)` with
`n >= 0`, build the unsigned condition for that index. The upper-bound
comparison remains. `constant_guard_integer` accepts a literal, `bool`,
character, cast, unary, or binary form evaluated with the C semantics: the constant evaluator wraps each operation at its result
type and masks shift counts at the left operand's width, so a wrapping binary
index is classified correctly. A non-constant expression keeps the full
guard.

### Invariants

- A negative constant keeps the signed condition. A negative constant into a
  fixed array is rejected by the existing semantic check; a negative constant
  into a slice or `str` is accepted and remains a runtime `E11211` trap.
- An unsigned index and a fixed-array constant index are unchanged.
- No upper-bound analysis, loop-bound analysis, or fact database is added.
- The trap code, message, and exit status are unchanged.

### Rejected alternatives

| Alternative | Reason |
|---|---|
| Remove the upper check by hand or with a range analysis | Measured m68k regression (2,165,514 → 2,436,214) |
| Re-implement the unsigned/fixed-array cases | Already correct; no red test |
| Add a compile-time rejection for slice/string negative literals | New diagnostic/language change |

## R7 — Constant Division and Remainder Guard

### Current behavior

`compiler/src/codegen/guards.rs::division_guard_condition` emits
`d == 0 || (a == MIN && d == -1)` for a signed divisor and `d == 0` for an
unsigned one. A constant divisor still emits the constant-false disjuncts.

### Proposed behavior

After the integer check, call `constant_guard_integer(right)`:

| Constant divisor | Emitted guard |
|---|---|
| `0` | none; the value renderer's trap expression is unchanged, including a binary or finite-float-cast zero |
| `-1`, signed dividend | `dividend == MIN` only |
| `-1`, unsigned dividend | none |
| any other non-zero | none |
| non-constant | the existing full guard |

The helper sees through `Unary(Neg)`, casts, a folded const, and a binary
expression evaluated with C semantics. A binary divisor such as
`i32(2147483647) + i32(2147483647) + i32(1)` evaluates to the C `-1`, so the
`MIN / -1` check is emitted; a shift divisor masks its count, so `V >> 8` is
not mistaken for zero. The value renderer uses the same zero classification
for its trap expression.

### Invariants

- A literal zero still traps through the existing value renderer; no second
  `if` guard is added.
- `MIN / -1` still traps for a signed dividend.
- A run-time divisor keeps the full guard, including the zero check.
- The trap code, message, and exit status are unchanged.

### Rejected alternatives

| Alternative | Reason |
|---|---|
| Leave the dead disjuncts | Every constant division carries a constant-false branch in the emitted C |
| Add an explicit `if` guard for literal zero | Duplicates the existing trap expression |

## Cross-Cutting Invariants

- No change under `spec/`; no new diagnostic; no ABI, symbol, struct layout,
  handle, profile, stdlib, or runtime-shim change.
- Same source, profile, and flags produce byte-identical C. The new loop local
  uses the existing stable `unique_local` counter.
- Every emitted form is C99 and compiles for the host, m68k GCC (nix20/nix13),
  vbcc, and freestanding where the shape can appear.
- The base-versus-head corpus diff attributes every change to R4–R7.

## Error Handling

| Case | Handling |
|---|---|
| Const initializer does not fold | Keep the place; no diagnostic change |
| Const type is not a supported scalar | Keep the place |
| Const address is taken | Keep the place everywhere |
| Fill below the threshold | Existing per-element spelling |
| Fill target is an adjusted parameter | Use the element-count byte size |
| Slice/string negative constant | Keep the signed guard and the runtime trap |
| Constant zero divisor | Keep the existing trap expression |
| Non-constant divisor | Keep the full guard |

## Testing Requirements

The behavior and emission cases are owned by
[Testing Strategy](07-testing-strategy.md). The key proof obligations are the
address-taken const case, the shadowing local, the imported/expression
initializer, negative and narrow-arithmetic constants, the 15/16 and 16/64
threshold boundaries, the
adjusted-parameter byte count, the volatile fill count, the slice/string
negative trap, and the literal zero/`-1` division traps.
