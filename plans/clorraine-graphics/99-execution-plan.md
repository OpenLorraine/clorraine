# Execution Plan: Clorraine Amiga Graphics

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-24 20:55
> **Progress**: 27/27 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Add `std.amiga.intuition` (owned `Screen`, `openScreen`) and
`std.amiga.graphics` (`setColor`, `fill`, `draw`, `clear`) over six hosted-amiga
shim functions, a graphics corpus program with its golden, the CI legs that run
it, the spec documentation, and a story-video chapter.

**Precondition (PF-001, PF-010):** the working tree already carries the
uncommitted spike: `stdlib/std.amiga.graphics.clo` (a single module owning
`Screen` plus four shim calls) and edits to `compiler/src/stdlib.rs` and
`compiler/runtime/shim_hosted_amiga.c`. This plan **reconciles** that spike
(Phase 1 tasks 1.2.1–1.2.3) rather than starting from an empty tree, and the
spike evidence under `spikes/05-graphics/` should be committed with the plan
(git-commit skill).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Standard-library modules and shim | 13 |
| 2 | Corpus program and CI legs | 6 |
| 3 | Specification documentation | 4 |
| 4 | Story-video chapter | 4 |

**Total: 27 tasks across 4 phases** (no fabricated hour estimates — scope is
bounded by the plan's task-size criteria: one reviewable change per task)

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
>    Last Updated stamp after EVERY task — never batch updates. Only `[x]`
>    counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]`
>    task is resumed first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on
>    the same line. The plan lifecycle is `Ready`, `Executing`, `Done`, or
>    `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Standard-library modules and shim

> **Phase baseline tree**: 8e4723e51b70b00e3379d0801b3cbbe7dc3fd865
> **Scope mode**: strict
> **Lenses**: semantics, concurrency — informational; activation stays profile-driven

### Step 1.1: Specification tests (red phase)

**Reference**: [03-01](03-01-screen-lifecycle.md) · [03-02](03-02-drawing.md) · [07](07-testing-strategy.md) · AR-11, AR-22
**Objective**: Write the hosted-amiga compile and nullable-use spec tests before the modules exist, and confirm they fail.

- [x] 1.1.1 [spec-author] Write `compiler/tests/graphics_spec_test.rs` with ST-2 (importing both modules compiles clean for hosted-amiga) and ST-5 (an unchecked `Screen?` is rejected with `E10010`, a green-only regression — the rule already exists) ✅ (completed: 2026-09-24 14:53)
- [x] 1.1.2 Confirm the red phase for ST-2 only: the test fails because the modules are unresolved (ST-5 cannot demonstrate red before the module exists) ✅ (completed: 2026-09-24 14:53)

**Deliverables**:
- [ ] `graphics_spec_test.rs` exists and fails for the expected reason

**Verify**: `make check`

---

### Step 1.2: Modules and registration (reconcile the spike)

**Reference**: [02](02-current-state.md) §What Exists · [03-01](03-01-screen-lifecycle.md) §Implementation · [03-02](03-02-drawing.md) §Implementation · AR-5, AR-6, AR-7, AR-8, AR-35 · PF-001, PF-007
**Objective**: Relocate the spike's `Screen` into the split design, extend the drawing module, and prove the cross-module import resolves.

- [x] 1.2.1 Create `stdlib/std.amiga.intuition.clo` (`Screen`, `openScreen`) — `stdlib/std.amiga.intuition.clo` ✅ (completed: 2026-09-24 14:55)
- [x] 1.2.2 Rewrite the existing spike module `stdlib/std.amiga.graphics.clo` to import `Screen` from `std.amiga.intuition` and export `setColor`, `fill`, `draw`, `clear` (the spike currently owns `Screen` and lacks `draw`/`clear`) ✅ (completed: 2026-09-24 14:55)
- [x] 1.2.3 Update the existing registration in `compiler/src/stdlib.rs`: add the `std.amiga.intuition` entry and move `Screen`/`openScreen` out of the `std.amiga.graphics` export list ✅ (completed: 2026-09-24 14:55)
- [x] 1.2.4 Cross-import probe (AR-35): run the hosted-amiga stdlib-slice test `stdlib_slice_compiles_with_no_diagnostics_under_hosted_amiga` and compile a program importing both modules; if the embedded `std.amiga.graphics` import of `Screen` does not resolve, apply the transitive `inject_stdlib` fix — `compiler/src/driver.rs` ✅ (completed: 2026-09-24 14:55; probe passed — single-pass injection resolves the cross-module import, so no `driver.rs` change was needed)
- [x] 1.2.5 Green phase: ST-2 and ST-5 pass; the existing `stdlib` unit tests (ST-3, ST-6) pass ✅ (completed: 2026-09-24 14:55)

