# Shadowed Narrowing: Borrow-View Moves and Shadowed Narrowing

> **Document**: 03-02-shadowed-narrowing.md
> **Parent**: [Index](00-index.md)

## Overview

This component makes a declaration's initializer resolve against the outer
scope, so a shadowing initializer under a `!= null` narrowing stops reporting a
spurious `E10010` (R2, R3; AR-5, AR-6).

## Architecture

### Current Architecture

`check_let` (`own/statements.rs:178-207`) and `check_const` (`:256-281`) already
resolve the store shape and the ownership effect before declaring the binding —
the file's own comment says the initializer must tie to the outer binding it may
shadow. The null check is the exception: `check_value_into` runs **after**
`declare_local` and `set_place`, and it resolves the initializer's name through
`plain_binding`, which now finds the new (not narrowed) binding and reports
`E10010`.

### Proposed Changes

Move the `check_value_into(value, &target, span)` call before
`declare_local` in both `check_let` and `check_const`, so every initializer
check sees the same pre-declaration scope the store preparation sees. The call
emits no IR and reads only the expected type, the initializer's recorded type,
and the narrowing state, so the reorder changes nothing else (AR-5, AR-6).

## Implementation Details

### New Types/Interfaces

None.

### New Functions/Methods

None.

### Integration Points

- `check_nonnull_expr` (`own/expr.rs:291`) resolves the initializer's place
  through `plain_binding`; with the call moved, the place is the outer binding
  and its narrowing holds inside the `if (b != null)` body.
- `check_value_into`'s other checks are unaffected: `null` against a
  non-nullable declared type still reports `E10012`, and a nullable value
  against a non-nullable target still runs the narrowing check — now against the
  outer binding.
- No IR, lowering, or emission change: `prepared` and `effect` were already
  computed before the declaration.

## Code Examples

### Example 1: Accepted shadowing initializer

```clorraine
import { Buffer, bufferNew } from std.mem;

func main(): i32 {
    let b: Buffer? = bufferNew(8);
    if (b != null) {
        let b: Buffer = b;     // the outer binding is narrowed here
        return 0;
    }
    return 1;
}
```

### Example 2: Guard — an un-narrowed use still reports

```clorraine
func use(b: Buffer): i32 { return 0; }

func main(): i32 {
    let b: Buffer? = bufferNew(8);
    return use(b);             // E10010: value 'b' may be null here
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Shadowing initializer under a narrowing | Accepted; resolves to the outer binding | AR-5 |
| `const` shadowing initializer | Same fix and behavior | AR-6 |
| Nullable use without a narrowing check | Unchanged: `E10010` | AR-13 |
| `null` against a non-nullable declared type | Unchanged: `E10012` | AR-13 |

> **Traceability:** Every error-handling strategy and design choice must
> reference the Ambiguity Register entry (AR #) that resolved it. See
> `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests ST-9 – ST-12 in `07-testing-strategy.md`.
- Implementation tests pin that the initializer's diagnostic (when one is due)
  still reports once and that the shadowing binding itself is usable after the
  declaration.
