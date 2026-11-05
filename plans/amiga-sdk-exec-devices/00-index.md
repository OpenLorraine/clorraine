# Amiga SDK Bundle 7: Exec and Devices Implementation Plan

> **Feature**: Worker tasks, signals, message ports, libraries, device I/O, interrupt servers, and input/keyboard/console devices on the A500/KS1.3 surface, proven by a worker-task + async-I/O demo.
> **Status**: Done — 18/18; Bundle 7 accepted on the A500/KS1.3, issue #8 closed, independent review fixes complete
> **Created**: 2026-11-05 18:50
> **Last Updated**: 2026-11-05 20:50
> **Implements**: —
> **CodeOps Artifact Schema**: 1

## Overview

Bundle 7 extends `std.amiga.exec` from private message ports, messages, and
signals to the rest of the Kickstart 1.3 Exec surface the A500 baseline needs:
worker tasks with join, library open/close/version, generic device I/O
requests (`DoIO`, `SendIO`, `WaitIO`, `CheckIO`, `AbortIO`), and the devices
that carry input, keyboard, and console traffic. It also extends
`std.amiga.interrupt` with a second interrupt server for the ports interrupt.

The plan implements SDK-EXE-001 through SDK-EXE-006, SDK-EXE-008, and
SDK-EXE-009. The three advanced registry rows (signal semaphores 2.0+, hooks
2.0+, and the utility library 2.0+) stay deferred because the acceptance
baseline is KS1.3.

## Minimum-Sufficient Baseline

**Original goal:** add the remaining core Exec and device surface on the
A500/KS1.3, with a worker-task and asynchronous-I/O demo.

**Smallest viable design:** keep the existing private-port and message
surface; add one task handle backed by `CreateTask`/`RemTask` and one signal
bit for join; wrap `OpenLibrary`/`CloseLibrary`; add one generic device handle
backed by a single `IOStdReq` and one typed `openConsole(window, unit)` helper
because `console.device` needs an Intuition `Window`; add one ports-interrupt
slot beside the existing VBlank slot; prove the surface with a worker stress
loop, two trackdisk reads, a console device write and aborted read on a small
window, and input/keyboard device open/close. Reuse the existing ADF boot,
report-file handshake, and line-crop golden pattern.

**Excluded machinery:** public named ports, DOS processes
(`CreateNewProc`), signal semaphores, hooks, the utility library, typed
console/keyboard modules, device-specific request structs (timer and audio
keep their existing modules), task cancellation, and any language-level
concurrency. No language, grammar, diagnostic, or ABI change.

**Approved complexity:** none beyond the direct surface above.

## Document Index

| #   | Document                                                   | Description                                 |
| --- | ---------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)             | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                        | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                         | Feature requirements and scope              |
| 02  | [Current State](02-current-state.md)                       | Analysis of the current Exec surface        |
| 03  | [Tasks and Signals](03-01-tasks-and-signals.md)             | Worker task lifecycle, join, and ownership  |
| 04  | [Libraries and Devices](03-02-libraries-and-devices.md)     | Library and generic device I/O surface      |
| 05  | [Interrupts and Input](03-03-interrupts-and-input.md)       | Port interrupt server and device coverage   |
| 06  | [Demo and A500 Smoke](03-04-demo-and-smoke.md)              | Worker/async-I/O demo and acceptance gate   |
| 05  | [Findings](05-findings.md)                                  | Implementation and closeout findings        |
| 07  | [Testing Strategy](07-testing-strategy.md)                  | Spec cases, impl cases, and verification    |
| 99  | [Execution Plan](99-execution-plan.md)                      | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

```clorraine
import { Task, createTask, joinTask, reportTaskResult } from std.amiga.exec;
import { println } from std.io;

func worker(): void {
    reportTaskResult(55);
}

func main(): i32 {
    let task: Task? = createTask(worker, 8192);
    if (task == null) { return 1; }
    println($"worker: {joinTask(task)}");
    return 0;
}
```