**Deliverables**:
- [ ] Both modules registered; spec tests green

**Verify**: `make check`

> **Ordering note (execution):** tasks 1.3.1 and 1.3.2 (the four NDK includes and
> the `cl_draw`/`cl_clear` shim functions) were reconciled during this step. The
> `stdlib` shim-symbol test asserts every `ffi` symbol the graphics module binds,
> so it cannot pass while `cl_draw`/`cl_clear` are absent; adding them here lets
> task 1.2.5 reach its green phase. The probe found that the single-pass injection
> already resolves the embedded cross-module import, so no `driver.rs` change was
> needed.

---

### Step 1.3: Runtime shim

**Reference**: [03-03](03-03-runtime-shim.md) §Implementation · AR-9, AR-23, AR-24
**Objective**: Implement the six shim functions cleanly under the profile flags.

- [x] 1.3.1 Add the four NDK includes to `compiler/runtime/shim_hosted_amiga.c` ✅ (completed: 2026-09-24 14:55; the includes were already present from the spike)
- [x] 1.3.2 Reconcile the shim: the spike already defines `cl_open_screen`, `cl_set_color`, `cl_fill`, and `cl_close_screen`; add `cl_draw` and `cl_clear` and keep the two library bases — `compiler/runtime/shim_hosted_amiga.c` ✅ (completed: 2026-09-24 14:55)
- [x] 1.3.3 Build a small inline probe program (or the spike program) that imports both modules, for hosted-amiga, and confirm zero warnings (`-Wall -Wextra -Werror`) — the graphics corpus program does not exist until Phase 2 (PF-002) ✅ (completed: 2026-09-24 14:56; a throwaway probe compiled clean with the exact toolchain flags)
- [x] 1.3.4 Verify the `title` lifetime contract (PF-004): confirm whether `OpenScreen` copies or retains `DefaultTitle`, then document in the module/requirements that the caller's `title` must outlive the screen ✅ (completed: 2026-09-24 14:56; `Screen.Title` is a `STRPTR`, so Intuition retains the pointer, not a copy)

**Deliverables**:
- [ ] Shim compiles warning-free; a hosted-amiga program links and builds

**Verify**: `make check`

---

### Step 1.4: Impl tests and phase verification

**Reference**: [07](07-testing-strategy.md) · [03-03](03-03-runtime-shim.md) · AR-11, AR-19, AR-22
**Objective**: Add the implementation tests and close the phase.

- [x] 1.4.1 Write `compiler/tests/graphics_impl_test.rs`: the generated C uses the six shim calls and no application-level `ffi`/`unsafe`; the drop calls `cl_close_screen` once ✅ (completed: 2026-09-24 14:59)
- [x] 1.4.2 Phase verification ✅ (completed: 2026-09-24 15:01; `make check` 15/15 green and `make verify` all legs passed after AR-36 option A)

**Deliverables**:
- [ ] Impl tests pass; no regressions

**Verify**: `make check` then `make verify`

---

## Phase 2: Corpus program and CI legs

> **Phase baseline tree**: 1ed1162917ac16afb0489fdc2dc29d75f42942cc
> **Scope mode**: strict
> **Lenses**: semantics, security — informational

### Step 2.1: Corpus program

**Reference**: [03-04](03-04-spec-corpus-video.md) §Corpus · AR-10, AR-27
**Objective**: Add the graphics corpus program, its golden, and its README.

- [x] 2.1.1 Create `spec/tests/corpus/graphics/program.clo` and `expected-output.txt` (`graphics: ok`) ✅ (completed: 2026-09-24 15:16)
- [x] 2.1.2 Create `spec/tests/corpus/graphics/README.md` (coverage, warning audit, hand-check record) ✅ (completed: 2026-09-24 15:16)

**Deliverables**:
- [ ] The corpus program builds for hosted-amiga and prints the golden

**Verify**: `make check`

---

### Step 2.2: Corpus harness and hosted-amiga build leg

**Reference**: [03-04](03-04-spec-corpus-video.md) §CI Legs · [07](07-testing-strategy.md) ST-1 · AR-20, AR-21
**Objective**: Classify the program as Amiga-only and build it in the corpus leg.

