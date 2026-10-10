# Possibly-Null Widening: Callable Release Integrity

> **Document**: 03-04-possibly-null-widening.md
> **Parent**: [Index](00-index.md)

## Overview

DEF-141: `len(s)` on a null `String` segfaults because the implicit `String`→`str` widening reads
the handle's bytes with no null test, and the accepted program carries no narrowing report. Per
the user ruling (AR-9), the widening is a use: a possibly-null operand reports `E10010` unless it
is narrowed first — the frozen narrowing rule, with no silently-defined empty behavior.

## Architecture

### Current Architecture

- The narrowing rule lives in the ownership stage: a possibly-null value used before its `== null`
  / `!= null` check reports `E10010` (`own/expr.rs:481`, with the documented `unsafe` carve-out at
  `:495`).
- The implicit `String`→`str` widening is inserted at several sites — argument conversion
  (`sema/call.rs:68`), argument rendering (`codegen/calls.rs:212`), returns
  (`codegen/function.rs:1926`), and module initializers (`codegen/program.rs:119`) — and none of
  them consults the source value's narrowing state, so a possibly-null `String` reaches the
  widening's handle read.

### Proposed Changes

The widening operand is checked with the same predicate direct uses use: when the source value's
type is possibly-null (`String?`, or any nullable handle/view converted implicitly) and its
narrowing state is not `non-null`, the use reports `E10010` (`value '{name}' may be null here`,
help `check \`!= null\` before the use`) at the widening site. The check reuses the existing
predicate and carve-outs (including the `unsafe` exemption) so the rule has exactly one
definition; the conversion itself is unchanged for accepted operands. The check runs at every
widening site the checker controls, so function bodies, returns, arguments, and module
initializers report identically.

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `own/expr.rs` | the existing narrowing/`E10010` predicate (extracted or reused) |
| `sema/call.rs` | the argument conversion site |
| `own/lower.rs` | the intrinsic/argument lowering that introduces the widening value |
| `codegen/calls.rs`, `codegen/function.rs`, `codegen/program.rs` | the rendering sites (no check there; they render accepted values) |

### Invariants

- Exactly one report per use site; the message, help, and code are the existing `E10010` row (no
  registry change, AR-11).
- A narrowed operand (inside the `!= null` branch or after a check) converts with no report; a
  non-nullable `String` is unaffected; the `unsafe` carve-out matches the direct-use rule.
- No emission change for accepted programs: the widening renders exactly as before (byte-identical
  where pinned).

### Code Examples

```clorraine
// Rejected: the widening is a use of a possibly-null value.
let s: String? = null;
let n: i32 = i32(len(s));        // E10010 at `s`

// Accepted: narrowed first.
if (s != null) {
    let n: i32 = i32(len(s));    // builds and runs
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Possibly-null operand widened without narrowing | `E10010` at the widening use | AR-9 |
| Narrowed operand | Accepted; renders as today | AR-9 |
| Non-nullable `String` | Accepted; unchanged | AR-9 |
| Inside `unsafe` | Matches the direct-use carve-out | AR-9 |
| Module-initializer widening | Same report (consistency across sites) | AR-9 |

## Testing Requirements

- Specification cases: ST-13 with the narrowed and non-nullable controls
  (`07-testing-strategy.md`); the filed reproduction is re-derived red at HEAD first.
- Implementation pins: one report per site; the message/help text; the accepted spellings'
  byte-identity; the module-initializer spelling.
- Review lenses: correctness + semantics (the narrowing rule is frozen).
