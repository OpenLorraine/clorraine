# Zero-Defect Closeout Implementation Plan

> **Feature**: Close every remaining open defect in `plans/00-roadmap.md` so the repository can resume feature work on a zero-open-DEF baseline.
> **Status**: Plan Created — the Zero-Ambiguity Gate passed on 2026-10-28; Phase 0 revalidation is the first execution step.
> **Created**: 2026-10-28
> **Defects**: DEF-544, DEF-545, DEF-546, DEF-547, DEF-548, DEF-549, DEF-550, DEF-551, DEF-552, DEF-553, DEF-554, DEF-555, DEF-556, DEF-557, DEF-190, DEF-191, DEF-192, DEF-193, DEF-195, DEF-246, DEF-247, DEF-248, DEF-249, DEF-383, DEF-414, DEF-417, DEF-420, DEF-467, DEF-483, DEF-542, DEF-169, DEF-173, DEF-174, DEF-198, DEF-203, DEF-205, DEF-212, DEF-214, DEF-236, DEF-239, DEF-240, DEF-241, DEF-244, DEF-254, DEF-349, DEF-382, DEF-387, DEF-388, DEF-389, DEF-404, DEF-405, DEF-419, DEF-250, DEF-252, DEF-311, DEF-312, DEF-354, DEF-370, DEF-422, DEF-434, DEF-444, DEF-463, DEF-474, DEF-484, DEF-490, DEF-491, DEF-492, DEF-494, DEF-524, DEF-530, DEF-194, DEF-199, DEF-200, DEF-201, DEF-202, DEF-206, DEF-208, DEF-217, DEF-218, DEF-221, DEF-222, DEF-223, DEF-224, DEF-225, DEF-226, DEF-227, DEF-228, DEF-234, DEF-253, DEF-263, DEF-313, DEF-320, DEF-324, DEF-325, DEF-326, DEF-327, DEF-328, DEF-330, DEF-334, DEF-352, DEF-355, DEF-358, DEF-392, DEF-393, DEF-438, DEF-482, DEF-500, DEF-210, DEF-314, DEF-316, DEF-317, DEF-318, DEF-319, DEF-321
> **CodeOps Artifact Schema**: 1

## Overview

The roadmap holds 114 open DEF rows: 5 critical, 9 major, and 100 minor. They
span conditional-store and take integrity, view returns and loans, null and
narrowing rules, call resolution, emission order and shapes, initializer
carriers, and a few driver/runtime gaps. This plan closes them in six
risk-ordered batches after a revalidation phase. Every row gets a
specification regression that is red before its fix, a direct fix in the stage
that owns the rule, and an implementation or ASAN/strict-C pin where the class
needs one.

The plan reuses the existing ownership, instance-audit, conditional-emission,
and strict-build machinery. The only new support surface is the bounded
instance-bound static-return marker approved for `DEF-557` (AR-5); no general
callee-summary subsystem is added.

## Scope

**IN scope:** the 114 open rows listed in `> **Defects**`, the authorized
registry/spec additions (`E10309`, the `E10010` call spelling, the `len`
surface), and the `DEF-557` marker.

**OUT of scope:** feature work, the pre-existing non-DEF list recorded in
`def-critical-major-closeout/05-findings.md`, and any generalized analysis
surface beyond the approved marker.

## Batches

| Phase | Batch | Rows | Focus |
|---|---|---|---|
| 0 | Revalidation | 114 | Reconfirm every row at HEAD; close stale/incidental rows with evidence |
| 1 | Critical/major backlog | 14 | Conditional store/take, view returns, initializer order |
| 2 | Null and type rules | 16 | Null contexts, narrowing, `len`, nullable view stores |
| 3 | Calls and resolution | 22 | Arity, members, ffi, markers, diagnostics, imports |
| 4 | Borrow and loans | 18 | Moves, captures, element loans, carrier returns |
| 5 | Emission and ABI | 37 | Strict C, materialization, evaluation order, folds |
| 6 | Initializers and carriers | 7 | Fresh-root leaks and initializer literals |
| 7 | Close-out | — | Full verify, reviews, roadmap zero, final summary |

## Documents

- [Ambiguity register](00-ambiguity-register.md)
- [Requirements](01-requirements.md)
- [Current state](02-current-state.md)
- [Batch A — critical/major backlog](03-A-critical-backlog.md)
- [Batch B — null and type rules](03-B-null-and-types.md)
- [Batch C — calls and resolution](03-C-calls-and-resolution.md)
- [Batch D — borrow and loans](03-D-borrow-and-loans.md)
- [Batch E — emission and ABI](03-E-emission-and-abi.md)
- [Batch F — initializers and carriers](03-F-initializers-and-carriers.md)
- [Findings ledger](05-findings.md)
- [Testing strategy](07-testing-strategy.md)
- [Execution plan](99-execution-plan.md)

## Specialist Agents

None. The four committed project agents — `correctness-reviewer`,
`semantics-reviewer`, `memory-safety-auditor`, and
`emission-integrity-auditor` — already cover every finding area in the 114
rows (ownership/drop, language rules, host sanitizers, and emission/strict C).
The specialist check is recorded in `codeops/specialist-check.json`
(2026-10-27). No new specialist candidate is justified by the plan's evidence.
