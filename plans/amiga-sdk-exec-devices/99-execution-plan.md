# Execution Plan: Amiga SDK Bundle 7 — Exec and Devices

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-05 20:33
> **Progress**: 17/18 tasks (94%) — Executing
> **CodeOps Artifact Schema**: 1

## Overview

Four phases: enable and extend the Exec task/signal core, add library and
generic device I/O, add the ports interrupt server and device coverage, then
build the worker/async-I/O demo, prove it on the real A500/KS1.3, and close the
bundle. Specification tests come before implementation in every phase.

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Tasks, signals, and 1.3 enablement | 5 |
| 2 | Libraries and generic devices | 4 |
| 3 | Interrupts and input devices | 3 |
| 4 | Demo, A500 acceptance, and closeout | 6 |

**Total: 18 tasks across 4 phases.**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of
> truth** for progress. Every task line appears exactly once in this document.
> The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the
>    Last Updated stamp after EVERY task — never batch updates. Only `[x]` counts
>    as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]`
>    task is resumed first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on
>    the same line. The plan lifecycle is `Ready`, `Executing`, `Done`, or
>    `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep
> the marks current means progress is invisible after crashes, context resets, or
> session handoffs.

---

## Phase 1: Tasks, signals, and 1.3 enablement

> **Reasoning**: high — a new worker-task lifecycle and a public module
> enabled on the 1.3 baseline.

### Step 1.1: Task core

**Reference**: [03-01](03-01-tasks-and-signals.md) · AR-2–AR-6, AR-15–AR-17

- [x] 1.1.1 [spec-author] Extend `compiler/tests/amiga_exec_spec_test.rs` with the task portion of the full surface (ST-1, ST-3) and the export/appendix check (ST-6); keep the host rejection and transcripts (ST-2, ST-4, ST-5); run it red ✅ (completed: 2026-11-05 19:19; red evidence: `E11102` for `Task`/`createTask`/`reportTaskResult`/`joinTask` and `E11200` on `hosted-amiga-13`)
- [x] 1.1.2 Add `std.amiga.exec` to the `hosted-amiga-13` module list in `compiler/src/profile.rs`; update the stale profile comment and the profile unit tests; update the module doc comment; verify the existing module now compiles for 1.3 ✅ (completed: 2026-11-05 19:20; the profile unit test now asserts 1.3 support; the module doc says classic 1.3 Exec)
- [x] 1.1.3 Add `Task`, `createTask`, `reportTaskResult`, `joinTask`, and the task record/entry/signal/`Forbid`+`RemTask` join shim to `stdlib/std.amiga.exec.clo` and `compiler/runtime/shim_hosted_amiga.c`; update the export table in `compiler/src/stdlib.rs` and the §13 task text in `spec/appendix-stdlib.md` ✅ (completed: 2026-11-05 19:21; the shim has the magic/parent/signal/result record, the `Wait(0)` worker, and the V34-safe removal; the appendix §13.1 documents it)
- [x] 1.1.4 Verify the task spec cases and the existing `exec-self-test`/`message-drop` transcripts; run the task drop routing with a small probe ✅ (completed: 2026-11-05 19:22; 8/8 spec tests green including both transcripts; the emitted drop is `if (task) { cl_task_free(task); }`)
- [x] 1.1.5 Add the task implementation cases (IT-3, IT-11, IT-10) to `compiler/tests/amiga_exec_impl_test.rs`; run them green ✅ (completed: 2026-11-05 19:22; 5/5 impl tests green; the wrapper bodies, record contract, limits, and no-2.0+ checks are pinned)

**Verify**: `cargo test --lib --test amiga_exec_spec_test --test amiga_exec_impl_test`

---

## Phase 2: Libraries and generic devices

> **Reasoning**: high — a new library handle and a generic device request
> state machine on 1.3.

### Step 2.1: Library and device surface

**Reference**: [03-02](03-02-libraries-and-devices.md) · AR-7–AR-10, AR-15

- [x] 2.1.1 [spec-author] Add the library and device surface to the spec source (ST-1/ST-3) and run it red; add the `openConsole` path and the `Window` import ✅ (completed: 2026-11-05 19:25; red evidence: `E11102` for `Library`, `Device`, the `CMD_*` constants, and every wrapper)
- [x] 2.1.2 Add `Library`, `openLibrary`, `closeLibrary`, `libraryVersion`, and `libraryRevision` to the module and shim; update the export table ✅ (completed: 2026-11-05 19:26; the `Library` contract maps to `struct Library *`; the shim matches)
- [x] 2.1.3 Add `Device`, the `CMD_*` constants, the open/openConsole/close/command/buffer/offset/do/send/wait/check/abort/actual/error wrappers, the `IOStdReq` reply port, and the console-window shim; use the `raw u8`+`u32` FFI for `deviceSetBuffer`; add `#include <exec/io.h>` and the `struct IORequest *` casts; update the export table and the §13 library/device text ✅ (completed: 2026-11-05 19:26; the reply port, pending state machine, console window fields, and appendix §13.3/§13.4 are in)
- [x] 2.1.4 Add the library/device implementation cases (IT-4, IT-5, IT-7, IT-12) to `amiga_exec_impl_test.rs`; run them green ✅ (completed: 2026-11-05 19:27; 9/9 impl tests green; wrapper bodies, reply port, state machine, and console helper pinned)

