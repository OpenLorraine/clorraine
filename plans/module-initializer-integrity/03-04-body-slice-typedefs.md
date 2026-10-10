# Body Slice Typedefs: Module Initializer Integrity

> **Document**: 03-04-body-slice-typedefs.md
> **Parent**: [Index](00-index.md)

## Overview

DEF-137 is the body-side twin of the initializer slice-typedef collection fixed by the preceding
batch. A written type argument such as `sizeof(i32[])` inside a function body lowers to an IR
`ValueKind::Intrinsic { type_arg }` (`own/lower.rs:310-337`) but the emission's body-side
collector scans only locals, temporaries, signatures, and instance substitutions
(`codegen/program.rs:268-312`), so `ClSlice_i32` is never defined and the strict build fails.
The fix adds the missing value walk to the collector.

## Architecture

### Current Architecture

- `slice_element_types` (`codegen/program.rs:211-314`) assembles the slice typedef set; the
  initializer side walks every expression form including written type arguments
  (`collect_init_slice_types`, `:323-396`, arm at `:374-386`); the body side
  (`:268-312`) has no statement/value walk.
- The element walk itself is `collect_slice_types` (`codegen/types.rs:345-363`); typedef
  emission splits into early/late rows (`program.rs:152-174`, `:200-208`).

### Proposed Changes

1. **Collect from body IR values (DEF-137, PD-2).** Beside the `:268-275` loop, walk each
   function's statements/values for `ValueKind::Intrinsic { type_arg: Some(ty) }` and call
   `collect_slice_types(&ty, &mut found)`. The functions list holds per-instance IR (already
   substituted), so the collected types match the emitters' spelling; generic-body written
   types arrive substituted like every other type in that function's IR.
2. **Shape gating.** The addition only adds types to the set; units that already carried their
   typedefs emit byte-identically (AR-13).

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `codegen/program.rs` | the body-side collection loop gains the value walk |
| `codegen/types.rs` | `collect_slice_types` unchanged |
| `own/lower.rs` | the `type_arg` production is the contract (unchanged) |

### New Functions/Methods

- A small recursive value walk (statements → values → nested values) collecting `type_arg`
  slice types, mirroring the init-side collector's arm.

## Code Examples

```clorraine
// DEF-137 (R10): the body spelling defines its slice typedef
func size(): usize {
    let n: usize = sizeof(i32[]);
    return n;
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Written type argument that is not a slice | `collect_slice_types` ignores non-slice types | AR-11 |
| Duplicate collection | The set deduplicates | AR-11 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification case ST-F10: the body spelling builds under the strict flags and runs.
- Implementation pin: the emitted `ClSlice_i32` typedef plus the `sizeof` use.
- Byte-identity for units whose sets already contained their types.
