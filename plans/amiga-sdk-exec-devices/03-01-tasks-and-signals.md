# Design: Tasks and Signals

> **Document**: 03-01-tasks-and-signals.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1, R2, R9 (task part), AR-2–AR-6, AR-15

## Public API

```clorraine
module std.amiga.exec;

export handle Task freedBy cl_task_free;

ffi stdexec {
    func cl_create_task(entry: func(): void, stack_size: u32): Task?;
    func cl_task_result(value: i32): void;
    func cl_join_task(take task: Task): i32;
    func cl_task_free(take task: Task): void;
}

export func createTask(entry: func(): void, stackBytes: u32): Task? {
    return cl_create_task(entry, stackBytes);
}

export func reportTaskResult(value: i32): void {
    cl_task_result(value);
}

export func joinTask(take task: Task): i32 {
    return cl_join_task(task);
}
```

The existing `MsgPort`, `NewMessage`, `Message`, and their wrappers are
unchanged. `std.amiga.exec` is added to the `hosted-amiga-13` module list; the
host profile still rejects it with `E11200`.

## Task Record

The shim owns one record per task:

```c
typedef struct cl_task {
    ULONG magic;            /* CL_TASK_MAGIC while the record is live */
    struct Task *task;      /* the exec task created by CreateTask */
    struct Task *parent;    /* the task that called createTask */
    cl_handler_fn entry;    /* the capture-less Clorraine callback */
    BYTE signal_bit;        /* AllocSignal(-1), used for join */
    int joined;             /* the signal bit has been freed */
    LONG result;            /* reportTaskResult value */
} cl_task;
```

The record is allocated with `MEMF_PUBLIC | MEMF_CLEAR` because both tasks
read it. `magic` guards `reportTaskResult` against a task that was not created
by the module.

## Creation

`cl_create_task(entry, stack_size)`:

1. Reject a null entry or `stack_size` outside 4096..65536 with `NULL`.
2. Allocate the record; on failure return `NULL`.
3. `AllocSignal(-1)`; a negative result frees the record and returns `NULL`.
4. Record the parent from `FindTask(NULL)`, the signal bit, and the entry.
5. `CreateTask("clorraine.worker", -1, cl_task_entry, stack_size)`.
   A null result frees the signal and the record and returns `NULL`.
6. Store the record in `task->tc_UserData` and return it.

The worker priority is -1. The normal Clorraine program runs as a DOS process
at priority 0, so the worker cannot preempt the parent before
`task->tc_UserData` is set. The worker runs when the parent waits in
`joinTask` or at scope end.

## Entry, Signal, and Removal

```c
static void cl_task_entry(void)
{
    struct Task *self = FindTask(NULL);
    cl_task *record = (cl_task *)self->tc_UserData;
    if (record != NULL && record->magic == CL_TASK_MAGIC &&
        record->entry != NULL) {
        record->entry();
    }
    if (record != NULL && record->magic == CL_TASK_MAGIC) {
        Signal(record->parent, 1UL << record->signal_bit);
    }
    Wait(0);   /* the parent removes this task */
}
```

The worker signals *after* the entry returns, then blocks in `Wait(0)`. The
parent waits on `1UL << signal_bit`, then removes the worker:

```c
Wait(1UL << record->signal_bit);
record->joined = 1;
Forbid();
RemTask(record->task);
Permit();
FreeSignal(record->signal_bit);
```

The `Forbid`/`Permit` bracket is required on Kickstart 1.3: removing another
task created with `CreateTask` has a pre-V36 crash risk without it. The worker
has signaled and then parks in `Wait(0)`; the parent's `Forbid` also prevents
a worker that was preempted inside `Signal` from running while `RemTask`
removes it. `cl_join_task` then copies the result, clears the magic, and frees
the record. `cl_task_free` runs the same removal path without returning the
result; it is the generated drop for a task the caller never joined.

The worker signals after the entry returns, so `reportTaskResult` always lands
before the parent wakes. The worker does not touch the record after `Signal`;
the parent removes and frees it. This is race-free on the single-CPU target and
does not unload program text while the worker is still running.

## Worker Rules

A worker created by `createTask` is a plain Exec task, not a DOS process. Its
entry must:

- use only arithmetic, module-level scalars, and `reportTaskResult`;
- return promptly and must not block;
- not call DOS, Intuition, `std.io`, `std.amiga.dos`, `std.amiga.intuition`,
  or any other library that needs a DOS process;
- not allocate or drop owning values;
- not read or write the parent's handles.

The shim's final `Wait(0)` is outside the entry and is the only blocking call
in the worker path. `reportTaskResult` is a no-op outside a task created by the
module. The compiler cannot enforce these rules; they are documented in
`docs/known-limitations.md` and audited.

## Failure and Ownership

| Situation | Result |
|---|---|
| null entry, bad stack size | `null`; no allocation |
| record allocation fails | `null` |
| `AllocSignal` fails | `null`; record freed |
| `CreateTask` fails | `null`; signal and record freed |
| `joinTask` / drop | waits, removes the worker, frees the signal and record exactly once |
| `reportTaskResult` outside a created task | no-op |
| worker entry never returns | join/drop blocks; the caller owns the contract |
| `createTask` result ignored | the generated drop joins at scope end |

No public call panics. The handle is move-only; `joinTask` consumes it.

## Test Hooks

The specification test compiles a program that creates a task, calls
`reportTaskResult`, and joins it on both hosted profiles. The A500 demo runs a
stress loop of eight `workerSum` workers plus one `workerProduct`, creating and
joining one at a time, so the create/signal/`Forbid`+`RemTask` path runs nine
times on the real machine. The implementation test pins the emitted shim
calls, the generated drop routing, and the record/removal branches; it does
not run a task on the host profile because the host has no Exec.
