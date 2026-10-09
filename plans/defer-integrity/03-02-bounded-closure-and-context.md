# Bounded Closure and Ambient Context: Defer Integrity

> **Document**: 03-02-bounded-closure-and-context.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-84 (plus the rung-hosted sibling, guarded)
> **AR**: AR-4, AR-6, AR-10

## Overview

A `continue` or `break` inside a deferred block must reach the loop's corresponding exit edge. The
filed do-while shape emits `goto __cl_b2` to a label that is never printed (the strict C build
fails); the while shape re-enters the loop emission and swallows the whole loop; the for shape
prints outer blocks inside the copy. Two root causes: `emit_closure` resets the flow context to
`FlowContext::default()` (`compiler/src/codegen/control.rs:140`), so out-edges cannot classify as
`Continue`/`Break`; and the closure follows the out-edge into the whole loop
(`cleanup.rs:693-705`), so the copy's fallback walk emits outer blocks. This component threads the
ambient context into the copy and bounds the copy to the deferred block's own block range.

## Architecture

### Current Architecture

- `emit_flow` owns the `FlowContext` (`control.rs:154-177`, `classify_jump` 243-262) used for
  structured `continue`/`break` emission.
- `emit_closure` is called from `emit_actions`/`emit_defer` (no context parameter) and hard-codes
  `FlowContext::default()`; its reachable set is the unbounded successor closure of the defer root.
- Out-edge degenerates: `Jump::Plain` → `emit_goto(target)` (`control.rs:265-268`), which records a
  `pending_labels` entry; if the target block was already printed (the do-while condition block),
  the label is never printed. If the target is a `loop_entry` key (the while head), the copy
  re-enters `emit_loop`; if it is neither printed yet nor a loop entry (the for step), the label is
  printed inside the copy — mangled but compiling.

### Proposed Changes

1. **Record the deferred block's own range at lowering.** `check_defer`
   (`compiler/src/own/statements.rs:784-819`) knows the block ids allocated while lowering the
   body; the implementation records the range `[root, end)` (contiguous in the block arena — the
   walk is single-threaded) and carries it to the emitter through the `Defer` marker and the
   `Defer` action. Nested deferred bodies inside the range are part of it. This is the only IR
   schema addition; keep it as small as the mechanism allows (an end id beside the root).
2. **Bound the copy.** `emit_closure` walks only blocks inside the recorded range (the reachable
   subset of the range from the root); the fallback loop iterates the range, never the whole
   function. Out-edge targets (outside the range) are never emitted into the copy.
3. **Thread the ambient flow context.** The cleanup call chain gains the enclosing `FlowContext`:
   `emit_flow` → `emit_block_body` → `emit_scope_inline` / `emit_actions` → `emit_defer` →
   `emit_closure`. A copy emitted inside a loop body sees the loop's continue/break targets, so a
   `continue`/`break` out-edge classifies as `Continue`/`Break` and prints the C statement.
   Rung activations (`emit_ladder`) have no enclosing loop context; a rung-hosted copy must not
   contain a `break`/`continue` out-edge — enforced by the guard below.
4. **Inline-only guard for rung-hosted out-edges.** When `collect_cleanup` computes rungs, a scope
   whose deferred regions contain a `Goto` to a block **outside** the region (a break/continue
   out-edge) is excluded from the rung set: its cleanup runs inline at each exit edge, where the
   ambient context is available (inside the switch/loop construct; real C `break`/`continue` bind
   correctly — the switch emitter prints a real C `switch` and sets the in-switch context,
   `control.rs:443-490`). This closes the rung-hosted sibling with the smallest rule; the residual
   is reported, not claimed (AR-6).

## Implementation Details

### New Types/Interfaces

- The deferred range: an `end` block id carried with the `Defer` IR statement and the `Action::Defer`
  variant (`compiler/src/ir.rs`, `compiler/src/codegen/cleanup.rs`). Contract: `root` ≤ blocks <
  `end` are the body's own blocks; out-edge targets are outside.

### New Functions/Methods

- `Emitter::emit_closure(function, root, range, ctx)` — bounded walk with the ambient context.
- `classify_jump` gains the ambient context for copies (unchanged signature; the context reaches
  it through the copy's `FlowContext`).
- `collect_cleanup` — the rung filter gains the out-edge check over each scope's deferred regions.

### Integration Points

- `prepare_regions` (`control.rs`) already maps loop entries; the bounded copy no longer re-enters
  `emit_loop` for out-edge targets because they are classified before the loop-entry check.
- The label mechanism (`pending_labels`, `emit_block_label`) is unchanged; with the bounded range,
  a `Plain` out-edge can no longer point at an already-printed block without its label — if one
  ever occurs, the strict build fails loudly and the verification probes catch it.

## Code Examples

### Example 1: the filed do-while shape after the fix

```c
do {
    defer { if (i == 1) { continue; } }   /* the copy sees the do-while continue target */
    i = i + 1;
} while ((i < 3));
return i;                                  /* 3; the copy is bounded, no loop inside it */
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Out-edge target inside the copy range | Structured emission as today (internal branch/join) | — |
| Out-edge target outside the range in an inline copy | Classified `Continue`/`Break` via the ambient context; plain C statement | AR-6 |
| Out-edge target outside the range in a rung copy | Prevented by forcing the scope's cleanup inline (the guard) | AR-6 |
| A `Plain` out-edge remains after bounding | Loud strict-C failure; the verification probe records it; fallback is to file the residual | AR-6 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-5 (filed do-while continue — strict build + exit 3), ST-6 (while
  continue), ST-7 (break inside a defer) — see [07-testing-strategy.md](07-testing-strategy.md).
- Implementation tests: bounded-copy shape (no outer block printed inside the copy), the
  `continue;`/`break;` spelling, and byte-identical re-emission.
- Verification probe: the rung-hosted sibling (switch-arm defer with a `break` in its body and an
  early edge on the arm scope) builds strict and runs; the phase records the outcome.
