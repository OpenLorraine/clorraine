# The Generic Walk's Projected-Argument Resolver: Residual Major Closeout II

> **Document**: 03-02-generic-walk-resolver.md
> **Parent**: [Index](00-index.md)

## Overview

Three rows are one resolver gap in the monomorphization walk: a call whose argument is a
projection the emitter can follow but the walk declines keys no instance (or an unrecorded one),
so the emitted C calls an undeclared or wrongly keyed specialization. The emitter's
`resolved_call_type_args` follows statement temporaries (`codegen/expr.rs:887`); the walk's
`resolved_ir_value_ty` requires a `Local` base (`own/generics.rs:1109`) and declines a
local-headed qualified receiver, a temp-rooted projection, and a conditional/non-local place
base. The batch-K phase-2 fix (DEF-229/231) covered the simple projected-place argument; these
three are its declined siblings.

## Design

### 1. One shared resolver

Extract the emitter's temp-following projection resolution into the walk's resolver (or mirror
it precisely in `resolve_ir_call_args`/`resolved_ir_value_ty`, `own/generics.rs:1031/:1063`) so
the walk and the emitter derive the same value type from the same written argument. The resolver
follows: a `Local` through the local table; a `Temp`/`Materialize` to its bound value; a
projection chain (`Member`/`Index`/`Place`) through the substituted instance fields; a
conditional whose arms agree (the emitter's both-arms-agree rule the batch-K F-2 fix added); and
a qualified name whose head is a **local binding** (not a module) by resolving the head first
and then the field path.

### 2. The local-headed qualified receiver (`DEF-338`)

`let g: i32 = b.pick(7);` with `b: Box` (a module-level binding) lowers the initializer dot call
with the receiver `b`; the walk declines the qualified local-headed receiver, so the instance
stays unkeyed and the emitted `cl_pick__Box` call fails the strict build
(`emission-audit/k1/variants/d03_init_method_wrongkey2.clo`). The receiver-inclusive inference
the phase-1 batch-K correction added covers the direct dot forms; extend it to the
local-headed qualified receiver in the walk (and confirm the emitter's key derivation already
agrees — the body's IR walk keys the same shape, the split the pass relies on).

### 3. The temp-rooted projected argument (`DEF-340`)

`keep(v.name)` inside `each<T>`'s borrow loop: the emitter resolves `cl_keep__String` while the
walk records nothing (the base `v` is an element, and the kernel/value travels through the
element materialization rather than a `Local`). `id(v[i].y)` (the recorded green control,
`K2S-review/src/p2f_elem_member.clo`) already resolves; the failing spelling is the member of
the element whose value position is a temp. The shared resolver accepts the temp root and
derives the projected type.

### 4. The conditional and non-local place bases (`DEF-342`)

`id(c ? a.y : b.y)` (`K2S-review/src/p2b_condarg.clo`) keys no instance (`cl_id__i32`
implicit). The recorded controls: `id(s.y)` through a local runs 3; a crafted non-local place
base (`id(o.s.y)`) runs 1 at the head. The resolver extends to the conditional (both arms
agreeing, per §1) and to a non-local place base (a field chain rooted at a parameter or global).
The nested double-placeholder loop and the nested call result named by the row are the same
resolution and are probed in the phase.

## Integration Points

- `compiler/src/own/generics.rs:407` (`record_call`), `:1002` (`record_call_instance`),
  `:1031` (`resolve_ir_call_args`), `:1063` (`resolved_ir_value_ty`), `:1109` (the `Local`-base
  requirement).
- `compiler/src/codegen/expr.rs:887` (`resolved_call_type_args` — the emitter's resolver to
  unify with).
- `compiler/src/types/mod.rs` (`instance_bindings`, the element mapping the resolver uses).
- Controls: the phase-2 `elementmembercall`/`elementinfer` cases stay green; the direct dot
  forms and the explicit-type-argument spellings stay byte-identical.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| An argument whose projections cannot determine a unique type | No instance is guessed; the previous behavior (the type stage's judgment governs) | — |
| A conditional whose arms disagree | The emitter's both-arms-agree rule; otherwise decline | — |
| A qualified head that is a *module* path | The module binding path (not this resolver) | — |
| A key that still cannot be derived | The existing unresolved-callee diagnostic path | — |

## Testing Requirements

- Spec: the initializer dot call (builds, runs, the receiver evaluated once), the temp-rooted
  member argument (the keyed instance runs), the conditional argument (both selections), and the
  non-local place base; the recorded controls (`id(v[i].y)`, `id(s.y)`, `id(o.s.y)`, the direct
  dot forms) stay green; every accepted program folds a read-back.
- Impl: the keyed instance symbols present (and the opaque/undeclared keys absent), the
  conditional both-arms agreement, the receiver evaluation count; a re-emission determinism pin.
