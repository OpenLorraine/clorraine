# Volatile Nested Fill Fix (PLAN-23)

> **Feature**: Make a nested fixed-array fill into `volatile` storage emit valid C by copying each row leaf by leaf instead of calling `memcpy` through a volatile-qualified object.
> **Status**: Preflight clean (three passes); execution ready
> **Created**: 2026-11-09
> **Last Updated**: 2026-11-09
> **Base commit**: `7cdbb71`
> **Implements**: R1-R4 (fix). Phase 5 executes the separate PLAN-24 release row.
> **CodeOps Artifact Schema**: 1

**Original goal:** close the invalid-C defect surfaced by the nested fill
closeout before cutting the v1.1.1 patch release. The compiler accepts a
function-body nested fill into a `volatile` module array or a `volatile` struct
field, then emits `memcpy` into a volatile-qualified object. Strict C rejects
the call with `-Werror=discarded-array-qualifiers`, so accepted source fails
the build.

## Why This Fix

The nested fill writer evaluates the row once into a row temporary and copies
it to every covered row. The copy assumes the destination is an ordinary
object. Volatility is a property of module-level `let` bindings and struct
fields, and both reach the copy loop:

| Shape | Emitted copy | Strict C |
|---|---|---|
| `volatile let g: i32[8][9]`; `g = [; [; 0]];` | `memcpy(&(cl_..._g[0]), &(__cl_fillrow_1), ...)` | rejected |
| `struct S { a: volatile i32[2][3]; }`; `regs.a = [; [; 0]];` | `memcpy(&(regs->a[0]), ...)` | rejected |
| `volatile let g: i32[4][4]`; `g = [row; [; 0]];` | `memcpy(&(cl_..._g[0]), &(row), ...)` for the prefix row | rejected |
| plain local, parameter, return slot | `memcpy(...)` or one `memset` | valid, unchanged |

The fix keeps the evaluate-once temporary and changes only the copy spelling
for a volatile destination: every deepest element is stored through the
volatile lvalue, in row-major order. That is the same byte value, keeps each
access volatile, and is valid C99. The prefix-row path uses a dedicated
wrapper so the shared fixed-array store core and its whole-array copies are
not changed.

## Scope Boundary

In scope: volatile-target detection for the two nested fill copy sites, the
per-leaf copy spelling, the behaviour and emission tests, the strict-shape
fixture, the required reviews, and the closeout records. The v1.1.1 release is
the separate PLAN-24 roadmap row; Phase 5 of the execution plan is its
checklist and follows `RELEASE.md`.

Out of scope: whole-array assignment or move copies into a volatile array
(`g = other`, a call result, a self-reading literal), a volatile array used as
a copy source into a row temporary, struct-literal prefix elements with a
fixed-array field, a nested fill through a `for borrow` alias over volatile
storage, a `borrow` parameter bound to a volatile field, an owning explicit row
copy, and any language, grammar, ABI, runtime, stdlib, driver, LSP, or
benchmark change. These are recorded as separate limitations or follow-ups. A
conditional or call-valued prefix row is in scope: it is evaluated once into a
row temporary.

## Document Index

| # | Document | Description |
|---|---|---|
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Functional, test, verification, and closeout requirements |
| 02 | [Current State](02-current-state.md) | Reproduction, root cause, affected and excluded shapes |
| 03 | [Design](03-design.md) | Volatility predicate, copy helper, prefix wrapper, safety argument |
| 05 | [Findings](05-findings.md) | Red/green record, benchmark, emission audit, reviews, gates |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec cases, emission pins, integrity mutations |
| PF | [Preflight Report](00-preflight-report.md) | Three-pass adversarial audit and rulings |
| 99 | [Execution Plan](99-execution-plan.md) | Phases, tasks, commit plan, release checklist |

## Decisions

| Decision | Value |
|---|---|
| Base commit | `7cdbb71`; plan docs committed before the change |
| Fix sites | fill rows, prefix rows (stable or one-time temporary), and run-time indexed elements |
| Volatility signal | `ArrayTarget::is_volatile`, computed from the place |
| Copy spelling | one store per leaf through the destination lvalue; impure prefix values evaluate once |
| Non-volatile path | unchanged `memcpy` / `memset` |
| Shared fixed-array core | unchanged; whole-array copies remain a separate limit |
| Benchmark base | fresh `7cdbb71` compiler; bare 1% comparison |
| Push cadence | commit at phase boundaries; push after the full fix gate |
| Release | PLAN-24, a separate row, after the fix closeout; publication requires explicit session authorization |

## Specialist Agents

| Role | Agent | Phase |
|---|---|---|
| Correctness reviewer | `correctness-reviewer` | Codegen |
| Memory-safety auditor | `memory-safety-auditor` | Codegen (read-only ASan probes) |
| Emission-integrity auditor | `emission-integrity-auditor` | Codegen (read-only base/head diff) |
| Performance auditor | `performance-auditor` | Codegen (read-only no-regression check) |

Review packets include a blast-radius step and a test-integrity step. Critical
and major findings pause execution for a ruling.
