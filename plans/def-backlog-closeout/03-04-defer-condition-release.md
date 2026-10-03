# Defer Condition Release: Def Backlog Closeout

> **Document**: 03-04-defer-condition-release.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-68
> **AR**: AR-7

## Overview

A do-while body's condition is materialized after the body is lowered, so its release list is
appended to the body's reachable return terminators. A `return` inside a `defer` block lives in a
separate block that no CFG edge reaches from the body, so the condition temporary leaks. The fix
extends the existing successor walk to the registered defer blocks; the release-append mechanism
itself is unchanged.

## Architecture

### Current Architecture

- `check_defer` (`compiler/src/own/statements.rs:763-798`) creates the `defer_block`, emits the
  `Statement::Defer { block, scope, order }` marker (`:768-772`), lowers the deferred statements
  into the block (`:781-782`), and pushes a `DeferEntry` (`:793-797`; struct
  `compiler/src/own/flow.rs:215-224`).
- `check_do_while` (`compiler/src/own/loops.rs:77-162`) records `body_floor`, checks the body,
  lowers the condition, and calls `append_condition_releases` (`:143`).
- `append_condition_releases` (`:174-221`) slices new pending releases at `release_floor`, walks
  successors from `body_block` (`:183-204` via `compiler/src/own/liveness.rs:150-172`), and collects
  only blocks `>= body_floor` whose terminator is `Terminator::Return`.
- `Builder::append_return_releases` (`compiler/src/ir.rs:719-747`) prepends the releases, newest
  first, deduped by `Release.temp`.
- The defer block runs through the scope cleanup ladder (`compiler/src/codegen/cleanup.rs:615-636`),
  which is not a CFG edge; `check_defer_exit` (`compiler/src/own.rs:552-580`) re-checks the block on
  the real return edge against a cloned builder that is discarded.

### Proposed Changes

Extend `append_condition_releases` (`compiler/src/own/loops.rs:174-221`):

1. Keep the existing successor walk from `body_block`.
2. Additionally collect the `Statement::Defer { block, .. }` markers in every block reachable from
   `body_block` (the same reachable set the walk already visits).
3. For each defer block, walk its successors (`compiler/src/own/liveness.rs:150-172`) and union the
   blocks ending in `Terminator::Return` into the append list.
4. Call `Builder::append_return_releases` once with the deduped union.

`body_floor` and `release_floor` semantics stay: no release is appended to a return outside the
loop body. Deduplication is already handled by `Release.temp`. The fix must not rely on the cloned
builder in `check_defer_exit`.

## Implementation Details

### Integration Points

- The walk uses the existing reachability helper; no new traversal machinery.
- `while` bodies lower their condition before the body, so a deferred return already copies the
  release at registration; no change there (contrast case ST-18).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Return inside a `defer` block registered in the body | Union its return successors into the append | AR-7 |
| Direct early return | Existing walk unchanged | AR-7 |
| Return outside the loop (e.g. after the loop) | Excluded by `body_floor`/`release_floor` | AR-7 |
| Same return reachable both directly and through a defer | Deduped by `Release.temp` | AR-7 |
| Nested loops with defer blocks | Each loop's walk collects only its own body's markers | AR-7 |

> **Traceability:** Every strategy references the Ambiguity Register entry (AR #) that resolved it.
> See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-16 – ST-19 (`07-testing-strategy.md`).
- Implementation tests: the return terminator's release list for the deferred shape and the absence
  of an appended release on the post-loop return.
- Host-ASan probes: the DEF-68 program with a guarded condition and a deferred return.
