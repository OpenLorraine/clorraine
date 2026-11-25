# Execution Plan: Independent Assessment of Clorraine

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-25 13:17
> **Progress**: 0/89 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

A read-only assessment in eleven phases: a baseline, eight workstreams, adversarial
verification, and synthesis. Each workstream phase follows the assessment form of
specification-first ordering (AR #10): Step 1 writes the probes and their expected outcomes
from `spec/` or the C references before anything runs, and proves each probe can fail (the
red-phase analogue); Step 2 runs the analysis and records observed outcomes (the green-phase
analogue); Step 3 tries to refute every finding and checks the phase gate. Method and evidence
rules: 03-01. Checklists: 03-02, 03-03. Report: 03-04. Acceptance: `07` ST cases.

Execution starts only after the working tree is settled (AR #7). No source file changes
(AR #12). Verified criticals are escalated immediately (03-01 §Escalation).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks | Reasoning |
| ----- | ----- | ----- | --------- |
| 0 | Baseline and C references | 9 | medium |
| 1 | WS1 Semantic core | 7 | high |
| 2 | WS2 Ownership, borrowing, drops | 9 | xhigh |
| 3 | WS3 IR and C codegen correctness | 8 | xhigh |
| 4 | WS4 Generated-code quality and optimizer | 8 | high |
| 5 | WS5 Runtime shims | 10 | xhigh |
| 6 | WS6 Standard library | 7 | high |
| 7 | WS7 Runtime and compile-time performance | 8 | high |
| 8 | WS8 Test integrity and defect history | 8 | high |
| 9 | Adversarial verification | 9 | xhigh |
| 10 | Synthesis and report | 6 | high |

**Total: 89 tasks across 11 phases.** Phases 1–7 are independent of each other after Phase 0
and may run in separate sessions in any order; Phase 8 needs 1–7; Phase 9 needs 8; Phase 10
needs 9.

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

> **Assessment rules for every phase:** never edit a file outside
> `plans/independent-assessment/` and `build/` (ST-7); never run `make check`, `make verify`,
> `make asan`, or `make ubsan` (AR #3); never request `max` effort (ST-16); every probe and
> number follows 03-01. The phase gate replaces the project verify command (AR #20).

---

## Phase 0: Baseline and C references

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: medium — mechanical baseline capture and small reference programs (AR #19)

### Step 0.1: Probe specification

**Reference**: 03-01 §Ground rules, §Probe layout · 02-current-state · AR #5, AR #7, AR #11
**Objective**: Pin the baseline and write the C references with their expectations first.

- [ ] 0.1.1 Confirm the working tree is settled with the user, then record the baseline SHA, date, `clc` build, and toolchain versions (host GCC, `m68k-amigaos-gcc`, vbcc, `vamos`, FS-UAE) in `findings/WS0-baseline.md` (AR #7)
- [ ] 0.1.2 [spec-author] Write one hand-written idiomatic C reference per benchmark (`container`, `local-table`, `lookup`, `matrix`, `moves`, `sieve`, `strings`, `text-scan`) under `probes/WS0/ref/`, with the benchmark's `expected-output.txt` as the expectation (ST-13, AR #5)
- [ ] 0.1.3 Red check: show the output comparison reports a mismatch for one mutated expectation per reference; log in `findings/WS0-baseline.md` §Falsifiability log (ST-5)

### Step 0.2: Measurement

**Reference**: 03-01 §Measurement rules · AR #14, AR #17
**Objective**: Capture every baseline number later phases cite.

- [ ] 0.2.1 Build `clc` release in the container; emit C for the spec corpus, the benchmarks, and `examples/amiga/` for every profile into `build/tmp/assessment/baseline/emit/`, twice, and record whether the two emissions are byte-identical (input to WS3-09)
- [ ] 0.2.2 Run `make bench`, `make bench-compile`, and `make bench-flags` once; copy the JSON into `build/tmp/assessment/baseline/`; record the per-benchmark numbers and any failing floor in `findings/WS0-baseline.md` (ST-12)
- [ ] 0.2.3 Build and run the C references on host and hosted-amiga (`-O2` and `-Os`, shipped flags) under `vamos`; confirm byte-identical output (ST-13, green); record net instructions and `.text` next to the Clorraine numbers
- [ ] 0.2.4 Check whether `vamos` runs a 68020 CPU; measure one benchmark as a `-m68020` probe build; record the method or the FS-UAE fallback (ST-15, AR #17)
- [ ] 0.2.5 Re-record the module inventory of 02-current-state §What Exists at the baseline SHA and note every difference

### Step 0.3: Hardening and gate

- [ ] 0.3.1 Phase gate: check ST-5, ST-7, ST-12, ST-13, ST-15, ST-16 against `findings/WS0-baseline.md` and `probes/WS0/`; record the result in the baseline file

**Deliverables**:
- [ ] `findings/WS0-baseline.md` with SHA, versions, measurements, 68020 method
- [ ] `probes/WS0/ref/*.c` producing the golden outputs

**Test slices**: N/A — full verify only
**Verify (gate)**: ST-5, ST-7, ST-12, ST-13, ST-15, ST-16 (AR #20)

---

## Phase 1: WS1 Semantic core

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — conformance review of mature front-end code against the spec (AR #19)

### Step 1.1: Probe specification

**Reference**: 03-02 §WS1 · 03-01 §Spec oracle · AR #10
**Objective**: Expected outcomes from `spec/` before any probe runs.

- [ ] 1.1.1 [spec-author] Write probes and `.expected` files for WS1-01..WS1-10 under `probes/WS1/`, each expectation citing its spec chapter and section (ST-5)
- [ ] 1.1.2 Red check: show each probe can fail (mutated expectation or contrasting input); log in `findings/WS1-semantic-core.md` §Falsifiability log (ST-5)

### Step 1.2: Analysis

- [ ] 1.2.1 Review `lex`, `parse`, `resolve`, `load` against WS1-01..WS1-03; run their probes; record outcomes and findings in `findings/WS1-semantic-core.md`
- [ ] 1.2.2 Review `sema`, `types`, `consts` against WS1-04..WS1-06, including the folded-vs-runtime differential and `sizeof` probes on host and m68k
- [ ] 1.2.3 Review `ffi.rs`, `abi.rs`, `profile.rs`, data inclusion, and robustness against WS1-07..WS1-10

### Step 1.3: Hardening and gate

- [ ] 1.3.1 Self-refutation: re-read each finding against `spec/`, `docs/known-limitations.md`, and earlier plans (03-01 §Independence); fix confidence and severity
- [ ] 1.3.2 Phase gate: ST-1..ST-8, ST-11, ST-12 for `findings/WS1-semantic-core.md` and `probes/WS1/`

**Test slices**: N/A — full verify only
**Verify (gate)**: ST-1..ST-8, ST-11, ST-12 (AR #20)

---

## Phase 2: WS2 Ownership, borrowing, drops

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: xhigh — the largest and riskiest component; false accepts are memory-unsafe (AR #19)

### Step 2.1: Probe specification

**Reference**: 03-02 §WS2 · spec chapters 03, 11, 16 · AR #10, AR #14
**Objective**: Accept/reject and ASan-clean expectations from the spec before running.

- [ ] 2.1.1 [spec-author] Write probes and expectations for WS2-01..WS2-06 (WS2a) under `probes/WS2/`
- [ ] 2.1.2 [spec-author] Write probes and expectations for WS2-07..WS2-12 (WS2b) under `probes/WS2/`
- [ ] 2.1.3 Red check: prove the ASan/UBSan probe builds detect a deliberate leak, double free, and overflow in a C control program, and that each Clorraine probe can fail; log in `findings/WS2-ownership.md` (ST-5)

### Step 2.2: Analysis

- [ ] 2.2.1 WS2a: review and probe WS2-01..WS2-03 (moves, drop elaboration, loops)
- [ ] 2.2.2 WS2a: review and probe WS2-04..WS2-06 (`defer`, temporaries, NLL)
- [ ] 2.2.3 WS2b: review and probe WS2-07..WS2-09 (views and escapes, generics, closures)
- [ ] 2.2.4 WS2b: review and probe WS2-10..WS2-12 (projections, initialization, structural review of move/drop decision sites)

### Step 2.3: Hardening and gate

- [ ] 2.3.1 Self-refutation against `spec/`, known limitations, and the DEF ledger; mark regressions of closed DEFs as such
- [ ] 2.3.2 Phase gate: ST-1..ST-8, ST-11, ST-12 for `findings/WS2-ownership.md` and `probes/WS2/`

**Test slices**: N/A — full verify only
**Verify (gate)**: ST-1..ST-8, ST-11, ST-12 (AR #20)

---

## Phase 3: WS3 IR and C codegen correctness

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: xhigh — undefined behavior and 68000 faults hide in valid-looking C (AR #19)

### Step 3.1: Probe specification

**Reference**: 03-02 §WS3 · spec chapter 17 · AR #9, AR #10
**Objective**: Expected emission properties written before builds run.

- [ ] 3.1.1 [spec-author] Write probes and expectations for WS3-01..WS3-11 under `probes/WS3/` (strict-flag build scripts, UB probes, odd-address probes, MMIO probes, collision probes)
- [ ] 3.1.2 Red check: each strict-flag script fails on a seeded bad C file; each UB probe fails on a seeded violation; log in `findings/WS3-codegen.md` (ST-5)

### Step 3.2: Analysis

- [ ] 3.2.1 WS3-01, WS3-02: strict host/m68k builds and vbcc builds of the Phase 0 emission; classify every warning class
- [ ] 3.2.2 WS3-03, WS3-04: evaluation order, overflow paths outside `-fwrapv`, shifts, conversions, aliasing; UBSan probes on host
- [ ] 3.2.3 WS3-05..WS3-07: 68000 odd-address access (`vamos` and FS-UAE A500 where needed), `volatile`/MMIO lowering, name collisions
- [ ] 3.2.4 WS3-08..WS3-11: guards and fault paths, determinism (Phase 0 double emission), ABI at the C boundary, shape robustness

### Step 3.3: Hardening and gate

- [ ] 3.3.1 Self-refutation; cross-check WS3-08 findings with WS2-02
- [ ] 3.3.2 Phase gate: ST-1..ST-8, ST-11, ST-12 for `findings/WS3-codegen.md` and `probes/WS3/`

**Test slices**: N/A — full verify only
**Verify (gate)**: ST-1..ST-8, ST-11, ST-12 (AR #20)

---

## Phase 4: WS4 Generated-code quality and optimizer

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — measurement-driven analysis with the Phase 0 references (AR #19)

### Step 4.1: Probe specification

**Reference**: 03-02 §WS4 · 03-01 §Measurement rules · AR #5, AR #17
**Objective**: Each patched-C probe states its expected output (unchanged) and the shape it removes.

- [ ] 4.1.1 [spec-author] Write the patched-C probe specifications for WS4-03..WS4-06 under `probes/WS4/` (target shape, expected byte-identical output, metric to compare) (ST-14)
- [ ] 4.1.2 Red check: show the output comparison rejects a deliberately wrong patch; log in `findings/WS4-code-quality.md` (ST-5)

### Step 4.2: Analysis

- [ ] 4.2.1 WS4-01, WS4-02: overhead table against the C references and attribution by symbol-mapped `vamos` traces and disassembly
- [ ] 4.2.2 WS4-03..WS4-05: guard, drop-flag, and temporary/copy probes; measured gain per probe
- [ ] 4.2.3 WS4-06..WS4-08: `volatile`/indirection, A500 code size, flag variants on 68000 and 68020 probe builds
- [ ] 4.2.4 WS4-09, WS4-10: pass-candidate table and the answer to the optimizer question with evidence

### Step 4.3: Hardening and gate

- [ ] 4.3.1 Self-refutation against `real-workload-audit`, `generated-c-optimization`, and `performance-levers-closeout` conclusions (new measurement required to contradict them)
- [ ] 4.3.2 Phase gate: ST-1..ST-8, ST-12, ST-14, ST-15 for `findings/WS4-code-quality.md` and `probes/WS4/`

**Test slices**: N/A — full verify only
**Verify (gate)**: ST-1..ST-8, ST-12, ST-14, ST-15 (AR #20)

---

## Phase 5: WS5 Runtime shims

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: xhigh — 11.6k lines of C with no sanitizer coverage on the Amiga paths (AR #19)

### Step 5.1: Probe specification

**Reference**: 03-03 §WS5 · `spec/appendix-amiga.md` · AR #6, AR #9
**Objective**: Kickstart-version and resource expectations written before probes run.

- [ ] 5.1.1 [spec-author] Write WS5a probes and expectations (WS5-01..WS5-10 on `shim_hosted_amiga.c` and `shim_hosted_amiga_20.c`) under `probes/WS5/`, naming the `vamos` or FS-UAE configuration each needs
- [ ] 5.1.2 [spec-author] Write WS5b probes and expectations (WS5-11..WS5-14 and WS5-01..WS5-10 for the optional shims) under `probes/WS5/`
- [ ] 5.1.3 Red check: each probe can fail; the host-shim sanitizer probes detect a seeded fault; log in `findings/WS5-shims.md` (ST-5)

### Step 5.2: Analysis

- [ ] 5.2.1 WS5a: WS5-01..WS5-03 (availability, library lifecycle, memory and chip placement) across every exported function
- [ ] 5.2.2 WS5a: WS5-04..WS5-08 (failure paths, stack and `Forbid`, signals/ports/IORequests, Ctrl-C, exit and cleanup)
- [ ] 5.2.3 WS5a: WS5-09, WS5-10 (hardware access, data-model crossings), with FS-UAE A500/KS1.3 and A1200/KS3.1 probes where `vamos` cannot model them
- [ ] 5.2.4 WS5b: the four optional shims and `picasso96.h` against WS5-01..WS5-10
- [ ] 5.2.5 WS5b: WS5-11..WS5-14 (host shim under ASan/UBSan, freestanding shim, hot paths on 68000/68020, duplication and drift)

### Step 5.3: Hardening and gate

- [ ] 5.3.1 Self-refutation with the `amiga-platform-reviewer` brief (`codeops/specialists/amiga-platform-reviewer.md`) as the checklist
- [ ] 5.3.2 Phase gate: ST-1..ST-8, ST-11, ST-12, ST-15 for `findings/WS5-shims.md` and `probes/WS5/`

**Test slices**: N/A — full verify only
**Verify (gate)**: ST-1..ST-8, ST-11, ST-12, ST-15 (AR #20)

---

## Phase 6: WS6 Standard library

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — API conformance and resource ownership across 45 modules (AR #19)

### Step 6.1: Probe specification

**Reference**: 03-03 §WS6 · `spec/appendix-stdlib.md`, `spec/appendix-amiga.md` · AR #10
**Objective**: Expected API behavior from the appendices before probes run.

- [ ] 6.1.1 [spec-author] Write probes and expectations for WS6-01..WS6-07 under `probes/WS6/`
- [ ] 6.1.2 Red check: each probe can fail; log in `findings/WS6-stdlib.md` (ST-5)

### Step 6.2: Analysis

- [ ] 6.2.1 WS6-01..WS6-03: API conformance, core-module boundary cases under ASan/UBSan, hidden costs
- [ ] 6.2.2 WS6-04, WS6-05: Kickstart requirements per Amiga module and handle ownership on drop and fault paths
- [ ] 6.2.3 WS6-06, WS6-07: cross-module consistency and `stdlib.rs` embedding and gating

### Step 6.3: Hardening and gate

- [ ] 6.3.1 Self-refutation; cross-check handle findings with WS5-02/WS5-03
- [ ] 6.3.2 Phase gate: ST-1..ST-8, ST-11, ST-12 for `findings/WS6-stdlib.md` and `probes/WS6/`

**Test slices**: N/A — full verify only
**Verify (gate)**: ST-1..ST-8, ST-11, ST-12 (AR #20)

---

## Phase 7: WS7 Runtime and compile-time performance

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — measurement across both target lenses (AR #19)

### Step 7.1: Probe specification

**Reference**: 03-03 §WS7 · 03-01 §Measurement rules · AR #2, AR #14, AR #17
**Objective**: The measurement matrix and expected outputs fixed before measuring.

- [ ] 7.1.1 [spec-author] Write the measurement matrix and scaling-probe generators for WS7-01..WS7-07 under `probes/WS7/`, with the golden outputs as expectations
- [ ] 7.1.2 Red check: each measurement script rejects a run whose output differs from the golden; log in `findings/WS7-performance.md` (ST-5)

### Step 7.2: Analysis

- [ ] 7.2.1 WS7-01: the full benchmark matrix on host, A500 `-O2`/`-Os`, and A1200 68020 probe builds, Clorraine vs. C reference
- [ ] 7.2.2 WS7-02, WS7-04: corpus and `examples/amiga/` footprint against the A500 budget; chip vs. fast placement
- [ ] 7.2.3 WS7-03, WS7-05: 68020 gains and the profile-variant question; FS-UAE frame timing only where WS5/WS6 flagged a shim cost
- [ ] 7.2.4 WS7-06, WS7-07: `clc` compile time and RSS scaling (2×, 4×, 8×), hotspot attribution in `own/` and `codegen/`

### Step 7.3: Hardening and gate

- [ ] 7.3.1 Self-refutation against `compiler-throughput` and `performance-levers-closeout` (new measurement required to contradict them)
- [ ] 7.3.2 Phase gate: ST-1..ST-8, ST-12, ST-15 for `findings/WS7-performance.md` and `probes/WS7/`

**Test slices**: N/A — full verify only
**Verify (gate)**: ST-1..ST-8, ST-12, ST-15 (AR #20)

---

## Phase 8: WS8 Test integrity and defect history

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — cross-cutting analysis over the WS1–WS7 results and the DEF ledger (AR #19)

### Step 8.1: Probe specification

**Reference**: 03-03 §WS8 · AR #18
**Objective**: The mutation probes and the defect classification scheme fixed before analysis.

- [ ] 8.1.1 [spec-author] Write the assertion-strength mutation probes (WS8-03) and the DEF classification scheme (WS8-07 classes and components) under `probes/WS8/`
- [ ] 8.1.2 Red check: each mutation probe is shown to change the guarded behavior; log in `findings/WS8-test-integrity.md` (ST-5)

### Step 8.2: Analysis

- [ ] 8.2.1 WS8-01, WS8-02: coverage map per checklist area and the gap reason for every verified WS1–WS7 finding
- [ ] 8.2.2 WS8-03..WS8-06: assertion strength sample, fuzzer construct coverage, sanitizer holes, pinned-text tests
- [ ] 8.2.3 WS8-07: classify every DEF row in `plans/00-roadmap.md` by component and class; recurrence over time
- [ ] 8.2.4 WS8-08: structural root cause per recurring class, presence at the baseline (cross-check WS2-12), and a structural remedy proposal

### Step 8.3: Hardening and gate

- [ ] 8.3.1 Self-refutation of every root-cause claim against the code at the baseline SHA
- [ ] 8.3.2 Phase gate: ST-1..ST-8, ST-12 for `findings/WS8-test-integrity.md` and `probes/WS8/`

**Test slices**: N/A — full verify only
**Verify (gate)**: ST-1..ST-8, ST-12 (AR #20)

---

## Phase 9: Adversarial verification

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: xhigh — adversarial review that decides which findings survive (AR #16, AR #19)

### Step 9.1: Verification specification

**Reference**: 03-01 §Verification protocol, §Agent routing · 03-04 §`05-findings.md` · AR #16
**Objective**: Blind packets that let a fresh session reproduce and refute.

- [ ] 9.1.1 Prepare one verification packet per workstream: findings with evidence and probe paths, the checklist, and 03-01 §Verification protocol; exclude the author's reasoning beyond the finding text
- [ ] 9.1.2 Red check: confirm every packet's reproduction commands resolve to committed probe files at the baseline SHA; missing files are reported back as a finding defect

### Step 9.2: Verification

- [ ] 9.2.1 Verify WS1 and WS6 findings in a fresh session (`semantics-reviewer`, `correctness-reviewer`); record verdicts in the workstream files
- [ ] 9.2.2 Verify WS2 findings in a fresh session (`memory-safety-auditor`, `semantics-reviewer`)
- [ ] 9.2.3 Verify WS3 and WS4 findings in a fresh session (`emission-integrity-auditor`, `performance-auditor`)
- [ ] 9.2.4 Verify WS5 findings in a fresh session (`amiga-platform-reviewer`, `memory-safety-auditor` for the host shim)
- [ ] 9.2.5 Verify WS7 and WS8 findings in a fresh session (`performance-auditor`, `correctness-reviewer`)

### Step 9.3: Ledger and gate

- [ ] 9.3.1 Build `05-findings.md` per 03-04: ledger, merges, refuted, spec questions, escalations
- [ ] 9.3.2 Phase gate: ST-1..ST-9, ST-11 across all workstream files and `05-findings.md`

**Test slices**: N/A — full verify only
**Verify (gate)**: ST-1..ST-9, ST-11 (AR #20)

---

## Phase 10: Synthesis and report

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — synthesis of verified evidence into the report (AR #19)

### Step 10.1: Report specification

**Reference**: 03-04 §Report structure, §DEF rows, §Handoff · AR #8, AR #13
**Objective**: The report skeleton and the ST-10 checklist fixed before writing.

- [ ] 10.1.1 Create `10-assessment-report.md` with the ten sections of 03-04 and an ST-10 checklist at the end

### Step 10.2: Writing

- [ ] 10.2.1 Write sections 1–7 (summary, scope and limits, scorecard, findings, themes, performance, quick wins) from `05-findings.md` and the workstream files
- [ ] 10.2.2 Write sections 8–10 (remediation roadmap, spec questions, appendix)
- [ ] 10.2.3 Add one DEF row per verified Critical/Major finding to `plans/00-roadmap.md` per 03-04 §DEF rows; write the DEF IDs back into `05-findings.md` and the findings; update this plan's roadmap row through the roadmap skill

### Step 10.3: Review and gate

- [ ] 10.3.1 Independent `correctness-reviewer` review of the report against `05-findings.md` and ST-10/ST-12; fix every critical/major review finding
- [ ] 10.3.2 Phase gate: ST-1..ST-16; present the report to the user for the remediation-plan selection (03-04 §Handoff)

**Test slices**: N/A — full verify only
**Verify (gate)**: ST-1..ST-16 (AR #20)

---

## Dependencies

```
Phase 0
   ↓
Phases 1, 2, 3, 4, 5, 6, 7   (independent of each other; 4 and 7 use the Phase 0 C references)
   ↓
Phase 8
   ↓
Phase 9
   ↓
Phase 10
```

---

## Success Criteria

**The assessment is complete when:**

1. ✅ All phases completed
2. ✅ Every phase gate passed its ST cases (AR #20)
3. ✅ Every checklist item has an outcome (ST-8)
4. ✅ Every finding has an independent verdict (ST-9)
5. ✅ `10-assessment-report.md` delivered and reviewed (ST-10)
6. ✅ DEF rows added for every verified Critical/Major finding (AR #13)
7. ✅ No file outside the allowed paths changed (ST-7)
8. ✅ The user has the remediation roadmap to choose from
