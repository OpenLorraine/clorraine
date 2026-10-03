# Do-While Condition Release: Def Backlog Completion

> **Document**: 03-03-do-while-condition-release.md
> **Parent**: [Index](00-index.md)

## Overview

Close DEF-62: an early `return` inside a `do ... while` body does not release the condition's
materialized temporaries, so the final condition value leaks. The fix appends the condition's newly
registered releases to the body's return terminators in the ownership checker. This document owns
the design; the reproduction evidence is in [02-current-state.md](02-current-state.md)
§Reproductions, and the ruling is AR-4.

## Architecture

### Current Architecture

`check_do_while` lowers the body before the condition (`compiler/src/own/loops.rs:123-136`). A
`return` in the body snapshots its release list at that moment
(`compiler/src/own/statements.rs:927-936`), so the condition's materialized temporaries — created
and registered afterward by `materialize_owned_calls` — are absent. Their only release drains into
the loop's `exit` block (`own/loops.rs:149-154`, `own/statements.rs:107-132`), which a return never
reaches. `while` lowers the condition before the body and is unaffected.

Three verified facts shape the fix:

- compiler temporaries are declared at function scope (`compiler/src/codegen/function.rs:287-301`),
  so a body return may reference the condition temporary safely;
- pending releases are copied newest-first (`compiler/src/own.rs:1044-1046`), so appending must
  preserve that order;
- the body/condition/exit blocks are all allocated before the body is lowered, so "the body's
  returns" is a reachability question, not a block-id range — `compiler/src/own/liveness.rs:150-170`
  already provides the successor walk.

## Proposed Changes

In `check_do_while`, after the condition lowers and `materialize_owned_calls` registers its
temporaries:

1. **Checkpoint.** Before lowering the condition, record the current pending-release count. Only
   the releases registered between the checkpoint and the end of condition lowering are the
   condition's; outer releases already appear in the body returns' snapshots and must not be
   added twice.
2. **Collect the body's returns.** Capture the block count before `check_block(&decl.body)`; after
   the condition lowers, walk the blocks created during body lowering and keep the
   `Terminator::Return` blocks reachable from `body_block` via the existing `successors` walk
   (`body_block` itself is included in the candidate set; a return there means the condition never
   ran, so it carries nothing to append). This includes returns inside nested blocks and nested
   loops; the outer loop's later append adds only its own new places, so a nested return receives
   both loops' condition releases.
3. **Prepend the new releases.** For each collected return, prepend the condition's release places
   in newest-first order (matching `pending_release_copies`), skipping any place already present in
   that return's list so a release is never duplicated.
4. **Record the release events.** The appended places go through the same release-event recording
   the return path already uses (`own/statements.rs:929`), so the borrow checker sees the same
   events as the fall-through exit path.
5. **Builder mutator.** Add `Builder::append_return_releases(&mut self, blocks: &[BlockId],
   releases: &[Release])` in `compiler/src/ir.rs` (the builder currently exposes only
   `set_branch_shape`, `ir.rs:701`), implemented as an order-preserving prepend per block. The
   caller converts the checkpoint slice of `pending_releases` into `Release` copies through the
   existing `pending_release_copies` shape (`own.rs:1044-1046`), which already reverses to
   newest-first and matches `Terminator::Return { releases: Vec<Release> }` (`ir.rs:71,468`).

**Emitter fallback (bounded).** AR-4 records emitter-side tracking as the fallback only if the
design probe shows the checker-local path cannot reference the condition temporary from body
returns. The probe confirms function-scope temp hoisting, so the checker-local path is the
implementation; the fallback is not needed and would only be reconsidered if the hoisting rule
changes.

## Implementation Details

### Integration Points

| Site | Change |
| ---- | ------ |
| `compiler/src/own/loops.rs` | Checkpoint, body-block capture, return collection, release prepend in `check_do_while` |
| `compiler/src/ir.rs` | `Builder::append_return_releases` (order-preserving prepend) |
| `compiler/src/own/liveness.rs` | Reused unchanged: the `successors` walk selects the body's reachable blocks |

### Behavior Rules

- A condition that materializes several temporaries releases all of them on an early return
  (ST-13).
- Fall-through, `break`, and `continue` paths keep their current releases; break/continue need no
  append because the exit drain and the back edge already handle them (ST-14).
- A nested do-while return releases both loops' condition temporaries (ST-15).
- Every temporary is released exactly once per path: the append skips places already present.
- A condition with no materialized temporaries appends nothing; the program is byte-identical to
  the pre-fix output.
- The fix changes no emitted C for a `do`-`while` without an early `return` (re-emission pins stay
  byte-identical).

## Code Examples

```clorraine
// The early return releases the condition temporary; the leak disappears.
func grow(): Array<String>[2] { /* ... */ }
func main(): i32 {
    let i: i32 = 0;
    do { i = i + 1; if (i == 2) { return i; } } while (grow()[0].count > 0);
    return 0;
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Early return in the body | Prepend the condition's new releases to each reachable body return | AR-4 |
| Multiple materialized condition temporaries | All new releases are captured by the checkpoint slice | AR-4 |
| Return inside a nested block or nested loop | Reachability walk includes it; nested loops add their own releases too | AR-4 |
| Condition with no materialized temporaries | Empty slice; no append; output unchanged | AR-4 |
| Release already present on the return | Skipped by the dedup check | AR-4 |
| `break`/`continue` | Unchanged; the exit drain and back edge already handle their releases | AR-9 |
| Temp declaration placement | Function-scope hoisting makes body returns safe; the bounded emitter fallback stays unused | AR-4 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry (AR #) that
> resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification cases ST-12 – ST-15 in
  [07-testing-strategy.md](07-testing-strategy.md): the early-return leak, multiple condition
  temporaries, fall-through/break regressions, and the nested do-while (each with a pinned exit
  code).
- Memory cases run with the `RDC_ASAN` opt-in and a host AddressSanitizer probe; the close-out
  runner includes a DEF-62 probe with a timeout and `handle_segv=0` (PF-025).
- Implementation tests: the appended release shape on the return arm, both loops' releases for a
  nested return, no append outside the body, and byte-identical re-emission.
- Full verification: `make check`.
