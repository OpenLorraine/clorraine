# Generated Identities: Callable Release Integrity

> **Document**: 03-01-generated-identities.md
> **Parent**: [Index](00-index.md)

## Overview

Three defects share one root: generated identities are keyed by spellings instead of identity —
the ownership layer's `__cl_owned{N}` temporaries enter the source name map (DEF-138, critical
double-free), the `func` type key drops its signature (DEF-140, struct redefinition), and an
inline capturing lambda emits its environment struct twice (DEF-165, redefinition). The fixes make
generated identities unique by construction; none changes an accepted program's semantics outside
the defective shapes (R1, R3, R12; AR-2, AR-3, AR-4, AR-5).

## Architecture

### Current Architecture

- `materialize_owned_argument` (`own/lower.rs:1839`) and `materialize_scoped_owner`
  (`own/lower.rs:1881`) generate `__cl_owned{N}` names and declare them through
  `declare_local` (`own.rs:799`), which inserts the name into the innermost scope's name map;
  `lookup` (`own.rs:920`) resolves source names against those maps. `move_binding_to_body_scope`
  (`own/lower.rs:1908`) moves the entry between scope maps by name.
- `type_key` renders `TyKind::Func(_)` as the literal `fn` (`types/keys.rs:66`), so every func
  signature shares one key.
- Env struct names render as `struct ClEnv_{site}__{key}` / `struct ClEnv_{site}`
  (`codegen/program.rs:429-430`, `codegen/function.rs:275-276`) and are referenced by compound
  literals at closure call sites (`codegen/expr.rs:845`).

### Proposed Changes

1. **Generated bindings stay out of the source name map (DEF-138).** The owner constructors
   declare their binding through a generated-binding path that creates the `bindings` entry and
   its scope-order slot but never inserts the spell into `scopes[].names`. Source `lookup` can
   therefore never return a generated binding, and a source local spelled `__cl_owned{N}` is an
   ordinary local. Every ownership-layer reference is already id/place-based (`binding_place(id)`,
   scope-order cleanup); no name-based reference to a generated binding remains.
2. **Collision-safe C spelling (DEF-138).** A generated C local whose spelling matches a source
   local renames with the existing generated-name disambiguator (the `__cl_tmp0` → `__cl_tmp0_1`
   mechanism from the strict-build round). Renaming is a spelling change only; the binding's
   identity and cleanup are unaffected.
3. **The func key carries the signature (DEF-140).** `TyKind::Func(_)` renders as
   `fn_<param keys>__<ret key>`, with each component rendered by `type_key` recursively and the
   return separated by a double underscore. Each parameter's key carries its mode
   (`borrow_`/`take_`/`const_` prefix) because a `borrow` parameter reaches the callee as a
   pointer and the type model treats mode-differing signatures as different types
   (`types/mod.rs:82-91`): `func(i32): i32` → `fn_i32__i32`, `func(borrow String): i32` →
   `fn_borrow_String__i32`, `func(Wrap<String>): u32` → `fn_Wrap__String__u32`. Any further
   signature-bearing component the type carries joins the key. Residual crafted-name collisions
   remain under the documented, fail-closed limitation shared with the `Q` marker (DEF-103
   evidence): a collision can only produce a strict-build failure, never a runnable wrong
   program.
4. **One env definition per unique (site, instance) (DEF-165).** The program emitter tracks
   emitted env struct names and emits each definition exactly once; every use site only
   references the type. The existing site/instance naming is kept, so accepted units' symbol
   names do not change.

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `own.rs` | `declare_local` (generated-binding path), `lookup` (unchanged for source names) |
| `own/lower.rs` | the two owner constructors; `move_binding_to_body_scope` |
| `types/keys.rs` | the func arm of `type_key` |
| `codegen/program.rs`, `codegen/function.rs`, `codegen/expr.rs` | env definition dedupe; collision rename |
| `own.rs` loop state | `next_owned_temp` save/restore stays (speculative passes keep identities stable) |

### Invariants

- Source resolution never returns a generated binding; generated references never resolve by
  name (no second name-keyed path may appear).
- Each generated owner releases exactly once at its scope end; the speculative loop pass creates
  the same generated identity as the real pass (Phase-5 rule, preserved).
- Distinct func-signature instances have distinct keys; mode-different same-type signatures share
  one key (documented).
- Each `struct ClEnv_*` name is defined exactly once per translation unit.

### Code Examples

```clorraine
// DEF-138: the user's local is an ordinary local; the generated owner of the
// `use` argument is a different binding with a distinct C spelling.
func use(s: borrow String): i32 { return i32(len(s)); }
func main(): i32 {
    let __cl_owned0: String = stringOf("q");
    let n: i32 = use(stringOf("w"));
    return n + i32(len(__cl_owned0));
}

// DEF-140: two keys, two structs.
let a: Holder<func(i32): i32> = Holder<func(i32): i32> { call: addOne };
let b: Holder<func(u32): u32> = Holder<func(u32): u32> { call: addOneU };

// DEF-165: one env struct for the site, whichever path consumes the closure.
let n: i32 = ([s] () => i32(len(s)))();
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Source local spelled like a generated owner | Accepted; ordinary local; distinct C spelling | AR-2, AR-3 |
| Distinct func signatures | Distinct keys; two struct definitions; strict build passes | AR-4 |
| Same-type, mode-different signatures | One key (documented C-representation identity) | AR-4 |
| Crafted key collision | Fail-closed strict-build failure (documented limitation) | AR-4 |
| Duplicate env use | One definition; uses reference it | AR-5 |

## Testing Requirements

- Specification cases: ST-1 – ST-3 (`07-testing-strategy.md`) — the collision program
  ASan-clean, the two-instance build and run, the inline capturing lambda build and run.
- Implementation pins: the generated binding's absence from the source name map; the C rename
  spelling at a collision; the exact func keys for the two instances; the emitted count of
  `struct ClEnv_` definitions (one); byte-identity for the no-collision corpus.
- Review lenses: correctness + memory-safety (138) + emission-integrity (keys/names).
