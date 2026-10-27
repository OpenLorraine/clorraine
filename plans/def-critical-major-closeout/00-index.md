# Critical/Major DEF Closeout Implementation Plan

> **Feature**: Close every open critical and major DEF in the Clorraine roadmap
> **Status**: Complete — 34/34 active rows Done; the final review findings are fixed; PRE-1 and SR-002 are filed as `DEF-556`/`DEF-557`; the remaining pre-existing list is recorded for backlog triage
> **Created**: 2026-10-23 01:00
> **Defects**: DEF-543; DEF-454, DEF-455, DEF-456, DEF-457, DEF-458, DEF-460, DEF-461, DEF-462, DEF-464, DEF-465, DEF-466; DEF-470, DEF-471, DEF-472, DEF-473, DEF-475, DEF-476, DEF-477, DEF-478, DEF-479, DEF-480, DEF-481; DEF-411, DEF-412, DEF-413, DEF-415, DEF-505, DEF-506, DEF-507, DEF-513, DEF-522; DEF-167, DEF-171, DEF-416
> **CodeOps Artifact Schema**: 1

## Overview

The roadmap holds 136 open defects at the audit baseline HEAD `ef6fd95`:
**35 are critical or major** (1 critical + 34 major) and 101 are minor. They
block further feature work because they include use-after-free, double-free,
silently wrong stores, false rejections of safe programs, and accepted programs
whose emitted C does not build. `DEF-470` was revalidated at that HEAD and is
already fixed, so the active set is **34 rows: 1 critical and 33 major**. The
plan covers **35 DEF rows** including the stale `DEF-470` close. The working
tree already has `DEF-470` Done; the plan records the proof rather than
re-closing it.

This plan closes that active set in five risk-ordered batches across seven
execution phases (0–6). Every active defect gets a specification regression that
fails before the fix and passes after it (ST-12 is the stale green pin for
`DEF-470`), an implementation or ASAN/strict-build control where the class needs
one, and a Done roadmap row with the commit evidence. The plan reuses the existing
ownership, instance-audit, and conditional-emission machinery; it does not add a
general analysis framework.

## Minimum-Sufficient Baseline

**Original goal:** no open critical or major DEF remains in the plan's 34-row active set; the execution-filed rows and the pre-existing report-only findings are the user-scoped backlog.
**Smallest viable design:** a regression per defect plus a direct fix in the
stage that already owns the rule — ownership for moves and aliases, instance
audit for substitutions, codegen for materialization and evaluation order, and
parser/type tables for the strict-build rows.
**Excluded machinery:** no generalized callee-summary subsystem, no new test
harness, no new runtime layer, and no feature work outside the 34 rows. A
material new support surface stops under the Complexity Escalation Gate.
**Approved complexity:** none (AR-12). **Approved integration surface:** the
`DEF-167` parse-only/stage-select `clc` option (AR-19); it is a driver/test
affordance, not a language, analysis, or type-identity layer.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Gate decisions (confirmed) |
| PF | [Preflight Report](00-preflight-report.md) | Iteration 1 audit, findings, rulings |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Scope and acceptance criteria |
| 02 | [Current State](02-current-state.md) | Current behavior, files, gaps |
| 03-A | [Critical Flow](03-A-critical-flow.md) | DEF-543 source-state join |
| 03-B | [Raw Store Integrity](03-B-raw-store.md) | DEF-454…466 store/alias fixes |
| 03-C | [Conditional Materialization](03-C-conditional-materialization.md) | DEF-470…481 read-base fixes |
| 03-D | [Semantic Parity](03-D-semantic-parity.md) | DEF-411…522 generic/null parity |
| 03-E | [Strict Build](03-E-strict-build.md) | DEF-167/171/416 |
| 05 | [Findings Ledger](05-findings.md) | Execution findings and phase records |
| 07 | [Testing Strategy](07-testing-strategy.md) | ST cases and verification |
| 99 | [Execution Plan](99-execution-plan.md) | Phases, tasks, progress |

## Key Decisions

| Decision | Outcome | AR |
|---|---|---|
| Scope | 34 active critical/major DEFs (1 critical + 33 major); minors excluded | AR-1, AR-2, AR-18 |
| Shape | One umbrella plan, five batches across seven execution phases | AR-3 |
| Order | Critical flow → raw store → conditional value → semantic parity → strict build | AR-4, AR-6 |
| Behavior | Match the concrete twin and frozen spec; no silent spec change | AR-5, AR-11 |
| Precision | Direct existing-pattern fixes; no generalized summary | AR-12 |
| Closure | Failing spec regression, green fix, roadmap Done with evidence; ST-12 and any AR-16 stale close are the green pins | AR-8, AR-16 |
| DEF-167 | Parse-only/stage-select mode; a full compile of the filed local-i32 shape rejects with one E10300 | AR-14, AR-19 |
| DEF-171 | E10213 outside a constant context; module `const` keeps E10224; module-level `let` reports E10213 (R-6: follow `spec/04` §16) | AR-15, AR-23 |
| DEF-416 | Conservative collision report (E11101) from `sema`; no type-identity layer | AR-20 |
| Roadmap | Restore the row evidence; closes go through the roadmap skill | AR-17, AR-21 |

## Specialist Agents

The repository already carries the specialist reviewers this program needs; no
new agent is required.

| Role | Kind | Use | AR Ref |
|---|---|---|---|
| `correctness-reviewer` | reviewer | Every phase; blast radius and test integrity | AR-10 |
| `semantics-reviewer` | reviewer | Phases A, C, D, E; language-rule, evaluation-order, and parity changes | AR-10 |
| `memory-safety-auditor` | reviewer | Phases A, B, C, D; ASAN/LSan probes | AR-10 |
| `emission-integrity-auditor` | reviewer | Phases B, C, D, E; strict-C and corpus diffs | AR-10 |

_Detection evidence: `AGENTS.md` phase-review dispatch names these four project
agents; `.opencode/agents/` carries their briefs. The capability-gap check found
no missing specialist role._

## Related Files

- `compiler/src/own/call_check.rs` — callee source scans and caller guards
- `compiler/src/own/expr.rs`, `place.rs`, `statements.rs`, `loops.rs` — moves, aliases, conditional stores, loop alias roots
- `compiler/src/own/instance_check.rs` — substituted duties
- `compiler/src/own/flow.rs`, `lower.rs`, `own.rs` — flow-sensitive source state and constructors
- `compiler/src/codegen/function.rs` — fixed-array conditional assignment, struct-literal stores, take captures
- `compiler/src/codegen/guards.rs`, `program.rs`, `expr.rs`, `calls.rs` — guarded materialization, initializer literals, literal rendering, argument materialization
- `compiler/src/types/table.rs`, `types/mod.rs`, `types/keys.rs`, `codegen/types.rs` — type identity and C symbol keys
- `compiler/src/sema/call.rs`, `sema/ops.rs`, `sema/expr.rs` — E10300 call checks, conditional/null rules, intrinsic typing
- `compiler/tests/borrow_store_integrity_*` and the new `def_*_integrity_*` pairs
- `plans/00-roadmap.md`, `plans/def-critical-major-closeout/00-preflight-report.md`, `05-findings.md`
