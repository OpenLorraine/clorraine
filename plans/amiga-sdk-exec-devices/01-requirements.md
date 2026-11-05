# Requirements: Amiga SDK Bundle 7 — Exec and Devices

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1–R11

## Feature Overview

Bundle 7 completes the core Exec and device surface that the A500/KS1.3
baseline needs. The existing `std.amiga.exec` module already owns private
message ports, messages, and signals. This bundle adds worker tasks with join,
library open/close/version, generic device I/O requests, a ports interrupt
server, and the input, keyboard, and console device paths. A demo with a
nine-cycle worker stress loop and real asynchronous I/O proves the surface on
the A500.

The bundle does not change the language, grammar, diagnostics, or ABI. It adds
no 2.0+ library call to the 1.3 build.

## Functional Requirements

### Must Have

- **R1 — Worker tasks and join.** `createTask(entry: func(): void,
  stackBytes: u32): Task?` starts a capture-less worker task and returns an
  owned `Task?`, or `null` on allocation failure. `stackBytes` is 4096..65536.
  The caller must run at priority 0 or above; a lower-priority creator gets
  `null` because the worker starts at -1 and the record must be published
  before the worker runs. `reportTaskResult(value: i32)` stores the worker's result in
  the current task's record. `joinTask(take task: Task): i32` waits for the
  worker to finish, removes the worker with `Forbid(); RemTask(...); Permit();`
  (the V34-safe form), frees the join signal, and returns the result. The
  worker signals, then waits in the shim's final `Wait(0)` until the parent
  removes it. The generated drop joins an unjoined task. The worker entry must
  use only Exec-safe, non-blocking code: arithmetic, module-level scalars, and
  `reportTaskResult`. It must not call DOS, Intuition, `std.io`, allocate heap
  memory, drop an owning value, or block; the only blocking call in the worker
  path is the shim's final wait.
- **R2 — Signals and message ports.** The existing private-port, message, and
  signal surface stays unchanged: `MsgPort`, `NewMessage`, `Message`,
  `createMsgPort`, `deleteMsgPort`, `createMessage`, `messagePayload`,
  `putMsg`, `signals`, `waitSignals`, `getMsg`, `replyMsg`, `freeMessage`, and
  `releaseMsg`. `std.amiga.exec` becomes available on `hosted-amiga-13`; the
  `exec-self-test` and `message-drop` transcripts stay fixed. No public named
  ports.
- **R3 — Library open/close and version.** `openLibrary(name: cstring,
  version: u16): Library?` opens a library at or above `version` (0 opens any
  version). `closeLibrary(take library: Library): void` closes it.
  `libraryVersion(borrow library: Library): u16` and `libraryRevision(borrow
  library: Library): u16` read the base fields. A null handle returns 0; a null
  or missing library returns `null`.
- **R4 — Generic device I/O.** `openDevice(name: cstring, unit: i32, flags:
  u32): Device?` opens a standard `IOStdReq` device with one owned request,
  one private reply port, and an optional device-owned asynchronous buffer, or
  returns `null`. Extended-request devices (`audio.device`, `timer.device`,
  `serial.device`, `parallel.device`) are rejected; they keep their typed
  modules. `closeDevice(take device: Device): void` aborts a pending request,
  waits for it, closes the device, frees the buffer, deletes the reply port,
  frees the request, and releases the console window. `openConsole(borrow
  window: Window, unit: u8): Device?` is the typed console path: it opens
  `console.device` with the caller's Intuition window in `io_Data` and
  `sizeof(struct Window)` in `io_Length`, because the console device is bound
  to a window. The window is refcounted, so closing the `Window` handle while
  the console is open defers the actual close until the device closes. The
  request surface is:
  - `deviceCommand(borrow device, command: u16): bool` sets `io_Command`,
    returns `true`; idle only, `false` for a null handle or a pending request.
  - `deviceSetBuffer(borrow device, length: u32): bool` reserves a
    device-owned asynchronous buffer; length 0 releases it. `false` for a null
    handle, a pending request, or allocation failure.
  - `deviceWriteBuffer(borrow device, borrow data: u8[]): bool` copies a write
    payload into the device-owned buffer; `false` without a buffer, while
    pending, or when `data` is longer than the buffer.
  - `deviceSetOffset(borrow device, offset: u32): bool` sets `io_Offset`,
    returns `true`; idle only, `false` for a null handle or a pending request.
  - `deviceDoIO(borrow device, borrow data: u8[]): i32` borrows the caller's
    buffer for the call, runs a synchronous request, and returns `io_Error`; a
    pending request returns -1. A synchronous read writes into the caller's
    buffer during the call.
  - `deviceSendIO(borrow device): bool` starts an asynchronous request over the
    device-owned buffer; a pending request returns `false`.
  - `deviceWaitIO(borrow device, borrow data: u8[]): i32` waits for a pending
    request, clears the pending flag, copies the result into `data`, and
    returns `io_Error`; on an idle handle it returns the last `io_Error`
    without copying, and a null handle returns -1.
  - `deviceCheckIO(borrow device, borrow data: u8[]): bool` reports whether a
    pending request has completed; a completed request is reaped with
    `WaitIO`, its result is copied into `data`, and the flag is cleared.
  - `deviceAbortIO(borrow device): void` requests an abort; the caller waits
    with `deviceWaitIO`.
  - `deviceActual(borrow device): u32` reads `io_Actual`.
  - `deviceError(borrow device): i32` reads `io_Error`.
  - The `CMD_INVALID`, `CMD_RESET`, `CMD_READ`, `CMD_WRITE`, `CMD_UPDATE`,
    `CMD_CLEAR`, `CMD_STOP`, `CMD_START`, and `CMD_FLUSH` constants are
    exported.
  The console helper imports `Window` from `std.amiga.intuition`; it does not
  add a console module.
