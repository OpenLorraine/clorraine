# Design: Nested Constant-Zero Fill Compaction

> **Document**: 03-02-nested-fill-compaction.md
> **Parent**: [Index](00-index.md)

## Current Emission

For `let a: i32[32][32] = [; [; 0]];` the emitter takes the nested-fill branch
of `emit_array_assign_to` (`compiler/src/codegen/function.rs`, currently
lines 4138-4173):

```c
i32 __cl_fillrow_1[32] = {0};
__cl_fillrow_1[0] = 0;
...
__cl_fillrow_1[31] = 0;
memcpy(&(a[0]), &(__cl_fillrow_1), sizeof(a[0]));
...
memcpy(&(a[31]), &(__cl_fillrow_1), sizeof(a[31]));
```

The row temporary is materialized because a row has no C assignment form. For
a uniform constant fill the temporary is unnecessary: every covered row
receives the same bytes, and a single `memset` writes those bytes directly.

## Proposed Emission

```c
memset(&(a[0]), 0, 32 * sizeof(a[0]));
```

One statement, no temporary, no per-row copy. The count and the row size are
compile-time constants.

For a prefix, only the suffix is compacted. The example uses 8 prefix rows of
an `i32[16][8]` so the covered region is 8 * 8 = 64 slots:

```clo
let a: i32[16][8] = [row, row, row, row, row, row, row, row; [; 0]];
```

```c
// prefix rows keep their existing stores
memset(&(a[8]), 0, 8 * sizeof(a[8]));
```

When the prefix covers the whole extent, a uniform zero fill emits nothing:
there is no temporary and no unused marker, because the fill has no side
effect.

## Eligibility

The branch compacts only when every condition holds:

| Condition | Check |
|---|---|
| Fixed nested array | the type has at least two extents and every extent, including the first, is `Some` |
| Compactable target | the existing `compact` flag is true |
| Uniform zero fill | the fill is an `ArrayLit` chain with an empty explicit-element list at every level, ending in a leaf value with width-correct constant value zero. A top-level prefix before the fill is allowed and is not part of the chain. |
| Non-float scalar leaf | the base element is `i8`, `i16`, `i32`, `u8`, `u16`, `u32`, `usize`, `isize`, or `bool` |
| Size threshold | for a non-empty covered region, `(extent - prefix) * row_slots >= 64`, computed with `checked_mul`; overflow or a missing extent rejects |
| Covered rows | `prefix >= extent` is handled before the threshold: a uniform zero fill emits nothing |

The threshold check is deliberately **not** part of the uniform-zero test.
`nested_zero_fill` returns true for a uniform zero fill with `covered_rows ==
0`, so the early return for a full prefix is reachable. The size threshold
applies only when the covered region is non-empty.

The `compact` flag is set by `fill_target_compactable` and by the caller. It is
false for a field projection, a `DerefTemp` alias, a volatile module array,
and the materialized, conditional-assignment, and self-read callers. It is
true for a local, an adjusted array parameter, a hidden return slot, a closure
environment field (`EnvField`), a compiler temporary (`Temp`), and a
hidden-return conditional arm; those name real storage, so the same byte
argument applies. The nested branch inherits the flag and adds no new target
rule.

The zero test uses `constant_fill_integer` (`codegen/expr.rs:1788`), the
restricted width-correct helper. It does not use an untyped evaluator. A
`Materialize` wrapper is followed before the `ArrayLit` match at every level.
`constant_fill_integer` rejects a float-to-integer cast, so such a fill is
excluded.

## Helper Shape

```rust
/// True when every covered slot receives the same all-zero scalar row.
fn nested_zero_fill(
    &self,
    fill: &ir::Value,
    element: &Ty,
    extents: &[Option<usize>],
    covered_rows: usize,
) -> bool
```

- Reject a float scalar leaf and a non-scalar leaf.
- Require every extent, including the first, to be `Some`.
- Call `uniform_zero_fill(fill)`; a false result rejects.
- If `covered_rows == 0`, return true (the caller emits nothing; no size
  arithmetic runs).
- Fold `extents[1..]` and the covered row count with `checked_mul`; a missing
  extent or an overflow rejects.
- Reject a covered leaf count below `FILL_LOOP_THRESHOLD`.
- Walk the fill with `uniform_zero_fill`, which follows `Materialize` before
  matching `ArrayLit` at every level:

```rust
/// True when an `ArrayLit` fill chain is a uniform constant zero.
fn uniform_zero_fill(value: &ir::Value) -> bool {
    match &value.kind {
        ValueKind::Materialize { value, .. } => uniform_zero_fill(value),
        ValueKind::ArrayLit { elements, fill } if elements.is_empty() => {
            fill.as_deref().is_some_and(|inner| {
                uniform_zero_fill(inner) || constant_fill_integer(inner) == Some(0)
            })
        }
        _ => false,
    }
}
```