```clorraine
import { Library, openLibrary, closeLibrary, libraryVersion } from std.amiga.exec;
import { stringOf, toCString } from std.text;

func version(): u16 {
    let library: Library? = openLibrary(toCString(stringOf("exec.library")), 33);
    if (library == null) { return 0; }
    let version: u16 = libraryVersion(library);
    closeLibrary(library);
    return version;
}
```

```clorraine
import { Device, CMD_READ, openDevice, closeDevice, deviceCommand,
         deviceSetBuffer, deviceSetOffset, deviceDoIO, deviceActual } from std.amiga.exec;
import { stringOf, toCString } from std.text;

func readBootBlock(borrow buf: u8[]): u32 {
    let device: Device? = openDevice(toCString(stringOf("trackdisk.device")), 0, 0);
    if (device == null) { return 0; }
    deviceCommand(device, CMD_READ);
    deviceSetBuffer(device, borrow buf);
    deviceSetOffset(device, 0);
    let error: i32 = deviceDoIO(device);
    let actual: u32 = deviceActual(device);
    closeDevice(device);
    return error == 0 ? actual : 0;
}
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Core rows | 8 done, 3 advanced 2.0+ rows deferred (AR-1) |
| Task model | Creator at priority >= 0; `CreateTask` at -1; worker signals then `Wait(0)`; parent removes with `Forbid`/`RemTask`/`Permit` (AR-3) |
| Task result | `reportTaskResult` stores in the current task's `tc_UserData` (AR-4) |
| Device model | One `IOStdReq`-sized device plus one reply port and an optional device-owned async buffer; sync calls borrow the caller buffer (AR-8) |
| Console | `openConsole(window, unit)`; the window is refcounted and closes after the console (AR-12) |
| Interrupts | One VBlank slot and one ports slot; shared token counter (AR-11) |
| Demo | Worker stress loop with a dropped task + trackdisk/console/keyboard/input I/O; console-read abort; generated drops; `task 464`, `io 13`, `key -2` (AR-13) |
| Gate | ADF report file plus a non-blank 620x22 line-crop golden (AR-14) |

## Specialist Agents

**None** — the capability-gap check found no gap a standing specialist would
close. The phase reviews use the existing correctness reviewer plus the
read-only memory-safety auditor, because the change adds C-shim ownership
(task records, signal bits, device requests) without changing the compiler
semantics. The emission-integrity auditor is not dispatched: the bundle adds
stdlib wrappers and C shim code but no IR, codegen, or symbol-name change.

_Detection evidence: `compiler/runtime/shim_hosted_amiga.c` owns the C
resources; `stdlib/std.amiga.exec.clo` and `stdlib/std.amiga.interrupt.clo`
are the wrappers; `compiler/src/profile.rs:86-112` adds `std.amiga.exec` to the
1.3 list and `compiler/src/stdlib.rs` updates the export table; no
`compiler/src/{ir,codegen}` file is touched._

## Related Files

- `stdlib/std.amiga.exec.clo` — task, library, and device surface
- `stdlib/std.amiga.interrupt.clo` — port interrupt handler
- `compiler/runtime/shim_hosted_amiga.c` — C shim implementations
- `compiler/src/profile.rs` — enable `std.amiga.exec` on `hosted-amiga-13`
- `compiler/src/stdlib.rs` — export table
- `compiler/tests/amiga_exec_spec_test.rs` — specification suite
- `compiler/tests/amiga_exec_impl_test.rs` — new implementation suite
- `examples/amiga/exec-demo.clo` — A500 acceptance demo
- `docker/scripts/a500-exec-smoke.sh` — acceptance harness
- `docs/amiga-sdk/a500-exec.png` — line-crop golden
- `spec/appendix-amiga.md`, `spec/appendix-stdlib.md` — frozen appendices
