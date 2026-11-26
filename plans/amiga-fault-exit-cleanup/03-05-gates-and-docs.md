# Gates and docs: Amiga fault-exit cleanup

> **Document**: 03-05-gates-and-docs.md
> **Parent**: [Index](00-index.md)

## Overview

The acceptance evidence comes from two legs (AR-7, AR-17):
- `vamos` (exec and DOS kinds, leak detection) on both hosted profiles;
- one FS-UAE gate on A500/KS1.3 (interrupt servers, display, window, library bases, workers,
  Ctrl-C).

This document also lists the doc updates and the bench evidence (AR-18, AR-19).

## Probe programs

Location: `examples/robustness/fault-cleanup/` (AR-20). Expected behaviour is owned by the
ST-cases in [07](07-testing-strategy.md); this list only names the programs.

| Program | Leg | Acquires, then | ST |
|---|---|---|---|
| `lock-fault.clo` | vamos | `lockPath`, `pathInfo`; index fault | ST-1 |
| `port-fault.clo` | vamos | `createMsgPort`, `createPublicPort`; division fault | ST-2 |
| `message-fault.clo` | vamos | two own ports, `createMessage`, `putMsg` to own port; index fault | ST-3 |
| `reply-fault.clo` | vamos | two own ports, `createMessage`, `putMsg`, `getMsg` on the destination, `replyMsg`; index fault before the reply is fetched | ST-3b |
| `library-fault.clo` | vamos | `openLibrary("dos.library", 33)`; index fault | ST-4 |
| `panic-after-acquire.clo` | vamos | `lockPath`; `panic("stop")` | ST-5 |
| `assert-after-acquire.clo` | vamos | `lockPath`; failing `assert` | ST-6 |
| `utility-fault.clo` | vamos, `hosted-amiga` only | `newTagList`, `currentDateStamp`, `createSemaphore`; index fault | ST-19 |
| `handler-fault.clo` | A500 | `addHandler`, `createPublicPort("ClFaultCleanup")`; index fault | ST-10 |
| `display-fault.clo` | A500 | two `newBitmap`, `openDisplay`, `displayFlip`; index fault | ST-11 |
| `window-fault.clo` | A500 | `openWorkbenchWindow`, `openConsole` on it (left open); index fault | ST-12 |
| `font-fault.clo` | A500 | `openDiskFont` (the font the diskfont gate uses); index fault | ST-13 |
| `worker-fault.clo` | A500 | worker divides by zero; parent joins | ST-14 |
| `break-timer.clo` | A500 | `addHandler`, `openTimer`; prints `waiting`; `timerWait` of 3000 ticks, then prints `done` | ST-15 |
| `break-own-mask.clo` | A500 | prints `waiting`; `waitSignals(0x1000)`; prints `break bit` | ST-16 |
| `clean-exit.clo` | A500 | `addHandler` without `removeHandler`, `openDiskFont`; returns 0 | ST-17 |
| `worker-fault-unjoined.clo` | A500 | `openTimer`; worker divides by zero; the parent calls `timerWait(timer, 50)`, then faults before joining | ST-18 |

## `vamos` leg

- **Spec test.** `compiler/tests/amiga_fault_cleanup_spec_test.rs` builds each vamos-leg probe
  for `hosted-amiga` (`-mcrt=nix20`) and `hosted-amiga-13` (`-mcrt=nix13`) with the debug flag
  set. It runs each under `vamos` and asserts the ST expectation. It fails when the `vamos` output
  matches the leak pattern `can't expunge|mem_alloc:WARNING:\s+orphan|shutdown: can't expunge`
  (the pattern the assessment showed fires on a seeded leak). The test skips itself without the
  SDK, like `amiga_exec_spec_test.rs` (AR-7, AR-19, AR-20).
- **`run-fault-amiga.sh`** builds on both profiles and applies the same pattern to every case,
  including the existing 8. It adds the vamos-leg probes as cases. The header and `Makefile:16` /
  `AGENTS.md` counts are corrected to the real number of cases (AR-7, AR-19).

## A500 gate: `sdk-a500-fault-cleanup`

- **Script.** `docker/scripts/a500-fault-cleanup-smoke.sh` follows `a500-diskfont-smoke.sh`:
  - it decrypts the Kickstart 1.3 ROM under `build/` and removes it on exit;
  - it copies the licensed Workbench 1.3.5 HDF from `${AMIGA_DATA}/hdf` per run;
  - it uses a private Xvfb display;
  - it names no ROM file or home path in committed text (AR-17).
