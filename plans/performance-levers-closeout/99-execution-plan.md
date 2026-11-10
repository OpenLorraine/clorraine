# Execution Plan: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-10 15:46
> **Progress**: 44/44 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Implement the per-profile release flags, update the frozen compile matrix's
two GCC release rows, align the benchmark harness, add the `flag_audit.py`
gate, close the release Workbench and 1.3 corpus gaps, and prove the no-
regression guard with a `f9d5be8` base.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Baseline and specification tests (red) | 9 |
| 2 | Release flag implementation (green) | 8 |
| 3 | Harness, audit, and release-path gates | 12 |
| 4 | End-to-end acceptance and reviews | 10 |
| 5 | Documentation, issues, and roadmap closeout | 5 |

**Total: 44 tasks across 5 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the single source of truth for progress.
> Mark a task `[~]` with the implementation timestamp, promote it to `[x]`
> with the completion timestamp after its verify passes, and update the
> Progress header after every task. A blocker is `[!]` with a short reason.
> Timestamps come from `date '+%Y-%m-%d %H:%M'`; never invent one.

---

## Phase 1: Baseline and Specification Tests (Red)

> **Phase baseline tree**: recorded by the exec-plan skill at phase start.
> **Lenses**: compiler and language; data and migration (the frozen matrix is
> a versioned contract).
> **Reasoning**: high — the base identity, the spec ruling, and the red
> tests determine every later measurement.

### Step 1.1: Preserve the baseline and write the red tests

**Reference**: [03-01](03-01-release-flags.md) · [03-02](03-02-measurement-and-gates.md) · ST-1–ST-14, ST-21–ST-22 · AR-03/AR-04/AR-19/AR-23

**Objective**: capture the pre-change evidence and prove every new test fails
before the implementation.

- [x] 1.1.1 Record the clean head, the `clc` SHA-256, and preserve the current
  `build/perf/{base,head}.json` as historical evidence before anything is
  overwritten — `build/perf/` ✅ (completed: 2026-11-10 10:46)
- [x] 1.1.2 Land the behavior-neutral test seams: the four release constants
  with the current `-O2` values, `Tool.release`, the pure
  `host_release_flags(bool)`, the two `build.rs` test-double fields, and a
  `flag_audit` module skeleton with documented function signatures — no
  shipped behavior changes yet ✅ (completed: 2026-11-10 10:48)
- [x] 1.1.3 [spec-author] Write the Rust command-vector tests for ST-1–ST-7
  (release/debug per profile, both host predicate outcomes, both freestanding
  host-run branches) — `compiler/src/clorraine/toolchain.rs` ✅ (completed: 2026-11-10 10:49)
- [x] 1.1.4 [spec-author] Write the harness tests for ST-9–ST-11 —
  `tools/perf/tests/test_bench.py` ✅ (completed: 2026-11-10 10:49)
- [x] 1.1.5 [spec-author] Write the audit floor/repeatability tests for
  ST-12–ST-14 — `tools/perf/tests/test_flag_audit.py` ✅ (completed: 2026-11-10 10:49)
- [x] 1.1.6 [spec-author] Write the spec-row and CLI help tests for ST-8 and
  ST-22 — `compiler/tests/clorraine_driver_spec_test.rs` ✅ (completed: 2026-11-10 10:49)
- [x] 1.1.7 Run the focused red set and record the exact failures; vector tests
  must fail on the expected value, and the new audit module is expected to
  fail at its skeleton boundary — `build/perf/` ✅ (completed: 2026-11-10 10:49)
- [x] 1.1.8 Capture the old-flags probe matrix, the rejected candidates
  (`-O3`, `-Os`, frame-pointer, LTO, LTO+GC, host unwind), the freestanding
  GC probe (208 → 232 bytes), and the constructor/section audit (removed
  `__amigapath.o`/`environ.o` entries; remaining list; `_start` loop and a
  remaining constructor in the `vamos` trace) — `build/perf/` and
  `05-findings.md` ✅ (completed: 2026-11-10 10:56)
- [x] 1.1.9 Write the baseline section of the findings: old flags, old sizes,
  old instructions, constructor evidence, and the base identity — `05-findings.md` ✅ (completed: 2026-11-10 10:56)

**Deliverables**:
- [ ] All red tests fail for the expected reason, not a compile/syntax error.
- [ ] The baseline evidence is recorded and the old JSON is preserved.
- [ ] The constructor audit names the removed objects and entries.

**Verify**: `cargo test -p clc --lib toolchain::tests`,
`cargo test -p clc --test clorraine_driver_spec_test`, and `make bench-test`
show the expected red failures; the baseline files exist.

