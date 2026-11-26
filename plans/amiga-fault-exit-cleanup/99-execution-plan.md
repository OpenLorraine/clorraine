# Execution Plan: Amiga fault-exit cleanup

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-26 11:22
> **Progress**: 0/55 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

The plan has six phases:
1. Build the registry and exit hook, and register the exec and DOS kinds, proven on `vamos`.
2. Register the interrupt servers, the remaining base-shim kinds, library bases, and chip state,
   proven by the new A500 gate.
3. Register the 2.0+ and optional suffix kinds.
4. Handle worker-task faults.
5. Add Ctrl-C.
6. Close out: wire the gate into `make verify`, update the docs, and record the bench evidence.

Feature phases 1–5 follow the specification-first order. Phase 6 is a closeout phase with no new
behaviour.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Registry, exit hook, exec and DOS kinds | 12 |
| 2 | Interrupt servers, base-shim kinds, library bases, chip state | 13 |
| 3 | 2.0+ and optional suffix kinds | 10 |
| 4 | Worker-task faults | 7 |
| 5 | Ctrl-C | 8 |
| 6 | Closeout | 5 |

**Total: 55 tasks across 6 phases.**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated
>    stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks
> current means progress is invisible after crashes, context resets, or session handoffs.

Rules for every task:
- Source, tests, and doc comments restate rationale in plain language. They never cite AR/ST/ASM/
  DEF identifiers or `plans/` (AGENTS.md).
