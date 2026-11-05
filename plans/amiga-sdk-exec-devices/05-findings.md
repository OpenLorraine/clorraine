# Findings: Amiga SDK Bundle 7 — Exec and Devices

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Done — implementation, review, and closeout findings resolved

## Implementation findings

These are target/implementation findings surfaced while executing the plan.
The plan's preflight report records the document-level findings (PF-001–PF-033);
this ledger records what the real 1.3 machine and the emitted C taught us.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-1 | 🟠 Major | The 1.3 `keyboard.device` does not keep a `CMD_READ` pending for the abort proof: the first A500 run returned an immediate event/error (`exec: err 13`), and `CMD_FLUSH` is not a supported keyboard command (`exec: err 11`). | **Fixed** — the abort proof moved to a `console.device` `CMD_READ` on the demo's dedicated window, where no input is pending; the read is aborted and returns `IOERR_ABORTED` (-2). `keyboard.device` and `input.device` stay in the demo as open/close coverage. The final gate reports `key -2`. |
| RV-2 | 🟠 Major | The smoke's `grep -aq "exec: ok task"` matched the **program binary** on the ADF (the format-string literal), so it reported the file before the demo had written it. The ADF listing showed no `exec.txt`. | **Fixed** — the smoke matches the runtime value `exec: ok task 464` (not in the binary), and fails clearly with the ADF listing when `exec.txt` is missing. |
| RV-3 | 🟡 Minor | The hello gate's `430x22` crop cut the 50-character Exec line at the emulator's 1.5x scale; the first golden ended at `key`. | **Fixed** — the smoke crops `620x22` at `(160,88)`; the recaptured golden shows the full `exec: ok task 464 lib 34 dev 512 io 13 key -2 in 1` line. |
| RV-4 | 🟡 Minor | `#include <intuition/window.h>` does not exist in the 1.3 NDK, so the emitted C failed to compile until the include was removed; `intuition/intuition.h` provides the complete `struct Window` used by `sizeof`. | **Fixed** — the shim keeps `intuition/intuition.h` and drops the nonexistent header. |
| RV-5 | 🟡 Minor | The `Library` handle is a contract handle mapped to `struct Library *`, not an opaque `void *`; the first shim signatures conflicted with the emitted prototypes. | **Fixed** — `cl_open_library`/`cl_close_library`/`cl_library_version`/`cl_library_revision` use `struct Library *`, matching `compiler/src/abi.rs`. |
| RV-6 | 🟡 Minor | The `deviceSetBuffer` FFI must take `raw u8` + `u32`; an array parameter crosses as one `APTR` and the length would never reach the shim. | **Fixed** as planned by PF-022; the wrapper passes `raw u8(data.ptr)`/`u32(len(data))` and the shim takes `APTR` + `ULONG`. |
| RV-7 | 🟠 Major | Five pre-Bundle-7 spec tests still asserted that `hosted-amiga-13` rejects `std.amiga.exec` and expected `E11200`: `amiga_a500_spec_test` and the audio/sprite/text/video cross-bundle controls. The first `make check` failed on the now-obsolete contract. | **Fixed** — `amiga_a500_spec_test` now asserts the A500 profile accepts the module and emits C with zero diagnostics; the four redundant cross-bundle controls were removed because `amiga_exec_spec_test` now owns both-profile acceptance and the host rejection. |

## Closeout review findings

