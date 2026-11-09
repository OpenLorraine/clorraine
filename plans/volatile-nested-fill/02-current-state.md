# Current State and Root Cause (PLAN-23)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Repository State

| Item | Value |
|---|---|
| Base commit | `7cdbb71` (`docs(plans): close the nested fill plan`) |
| `origin/main..HEAD` | 0 |
| Tracked tree | clean; the `plans/volatile-nested-fill/` set is untracked until the Phase 0 commit |
| Latest tag | `v1.1.0` |
| Roadmap | 566/568 done, 0 open, 2 parked |

## Reproduction

### Minimal volatile module array

```clorraine
module probe.NestedFill;
volatile let g: i32[8][9] = [; [; 0]];
func main(): i32 {
    g = [; [; 0]];
    return g[0][0];
}
```

`target/debug/clc --profile host` accepts the program. The function body emits:

```c
i32 __cl_fillrow_1[9] = {0};
__cl_fillrow_1[0] = 0;
...
memcpy(&(cl_probe_NestedFill_g[0]), &(__cl_fillrow_1), sizeof(cl_probe_NestedFill_g[0]));
...
memcpy(&(cl_probe_NestedFill_g[7]), &(__cl_fillrow_1), sizeof(cl_probe_NestedFill_g[7]));
```

Strict host C rejects every row copy:

```
error: passing argument 1 of 'memcpy' discards 'volatile' qualifier
from pointer target type [-Werror=discarded-array-qualifiers]
```

The module-level initializer is already valid: `emit_global_array_fill` writes
`g[r][c] = 0;` element by element. Only the function-body nested fill is
affected.

### Volatile overlay field

```clorraine
module probe.OverlayField;
struct S { a: volatile i32[2][3]; }
func main(): i32 {
    let regs: raw S = raw S(0x00DFF000);
    unsafe {
        regs.a = [; [; 0]];
    }
    return 0;
}
```

This is the overlay shape the specification permits (access through a `raw`
pointer inside `unsafe`). It emits `memcpy(&(regs->a[0]), ...)` and fails the
same strict build.

### Prefix rows

A nested fill can carry explicit rows before the fill. A literal prefix row is
already stored element by element, but a place-valued prefix row is delegated
to the shared fixed-array store core, which emits a whole-row `memcpy` into the
volatile sub-target:

```clorraine
volatile let g: i32[4][4] = [; [; 0]];
func main(): i32 {
    let row: i32[4] = [; 1];
    g = [row, row; [; 0]];
    return g[0][0];
}
```

```c
memcpy(&(cl_probe_PrefixPlace_g[0]), &(row), sizeof(cl_probe_PrefixPlace_g[0]));   // row place, invalid
memcpy(&(cl_probe_PrefixPlace_g[1]), &(row), sizeof(cl_probe_PrefixPlace_g[1]));   // row place, invalid
...
memcpy(&(cl_probe_PrefixPlace_g[2]), &(__cl_fillrow_1), ...); // fill row, invalid
memcpy(&(cl_probe_PrefixPlace_g[3]), &(__cl_fillrow_1), ...); // fill row, invalid
```

Both copy sites are in the nested fill path and are in scope. The prefix row
is fixed with a dedicated wrapper, because the shared fixed-array store core
also serves whole-array copies that are not fills.

### A field named `len`

`len` is a structural member on a fixed array, but it is also a legal struct
field name. A volatile field named `len` must be tested as a field, not folded
to the array length:

```clorraine
struct S { len: volatile i32[2][3]; }
func main(): i32 {
    let regs: raw S = raw S(0x00DFF000);
    unsafe { regs.len = [; [; 0]]; }
    return 0;
}
```

The predicate must mirror `place()`: fold `.len` only when the current type is
a fixed array.

### Excluded shapes

