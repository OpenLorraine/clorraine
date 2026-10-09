# Deferred Return Continuation and Ordering: Defer Integrity

> **Document**: 03-03-deferred-return.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-85, DEF-88 (plus the loop-local root cause)
> **AR**: AR-4, AR-5, AR-8, AR-10

## Overview

Two mechanisms fix the deferred-return family. First, a `return` inside a deferred block currently
emits `ScopeExit { owning scope }` markers that the emitter maps back to the rung **currently
executing**, so the copy jumps to its own label — an infinite loop (DEF-85). The return must
instead continue the running cleanup: the remaining actions of the scope whose cleanup is
executing (actions older than the deferred block's position), then the outer rungs. Second, for a
`Return`-terminated block the emitter prints its trailing inline cleanup before the terminator's
value store, so the value expression reads storage the cleanup just released (DEF-88) — the same
ordering bug reproduces without any defer for a return inside a loop body (the loop-local root
cause). This component adds a running-cleanup frame with a continuation label, and gates the
value-before-cleanup ordering to value returns.

## Architecture

### Current Architecture

- A `return` inside a deferred block: `check_return` → `emit_return_cleanup`
  (`compiler/src/own/statements.rs:996`; `compiler/src/own.rs:1183-1193`) walks **every** open
  scope and emits `ScopeExit` markers, including the defer's owning scope.
- At the cleanup site, `emit_block_body` maps the edge scopes to rungs
  (`compiler/src/codegen/cleanup.rs:181-184`) and `emit_return` prints `goto __cl_c{jump}`
  (`cleanup.rs:376-377`). Inside rung `__cl_cK` this is a self-jump.
- `emit_block_body` prints inline cleanup during the statement walk (`cleanup.rs:134-153`,
  `:195-199`); the `Return` value store prints later at the terminator (`control.rs:226-228`).
  The rung path already orders store-before-drop (`d88b`), which the store-before-release pins
  protect.

### Proposed Changes

1. **Running-cleanup frames.** When a scope's actions run at a site, `emit_actions` pushes a
   frame `{ scope, action position, activation }` for the duration of that run. A copy emitted
   under a frame carries it; when the copy's region contains a `Return` terminator, the emitter
   creates a **continuation label** placed immediately after the copy's text, and the copy's
   returns jump to it (`__cl_ret = v; goto <continuation>`). Control then falls through the
   remaining actions (positions older than the current one, already in emission order), then the
   site's natural tail (the ladder's next-rung `goto` or final return; or, for inline sites, the
   terminator the block walk emits after the walk).
2. **Nested re-entry runs only the remaining actions.** A copy's return cleanup may re-enter the
   running scope (the `ScopeExit { scope }` of a scope whose cleanup is executing). The nested
   activation looks up the active frame for that scope and emits only the actions **older than the
   frame's position**; if no frame is active, it emits the full list as today. The nested run
   pushes its own frame for the remaining sequence. This is the rule that makes DEF-87 correct on
   both paths and prevents a return in an older defer from re-running a newer one.
3. **The continuation is emitted only when needed.** A copy whose region contains no `Return`
   terminator emits no continuation label (no unused-label warnings under `-Werror`); the label
   spelling follows the copy namespace from [03-01](03-01-per-site-deferred-copies.md).
4. **Deferred returns no longer target the own rung.** While a copy is emitted under a frame, its
   `Return` terminators use the frame's continuation jump instead of the `edge_scopes.first()`
   rung mapping. The outer scopes' cleanup still runs: on a rung site, the ladder tail chains to
   the next rung; on an inline site, the natural terminator emission jumps to the outer rung (or
   returns).
