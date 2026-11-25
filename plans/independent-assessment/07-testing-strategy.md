# Testing Strategy: Independent Assessment of Clorraine

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The assessment changes no code, so there is no test suite to extend. The "tests" are the
acceptance contract for the deliverables (AR #10): each phase gate checks the ST cases in its
row of §Check points, and Phase 10 checks them all. A case with no matching input in a phase
(for example ST-14 when the phase made no patched-C probe) passes as `n/a`. Probes are the
assessment's evidence and follow 03-01 §Falsifiability.

### Coverage Goals

Code-coverage targets do not apply (no code is written). The coverage goal is checklist
coverage: every item in 03-02 and 03-03 has an outcome with evidence (ST-8).

## Test Slices & Groups (optional)

N/A — no test map exists. Per-task verification is the `Task verify` command in
`99-execution-plan.md`; the phase gate adds the phase's ST row (AR #20).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived from `01-requirements.md`, 03-01, 03-04, the Ambiguity Register, and the preflight
> report. They are the immutable acceptance contract of the deliverables: a deliverable that
> violates one is wrong, not the case.

### Findings and evidence

| #    | Input / Scenario | Expected Output / Behavior | Source |
|------|------------------|----------------------------|--------|
| ST-1 | Any finding in any `findings/WS<n>-*.md` | Has every field of 03-01 §Finding template, including `Author session`; ID matches `ASM-WS<n>-<NNN>`; severity is one of the four levels; effort is S/M/L/XL | R6, AR #15 |
| ST-2 | A finding with confidence `verified` | Its evidence names a committed probe under `probes/WS<n>/` (or a measurement record) and a command that reproduces the stated observation at HEAD with the drift check passing | R6, AR #11, 03-01 §Ground rules |
| ST-3 | A finding with confidence `hypothesis` | Severity is Minor or Opportunity | AR #15 |
| ST-4 | A performance finding rated Major | Records both numbers, the same lens and compiler on both sides, each side netted against its own calibration, the equivalent C reference as comparison, a shipped 68000 lens, and a delta ≥10% in net m68k instructions or `.text`; flag findings instead state both flag sets | AR #15, 03-01 §Measurement rules |
| ST-5 | Any probe | Its expectation exists, cites an accepted source (03-01 §Expectation sources), and was committed before the run (its commit precedes the commit that records the observation); the falsifiability log has one line naming the form used — contrasting input or seeded defect for probes behind a finding or `clean`, mutated expectation only for comparators | AR #10, 03-01 §Falsifiability |
| ST-6 | A disagreement between code and `spec/` | Filed as a code finding citing the spec chapter and section; a suspected spec defect is filed under `## Spec questions` instead; no file under `spec/` differs from the baseline | 03-01 §Spec oracle |
| ST-7 | The repository at any phase end | Task verify passes (drift check, 03-01 §Ground rules); every commit this plan made touches only `plans/independent-assessment/` and `plans/00-roadmap.md`, the latter only for this plan's row, exec-plan blocker rows, and — after Phase 10 starts — finding-derived DEF rows | AR #12, AR #20 |
| ST-8 | Any workstream file at its phase gate | Every checklist ID of its workstream appears under `## Checklist outcomes` with finding IDs, `clean` with probe IDs or a `file:line` site list, or `limit` with its reason; every coverage-table row is complete | R2, AR #18, 03-01 §Checklist outcomes |

### Verification and report

| #     | Input / Scenario | Expected Output / Behavior | Source |
|-------|------------------|----------------------------|--------|
| ST-9  | Every finding, and every sampled `clean`, after Phase 9 | Has exactly one verdict (`confirmed`, `refuted`, `downgraded`, `upgraded`, `merged into`) with a reason, issued by a verification session whose identifier differs from the finding's `Author session`; at least two `clean` outcomes per workstream carry a verdict | R7, AR #16, 03-01 §Verification protocol |
| ST-10 | `10-assessment-report.md` | Contains the ten sections of 03-04; its ranked findings are exactly the surviving ledger rows; every surviving Critical/Major has a DEF row in `plans/00-roadmap.md`, and every finding-derived DEF row added by this plan maps to one | R8, R9, AR #13, AR #21 |
| ST-11 | A Critical at confidence `verified` or `probable`, or a Phase 9 verdict resulting in a Critical | Appears under `## Escalations` with a timestamp and the exact message text; a later Phase 9 refutation or downgrade is recorded beside it with its own timestamp | R10, AR #12, 03-01 §Escalation |
| ST-12 | Any measured quantity (instructions, `.text`, time, RSS, footprint) in a finding or the report | Cites a measurement record giving command, lens, flags, CPU, baseline SHA, and raw output path | 03-01 §Measurement rules |

### Measurement probes

| #     | Input / Scenario | Expected Output / Behavior | Source |
|-------|------------------|----------------------------|--------|
| ST-13 | Each C reference and `calibration.c` in `probes/WS0/ref/` | Output identical after `bench.normalize_output` to the benchmark's `expected-output.txt` on host and on the A500 and A1200 shipped lenses under `vamos`; the reference meets the fairness rules of 03-01 §Measurement rules | R4, AR #5, PF-009 |
| ST-14 | Each optimizer estimate (patched emitted C) | Output identical after normalization to the unpatched build; the patch is committed as a diff; the gain is reported against the unpatched build with the same lens and flags | R5, AR #17 |
| ST-15 | Each 68020 number | Labelled "68020 probe build" with the CPU option used, never rated above Opportunity, never presented as a shipped-profile number | AR #2, AR #17, PF-009 |
| ST-16 | Every `> **Reasoning**:` line and every `## Dispatch record` line | Level is `medium`, `high`, or `xhigh` per AR #19; never `max` | AR #4, AR #19 |

## Check points

The single authority for which ST cases each phase gate checks (PF-001).

| Phase gate | ST cases |
|------------|----------|
| Phase 0 | ST-1..ST-3, ST-5, ST-7, ST-11, ST-12, ST-13, ST-15, ST-16 |
| Phases 1–8 | ST-1..ST-8, ST-11, ST-12, ST-14, ST-15, ST-16 |
| Phase 9 | ST-1..ST-9, ST-11, ST-16 |
| Phase 10 | ST-1..ST-16 |

## Test Categories

### Specification Tests (from ST-cases above)

No test files are written. The ST cases are checked by each phase's gate task (inspection of
the deliverables against this document) and by the Phase 10 report review.

### Implementation Tests (edge cases, internals)

N/A — no implementation.

### Integration Tests

N/A — no implementation.

### End-to-End Tests

| Scenario | Steps | Expected Result |
|----------|-------|-----------------|
| Report usable for remediation | User reads the report and picks a remediation entry | The entry names its findings, goal, dependencies, and effort well enough to start `make-plan` (03-04 §Report structure item 8) |

## Test Data

### Fixtures Needed

The benchmarks, corpus, and examples at the baseline; the Phase 0 C references and calibration
program; the licensed Kickstart and Workbench media under `~/amiga` for FS-UAE probes.

### Mock Requirements

None. A host mock harness for the Amiga shims is excluded machinery (00-index).

## Verification Checklist

- [ ] All ST cases defined with concrete input/expected pairs
- [ ] Every ST case traces to a requirement, design doc, or AR entry
- [ ] Every probe's expectation committed before its run (ST-5)
- [ ] Every probe behind a finding or `clean` shown to discriminate (ST-5)
- [ ] Every finding and the clean sample verified by another session (ST-9)
- [ ] Every phase gate passes its §Check points row
- [ ] No change outside the allowed paths, drift check passing (ST-7)
