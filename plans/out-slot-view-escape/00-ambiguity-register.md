# Ambiguity Register: out-slot-view-escape

> **Status**: ✅ GATE PASSED — 1 runtime item resolved
> **Last Updated**: 2026-10-22 01:18
> **Scope**: The multi-phase task plan and its execution; runtime items are tagged `(runtime)`.

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|---|---|---|---|---|
| 1 | Technical (runtime) | Existing fixture migration sequencing | A: migrate all existing fixtures before the rule; B: land the rule against the new independent spec cases, then migrate the existing fixtures | AI — delegated by `--auto-design` (see AR-1) | ✅ Resolved |

## Resolution Notes

**AR-1:**
Authority: AI — delegated by `--auto-design`
Eligibility: implementation and testing sequencing inside the approved foreign-lend store rule; no product behavior, scope, or acceptance-criteria change.
Objective: implement the approved rule without guessing the final diagnostic count and message at each old fixture.
Decision: option B. Land the concrete and generic judgments against the new spec cases (1.1.1/1.1.2), which are the independent red oracle, then migrate the existing fixtures (1.1.3, 1.1.5-1.1.8) against the final diagnostics.
Evidence: the new cases pin every accepted and rejected source shape and were confirmed red at the foreign-store cases; the old fixtures encode the reversed acceptance, and their caller-side E10030 expectations can change again when the one-report ownership lands.
Rejected alternatives: option A edits the 188+ fixtures twice and risks fitting them to an intermediate diagnostic.
Strongest counterargument: the plan ordered the fixture migration before implementation.
Confidence: High — reopen if an old fixture reveals a behavior the new cases do not pin.
Hardening: not required; eligible sequencing with no material support surface.
Policy version: 1
Root invocation ID: exec-out-slot-view-escape-20261022
Reopen triggers: a new case or an old fixture exposes a source shape, accepted control, or report count not covered by the new spec cases.
