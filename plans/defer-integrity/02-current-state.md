# Current State: Defer Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

A `defer` registers a `Statement::Defer { block, scope, order }` marker at its site and lowers its
body once into a dedicated IR block (`compiler/src/own/statements.rs:784-819`). Each scope's
cleanup site — the fall-through `ScopeEnd`, an early-edge `ScopeExit`, or a function-level ladder
rung `__cl_cN` — runs that scope's collected actions (drops and deferred copies) in reverse
registration order (`compiler/src/codegen/cleanup.rs:39-86`, `:122-184`, `:237-244`). A deferred
copy is emitted by `emit_closure` (`compiler/src/codegen/control.rs:136-148`), which computes the
transitive successor closure of the defer root, resets the flow context to
`FlowContext::default()` (`control.rs:140`), and shares the function-global `self.emitted` set
(`control.rs:166-168`). A `return` inside a defer emits `ScopeExit { owning scope }` markers via
`emit_return_cleanup` (`own/statements.rs:996`; `own.rs:1183-1193`), which the emitter maps to the
owning scope's rung.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/statements.rs` | `check_defer`, `check_block`, `check_continue`, `check_return` | Record the deferred block's own block range; keep the defer entry across exit edges |
| `compiler/src/own.rs` | `run_scope_cleanup`, `check_defer_exit`, `emit_return_cleanup` | Per-edge effect checking; no change to the cleanup-order model |
| `compiler/src/codegen/control.rs` | `emit_closure`, `emit_flow`, `emit_block_label`, `classify_jump`, `emit_goto` | Thread the copy context; bound the closure; per-copy label spelling |
| `compiler/src/codegen/cleanup.rs` | `collect_cleanup`, `emit_block_body`, `emit_actions`, `emit_ladder`, `emit_return` | Per-site copies; running-cleanup frame and continuation; value-before-cleanup ordering |
| `compiler/src/codegen/function.rs` | The structured body emission | Pass the copy/continuation context where the cleanup path calls in |
| `compiler/src/ir.rs` | IR definitions | The deferred block's own range (for the bounded closure), if not carried beside the IR |
| `compiler/tests/defer_integrity_spec_test.rs` | New specification suite | Create |
| `compiler/tests/defer_integrity_impl_test.rs` | New implementation suite | Create |

### Code Analysis

The emitter call path for a cleanup site is singular: `emit_flow` → `emit_block_body`
(`control.rs:177`) → `emit_scope_inline` / `emit_actions` (`cleanup.rs:195-244`) → `emit_defer` →
`emit_closure`. All line references are at the plan baseline tree (recorded in
[99-execution-plan.md](99-execution-plan.md) Phase 1).

## Gaps Identified

### Gap 1: The copy resets the flow context (DEF-84)

`emit_closure` hard-codes `FlowContext::default()` (`control.rs:140`), so a `goto` from inside the
deferred block to the loop's continue/break block cannot classify as `Continue`/`Break`
(`classify_jump`, `control.rs:243-262`). It degrades to `Jump::Plain` → `goto __cl_bN`
(`control.rs:265-268`); if the target block was already printed, the label is never emitted (the
filed do-while shape: strict build fails with `label '__cl_b2' used but not defined`). In the
while shape the target is the loop head, which is a `loop_entry` key, so the copy re-enters
`emit_loop` and swallows the loop; in the for shape the step block gets printed inside the copy.
Compounding it, `closure_from` (`cleanup.rs:693-705`) follows the out-edge into the whole loop, so
the fallback loop (`control.rs:142-146`) emits outer blocks inside the copy.

### Gap 2: One copy per deferred root, not per cleanup site (DEF-86, DEF-87)

`emit_closure` shares the function-global `self.emitted` set; the first cleanup site marks the
defer's blocks printed and every later site's copy emits nothing. Traced example (DEF-86): the
continue edge emits the copy, the fall-through `ScopeEnd` site's `emit_defer` runs with
`already_emitted=[4,5,6,7]` and produces no code — the deferred return guarded on the later
iteration never fires (exit 0 instead of 9). The same mechanism explains DEF-87: the later defer's
guard-return edge inlines the scope's cleanup (emitting the earlier defer inside the guard); the
genuine outer site is then suppressed, so with the guard false the earlier returning defer never
runs (exit 0 instead of 7). An unfiled sibling of the same family: an early `return` in a scope
that also falls through jumps to a cleanup rung that emits no deferred code.

### Gap 3: A deferred return jumps to its own rung (DEF-85)