**Verify**: `cargo test --lib --test amiga_exec_spec_test --test amiga_exec_impl_test`

---

## Phase 3: Interrupts and input devices

> **Reasoning**: medium — a second interrupt slot and the generic device
> coverage for input, keyboard, and console.

### Step 3.1: Port interrupt and device coverage

**Reference**: [03-03](03-03-interrupts-and-input.md) · AR-11, AR-12, AR-16

- [x] 3.1.1 [spec-author] Add `addPortHandler` to the spec source and the appendix check (ST-1/ST-6) and run it red ✅ (completed: 2026-11-05 19:28; red evidence: `E11102` for `addPortHandler`)
- [x] 3.1.2 Add the VBlank/ports server pair, shared token counter, and `INTB_PORTS` removal to `stdlib/std.amiga.interrupt.clo` and the shim; update the export table, the module doc comment, and the §7 text in `spec/appendix-stdlib.md` ✅ (completed: 2026-11-05 19:28; the shim has two one-slot servers and one shared token counter; `removeHandler` checks both)
- [x] 3.1.3 Add the interrupt implementation cases (IT-6) to `amiga_exec_impl_test.rs`; run them green ✅ (completed: 2026-11-05 19:29; 10/10 impl tests green; the wrapper body and both server slots are pinned)

**Verify**: `cargo test --lib --test amiga_exec_spec_test --test amiga_exec_impl_test`

---

## Phase 4: Demo, A500 acceptance, and closeout

> **Reasoning**: high — real KS1.3 task/device behavior, a new A500 gate, and
> the bundle closeout.

### Step 4.1: Demo, target proof, and closeout

**Reference**: [03-04](03-04-demo-and-smoke.md) · AR-13, AR-14, AR-18–AR-22

- [x] 4.1.1 Add `examples/amiga/exec-demo.clo` with the worker stress loop, library, trackdisk reads, console helper/window/device with an aborted read, input/keyboard open/close, and report line; compile it on both hosted profiles and run the demo-shape case (IT-8) ✅ (completed: 2026-11-05 19:30; both `hosted-amiga-13` and `hosted-amiga` builds pass; 11/11 impl tests green)
- [x] 4.1.2 Add `docker/scripts/a500-exec-smoke.sh` (including the `identify` non-blank check), the `sdk-a500-exec` Makefile target, and the `verify` leg; run `make sdk-a500-exec` on the FS-UAE A500/KS1.3, fix target-only defects, and capture the non-blank `docs/amiga-sdk/a500-exec.png` ✅ (completed: 2026-11-05 19:46; final gate reports `exec: ok task 464 lib 34 dev 512 io 13 key -2 in 1` and the 620x22 golden matches; RV-1–RV-3 fixed)
- [x] 4.1.3 Final documentation consistency pass: `spec/appendix-amiga.md`, the per-phase `spec/appendix-stdlib.md` §7/§13 text, `docs/known-limitations.md`, `AGENTS.md`, `docs/amiga-sdk/registry.tsv` (8 done, 3 deferred, with the acceptance/evidence/spec columns), `bundles.tsv`, `coverage.md` via `make sdk-registry`, issue #8, `plans/amiga-sdk/00-index.md`, `plans/00-roadmap.md`, and `codeops/specialist-check.json` ✅ (completed: 2026-11-05 19:49; appendix A/B aligned, `make sdk-registry` OK with 79 capabilities and 10 bundles, 62 plan folders recorded, issue #8 body regenerated; `spec/check.sh` OK)
- [x] 4.1.4 Run `make check`, `make verify`, and `make sdk-registry`; regenerate the issue body, comment, and close #8; commit locally through the active commit mode; confirm a clean tree with no decrypted ROM ✅ (completed: 2026-11-05 20:05; `make check` 107 suites green; `make verify` all legs passed headlessly including `sdk-a500-exec`; `make sdk-registry` OK; issue #8 commented and CLOSED; the stale pre-Bundle-7 exec-gating controls were updated/removed — RV-7)
- [x] 4.1.5 Run the independent correctness/memory-safety closeout review; fix every critical/major finding and re-run the affected gates ✅ (completed: 2026-11-05 20:33; RV-C1–RV-C9 and MS-1–MS-5 fixed: `CheckIO` reply reap, device-owned async buffers with copy-out, refcounted console windows, creator priority guard, extended-device rejection, generated-drop and demo checks; `make check` 107 suites and `make verify` all legs green on the review-fixed tree)
- [ ] 4.1.6 Re-run `spec/check.sh`, the affected Rust suites, and the final status checks; commit the review fixes locally; update the Progress header to Done

**Verify**: `make check && make verify && make sdk-registry`

---

## Success Criteria

1. The full Exec/device/interrupt surface compiles on `hosted-amiga-13` and
   `hosted-amiga`; the host profile rejects it with `E11200`.
2. The existing `exec-self-test` and `message-drop` transcripts are unchanged.
3. `make sdk-a500-exec` reports
   `exec: ok task 464 lib 34 dev 512 io 13 key -2 in 1` and the non-blank line
   crop matches `docs/amiga-sdk/a500-exec.png`.
4. The earlier A500 gates and the full `make verify` stay green.
5. Registry Bundle 7 is 8/11 done with SDK-EXE-007/010/011 deferred; issue #8
   is closed; the programme index and roadmap show Bundle 7 done and Bundle 8
   next.
6. No language, grammar, diagnostic, or ABI change.