The predicate is a free function so a `#[cfg(test)]` module in
`codegen/function.rs` can pin its zero, non-zero, float, `Materialize`, and
missing-element cases without a compiler instance. The eligibility helper gets
its own unit cases for 63/64 slots, a missing extent, `usize` overflow, and
`covered_rows == 0`.

## Insertion Point

Immediately before the existing row-temporary block in the nested branch:

```rust
let nested_zero = nested_zero_fill(
    fill,
    element,
    extents,
    extent.saturating_sub(prefix),
);
if prefix >= extent && nested_zero {
    return;
}
if compact && nested_zero {
    let first = target.element(prefix);
    self.push(&format!(
        "    memset({}, 0, {} * {});",
        first.address,
        extent - prefix,
        first.bytes
    ));
    return;
}
```

The eligibility helper returns true for a uniform zero fill with zero covered
rows and returns false only when a non-empty region is below the threshold.
The `prefix >= extent` return is **outside** the `compact` gate: a zero-covered
fill writes no store, so even a field, alias, volatile, or materialized target
emits nothing. The prefix stores themselves still run. A non-zero or impure
fill keeps the existing temporary and marker. IP-24 pins the non-compact full
prefix.

`ArrayTarget::element(prefix)` already produces the correct spelling for a
plain object, an adjusted array parameter, and a hidden return slot:

| Target | `element(prefix).address` | `element(prefix).bytes` |
|---|---|---|
| local `a` | `&(a[2])` | `sizeof(a[2])` |
| parameter `i32 a[N][N]` | `&(a[2])` | `sizeof(a[2])` (row size, not pointer) |
| return slot | `&(slot[2])` | `sizeof(slot[2])` |

The count uses **`first.bytes`**, where `first = target.element(prefix)`.
`target.bytes` is the whole-array byte count for an adjusted parameter and
must not be multiplied by the row count. IP-15 pins the parameter case.

## Exclusions

| Shape | Emission |
|---|---|
| One-extent array | existing path, unchanged |
| Non-zero constant nested fill | row temporary + copies, unchanged |
| Float leaf (`f32`/`f64`) | row temporary + copies; avoids the float-folding boundary |
| Struct or aggregate leaf | row temporary + copies |
| Row with explicit prefix elements (`[1, 2; 0]`) | row temporary + copies; the row is not uniform |
| Volatile target, field, `DerefTemp` alias | `compact` is false; row temporary + copies. The volatile spelling is pre-existing invalid C on the host (`memcpy` discards the qualifier); this change neither compacts nor fixes it |
| Fewer than 64 covered leaf slots | row temporary + copies; avoids a small-case call overhead regression |
| Impure fill (call, conditional) | row temporary + copies; the evaluate-once rule is unchanged |

## Safety Argument

- **Same bytes.** `memset` writes zero to every byte of each covered row. An
  integer or bool scalar has an all-zero representation for zero. No padding
  exists in an array of scalars.
- **Same extent.** The count is the number of covered rows times the row size.
  The region is contiguous by C array layout.
- **Same order.** The fill is a constant zero with no side effects and no
  operand guards. Prefix rows are stored before the `memset`, exactly as
  before.
- **No trap removal.** A constant zero fill has no division, bounds, or
  float-to-integer operation. No runtime trap becomes unreachable.
- **No ownership change.** The leaf is a scalar, so no drop runs for an
  element. The target storage is unchanged.
- **C99.** `memset` is declared in `<string.h>` in every emitted translation
  unit; the one-extent byte-fill path already calls it. `memset` accepts a
  `void *`, and `&(a[n])` is an object pointer.

## Threshold Rationale

PLAN-14 measured that a non-byte one-extent loop is slower than unrolled
stores at 16-63 slots and accepted the loop at 64 slots for code size. That
measurement is for a loop, not for a nested `memset`; 64 is reused here as a
conservative boundary, not as a measured nested-fill optimum. The nested case
also has a per-row cost (`memcpy` call and row temporary) that the one-extent
case does not, so a 64-slot floor is unlikely to regress a meaningful shape.
Matrix covers 1,024 slots. A 63-slot `i32[7][9]` keeps the old spelling; a
64-slot `i32[8][8]` compacts. The 64-slot choice is an accepted, recorded risk;
a later measurement can lower it.

## Expected Result

| Metric | Base | Head | Delta |
|---|---:|---:|---:|
| matrix net instructions | 10,959 | 9,908 | -9.6% |
| matrix m68k `-O2` `.text` | 80,712 | 76,676 | -5.0% |
| matrix emitted C bytes (exploratory) | 315,167 | 312,510 | -0.8% |
| sieve, strings, moves | unchanged | unchanged | 0% |

The absolute C-byte values are exploratory and are re-measured in Phase 1 with
the committed base compiler; the instruction and `.text` deltas are the gate.
The initial requirements are `matrix:instructions:5` and
`matrix:text:3`, declared below the probe result so a small toolchain variation
does not make the gate flaky. Phase 1 records the final requirement after the
clean baseline; a change is a recorded ruling.
