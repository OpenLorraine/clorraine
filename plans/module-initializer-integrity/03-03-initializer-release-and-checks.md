# Initializer Release and Checks: Module Initializer Integrity

> **Document**: 03-03-initializer-release-and-checks.md
> **Parent**: [Index](00-index.md)

## Overview

Two holes surround the init pipeline rather than sitting inside it. DEF-130: initializer
temporaries that own storage (`__cl_itmp0 = mkBox(); ...`) release nowhere — `emit_init_functions`
has no cleanup section and `materialize_init_local` keeps no registry — so a member projection
with an owned member, a `String[2]` element projection, and an owned conditional arm leak once
per program start. DEF-152: the own stage never visits initializer expressions, so a write
through a `const` global (`i32(G++)`) or an immutable view (`S[0]++`) is accepted although the
function-body spelling reports. The fixes add a null-guarded release pass at the init function's
end and a check-only mutability walk over initializer expressions reusing the extracted
predicates.

## Architecture

### Current Architecture

- `emit_init_functions` (`codegen/program.rs:756-770`) writes the init function body and closes
  it with no drops; `materialize_init_local` (`:991-1056`) mints inline `__cl_itmp<N>` locals and
  records nothing; `init_locals` (`codegen.rs:119-124`) is a counter only. The body path's drops
  come from `own.rs:582-646` (scope cleanup) through `codegen/drop.rs` and
  `codegen/cleanup.rs`.
- The own stage enumerates `Item::Func` only (`own.rs:107-138`); `check_globals`
  (`own/program.rs:229-258`) inspects declared types. The mutability reports live in
  `check_update`/`check_update_value_write` (`own/expr.rs:907-970`, `:986-1020`) and the
  immutable-view reporters (`own/place.rs:215-242`).

### Proposed Changes

1. **Init-owner registry and release (DEF-130, PD-3).** The emitter keeps, beside `init_locals`,
   a `Vec<(String, Ty)>` of materialized init locals whose type owns (`owns_drop || maybe_owns`),
   reset per init function. `materialize_init_local` (and the destination/fill paths that mint
   locals for owning values) records each owner in creation order. `emit_init_functions` emits,
   before the closing brace, a null-guarded drop per owner in reverse creation order using the
   existing drop emitter (`emit_drop_expr`), so a local left zero on an untaken conditional fill
   releases nothing. The global itself never releases: a global must be duplicable or `const`
   (`E11106`, `spec/10-modules.md` MOD-25).
2. **Initializer write checks (DEF-152, PD-3).** Extract the pure report predicates from the
   checker's write paths (a global `const` write reports `E10023`; an immutable-view write
   reports its existing code) into small functions shared by the checker and a new
   **check-only walk** over each `Item::GlobalLet`/`GlobalConst` value, invoked from the own
   stage's global loop. The walk resolves write targets with a globals-only place resolution
   (initializers have no locals) and visits the write forms the expression grammar admits:
   update expressions (`++`/`--`), assignments, and writes through indices/members. It reports
   through the same codes and messages as the body spelling and emits nothing.

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `codegen/program.rs` | the registry, its recording points, and the release emission |
| `codegen/drop.rs` | `emit_drop_expr` reused for the null-guarded drops |
| `codegen.rs` | the emitter field's initialization/reset beside `init_locals` |
| `own/program.rs` | `check_globals` invokes the write walk |
| `own/expr.rs`, `own/place.rs` | predicates extracted; the walk reuses them |

### New Functions/Methods

- The write-walk entry over an initializer expression (module + expression), reporting through
  the extracted predicates; no IR, no diagnostics beyond the existing codes.
- The owner-recording helper at each init-local mint.

## Code Examples

```clorraine
// DEF-130 (R8): the owned intermediate releases at the init function's end
struct Box { count: i32; name: String; }
func mkBox(): Box { return Box { count: 3, name: stringOf("n") }; }
let n: i32 = mkBox().count;          // the Box intermediate owns `name`

// DEF-152 (R9): const writes report like the body spelling
const G: i32 = 0;
let h: i32 = i32(G++);               // E10023

let S: str = "ab";
let t: i32 = i32(S[0]++);            // the immutable-view write reports
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Owner assigned only on a conditional fill path | Null-guarded drop releases nothing on the other path | AR-10 |
| A write form outside the walked set | The walk covers update/assign/index/member writes; any other form is a finding for the phase review | AR-4 |
| A new diagnostic would be needed | Stop and escalate (reserved) | AR-12 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-F8 (the three leak shapes host-ASan-clean) and ST-F9 (the write
  rejections with the body-spelling parity and read-only controls).
- Implementation pins: the release text at the init function's end for each shape; the
  null-guard on a conditional-fill owner; one report per write site with the frozen
  message/help.
- Memory-safety and semantics reviewers re-probe the phase shapes (AR-15).