5. **Value before trailing inline cleanup.** For a block whose terminator is `Return { value:
   Some(_) }`, the emitter buffers the inline cleanup encountered during `emit_block_body`'s walk
   and prints it **after** the value store and the terminator's releases, before the ladder jump /
   return. Void returns, non-return terminators, and the rung path are unchanged. Every return
   form that stores the value and leaves (the result-variable tail, the no-ladder block-local
   form, the fixed-array out-parameter, and the struct-literal / conditional store forms) must
   honor the order: **evaluate and store the value, then run the buffered cleanup, then leave**.

## Implementation Details

### New Types/Interfaces

- A frame stack in the emitter (private): each frame carries the scope id, the current action
  position, the action list, and the continuation label when one was created.
- The buffered inline-cleanup list for value-returning blocks (private to the block walk).

### New Functions/Methods

- `Emitter::emit_actions` gains the frame push/pop and the nested remaining-actions rule.
- `Emitter::emit_defer` passes the active frame to the copy so its returns resolve to the
  continuation.
- `Emitter::emit_return` gains the buffered inline cleanup (emitted after the value store on every
  leaving path).
- `emit_block_body` returns or forwards the buffered cleanup for `Return { value: Some(_) }`
  blocks instead of printing it inline.

### Integration Points

- The checker's `ScopeExit` markers stay as they are; the emitter now interprets them against the
  frame instead of only the rung map. No checker change is required for the continuation in this
  phase.
- The ordering change interacts with the fixed-array and struct-literal return forms
  (`cleanup.rs:396-497`); the value-before-cleanup invariant is the acceptance, and the
  store-before-release pins (`def_backlog_closeout_impl_test.rs:1287-1315`) are the regression
  oracle.
- Emission diffs are expected on defer-return shapes and on value returns with trailing inline
  cleanup (including the loop-local shape); the emission-integrity auditor traces each to its
  requirement.

## Code Examples

### Example 1: the filed function-scope shape after the fix

```c
    __cl_ret = 0;
    goto __cl_c1;
__cl_c1:                    /* scope 1 */
    __cl_ret = 7;           /* the deferred return stores ...            */
    goto __cl_c1_cont;      /* ... and continues the running cleanup     */
__cl_c1_cont:
    return __cl_ret;        /* the remaining actions and the ladder tail */
```

### Example 2: remaining actions (`d85c`) and the ordering (D88)

```c
__cl_c1:                    /* actions: [Defer, Drop(a)] — Defer emitted first */
    ... deferred body: __cl_ret = len(a); goto __cl_c1_cont;
__cl_c1_cont:
    if (a) { FreeString(a); }   /* the remaining drop still runs   */
    return __cl_ret;            /* 3, sanitizer-clean              */
```

```c
    s = cl_std_text_stringOf(cl_str("x"));
    __cl_ret = (i32)((cl_string_view(s)).len);  /* the value is read first */
    if (s) { FreeString(s); }                   /* then the local drops    */
    s = NULL;
    return __cl_ret;                            /* 1, sanitizer-clean      */
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Deferred return inside a rung | Continuation label after the copy, then the remaining actions and the ladder tail | AR-5 |
| Deferred return on an inline site | Same continuation; the natural terminator emission reaches the outer rung | AR-5 |
| Nested re-entry into the running scope | Emit only actions older than the frame's position | AR-7 (AR-5) |
| A copy with no return | No continuation label emitted | AR-5 |
| Value return with trailing inline cleanup | Store, then the buffered cleanup, then leave — on every return form | AR-8 |
| Void returns / non-return terminators | Unchanged | AR-8 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-8 (function-scope deferred return), ST-9 (outer droppable — remaining
  drop runs, ASan clean), ST-10 (returning switch arm), ST-11 (two-defers rung), ST-12 (older
  defer runs after the return), ST-13 (defer-local ordering), ST-14 (loop-local root cause) — see
  [07-testing-strategy.md](07-testing-strategy.md).
- Implementation tests: the continuation shape (`__cl_c1_cont` or the pinned spelling), the
  remaining-actions order, the buffered-cleanup order, and byte-identical re-emission.
- Host-ASan probes: release counts and side effects are exactly one per runtime path.
