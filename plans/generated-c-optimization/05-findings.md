# Findings and Evidence: Generated-C Optimization

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

## Purpose

This is the evidence ledger for the plan. It owns the committed base/head
measurements, the red/green pin record, the accepted limits, and the rejected
candidates. The exploratory study stays in
[Current State](02-current-state.md); the requirements and design stay in
[Requirements](01-requirements.md) and
[Generated-C Changes](03-02-generated-c-changes.md).

## Committed Baseline

Filled by the execution task after `make bench-base` and before the first
optimization. The base compiler is the recorded commit, built by
`docker/scripts/bench-base.sh`; the retained binary is
`build/perf/base-clc`.

| Benchmark | `instructions_net` | m68k `.text` `-O2` | C bytes | Host `-O2` median |
|---|---:|---:|---:|---:|
| calibration | 3,373 | — | — | 0.55 ms |
| sieve | 1,520,792 | 101,396 | 489,983 | 0.90 ms |
| matrix | 22,059 | 80,732 | 313,475 | 0.93 ms |
| strings | 2,972,051 | 77,540 | 312,281 | 2.36 ms |
| moves | 45,205 | 76,880 | 312,756 | 2.16 ms |

**Base commit:** `0ac87d3e03ece01a0df474f1cd43826df90c0f41`

**Toolchain identity:** host `cc` Debian 12.2.0; `m68k-amigaos-gcc`
16.2.0b; `amitools 0.8.1.dev274+gd0b9727da`; Python 3.11.2.

**Commands:** `make bench-base COMMIT=0ac87d3`, `make bench`,
`make bench-compare`. The retained base compiler is `build/perf/base-clc`.

## Head and Delta

Filled after each optimization phase and at the final acceptance.

| Benchmark | Base instructions | Head instructions | Delta | Base `.text` | Head `.text` | Delta | C bytes delta | Output |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| sieve | 1,520,792 | 1,040,581 | −31.6% | 101,396 | 76,664 | −24.4% | −36.6% | equal |
| matrix | 22,059 | 10,959 | −50.3% | 80,732 | 80,652 | −0.1% | −0.03% | equal |
| strings | 2,972,051 | 2,972,051 | 0.0% | 77,540 | 77,492 | −0.1% | −0.01% | equal |
| moves | 45,205 | 45,205 | 0.0% | 76,880 | 76,832 | −0.1% | −0.01% | equal |

**Head identity:** commit `8bec4b5`, clean tree, `clc` SHA-256
`f1fcc6b6b78017c0e9abaeceace0a90e72bdacc39370c6755d6da34e23ded7b0`.
Host time is advisory; the container medians vary by tens of percent and are
not part of the pass/fail gate.

**Final verdict:** `make bench-compare` passed. The R4.8 sieve target
(−20% instructions) and the R5.7 targets (−10% `.text`, −25% C bytes) are
met; no benchmark regressed beyond 1%. The matrix instruction drop is the
same const-fold mechanism: its loop bound `N` is now a literal. The matrix
`.text` win is small because its nested fill is outside the compact scope and
the removed float folds no longer reduce the standard library's `.text`.

## Emission Audit

The base compiler (`build/perf/base-clc`) and the head compiler emitted the
same 40 sources: the four corpus programs, the four benchmark programs plus
calibration, and every `examples/amiga/*.clo` program (three corpus programs
needed the hosted-amiga profile). Every changed unit falls into one of four
classes; no diff is unexplained:

| Class | Example | R |
|---|---|---|
| Scalar module const read replaced by a literal | `cl_std_io_MODE_NEWFILE` → `1006`, `cl_std_amiga_custom_COP_DMA` → `128u`, Boing `WIDTH/HEIGHT/TILE` → literals | R4 |
| Large one-extent scalar fill replaced by `memset` or a loop | sprite-demo `pixels` fill, imageviewer/paddle/boing buffers | R5 |
| Dead `< 0` half removed for a non-negative slice/str constant | `(0 < 0)` in `std.mem` and `buffer-container` | R6 |
| Constant divisor guard removed | `2 == 0`, `3 == 0`, and folded `TILE` guards | R7 |

