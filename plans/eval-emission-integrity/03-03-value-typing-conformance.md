# Value Typing and Checker Conformance: Evaluation and Emission Integrity

> **Document**: 03-03-value-typing-conformance.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 3 conforms the checker and the value typing to the frozen rules: the generic field
projection resolves through its substituted base (DEF-99/111), `let x = null;` keeps the nullable
inference and builds (DEF-108), view-pair equality rejects (DEF-107), a `str` element loop rejects
with the new `E10043` (DEF-118), and a member call on a bare parameter receiver rejects with
`E10212` (DEF-129).

## Architecture

### Current Architecture

- A generic member `b.s` types as `Unknown` (`sema/expr.rs:526-558`, `_ => Ty::unknown()`), which
  is copied into the IR value; per-instance substitution cannot see through it, so the `len`
  renderer falls to `"0"` (`codegen/calls.rs:239`) and the emitted projection folds.
- `null` forms `{ Unknown, nullable }` (`sema/expr.rs:51-54`); the binding keeps it (the re-review
  corrected the row's wording), but `c_type`'s `Unknown` arm spells `i32` (`types.rs:94`) while
  `null_value` spells `NULL` (`codegen/expr.rs:325-339`), so the declaration and the initializer
  disagree.
- `comparable` whitelists `Str`/`StaticStr`/`CString` (`sema/ops.rs:455-467`); the `Eq` arm's only
  exemption is a null pair (`:141-149`), so a view pair reaches `({a} == {b})` over C structs.
- `iterated_element` returns `Unknown` for a non-array (`types/mod.rs:368-386`); the element binds
  as `raw Unknown` and spells `i32 *` over `const unsigned char *`.
- A member callee on a bare-`Param` receiver falls through `check_dot_call`
  (`sema/call.rs:444-465`) with no report.

### Proposed Changes

1. **Substituted projections (DEF-99/111).** The value path resolves a `Member`'s type through the
   substituted base — reusing the place path's `field_type`/`instance_field` resolution
   (`codegen/expr.rs:228-246`) or threading the instance type at lowering
   (`own/lower.rs:645-686`). DEF-99 is reproduced first; when it is the same fallback, both pins
   land together. Concrete-base emissions stay byte-identical.
2. **Null inference (DEF-108).** The binding keeps the literal's nullable type; the declaration
   and the `null` spelling are made to agree (the `c_type`/`null_value` pair for a nullable
   unknown), and the accepted view pairing stays (spec/17-codegen.md:116,124 fixes the mapping
   contract: `T?` lowers to the same C type as `T`; `null` maps to zero/`NULL`).
3. **View equality (DEF-107).** `Str`/`StaticStr` leave `comparable` (the frozen table,
   spec/04-expressions.md:104); the null-pair exemption stays; `cstring` keeps its pointer
   comparison; the report is `E10222` (`sema/ops.rs:273-284`).
4. **`str` loops (DEF-118).** The element-loop check rejects a `str` iterable with the approved
   `E10043` at the loop header (read and write-through spellings); fixed-array and slice loops are
   untouched.
5. **Bare-parameter members (DEF-129).** `check_dot_call`'s fallthrough reports `E10212` at the
   member callee when the receiver is a bare `Param` (one report per site); struct/enum receivers
   and callable-bound members are unaffected.

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `sema/{expr,ops,call,stmt}.rs` | the checker rejections and the inference |
| `codegen/{calls,expr,types}.rs` | the projection resolution and the null spelling |
| `own/lower.rs` | the optional instance-type threading (the fallback lever) |
| `spec/14-diagnostics.md` | the approved `E10043` row and the `E10212` trigger |
| `spec/tests/compile-fail/` | the `E10043_str_loop.clo` golden |

## Code Examples

```clorraine
// DEF-111: reads 7, not the fallback 3
func takeBox<T>(borrow b: T, fallback: i32): i32 {
    if (i32(len(b.s)) > 0) { return i32(len(b.s)); }
    return fallback;
}

// DEF-107 / DEF-118 / DEF-129: rejected per the frozen rules
let a: str = "ab"; let b: str = "ab";
if (a == b) { }                        // E10222
func sum(s: str): i32 { let t = 0; for (borrow c in s) { t = t + i32(c); } return t; }  // E10043
func g<T>(x: T): i32 { return x.foo(5); }  // E10212

// DEF-108: builds; the pairing stays accepted
let x = null;
let s: str = "ab";
if (x == s) { }
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Generic projection through `len` | accepted; instance-resolved emission | AR-B13 |
| `let x = null;` | accepted; nullable declaration | AR-B4 |
| `str`/`static str` pair equality | one `E10222` | AR-B7 |
| `str` element loop (read or write) | one `E10043` | AR-B6, AR-B8 |
| Bare-parameter member callee | one `E10212` | AR-B6, AR-B9 |
| Concrete receivers, bounded members, null pairs, `cstring` pairs | unchanged | AR-B7, AR-B9 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-17 – ST-25 ([07-testing-strategy.md](07-testing-strategy.md)): the
  projection run (7 vs 3), the null-local build and pairing control, the three rejections with
  their accept controls, and the golden.
- Implementation pins: the emitted projection spelling per instance, the unchanged concrete
  emissions (byte-identical), the one-report counts, and the `E10043`/`E10212` spans.