`check_return` inside a deferred block runs `emit_return_cleanup()`, which walks **every** open
scope and emits `ScopeExit` markers including the defer's owning scope
(`own/statements.rs:996`; `own.rs:1183-1193`). At the cleanup site the emitter maps those to the
owning scope's rung, so a defer emitted at rung `__cl_cK` returns via `goto __cl_cK` — inside
rung `__cl_cK` (`cleanup.rs:181-184`, `:376-377`). The filed function-scope shape hangs (exit
124); the switch-arm form hangs inside the arm. Skipping the owning rung is **not** sufficient:
with `[Defer, Drop(a)]` in one rung, the drop runs after the defer and must still run on the
return path (probe `d85c`).

### Gap 4: Trailing inline cleanup prints before the value store (DEF-88, and the loop-local root cause)

`emit_block_body` prints inline cleanup during the statement walk (`cleanup.rs:134-153`,
`:195-199`); a `Return` terminator's value store is printed later by `emit_return`
(`control.rs:226-228`; `cleanup.rs:321-373`). The IR for the filed shape is
`ScopeExit{2}, Drop{s, scope:2}, ScopeExit{1}, ScopeExit{0}, Return{value: len(s)}` — the drop is
printed before the read, so the program reads the nulled handle (exit 139). The defer block's own
scope has no ladder rung (its `ScopeExit` markers live only inside the unreachable defer block,
which `collect_cleanup`'s reachable filter excludes, `cleanup.rs:54-65`), so its cleanup runs
inline. The same ordering defect reproduces with **no defer at all** for a return inside a loop
body reading a loop-local (`d88c`, exit 139): one root cause, two shapes (AR-8). The rung path
already orders correctly (`d88b`: `__cl_ret = …; goto __cl_c1;` with the drop on the rung), which
is the shape the store-before-release pins protect.

## Reproductions (baseline `f558b8b`; probes under `build/defer-probe/`)

| Defect | Probe | Program core | Expected | Observed |
| ------ | ----- | ------------ | -------- | -------- |
| DEF-84 | `d84b.clo` | `do { defer { if (i == 1) { continue; } } i = i + 1; } while (i < 3); return i;` | strict build + 3 | strict build fails: `label '__cl_b2' used but not defined` |
| DEF-84 | `d84.clo` / `d84c.clo` | the `while` / `for` forms | 3 | 1 (loop swallowed into the copy) / 3 with mangled C |
| DEF-85 | `d85.clo` | `defer { return 7; } return 0;` | 7 | hang (124) |
| DEF-85 | `d85b.clo` | the same inside a `switch` arm | 7 | hang (124) |
| DEF-85 | `d85c.clo` | `let a: String = …; defer { return i32(len(a)); }` | 3, ASan clean | hang (124) |
| DEF-86 | `d86.clo` | `while (i < 2) { defer { if (i == 2) { return 9; } } i = i + 1; if (i == 1) { continue; } }` | 9 | 0 |
| DEF-87 | `d87d.clo` | block body; defer1 `return 7`; defer2 `if (i == 99) { return 0; }` | 7 | 0 |
| DEF-88 | `d88.clo` | `defer { let s: String = stringOf("x"); return i32(len(s)); }` | 1, ASan clean | segfault (139) |
| DEF-88 (root cause) | `d88c.clo` | a return in a loop body reading a loop-local `String`, no defer | 1 | segfault (139) |

The filed mechanisms match the probes (`build/defer-probe/{d84b,d85,d85b,d85c,d86,d87d,d88,d88c}.clo`
with emitted `.c`, IR dumps, and emission traces retained).

## Dependencies

### Internal Dependencies

- The scope-action/rung model (`collect_cleanup`, `scope_actions`, `labels`, `rungs`) is reused
  unchanged; the design threads context through it rather than replacing it (AR-4).
- The label preparation pass (`control.rs:83-89` `pending_labels`) and the emission pass must
  agree on copy numbering (AR-4 reopen trigger).

### External Dependencies

- None. The pinned container toolchain and host compilers are already used by the existing suites.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Per-site copies change bytes for single-site defers | Low | Medium | First copy of each block keeps its `__cl_b{id}` spelling; the emission-integrity audit classifies every base-vs-head diff |
| Copy/label numbering non-deterministic across the two passes | Medium | High | Pin the numbering scheme in [03-01](03-01-per-site-deferred-copies.md); the determinism spec suite and byte-identical re-emission tests are the oracle |
| The continuation emits remaining actions twice or zero times | Medium | High | The frame position excludes the running action; host-ASan probes count releases and side effects; ST cases pin the observable values |
| The ordering change alters working value-return programs | Medium | Medium | Gate to `Return`-terminated blocks with trailing inline cleanup; store-before-release pins stay green; the emission auditor traces every diff |
| The rung-hosted break/continue sibling resists the guard | Low | Medium | The guard forces inline cleanup; the Phase 2 verification probe records the outcome; fallback is to file the residual shape |
