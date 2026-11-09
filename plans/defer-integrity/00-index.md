# Defer Integrity Implementation Plan

> **Feature**: Close the second batch of the open defect backlog in the Clorraine compiler: the five
> `defer` implementation defects — the undefined cleanup label when a deferred block contains
> `continue` (DEF-84), the deferred `return` self-looping its own cleanup rung (DEF-85), the defer
> skipped on the fall-through path when the scope also continues (DEF-86), the earlier returning
> defer nested inside a later defer's guard and never firing (DEF-87), and the deferred return
> reading its own local after the drop (DEF-88).
> **Status**: Done — plan and roadmap rows complete (2026-11-09 closeout).
> **Created**: 2026-10-09 15:32
> **Implements**: DEF-84, DEF-85, DEF-86, DEF-87, DEF-88
> **CodeOps Artifact Schema**: 1

## Overview

The def-backlog close-out review filed nine pre-existing cleanup-emission defects; the
slice-temporary-integrity plan closed seven others. This plan closes the `defer` family, the
second of the three named follow-up batches (AR-1). All five defects are 🟠 major, all were
reproduced at HEAD, and all live in one subsystem: the lowering of a deferred block
(`compiler/src/own/`) and its emission at cleanup sites (`compiler/src/codegen/`).

One architecture gap explains four of the five. A deferred block is lowered once and then
"copied into the ladder" at each cleanup site — but the copy is emitted as an isolated function
body: it resets the flow context (so a `continue` inside it becomes a `goto` to a label that may
never be printed, or drags the whole loop into the copy), it shares the emitter's function-global
printed-block set (so only the *first* cleanup site of a scope emits its defer and every other
exit edge silently skips it), and it has no knowledge of the cleanup currently executing (so a
`return` inside it jumps back to its own rung — an infinite loop). The fifth defect is an ordering
bug: a value return prints its trailing inline cleanup *before* the terminator's value store, so
the value expression reads its own dropped local. The fix threads one shared deferred-copy context
through the existing emitter — per-copy printed set and label namespace, ambient flow context,
bounded closure, and a running-cleanup frame with a continuation — plus a gated ordering change
for value returns. The frozen specification already defines the required behavior; no chapter or
golden changes (AR-2).

The design was resolved under the active auto-design delegation (AR-4–AR-8) with an independent
challenger that verified the per-defect alternatives are incorrect (they leak, collide labels, or
skip required cleanup) before converging on the shared model.

## Minimum-Sufficient Baseline

**Original goal:** Make every filed DEF-84 – DEF-88 reproduction behave per the frozen cleanup
rules — the strict C99 unit builds, every scope exit edge runs its deferred block once in reverse
registration order, a `continue`/`break` inside a defer reaches the loop's edge, a `return` inside
a defer evaluates its value, runs the remaining cleanup, and returns — with no hang, no segfault,
no leak, no double free, and byte-identical emission for programs that do not exercise the
defective shapes.

**Smallest viable design:** one deferred-copy emission context threaded through the existing
`emit_closure` / `emit_scope_inline` / `emit_actions` path, carrying (i) a per-copy printed-block
set and label namespace, (ii) the ambient `FlowContext`, (iii) the copy's closure bounded to the
deferred block's own block range, and (iv) a running-cleanup frame position whose continuation
runs the remaining actions of the scope before the outer rungs; plus one gated ordering change in
the block walk (a value return stores its value before its trailing inline cleanup). No new layer,
dependency, harness, or crate (AR-4, AR-14).

**Excluded machinery:** re-lowering the deferred body at each edge instead of copying it in the
emitter (AR-4 option C); a generalized cleanup framework or per-action label machine beyond the
continuation the filed shapes need (AR-4, AR-5); any `spec/` change (AR-2); the other two defect
batches and DEF-95 (AR-1).