- **R5 — Interrupt servers.** `std.amiga.interrupt` keeps
  `addHandler(hook): HandlerToken` for the vertical-blank server and adds
  `addPortHandler(hook): HandlerToken` for the ports interrupt (`INTB_PORTS`).
  Each kind has one slot, so a new registration replaces the previous one of
  the same kind. `removeHandler` removes whichever slot owns the token and is
  idempotent. Handlers run in interrupt context and stay allocation-free,
  drop-free, and panic-free.
- **R6 — Input, keyboard, and console devices.** The generic device surface
  opens and operates `input.device` and `keyboard.device`; the input handle is
  closed by its generated drop and the keyboard handle is closed explicitly.
  `console.device` is opened through `openConsole(window, unit)` on a
  caller-supplied Intuition window, then written with `CMD_WRITE` and `DoIO`.
  The demo also sends a `CMD_READ` on that window using the device-owned
  asynchronous buffer, aborts it, and waits; the aborted read returns
  `IOERR_ABORTED` (-2) because the dedicated window has no pending input. The
  console window is refcounted, so the `Window` handle may close before the
  console device. No separate console or keyboard module is added. (The 1.3
  keyboard device does not keep a read pending reliably; see RV-1.)
- **R7 — Worker and async-I/O demo.** `examples/amiga/exec-demo.clo`:
  1. runs a worker stress loop: eight `workerSum` workers (55 each) and one
     `workerProduct` worker (24), created and joined one at a time, for
     `8 * 55 + 24 = 464`; a tenth `workerZero` task is left to the generated
     `Task` drop;
  2. opens `exec.library` version 33, reports its version, and lets the
     generated `Library` drop close it;
  3. reads the boot block with `trackdisk.device` `CMD_READ` using
     `deviceDoIO` over the caller's buffer, and fails stage 5 before starting
     the async read if the first read is short or fails;
  4. reserves a device-owned buffer, reads the block again with `SendIO`, and
     copies the result out with `WaitIO`;
  5. opens a 320x80x3 Intuition screen and a 300x64 window, writes the 13-byte
     line `exec console\n` to `console.device` on that window with `CMD_WRITE`
     and `DoIO`, sends a `CMD_READ` on the same window using the device-owned
     buffer, aborts it, expects `IOERR_ABORTED` (-2), then closes the console
     and window and lets the screen drop;
  6. opens `input.device`, reads its last error, and lets the generated
     `Device` drop close it;
  7. opens and closes `keyboard.device`;
  8. prints the line, writes `DF0:exec.txt`, and checks the written byte count
     against the line length.
  The demo exits 0 on every success and prints `exec: err <code>` on failure;
  a failed console read adds `key <error>` to the failure line for diagnosis.
