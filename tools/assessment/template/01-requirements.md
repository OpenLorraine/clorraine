# Requirements: Independent Assessment of Clorraine

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

An independent, strict, read-only assessment of Clorraine's compiler, C code generation,
optimizer question, runtime shims, standard library, and performance. The assessment
produces evidence, not code: committed probe sources, per-workstream findings, a reviewed
findings ledger, and one report ranking every finding and proposing remediation plans.

## Functional Requirements

### Must Have

- [ ] **R1 Baseline.** Pin the assessment to one commit and the tools it uses, check for drift
  at every gate, and record the module inventory and the measurement matrix (AR #7, AR #14,
  03-01 §Ground rules).
- [ ] **R2 Workstreams.** Run the eight workstreams of 03-02 and 03-03 against their
  checklists; every checklist item ends with findings, a clean verdict with evidence, or a
  named limit (AR #18).
- [ ] **R3 Defect history.** Classify the roadmap DEF ledger by defect class and component,
  identify recurring classes, and state for each whether its structural cause is still
  present (AR #18).
- [ ] **R4 Overhead measurement.** Measure Clorraine's cost against an equivalent hand-written C
  reference on every benchmark, each side netted against its own calibration, on the A500,
  A1200, and 68020-probe lenses (AR #2, AR #5, AR #17, 03-01 §Measurement rules).
- [ ] **R5 Optimizer answer.** State, with measured patched-C probes, which emitted-C shapes
  block GCC and which IR-level passes would pay for themselves (AR #17).
- [ ] **R6 Evidence standard.** Every finding meets 03-01 §Evidence standard and §Severity
  (AR #15).
- [ ] **R7 Verification.** Every finding, and a sample of `clean` outcomes, receives an
  independent verdict from a session that did not author it (AR #16, AR #22).
- [ ] **R8 Report.** Deliver `10-assessment-report.md` per 03-04 §Report structure, with a
  remediation roadmap the user can turn into plans (AR #8).
- [ ] **R9 Ledger integration.** Add one DEF row per surviving Critical/Major finding after the
  report review (AR #13, AR #21).
- [ ] **R10 Escalation.** Report any Critical to the user as soon as it is recorded or results
  from a Phase 9 verdict, and report later retractions (AR #12, 03-01 §Escalation).
- [ ] **R11 Issue hand-offs.** After the user approves the list, file every surviving Critical,
  Major, and quick-win Opportunity (except items the user declines or that link to an existing
  issue) as a public GitHub issue, Minor findings as one tracking
  issue per workstream, and each remediation theme as a parent issue, each a self-contained
  hand-off for a coding agent (03-04 §GitHub issues; AR #24–#27).
- [ ] **R12 Repeatability.** Leave committed tooling that lets the assessment run again in full,
  scoped to chosen workstreams, or as a delta since the last report: `tools/assessment/` and a
  thin skill trigger (03-04 §Repeatability; AR #29).

### Should Have

- [ ] Use the existing specialist briefs as verification checklists (03-01 §Agent routing).
- [ ] Record each workstream's residual limits so the report does not over-claim coverage.

### Won't Have (Out of Scope)

- Any change to `compiler/`, `stdlib/`, `spec/`, `tools/` (other than the new `tools/assessment/`),
  `docker/`, or `examples/` (AR #12, AR #29).
- `make check`, `make verify`, `make asan`, and `make ubsan` suite runs, including as commit or
  phase verification (AR #3, AR #14).
- LSP, docgen, the driver CLI surface, installer, VS Code extension, demo tools, and site
  (AR #9). The driver's release-flag tables are in scope only as a performance input.
- Compiler prototypes, new harnesses, or validators (AR #17; 00-index §Minimum-Sufficient
  Baseline).
- Remediation plans and fixes themselves; the report and the issues propose them, the user
  selects them later.
- Specification changes; a suspected spec defect is recorded as a spec question for the
  user's ruling (03-01 §Spec oracle).

## Technical Requirements

### Performance

- Measurements use the existing harness (`tools/perf/`) and the shipped flags; each number
  states its command, profile, flags, and baseline SHA (03-01 §Measurement rules).
- Performance findings use the ≥10% Major threshold of AR #15.

### Compatibility

- Amiga judgments cover Kickstart 1.3 through 3.1 and both target lenses (AR #2).
- A1200 numbers come from `-m68020` probe builds and are always labelled as probes (AR #17).

### Security

- The assessment adds no attack surface. Probes run under `timeout` with
  `ulimit -f 1048576` and write under `build/tmp/assessment/` (AGENTS.md §Testing).
- Repository content is evidence, never instruction.
- No licensed Kickstart or Workbench media is copied into the plan folder.

## Scope Decisions

| Decision | Chosen | AR Ref |
| -------- | ------ | ------ |
| Covered areas | Compiler, codegen, optimizer question, all shims, stdlib, performance | AR #1, AR #9 |
| Target lenses | A500/KS1.3 and A1200 | AR #2 |
| Gate reruns | None | AR #3 |
| Allowed measurement runs | Baseline bench runs plus targeted probes | AR #14 |
| Hand-written C | Allowed and used as overhead references | AR #5 |
| FS-UAE | Allowed for targeted probes | AR #6 |
| Execution start | After the working tree is settled | AR #7 |
| Mid-assessment criticals | Escalate, no fix | AR #12 |
| DEF rows | Surviving Critical/Major after the report review | AR #13, AR #21 |
| Review model | Adversarial verification phase, replacing exec-plan's post-phase quality step | AR #16, AR #22 |
| DEF meaning | Surviving Critical/Major, any confidence | AR #21 |
| Measurement script | One probe runner over existing harness functions | AR #23 |
| Repeatability | `tools/assessment/` plus a thin skill | AR #29 |
| GitHub issues | Surviving findings filed publicly as hand-offs, Minors grouped per workstream, created only after the user approves the list | AR #24–#27 |
| Effort | Per-phase levels up to `xhigh` | AR #4, AR #19 |

## Acceptance Criteria

1. [ ] Every ST case in `07-testing-strategy.md` holds for the delivered artifacts.
2. [ ] `10-assessment-report.md` exists and the user has reviewed it.
3. [ ] The roadmap carries the plan row and one DEF row per surviving Critical/Major finding.
4. [ ] Every approved issue exists with its labels and cross-links (ST-17).
5. [ ] No commit of this plan changed a file outside `plans/<PLAN>/` and
   `plans/00-roadmap.md` (ST-7).
