# Preflight Report: Amiga SDK Bundle 7 — Exec and Devices

> **Artifact**: `plans/amiga-sdk-exec-devices/` (full implementation plan set)
> **Audit target**: the ten plan documents in that folder
> **Audited**: 2026-11-05 19:16
> **Auditor**: independent read-only general agent (fresh context)
> **Verdict**: ✅ PREFLIGHT PASSED — 33 findings resolved, 0 open

## Audit scope

The audited set is `00-index.md`, `00-ambiguity-register.md`,
`01-requirements.md`, `02-current-state.md`, `03-01-tasks-and-signals.md`,
`03-02-libraries-and-devices.md`, `03-03-interrupts-and-input.md`,
`03-04-demo-and-smoke.md`, `07-testing-strategy.md`, and
`99-execution-plan.md`.

Context read, not audited: `stdlib/std.amiga.exec.clo`,
`stdlib/std.amiga.intuition.clo`, `stdlib/std.amiga.interrupt.clo`,
`compiler/runtime/shim_hosted_amiga.c`, `compiler/src/profile.rs`,
`compiler/src/stdlib.rs`, `compiler/src/abi.rs`,
`compiler/tests/amiga_exec_spec_test.rs`, the existing A500 smoke scripts,
`docs/amiga-sdk/registry.tsv`, and the frozen appendices.

## Checks run

| Check | Result |
|---|---|
| `cd spec && ./check.sh` | `spec check: OK (27 files, 111 compile-fail tests)` |
| `cargo test --test amiga_exec_spec_test` in the SDK container | 6 passed, 0 failed, 0 skipped |
| `bash -n docker/scripts/a500-*.sh` | all existing scripts pass |
| `git diff --check` | clean |
| Read-only codebase/NDK verification | `CreateTask`, `AllocSignal`, `Signal`, `Wait`, `FindTask`, `OpenLibrary`, `CloseLibrary`, `OpenDevice`, `CloseDevice`, `DoIO`, `SendIO`, `WaitIO`, `CheckIO`, `AbortIO`, `RemTask`, `Forbid`, `Permit`, `struct IOStdReq`, `INTB_PORTS`, and `IOERR_ABORTED` all exist in the 1.3 NDK |

## Findings and resolutions

### Iteration 1 blockers

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-001 | 🔴 | The device record had no reply port; `DoIO`/`SendIO`/`WaitIO` cannot reply through a null `mn_ReplyPort`. | Added a `CreatePort(NULL, 0)` reply port, set before `OpenDevice`, deleted on every failure path and on close. |
| PF-002 | 🔴 | `console.device` is bound to an Intuition `Window`; the generic `openDevice` shape cannot open it, and a console prefix would not share the Shell line. | Added `openConsole(borrow window: Window, unit: u8)` and a 320x80x3 screen / 300x64 window in a `writeConsole()` helper; the Shell report line is independent. |
| PF-003 | 🟠 | The worker signaled then `RemTask(NULL)`; the higher-priority parent could free the record or exit before the worker was removed. | The worker now signals then parks in `Wait(0)`; the parent removes it with `Forbid(); RemTask(record->task); Permit();` (the V34-safe autodoc form), then frees the signal and record. |

### Iteration 2 blockers

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-022 | 🔴 | An `ffi` `u8[]` parameter crosses as one `APTR`; the shim would not receive the length and the prototype would conflict. | Changed the `ffi` to `data: raw u8, length: u32`; the public wrapper passes `raw u8(data.ptr)`/`u32(len(data))`, matching `windowString`. |
| PF-023 | 🔴 | The console buffer was a temporary `String` dropped before `DoIO`. | Bound `line` before the command/buffer/do calls and kept it alive through the write. |
| PF-024 | 🟠 | The spec source/ITs were described as one full addition while the plan phased the exports, so Phase 1–3 Verify could not be green. | Phased `EXEC_SOURCE`, the appendix text, and the IT cases in `07` and `99`; each phase's Verify now runs the matching surface. |

