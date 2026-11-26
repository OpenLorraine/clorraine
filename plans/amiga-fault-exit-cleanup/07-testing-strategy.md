# Testing Strategy: Amiga fault-exit cleanup

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Registry, hook phases, worker fault path, break routine | every branch exercised by an ST-case or an impl test |
| Registration sites (52 registered `std.amiga` kinds; `Task` uses the worker phase) | every acquire and remove point checked by an emitted-shim shape test |
| Docs and gate wiring | reviewed |

- Rust test names state behavior (`a_lock_is_released_when_the_program_faults`).
- Integration tests run on the SDK legs: m68k GCC + `vamos` in `make check`, FS-UAE in
  `make verify`. Host-only CI skips the m68k legs, as the existing Amiga suites do.
- E2E: the A500 gate is the end-to-end test on real Kickstart 1.3 (AR-7, AR-17). An A1200 leg
  is `N/A`. The user chose A500 only, so the 2.0+ suffix kinds are covered by shape tests and
  review, not by a boot (AR-7).

## Test Slices & Groups

| Slice | Tier | Test Files | Sources | Depends On | Gate |
| ----- | ---- | ---------- | ------- | ---------- | ---- |
| vamos fault cleanup | task | `compiler/tests/amiga_fault_cleanup_spec_test.rs`, `…_impl_test.rs` | `compiler/runtime/shim_hosted_amiga*.c`, `examples/robustness/fault-cleanup/` | m68k GCC, `vamos` | `make check` |
| run-fault leg | release | `docker/scripts/run-fault-amiga.sh` | same | `make check` | `make run-fault-amiga` (in `make verify`) |
| A500 acceptance | release | `docker/scripts/a500-fault-cleanup-smoke.sh` | same + `docker/scripts/fault-cleanup-walker.c` | licensed media under `~/amiga` | `make sdk-a500-fault-cleanup` (in `make verify`) |

**Task verify**: `make check` (AR-19).
**Task verify evidence**: 4405 Rust tests passed in 157 test binaries, plus the bench, fuzz-unit,
and fuzz-smoke suites, rc 0, observed 2026-11-26 09:14 at `1462063e`.
**Verify (gate)**: `make verify` (AR-19).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These test cases are derived EXCLUSIVELY from requirements (`01-requirements.md`), component
> specs (`03-XX-*.md`), the exec/dos/graphics autodocs, and the Ambiguity Register. They define
> expected behavior BEFORE any implementation exists.
>
> **IMMUTABLE ORACLE RULE:** Do NOT modify these expectations to match the implementation. If the
> implementation does not match a spec test case, the implementation is wrong — not the test.
>
> Test code quotes the behavior in plain language, never an ST/AR identifier or a `plans/` path.

**Leak pattern** (used by ST-1..ST-7, ST-3b, and ST-19): a `vamos` output line matching
`can't expunge|mem_alloc:WARNING:\s+orphan|shutdown: can't expunge` is a leak (AR-20).

