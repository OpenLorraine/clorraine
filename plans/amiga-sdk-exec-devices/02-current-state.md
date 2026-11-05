# Current State: Amiga SDK Bundle 7 — Exec and Devices

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)
> **Analysis date**: 2026-11-05

## Existing Implementation

### What Exists

| Area | Current state | Evidence |
|---|---|---|
| `std.amiga.exec` | Private message ports, messages, signals, and message drop routing exist. | `stdlib/std.amiga.exec.clo`; `compiler/tests/amiga_exec_spec_test.rs` |
| Exec shim | `cl_create_msg_port`, `cl_delete_msg_port`, `cl_create_message`, `cl_message_payload`, `cl_put_msg`, `cl_signals`, `cl_wait_signals`, `cl_get_msg`, `cl_reply_msg`, `cl_free_message`, `cl_release_message`. | `compiler/runtime/shim_hosted_amiga.c:2562-2660` |
| Interrupts | One VBlank handler slot with a token and idempotent removal. | `stdlib/std.amiga.interrupt.clo`; `compiler/runtime/shim_hosted_amiga.c:580-632` |
| 1.3 gating | `std.amiga.exec` is rejected on `hosted-amiga-13`; the comment says it arrives with the Exec bundle. | `compiler/src/profile.rs:86-112` |
| Export table | The exec export table lists the existing 14 names. | `compiler/src/stdlib.rs:440-466` |
| Hosted-amiga runtimes | `exec-self-test` and `message-drop` run under `vamos` with fixed transcripts. | `examples/amiga/exec-self-test.clo`, `examples/amiga/message-drop.clo` |
| Device I/O | `audio.device`, `timer.device`, and the gameport have typed modules; there is no generic device handle. | `stdlib/std.amiga.audio.clo`, `stdlib/std.amiga.timer.clo`, `stdlib/std.amiga.input.clo` |
| Libraries | `graphics.library`, `intuition.library`, `diskfont.library`, and `layers.library` are opened inside the shim; there is no public library handle. | `compiler/runtime/shim_hosted_amiga.c:1068-1072`, `:4794`, `:4911` |
| Tasks | No task wrapper; the language has no concurrency surface. | `stdlib/` has no task API |

### Relevant Files

| File | Role |
|---|---|
| `stdlib/std.amiga.exec.clo` | Public exec module; gains the task, library, and device handles |
| `stdlib/std.amiga.interrupt.clo` | Public interrupt module; gains the ports handler |
| `compiler/runtime/shim_hosted_amiga.c` | C shim; gains the task, library, device, and port-interrupt code |
| `compiler/src/profile.rs` | Profile gate; adds `std.amiga.exec` to the 1.3 list |
| `compiler/src/stdlib.rs` | Export table; lists every new export |
| `compiler/tests/amiga_exec_spec_test.rs` | Specification suite; extended for the new surface |
| `compiler/tests/amiga_exec_impl_test.rs` | New implementation suite |
| `examples/amiga/exec-demo.clo` | New A500 acceptance demo |
| `docker/scripts/a500-exec-smoke.sh` | New A500 gate |
| `Makefile` | New `sdk-a500-exec` target and `verify` leg |
| `spec/appendix-amiga.md` | Exec/async documentation; remove the old 1.3 exclusion |
| `spec/appendix-stdlib.md` | §7 (interrupts) and §13 (exec) documentation |
| `docs/amiga-sdk/registry.tsv` | Bundle 7 rows 58–68; status/acceptance/evidence/spec columns |
| `docs/known-limitations.md` | Accepted worker-task restrictions |

### Code Analysis

The existing exec module is a thin wrapper: every public function calls one
`ffi` function, the shim names the Exec primitive, and the handles are opaque
`void *` values in the generated C. The module has no ownership logic in
Clorraine beyond `export handle ... freedBy ...`; the shim owns the records and
the generated drop calls the deleter exactly once. This pattern extends
cleanly to tasks, libraries, and devices.

The interrupt module already stores a capture-less C function pointer. The
same mechanism works for a worker task: `createTask` takes a `func(): void`,
the shim stores the pointer in a task record, and a C entry function calls it.
The worker cannot capture the parent's handles, so the result path uses the
current task's `tc_UserData` slot.

