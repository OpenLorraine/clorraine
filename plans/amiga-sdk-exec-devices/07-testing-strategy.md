# Testing Strategy: Amiga SDK Bundle 7 — Exec and Devices

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The suite follows the repository's specification-first split:

- **Specification tests** (`amiga_exec_spec_test.rs`) derive from R1–R11 and
  from the existing exec transcripts. They are the oracle: a failure means the
  implementation is wrong.
- **Implementation tests** (`amiga_exec_impl_test.rs`) pin the emitted shim
  calls, the handle/drop routing, the request state machine, the failure
  values, and the demo shape.
- **A500 acceptance** (`make sdk-a500-exec`) proves the runtime behavior on
  the real KS1.3 machine.

The existing `exec-self-test` and `message-drop` transcripts are immutable.
The new surface compiles on both hosted profiles and cross-builds with
`nix13`/`nix20`.

## Specification Test Cases

### Profile and surface

| # | Venue | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|---|
| ST-1 | Host | A program imports `Task`, `Library`, `Device`, every task/library/device wrapper, `CMD_*`, `addPortHandler`, and the existing exec/intuition names; compiles for `hosted-amiga` and `hosted-amiga-13` | Exit 0 and emitted C on both profiles; zero diagnostics | R1–R7, R10 |
| ST-2 | Host | The same program is compiled for `host` | Exit 1 with `E11200`; no C emitted | R10 |
| ST-3 | Host | The full surface is cross-built with `m68k-amigaos-gcc` under `nix13` and `nix20`; the test skips itself when the cross toolchain is absent (the host-only CI has no m68k compiler) | Both builds pass `-std=c99 -m68000 -fwrapv -Wall -Wextra -Werror -lamiga -lm` | R10 |
| ST-4 | Host | `exec-self-test` runs under `vamos` twice | Both runs match the fixed five-line transcript | R2 |
| ST-5 | Host | `message-drop` runs under `vamos` | It matches the fixed transcript; unsent messages are freed, fetched messages reply, explicit frees clean | R2, R9 |
| ST-6 | Host | The stdlib appendix §7 and §13 are read; the §13 anchor is updated with the section title | Every exported interrupt and exec name appears as a whole identifier | R5, R11 |

### A500 runtime

| # | Venue | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|---|
| ST-7 | A500 | The demo runs the worker stress loop: eight `workerSum` workers and one `workerProduct`, created and joined one at a time | `task 464` in the report line; all nine joins return | R1, R7 |
| ST-8 | A500 | The demo opens `exec.library` v33 and reads the version | `lib 34` on KS1.3 | R3, R7 |
| ST-9 | A500 | The demo reads the boot block with `DoIO` and with `SendIO`/`WaitIO` on `trackdisk.device` | Both reads return error 0 and `io_Actual` 512; `dev 512` | R4, R7 |
| ST-10 | A500 | The demo opens a 320x80x3 screen and a 300x64 window, opens `console.device` on the window, writes the 13-byte line `exec console\n` with `DoIO`, sends a `CMD_READ`, aborts it, and closes the console/window/screen | Write error 0 and `io_Actual` 13; the read returns `IOERR_ABORTED` (-2); `io 13`, `key -2`; the Shell report line is unaffected | R4, R6, R7 |
| ST-11 | A500 | The demo opens/closes `input.device` and `keyboard.device` | `in 1`; both closes are clean; no hang | R4, R6, R7 |
| ST-12 | A500 | The demo registers a VBlank handler, waits three ticks, and removes it; it also registers/removes a ports handler | The VBlank counter advances; both removals are clean; the demo reaches the library stage | R5, R7 |
| ST-13 | A500 | The smoke reads `DF0:exec.txt` | The file matches `exec: ok task 464 lib 34 dev 512 io 13 key -2 in 1` | R7, R8 |
| ST-14 | A500 | The smoke captures the report line | The `620x22+160+88` crop matches `docs/amiga-sdk/a500-exec.png` byte-exact and is not blank | R8 |

## Implementation Test Cases

