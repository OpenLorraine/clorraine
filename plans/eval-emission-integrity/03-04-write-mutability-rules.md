# Write-Mutability Rules: Evaluation and Emission Integrity

> **Document**: 03-04-write-mutability-rules.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 4 applies the two user rulings: value-position updates route through the statement-form
write checks (DEF-119), and `cstring` element writes report `E10042` with the registry wording
extended (DEF-124).

## Architecture

### Current Architecture

- The statement form routes through `check_update` (`own/expr.rs:839-896`): `read_place` then
  `write_place` for locals (`own/place.rs:18-127`, precedence `E10023` const → `E10024` read-only
  param → `E10042` immutable view), and the const/immutable checks for globals.
- The value form calls `check_update_value_write` (`own/expr.rs:898-915`), which runs only the
  immutable-view predicate — so `let y: i32 = i32(xs[0]++);` on `const xs: u8[]` or an unmarked
  parameter is accepted and the C expression writes through; a `str` value update reports `E10042`
  where the statement form would report `E10023`/`E10024` first.
- `report_immutable_view_write` matches only `Str`/`StaticStr` prefixes (`own/place.rs:251-282`),
  so `borrow c: cstring; c[0] = 65;` passes; the emitted `char *` store is valid C — the defect is
  conformance.

### Proposed Changes

1. **Value-position updates (DEF-119, user ruling).** `check_update_value_write` routes locals
   through `read_place(id, projections, span)` then `write_place(id, projections, span)`, and
   globals through the const check (with the through-`raw` carve-out) then
   `check_global_immutable_view_write` — exactly the statement taxonomy, one report per site. The
   statement path's split (evaluate the operand, then `check_update`) is untouched, and the
   `borrow s: str` value-update pin keeps `E10042`.
2. **`cstring` element writes (DEF-124, user ruling).** `TyKind::CString` joins the prefix match in
   `report_immutable_view_write`; the registry row becomes `Write to an element of an immutable
   view (\`str\`, \`static str\`, or \`cstring\`)` with the same message and help. Whole-view
   rebinds and writable slice writes stay accepted.

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `own/expr.rs` | `check_update_value_write` and the statement path (untouched) |
| `own/place.rs` | `read_place`, `write_place`, `report_immutable_view_write` |
| `spec/14-diagnostics.md` | the `E10042` wording (only the trigger text changes) |
| `spec/tests/compile-fail/E10042_write_immutable_view.clo` | unchanged; still covers the code |

## Code Examples

```clorraine
const xs: u8[2] = [1, 2];
let y: i32 = i32(xs[0]++);            // E10023 (was accepted)

func f(xs: u8[]): i32 {
    let z: i32 = i32(xs[0]++);        // E10024 (was accepted)
    return z;
}

func g(borrow s: str): i32 {
    let w: i32 = i32(s[0]++);         // E10042 (unchanged pin)
    return w;
}

func h(borrow c: cstring): i32 {
    c[0] = 65;                         // E10042 (was accepted; the ruling)
    return 0;
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Value update on a `const` | one `E10023` | AR-B10 |
| Value update on a read-only parameter | one `E10024` | AR-B10 |
| Value update through an immutable `str` view | one `E10042` | AR-B10 |
| `cstring` element write | one `E10042` | AR-B11 |
| Statement forms, writable views, whole-view rebinds | unchanged | AR-B10, AR-B11 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-26 – ST-31 ([07-testing-strategy.md](07-testing-strategy.md)): the three
  value-update rejections, the `cstring` write rejection, the unchanged statement forms, and the
  writable controls.
- Implementation pins: the exact code per shape, one report per site, the registry wording, and
  byte-identical emission for the accepted controls.