- [x] 2.2.1 Add the host-rejection spec test (ST-1, `E11200`) to `compiler/tests/corpus_spec_test.rs` ✅ (completed: 2026-09-24 15:18)
- [x] 2.2.2 Add the graphics build-only section to `docker/scripts/corpus-amiga.sh` ✅ (completed: 2026-09-24 15:18)

**Deliverables**:
- [ ] `make corpus-amiga` builds the graphics binary; the host-rejection test passes

**Verify**: `make check` then `make corpus-amiga`

---

### Step 2.3: FS-UAE graphics leg

**Reference**: [03-04](03-04-spec-corpus-video.md) §CI Legs · [07](07-testing-strategy.md) ST-4 · AR-21
**Objective**: Run and diff the graphics program under FS-UAE.

- [x] 2.3.1 Extend `docker/scripts/corpus-fs-uae.sh` to run the graphics program with `RUN_CAPTURE=1`, wait for the `graphics: ok` sentinel, and diff the golden (factor the shared sentinel/diff logic so interrupt-counter still works) ✅ (completed: 2026-09-24 15:19; both legs match under FS-UAE)

**Deliverables**:
- [ ] `make corpus-fs-uae` passes both legs

**Verify**: `make corpus-fs-uae`

---

### Step 2.4: Phase verification

**Reference**: AR-19
**Objective**: Close the corpus phase.

- [x] 2.4.1 Phase verification ✅ (completed: 2026-09-24 15:20; `make check` 15/15 and `make verify` all legs, including the graphics FS-UAE leg)

**Deliverables**:
- [ ] All corpus and spec tests pass; no regressions

**Verify**: `make check` then `make verify`

---

## Phase 3: Specification documentation

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: none

### Step 3.1: Appendix edits

**Reference**: [03-04](03-04-spec-corpus-video.md) §Specification Edits · AR-3, AR-27
**Objective**: Document the v1 surface and keep the spec check green.

- [x] 3.1.1 Update `spec/appendix-amiga.md` §1 (the `intuition`/`graphics` row from "later" to v1) and add the surface subsection ✅ (completed: 2026-09-24 19:16)
- [x] 3.1.2 Add the module sections to `spec/appendix-stdlib.md` ✅ (completed: 2026-09-24 19:16; the new §8/§9 shift the later sections and the `16-borrow-rules.md` §9 reference is updated to §11)
- [x] 3.1.3 Verify `spec/check.sh` stays green after the edits ✅ (completed: 2026-09-24 19:16; `spec check: OK (27 files, 102 compile-fail tests)`)
- [x] 3.1.4 Update `AGENTS.md` for the new surface: the two test files in the test naming map, and the two stdlib modules plus shim in the layout notes (PF-006) ✅ (completed: 2026-09-24 19:16)

**Deliverables**:
- [ ] Both appendices document the surface; `./spec/check.sh` passes

**Verify**: `make check`

---

## Phase 4: Story-video chapter

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: none

### Step 4.1: Graphics chapter

**Reference**: [03-04](03-04-spec-corpus-video.md) §Video Chapter · AR-28
**Objective**: Add the graphics program and an emulator capture to the story.

- [x] 4.1.1 Add the graphics chapter (program + caption card) to `demo/story.sh` ✅ (completed: 2026-09-24 20:54)
- [x] 4.1.2 Regenerate the story and confirm the graphics chapter renders the screen ✅ (completed: 2026-09-24 20:54; a frame from the chapter shows the bars and diagonal on a custom screen)
- [x] 4.1.3 Update `README.md`'s demo section to mention the graphics chapter ✅ (completed: 2026-09-24 20:54)
- [x] 4.1.4 Final verification of the whole feature ✅ (completed: 2026-09-24 20:55; `make check` 15/15 and `make verify` all legs, graphics FS-UAE leg included)

**Deliverables**:
- [ ] `demo/demo-full.mp4` includes the graphics chapter

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (modules + shim)
    ↓
Phase 2 (corpus + CI)  ← needs the modules and the corpus program
    ↓
Phase 3 (spec docs)    ← can run after Phase 1; independent of Phase 2's scripts
    ↓
Phase 4 (video)        ← needs Phase 2's corpus program and Phase 1's shim
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ `make check` and `make verify` pass, including the new graphics leg
3. ✅ No warnings under `-Wall -Wextra -Werror`; no dead code
4. ✅ The application surface stays above `ffi`/`unsafe`
5. ✅ `spec/check.sh` green; both appendices document the v1 surface
6. ✅ `plans/00-roadmap.md` carries `PLAN-08` at `Plan Created`
7. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