| # | Target | Assertion |
|---|---|---|
| IT-1 | `compiler/src/stdlib.rs` unit test | The exec export table lists every new name (`Task`, `Library`, `Device`, `createTask`, `reportTaskResult`, `joinTask`, `openLibrary`, `closeLibrary`, `libraryVersion`, `libraryRevision`, `openDevice`, `openConsole`, `closeDevice`, `deviceCommand`, `deviceSetBuffer`, `deviceSetOffset`, `deviceDoIO`, `deviceSendIO`, `deviceWaitIO`, `deviceCheckIO`, `deviceAbortIO`, `deviceActual`, `deviceError`, and the `CMD_*` constants); the interrupt table lists `addPortHandler` |
| IT-2 | `compiler/src/profile.rs` unit test | `hosted-amiga-13` supports `std.amiga.exec`; `host` does not; the shared shim is unchanged |
| IT-3 | Emitted C | The task wrappers call `cl_create_task`, `cl_task_result`, `cl_join_task`, `cl_task_free`; the generated drop routes an unjoined task to `cl_task_free` |
| IT-4 | Emitted C | The library wrappers call `cl_open_library`, `cl_close_library`, `cl_library_version`, `cl_library_revision`; the drop routes to `cl_close_library` |
| IT-5 | Emitted C | The device wrappers call every `cl_device_*` shim and `cl_open_console`; `CMD_*` lower to the expected values; the drop routes to `cl_close_device` |
| IT-6 | Shim | The interrupt shim has a VBlank and a ports server, one shared token counter, `INTB_PORTS`, and a `remove` that checks both slots |
| IT-7 | Shim | The device state machine rejects `DoIO`/`SendIO` while pending, treats the setters as idle-only, clears pending on `WaitIO`/`CheckIO`, aborts and waits on close, and returns the documented failure values |
| IT-8 | Demo shape | `examples/amiga/exec-demo.clo` runs the worker stress loop, uses `exec.library`, `trackdisk.device`, a screen/window plus `openConsole` with a write and an aborted read, `input.device`, and `keyboard.device`, and writes the pinned report line to `DF0:exec.txt` |
| IT-9 | `amiga_exec_spec_test.rs` appendix check | §13 mentions every exec export as a whole identifier; §7 mentions `addPortHandler`; the test's section anchor matches the new §13 title |
| IT-10 | Test integrity | The existing `amiga_exec_spec_test` host-rejection and transcript cases still run; no golden is weakened |
| IT-11 | Shim | The task record has the magic guard, the worker signals then `Wait(0)`, and join/drop remove it with `Forbid`/`RemTask`/`Permit` before freeing the signal and record; the demo's nine create/join cycles exercise the path |
| IT-12 | Shim | The device record creates a private reply port before `OpenDevice`, sets it in `io_Message.mn_ReplyPort`, deletes it on every failure path and on close, and the console helper fills `io_Data`/`io_Length` from the window |

## Test Files

| File | Change |
|---|---|
| `compiler/tests/amiga_exec_spec_test.rs` | Extend `EXEC_SOURCE` in phase order (task, then library/device, then `addPortHandler`); add ST-1/ST-3; keep ST-2/ST-4/ST-5; extend ST-6 in the same phase as each export |
| `compiler/tests/amiga_exec_impl_test.rs` | New; IT-3–IT-8, IT-10–IT-12 |
| `compiler/src/stdlib.rs` | Export table and IT-1 unit test |
| `compiler/src/profile.rs` | 1.3 module list and IT-2 unit test |
| `stdlib/std.amiga.exec.clo` | New handles/wrappers |
| `stdlib/std.amiga.interrupt.clo` | `addPortHandler` |
| `compiler/runtime/shim_hosted_amiga.c` | New C shim |
| `examples/amiga/exec-demo.clo` | New demo |
| `docker/scripts/a500-exec-smoke.sh` | New smoke |

## Red/Green Order

1. Add the task part of `EXEC_SOURCE`, ST-1/ST-3 task cases, and the §13
   task text; run the spec suite red (unresolved exports / missing profile
   support).
2. Add the task module, shim, export table, and profile gate; run ST-1–ST-6
   green and keep the existing transcripts green.
3. Add the library/device part of `EXEC_SOURCE`, ST-1/ST-3 library/device
   cases, and the §13 library/device text; add the shim and export entries;
   run green.
4. Add `addPortHandler`, the ports server, and the §7 text; run green.
5. Add the phase's IT cases: the source unit tests (IT-1/IT-2) and the
   integration cases (IT-3–IT-8, IT-10–IT-12); IT-9 is the appendix check.
   Fix the emitted shapes and shim branches until green.
6. Build and run the demo on `hosted-amiga-13`; fix target-only defects.
7. Run `make sdk-a500-exec`, capture the report line and the line golden.
8. Run `make check`, `make verify`, and `make sdk-registry`.
9. Run the independent phase review; fix findings; commit.

## Verification Commands

```text
cargo test --lib --test amiga_exec_spec_test --test amiga_exec_impl_test
make check
make sdk-a500-exec
make verify
make sdk-registry
./spec/check.sh
bash -n docker/scripts/a500-exec-smoke.sh
```

## Evidence to Keep

| Artifact | Path |
|---|---|
| Spec transcript | `examples/amiga/exec-self-test.expected.txt`, `message-drop.expected.txt` |
| A500 report line | `DF0:exec.txt` in the ADF; asserted as the pinned string |
| A500 line crop | `docs/amiga-sdk/a500-exec.png` |
| Last failure screen | `build/amiga-sdk/exec-smoke/screen.png` (gitignored) |
| Emitted shapes | `amiga_exec_impl_test` assertions |
| Registry/coverage | `docs/amiga-sdk/registry.tsv`, `coverage.md` |
| Issue | GitHub #8 body, comment, and closed state |