- **Build.** All A500 probes and the walker are built for `hosted-amiga-13` (`-mcrt=nix13`).
- **Walker.** `docker/scripts/fault-cleanup-walker.c` (test-only C, AR-20) prints one line. It
  takes the system state under `Forbid` and prints it after `Permit`:
  `<tag> vertb <servers on the VERTB chain> port <1 if "ClFaultCleanup" exists else 0> workers <tasks named "clorraine.worker" on TaskReady+TaskWait> view <sys when GfxBase->ActiView is &IntuitionBase->ViewLord, else other> windows <windows on the Workbench screen> diskfont <diskfont.library open count, 0 when not loaded>`.
- **Capture disk.** FS-UAE holds the HDF locked while it runs, so the HDF can be read only
  after the run (`a500-diskfont-smoke.sh:131-137`). The gate therefore also mounts a blank
  capture ADF, created per run with `xdftool`, in DF1 (`--floppy-drive-1`). DF0 stays empty, so
  Kickstart 1.3 still boots the HDF; a disk in DF0 would come first in the boot order. Every
  walker line, every probe's output, the `***Break` line, and the `rc fail` echo are redirected
  to files on `DF1:`. The host greps the raw ADF while FS-UAE runs (the
  `a500-gui-smoke.sh:185-197` pattern) and reads the files with `xdftool` after the run.
- **Startup-sequence** (`failat 100`, on the HDF). For each probe:
  1. walker `before`, written to `DF1:`;
  2. the probe, with its output redirected to `DF1:`;
  3. `IF FAIL` → `echo "rc fail"`, written to `DF1:`;
  4. walker `after`, written to `DF1:`.

  Before each of the two break probes, the startup-sequence writes one line to the console
  without redirection. That makes the boot CLI process, which runs the probe on 1.3, the
  console's last user. The gate polls the capture ADF for the probe's `waiting` line, focuses
  the FS-UAE window, and sends `ctrl+c` with `xdotool` (the `a500-gui-smoke.sh` focus-and-key
  pattern) once. It does not retry: a stray second break could reach the next command. It then
  waits, with a bound, for `***Break` (ST-15) or `break bit` (ST-16) to appear in the capture.
  Break delivery to a probe with redirected output on Kickstart 1.3 is an unproven assumption.
  The Phase 5 red phase (task 5.1.2, through ST-16's `break bit`) must show it before any
  implementation; if it fails, stop and ask the user (AR-17).
  The gate fails on a timeout, a missing line, or any `before`/`after` mismatch.
- **Make wiring.** A `Makefile` target `sdk-a500-fault-cleanup` is added, listed in `.PHONY` and
  in the `verify` recipe after `sdk-a500-exec`. The `AGENTS.md` command table and the verify row
  are updated (AR-19).

## Docs

Updated in the closing phase (AR-19):

| File | Content |
|---|---|
| `stdlib/std.amiga.exec.clo` | Worker faults are reported by the parent at join (or at exit). A pending Ctrl-C ends `joinTask`, `deviceDoIO`, and `deviceWaitIO`. `waitSignals` returns the break bit only when the mask includes it. |
| `stdlib/std.amiga.interrupt.clo` | Handlers are removed at every exit. A fault inside a handler is not supported (AR-4). |
| `stdlib/std.amiga.dos.clo`, `std.amiga.timer.clo` | Locks are released at every exit. The timer wait is breakable. |
| `compiler/runtime/shim_hosted_amiga.c` | The registry comment block carries the contract for later records ([03-01 §Contract for later plans](03-01-exit-registry.md#contract-for-later-plans)). |
| `docs/known-limitations.md` | Unbreakable waits (audio, services devices, `Delay`); unrestored DMACON and user `std.amiga.custom` writes; faults in interrupt context; in-flight messages held by foreign ports stay allocated. |
| `AGENTS.md`, `site/project/verification.md` | The new gate; the corrected run-fault count. |

No `spec/` file changes (AR-6).

## Bench evidence

The closing phase runs these and records the results in the execution plan (AR-18):
- `make bench-base COMMIT=<plan base commit>`;
- `make bench`;
- `make bench-compare` with an empty `REQUIRE`;
- `make bench-flags`.

## Follow-up issue

The plan does not implement faults in interrupt context. The user approved a separate GitHub
issue for it, filed as #95 (AR-4).
