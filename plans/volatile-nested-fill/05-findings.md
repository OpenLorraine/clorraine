# Findings and Evidence: Volatile Nested Fill Fix (PLAN-23)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

## Purpose

This ledger records the reproduction, the red/green test record, the
integrity mutations, the base/head emission audit, the review verdicts, and the
gate results for the volatile nested fill fix. The v1.1.1 release is a
separate roadmap row (PLAN-24) and records its evidence in `RELEASE.md`.

## Reproduction

Base commit `7cdbb71`, clean tracked tree. The function-body fill into a
`volatile let` global emits `memcpy` per row:

```
error: passing argument 1 of 'memcpy' discards 'volatile' qualifier
from pointer target type [-Werror=discarded-array-qualifiers]
```

The overlay-field shape, the place-valued prefix row, the fill rows, and a
volatile field named `len` fail with the same diagnostic. The module-level
initializer is already element-wise and valid.

## Red/Green Record

Filled after the tests are added and the fix lands.

| Status | Result |
|---|---|
| Behaviour cases on base | pending |
| Emission pins on base | pending |
| Behaviour cases on head | pending |
| Emission pins on head | pending |
| Integrity mutations | pending |

## Benchmark Comparison

The fix's parent is `7cdbb71`. A fresh base compiler is built from that commit;
the head is the fix commit.

| Item | Value |
|---|---|
| Base compiler | pending |
| Base SHA-256 / dirty | pending |
| Head compiler | pending |
| Head SHA-256 / dirty | pending |
| `make bench-compare` (1% guard, no `REQUIRE`) | pending |

## Emission Audit

Base compiler from `7cdbb71`; head compiler from the fix commit. The fix must
not change any non-volatile unit.

| Unit class | Expected | Observed |
|---|---|---|
| Committed corpus, benchmarks, Amiga examples | byte-identical | pending |
| Volatile global nested fill probe | per-leaf stores, no destination `memcpy` | pending |
| Volatile prefix place row probe | per-leaf stores, no destination `memcpy` | pending |
| Volatile overlay field probe | per-leaf stores, no destination `memcpy` | pending |
| Whole-array place copy `g = other` | unchanged `memcpy` (accepted limit, IP-V12) | pending |

## Reviews

| Review | Verdict | Evidence |
|---|---|---|
| Correctness | pending | pending |
| Memory safety | pending | pending |
| Emission integrity | pending | pending |
| Performance | pending | pending |

No critical or major finding may remain open.

## Accepted Limits

| Limit | Reason |
|---|---|
| Whole-array place copy into a volatile array (`g = other`) still emits `memcpy` | It uses the shared fixed-array store core, not the two nested fill copy sites; separate pre-existing class and follow-up. IP-V12 pins the unchanged emission |
| A call result or self-reading literal copied into a volatile array still emits `memcpy` | Same shared store core; separate follow-up |
| A struct-literal prefix element with a fixed-array field still emits `memcpy` | The struct-field store path does not carry the outer target's volatility; a separate follow-up |
| A conditional, call, or closure-call prefix row may still emit `memcpy` | The prefix wrapper deliberately falls back to the shared core for these forms; a separate follow-up |
| A `for borrow` alias over volatile storage (global or field) fails at the borrow conversion | The qualifier is discarded at the loop header before the fill; a separate alias/pointer issue |
| A `borrow` parameter bound to a volatile field fails at the call argument | The callee parameter type is non-volatile; the fill is not reached through a volatile type |
| A `borrow` parameter bound to a volatile global is rejected (`E10038`) | Not representable; the checker forbids borrowing a global owner |
| An owning explicit row copy (no fill) may still emit `memcpy` | The shared core owns it; a move-only fill is rejected (`E10215`) and a module-level `volatile let` requires a duplicable type (`E11106`), so no fill form is representable |
| Volatile nested fill still materializes a row temporary | The temporary is a local; the fix changes only the volatile destination copy. Removing the temporary for a constant fill is a separate optimization |

## Residual Opportunities

| Opportunity | Disposition |
|---|---|
| Element-wise copy for the whole-array store core | Follow-up; needs its own design for self-read, conditional, and drop paths |
| Volatility through alias and borrow indirection | Follow-up; needs the pointer/borrow types to carry the qualifier |
| Avoid the row temporary for a volatile constant-zero fill | Not needed for correctness; no measured need |

## Final Acceptance

| Gate | Result |
|---|---|
| Focused suites | pending |
| `make check` | pending |
| `make asan` | pending |
| `make ubsan` | pending |
| `make corpus-amiga` | pending |
| `make differential` | pending |
| `make bench-shape` | pending |
| `make bench-base COMMIT=7cdbb71` / `make bench` / `make bench-compare` | pending |
| `make verify` | pending |
| Emission audit | pending |
| Reviews | pending |