The non-constant-divisor guard remains (for example `std.math`'s
`x == 0 || (value == MIN && x == -1)`). The full check, ASan, corpus, and
differential legs build and run the same units; the emission diff is the
attribution record, not a substitute for those gates.

## Red/Green Pin Record

The base run of `generated_c_optimization_impl_test.rs` records:

| Status | Pins |
|---|---|
| Red on base | IP-1, IP-2, IP-3, IP-5, IP-6, IP-7, IP-9, IP-10, IP-14, IP-15, IP-17, IP-20 |
| Green on base | IP-4, IP-8, IP-11, IP-12, IP-13, IP-16, IP-18, IP-19 |

**Base evidence:** the base compiler emitted 8,192 per-element stores for the
sieve fill, read `cl_..._N` in the loop tests, wrote `(1 < 0)` for a slice
constant index, and wrote `2 == 0` in the `% 2` guard. The base compiler was
not asked to run the new impl suite (its tests use `CARGO_BIN_EXE_clc`), so the
red set is established from those emitted-C probes plus the base/head
instruction and size deltas.

**After the change:** `generated_c_optimization_spec_test` has 36 passing
cases and `generated_c_optimization_impl_test` has 34 passing pins. The
fix-pass cases cover volatile targets on every fill path, integer consts
through float casts, unsigned unary divisors, binary/float zero divisors,
the 63/64 non-byte threshold, and the materialized conditional.

## Accepted Limits

| Limit | Reason | Requirement |
|---|---|---|
| Module-level initializer reads do not fold | Separate lowering path; no measured need | R4.7 |
| All `f32`/`f64` consts stay places | The folder evaluates floats in `f64` and does not round an `f32` cast at each operation; a folded float could differ from its storage. The measured need is integer constants | R4.1, R4.5 |
| Narrow arithmetic initializers stay places | C promotes a sub-32-bit operation to `int`, while the typed folder wraps at the operand width; folding would disagree with the stored value. A direct narrow literal or cast still folds | R4.5 |
| Enum-derived and unresolved-name initializers stay places | The folder's enum auto-increment wraps at the backing type while the emitter writes the C sequence value | R4.1, R4.5 |
| Field, volatile-target, `DerefTemp`-alias, nested-row, struct-literal, deferred, and self-read fills keep their spelling | A field may name a volatile member, a volatile array cannot be passed to `memset`, and the shared self-read path preserves the old target bytes; the measured fill is a literal local | R5.1 |
| A non-byte fill below 64 slots stays unrolled | The loop saves code size but is slower than unrolled stores at 16–63 slots; a byte **constant** keeps the 16-slot `memset` threshold, while a byte non-constant uses 64 | R5.1, AR-6 |
| Nested-row, struct-literal, array-copy, and deferred expression fills keep their spelling | No measured need; the measured fill is a one-extent scalar statement fill | R5.1 |
| The upper index bound is not eliminated by the compiler front end | Manual elimination measured slower on m68k; the C compiler removes it after const folding | R6.4 |
| Host time is advisory | run-to-run spread is often tens of percent; the target metric is the m68k instruction count | R1.5, AR-2 |
| The harness has no FS-UAE cycle counter | FS-UAE 3.1.66 has no stable headless counter; `vamos` is deterministic | AR-16 |
| The harness does not report per-struct sizes | The linked section sizes are the accepted code-size metric; the ABI is frozen | AR-22 |

## Rejected Candidates

