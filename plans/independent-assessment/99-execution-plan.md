# Execution Plan: Independent Assessment of Clorraine

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-25 22:27
> **Progress**: 45/95 tasks (47%)
> **CodeOps Artifact Schema**: 1

## Overview

A read-only assessment in eleven phases run **sequentially in document order**: a baseline,
eight workstreams, adversarial verification, and synthesis. Each workstream phase follows the
assessment form of specification-first ordering (AR #10): Step 1 writes the probes and their
expected outcomes and commits them, then proves each probe discriminates (the red-phase
analogue); Step 2 runs the analysis and records observed outcomes (the green-phase analogue);
Step 3 tries to refute every finding and checks the phase gate. Method and evidence rules:
03-01. Checklists: 03-02, 03-03. Report: 03-04. Acceptance: `07` §Check points.

**Running this plan.**
- Run with `exec-plan independent-assessment --ask-commit`. Commit locally through the
  git-commit skill at two points per phase — at the end of the last Step N.1 writing task,
  before the red check runs anything, and at the phase gate — and nowhere else; push only when
  the user asks (PF-006, PB-004). Runtime ruling AR #30: this run commits at those same points
  without prompting and pushes each commit.
- The commit and task verification command is `**Task verify**` below; it never runs
  `make check`, `make verify`, `make asan`, or `make ubsan` (AR #3).
- AR #16 replaces exec-plan's post-phase quality step for Phases 0–9 (AR #22). Phase 9 is the
  independent review; Phase 10 adds the `correctness-reviewer` review of the report.
- Never edit a file outside `plans/independent-assessment/`, except this plan's own roadmap
  row and exec-plan blocker rows, and new files under `tools/assessment/` and
  `.opencode/skills/clorraine-assessment/` (03-01 §Ground rules, AR #29); never request `max` effort (ST-16).
- Criticals at `verified` or `probable` confidence, and Phase 9 verdicts resulting in a Critical, are escalated immediately (03-01 §Escalation).

**🚨 Update this document after EACH completed task!**

**Task verify**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment` (created in
task 0.1.1: whitespace check, baseline drift check, and no change under `plans/` other than
this plan's folder and the roadmap; 03-01 §Ground rules)

---

## Implementation Phases

| Phase | Title | Tasks | Reasoning |
| ----- | ----- | ----- | --------- |
| 0 | Baseline, pins, C references, measurement matrix | 11 | medium |
| 1 | WS1 Semantic core | 7 | high |
| 2 | WS2 Ownership, borrowing, drops | 9 | xhigh |
| 3 | WS3 IR and C codegen correctness | 8 | xhigh |
| 4 | WS4 Generated-code quality and optimizer | 8 | high |
| 5 | WS5 Runtime shims | 10 | xhigh |
| 6 | WS6 Standard library | 7 | high |
| 7 | WS7 Runtime and compile-time performance | 8 | high |
| 8 | WS8 Test integrity and defect history | 8 | high |
| 9 | Adversarial verification | 9 | xhigh |
| 10 | Synthesis, report, issue hand-offs, and repeatability | 10 | high |

**Total: 95 tasks across 11 phases**, run in document order (§Dependencies).

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
>    first, else the first `[ ]` task. Inside a task, resume from the workstream file's
>    coverage table.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks
> current means progress is invisible after crashes, context resets, or session handoffs.

---

## Phase 0: Baseline, pins, C references, measurement matrix

> **Phase baseline tree**: cbe40157ba205e36b0f58379963103c91013e3b6
> **Reasoning**: medium — mechanical baseline capture and small reference programs (AR #19)

### Step 0.1: Probe specification

**Reference**: 03-01 §Ground rules, §Probe layout, §Measurement rules · AR #5, AR #7, AR #11, AR #23
**Objective**: Pin the baseline and write the references, calibration, and measurement script with their expectations first.

- [x] 0.1.1 Confirm with the user that the working tree is settled; record `> **Baseline**: <sha>` and the date in `findings/WS0-baseline.md`; write `probes/WS0/drift-check.sh` implementing the drift check of 03-01 §Ground rules, taking the plan folder as its argument and reading the baseline from that folder's `findings/WS0-baseline.md` (no run path written in), and run it ✅ (completed: 2026-11-25 19:07)
- [x] 0.1.2 Write one equivalent C reference per scored benchmark (`container`, `local-table`, `lookup`, `matrix`, `moves`, `sieve`, `strings`, `text-scan`) and `calibration.c` under `probes/WS0/ref/`, following the fairness rules of 03-01 §Measurement rules, each with its golden output as the expectation (ST-13) ✅ (completed: 2026-11-25 19:10)
- [x] 0.1.3 Write `probes/WS0/measure.py` per 03-01 §Measurement rules (lens flag arrays; only the listed `bench.py` functions; the plan folder and baseline SHA as arguments; output under `build/tmp/assessment/`); commit Step 0.1 through the git-commit skill, local only; then red check: the comparator rejects a mutated expectation (comparator form, ST-5) ✅ (completed: 2026-11-25 19:12)

### Step 0.2: Measurement

**Reference**: 03-01 §Ground rules (Pinned tools), §Measurement rules · AR #14, AR #17
**Objective**: Capture every baseline number later phases cite.

- [x] 0.2.1 Build the release and debug `clc` in the container; copy the release binary to `build/tmp/assessment/baseline/clc` with its sha256; record the Docker image ID, the amitools commit, and the versions of host GCC, `m68k-amigaos-gcc`, vbcc, `vamos`, and FS-UAE ✅ (completed: 2026-11-25 19:13)
- [x] 0.2.2 With the pinned `clc`, emit C for the spec corpus, the benchmarks, and `examples/amiga/` on every profile that accepts each program, twice, into `build/tmp/assessment/baseline/emit/`; record expected rejections, whether the two emissions are byte-identical, the debug-`clc` emission for WS3-09, and the C compile time of each emitted program on each C compiler (for WS3-11) ✅ (completed: 2026-11-25 19:19)
- [x] 0.2.3 Run the bench harness, the compile-time harness, and the flag audit once per 03-01 §Ground rules (pinned `--clc`, new `build/perf/assessment-baseline-*.json` outputs copied to `build/tmp/assessment/baseline/`, `--compiler-git <sha>` where supported); record the numbers (labelled nix20 where they are) and any failing floor as a `WS0` observation in `findings/WS0-baseline.md` (ST-12) ✅ (completed: 2026-11-25 19:42)
- [x] 0.2.4 Check whether `vamos` runs a 68020 CPU; record the method, or the static-section-size fallback and its named limit (ST-15) ✅ (completed: 2026-11-25 19:42)
- [x] 0.2.5 Run `measure.py` for every benchmark and both calibrations, Clorraine and C, on every lens; confirm the outputs (ST-13, green); record the full matrix, raw and netted, in `findings/WS0-baseline.md` ✅ (completed: 2026-11-25 19:49)
- [x] 0.2.6 Re-record the inventory of `02-current-state.md` §What Exists at the baseline and note every difference ✅ (completed: 2026-11-25 19:49)

### Step 0.3: Hardening and gate

- [x] 0.3.1 Write the `tools/assessment/README.md` stub with the full-mode steps (copy this plan's documents to a new `plans/assessment-<date>/`, reset progress, fill the baseline, preflight, run `exec-plan`), so a repeat is possible even if this run stalls (AR #29) ✅ (completed: 2026-11-25 19:49)
- [x] 0.3.2 Phase gate: run Task verify and check the Phase 0 row of `07` §Check points; record the result in `findings/WS0-baseline.md`; escalate any Critical; commit through the git-commit skill, local only ✅ (completed: 2026-11-25 19:49)

**Deliverables**:
- [x] `findings/WS0-baseline.md` with baseline, pinned versions, emission record, harness numbers, and the measurement matrix
- [x] `probes/WS0/` with `drift-check.sh`, `measure.py`, and the C references
- [x] `tools/assessment/README.md` stub (full mode)

**Test slices**: N/A — no test map
**Verify (gate)**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment`, then the Phase 0 row of `07` §Check points (AR #20)

---

## Phase 1: WS1 Semantic core

> **Phase baseline tree**: 4f355e2f762bb54f2acb00700b9797134ee4e1fe
> **Reasoning**: high — conformance review of mature front-end code against the spec (AR #19)

### Step 1.1: Probe specification

**Reference**: 03-02 §WS1 · 03-01 §Spec oracle, §Expectation sources · AR #10
**Objective**: Expected outcomes from accepted sources before any probe runs.

- [x] 1.1.1 Write probes and expectations for WS1-01..WS1-10 under `probes/WS1/`, each citing its expectation source (ST-5); commit them, local only ✅ (completed: 2026-11-25 20:00)
- [x] 1.1.2 Red check: each probe discriminates by contrasting input or seeded defect; log in `findings/WS1-semantic-core.md` §Falsifiability log ✅ (completed: 2026-11-25 20:05)

### Step 1.2: Analysis

- [x] 1.2.1 Review `lex`, `parse`, `load`, `resolve`, and `analysis.rs` against WS1-01..WS1-03; run their probes; record outcomes, coverage, and findings in `findings/WS1-semantic-core.md` ✅ (completed: 2026-11-25 20:23)
- [x] 1.2.2 Review `sema`, `types`, `consts` against WS1-04..WS1-06, including the folded-vs-runtime differential and `sizeof` probes on host and m68k ✅ (completed: 2026-11-25 20:23)
- [x] 1.2.3 Review `ffi.rs`, `abi.rs`, `profile.rs`, `driver.rs`, data inclusion, and robustness against WS1-07..WS1-10 ✅ (completed: 2026-11-25 20:23)

### Step 1.3: Hardening and gate

- [x] 1.3.1 Self-refutation: re-read each finding against `spec/`, `docs/known-limitations.md`, and earlier plans (03-01 §Independence); fix confidence and severity; escalate any Critical (03-01 §Escalation) ✅ (completed: 2026-11-25 20:24)
- [x] 1.3.2 Phase gate: Task verify and the Phases 1–8 row of `07` §Check points for `findings/WS1-semantic-core.md` and `probes/WS1/`; commit, local only ✅ (completed: 2026-11-25 20:24)

**Test slices**: N/A — no test map
**Verify (gate)**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment`, then the Phases 1–8 row of `07` §Check points (AR #20)

---

## Phase 2: WS2 Ownership, borrowing, drops

> **Phase baseline tree**: 2f000c048f938ccf48c18365e06f7c1a40e2c00b
> **Reasoning**: xhigh — the largest and riskiest component; false accepts are memory-unsafe (AR #19)

### Step 2.1: Probe specification

**Reference**: 03-02 §WS2 · spec chapters 03, 05, 06 §6.1, 11, 16 · 03-01 §Ground rules (Sanitizer builds) · AR #10, AR #14
**Objective**: Accept/reject and sanitizer-clean expectations from the spec before running.

- [x] 2.1.1 Write probes and expectations for WS2-01..WS2-06 (WS2a) under `probes/WS2/` ✅ (completed: 2026-11-25 20:32)
- [x] 2.1.2 Write probes and expectations for WS2-07..WS2-12 (WS2b) under `probes/WS2/`; commit Step 2.1 probes and expectations, local only ✅ (completed: 2026-11-25 20:32)
- [x] 2.1.3 Red check: the ASan/UBSan builds of 03-01 detect a seeded leak, double free, and signed overflow in a C control program, and each Clorraine probe discriminates; log in `findings/WS2-ownership.md` ✅ (completed: 2026-11-25 20:34)

### Step 2.2: Analysis

- [x] 2.2.1 WS2a: review and probe WS2-01..WS2-03 (moves, drop elaboration, loops) ✅ (completed: 2026-11-25 20:36)
- [x] 2.2.2 WS2a: review and probe WS2-04..WS2-06 (`defer`, temporaries, NLL) ✅ (completed: 2026-11-25 20:36)
- [x] 2.2.3 WS2b: review and probe WS2-07..WS2-09 (views and escapes, generics, closures) ✅ (completed: 2026-11-25 20:36)
- [x] 2.2.4 WS2b: review and probe WS2-10..WS2-12 (projections, initialization, structural review of move/drop decision sites) ✅ (completed: 2026-11-25 20:36)

### Step 2.3: Hardening and gate

- [x] 2.3.1 Self-refutation against `spec/`, known limitations, and the DEF ledger; mark regressions of closed DEFs as such; escalate any Critical ✅ (completed: 2026-11-25 20:37)
- [x] 2.3.2 Phase gate: Task verify and the Phases 1–8 row of `07` §Check points for `findings/WS2-ownership.md` and `probes/WS2/`; commit, local only ✅ (completed: 2026-11-25 20:37)

**Test slices**: N/A — no test map
**Verify (gate)**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment`, then the Phases 1–8 row of `07` §Check points (AR #20)

---

## Phase 3: WS3 IR and C codegen correctness

> **Phase baseline tree**: a8e74bda08f8608d630d755090a04c75d5cef88c
> **Reasoning**: xhigh — undefined behavior and 68000 faults hide in valid-looking C (AR #19)

### Step 3.1: Probe specification

**Reference**: 03-02 §WS3 · spec chapters 07 §7, 11, 15, 17 · AR #9, AR #10
**Objective**: Expected emission properties written before builds run.

- [x] 3.1.1 Write probes and expectations for WS3-01..WS3-11 under `probes/WS3/` (strict-flag build scripts, UB probes, odd-address probes, MMIO probes, collision probes); commit them, local only ✅ (completed: 2026-11-25 20:42)
- [x] 3.1.2 Red check: each strict-flag script fails on a seeded bad C file; each UB and odd-address probe fires on a seeded violation or its item is marked `limit`; log in `findings/WS3-codegen.md` ✅ (completed: 2026-11-25 21:04)

### Step 3.2: Analysis

- [x] 3.2.1 WS3-01, WS3-02: strict host/m68k builds and vbcc builds of the Phase 0 emission; classify every warning class ✅ (completed: 2026-11-25 21:36)
- [x] 3.2.2 WS3-03, WS3-04: evaluation order, overflow paths outside `-fwrapv`, shifts, conversions, aliasing; UBSan probes on host ✅ (completed: 2026-11-25 21:36)
- [x] 3.2.3 WS3-05..WS3-07: 68000 odd-address access (`vamos`, and FS-UAE A500 where needed), `volatile`/MMIO lowering, name collisions ✅ (completed: 2026-11-25 21:36)
- [x] 3.2.4 WS3-08..WS3-11: guards and fault paths (cross-check WS2-02), determinism from the Phase 0 emissions, ABI at the C boundary, shape robustness ✅ (completed: 2026-11-25 21:36)

### Step 3.3: Hardening and gate

- [x] 3.3.1 Self-refutation; escalate any Critical ✅ (completed: 2026-11-25 21:36)
- [x] 3.3.2 Phase gate: Task verify and the Phases 1–8 row of `07` §Check points for `findings/WS3-codegen.md` and `probes/WS3/`; commit, local only ✅ (completed: 2026-11-25 21:36)

**Test slices**: N/A — no test map
**Verify (gate)**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment`, then the Phases 1–8 row of `07` §Check points (AR #20)

---

## Phase 4: WS4 Generated-code quality and optimizer

> **Phase baseline tree**: 25d1a1da835a6fd1e0cca9ee9596066dccf4657a
> **Reasoning**: high — measurement-driven analysis over the Phase 0 matrix (AR #19)

### Step 4.1: Probe specification

**Reference**: 03-02 §WS4 · 03-01 §Measurement rules · AR #5, AR #17
**Objective**: Every Phase 4 probe states its expected output and the quantity it measures.

- [x] 4.1.1 Write every Phase 4 probe under `probes/WS4/`: patched-C specifications for WS4-03..WS4-06 (target shape, output identical after normalization, metric), the attribution scripts for WS4-02, and the size and flag-variant scripts for WS4-07 and WS4-08 (ST-14); commit them, local only ✅ (completed: 2026-11-25 21:40)
- [x] 4.1.2 Red check: the output comparison rejects a deliberately wrong patch, and each attribution script maps a seeded known symbol correctly; log in `findings/WS4-code-quality.md` ✅ (completed: 2026-11-25 21:43)

### Step 4.2: Analysis

- [x] 4.2.1 WS4-01, WS4-02: overhead table from the Phase 0 matrix and attribution by symbol-mapped `vamos` traces and disassembly ✅ (completed: 2026-11-25 22:23)
- [x] 4.2.2 WS4-03..WS4-05: guard, drop-flag, and temporary/copy probes; measured gain per probe ✅ (completed: 2026-11-25 22:23)
- [x] 4.2.3 WS4-06..WS4-08: `volatile`/indirection, A500 fixed and per-feature code size, flag variants on the A500 and 68020 probe lenses ✅ (completed: 2026-11-25 22:23)
- [x] 4.2.4 WS4-09, WS4-10: pass-candidate table and the answer to the optimizer question with evidence ✅ (completed: 2026-11-25 22:23)

### Step 4.3: Hardening and gate

- [x] 4.3.1 Self-refutation against the `real-workload-audit`, `generated-c-optimization`, and `performance-levers-closeout` conclusions (a new measurement is required to contradict them) ✅ (completed: 2026-11-25 22:23)
- [x] 4.3.2 Phase gate: Task verify and the Phases 1–8 row of `07` §Check points for `findings/WS4-code-quality.md` and `probes/WS4/`; commit, local only ✅ (completed: 2026-11-25 22:23)

**Test slices**: N/A — no test map
**Verify (gate)**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment`, then the Phases 1–8 row of `07` §Check points (AR #20)

---

## Phase 5: WS5 Runtime shims

> **Phase baseline tree**: d5599054563b75ca31e0ebef11b11ce590d3321c
> **Reasoning**: xhigh — 11.6k lines of C with no sanitizer coverage on the Amiga paths (AR #19)

### Step 5.1: Probe specification

**Reference**: 03-03 §WS5 · `spec/appendix-amiga.md`, NDK autodocs · 03-01 §Ground rules (FS-UAE) · AR #6, AR #9
**Objective**: Kickstart-version and resource expectations written before probes run.

- [x] 5.1.1 Write WS5a probes and expectations (WS5-01..WS5-10 on `shim_hosted_amiga.c` and `shim_hosted_amiga_20.c`) under `probes/WS5/`, naming the `vamos` or FS-UAE configuration each needs ✅ (completed: 2026-11-25 22:27)
- [x] 5.1.2 Write WS5b probes and expectations (WS5-11..WS5-14, and WS5-01..WS5-10 for the five optional shims and `picasso96.h`) under `probes/WS5/`; commit Step 5.1 probes and expectations, local only ✅ (completed: 2026-11-25 22:27)
- [ ] 5.1.3 Red check: each probe discriminates; the host-shim sanitizer probes detect a seeded fault; detectors that cannot fire are marked `limit`; log in `findings/WS5-shims.md`

### Step 5.2: Analysis

- [ ] 5.2.1 WS5a: WS5-01..WS5-03 (availability, library lifecycle, memory and chip placement), function by function, tracked in the coverage table
- [ ] 5.2.2 WS5a: WS5-04..WS5-08 (failure paths, stack and `Forbid`, signals/ports/IORequests, Ctrl-C, exit and cleanup)
- [ ] 5.2.3 WS5a: WS5-09, WS5-10 (hardware access including the `cl_video_display_flip` DMA write, data-model crossings), with FS-UAE A500/KS1.3 and A1200/KS3.1 probes where `vamos` cannot model them
- [ ] 5.2.4 WS5b: the five optional shims and `picasso96.h` against WS5-01..WS5-10
- [ ] 5.2.5 WS5b: WS5-11..WS5-14 (host shim under ASan/UBSan, freestanding shim, hot paths on 68000 and the 68020 probe, duplication and drift)

### Step 5.3: Hardening and gate

- [ ] 5.3.1 Self-refutation with `codeops/specialists/amiga-platform-reviewer.md` as the checklist; escalate any Critical
- [ ] 5.3.2 Phase gate: Task verify, the Phases 1–8 row of `07` §Check points (every coverage-table row complete for every WS5 file); commit, local only

**Test slices**: N/A — no test map
**Verify (gate)**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment`, then the Phases 1–8 row of `07` §Check points (AR #20)

---

## Phase 6: WS6 Standard library

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — API conformance and resource ownership across 43 modules (AR #19)

### Step 6.1: Probe specification

**Reference**: 03-03 §WS6 · `spec/appendix-stdlib.md`, `spec/appendix-amiga.md` · AR #10
**Objective**: Expected API behavior from the appendices before probes run.

- [ ] 6.1.1 Write probes and expectations for WS6-01..WS6-07 under `probes/WS6/`; commit them, local only
- [ ] 6.1.2 Red check: each probe discriminates; log in `findings/WS6-stdlib.md`

### Step 6.2: Analysis

- [ ] 6.2.1 WS6-01..WS6-03: API conformance, core-module boundary cases under ASan/UBSan, hidden costs
- [ ] 6.2.2 WS6-04, WS6-05: Kickstart requirements per Amiga module and handle ownership on drop and fault paths
- [ ] 6.2.3 WS6-06, WS6-07: cross-module consistency and `stdlib.rs` embedding and gating

### Step 6.3: Hardening and gate

- [ ] 6.3.1 Self-refutation; cross-check handle findings with WS5-02/WS5-03; escalate any Critical
- [ ] 6.3.2 Phase gate: Task verify, the Phases 1–8 row of `07` §Check points (every per-module coverage-table row complete); commit, local only

**Test slices**: N/A — no test map
**Verify (gate)**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment`, then the Phases 1–8 row of `07` §Check points (AR #20)

---

## Phase 7: WS7 Runtime and compile-time performance

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — measurement across both target lenses (AR #19)

### Step 7.1: Probe specification

**Reference**: 03-03 §WS7 · 03-01 §Measurement rules · AR #2, AR #14, AR #17
**Objective**: WS7's own probes and their expectations fixed before measuring.

- [ ] 7.1.1 Write WS7's probes under `probes/WS7/`: the host-time runner (≥5 runs), the ownership-heavy, generic, and closure scaling generators that `compile_time.py` lacks, and the FS-UAE frame-timing probe used only if WS5/WS6 flagged a shim cost; expectations are the golden outputs or the generator's documented size; commit them, local only
- [ ] 7.1.2 Red check: each script rejects a run whose output differs from its expectation; log in `findings/WS7-performance.md`

### Step 7.2: Analysis

- [ ] 7.2.1 WS7-01: analyze the Phase 0 matrix per lens and add host time
- [ ] 7.2.2 WS7-02, WS7-04: corpus and `examples/amiga/` footprint against the A500 budget; chip vs. fast placement
- [ ] 7.2.3 WS7-03, WS7-05: 68020 probe gains (citing WS4-08 for flags) and the profile-variant question; frame timing only where WS5/WS6 recorded a suspected shim cost
- [ ] 7.2.4 WS7-06, WS7-07: compile time and RSS scaling over the `compile_time.py` series and the WS7 generators; hotspot attribution by scaling exponent and code reading

### Step 7.3: Hardening and gate

- [ ] 7.3.1 Self-refutation against `compiler-throughput` and `performance-levers-closeout` (a new measurement is required to contradict them)
- [ ] 7.3.2 Phase gate: Task verify and the Phases 1–8 row of `07` §Check points for `findings/WS7-performance.md` and `probes/WS7/`; commit, local only

**Test slices**: N/A — no test map
**Verify (gate)**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment`, then the Phases 1–8 row of `07` §Check points (AR #20)

---

## Phase 8: WS8 Test integrity and defect history

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — cross-cutting analysis over the WS1–WS7 results and the DEF ledger (AR #19)

### Step 8.1: Probe specification

**Reference**: 03-03 §WS8 · AR #18
**Objective**: The assertion classification scheme and the DEF classification scheme fixed before analysis.

- [ ] 8.1.1 Write the static assertion-classification scheme (WS8-03) and the DEF classification scheme (WS8-07 classes, components, and the recurring threshold) under `probes/WS8/`; commit them, local only
- [ ] 8.1.2 Red check: the assertion scheme separates a known value-asserting test from a known compiles-only test, and the DEF scheme puts two hand-picked rows of different kinds in different classes; log in `findings/WS8-test-integrity.md`

### Step 8.2: Analysis

- [ ] 8.2.1 WS8-01, WS8-02: coverage map per checklist area and the gap reason for every WS1–WS7 finding of Minor or above
- [ ] 8.2.2 WS8-03..WS8-06: static assertion classification (five tests per area), fuzzer construct coverage, sanitizer holes, pinned-text tests
- [ ] 8.2.3 WS8-07: classify every DEF row in `plans/00-roadmap.md` by component and class; recurrence over time
- [ ] 8.2.4 WS8-08: structural root cause per recurring class, presence at the baseline (cross-check WS2-12), and a structural remedy proposal

### Step 8.3: Hardening and gate

- [ ] 8.3.1 Self-refutation of every root-cause claim against the code at the baseline
- [ ] 8.3.2 Phase gate: Task verify and the Phases 1–8 row of `07` §Check points for `findings/WS8-test-integrity.md` and `probes/WS8/`; commit, local only

**Test slices**: N/A — no test map
**Verify (gate)**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment`, then the Phases 1–8 row of `07` §Check points (AR #20)

---

## Phase 9: Adversarial verification

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: xhigh — adversarial review that decides which findings survive (AR #16, AR #19)

### Step 9.1: Verification specification

**Reference**: 03-01 §Verification protocol, §Agent routing · 03-04 §`05-findings.md` · AR #16, AR #22
**Objective**: Blind packets that let a fresh session reproduce, refute, and sample cleans.

- [ ] 9.1.1 Prepare one verification packet per workstream group per 03-01 §Agent routing: findings with evidence and probe paths, the checklist, the `clean` outcomes to sample, the checklist brief, and 03-01 §Verification protocol; exclude the author's reasoning beyond the finding text; commit the packets, local only
- [ ] 9.1.2 Red check: every packet's reproduction commands resolve to files committed at HEAD with Task verify passing; missing files go back to the author as finding defects

### Step 9.2: Verification

- [ ] 9.2.1 Verify WS0, WS1, and WS6 findings and the WS1/WS6 clean samples in a fresh read-only `general` session; record verdicts with the verifier session in the workstream files
- [ ] 9.2.2 Verify WS2 findings and its clean sample in a fresh session
- [ ] 9.2.3 Verify WS3 and WS4 findings and their clean samples in a fresh session
- [ ] 9.2.4 Verify WS5 findings and its clean sample in a fresh session
- [ ] 9.2.5 Last: verify WS7 and WS8 findings and their clean samples in a fresh session, and re-point WS8 gap links and themes for every refuted or merged ID

### Step 9.3: Ledger and gate

- [ ] 9.3.1 Build `05-findings.md` per 03-04: ledger, clean sample, merges, refuted, spec questions, escalations
- [ ] 9.3.2 Phase gate: Task verify and the Phase 9 row of `07` §Check points; commit, local only

**Test slices**: N/A — no test map
**Verify (gate)**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment`, then the Phase 9 row of `07` §Check points (AR #20)

---

## Phase 10: Synthesis, report, issue hand-offs, and repeatability

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — synthesis of surviving findings into the report and the issue hand-offs (AR #19)

### Step 10.1: Report specification

**Reference**: 03-04 §Report structure, §DEF rows, §GitHub issues, §Handoff · AR #8, AR #13, AR #21, AR #24–#27
**Objective**: The report skeleton and the ST-10 checklist fixed before writing.

- [ ] 10.1.1 Create `10-assessment-report.md` with the ten sections of 03-04 and an ST-10 checklist at the end; commit, local only

### Step 10.2: Writing

- [ ] 10.2.1 Write sections 1–7 (summary, scope and limits, scorecard, findings, themes, performance, quick wins) from `05-findings.md` and the workstream files, citing surviving findings only
- [ ] 10.2.2 Write sections 8–10 (remediation roadmap with unranked recommendations, spec questions, appendix without DEF IDs yet)

### Step 10.3: Review, DEF rows, issues, repeatability, and gate

- [ ] 10.3.1 Independent `correctness-reviewer` review of the report against `05-findings.md`, ST-10, and ST-12; record its findings under `05-findings.md` §Report review and fix every critical/major one
- [ ] 10.3.2 Allocate one DEF row per surviving Critical/Major in `plans/00-roadmap.md` per 03-04 §DEF rows; write the IDs back into the ledger, the findings, and the report appendix; re-check ST-10
- [ ] 10.3.3 Draft every issue under `issues/` per 03-04 §GitHub issues (finding issues, Minor tracking issues, theme parent issues): search all open issues for overlaps and propose links or comments instead of duplicates, apply the redaction rule and grep the drafts for it; present the full list, overlaps, and labels to the user and wait for approval (AR #27)
- [ ] 10.3.4 After approval: ask the user whether to push the plan commits first (permalinks are added only if pushed); create any missing labels, then the approved theme parents, then finding issues with `gh issue create --parent <theme#>`, then tracking issues, checking all open issues for duplicates first; post the approved comments on overlapping existing issues; record every issue number in the findings, the ledger, the DEF rows, and the report appendix
- [ ] 10.3.5 Promote the reusable assets per 03-04 §Repeatability: copy the two scripts and `probes/WS0/ref/` into `tools/assessment/`, and the method documents into `tools/assessment/template/` with placeholders, cleared checkboxes, the standing-default register, the widened drift-check exclusion, and only the lessons Phase 9 confirmed
- [ ] 10.3.6 Complete `tools/assessment/README.md` (full, scoped, and delta modes; the path-to-workstream table; labels and issue filing) and write the thin `.opencode/skills/clorraine-assessment/SKILL.md`, staged with `git add -f` (ST-18)
- [ ] 10.3.7 Phase gate: Task verify and the Phase 10 row of `07` §Check points; set this plan's roadmap row to Done through the roadmap skill; commit, local only; present the report and the issues to the user for the remediation-plan selection (03-04 §Handoff)

**Test slices**: N/A — no test map
**Verify (gate)**: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment`, then the Phase 10 row of `07` §Check points (AR #20)

---

## Dependencies

Phases run in document order. The true data edges, all satisfied by that order:

```
Phase 0 ──► every later phase (baseline, pins, emission, measurement matrix)
Phase 2 ──► Phase 3   (WS3-08 cross-checks WS2-02)
Phase 4 ──► Phase 7   (WS7-03 cites WS4-08)
Phase 5 ──► Phase 6   (WS6 handle findings cross-check WS5-02/03)
Phases 5, 6 ──► Phase 7   (WS7-05 runs only on a WS5/WS6 shim-cost flag)
Phases 1–7 ──► Phase 8 ──► Phase 9 ──► Phase 10
```

---

## Success Criteria

**The assessment is complete when:**

1. ✅ All phases completed
2. ✅ Every phase gate passed Task verify and its `07` §Check points row (AR #20)
3. ✅ Every checklist item has an outcome with evidence (ST-8)
4. ✅ Every finding and the clean sample have an independent verdict (ST-9)
5. ✅ `10-assessment-report.md` delivered and reviewed (ST-10)
6. ✅ DEF rows added for every surviving Critical/Major finding (AR #13, AR #21)
7. ✅ The approved GitHub issues exist as hand-offs, cross-linked (ST-17)
8. ✅ No change outside the allowed paths; drift check passing (ST-7)
9. ✅ The user has the remediation roadmap to choose from
10. ✅ The assessment can be repeated from `tools/assessment/` (ST-18)