Independent correctness review and read-only memory-safety audit after the full
gate. Every critical/major finding is fixed; the minor findings are fixed or
accepted with the recorded rationale.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-C1 | 🟠 Major | `cl_device_check_io` cleared `pending` when `CheckIO` returned the request but never reaped the reply, leaving the message linked in the private port and allowing a reused request to see a stale reply. | **Fixed** — a completed `CheckIO` now calls `WaitIO` before clearing pending and copying the result out; the appendix, design, and impl test pin the reap. |
| RV-C2 | 🟠 Major | The generated-drop tests matched the `take` wrapper's own dead-parameter drop, not the caller's cleanup; a missing caller drop would pass. | **Fixed** — the tests now pin the caller cleanup block, and a `DEVICE_DROP_SOURCE` covers the device drop. |
| RV-C3 | 🟡 Minor | Some device wrapper bodies and failure guards were only name-pinned. | **Fixed** — the exact wrapper bodies (`setBuffer(length)`, `writeBuffer`, `doIO`, `waitIO`, `checkIO`) and the null/unit/buffer guards are pinned. |
| RV-C4 | 🟡 Minor | The demo ignored the `writeBytes` result, so a short `DF0:exec.txt` write reported success. | **Fixed** — the written byte count is checked against the line length; a mismatch prints `exec: err 14`. |
| RV-C5 | 🟡 Minor | The first trackdisk `DoIO` failure was only checked after the async read and mapped to stage 7. | **Fixed** — the first read is checked immediately and maps to stage 5; the async read is not started after a failed first read. |
| RV-C6 | 🟡 Minor | The design's keyboard row still claimed a pending keyboard read and a -2 abort, contradicting RV-1. | **Fixed** — the keyboard/input rows are open/close coverage; the -2 proof is the console read; RV-1 is cited. |
| RV-C7 | 🟡 Minor | Appendix B §13's intro omitted the new `Library` and `Device` handles. | **Fixed** — the intro names all four handle kinds. |
| RV-C8 | 🟡 Minor | The roadmap/index overclaimed 18/18 while the closeout review was open. | **Fixed** — the roadmap and index show the review state until 4.1.5/4.1.6 complete, then 18/18. |
| RV-C9 | 🟡 Minor | The index said `profile.rs` was the only compiler change; the SDK-EXE-006 evidence omitted the A500 gate. | **Fixed** — the index lists the `stdlib.rs` export table too; the registry evidence includes `make sdk-a500-exec`. |
| MS-1 | 🟠 Major | `deviceSetBuffer(borrow data)` stored a caller slice pointer in the request; a function could return while a pending `SendIO` held a dead stack pointer. | **Fixed** — async I/O now uses a device-owned buffer (`deviceSetBuffer(length)` + `deviceWriteBuffer`); `deviceWaitIO`/`deviceCheckIO` copy the result out. Synchronous `deviceDoIO(device, buffer)` borrows the caller buffer only for the blocking call. |
| MS-2 | 🟠 Major | `openConsole(borrow window)` kept a raw window pointer; closing the `Window` first left the console with freed Intuition memory. | **Fixed** — the window is refcounted; `closeWindow` defers the actual close until the last console device closes. |
| MS-3 | 🟡 Minor | A creator at priority below -1 could run the worker before `tc_UserData` was published, hanging the join. | **Fixed** — `createTask` rejects a creator at priority below 0. |
| MS-4 | 🟡 Minor | `openDevice` would accept extended-request devices (`audio`, `timer`, `serial`, `parallel`) with a short `IOStdReq`. | **Fixed** — those names are rejected; they keep their typed modules. |
| MS-5 | 🔵 Observation | The generated drops were only compile-pinned; the A500 demo always joined/closed explicitly. | **Fixed** — the demo leaves a `workerZero` task, the `exec.library` handle, and the `input.device` handle to their generated drops on the real machine. |

## Closeout verification

| Gate | Result |
|---|---|
| `cargo test --lib --test amiga_exec_spec_test --test amiga_exec_impl_test` | 400 lib + 11 impl + 14 spec green |
| `make sdk-a500-exec` | `exec: ok task 464 lib 34 dev 512 io 13 key -2 in 1`; 620x22 golden matches |
| `make check` | 107 test suites green, 0 failures (re-run on the review-fixed tree) |
| `make verify` | All legs passed headlessly on the review-fixed tree: A500 hello, Boing, sprites, paddle, text, assets, GUI, Exec, differential, corpus-fs-uae, and the six runtime faults |
| `make sdk-registry` | OK (79 capabilities, 10 bundles); coverage regenerated |
| GitHub issue #8 | Updated with the checked body, commented, and **CLOSED** |