| Candidate | Evidence | Decision |
|---|---|---|
| Manual upper-bound elimination | m68k instructions 2,165,514 → 2,436,214 after const folding; container host `-O2` 31.0 → 35.0 ms median | Rejected |
| `static const` / read-only storage | Separate storage/init change; would break accepted `unsafe &N` mutation | Rejected |
| `if (sizeof(...))` marker removal | Compile-time no-op; no measured cost | Rejected |
| Trailing `continue` removal | Removed by the C compiler at `-O2`; no measured delta | Rejected |
| `bool` `memset` | `bool` is 32-bit on the target | Rejected |
| `memset` for the whole fill when a prefix exists | Must start at `&target[prefix]` | Rejected (design corrected) |
| Binary constant divisors in the guard fold | A wrapping binary divisor can lose the `MIN / -1` trap | Rejected |

## Review Findings and Resolutions

The five independent phase reviews (correctness, memory safety, emission
integrity, performance, semantics) found one critical and several major
defects in the first implementation. All were fixed in the fix pass and are
pinned by the expanded suites:

| Finding | Severity | Resolution |
|---|---|---|
| RV-001 / MS-003 / SR-002: the fill `memset` used the untyped 128-bit constant evaluator, losing operand-width wrap and shift masking | CRITICAL | The fill uses the restricted, width-correct constant helper; a binary fill falls back to the loop. The unary constant evaluator now casts its result to the unary type. Pinned by ST/impl shift and wrapping-divisor cases |
| MS-001: a huge negative unary slice index could lose the `< 0` guard | CRITICAL | Same width-correct unary fix; the guard sees the negative C value. Pinned by ST/impl `-i32(2147483648)` |
| MS-002: `~u32(4294967295)` divisor could lose the zero trap | MAJOR | Same unary fix; the zero test now uses the restricted helper. Pinned by ST/impl bitnot divisor |
| RV-002 / F6: address-taken exclusion was not transitive across const initializers | MAJOR | `stable_const_initializer` rejects an address-taken referenced const. Pinned by the chain-mutation spec case |
| RV-003 / SR-001 / SR-003 / F3 / F4: float folding could emit `inf.0`, mis-round `f32`, or fold f32 arithmetic | MAJOR | Floats stay places. Pinned by the float-place spec case |
| SR-004: narrow arithmetic const reads disagreed with their emitted storage | MAJOR | Narrow-arithmetic initializers stay places. Pinned by the narrow-const spec case |
| SR-005: enum-derived const folded differently from the emitted member | MINOR | Unresolved names and enum-derived initializers stay places. Pinned by the enum-const spec case |
| RV-004 / F8: the materialized and struct-literal fills compacted although R5 excludes them | MAJOR | The emitter has a `compact_fills` flag, a scalar-element gate, and field/volatile target gating; the materialized caller clears it. Pinned by the while-materialized and struct-fill impl cases |
| F5: a folded shifting divisor could be misclassified as zero | MAJOR | The value renderer's zero test uses the restricted helper; a binary divisor keeps the full guard. Pinned by the bitnot case |
| F2: `memset` on a volatile target was invalid C | MAJOR | Field and volatile targets keep the unrolled spelling. Pinned by the volatile-target spec/impl case |
| RV-005: address-taken pins were weak; the shape fixture lacked a prefixed fill/slice | MINOR | Direct pin counts exactly four occurrences; the nested spec case includes a lambda; the shape fixture adds a prefix and a guarded slice |
| RV-006 / PE-002: the trace timeout was not enforced while stdout was drained | MINOR | Both streams are drained in threads and `wait(timeout)` is the watchdog; a stub-`vamos` timeout test covers it |
| RV-007: `bench-shape.sh` deleted an unvalidated `OUTDIR` | MINOR | The script confines `OUTDIR` under `build/perf/` and rejects `..` |
| PE-001: a non-byte fill loop was slower than unrolled stores at 16–63 slots | MINOR | The non-byte loop threshold is 64; the byte memset threshold stays 16. The trade-off is recorded above |
| PE-003: `head.json` was taken from a dirty tree | MINOR | `make bench` is rerun from the clean fix commit; the harness records a dirty flag and the compiler SHA-256 |
| PE-004: host time was not displayed and the noise text was too narrow | MINOR | The compare prints an advisory host/C-byte table; the docs say host noise is tens of percent, not 6–10% |
| Second fix pass: volatile targets on the reads/conditional/field paths still compacted | CRITICAL | `emit_array_assign_to` now takes an explicit `compact` flag; only the plain statement target compacts, every bypass path passes false. Pinned by the volatile-path spec and three impl cases |
| Second fix pass: integer consts through an `f32` cast folded unrounded | MAJOR | `value_initializer_is_stable` rejects every float-typed subexpression and every float const reference. Pinned by the f32-cast spec/impl case |
| Second fix pass: an unsigned sub-int unary divisor lost the `MIN/-1` guard | MAJOR | The unary renderer and constant evaluator both wrap unsigned unaries at the operand type. Pinned by the unsigned-unary spec/impl case |
| Second fix pass: a binary or float-cast zero divisor lost the static trap | MAJOR | The constant evaluator reproduces C wrap/shift semantics and the renderer's zero helper recognizes a finite float cast to zero. Pinned by the binary/float-zero spec and impl cases |
| Second fix pass: a materialized conditional fill still compacted | MINOR | The explicit flag is false on that path. Pinned by the materialized-conditional spec/impl case |
| Second fix pass: the 63/64 non-byte threshold and byte non-constant threshold were unpinned | MINOR | Added the 63/64 impl case; a byte non-constant uses the 64 loop threshold |
| Second fix pass: the `DerefTemp` volatile alias could compact | MINOR | `fill_target_compactable` rejects a `DerefTemp` target; the shape is not strict-valid today and remains recorded as a conservative gate |
| Second fix pass: trace timeout left the child unreaped | MINOR | The timeout path kills, waits, and closes both streams; the stub-`vamos` test covers it |