**Approved complexity:** None — the challenger's missing-option search found no material support
surface; every change reuses the existing ownership/emission machinery (AR-14).

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Feature requirements and scope |
| 02  | [Current State](02-current-state.md) | The five reproduction mechanisms and the affected machinery |
| 03-01 | [Per-Site Deferred Copies](03-01-per-site-deferred-copies.md) | DEF-86, DEF-87: one copy per cleanup site |
| 03-02 | [Bounded Closure and Ambient Context](03-02-bounded-closure-and-context.md) | DEF-84: the copy's own region and loop edges |
| 03-03 | [Deferred Return Continuation and Ordering](03-03-deferred-return.md) | DEF-85, DEF-88: remaining actions, self-loop, value-before-drop |
| 07  | [Testing Strategy](07-testing-strategy.md) | ST cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Usage Examples

The five defect shapes are small programs; the filed reproductions live in
[02-current-state.md](02-current-state.md) §Reproductions. Representative contract:

```clorraine
func f(): i32 {
    let i: i32 = 0;
    do {
        defer { if (i == 1) { continue; } }   // DEF-84: builds and reaches the condition edge
        i = i + 1;
    } while (i < 3);
    return i;                                  // 3
}
```

```clorraine
func f(): i32 {
    let s: String = stringOf("abc");
    defer { return i32(len(s)); }              // DEF-85/88: reads s, runs remaining cleanup, returns 3
    return 0;
}
```

### Key Decisions

| Decision | Outcome | AR |
| -------- | ------- | -- |
| Batch scope | The defer family (DEF-84 – DEF-88); later batches untouched | AR-1 |
| Spec stance | Implementation-only; `spec/` untouched | AR-2 |
| Core design | One shared deferred-copy emission context, smallest threaded implementation | AR-4 |
| Return continuation | Remaining actions of the running scope, then the outer rungs | AR-5 |
| Loop edges | Ambient context + bounded closure + inline-only guard for rung-hosted break/continue | AR-6 |
| Every edge runs | One copy per cleanup site with per-copy namespaces | AR-7 |
| Value before drop | Value store before trailing inline cleanup for value returns | AR-8 |
| Execution mode | `--auto-commit` (user instruction) — commit and push after each verified task | — |

## Specialist Agents

**None** — the capability-gap check ran against this repository's evidence and found no gap a
standing specialist would close.

_Detection evidence: the change surface is the ownership/emission machinery already covered by the
installed project agents — `correctness-reviewer`, `memory-safety-auditor`,
`emission-integrity-auditor`, and `semantics-reviewer` (`.opencode/agents/`), dispatched per
`AGENTS.md` for ownership/drop/emission/language-rule changes. The defer semantics are fully
described by existing chapters (`spec/05-statements.md` §13/§15, `spec/17-codegen.md` §4), so no
domain knowledge beyond the standing reviewers is required; a dynamic packet carries the phase
diff and the design docs. No new recurring invariant, DSL, or review checklist was identified
(specialist-agents.md detection criteria all fail)._

## Related Files

- `compiler/src/own/statements.rs` — `check_defer` (784), `check_block` (89), `check_continue` (844), `check_return` (888)
- `compiler/src/own.rs` — `run_scope_cleanup` (482), `check_defer_exit` (574), `pop_scope` (464), `emit_return_cleanup` (1183), `DeferEntry` (194)
- `compiler/src/own/loops.rs` — loop frames, `loop_entry_state` (615), `materialize_owned_calls` (682)
- `compiler/src/codegen/cleanup.rs` — `collect_cleanup` (39), `emit_block_body` (122), `emit_actions` (237), `emit_ladder` (615), `emit_return` (255)
- `compiler/src/codegen/control.rs` — `emit_regions` (115), `emit_closure` (136), `emit_flow` (154), `emit_block_label` (236), `classify_jump` (243)
- `compiler/tests/defer_integrity_spec_test.rs`, `compiler/tests/defer_integrity_impl_test.rs` — the new suites
