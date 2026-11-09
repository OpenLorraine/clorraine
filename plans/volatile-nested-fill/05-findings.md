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

| Status | Result |
|---|---|
| Behaviour cases on base | 13 passed, 10 failed (ST-V1..V10); the `memcpy` shapes fail the strict host build, and ST-V8 also observes the prefix call re-evaluated four times |
| Emission pins on base | 24 passed, 10 failed (IP-V1..V6 and IP-V13..V16); every positive pin finds the destination `memcpy` it forbids |
| Behaviour cases on head | 23/23 pass |
| Emission pins on head | 34/34 pass |
| Integrity mutations | 8 mutations, each caught |

Base failure shape (repeated per volatile case):

```text
error: passing argument 1 of 'memcpy' discards 'volatile' qualifier
from pointer target type [-Werror=discarded-array-qualifiers]
```

The base-green guards are IP-V7 (non-volatile field `memcpy`), IP-V8 (local
`memset`, covered by the existing compaction pin), IP-V9 (volatile one-extent
stores), IP-V10 (volatile full prefix), IP-V11 (volatile explicit rows), and
IP-V12 (whole-array place `memcpy`). They prove the fix does not widen beyond
the two nested fill copy sites.

### Test-Integrity Mutations

Each mutation was applied to the fix, the focused suite was run, and the
source was restored.

| Mutation | Observed failure |
|---|---|
| Restore the fill-row `memcpy` | impl 24/34 (IP-V1..V6, IP-V13, IP-V15 fail); spec 13/23 (ST-V1..V7, ST-V9, ST-V10 fail) |
| Fall the prefix row back to the shared core | impl 32/34 (IP-V5, IP-V14 fail); spec 21/23 (ST-V5, ST-V8 fail) |
| Force `fill_target_volatile` false | impl 24/34 (the volatile place pins fail); spec 13/23 |
| Force `fill_target_volatile` true | impl 26/34 (IP-V7 and the non-volatile compaction pins fail); spec stays green |
| Make the `.len` guard unconditional | impl 33/34 (IP-V6 fails) |
| Remove the copy recursion (one dimension only) | impl 33/34 (IP-V2 fails); spec 22/23 (ST-V2 fails) |
| Treat an impure prefix value as stable | impl 33/34 (IP-V14 fails); spec 22/23 (ST-V8 fails with `calls: 4`) |
| Disable the run-time index volatility | impl 32/34 (IP-V13, IP-V15 fail); spec 21/23 (ST-V7, ST-V9 fail) |

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
| Run-time indexed volatile element probe | per-leaf stores, no destination `memcpy` | pending |
| Impure prefix row probe | one call, row temporary, per-leaf stores | pending |
| Volatile pointer-field probe | per-leaf stores, no destination `memcpy` | pending |
| Whole-array place copy `g = other` | unchanged `memcpy` (accepted limit, IP-V12) | pending |

## Reviews

The first codegen commit `9e35a65` received four independent reviews. Three
found issues that the second commit fixes; one was investigated and closed as
a non-defect.

| Review | Verdict | Evidence |
|---|---|---|
| Correctness | one major, two minor | RV-VOL-001 run-time indexed target gap; RV-VOL-002 volatile pointer field; RV-VOL-003 doc wording |
| Memory safety | one major | the prefix wrapper re-evaluated an impure source once per leaf (`getptr().r` called four times, ASan global-buffer-overflow) |
| Emission integrity | no introduced finding | 338 emitted units, zero non-volatile diffs, zero symbol/declaration changes; PRE-1 run-time index and PRE-2 volatile source copy reported as pre-existing |
| Performance | land | no scored change; the volatile copy is compile-time-only on non-volatile paths |

| Finding | Severity | Resolution |
|---|---|---|
| RV-VOL-001: a run-time indexed whole-element target (`g[i]`, `regs.a[i]`) bypassed the volatility predicate and still emitted `memcpy` into volatile storage | MAJOR | The `StorePointer` fixed-array branch derives the flag from the addressed value tree; ST-V7/ST-V9 and IP-V13/IP-V15 pin the global and field forms |
| RV-VOL-002: a `volatile raw` pointer field was said to make a non-volatile pointee volatile | MINOR | Investigated: the emitter spells the field `volatile T *p`, so the pointee is volatile and the per-leaf copy is the valid spelling. IP-V16 pins the shape; no predicate change |
| RV-VOL-003: the copy helper's doc claimed "the same bytes" for a struct leaf, but C struct assignment leaves padding unspecified | MINOR | The doc now says "the same typed values" and notes the padding boundary |
| MEM-001: the prefix wrapper rendered an impure value once into a C expression, then appended an element index per leaf, re-running a hidden call | MAJOR | The wrapper evaluates an impure value once into a row temporary with the shared core, then copies from it; ST-V8 counts one call and IP-V14 pins the single call and the temporary |
| PRE-1 (emission audit): run-time indexed target | MAJOR pre-existing | Fixed by the same change as RV-VOL-001 |
| PRE-2 (emission audit): a volatile array used as a copy source into the row temporary | MAJOR pre-existing | Recorded as a separate shared-core limitation; not a nested fill destination |
| PRE-3 (emission audit): a whole-array copy *from* volatile storage | MINOR pre-existing | Recorded with the whole-array copy limitation |

Both review-driven code changes were committed and the full fix gate is rerun
on the new clean commit.

## Accepted Limits

| Limit | Reason |
|---|---|
| Whole-array place copy into a volatile array (`g = other`) still emits `memcpy` | It uses the shared fixed-array store core, not the nested fill copy sites; separate pre-existing class and follow-up. IP-V12 pins the unchanged emission |
| A call result or self-reading literal copied into a volatile array still emits `memcpy` | Same shared store core; separate follow-up |
| A volatile array used as a copy source into the row temporary still emits `memcpy` | The row temporary's fill uses the shared core; the source qualifier is a separate pre-existing class (PRE-2) |
| A struct-literal prefix element with a fixed-array field still emits `memcpy` | The struct-field store path does not carry the outer target's volatility; a separate follow-up |
| A `for borrow` alias over volatile storage (global or field) fails at the borrow conversion | The qualifier is discarded at the loop header before the fill; a separate alias/pointer issue |
| A `borrow` parameter bound to a volatile field fails at the call argument | The callee parameter type is non-volatile; the fill is not reached through a volatile type |
| A `borrow` parameter bound to a volatile global is rejected (`E10038`) | Not representable; the checker forbids borrowing a global owner |
| An owning explicit row copy (no fill) may still emit `memcpy` | The shared core owns it; a move-only fill is rejected (`E10215`) and a module-level `volatile let` requires a duplicable type (`E11106`), so no fill form is representable |
| Volatile nested fill still materializes a row temporary | The temporary is a local; the fix changes only the volatile destination copy. Removing the temporary for a constant fill is a separate optimization |

## Residual Opportunities

| Opportunity | Disposition |
|---|---|
| Element-wise copy for the whole-array store core | Follow-up; needs its own design for self-read, conditional, and drop paths |
| Volatile source copy into a row temporary (PRE-2) | Follow-up; the shared core would need a source-qualifier path |
| Struct-literal prefix element with a fixed-array field | Follow-up; the struct-field store path would need the outer target's qualifier |
| Volatility through alias and borrow indirection | Follow-up; needs the pointer/borrow types to carry the qualifier |
| Volatility lookup keyed by simple struct name | Uses the existing `structs` map, which already keys field emission by simple name; a duplicate name across modules inherits that pre-existing type-table ambiguity |
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