---

## Phase 2: Release Flag Implementation (Green)

> **Phase baseline tree**: recorded by the exec-plan skill at phase start.
> **Lenses**: compiler and language; data and migration.
> **Reasoning**: high — this changes the shipped build and the frozen matrix.

### Step 2.1: Per-profile release flags

**Reference**: [03-01](03-01-release-flags.md) · ST-1–ST-8 · AR-04/AR-06/AR-07/AR-12/AR-19

**Objective**: make the release command match the requirements and the spec.

- [x] 2.1.1 Fill the four release slices, `Tool.release`, and the pure
  `host_release_flags(gnu_linker)` / `host_is_linux()` seam; update the two
  `build.rs` test-double fields — `compiler/src/clorraine/toolchain.rs`,
  `compiler/src/clorraine/build.rs` ✅ (completed: 2026-11-10 11:02)
- [x] 2.1.2 Make `command()` use `tool.release`; give the freestanding m68k and
  host-run tools `FREESTANDING_RELEASE_FLAGS`; remove the global
  `RELEASE_FLAGS` — `compiler/src/clorraine/toolchain.rs` ✅ (completed: 2026-11-10 11:02)
- [x] 2.1.3 Update the module/`Tool` docs to the new matrix —
  `compiler/src/clorraine/toolchain.rs` ✅ (completed: 2026-11-10 11:02)
- [x] 2.1.4 Update only the two GCC release rows in the compile matrix and add
  the ST-8 spec-row pin — `spec/17-codegen.md` §13,
  `compiler/tests/clorraine_driver_spec_test.rs` (AR-19) ✅ (completed: 2026-11-10 11:02)
- [x] 2.1.5 Update the `--release` help and `Options.release` doc with the
  freestanding/non-Linux exclusion — `compiler/src/clorraine/cli.rs` ✅ (completed: 2026-11-10 11:02)
- [x] 2.1.6 Run the focused Rust and driver tests for ST-1–ST-8 and ST-22 and
  make them green — `compiler/src/clorraine/toolchain.rs`,
  `compiler/tests/clorraine_driver_spec_test.rs` ✅ (completed: 2026-11-10 11:02)
- [x] 2.1.7 Record the exact vectors, the spec diff, and the pre-existing
  freestanding row wording discrepancy in the findings — `05-findings.md` ✅ (completed: 2026-11-10 11:02)
- [x] 2.1.8 Dispatch the end-of-phase correctness and emission-integrity
  reviews (blast radius and test integrity included) and fix every
  critical/major finding — review packets, `05-findings.md` ✅ (completed: 2026-11-10 11:02)

**Deliverables**:
- [x] Every command vector matches 03-01 exactly.
- [x] Debug commands are byte-identical to the old lines.
- [x] Freestanding m68k and host-run release stay `-O2`.
- [x] Only the two authorized spec rows changed.

**Verify**: `cargo test -p clc --lib toolchain::tests`,
`cargo test -p clc --test clorraine_driver_spec_test`, and
`git diff --check`.

---

## Phase 3: Harness, Audit, and Release-Path Gates

> **Phase baseline tree**: recorded by the exec-plan skill at phase start.
> **Lenses**: compiler and language; data and migration.
> **Reasoning**: high — the measurement and gate wiring are the proof.

### Step 3.1: Harness and audit

**Reference**: [03-02](03-02-measurement-and-gates.md) · ST-9–ST-17 · AR-08/AR-22/AR-23/AR-25

**Objective**: make the harness describe the shipped build and add the
repeatable floor gate.

- [x] 3.1.1 Update `HOST_RELEASE`/`M68K_RELEASE`, add `binary_bytes`, and
  record `m68k_release_bytes` for scored programs —
  `tools/perf/bench.py` ✅ (completed: 2026-11-10 11:30)
- [x] 3.1.2 Fill the audit skeleton: CLI (`run`, `--repeat`, `--check`,
  `--out`), the `"<profile>/<program>"` key, `clc`/`toolchain` identity, and
  the old/GC/new variant model — `tools/perf/flag_audit.py` ✅ (completed: 2026-11-10 11:30)
- [x] 3.1.3 Add the emit/build/run stage: private per-program run directory,
  corpus expected-output reader, host/`vamos` execution, section sizes, and
  instruction traces — `tools/perf/flag_audit.py` ✅ (completed: 2026-11-10 11:30)
- [x] 3.1.4 Add the exact-integer floor checks: every group, every miss,
  freestanding equality, JSON written before exit 1 — `tools/perf/flag_audit.py` ✅ (completed: 2026-11-10 11:30)