**Walker equality** (used by ST-10..ST-18): the `after` walker line equals the `before` line
field for field, apart from the tag ([03-05 §A500 gate](03-05-gates-and-docs.md#a500-gate-sdk-a500-fault-cleanup)).

### `vamos` leg — both `hosted-amiga` and `hosted-amiga-13`

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | `lock-fault.clo`: `lockPath("")` and `pathInfo("")` (the current directory), then index 5 of a 2-element array | stdout line 1 `error E11211: index out of range`; exit status 20; no leak line | FR-1, FR-2, AR-1, AR-2; dos `Lock`/exec `AllocMem` autodocs |
| ST-2 | `port-fault.clo`: `createMsgPort()` and `createPublicPort("ClFaultCleanup")`, then integer division by zero | line 1 `error E11210: integer division by zero`; exit 20; no leak line | FR-1, AR-1 |
| ST-3 | `message-fault.clo`: reply port and destination port (both own), `createMessage(reply, 7)`, `putMsg(dest, msg)`, then index fault | line 1 `error E11211: index out of range`; exit 20; no leak line | FR-5, AR-15, AR-21 |
| ST-3b | `reply-fault.clo`: reply port and destination port (both own), `createMessage(reply, 7)`, `putMsg(dest, msg)`, `getMsg(dest)`, `replyMsg`, then an index fault before the reply is fetched from `reply` | line 1 `error E11211: index out of range`; exit 20; no leak line | FR-5, AR-21 |
| ST-4 | `library-fault.clo`: `openLibrary("dos.library", 33)`, then index fault | line 1 `error E11211: index out of range`; exit 20; no leak line | FR-1, AR-1; exec `OpenLibrary`/`CloseLibrary` autodoc |
| ST-5 | `panic-after-acquire.clo`: `lockPath("")`, then `panic("stop")` | line 1 `panic: stop`; exit 20; no leak line | FR-2, AR-2 |
| ST-6 | `assert-after-acquire.clo`: `lockPath("")`, then `assert(false)` | line 1 `error E11213: assertion failed`; exit 20; no leak line | FR-2, AR-2 |
| ST-7 | the 8 existing `run-fault-amiga` programs | each keeps its current first line and exit 20, and now on both profiles with no leak line | FR-2, FR-6, AR-7, AR-9 rule 10 |
| ST-8 | `examples/amiga/exec-self-test.clo` and `hamurabi` runs | stdout equals the committed transcripts byte for byte, so the hook prints nothing on a clean exit | AR-9 rule 10 |
| ST-9 | `examples/hello/hello.clo` and `lock-fault.clo` built with the release flags but without `-s`, both profiles | `m68k-amigaos-nm` of hello shows neither `cl_exit_hook` nor `atexit`; `lock-fault` shows both | AR-18 |
| ST-19 | `utility-fault.clo` (`hosted-amiga` only; the 2.0+ suffix): `newTagList(4)`, `currentDateStamp()`, `createSemaphore()`, then index fault | line 1 `error E11211: index out of range`; exit 20; no leak line | FR-1, AR-1; exec `AllocMem` autodoc |

### A500/KS1.3 FS-UAE leg — `hosted-amiga-13` builds (`make sdk-a500-fault-cleanup`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-10 | `handler-fault.clo`: `addHandler(tick)`, `createPublicPort("ClFaultCleanup")`, index fault | `error E11211: index out of range`, then `rc fail`; walker equality (`vertb` and `port 0` unchanged) | FR-1, AR-1, AR-8; exec `AddIntServer` autodoc |
| ST-11 | `display-fault.clo`: two `newBitmap`, `openDisplay`, `displayFlip`, index fault | `error E11211: index out of range`, `rc fail`; walker equality (`view sys`) | FR-1, AR-1; graphics `LoadView` autodoc |
| ST-12 | `window-fault.clo`: `openScreen(320, 200, 2, …)`, `openWindow` on it, `openConsole(window, 0)` left open, index fault | `error E11211: index out of range`, `rc fail`; walker equality (`screens` unchanged: console, window, and screen are released newest first) | FR-1, AR-1, AR-22 |
| ST-13 | `font-fault.clo`: `openDiskFont` of the diskfont gate's font, index fault | `error E11211: index out of range`, `rc fail`; walker equality (`fonts` accessor count unchanged) | FR-1, AR-1, AR-23 |
| ST-14 | `worker-fault.clo`: a `createTask` worker divides by zero; the parent joins it | `error E11210: integer division by zero`, `rc fail`; walker equality (`workers 0`); the gate goes on to the next probe | FR-3, AR-3; RKM: DOS calls need a Process |
| ST-15 | `break-timer.clo`: `addHandler(tick)`, `openTimer`, prints `waiting`, `timerWait(timer, 3000)`; the gate sends Ctrl-C after `waiting` | `***Break`, `rc fail`; no `done` line; walker equality | FR-4, AR-5, AR-11, AR-12 |
| ST-16 | `break-own-mask.clo`: prints `waiting`, `waitSignals(0x1000)`, prints `break bit`, returns 0; the gate sends Ctrl-C after `waiting` | `break bit`; no `***Break`; no `rc fail`; walker equality | AR-13a |
| ST-17 | `clean-exit.clo`: `addHandler(tick)` with no `removeHandler`, `openDiskFont`, prints `clean`, returns 0 | `clean`; no `rc fail`; walker equality (`vertb`, `fonts`) | FR-1, FR-2, AR-2, AR-23 |
| ST-18 | `worker-fault-unjoined.clo`: the worker divides by zero; the parent calls `timerWait(timer, 50)`, then indexes out of range without joining | `error E11211: index out of range`, then `error E11210: integer division by zero`, then `rc fail`; walker equality (`workers 0`) | FR-3, AR-10 |
| ST-20 | `device-fault.clo`: `newBitmap`, `newLayer` on it, `newCopper`, `newSprite(2, 16)`, `newAnimation(4, 16, 2)`, `openAudio(1)`, `newSample`, index fault | `error E11211: index out of range`, `rc fail`; walker equality including `sprites`, `chip` (free chip memory), and `mem` (free memory of every kind; the emulated A500 also has slow RAM, where layers, copper lists, and device requests live) | FR-1, AR-1, AR-25 |

**Red-phase expectations** (baseline behaviour, from the assessment verifier runs):
- These fail before implementation: ST-1..ST-6, ST-3b, and ST-19 (orphan memory or open count), ST-9 (no hook
  exists), ST-10..ST-13, ST-17, and ST-20 (walker mismatch), ST-14 and ST-18 (hang), ST-15 (`done`
  line, no `***Break`).
- These may pass before implementation and are regression guards, with the justification
  recorded at the red phase: ST-7 (the existing faults acquire nothing; the new
  `hosted-amiga-13` leg and the leak check are new), ST-8, and ST-16 (`waitSignals` already
  returns a requested bit).

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/amiga_fault_cleanup_spec_test.rs` | ST-1..ST-6, ST-3b, ST-8, ST-9, ST-19 | registry, hook, vamos kinds |
| `docker/scripts/run-fault-amiga.sh` | ST-7 (and ST-1..ST-6, ST-3b, ST-19 as run-fault cases) | fault leg |
| `docker/scripts/a500-fault-cleanup-smoke.sh` | ST-10..ST-18, ST-20 | A500 acceptance |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/amiga_fault_cleanup_impl_test.rs` | emitted-shim shape: every acquire site in the 03-02 table registers after success and every listed remove point unregisters; every base site uses `cl_lib_open`; no bare `OpenLibrary` for a shim base | High |
| same | `cl_trap`/`cl_panic` test the worker-list head, never read `tc_UserData`, and never name `cl_exit_hook`; the worker-path walk runs under `Forbid`; the worker path has no stdio or `exit` | High |
| same | each breakable wait includes `SIGBREAKF_CTRL_C` and drops it under `cl_exiting`; the unbreakable sites are unchanged; `cl_timer_free` aborts a pending request | High |
| same | hook phases appear in the order servers → workers → registry → chip state → library bases; the registry loop pops before release | Med |
| same | a C harness around an emitted program calls `cl_lib_open` on test-only base slots for `dos.library` under `vamos`: a base opened there is closed at exit, an already-open base is left alone (AR-25). This is the only behavioural check of the library-close phase: on Kickstart 1.3, diskfont.library's open count did not move after a program left it open, and the graphics and intuition counts were not measured | High |
| same | cross-build of a program that imports every `std.amiga` module on both profiles with `-Werror` (both CRTs) | High |
| `amiga_exec_impl_test.rs`, `amiga_workbench_impl_test.rs`, `amiga_intuition_impl_test.rs` | update only the pins whose shim text intentionally changes (join, device wait, `WaitPort`) | Med |

### Integration Tests

| Test | Components | Description |
|---|---|---|
| `make corpus-amiga` | shim + corpus | GCC and vbcc (build-only) legs stay warning-free |
| existing `sdk-*` gates | shim + demos | report lines and captures unchanged (`make verify`) |

### End-to-End Tests

| Scenario | Steps | Expected Result |
|---|---|---|
| A500 fault cleanup | `make sdk-a500-fault-cleanup` | ST-10..ST-18 |

## Test Data

### Fixtures Needed
- The probes under `examples/robustness/fault-cleanup/` ([03-05 §Probe programs](03-05-gates-and-docs.md#probe-programs)).
- `docker/scripts/fault-cleanup-walker.c`.
- Licensed Kickstart 1.3 ROM and Workbench 1.3.5 HDF (local, never committed).

### Mock Requirements
None; every leg runs the real runtime.

## Security Test Cases

Exec safety is the boundary (01 §Security):
- ST-10, ST-14, ST-18 prove that no server or worker runs in freed code.
- ST-3 and ST-3b prove the own-port message rule, including a replied own message (AR-21).
- Licensed-media handling is checked by review of the gate script (AR-17). No other input
  surface changes.

## Verification Checklist
- [ ] All ST-cases defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase), with the listed guards justified
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] `make verify` passes; bench evidence recorded
- [ ] No regressions in existing tests
