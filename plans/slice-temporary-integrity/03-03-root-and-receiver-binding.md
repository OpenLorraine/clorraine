# Root and Receiver Binding: Slice and Temporary Integrity

> **Document**: 03-03-root-and-receiver-binding.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-71, DEF-89
> **AR**: AR-5

## Overview

Two owned values escape the existing bindings. First, a call with a `take` argument materializes
its result in a non-releasing temporary and empties the consumed places eagerly; the remaining
fields of the result have no owner and leak (DEF-71a). A projection whose root is a conditional of
calls is rejected by both root binds, which match only `Call`/`ClosureCall`, so neither arm's result
is released (DEF-71b). Second, a plain member read of an owned call result (`spawn().count`) is not
bound; only intrinsic arguments and dot-`len` receivers are (DEF-89).

The fix extends the existing binding machinery to those shapes: register the eager temporary's
release, bind a whole conditional root as one value through the destination/guard materializers,
and route owned member receivers through the intrinsic-argument binding.

## Architecture

### Current Architecture

- The eager move-out branch (`compiler/src/own/lower.rs:135-145`) uses `materialize_value`
  (`:895-905`), which registers no release; the deferred branch (`:124-134`) registers one. The
  released temporary's move-out empties the consumed places immediately.
- The root binds match only `Call`/`ClosureCall` roots
  (`bind_ownerless_projection_root` `:1374-1381`, `bind_guarded_projection_root` `:1424-1431`);
  a `Conditional` root is returned unchanged.
- `bind_intrinsic_argument` (`:1447-1486`) is the owned-read binder: it skips views, unknown
  types, place-rooted projections, conditionals with a place-rooted arm, values that may lend
  (`value_may_lend`), and already-registered temporaries, then materializes eagerly or through the
  guard operand. It is called for intrinsic arguments and dot-`len` receivers only.
- The owned-conditional store (`emit_conditional_move_assign`, `compiler/src/codegen/function.rs:1569-1640`)
  moves exactly one arm; the guard emitter uses it for a deferred conditional fill
  (`compiler/src/codegen/guards.rs:119-142`).

### Proposed Changes

1. **Bind the eager temporary through the destination-temp materializer, and empty it on
   adoption.** The eager move-out branch materializes its result with `mark_destination_temp` and a
   registered release (the `materialize_value_type` shape), and the owning adoption emits the
   emptying store (`AssignThenMoveOut`) when the lowered right-hand side is a registered
   destination temporary. A moved-away value empties the temporary, so the release becomes a
   no-op; a whole-result adoption moves the value out instead of copying it (preflight PF-003).
2. **Bind a conditional root as one value.** Both root binds gain a `Conditional` root arm, guarded
   by the existing place-arm predicate (`conditional_borrows_place`, `:1495-1507`): only a
   conditional whose arm tree has no place-rooted leaf is bound. The eager bind uses the
   release-registering materializer; the guarded bind uses `materialize_guard_operand`, which
   already fills an owned conditional through the branch store.
3. **Bind owned member receivers at read positions.** The receiver bind runs from the read
   consumers that take the member value (cast, intrinsic argument, borrowed/take argument
   handling, and return reads), not from the central `ExprKind::Member` lowering. Consume
   preparation, guarded positions, and loop-condition lowering keep their own machinery and are
   explicitly excluded (preflight PF-004).

### Integration Points

- `value_may_lend` (`:1516-...`), `projects_from_place` (`:1074-1095`), and the pending-release
  guards stay in force; the new call sites only reach shapes the existing predicates allow.
- `materialize_owned_argument` (`:1554-1653`) remains the binding implementation; its body-scope
  placement keeps a view derived from the bound value valid.
- The receiver bind runs only at read positions; consume preparation, guarded positions, and
  loop-condition lowering are excluded, and the guard lowering's deferred materialization is
  untouched (preflight PF-004).

## Code Examples

### Example 1: A take argument on the root call (DEF-71a)

```clorraine
sink(mkfrom(mka()).a);   // .a moves into sink; .b releases at statement end
```

### Example 2: A conditional root (DEF-71b)

```clorraine
if (flag && sink((c ? mk1() : mk2()).a)) { ... }   // the chosen call's sibling fields release
```

### Example 3: An owned member receiver (DEF-89)

```clorraine
return i32(spawn().count);   // the Array buffer releases at statement end
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Eager temporary adopted by a destination | The emptying adoption moves the value out; the registered release becomes a no-op (preflight PF-003) | AR-5 |
| Conditional root with a place-rooted arm | Keep the inline read (the existing place-arm rule; binding would adopt a place) | AR-5 |
| Conditional root with ownerless arms | Bind the whole conditional through the destination/guard materializer | AR-5 |
| Receiver already carrying a pending release | Leave it; a single release stays registered | AR-5 |
| Receiver that may lend or projects from a place | No bind | AR-5 |
| Guarded receiver | Deferred materialization; the fill runs only on the evaluated path | AR-5 |
| Member read in a consuming, guarded, or loop-condition position | No receiver bind; the existing preparation/machinery handles it (preflight PF-004) | AR-5 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests ST-9 – ST-13 (`07-testing-strategy.md`): the root-call take argument, the
  conditional-of-calls root on both branch selections, the owned member receiver, and the
  whole-result adoption / loop re-assignment forms; host-ASan clean each.
- Implementation tests: the destination-temp mark and release on the eager temporary; the emptying
  adoption shape; the absence of a second owner in the loop form; the conditional root's
  branch-store shape; the read-position receiver binding and its excluded positions; the place-arm
  conditional staying inline; the existing `__cl_owned` absence pins for adopting positions.
- Regression pins: `len(spawn())` and the dot-`len` receiver shapes stay clean; the loop-condition
  emission pins migrate where the receiver bind is excluded (preflight PF-004).
