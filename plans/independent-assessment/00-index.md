# Independent Assessment of Clorraine

> **Feature**: A deep, strict, evidence-based assessment of the compiler, C codegen, optimizer question, runtime shims, standard library, and performance, delivered as one report that feeds later remediation plans
> **Status**: Planning Complete — preflighted; execution starts with the user's confirmation of a settled tree (AR #7)
> **Created**: 2026-11-25
> **Implements**: — (standalone plan; no RD)
> **CodeOps Artifact Schema**: 1

## Overview

Clorraine has shipped v1.2.0 after roughly a hundred plans and more than five hundred
recorded defects. Each of those plans fixed what it was scoped to fix; none looked at the
compiler as a whole with fresh eyes. This plan does that: an independent, read-only audit
that asks whether the compiler is correct, whether the C it emits is sound and efficient on
both the A500 (68000, OCS, Kickstart 1.3) and the A1200 (68020, AGA), whether the runtime
shims are safe on every Kickstart version, whether the standard library is correct and
cheap, and where the measurable performance headroom is.

The assessment changes no compiler, runtime, standard-library, or spec source (AR #12). Its
outputs are evidence: committed probe sources, per-workstream findings, an adversarially
reviewed findings ledger, and a final report with a ranked remediation roadmap. Surviving
critical and major findings enter the roadmap's DEF ledger at synthesis (AR #13, AR #21). The user
then selects remediation plans, each of which goes through `make-plan` and `exec-plan`.

The compiler has no optimizer of its own: the IR carries no optimization pass and constant
folding is limited to `const` evaluation. Optimization is delegated to GCC through the
driver's release flags. The optimizer question is therefore answered by measurement: what
in the emitted C blocks GCC, and which IR-level passes would pay for themselves (AR #17).

## Minimum-Sufficient Baseline

**Original goal:** an independent analysis of Clorraine as a whole — compiler, codegen, the
optimizer question, shims, libraries, performance — delivered as a detailed, strict, full
report that the user then uses to enhance and fix Clorraine.
**Smallest viable design:** read the code against `spec/`, write small probe programs and
equivalent hand-written C references, measure with the existing bench harness (driven by one
probe script, AR #23), sanitizers, `vamos`, and
FS-UAE, record findings in Markdown, and verify each finding with an independent session. No
new harness, tool, or compiler prototype.
**Excluded machinery:** a host mock harness for the Amiga shims; a prototype IR optimizer on a
scratch branch (AR #17); per-phase exec-plan review loops (AR #16); automated finding
validators. Each may appear only as a recommendation in the report.
**Approved complexity:** None.

## Planning Coverage

> **Coverage**: 4/4 clusters · 12/12 categories · 11 candidates · 2 residual limits · Effort: high/xhigh→inline (C4 requested xhigh; run inline in the lead session) · Independence: uniform-effort (inline fallback)
> **Freshness**: Coverage as of 2026-11-25; artifacts AGENTS.md, plans/00-roadmap.md, plans/real-workload-audit/99-execution-plan.md, compiler/src/clorraine/toolchain.rs, compiler/runtime/, stdlib/, compiler/tests/

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 Product and scope | Scope boundary, deliverables, naming | User answers, AGENTS.md layout, roadmap conventions | — | CV-001 (AR #8), CV-002 (AR #9), CV-003 (AR #18) | inline fallback | medium |
| C2 Behavior and edges | Mid-assessment defects, roadmap integration, quality loop | Strict quality profile in AGENTS.md, DEF ledger format | — | CV-004 (AR #12), CV-005 (AR #13), CV-006 (AR #16) | inline fallback | medium |
| C3 Technical and data | Spec-first adaptation, probe storage, measurement methods | spec-first protocol, bench harness, toolchain flags | `vamos` 68020 CPU support not yet confirmed | CV-007 (AR #10), CV-008 (AR #11), CV-009 (AR #14), CV-010 (AR #17) | the `vamos` CPU option is verified in Phase 0 | medium |
| C4 Risk and stakeholders | Severity, thresholds, budget | User budget (AR #4), real-workload audit thresholds | No security surface (read-only assessment) | CV-011 (AR #15) | inline fallback | medium |

**Source note:** CV-001..CV-011 map to AR #8..#18 as listed in the register. No Discovery keys.

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Scope, deliverables, acceptance criteria |
| 02  | [Current State](02-current-state.md) | Facts about the code under assessment and prior audits |
| 03-01 | [Method and Evidence](03-01-method-and-evidence.md) | Ground rules, evidence standard, severity rubric, probe layout, finding template, verification protocol |
| 03-02 | [Compiler Workstreams](03-02-compiler-workstreams.md) | WS1–WS4 checklists: semantic core, ownership, codegen correctness, code quality and optimizer |
| 03-03 | [Runtime and Performance Workstreams](03-03-runtime-performance-workstreams.md) | WS5–WS8 checklists plus the defect-history analysis |
| 03-04 | [Report and Handoff](03-04-report-and-handoff.md) | Report structure, DEF rows, remediation roadmap |
| 07  | [Testing Strategy](07-testing-strategy.md) | ST cases: the acceptance contract for every deliverable |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |
| PF  | [Preflight Report](00-preflight-report.md) | Post-creation audit and its resolutions |

Deliverables created during execution (AR #8, AR #11):

| Path | Content |
|------|---------|
| `findings/WS0-baseline.md` | Baseline SHA, inventory, measurements |
| `findings/WS<n>-<slug>.md` | One findings file per workstream |
| `probes/WS0/` | `drift-check.sh`, `measure.py`, the equivalent C references and `calibration.c` |
| `probes/WS<n>/` | Committed probe sources (`.clo`, hand-written `.c`, small scripts) |
| `05-findings.md` | Merged ledger with Phase 9 verdicts, clean sample, escalations, and the report review |
| `10-assessment-report.md` | The final report |

## Quick Reference

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Scope | Compiler, codegen, optimizer question, all shims, stdlib, performance (AR #1, AR #9) |
| Target lenses | A500/KS1.3 and A1200 (AR #2) |
| Gate reruns | None; measurement runs only (AR #3, AR #14) |
| Source changes | None; criticals are escalated immediately (AR #12) |
| Effort | up to `xhigh`, never `max` (AR #4, AR #19) |
| Review | Adversarial verification phase is the independent review; it replaces exec-plan's post-phase quality step (AR #16, AR #22) |
| Order and commits | Sequential in document order; `--ask-commit`, local commits after Step N.1 and at each gate (PF-004, PF-006) |
| Measurement | Phase 0 owns the matrix via `probes/WS0/measure.py`; equivalent C references with their own calibration (AR #23, PF-008, PF-009) |
| Roadmap | Surviving Critical/Major findings become DEF rows after the report review (AR #13, AR #21) |

## Specialist Agents

**None** — the capability-gap check found no gap a standing specialist would close.

_Detection evidence: the project's agents under `.opencode/agents/` are edit-denied phase-diff
reviewers, so the executing session authors every probe and finding, and Phase 9 uses fresh
read-only `general` sessions with the matching briefs (`semantics-reviewer`,
`correctness-reviewer`, `memory-safety-auditor`, `emission-integrity-auditor`,
`codeops/specialists/amiga-platform-reviewer.md`) as checklists (03-01 §Agent routing,
PF-011). The briefs carry the domain knowledge; a new role would only restate them._

## Related Files

Created: everything under `plans/independent-assessment/`. Modified: `plans/00-roadmap.md` —
this plan's row in any phase, finding-derived DEF rows in Phase 10 only (03-01 §Ground rules,
AR #13). No other file is modified.
