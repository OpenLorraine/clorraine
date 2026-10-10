# Typedef Emission Order: Store-Emission Integrity

> **Document**: 03-03-typedef-order.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 3 fixes the emission order so every emitted typedef precedes its uses. At the baseline,
slice typedefs precede every struct, enum, and handle typedef, and enum typedefs follow struct
definitions, so accepted programs fail the strict C build (DEF-115 and its two adjacent verified
instances; AR-3, AR-8).

## Architecture

### Current Architecture

`codegen.rs::run` (347-361) emits, in order:

```text
emit_typedefs            -> preamble, slice typedefs, handle typedefs
emit_aggregate_declarations -> struct forward typedefs, struct definitions, enum typedefs
emit_closure_declarations
emit_prototypes
emit_globals / emit_init_functions / emit_bodies / emit_entry_stub
```

The slice typedef embeds the element type by name (`typedef struct { ClP *ptr; usize len; } ClSlice_P;`,
`codegen/program.rs:146-168`), so it needs the element's typedef to be declared first. The
dependencies at the baseline:

| Emitted entity | Depends on |
| -------------- | ---------- |
| slice typedef (`ClSlice_X`) | the typedef of `X`: struct, enum, handle, or a scalar/prelude type |
| struct definition | enum typedefs (fields), slice typedefs (never: view fields are `E10205`), other struct typedefs |
| struct forward typedef | nothing |
| enum typedef | nothing (a backing scalar type) |
| closure environment/typedef | struct forward typedefs and slice typedefs (captured views) |

View-typed struct fields are banned (`E10205`, `spec/14-diagnostics.md`), so no struct
definition depends on a slice typedef; the only later consumer is a closure environment. This
makes the corrected order acyclic.

### Proposed Changes

In `codegen.rs::run` and `codegen/program.rs`:

```text
emit_typedefs          (preamble + handle typedefs; slice typedefs removed)
emit_enum_typedefs     (enum typedefs and member macros; moved out of aggregate declarations)
emit_struct_forward_typedefs
emit_slice_typedefs    (+ zero helpers)
emit_struct_definitions
emit_closure_declarations
...
```

`emit_aggregate_declarations` splits into three small phases (forward typedefs, definitions,
enums moved earlier); `emit_slice_typedefs` and its collection logic stay unchanged apart from
placement. No emitted spelling changes — only statement order.

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `codegen.rs::run` | the phase sequence |
| `codegen/program.rs` | the split aggregate phase and the moved slice phase |
| `own.rs` / `sema` | unchanged: `E10205` keeps view fields banned |
| emission auditors | base/head diff over the corpus (ordering-only change expected) |

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Slice of struct / enum / handle | the element typedef precedes the slice typedef | AR-3, AR-8 |
| Struct with an enum field | enum typedefs precede struct definitions | AR-3, AR-8 |
| Closure capturing a slice | slices precede closure declarations | AR-8 (probe p3a regression) |
| View-typed struct field | still `E10205`; no ordering interaction | AR-8 |

> **Traceability:** every design choice above back-references its register entry — see
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-19 – ST-22 ([07-testing-strategy.md](07-testing-strategy.md)): the three
  ordering instances plus the closure-capture regression, each built under the strict flags and
  run.
- Implementation pins: the emitted order itself (slice typedef after the element forward typedef
  and before the definitions; enums before definitions), byte-identical re-emission.