- **R8 — A500 acceptance.** `make sdk-a500-exec` builds the demo in release
  mode for `hosted-amiga-13`, boots a clean ADF on the real KS1.3 ROM under
  FS-UAE as an A500, polls `DF0:exec.txt` for the report line, and compares a
  `620x22` line crop at `(160,88)` with `docs/amiga-sdk/a500-exec.png` (the
  longer Exec line needs the wider crop at the emulator's 1.5x scale). The
  smoke polls from 10 s every 2 s, re-finds the FS-UAE window, and uses a
  60-iteration bound. It is keyboard- and pointer-free.

### Should Have

- **R9 — Errors, limits, and ownership.** Every public call is null-safe.
  `openLibrary`, `openDevice`, `openConsole`, and `createTask` return `null` on
  failure. `libraryVersion`/`libraryRevision` return 0 on null. `deviceDoIO`
  returns -1 on a null handle, a null buffer with a non-zero length, or a
  pending request; `deviceWaitIO` returns -1 only on a null handle and
  otherwise returns the request's `io_Error` (idle included, no copy);
  `deviceCommand`, `deviceSetBuffer`, `deviceWriteBuffer`, `deviceSetOffset`,
  `deviceSendIO`, and `deviceCheckIO` return `false`; `deviceActual` returns
  0; `deviceError` returns -1 on null. `unit` must be 0..255 and
  extended-request devices are rejected. A second send or a synchronous `DoIO`
  while a request is pending is rejected; the command, buffer, and offset
  setters are idle-only. A completed `CheckIO` reaps its reply with `WaitIO`
  before the request can be reused.
  Every partial allocation (task record, signal bit, reply port, device
  request, device buffer, library handle) is released before a failure return.
  The generated drops are exactly-once. The console window is refcounted so
  the console cannot outlive the window.
- **R10 — Compatibility and gating.** The new surface compiles on both
  `hosted-amiga-13` and `hosted-amiga`; the host and freestanding profiles
  still reject `std.amiga.exec` with `E11200`. The 1.3 build calls only the
  Kickstart 1.3 vectors listed in AR-16. No existing public API behavior
  changes; the existing exec transcripts and the earlier A500 gates stay
  green. No language, grammar, diagnostic, or ABI change.
- **R11 — Documentation and tracking.** Update `spec/appendix-amiga.md`,
  `spec/appendix-stdlib.md` §7 and §13, `docs/known-limitations.md` (the
  worker restrictions), `AGENTS.md`, the registry/coverage/bundle tables
  (including the acceptance/evidence/spec columns), issue #8,
  `plans/amiga-sdk/00-index.md`, `plans/00-roadmap.md`, and
  `codeops/specialist-check.json`. Registry Bundle 7 ends at 8/11 done with the
  three advanced rows deferred.

### Won't Have (Out of Scope)

- Public (named) message ports or `FindPort`.
- DOS processes (`CreateNewProc`), task priorities, task cancellation, or
  task-local storage.
- Signal semaphores (SDK-EXE-011; 2.0+).
- Hooks (SDK-EXE-010; 2.0+) and the utility library (SDK-EXE-007; 2.0+).
- Typed `console.device`, `keyboard.device`, or `input.device` modules; the
  generic device request is the surface, plus the one `openConsole` helper
  because the console device needs a window.
- Device-specific request structs; `std.amiga.timer` and `std.amiga.audio`
  keep their existing wrappers.
- Language-level async/await, threads, or a runtime scheduler.
- Any language, grammar, diagnostic, or ABI change.

## Technical Requirements

### Performance

- A worker task uses the caller's stack request (4096..65536 bytes) and one
  signal bit. The demo uses 8192 bytes and creates/joins one worker at a time,
  nine times.
- The generic device handle adds one `IOStdReq` and one reply port per open
  device. The demo opens at most one device at a time.
- The A500 release build must fit the 512 KB chip-RAM machine with the demo's
  one worker at a time and one 512-byte read buffer.

### Compatibility

- Only the Kickstart 1.3 vector set in AR-16 is used. `CreateTask`,
  `AllocSignal`, `Signal`, `Wait`, `OpenLibrary`, `OpenDevice`, `DoIO`,
  `SendIO`, `WaitIO`, `CheckIO`, and `AbortIO` are 1.3-safe.
- The shim stays valid C99 and builds with both `-mcrt=nix13` and
  `-mcrt=nix20` under `-Wall -Wextra -Werror`.
- The host profile keeps rejecting the module; the 1.3 profile now provides it.

### Security

- Device names, library names, and task entry points come from the program,
  not from untrusted runtime input. The wrappers still validate null names,
  `unit`, stack size, and pending state, and never index with an unchecked
  value.
- A worker task must not touch the parent's handles or call non-Exec services;
  the restriction is documented because a plain 1.3 task has no DOS process
  context.

## Acceptance Criteria

1. `compiler/tests/amiga_exec_spec_test.rs` passes: the full surface compiles
   on both hosted profiles, the host profile rejects it with `E11200`, and the
   full surface cross-builds with `nix13` and `nix20` under `-Werror`.
2. `compiler/tests/amiga_exec_impl_test.rs` passes: emitted shim calls,
   ownership/drop routing, failure values, pending-state rules, interrupt
   tokens, and the demo shape are pinned.
3. `make sdk-a500-exec` passes on the real A500/KS1.3: `DF0:exec.txt` matches
   `exec: ok task 464 lib 34 dev 512 io 13 key -2 in 1` and the non-blank line
   crop matches `docs/amiga-sdk/a500-exec.png`.
4. `make check`, `make verify`, and `make sdk-registry` pass; the earlier A500
   gates stay green.
5. Registry Bundle 7 is 8/11 done with SDK-EXE-007/010/011 deferred; issue #8
   is closed; the programme index and roadmap show Bundle 7 done and Bundle 8
   next.
6. No language, grammar, diagnostic, or ABI change.