### Iteration 1–3 contract and consistency findings

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-004 | 🟠 | Phase 1 added the full surface before Phases 2–3 existed. | Phased the spec source and appendix text. |
| PF-005 | 🟠 | Device setters were allowed while a request was pending. | `deviceCommand`, `deviceSetBuffer`, and `deviceSetOffset` are idle-only and return `false` while pending. |
| PF-006 | 🟡 | The demo table mixed stage numbers and failure codes. | The column is `Failure code`; the table maps each action to its code. |
| PF-007 | 🟡 | `deviceSetBuffer` had no return type in R4. | It returns `bool`; the failure conditions are in R4/R9 and the design. |
| PF-008 | 🟡 | `deviceWaitIO` idle semantics were ambiguous. | Idle returns the last `io_Error`; only a null handle returns -1. |
| PF-009 | 🟡 | `1.1.5` referenced an “IT-7 task part” that does not exist. | It now references IT-3 and IT-11. |
| PF-010 | 🟡 | Phase 3 ran IT-8 before the demo existed. | IT-8 moved to 4.1.1; 3.1.3 runs IT-6. |
| PF-011 | 🟡 | Quick-reference examples missed `std.text` imports. | Imports added. |
| PF-012 | 🟡 | The new cross-build test could fail on host-only CI; the appendix anchor was stale. | Added the standard tool-availability skip and updated the anchor in the same task. |
| PF-013 | 🟡 | The appendix/commit provenance was not recorded. | AR-18/AR-22 cite the user's 2026-11-05 directive and the standing local-only commit rule. |
| PF-014 | 🟡 | The profile and module comments were stale. | Tasks 1.1.2/1.1.3/3.1.2 update the comments. |
| PF-015 | 🟡 | The smoke's Xvfb/crop assumptions were unstated. | The smoke uses the caller's `DISPLAY`/`XAUTHORITY` under the private Xvfb gate and the hello gate's FS-UAE window settings/crop origin. |
| PF-016 | 🟡 | A blank crop could match a blank golden. | The smoke checks `identify -format '%k'` at least 2 before the byte comparison. |
| PF-017 | 🔵 | The device record's allocation flags were unspecified. | `MEMF_PUBLIC | MEMF_CLEAR`. |
| PF-018 | 🔵 | `exec/io.h` and the request casts were missing. | Added the include and `(struct IORequest *)&device->request` casts. |
| PF-019 | 🔵 | The registry closeout did not mention the acceptance column. | Added acceptance/evidence/spec columns to 4.1.3. |
| PF-020 | 🔵 | The task-result key decision cited AR-3 instead of AR-4. | Corrected. |
| PF-021 | 🔵 | The worker restrictions were not in the known-limitations list. | Added `docs/known-limitations.md` to R11/4.1.3 and the design. |
| PF-025 | 🟡 | `identify` was used but not checked. | Added to the tool check. |
| PF-026 | 🟡 | IT-1 omitted `openConsole`. | Added. |
| PF-027 | 🟡 | `deviceCommand`/`deviceSetOffset` could not report a pending-state rejection. | Both return `bool`; pending/null returns `false`. |
| PF-028 | 🔵 | The console screen stayed open until the helper returned, after the report line. | The console shape is in `writeConsole()`; the screen drops at helper return before the report line. |
| PF-029 | 🔵 | The current-state file list omitted the acceptance column. | Added. |
| PF-030 | 🔵 | Timestamps were stale after revision. | Refreshed to 2026-11-05 19:16. |
| PF-031 | 🟡 | Requirements still said “two workers”. | Updated to the nine-cycle stress loop and one worker at a time. |
| PF-032 | 🟡 | IT-1/IT-2/IT-9 were not mapped to their real test files/phases. | IT-1/IT-2 are source unit tests, IT-9 is the spec appendix check; Phase Verify now runs `--lib`. |
| PF-033 | 🔵 | `ConsoleResult` could not distinguish an open failure from a zero-byte write. | Added `code: i32` (8 setup, 9 write) and the caller maps it to `failedAt`. |

## Verdict

**✅ PREFLIGHT PASSED** — every 🔴/🟠 finding is resolved, and every 🟡/🔵
finding is either resolved or explicitly folded into the execution tasks
above. The plan is grounded in the real 1.3 NDK and the existing shim/test
patterns, the scope stays inside the registry's core rows, and the test order
is specification-first.
