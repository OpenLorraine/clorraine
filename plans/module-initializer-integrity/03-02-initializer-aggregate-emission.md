# Initializer Aggregate Emission: Module Initializer Integrity

> **Document**: 03-02-initializer-aggregate-emission.md
> **Parent**: [Index](00-index.md)

## Overview

DEF-134/135/143 are emission-form gaps in the init path. `emit_global_init`
(`codegen/program.rs:779-824`) renders the lowered value through the pure expression renderer,
which has no destination context: a struct literal with a fixed-array field becomes a compound
literal C rejects, a `String`/concat value at a `str` destination misses the view wrapper, and a
qualified dot-`len` folds against the head binding's type and falls back to `0`. The fix gives
the init store the body store's destination-aware decisions, shape-gated so unaffected
initializers re-emit byte-identically.

## Architecture

### Current Architecture

- The body assign path consults `struct_literal_needs_stores` (`codegen/function.rs:1489-1509`)
  and switches to `emit_struct_literal_store` (`:1519-1545`) with `memcpy` for fixed-array
  fields (`:1083+`, `:456-458`); the init path renders `ValueKind::StructLit` as
  `(Cast){ ... }` (`codegen/expr.rs:99-101`, `:968-995`) with array fields as brace lists
  (`:951-959`).
- The body wraps owned `String` values at `str`/`static str` destinations in
  `cl_string_view` (`codegen/function.rs:1937-1941`, also `codegen/calls.rs:251-272`); the init
  store calls `self.value(&lowered)` directly (`program.rs:821-823`).
- The body folds `.len` structurally (`own/call.rs:288-337`, `own/place.rs:997-1024`); the init
  path's `global_len_receiver` (`program.rs:1799-1826`) builds the receiver from the head
  global's type, so `b.arr` renders against `Box` and `codegen/calls.rs:277-305` yields `"0"`.

### Proposed Changes

1. **Destination-aware init store (DEF-134, PD-2).** Before rendering, the init store decides:
   when the value is a struct literal whose fields need stores (the body predicate, reused or
   mirrored), emit the statement-form store into the destination place — the global for a whole
   init, or the init local — with per-field stores and `memcpy` for fixed-array fields
   (`emit_fixed_array_assign_to`'s contract). The compound-literal rendering stays for the
   shapes the body also renders that way.
2. **`str` view wrap (DEF-135, PD-2).** The init store applies the same destination rule as
   `store_text`: a `str`/`static str` destination receiving a `String`-typed value (a call
   result, a concat, a field read) wraps the rendered text in `cl_string_view(...)`.
3. **Structural `len` fold (DEF-143, PD-2).** `global_len_receiver` resolves the receiver by
   walking the qualified path's projections over the head global's type — a fixed-array field
   yields its extent — mirroring `qualified_structural_len`; the fold produces an integer
   constant in the init emission (no runtime receiver read).

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `codegen/program.rs` | `emit_global_init` and the materialization walk gain the destination decisions |
| `codegen/function.rs` | the predicates/emitters to mirror (`struct_literal_needs_stores`, `emit_struct_literal_store`, `store_text`) |
| `codegen/expr.rs` | the renderer stays pure; the store supplies the destination |
| `codegen/calls.rs` | the `len` renderer fallback stays for non-structural receivers |
| `own/place.rs` | `qualified_structural_len` is the contract for the fold |

### New Functions/Methods

- An init-store entry (`emit_global_init`/materialization helper) that takes the destination
  place and dispatches: struct-needs-stores → statement store; view wrap → wrapped text;
  otherwise the current single assignment.
- A projection-walking receiver resolver for the init `len` fold.

## Code Examples

```clorraine
// DEF-134 (R5): array field from a place — statement store with memcpy
struct BoxA { arr: i32[2]; tag: i32; }
let garr: i32[2] = [1, 2];
let boxed: BoxA = BoxA { arr: garr, tag: 3 };

// DEF-134: array field from a call result — same store form
let boxed2: BoxA = BoxA { arr: dup(), tag: 4 };

// DEF-135 (R6): template at a str destination — view wrap
let n: i32 = 4;
let label: str = "n=${n}";

// DEF-143 (R7): qualified .len — constant fold
let b: BoxA = BoxA { arr: garr, tag: 3 };
let count: usize = b.arr.len();
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Struct literal neither the body nor the init store can render | Not reachable: the mirrored body path is the complete contract | AR-7 |
| `len` receiver not structural | Falls back to the existing renderer path (unchanged) | AR-9 |
| Unaffected initializer shapes | Byte-identical emission (shape-gated) | AR-13 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-F5 – ST-F7: strict builds plus run cases pinning values.
- Implementation pins: the statement-form store fragments (`memcpy`, per-field stores) for
  DEF-134; the wrapped accessor text for DEF-135; the folded constant for DEF-143.
- Byte-identity: the existing init pins stay green untouched; a seeded control initializer
  re-emits identically.
