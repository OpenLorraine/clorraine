# Callable Effect Modes: Callable Release Integrity

> **Document**: 03-02-callable-effect-modes.md
> **Parent**: [Index](00-index.md)

## Overview

Four defects share one root: a callable's own signature does not reach the call it governs.
Func-typed fields (DEF-155), module-level func-typed globals (DEF-156), `Param`-typed callees
(DEF-157), and lambda parameters (DEF-166) all lose their parameter modes, so owned arguments
leak or double-free, and one name-based fallback reports a false `E10020` against a binding that
shadows a same-named module function. The fix threads each callee's own signature into the call's
modes and keeps resolution identity-based (R5 – R7, R13; AR-6, PD-3).

## Architecture

### Current Architecture

`lower_call` (`own/call.rs:152-250`) resolves the callee, then derives `modes`:

- A func-typed **binding** (`Func`) yields its signature's modes (`own/call.rs:221-228`) — the
  DEF-125 fix.
- A **field call** carries its receiver (`func_field_receiver`, `own/call.rs:183`); the receiver
  branch returns `None` (`own/call.rs:233-235`) — the field's signature is never consulted.
- A **module func-typed global** is invisible to `lookup` (source names only), so the written
  name falls through to `callee_modes(&name)` (`own/call.rs:231`), which resolves only module
  *functions* — a func-typed global yields `None`.
- A `Param`-typed binding matches `TyKind::Func(_) | TyKind::Param(_)` for the local-callee path
  (`own/call.rs:169`) but yields `None` at the modes match (`own/call.rs:229`).
- The **lambda-parameter** call shape never reaches a signature-resolving path at all: the root
  is a lambda parameter, which no walk judges per instance.

`take_move_out_places` (`own/call.rs:386-420`) and the call checker (`own/call_check.rs:243`)
mirror the same resolution; the false `E10020` comes from a name-based fallback reaching a
same-named module function's take places when the callee is really a shadowing binding.

### Proposed Changes

1. **Fields (DEF-155).** When the callable is a func-typed field, the modes come from the field's
   `Func` signature — the receiver branch consults the receiver value's type instead of returning
   `None`. The existing guard stays: a method-style dot-call whose receiver is not func-typed
   keeps its current no-modes path (a same-named module function's modes are never applied to it).
2. **Module func globals (DEF-156).** A written callee name that resolves to a module-level
   func-typed global uses the global's binding identity and its signature's modes, and emits the
   global's symbol spelling (not the written name). Resolution is identity-based: a local binding
   always wins; only a name with no local binding may resolve to the global.
3. **`Param` callees (DEF-157).** An accepted `Param`-typed callee always carries a callable
   bound (an unbounded callable position is `E10307` already). The modes come from the bound's
   signature; when the enclosing instance substitutes a concrete signature, the instance check
   verifies agreement and may refine within the frozen rules. The take move-out places derive
   from the resolved callee identity, so a parameter shadowing a same-named `take` module
   function reports no `E10020` (and the real module call still reports it).
4. **Lambda parameters (DEF-166).** A lambda's parameters are unmarked read-only lends, exactly
   like a module function's unmarked parameters (`spec/03-ownership.md` §2), so a closure call
   materializes its owned temporary arguments for release on the caller's side. An expression
   body is the lifted function's return: it owes the same move and lend-storage judgement a
   `return` statement does, so the filed shape (`((p: T) => Holder<T> { v: p })(x)` reached at
   `T = String`) reports the same single `E10034` its block-bodied spelling and its
   module twin already report — through the existing per-instance duty, which names the enclosing
   function's instance (AR-17).

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `own/call.rs` | callee resolution, the modes branches, `take_move_out_places` |
| `own/call_check.rs` | the call checker's callee lookup (`func_binding_callee`) |
| `own/lower.rs` | argument materialization (`materialize_owned_argument`) |
| `own/instance_check.rs` | the per-instance agreement/refinement of `Param` and lambda signatures |
| `types/keys.rs` | closure instance keys (reused) |

### Invariants

- A callee's modes always come from the signature of the callable the call actually reaches —
  never from a same-named different callable.
- A `take` argument moves (the caller's binding is emptied and a marker guards reuse); a `borrow`
  argument records the lend the module twin records; an owned temporary argument binds and
  releases exactly once.
- Applying a mode to a callee that does not carry it is impossible: every mode source is a
  resolved signature.
- Accepted programs whose callee kinds were already correct stay byte-identical where pinned.

### Code Examples

```clorraine
// DEF-155: field modes mirror the module twin.
let h: Holder<func(take String): i32> = …;
let r = h.take(s);          // s is moved; reuse of s reports E10020 exactly as the twin does

// DEF-156: a module func-typed global applies its signature.
let g: func(take String): i32 = sink;
g(s);                       // move-out plus the marker check

// DEF-157: the bound's signature governs; the shadowing binding is its own identity.
func wrap<F: func(borrow String): String>(f: F): i32 { return i32(len(f(stringOf("x")))); }

// DEF-166: the lambda parameter is judged at the instance.
let h = ((p: String) => Holder<String> { v: p })(x);   // x moves; one release
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| `take` field/global/param callee | Argument moves; caller reuse reports the twin's `E10020` | AR-6 |
| `borrow` callee | The lend the module twin records (including the twin's `E10032`) | AR-6 |
| Owned temporary argument | Materialized and released once (ASan-clean) | AR-6 |
| Param callee without a callable bound | Unchanged: `E10307` (existing rule) | AR-6 |
| Shadowing binding vs same-named module function | No false `E10020`; identity-based resolution | AR-6 |
| Concrete twin programs | Byte-identical / behavior-identical where pinned | R14 |

## Testing Requirements

- Specification cases: ST-4 – ST-7 and the twin controls (`07-testing-strategy.md`).
- Implementation pins: the mode vector per callee kind; the materialization shape of the owned
  temporary argument; the emitted callee spelling for a module func global; the `E10020`/`E10032`
  report counts for the shadowing and borrow pairs.
- Review lenses: correctness + memory-safety + semantics (lend/move rules); the emission diff is
  read by the phase reviewer for shape changes.
