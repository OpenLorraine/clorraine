# Worker faults: Amiga fault-exit cleanup

> **Document**: 03-03-worker-faults.md
> **Parent**: [Index](00-index.md)

## Overview

A runtime fault or panic inside a `createTask` worker must not run DOS or `exit()` on the
worker's plain Exec task. The worker records the fault and parks; the parent reports it. Workers
still alive at exit are removed without waiting (AR-3, AR-9 rules 6–8, AR-10).

## Architecture

### Current Architecture
[02 §What Exists](02-current-state.md#what-exists) (Worker tasks): the worker runs the entry,
signals, and parks; the join waits, then `RemTask`; `cl_trap`/`cl_panic` assume the main process.

### Proposed Changes

- **Worker list.** `cl_task` records are linked into a static worker list from `cl_create_task`
  (after `CreateTask` succeeds) until the join or exit removal frees them. Link and unlink run
  under `Forbid`/`Permit` because a worker reads the list in its fault path (Exec list-sharing
  rule). The worker list, not the registry, owns the records (AR-8).
- **Fault record.** `cl_task` gains the fault kind (none, trap, panic), the code pointer (static
  string), the message pointer and length, and a reported flag. These fields are written only by
  the worker before it signals, and read only by the parent after the signal.
- **Detection.** `cl_trap` and `cl_panic` first test the worker-list head, one load and compare
  of a static (AR-18). Only when it is non-null do they call the out-of-line worker path. Neither
  function calls or names the exit hook ([03-01 §Re-entry](03-01-exit-registry.md#re-entry)). If
  ST-9 or the bench guard shows the worker path linked into programs that create no task, route
  the call through one static function pointer that `cl_create_task` sets. That keeps the check
  at one load and compare.
- **Worker path.** Under `Forbid`, compare `FindTask(NULL)` with each record's `task` pointer,
  never dereferencing `tc_UserData` (AR-9 rule 7). The walk needs `Forbid` because the parent
  runs at a higher priority: it can preempt the walk, join another worker, and `FreeMem` the
  record being read. No match: `Permit` and take today's path unchanged. On a match, `Permit`,
  then store the kind, code, and message. The panic text is kept as pointer and length; it lives
  in the worker's stack or module data and stays valid until `RemTask` (AR-9 rule 8). The parent
  cannot free the matched record before this worker signals. Then `Signal(parent, 1 << signal_bit)`
  and `Wait(0)`. No stdio, no DOS, no `exit` (AR-3).

### Report at join

`cl_task_join_record` (reached from `joinTask` and the generated drop `cl_task_free`) waits as in
[03-04 §Task join](03-04-ctrl-c.md#breakable-waits). When the record holds a fault, the parent:

1. marks the record reported first, so a nested `exit` during the print (libnix's stdio break
   check) does not print the line a second time in phase 2;
2. prints exactly the line the main path would print: `error CODE: message` for a trap, or
   `panic: ` plus the message bytes for a panic, with the same writer and flush as `cl_trap` and
   `cl_panic`;
3. calls `exit(20)`.

The worker stays in the list, so exit phase 2 removes it. It is removed after the line is printed,
as rule 8 requires (AR-3, AR-9 rule 8). A recorded fault takes precedence over a break that
arrives in the same wait (AR-13c).

A worker without a fault keeps today's join: `Forbid`, `RemTask`, `Permit`, `FreeSignal`, unlink,
`FreeMem`, and return the result.

### Exit removal

Exit phase 2 ([03-01 §Exit phases](03-01-exit-registry.md#exit-phases)) handles every record still
in the worker list, newest first:

1. If the record holds a fault that was not reported, mark it reported, then print its line
   (AR-10). It follows any line the main path already printed, and the exit code is unchanged.
   Marking first keeps the pop-or-mark-before-act rule
   ([03-01 §Re-entry](03-01-exit-registry.md#re-entry)). A nested `exit` during the print
   re-runs phase 2, which then skips the print and goes on to remove the worker.
2. `Forbid`, `RemTask(task)`, `Permit`.
3. `FreeSignal(signal_bit)`, then unlink the record under `Forbid` and `FreeMem` it.

It never waits (AR-9 rule 6). A worker the parent never joined is therefore removed whether it is
parked, ready, or still running at priority -1.

### When a worker fault is reported

| Situation | Report | Exit code | AR |
|---|---|---|---|
| Parent joins (explicit or drop) after the worker faulted | at join | 20 | AR-3 |
| Parent faults, panics, or breaks first | phase 2, after the main line | 20 (main path) | AR-10 |
| Two workers fault; the parent joins the first | first at join; second in phase 2 | 20 | AR-10 |

A global cannot own a `Task` (E11106), so every unjoined worker belongs to a scope whose drop
joins on a normal exit (register note on AR-2 and AR-10).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| Fault in a worker | record, signal, `Wait(0)`; parent reports | AR-3 |
| Fault in the main task while workers are parked | main line, then phase 2 reports and removes them | AR-10 |
| Fault in interrupt context | Out of scope. `FindTask(NULL)` names the interrupted task, so the behaviour is undefined as it is today. Documented. | AR-4 |

## Testing Requirements

- Spec: ST-14 and ST-18 (FS-UAE A500) in [07](07-testing-strategy.md).
- Impl: the worker-list shape; the detection compare runs under `Forbid` with no `tc_UserData`
  dereference; `cl_trap`/`cl_panic` name no hook symbol; in join and in phase 2 the record is
  marked reported before printing, and the print comes before `RemTask`.