- [x] 3.1.5 Add the two-run repeat comparator, the `bench-flags` target,
  `.PHONY`, and one `AGENTS.md` command-table row — `tools/perf/flag_audit.py`,
  `Makefile`, `AGENTS.md` ✅ (completed: 2026-11-10 11:30)
- [x] 3.1.6 Make the harness/audit tests green (ST-9–ST-14) and add the
  malformed-matrix/missing-tool impl cases —
  `tools/perf/tests/test_bench.py`, `tools/perf/tests/test_flag_audit.py` ✅ (completed: 2026-11-10 11:30)
- [x] 3.1.7 Add the shipped flags to the warning-bearing nix20 shape line and
  the section flags to the nix13 compile-only line —
  `docker/scripts/bench-shape.sh` (ST-18) ✅ (completed: 2026-11-10 11:30)
- [x] 3.1.8 Keep the debug Workbench/services build, `mkdir -p build/amiga-sdk`,
  copy the debug binaries to `build/amiga-sdk/*-debug`, and add the release
  build to the default paths — `Makefile` `sdk-advanced-build` (ST-20) ✅ (completed: 2026-11-10 11:30)
- [x] 3.1.9 Update the benchmark README, known-limitations, and the
  production-readiness register entry — `benchmarks/README.md`,
  `docs/known-limitations.md`,
  `plans/v1-production-readiness/00-ambiguity-register.md` ✅ (completed: 2026-11-10 11:30)
- [x] 3.1.10 Run the real audit twice, assert every R3 floor plus the host
  `.symtab` absence, and record the full matrix, the 1.3 outputs, the
  freestanding GC probe, and the constructor trace proof —
  `build/perf/flags.json`, `05-findings.md` (ST-15–ST-17, ST-21) ✅ (completed: 2026-11-10 11:30)
- [x] 3.1.11 Run the release Workbench/services gates and record the report
  lines and goldens — `build/amiga-sdk/`, `05-findings.md` (ST-20) ✅ (completed: 2026-11-10 11:30)
- [x] 3.1.12 Dispatch the end-of-phase correctness review, emission-integrity
  review, and design challenge; fix every critical/major finding before the
  scored measurement — review packets, `05-findings.md` ✅ (completed: 2026-11-10 13:50)

**Deliverables**:
- [x] `make bench-flags` passes twice identically.
- [x] `make bench-shape` is warning-free.
- [x] The release Workbench/services gates pass.
- [x] The 1.3 corpus outputs match.

**Verify**: `make bench-flags`, `make bench-shape`, `make sdk-a600-services`,
`make sdk-a1200-workbench`.

---

## Phase 4: End-to-End Acceptance and Reviews

> **Phase baseline tree**: recorded by the exec-plan skill at phase start.
> **Lenses**: compiler and language; data and migration.
> **Reasoning**: max — this is the acceptance proof for a shipped build change.

### Step 4.1: Scored comparison and full gate

**Reference**: [03-02](03-02-measurement-and-gates.md) · ST-18–ST-21 · AR-03/AR-08/AR-10/AR-21

**Objective**: prove no regression and no safety loss on the final head.

- [x] 4.1.1 Regenerate `base.json` from `f9d5be8` with the updated harness and
  assert the full compiler hash and the harness-tree `dirty == false` —
  `build/perf/base.json` ✅ (completed: 2026-11-10 14:52)
- [x] 4.1.2 Run `make bench` and `make bench-compare`; record the table and
  the base/head identity — `build/perf/head.json`, `05-findings.md` ✅ (completed: 2026-11-10 14:52)
- [x] 4.1.3 Run `make check` and record the result — gate log ✅ (completed: 2026-11-10 14:52)
- [x] 4.1.4 Run `make asan` and `make ubsan` — gate logs ✅ (completed: 2026-11-10 14:52)
- [x] 4.1.5 Run `make corpus-amiga` and `make differential` — gate logs ✅ (completed: 2026-11-10 14:52)
- [x] 4.1.6 Run `make verify` and record `verify: all legs passed` — gate log ✅ (completed: 2026-11-10 14:52)
- [x] 4.1.7 Dispatch the final phase reviews: correctness reviewer,
  emission-integrity auditor (base/head emitted-C diff and strict C build),
  and a design challenge on the measurement/attribution design — review
  packets ✅ (completed: 2026-11-10 14:55)
- [x] 4.1.8 Fix every critical/major finding, rerun the affected gates, and
  record the rulings — `05-findings.md` ✅ (completed: 2026-11-10 14:55)
