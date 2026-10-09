# Per-Site Deferred Copies: Defer Integrity

> **Document**: 03-01-per-site-deferred-copies.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-86, DEF-87 (plus the early-return sibling)
> **AR**: AR-4, AR-7, AR-10

## Overview

A deferred block must run on **every** exit edge of its scope. Today the emitter copies the block
into the first cleanup site that runs its scope's actions and shares the function-global printed
block set, so every later site's copy is silently empty (DEF-86, DEF-87, and the unfiled early
return sibling). This component makes the copy **per site**: each cleanup activation emits its own
copy with a copy-local printed set and a deterministic label namespace, and the checker keeps the
deferred block checked on every exit edge.

## Architecture

### Current Architecture

- `collect_cleanup` (`compiler/src/codegen/cleanup.rs:39-86`) maps each scope to its actions
  (`Drop`/`Defer`) and rungs; a `Defer` action carries the lowered block id.
- Each cleanup activation runs `emit_actions` (`cleanup.rs:237-244`) → `emit_defer` → `emit_closure`
  (`compiler/src/codegen/control.rs:136-148`).
- `emit_closure` computes `closure_from(function, root)` (`cleanup.rs:693-705`) and emits it while
  `self.emitted` (the function-global printed set, `control.rs:166-168`) suppresses any block
  already printed — including blocks printed by an earlier copy of the same deferred block.
- The checker consumes a scope's `defer_entries` at the first cleanup run (`own.rs:482-499`
  `run_scope_cleanup` pops the entries), so later exit edges do not re-check the block's effects.

### Proposed Changes

1. **One copy per activation.** `emit_defer` allocates a copy activation: it saves the current
   printed-block set, installs a **copy-local** set for the walk, and restores both afterwards
   (mirroring the existing `reachable` save/restore in `emit_closure`). The copy-local set starts
   empty for each activation, so the same deferred root emits at every site.
2. **Active-root suppression only.** While a copy of a deferred root is being emitted, a nested
   activation for the **same** root is a no-op (prevents self-recursion through the copy's own
   return cleanup). Nested activations for other roots emit normally. The general
   remaining-actions rule for nested re-entry is [03-03](03-03-deferred-return.md)'s frame work;
   this phase only needs the active-root rule.
3. **Deterministic copy namespace.** A per-root copy counter is incremented at each `emit_defer`
   activation in emission order. The **first** copy of each root keeps the existing block-label
   spelling (`__cl_b{id}`) so single-site defers stay byte-identical; later copies suffix their
   block labels (`__cl_b{id}__{n}`, `n` ≥ 2). C labels are function-scoped, so the suffix is
   required, not cosmetic. The counter is assigned during emission from the deterministic block /
   action order; the pinned rule is: `n` is the 1-based ordinal of the activation for that root in
   the emitter's deterministic emission order (the same order in every run — the re-emission
   determinism suite is the oracle).
4. **Per-edge effect checking.** The checker keeps a scope's deferred entries available to every
   exit edge: `run_scope_cleanup` builds its action list from the matching entries **without**
   permanently consuming them, and an active-entry guard prevents re-entering a block whose check
   is in progress (the nested `check_defer_exit` truncation at `own.rs:578,594` continues to bound
   recursion). Requirement: a deferred block's effects (moves, loans, narrowing) are applied on
   every exit edge of its scope, in the same reverse registration order as the emitter runs it.

## Implementation Details

### New Types/Interfaces

- A copy-activation record inside the emitter (private): the root block, the copy ordinal, the
  saved/installed printed set, and the active-copy stack. No public API.
- No IR change in this phase (the block range for bounding arrives with
  [03-02](03-02-bounded-closure-and-context.md)).

### New Functions/Methods

- `Emitter::emit_defer_copy(root)` — replaces the direct `emit_closure` call from `emit_defer`:
  allocates the activation, installs the copy-local printed set, calls the closure walk, restores.
- `Emitter::copy_label(base: &str) -> String` — the suffix rule for copy-local C labels
  (`__cl_b{id}` for the first activation, `__cl_b{id}__{n}` afterwards).
- `run_scope_cleanup` (checker) — action-list construction keeps the entries; an active-entry set
  guards recursion.

### Integration Points

- `collect_cleanup`, the rung model, and `emit_ladder` are unchanged; only the copy emission and
  the checker's entry lifetime change.
- The emission-integrity auditor classifies every base-vs-head diff: programs without a
  defer-with-early-exit shape must be byte-identical (single-site defers keep their labels).

## Code Examples

### Example 1: the DEF-86 shape after the fix

```c
while ((i < 2)) {
    i = i + 1;
    if ((i == 1)) {
        if ((i == 2)) { ... }          /* the continue edge runs the defer once  */
        continue;
    } else { }
    if ((i == 2)) { return 9; }        /* the fall-through site runs it too     */
    continue;
}
```

### Example 2: the DEF-87 guard path

The later defer's guard-return edge re-enters the running scope; the earlier (returning) defer
emits **there** (active-root suppression keeps the later defer's own copy from repeating), and the
genuine outer site emits it again for the guard-false path. Both paths run the earlier defer
exactly once.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Nested activation of an actively-emitting root | No-op (active-root suppression) | AR-7 |
| Copy label collision across activations | Per-copy suffix (`__cl_b{id}__{n}`); first copy unchanged | AR-7 |
| Non-deterministic copy numbering | Rejected by the determinism suite and byte-identical re-emission tests; the pinned ordinal rule above is the only source | AR-4 |
| A deferred block re-entered while its check is in progress | Active-entry guard; nested `check_defer_exit` truncation stays | AR-7 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-1 (fall-through + continue), ST-2 (two defers with a guarded return),
  ST-3 (early-return sibling), ST-4 (three cleanup sites in one scope) — see
  [07-testing-strategy.md](07-testing-strategy.md).
- Implementation tests: the emitted shape has one copy per site (count the deferred guard in the
  emitted C), the first-copy label spelling is unchanged, later copies use the suffix, and two
  emissions are byte-identical.
- The checker probe: a deferred move/loan is visible on the fall-through edge (diagnostic or
  state evidence recorded in the phase).
