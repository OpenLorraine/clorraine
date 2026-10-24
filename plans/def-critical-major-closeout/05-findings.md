# Findings Ledger: Critical/Major DEF Closeout

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| — | — | — | Awaiting execution | — |

## Preflight Rulings

The iteration 1 preflight report (`00-preflight-report.md`) records 28 findings
(6 critical, 16 major, 6 minor) and the delegated technical resolutions. The
user ruled the reserved items on 2026-10-24:

| Ref | Ruling | Date |
|---|---|---|
| R-1 / PF-001 | Add `DEF-458` to Batch B; the active set is 34 (1 critical + 33 major), 35 DEF rows including `DEF-470` | 2026-10-24 |
| R-2 / PF-003 | Add a parse-only/stage-select mode for the `DEF-167` stage test; the full compile reports E10300 | 2026-10-24 |
| R-3 / PF-006 | Close `DEF-416` by the conservative collision report (E11101), not module-qualified identity | 2026-10-24 |
| R-4 / PF-021 | Restore the roadmap row evidence/header; phase closes go through the roadmap skill | 2026-10-24 |
| R-5 / PF-018 | Revalidate `DEF-506`/`507`; use the conservative proof; a summary surface needs a new ruling | 2026-10-24 |
| AR-22 | Apply the delegated fixes; the plan documents are corrected | 2026-10-24 |

## Iteration 2 Preflight Rulings

Preflight iteration 2 (2026-10-24) verified the iteration-1 corrections and
found residual and new defects. The report `00-preflight-report.md` carries the
full evidence. The applied corrections are:

| Ref | Ruling | Date |
|---|---|---|
| PF-002 | Add the constant-context flag; a module `const embed` reports only `E10224` | 2026-10-24 |
| PF-003 | The full compile of the filed local-i32 stage shape reports one `E10300`; keep the parse-only stage mode | 2026-10-24 |
| PF-004 | Put the source set on `PlaceState`; strong-update on assignment, join in `merge`, prune with the place; keep the destination-root, take, and type-parameter rules | 2026-10-24 |
| PF-005 | Test source-vs-destination containment; clear the taken subrange of the save copy; attach the range to the capture | 2026-10-24 |
| PF-006 | Report the `DEF-416` collision from `sema` with a pure `TypeTable` query, not the table constructor | 2026-10-24 |
| PF-008 | The nullable conditional loop head is a mandatory `E10012` case | 2026-10-24 |
| PF-010 | Complete the remaining ST sources, unsafe contexts, exit values, and strict-C assertions | 2026-10-24 |
| PF-011 | Pin the `DEF-522` arm backing and the exact `E10030` local-arm code | 2026-10-24 |
| PF-013 | Define the must-source walk for the cross-frame scan | 2026-10-24 |
| PF-017 | Thread the `DEF-472`/`473` destination through `statements.rs`, `place.rs`, and `mono.rs` | 2026-10-24 |
| PF-019 | Extend the test-impact inventory and add a retarget line to each phase close | 2026-10-24 |
| PF-023 | Require a struct declaration order that differs from the source order in ST-20 | 2026-10-24 |
| PF-029 | Allow the `DEF-506`/`DEF-507` stale green pin in R6 and the acceptance criteria | 2026-10-24 |
| PF-030 | Build the probe binary before the Phase 0 evidence probe; author the source-set pin after the field exists | 2026-10-24 |
| PF-031 | Use the pinned destination index and the run-time source shape in ST-9 | 2026-10-24 |
| PF-032 | Add the four new test pairs to the AGENTS.md test-naming map | 2026-10-24 |
| R-6 | **Pending user ruling:** `spec/04` §16 says a module-level `let` initializer is not a constant context; `spec/13` §1 says it is. `DEF-171`'s module-level `let embed` outcome depends on this | — |

## Red-Phase Evidence

Every active row records one line per ST case before its fix:

| ST | DEF | Command | Base commit | Observed failure |
|---|---|---|---|---|
| ST-N | DEF-N | `cargo test --test def_*_integrity_spec_test <name>` | `<sha>` | `<diagnostic, ASAN/LSan, strict-C, or trace>` |

ST-12 and any AR-16 stale close record the HEAD command and the accepted result
instead of a failure.

## Phase Records

Execution adds one section per phase with the revalidation results, review
findings, and the final verification evidence. No row is marked Done without a
current-HEAD reproduction or a documented stale proof.
