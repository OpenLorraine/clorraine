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

An impure prefix row is a separate hazard: the first wrapper rendered the
value once to a C expression and appended an element index per leaf, so a call
hidden under a member or index ran once per leaf. The final wrapper evaluates
an impure value once into a row temporary, then copies from it.

### Run-time indexed elements

A whole-element target with a run-time index is lowered to a pointer store:

```clorraine
volatile let g: i32[2][3][4] = [; [; [; 0]]];
func main(): i32 {
    let i: i32 = 1;
    g[i] = [; [; 9]];
    return g[1][0][0];
}
```

The pointer expression is an address-of value, not a `Place`, so the original
predicate did not see the volatile base. The emitted C copied with `memcpy`
into `volatile i32 (*)[4]` and failed the strict build. The final fix derives
the qualifier from the addressed value tree and copies leaf by leaf.

### A volatile pointer field

`struct S { p: volatile raw Inner; }` is emitted as `volatile Inner *p`, so the
pointee is volatile. A member behind the pointer is volatile in the emitted C
and takes the per-leaf copy. This is consistent with the C spelling, not a
false positive.

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
| Volatile array used as a row-temporary copy source | `memcpy` from the volatile source | rejected | separate pre-existing shared-core limitation |
| Struct-literal prefix with a fixed-array field | `memcpy` into the field | rejected | separate known limitation |
| `for borrow row in g` or `in regs.a` | borrow conversion discards the qualifier before the fill | rejected | separate limitation; the fill is never reached |
| `borrow` parameter bound to a volatile field (`fill(regs.a)`) | the call argument discards the qualifier | rejected | separate limitation; the callee's fill is not reached through a volatile type |
| `borrow` parameter bound to a volatile global | checker rejects the borrow (`E10038`) | n/a | not representable |
| Owning explicit row copy (no fill) | shared core may `memcpy` into a volatile field | possible | separate shared-core limitation; a move-only fill is rejected (`E10215`) and a module-level `volatile let` requires a duplicable type (`E11106`) |

A conditional or call-valued prefix row is in scope: the wrapper evaluates it
once into a row temporary and copies leaf by leaf.

## Root Cause

`emit_array_assign` computes one `compact` flag from `fill_target_compactable`.
That predicate returns false for a field projection, a `DerefTemp` alias, or a
volatile module array, but a false flag means only "do not compact". Three copy
sites then always use `memcpy`:

1. The nested fill branch creates a row temporary and copies each fill row
   (`compiler/src/codegen/function.rs:4256-4283`).
2. A place-valued prefix row is delegated to `emit_fixed_array_assign_to`,
   whose duplicable final branch copies the whole row
   (`compiler/src/codegen/function.rs:2126-2149`). That branch is shared with
   whole-array copies (`g = other`), so the fix uses a dedicated prefix-row
   wrapper instead of changing the shared core.
3. A run-time indexed whole-element target is lowered to
   `Statement::StorePointer` (`compiler/src/codegen/function.rs:860-905`); its
   fixed-array branch builds a target from the pointer expression, losing the
   base place's volatility.

`ArrayTarget` carries the C name, address, and byte count, but no volatility
signal, so none of the copies can distinguish a volatile destination.

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
| An impure prefix value is re-evaluated per leaf | ST-V8 counts one call; IP-V14 pins the single call and the temporary |
| A run-time indexed element target is missed | ST-V7/ST-V9 and IP-V13/IP-V15 pin the global and field forms |
| Out-of-scope volatile copies are mistaken for fixed | The known limitations list each remaining shape and its separate cause |
