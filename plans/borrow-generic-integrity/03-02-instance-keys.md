# Instance Keys: Borrow/Generic Integrity

> **Document**: 03-02-instance-keys.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns the P2 fix for DEF-79: type keys ignore nullability, so `Wrap<String>` and
`Wrap<String?>` (and every other keyed spelling carrying a nullable type argument) share one C
symbol and the emitted translation unit redefines the type. The fix folds the nullable flag into
`type_key` and keeps the one value-spelling consumer (the text conversion) unchanged.

## Architecture

### Current Architecture

- `compiler/src/types/keys.rs type_key` renders a struct argument through `type_key(arg)` with no
  nullability; the same function names C typedefs, slice keys, zero-view helpers, and the text
  conversion fallback.
- `compiler/src/codegen/types.rs instance_arg_key` appends `Q` to `type_key(ty)` for a nullable
  argument; it is used only on the function-instance symbol path (`codegen.rs`,
  `codegen/expr.rs`).
- `compiler/src/codegen/program.rs` (`struct_unit`) and `codegen/types.rs c_type` derive struct
  instance symbols and typedefs from `type_key`, so two instances whose arguments differ only in
  nullability render the same `Cl<name>__<key>` name and the C build fails with a redefinition.

### Proposed Changes

1. **Fold nullability into `type_key` (AR-4).** Return the same rendering with a `Q` suffix when
   the type carries the nullable flag, before the kind match, so the marker applies uniformly
   (including recursively through instance arguments; slice element types are never nullable,
   PF-009). The marker spelling stays the existing `Q` (AR-4).
2. **Remove `instance_arg_key` (AR-6).** With the rule in `type_key`, the wrapper would
   duplicate it; its two call sites (`codegen.rs`, `codegen/expr.rs`) use `type_key` directly.
   Top-level function-instance symbol spellings stay byte-identical because the wrapper produced
   the same `Q` form; nested nullable arguments change spelling because the wrapper only looked
   at the top-level flag — `pick<Wrap<String?>>` gains `Q` (an expected fixing diff; the two
   spellings collide on one symbol today; PF-003).
3. **Preserve the text-conversion spelling.** The text fallback in `compiler/src/codegen/expr.rs`
   (`text_call`) converts a value through the shim helpers, whose names are fixed
   (`cl_text_i32`, `cl_text_str`, ...). Clear the nullable flag before that one `type_key` call so
   the helper spelling is unchanged; every other consumer keys a type identity and keeps the
   marker. Limitation (PF-012): the `Q` marker is not distinguishable from a user identifier
   ending in `Q` (`Wrap<StringQ>` and `Wrap<String?>` render `Wrap__StringQ` after the fold;
   function instances carry the same pre-existing ambiguity). AR-4 pinned the marker spelling;
   the limitation is recorded here and pinned by an implementation-test note.

## Implementation Details

### New Types/Interfaces

None — one existing function changes, one wrapper is removed.

### Integration Points

- Struct instance symbols and typedefs (`codegen/program.rs`, `codegen/types.rs`): gain `Q` only
  for nullable arguments; non-nullable spellings are unchanged.
- Slice keys, zero-view helpers, and builtin conversion keys (`codegen/types.rs`,
  `codegen/calls.rs`): unchanged for nullable *slices* (`String[]?`), because a slice's element
  type cannot itself be nullable (the grammar forbids `String?[]`; PF-009) — only keys whose
  arguments carry nullability gain the marker.
- Function-instance keys (`codegen.rs`, `codegen/expr.rs`): top-level arguments identical to
  today; nested nullable arguments gain `Q` (PF-003).

## Code Examples

```clorraine
struct Wrap<T> { v: T; }

let a: Wrap<String> = Wrap<String> { v: stringOf("x") };    // ClWrap__String
let b: Wrap<String?> = Wrap<String?> { v: stringOf("y") };  // ClWrap__StringQ
```

Before the fix both spellings emit `struct ClWrap__String` and the strict build reports a
redefinition.

## Error Handling

No new diagnostics — the fix removes an emitted-C build failure for accepted source.

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Two instances differing only in nullability | Distinct keys; distinct C types | AR-4 |
| Text conversion of a nullable value | Flag cleared at the fallback; helper name unchanged | AR-4 |
| Function instance with a top-level nullable argument | Key identical to today | AR-4, AR-6 |
| Function instance with a nested nullable argument | Key gains `Q` (fixes a duplicate symbol) | PF-003 |

> **Traceability:** every decision above back-references the register entry that resolved it. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification tests ST-9 – ST-12 ([07-testing-strategy.md](07-testing-strategy.md)): distinct
  struct instances building and running, top-level function-instance spellings unchanged, nested
  function-instance keys gaining `Q`, the keyed nullable-slice witness, and byte-identical
  re-emission.
- The base-versus-head emission sweep over the corpus must show only expected key diffs
  (nullable-argument spellings) and no function-instance symbol changes.
- Implementation tests pin the emitted symbol names and the typedef shapes.