The 1.3 NDK provides `CreateTask` in `clib/alib_protos.h` and `AllocSignal`,
`Signal`, `Wait`, `OpenLibrary`, `OpenDevice`, `DoIO`, `SendIO`, `WaitIO`,
`CheckIO`, and `AbortIO` through `proto/exec.h`. The shim already includes
both headers. The 1.3 SFD has no `CreateMsgPort`; the existing shim uses
`CreatePort`, which is the 1.3-safe constructor.

The A500 gates use a clean ADF, a root `startup-sequence`, a decrypted licensed
KS1.3 ROM, and a host screenshot crop. The hello gate compares a `430x22`
crop at `(160,88)`; the GUI gate writes a native capture and report file. The
new Exec demo can reuse the report-file handshake and the line crop, because
it is a console program with one output line.

## Gaps Identified

### Gap 1: No task lifecycle

There is no `Task` handle, no worker entry, no join signal, and no result path.
The shim cannot start a second thread of execution.

### Gap 2: No library handle

`OpenLibrary` is used internally but not exposed. A program cannot check a
library version or close a library it opened.

### Gap 3: No generic device handle

`audio`, `timer`, and the gameport are typed modules. There is no way to open
`console.device`, `keyboard.device`, `input.device`, or `trackdisk.device`, set
a command and buffer, and run `DoIO`/`SendIO`/`WaitIO`/`CheckIO`/`AbortIO`.

### Gap 4: No ports interrupt server

The interrupt module has one VBlank slot. `INTB_PORTS` is not available.

### Gap 5: No console device path

`console.device` is bound to an Intuition window, so the generic
`openDevice(name, unit, flags)` shape is not enough. The plan adds one typed
`openConsole(window, unit)` helper that borrows a `Window` from
`std.amiga.intuition` and fills the console request's `io_Data`/`io_Length`.

### Gap 6: The 1.3 profile rejects exec

`compiler/src/profile.rs` excludes `std.amiga.exec` from `hosted-amiga-13`,
even though the existing shim calls are 1.3-safe.

### Gap 7: No A500 Exec acceptance

There is no `make sdk-a500-exec`, no `exec-demo.clo`, and no line golden. The
existing exec programs run only on `hosted-amiga` under `vamos`; they are not
proven on the real KS1.3 machine.

## Toolchain Facts

| Fact | Value | Source |
|---|---|---|
| 1.3 `CreateTask` | `struct Task *CreateTask(STRPTR, LONG, APTR, ULONG)` | `clib/alib_protos.h:51` |
| 1.3 signal calls | `AllocSignal`, `FreeSignal`, `Signal`, `Wait`, `FindTask` | `clib/exec_protos.h`, `proto/exec.h` |
| 1.3 device calls | `OpenDevice`, `CloseDevice`, `DoIO`, `SendIO`, `WaitIO`, `CheckIO`, `AbortIO` | `proto/exec.h`, `exec/io.h` |
| 1.3 library calls | `OpenLibrary`, `CloseLibrary`; `lib_Version`/`lib_Revision` | `exec/libraries.h` |
| Request block | `struct IOStdReq` carries message, device, unit, command, flags, error, actual, length, data, offset | `exec/io.h` |
| Request reply port | Every `DoIO`/`SendIO`/`WaitIO` path needs `io_Message.mn_ReplyPort`; the shim must create a private port per device | `exec/io.h`, existing timer/audio shims |
| Console device | `struct ConUnit` has `cu_Window`; `OpenDevice("console.device", ...)` needs the caller's `struct Window *` in `io_Data` and `sizeof(struct Window)` in `io_Length` | `devices/conunit.h` |
| Other-task removal | `RemTask` on a non-current task has a pre-V36 crash risk; bracket it with `Forbid()`/`Permit()` and leave the worker in `Wait(0)` | `exec.library/RemTask` autodoc |
| Ports interrupt | `INTB_PORTS` | `hardware/intbits.h` |
| A500 gate pattern | Decrypt ROM, build ADF, boot FS-UAE, capture line | `docker/scripts/a500-smoke.sh` |
| Reusable report handshake | `DF0:gui.txt` + `DF0:gui-*` files | `docker/scripts/a500-gui-smoke.sh` |