| Shape | Emission | Strict C | Disposition |
|---|---|---|---|
| Local `i32[8][8]` zero fill | one `memset` | valid | unchanged |
| Non-volatile field nested fill | row temporary + `memcpy` | valid | unchanged |
| Volatile one-extent `u8` fill | per-element stores | valid | unchanged |
| Volatile explicit rows `[[...], [...]]` | per-element stores | valid | unchanged |
| Volatile full-prefix zero fill | nothing | valid | unchanged |
| Whole-array copy `g = other` | `memcpy(&(cl_..._g), ...)` | rejected | separate known limitation; emission pinned by IP-V12 |
| Struct-literal prefix with a fixed-array field | `memcpy` into the field | rejected | separate known limitation |
| Conditional, call, or closure-call prefix row | the wrapper falls back to the shared core, which may `memcpy` | rejected | separate known limitation |
| `for borrow row in g` or `in regs.a` | borrow conversion discards the qualifier before the fill | rejected | separate limitation; the fill is never reached |
| `borrow` parameter bound to a volatile field (`fill(regs.a)`) | the call argument discards the qualifier | rejected | separate limitation; the callee's fill is not reached through a volatile type |
| `borrow` parameter bound to a volatile global | checker rejects the borrow (`E10038`) | n/a | not representable |
| Owning explicit row copy (no fill) | shared core may `memcpy` into a volatile field | possible | separate shared-core limitation; a move-only fill is rejected (`E10215`) and a module-level `volatile let` requires a duplicable type (`E11106`) |

## Root Cause

`emit_array_assign` computes one `compact` flag from `fill_target_compactable`.
That predicate returns false for a field projection, a `DerefTemp` alias, or a
volatile module array, but a false flag means only "do not compact". Two copy
sites then always use `memcpy`:

1. The nested fill branch creates a row temporary and copies each fill row
   (`compiler/src/codegen/function.rs:4256-4283`).
2. A place-valued prefix row is delegated to `emit_fixed_array_assign_to`,
   whose duplicable final branch copies the whole row
   (`compiler/src/codegen/function.rs:2126-2149`). That branch is shared with
   whole-array copies (`g = other`), so the fix uses a dedicated prefix-row
   wrapper instead of changing the shared core.

`ArrayTarget` carries the C name, address, and byte count, but no volatility
signal, so neither copy can distinguish a volatile destination.

## Affected Code

| Item | Location |
|---|---|
| Fill target entry | `compiler/src/codegen/function.rs:4062` (`emit_array_assign`) |
| Compact gate | `compiler/src/codegen/function.rs:4080` (`fill_target_compactable`) |
| Nested fill branch | `compiler/src/codegen/function.rs:4135` (`emit_array_assign_to`) |
| Fill-row copy loop | `compiler/src/codegen/function.rs:4256-4283` |
| Prefix-row store | `compiler/src/codegen/function.rs:4157-4162` -> `:2126-2149` |
| Array target | `compiler/src/codegen/function.rs:4468`, `:4561` |
| Existing volatile pin | `compiler/tests/nested_fill_compaction_impl_test.rs:260` |
| Shape fixture | `tools/perf/fixtures/shapes.clo` |

## Existing Evidence

- The nested fill closeout recorded the volatile invalid C as an accepted
  limit: `plans/nested-fill-compaction/05-findings.md`.
- Base and head emitted byte-identical `memcpy` spellings for the volatile
  shape, so the invalid C predates the nested fill compaction at `b68dae5`.
- The v1.1.0 release did not include a strict-build volatile nested-fill
  fixture, so the defect shipped.

## Residual Risk

| Risk | Control |
|---|---|
| A volatility signal is missed for a reachable in-scope shape | Predicate covers global bases and volatile fields through raw pointers; ST/IP cases cover both plus a field named `len` |
| The two copy sites diverge | Both call the same `emit_volatile_array_copy` helper; IP-V1 and IP-V5 pin them |
| The copy changes non-volatile emission | The flag defaults to false; IP-V7 and IP-V8 pin the old spelling |
| A struct-leaf copy is mishandled | ST-V3 fills a volatile array of plain structs |
| A prefix place row is missed | ST-V5 and IP-V5 pin the place row and the fill rows |
| Out-of-scope volatile copies are mistaken for fixed | The known limitations list each remaining shape and its separate cause |