## Pre-existing Defects (outside this change)

| Defect | Base evidence |
|---|---|
| A pure fill's operand guards are not emitted (`[; src[10]]` reads out of bounds) | `build/perf/base-clc` emits the same unguarded fill and the same ASan report; separate backlog item |
| Narrow arithmetic in a const/inline expression is not wrapped at its result type (`i32(u8(200)+u8(100))` prints 300) | The base compiler prints 300 in both positions; this change deliberately keeps those consts as places |
| Inline constant shift divisors can be misclassified (`u8(10)/(u8(128)>>8)`) | Both base and head trap; the const-name form was the new regression and is fixed |
| `const U8W: u8 = u8(200)+u8(100);` fails the strict C build | Both base and head fail the same way; the const stays a place after the fix |
| `ConstFolder` does not round an `f32` cast | Latent before this change; the reason floats stay places |
| Enum auto-increment wraps in the folder but not the emitter | Base emits `E_B = 256` while the folder wraps to 0; enum-derived consts stay places |

## Review Rulings

| Source | Finding | Ruling | Evidence |
|---|---|---|---|
| Requirements preflight | PF-REQ-001–027 | Resolved; see [Requirements Preflight](00-preflight-report-requirements.md) | All critical/major closed |
| Plan preflight | PF-PLAN-001–031 | Resolved; see [Plan Preflight](00-preflight-report.md) | All critical/major closed |
| Correctness review | RV-001/RV-002 resolved; second-round volatile/f32/unsigned/zero findings resolved; final focused re-review pending | `build/perf/verify.log` |
| Memory-safety review | MS-001–MS-004 resolved; second-round unsigned-divisor and DerefTemp findings resolved; final focused re-review pending | probes under `build/` |
| Emission-integrity review | F1–F8 resolved; second-round zero-divisor/volatile/f32 findings resolved; final focused re-review pending | base/head emission diff |
| Performance review | PE-001–PE-006 resolved; the byte non-constant loop uses 64 | base/head JSON |
| Semantics review | SR-001–SR-007 resolved; unsigned unary and binary/float zero fixed; final focused re-review pending | probes under `build/` |