- [x] 4.1.9 Re-run the audit and the full gate after any fix; a missed floor
  stops the landing — gate logs ✅ (completed: 2026-11-10 14:55)
- [x] 4.1.10 Add `make bench-flags` to the findings' gate evidence and to the
  final gate record; keep it out of the host-only CI — `05-findings.md`,
  `AGENTS.md` ✅ (completed: 2026-11-10 14:55)

**Deliverables**:
- [x] `bench-compare` passes with equal output and no >1% regression.
- [x] The audit floors pass with the exact measured values recorded.
- [x] The full gate is green.
- [x] No open critical/major review finding.

**Verify**: `make verify` plus the gate list in R4.7.

---

## Phase 5: Documentation, Issues, and Roadmap Closeout

> **Phase baseline tree**: recorded by the exec-plan skill at phase start.
> **Lenses**: compiler and language; data and migration.
> **Reasoning**: medium — bounded closeout.

### Step 5.1: Close the record

**Reference**: [00-index](00-index.md) · AR-14/AR-15/AR-18/AR-21

**Objective**: leave a complete, honest record and the next options.

- [x] 5.1.1 Complete `05-findings.md`: audit matrix, constructor audit, exact
  values, gate evidence, review rulings, accepted limits, and the two
  comparisons — `05-findings.md`
- [x] 5.1.2 Register the four deferred issues (LTO/LTO+GC, opt-in
  `-Os`/`--size`, macOS `-dead_strip`, freestanding strip/GC) with the
  `performance`, `deferred`, and `enhancement` labels — GitHub issues
- [x] 5.1.3 Mark PLAN-26 Done on the roadmap and update the progress counter
  to 570/572, 0 open, 2 parked — `plans/00-roadmap.md` ✅ (completed: 2026-11-10 15:00)
- [x] 5.1.4 Commit the final change with the git-commit skill, then push in
  push mode; confirm `main` equals `origin/main` and the tree is clean ✅ (completed: 2026-11-10 15:46)
- [x] 5.1.5 Remove the probe scratch under `build/perf/probe/` and any
  temporary artifacts; keep only the evidence the findings cite ✅ (completed: 2026-11-10 15:46)

**Deliverables**:
- [ ] The findings are complete and reproducible.
- [ ] Four issues exist without duplicates of #13–#19.
- [ ] The roadmap is Done and the branch is pushed.
- [ ] The workspace is clean.

**Verify**: `git status -sb`, `gh issue list`, and the final `make verify`
result.

---

## Contingency

- **A missed floor stops the landing.** It is escalated, never lowered
  (AR-21). The audit writes `flags.json` and the failing table before it
  exits 1.
- **A flaky gate may be rerun once.** Record the failing leg, its output, and
  the rerun; a second failure stops (AR-21).
- **A required constructor or safety helper that disappears stops the
  landing.** Add a targeted `KEEP`/`-u`, re-run the audit and every floor, and
  record the change (AR-13).
- **The base pin does not move.** `f9d5be8` stays authoritative even when the
  branch head advances; `bench-base` fails loudly if the commit is
  unreachable. The `dirty` flag proves the harness tree was clean, not the
  base commit's provenance; the full `compiler.git` hash is the base identity.
- **Phase transitions.** The plan preflight sets `Plan Preflighted`; the
  exec-plan skill sets `Executing` at Phase 1 and `Done` at 5.1.3.

---

## Dependencies

```
Phase 1
    ↓
Phase 2
    ↓
Phase 3
    ↓
Phase 4
    ↓
Phase 5
```

The audit (3.1.10) depends on phases 2 and 3.1.1–3.1.9. The scored
comparison (4.1.1–4.1.2) depends on 3.1.1. The release Workbench build
(3.1.8) and gate (3.1.11) depend on the flag change (phase 2).

## Success Criteria

**Feature is complete when:**

1. ✅ All 44 tasks complete.
2. ✅ The spec has exactly the two authorized release-row changes.
3. ✅ `make bench-flags` passes twice identically and the exact values
   are recorded.
4. ✅ `make bench-compare` shows no output, instruction, or `.text` regression.
5. ✅ `make check`, `make asan`, `make ubsan`, `make corpus-amiga`,
   `make differential`, `make bench-shape`, `make bench-base`, `make bench`,
   `make bench-compare`, `make bench-flags`, and `make verify` are green.
6. ✅ The release Workbench/services gates and the 1.3 release corpus pass.
7. ✅ No critical/major review finding remains.
8. ✅ The four issues are registered, the roadmap is Done, and the branch is
   pushed.
