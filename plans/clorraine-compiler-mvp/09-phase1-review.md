# Phase 1 Review: Accepted Codegen Spikes

> **Document**: 09-phase1-review.md
> **Parent**: [Index](00-index.md)
> **Date**: 2026-09-18
> **Scope mode**: strict

## What was reviewed

The complete Phase 1 diff from the Phase 0 completion commit (`5a9d9d8`)
through the spike checkpoint: the four spike fixtures, their Makefiles, and
their raw results; `docs/findings-phase2.md`; the checkpoint amendments to
`spec/17-codegen.md` and `spec/preflight-report.md`; and the register entries
AR-56..AR-59. Reviewer: an independent phase-review subagent carrying the
correctness and maintainability lenses. The diff stayed inside the expected
modification set; no scope drift.

## Findings and rulings

| ID | Severity | Finding | Ruling (user, 2026-09-18) |
|---|---|---|---|
| RV-001 | 🟠 MAJOR | `spec/04-expressions.md:321` still stated the float-to-int guard as `!(f >= lo && f <= hi)`, contradicting the AR-58 amendment in Ch 17 | Apply AR-58 to Ch 04 (the approved ruling propagated to the last stale site) |
| RV-002 | 🟡 MINOR | `spikes/04-run-faults/traps.c` claimed the old guard rejects NaN, which S4 disproved under `vamos` | Annotate that the fixture predates AR-58; keep the comparison so the recorded results still match |
| RV-003 | 🟡 MINOR | `spec/17-codegen.md` §13 still called the float legs unverified though S4 compiled and ran soft-float double on both compilers | Record the S4 exercise; only the 68881 FPU leg stays unverified |
| RV-004 | 🟡 MINOR | `03-06-codegen.md`, `03-02-spikes.md`, and `07-testing-strategy.md` still specified `Cl<Name>` handles and the old NaN guard | Propagate AR-57 and AR-58 into the plan documents |
| RV-005 | 🟡 MINOR | `spikes/01-vbcc-line/results/err.txt` contained a random vbcc temp path, so the committed evidence was not reproducible | Normalize the `/tmp/*.asm` token in the capture target and regenerate |
| RV-006 | 🟡 MINOR | The Phase 1 baseline block omitted the ruling-authorized spec and register files | List them alongside the AR-31 authorization |
| RV-007 | 🟡 MINOR | `spec/preflight-report.md` left PF-006/007/020 `OPEN` though S4 confirms PF-006 and AR-58 rules on PF-007/PF-020 | Mark PF-006 resolved and update PF-007/PF-020 to the AR-58 outcome |

## Areas confirmed clean

- All 31 arithmetic values were recomputed independently and are correct:
  wrapping `+`/`-`/`*`/unary `-`, shift counts masked with `& 31`, arithmetic
  signed `>>`, logical unsigned `>>`, and the division-guard sentinels.
- The trap first lines match Ch 14 exactly, and Ch 17 §10/§14 carry the
  `cl_isnan` guard consistently.
- Ch 17 sections 3, 4, 7, 12, and 14 agree on the ABI handle rule; no stale
  `ClFile` remains in the amended specification.
- The findings document matches the raw transcripts, including the S2 counter
  balance (4=4) and the S4 `vamos` NaN limitation.
- No build artifacts are committed; `.gitignore` covers `spikes/*/build/`.

## Fix batch applied

| Area | Files |
|---|---|
| Specification | `spec/04-expressions.md` (RV-001), `spec/17-codegen.md` §13 (RV-003), `spec/preflight-report.md` (RV-007) |
| Plan | `03-06-codegen.md`, `03-02-spikes.md`, `07-testing-strategy.md` (RV-004), `99-execution-plan.md` (RV-006) |
| Spikes | `spikes/04-run-faults/traps.c` comment (RV-002), `spikes/01-vbcc-line/Makefile` + regenerated `results/err.txt` (RV-005) |

## Verification and re-review

- `./spec/check.sh` stays green (27 files, 101 compile-fail tests).
- `make -C spikes/01-vbcc-line err` regenerates a path-normalized `err.txt`.
- The mandatory-one re-review is not triggered: RV-001 was the only
  critical/major finding, it was a propagation of the already-approved AR-58
  ruling rather than a new defect, and it was fixed by a documentation edit
  with no behavior change.