- Probe and scratch output goes under `build/tmp`.
- Review dispatch per phase: correctness reviewer plus `amiga-platform-reviewer`
  ([00-index §Specialist Agents](00-index.md#specialist-agents)).

---

## Phase 1: Registry, exit hook, exec and DOS kinds

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: xhigh — a new exit mechanism on the fault path of every hosted-amiga program;
> re-entry and ordering hazards.

### Step 1.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-1..ST-9, ST-3b · [03-05 §vamos leg](03-05-gates-and-docs.md#vamos-leg) · AR-7, AR-20
**Objective**: Express the `vamos` fault-cleanup behaviour before any shim change.

- [ ] 1.1.1 [spec-author] Write the seven `vamos` probes from ST-1..ST-6 and ST-3b — `examples/robustness/fault-cleanup/{lock,port,message,reply,library}-fault.clo`, `panic-after-acquire.clo`, `assert-after-acquire.clo`
- [ ] 1.1.2 [spec-author] Write the spec test for ST-1..ST-6, ST-3b, ST-8, ST-9 (both profiles, leak pattern, SDK skip) — `compiler/tests/amiga_fault_cleanup_spec_test.rs`
- [ ] 1.1.3 [spec-author] Extend the fault leg per ST-7: both profiles, the leak pattern on every case, the seven probes as cases, a corrected header count — `docker/scripts/run-fault-amiga.sh`
- [ ] 1.1.4 Red phase: run `make -C docker exec CMD="cargo test --workspace --test amiga_fault_cleanup_spec_test"` and `make run-fault-amiga`; record each failing ST and the justification for any that pass (ST-7, ST-8 per 07 §Red-phase expectations) in this task line

### Step 1.2: Implementation

**Reference**: [03-01](03-01-exit-registry.md) · [03-02 §Base shim](03-02-registration-sites.md#base-shim--both-hosted-profiles-shim_hosted_amigac) · AR-1, AR-2, AR-8, AR-9, AR-15, AR-18, AR-21
**Objective**: Registry, hook, re-entry, and the exec/DOS kinds.

- [ ] 1.2.1 Implement `cl_res` and `cl_res_add`/`cl_res_remove`/`cl_res_track`/`cl_res_untrack` with the once-only hook install, per 03-01 §New Types, §New Functions — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 1.2.2 Implement `cl_exit_hook` (phase 3, pop-before-release, idempotent under libnix's nested-exit re-run) and `cl_exiting`, per 03-01 §Exit phases, §Re-entry; `cl_trap`/`cl_panic` keep `exit(20)` and never name the hook — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 1.2.3 Register the exec kinds `MsgPort`, `NewMessage`, `Message`, and the user `Library`, with the in-flight set (including the reply hand-off of a program-created message) and the port release rule, per 03-01 §Messages in flight and the 03-02 rows — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 1.2.4 Register `Lock`, `PathInfo`, `VolumeInfo`, and `Device` per the 03-02 rows — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 1.2.5 Green phase: rerun the 1.1.4 commands; every ST in 1.1.2 and 1.1.3 passes (fix the implementation, never the test)

### Step 1.3: Implementation tests and hardening

**Reference**: [07 §Implementation Tests](07-testing-strategy.md#implementation-tests-edge-cases-internals)

- [ ] 1.3.1 Write impl tests for the registry operations, hook phase order, pop-before-release, and the phase-1 kind registrations; update only the intentionally changed pins — `compiler/tests/amiga_fault_cleanup_impl_test.rs`, `compiler/tests/amiga_exec_impl_test.rs`
- [ ] 1.3.2 Cross-check with the existing m68k corpus: `make corpus-amiga` (GCC and vbcc build-only legs warning-free)
- [ ] 1.3.3 Phase gate: `make check` and `make run-fault-amiga`

**Deliverables**:
- [ ] ST-1..ST-9 and ST-3b green on both profiles
- [ ] All verification passing

**Test slices**: vamos fault cleanup (task), run-fault leg (release)
**Task verify**: `make check` — 4405 tests observed 2026-11-26
**Verify (gate)**: `make check` and `make run-fault-amiga` (the full `make verify` runs at 6.1.5)

---

## Phase 2: Interrupt servers, base-shim kinds, library bases, chip state

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: xhigh — interrupt-server removal, display restore, and window ordering on real
> Kickstart 1.3.

### Step 2.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-10..ST-13, ST-17 · [03-05 §A500 gate](03-05-gates-and-docs.md#a500-gate-sdk-a500-fault-cleanup) · AR-7, AR-17, AR-20
**Objective**: The A500 gate exists and fails on the baseline.

- [ ] 2.1.1 [spec-author] Write the walker fixture and the gate script (HDF boot plus the per-run capture ADF in DF1, per 03-05 §A500 gate), plus a `Makefile` target `sdk-a500-fault-cleanup` (not yet in `verify`) — `docker/scripts/fault-cleanup-walker.c`, `docker/scripts/a500-fault-cleanup-smoke.sh`, `Makefile`
- [ ] 2.1.2 [spec-author] Write the probes for ST-10..ST-13 and ST-17, and their gate expectations — `examples/robustness/fault-cleanup/{handler,display,window,font}-fault.clo`, `clean-exit.clo`, `docker/scripts/a500-fault-cleanup-smoke.sh`
- [ ] 2.1.3 Red phase: `make sdk-a500-fault-cleanup`; record the walker mismatches per ST in this task line

### Step 2.2: Implementation

**Reference**: [03-01 §Exit phases, §Chip state](03-01-exit-registry.md#exit-phases) · [03-02 §Base shim, §Library bases, §Interrupt servers](03-02-registration-sites.md) · AR-1, AR-2, AR-8, AR-9, AR-14

- [ ] 2.2.1 Exit phase 1 and the hook install in `cl_add_handler`/`cl_add_port_handler` — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 2.2.2 Implement `cl_lib_open` and exit phase 5 (03-01 §Library bases); route the base-shim library opens (`GfxBase`, `IntuitionBase`, `DiskfontBase`, `LayersBase`) through it — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 2.2.3 Register `Screen` and `Window` (unregister in `cl_window_destroy`) — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 2.2.4 Register `ChipBuffer`, `Bitmap`, `Copper` (with the hand-off in `cl_video_display_set_copper`), and `Display` — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 2.2.5 Register `Sprite` and `SpriteAnimation`; add the SPRITES-flag restore and exit phase 4 (CLXCON) — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 2.2.6 Register `Audio`, `Sample`, `Timer`, `Font`, and `Layer` — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 2.2.7 Green phase: `make sdk-a500-fault-cleanup` (ST-10..ST-13, ST-17) and the 1.1.4 commands pass

### Step 2.3: Implementation tests and hardening

- [ ] 2.3.1 Write impl tests for the phase-2 kind registrations, phase 1 and phase 4, and the `cl_lib_open` use at the base-shim sites; update only the intentionally changed pins — `compiler/tests/amiga_fault_cleanup_impl_test.rs`, `compiler/tests/amiga_intuition_impl_test.rs`
- [ ] 2.3.2 Write the impl test that cross-builds a program importing every `std.amiga` module on both profiles with `-Werror` (both CRTs) — `compiler/tests/amiga_fault_cleanup_impl_test.rs`
- [ ] 2.3.3 Phase gate: `make check`, `make run-fault-amiga`, `make sdk-a500-fault-cleanup`, and the A500 gates the base shim touches (`make sdk-a500-boing sdk-a500-sprites sdk-a500-paddle sdk-a500-text sdk-a500-assets sdk-a500-gui sdk-a500-exec sdk-a500-diskfont`)

**Deliverables**:
- [ ] ST-10..ST-13 and ST-17 green on the A500
- [ ] Existing A500 gate report lines and captures unchanged

**Test slices**: A500 acceptance (release), vamos fault cleanup (task)
**Verify (gate)**: as 2.3.3

---

## Phase 3: 2.0+ and optional suffix kinds

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — mechanical registration across five files, following the phase-1 rules.

### Step 3.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-19 · AR-7

- [ ] 3.1.1 [spec-author] Write the ST-19 probe, its spec-test case, and its run-fault case (`hosted-amiga` only) — `examples/robustness/fault-cleanup/utility-fault.clo`, `compiler/tests/amiga_fault_cleanup_spec_test.rs`, `docker/scripts/run-fault-amiga.sh`
- [ ] 3.1.2 Red phase: run the ST-19 case; record the leak line

### Step 3.2: Implementation

**Reference**: [03-02 §2.0+ suffix, §Optional suffixes, §Library bases](03-02-registration-sites.md) · [03-01 §Messages in flight](03-01-exit-registry.md#messages-in-flight) · AR-1, AR-9, AR-15

- [ ] 3.2.1 Register the 2.0+ kinds and route their bases through `cl_lib_open` — `compiler/runtime/shim_hosted_amiga_20.c`
- [ ] 3.2.2 Register the `_optional.c` kinds and route its bases through `cl_lib_open` (`UtilityBase` per 03-01 §Library bases) — `compiler/runtime/shim_hosted_amiga_optional.c`
- [ ] 3.2.3 Register the UI and net kinds and their bases — `compiler/runtime/shim_hosted_amiga_optional_ui.c`, `compiler/runtime/shim_hosted_amiga_optional_net.c`
- [ ] 3.2.4 Register the services kinds and their bases; apply the ARexx in-flight rule — `compiler/runtime/shim_hosted_amiga_optional_services.c`
- [ ] 3.2.5 Register the RTG kinds and their bases — `compiler/runtime/shim_hosted_amiga_optional_rtg.c`
- [ ] 3.2.6 Green phase: ST-19, ST-1..ST-9, and ST-3b pass

### Step 3.3: Implementation tests and hardening

- [ ] 3.3.1 Write impl tests that every suffix acquire site registers and every base site uses `cl_lib_open`; update only the intentionally changed pins — `compiler/tests/amiga_fault_cleanup_impl_test.rs`, `compiler/tests/amiga_workbench_impl_test.rs`
- [ ] 3.3.2 Phase gate: `make check`, `make run-fault-amiga`, `make corpus-amiga`, and the 2.0+ gates (`make sdk-a600-services sdk-a1200-workbench sdk-a600-addons sdk-a600-ks31-addons sdk-a1200-ks30-addons sdk-a1200-addons sdk-a1200-rtg sdk-a4000-rtg-piv`)

**Deliverables**:
- [ ] ST-19 green; the 2.0+ gates unchanged

**Test slices**: vamos fault cleanup (task), run-fault leg (release)
**Verify (gate)**: as 3.3.2

---

## Phase 4: Worker-task faults

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: xhigh — cross-task fault handling under Exec scheduling rules.

### Step 4.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-14, ST-18 · [03-03](03-03-worker-faults.md) · AR-3, AR-10

- [ ] 4.1.1 [spec-author] Write the ST-14 and ST-18 probes and their gate steps — `examples/robustness/fault-cleanup/worker-fault.clo`, `worker-fault-unjoined.clo`, `docker/scripts/a500-fault-cleanup-smoke.sh`
- [ ] 4.1.2 Red phase: `make sdk-a500-fault-cleanup`; record the hang or timeout per ST

### Step 4.2: Implementation

**Reference**: [03-03](03-03-worker-faults.md) · AR-3, AR-9 rules 6–8, AR-10, AR-13c, AR-18

- [ ] 4.2.1 Add the worker list (with `Forbid` around link and unlink), the fault record, and the detection and worker path in `cl_trap`/`cl_panic` per 03-03 §Proposed Changes — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 4.2.2 Add the report at join and exit phase 2 (removal and the unreported-fault line) per 03-03 §Report at join, §Exit removal — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 4.2.3 Green phase: ST-14 and ST-18 pass; ST-10..ST-13 and ST-17 still pass

### Step 4.3: Implementation tests and hardening

- [ ] 4.3.1 Write impl tests that detection never reads `tc_UserData` and walks under `Forbid`, `cl_trap`/`cl_panic` never name `cl_exit_hook`, the worker path has no stdio or `exit`, and the record is marked reported before the line is printed and the line before `RemTask`; update the join pins — `compiler/tests/amiga_fault_cleanup_impl_test.rs`, `compiler/tests/amiga_exec_impl_test.rs`
- [ ] 4.3.2 Phase gate: `make check`, `make sdk-a500-fault-cleanup`, `make sdk-a500-exec`

**Deliverables**:
- [ ] ST-14 and ST-18 green on the A500

**Verify (gate)**: as 4.3.2

---

## Phase 5: Ctrl-C

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — device-protocol rewrites at the listed waits, with abort-and-reap on a
> break.

### Step 5.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-15, ST-16 · [03-04](03-04-ctrl-c.md) · AR-5, AR-11–13, AR-17

- [ ] 5.1.1 [spec-author] Write the ST-15 and ST-16 probes and the `xdotool` Ctrl-C steps in the gate — `examples/robustness/fault-cleanup/break-timer.clo`, `break-own-mask.clo`, `docker/scripts/a500-fault-cleanup-smoke.sh`
- [ ] 5.1.2 Red phase: `make sdk-a500-fault-cleanup`; record ST-15 failing (`done`, no `***Break`) and ST-16's pre-implementation pass with its justification. ST-16's `break bit` proves that the console Ctrl-C reaches a probe with redirected output; if it does not appear, mark this task `[!]` and ask the user before any implementation (03-05 §A500 gate)

### Step 5.2: Implementation

**Reference**: [03-04 §Breakable waits, §Precedence, §During the exit](03-04-ctrl-c.md) · AR-11, AR-12, AR-13

- [ ] 5.2.1 Add the break routine, and the breakable window-event wait, `waitSignals` mask rule, and join wait — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 5.2.2 Convert `cl_device_do_io`, `cl_device_wait_io`, and the timer wait to SendIO + `Wait` with abort-and-reap; add `pending` to `cl_timer` and the abort in `cl_timer_free` — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 5.2.3 Make the AppMessage, GadTools, and AmigaGuide waits breakable — `compiler/runtime/shim_hosted_amiga_20.c`, `shim_hosted_amiga_optional_ui.c`, `shim_hosted_amiga_optional_net.c`
- [ ] 5.2.4 Green phase: ST-15 and ST-16 pass; ST-10..ST-14, ST-17, ST-18 still pass

### Step 5.3: Implementation tests and hardening

- [ ] 5.3.1 Write impl tests: the break bit at every listed wait (dropped under `cl_exiting`), the unchanged unbreakable sites, and the timer abort; update the `WaitPort`/`WaitIO` pins — `compiler/tests/amiga_fault_cleanup_impl_test.rs`, `amiga_exec_impl_test.rs`, `amiga_workbench_impl_test.rs`
- [ ] 5.3.2 Phase gate: `make check`, `make sdk-a500-fault-cleanup`, and the gates that use the converted waits (`make sdk-a500-exec sdk-a500-gui sdk-a600-addons sdk-a1200-addons`)

**Deliverables**:
- [ ] ST-15 and ST-16 green on the A500

**Verify (gate)**: as 5.3.2

---

## Phase 6: Closeout

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: medium — wiring, docs, and measurement.

### Step 6.1: Gate wiring, docs, and evidence

**Reference**: [03-05 §A500 gate, §Docs, §Bench evidence](03-05-gates-and-docs.md) · AR-6, AR-18, AR-19

- [ ] 6.1.1 Add `sdk-a500-fault-cleanup` to `make verify` and `.PHONY`; update the `AGENTS.md` command table and verify row; correct the run-fault count — `Makefile`, `AGENTS.md`, `site/project/verification.md`
- [ ] 6.1.2 Update the doc comments and add the registry contract comment block per 03-05 §Docs — `stdlib/std.amiga.exec.clo`, `std.amiga.interrupt.clo`, `std.amiga.dos.clo`, `std.amiga.timer.clo`, `compiler/runtime/shim_hosted_amiga.c`
- [ ] 6.1.3 Add the known-limitations entries per 03-05 §Docs — `docs/known-limitations.md`
- [ ] 6.1.4 Bench evidence: `make bench-base COMMIT=<the commit before task 1.1.1>`, `make bench`, `make bench-compare` (empty `REQUIRE`), `make bench-flags`; record the results here and in `07` (AR-18)
- [ ] 6.1.5 Plan gate: `make verify` (full, including `sdk-a500-fault-cleanup` and `run-fault-amiga`)

**Deliverables**:
- [ ] `make verify` green; bench comparison within the 1% guard
- [ ] Docs updated

**Verify (gate)**: `make verify`

---

## Dependencies

```
Phase 1 (registry, hook)
    ↓
Phase 2 (servers, base kinds, A500 gate) ──► Phase 3 (suffix kinds)
    ↓
Phase 4 (worker faults; uses the gate and exit phases)
    ↓
Phase 5 (Ctrl-C; uses the join and device changes)
    ↓
Phase 6 (closeout)
```

Phase 3 depends only on Phase 1's registry. It runs after Phase 2 so the base-shim rules are
settled first.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ `make verify` passing, including the new gate and the extended fault leg
3. ✅ No warnings or errors (GCC `-Werror` on both CRTs; vbcc build leg)
4. ✅ No dead code: no unused thunks, fields, or helpers
5. ✅ Exec safety: no server, worker, or foreign task touches freed memory (ST-10, ST-14, ST-18, ST-3)
6. ✅ Documentation updated (03-05 §Docs)
7. ✅ Phase reviews passed (correctness reviewer + `amiga-platform-reviewer`)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
