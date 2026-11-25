# Testing Strategy: Independent Assessment of Clorraine

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The assessment changes no code, so there is no test suite to extend. The "tests" are the
acceptance contract for the deliverables (AR #10): every ST case below is checked against the
artifacts at each phase gate and again in Phase 10. Probes are the assessment's evidence and
follow 03-01 §Falsifiability.

### Coverage Goals

Code-coverage targets do not apply (no code is written). The coverage goal is checklist
coverage: every item in 03-02 and 03-03 has an outcome (ST-8).

## Test Slices & Groups (optional)

N/A — full verify only. No test map exists, and the phase gate is the ST checks of AR #20.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived from `01-requirements.md`, 03-01, 03-04, and the Ambiguity Register. They are the
> immutable acceptance contract of the deliverables: a deliverable that violates one is wrong,
> not the case.

### Findings and evidence

| #    | Input / Scenario | Expected Output / Behavior | Source |
|------|------------------|----------------------------|--------|
| ST-1 | Any finding in any `findings/WS<n>-*.md` | Has every field of 03-01 §Finding template; ID matches `ASM-WS<n>-<NNN>`; severity is one of the four levels; effort is S/M/L/XL | R6, AR #15 |
| ST-2 | A finding with confidence `verified` | Its evidence names a committed probe under `probes/WS<n>/` (or a measurement record) and a command that reproduces the stated observation at the baseline SHA | R6, AR #11 |
| ST-3 | A finding with confidence `hypothesis` | Severity is Minor or Opportunity, never Major or Critical | AR #15 |
| ST-4 | A performance finding rated Major | Records both numbers of the comparison, the same profile and flags on both sides, and a delta ≥10% in net m68k instructions or `.text` | AR #15, 03-01 §Measurement rules |
| ST-5 | Any probe | Its `.expected` file (or expected block) exists and states the source of the expectation (spec chapter §, C reference, or benchmark golden); the falsifiability log has one line for it showing it can fail | AR #10, 03-01 §Falsifiability |
| ST-6 | A disagreement between code and `spec/` | Filed as a code finding citing the spec chapter and section; a suspected spec defect is filed under `## Spec questions` instead; no file under `spec/` differs from the baseline | 03-01 §Spec oracle |
| ST-7 | The working tree at any phase end | No tracked or untracked change outside `plans/independent-assessment/`, `build/`, and (Phase 10 only) `plans/00-roadmap.md` | AR #12, AR #20 |
| ST-8 | Any workstream file at its phase gate | Every checklist ID of its workstream (03-02, 03-03) appears under `## Checklist outcomes` with finding IDs, `clean` plus evidence, or `limit` plus reason | R2, AR #18 |

### Verification and report

| #     | Input / Scenario | Expected Output / Behavior | Source |
|-------|------------------|----------------------------|--------|
| ST-9  | Every finding after Phase 9 | Has exactly one verdict (`confirmed`, `refuted`, `downgraded`, `upgraded`, `merged into`) recorded by a session other than the author's, with a reason | R7, AR #16 |
| ST-10 | `10-assessment-report.md` | Contains the ten sections of 03-04 §Report structure; its ranked findings are exactly the non-refuted, non-merged ledger rows; every Critical/Major has a DEF row in `plans/00-roadmap.md` and every DEF row added by this plan maps to a Critical/Major finding | R8, R9, AR #13 |
| ST-11 | A `verified` Critical finding | Appears under `## Escalations` with a timestamp, and the user was told in the session at that time | R10, AR #12 |
| ST-12 | Any number in a finding or the report | Cites a measurement record giving command, profile, flags, baseline SHA, and raw output path | 03-01 §Measurement rules |

### Measurement probes

| #     | Input / Scenario | Expected Output / Behavior | Source |
|-------|------------------|----------------------------|--------|
| ST-13 | Each hand-written C reference in `probes/WS0/ref/` | Prints output byte-identical to the benchmark's `expected-output.txt` on host and on hosted-amiga under `vamos` | R4, AR #5 |
| ST-14 | Each optimizer estimate (patched emitted C) | Program output byte-identical to the unpatched build; the patch is committed as a diff; the gain is reported against the unpatched build with the same flags | R5, AR #17 |
| ST-15 | Each A1200 number | Labelled "68020 probe build", with the CPU option used; never presented as a shipped-profile number | AR #2, AR #17 |
| ST-16 | Every `> **Reasoning**:` line and every agent dispatch | Level is `medium`, `high`, or `xhigh` per AR #19; never `max` | AR #4, AR #19 |

## Test Categories

### Specification Tests (from ST-cases above)

No test files are written. The ST cases are checked by the phase-gate task of each phase
(manual inspection against this table) and by the Phase 10 report review.

| Check point | ST Cases Covered |
|-------------|------------------|
| Phase 0 gate | ST-5, ST-7, ST-12, ST-13, ST-15, ST-16 |
| Phases 1–8 gates | ST-1..ST-8, ST-11, ST-12, ST-14, ST-15 |
| Phase 9 gate | ST-1..ST-9, ST-11 |
| Phase 10 gate | ST-1..ST-16 |

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

The benchmarks, corpus, and examples at the baseline SHA; the Phase 0 C references; the
licensed Kickstart and Workbench media under `~/amiga` for FS-UAE probes.

### Mock Requirements

None. A host mock harness for the Amiga shims is excluded machinery (00-index).

## Verification Checklist

- [ ] All ST cases defined with concrete input/expected pairs
- [ ] Every ST case traces to a requirement, design doc, or AR entry
- [ ] Every probe records its expectation before it is run (ST-5)
- [ ] Every probe shown falsifiable (ST-5)
- [ ] Every finding verified by another session (ST-9)
- [ ] Phase gates pass their ST cases
- [ ] No change outside the allowed paths (ST-7)
