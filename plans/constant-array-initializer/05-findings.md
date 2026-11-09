# Findings and Evidence: Constant Array Initialization (PLAN-25)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

## Purpose

This ledger owns the committed baseline/head measurements, the probe evidence,
the red/green pin record, the emission audit, the accepted limits, the rejected
candidates, the review findings, and the final acceptance table. The
exploratory study stays in [Current State](02-current-state.md); the
requirements and design stay in [Requirements](01-requirements.md) and
[Design](03-design.md).

## Committed Baseline

Filled by Phase 1 after the benchmark commit, `make bench-base
COMMIT=v1.1.0`, and `make bench`. Do not reuse a stale `build/perf` document;
remove the old files first and check the identity.

| Benchmark | `instructions_net` | m68k `.text` `-O2` | C bytes | Host `-O2` median |
|---|---:|---:|---:|---:|
| lookup | _pending_ | _pending_ | _pending_ | _pending_ |
| sieve | _pending_ | _pending_ | _pending_ | _pending_ |
| matrix | _pending_ | _pending_ | _pending_ | _pending_ |
| strings | _pending_ | _pending_ | _pending_ | _pending_ |
| moves | _pending_ | _pending_ | _pending_ | _pending_ |

### Compiler identities

| Role | Path | SHA-256 | Commit | Version line | Dirty |
|---|---|---|---|---|---|
| Base (`v1.1.0`) | `build/perf/base-clc` | _pending_ | `c0e34568f423a1ff2994dddd6f2f654e50f338d6` | `clc 1.1.0` | false |
| Isolated base (benchmark commit) | `build/perf/isolated-base-clc` | _pending_ | _pending_ | _pending_ | false |
| Final head (feature commit) | `target/debug/clc` | _pending_ | _pending_ | _pending_ | false |

**Base compiler source:** `v1.1.0` (`c0e34568f423a1ff2994dddd6f2f654e50f338d6`) — record the exact full commit
and the `clc --version` line.

**Isolated-base compiler:** the compiler built from the benchmark commit,
retained as `build/perf/isolated-base-clc`; the v1.1.0 `base-clc` is a
separate cumulative-base artifact.

**Commands:** `make bench-base COMMIT=v1.1.0`, `make bench`, the isolated
copy, and the isolated comparison.

### Threshold measurement

Phase 1 measures a small-table boundary with the isolated-base compiler and
records the final lower bound here. The test matrix uses 32×`i32` (128 bytes),
64×`u8`/`i8`/`bool` (64 bytes), 32×`i16`/`u16` (64 bytes), 16×`u32`
(64 bytes), and 32×`usize`/`isize` (128 bytes), all at or above the
provisional 16-element / 64-byte floor.

| Shape | Elements | Bytes | `.text` vs stores | Instructions vs stores | Verdict |
|---|---:|---:|---:|---:|---|
| `i32[4]` | 4 | 16 | _pending_ | _pending_ | well below floor |
| `i32[8]` | 8 | 32 | _pending_ | _pending_ | below-floor control |
| `i32[16]` | 16 | 64 | _pending_ | _pending_ | candidate floor |
| `i32[32]` | 32 | 128 | _pending_ | _pending_ | test floor |
| `u8[64]` | 64 | 64 | _pending_ | _pending_ | test floor |

The final threshold is recorded after this table; the ST cases are sized from
it before Phase 2. The 64-byte cases are at the floor, not “safely above”; the
32-element `i32`/`usize`/`isize` cases are above it, and the `u32` case is 16
elements (64 bytes).

## Exploratory Probe

The probe is recorded here for attribution; Phase 1 re-runs it with the
committed harness.

| Metric | Current | Template + `memcpy` | Delta |
|---|---:|---:|---:|
| `lookup` raw instructions | 17,025 | 12,842 | -24.6% |
| `lookup` net instructions (minus 3,372) | 13,653 | 9,470 | -30.6% |
| `lookup` m68k `.text` | 117,440 | 92,976 | -20.8% |
| Output | `lookup: 134217728` | `lookup: 134217728` | equal |

**Probe method:** emit `benchmarks/lookup/program.clo` for `hosted-amiga`
with the pre-change compiler; replace the 4,096 stores in
`cl_init_bench_Lookup` with one block-scope `static const i32
__cl_ctmpl_0[4096]` and one `memcpy`; build both with `m68k-amigaos-gcc
-std=c99 -mcrt=nix20 -m68000 -fwrapv -O2`; count `instr:` lines from
`vamos --instr-trace`; compare `m68k-amigaos-size` `.text`.

### Benchmark sensitivity

The committed table is 4,096 entries, a realistic audio/graphics table size;
the in-repo `cube` example uses a 256-entry display table. Phase 1 also records
a 256-entry sensitivity probe of the same shape (without changing the committed
benchmark) to show the `.text`/instruction win is not an artifact of the 4,096
size. The benchmark is committed before the codegen change and its source is
not modified afterward.

The checksum is a plain sum of an offset sine table. It detects a skipped,
zeroed, or partially copied template, but it is not permutation-sensitive; the
behaviour and emission suites cover the exact element values and order.

## Head and Delta

Filled after the implementation and each fix pass.

### Instructions

| Benchmark | Base `v1.1.0` | Isolated base | Head | Cumulative delta | Isolated delta | Output |
|---|---:|---:|---:|---:|---:|---|
| lookup | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| sieve | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| matrix | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| strings | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| moves | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |

### `.text` and C bytes

| Benchmark | Base `.text` | Isolated `.text` | Head `.text` | Isolated delta | Base C | Isolated C | Head C | Isolated C delta |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| lookup | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| sieve | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| matrix | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| strings | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |
| moves | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ | _pending_ |

**Head identity:** record the feature commit, the clean/dirty flag, and the
`clc` SHA-256. The head document is valid only if its commit equals the
feature head and `dirty` is false.

**Comparison requirements:** `lookup:instructions:10 lookup:text:10`
(re-derived in Phase 1 if the committed measurements differ). Both the
cumulative and isolated comparisons pass the same requirement.

**Comparisons:** record the exact cumulative and isolated `bench.py compare`
output, including the isolated `--require` rows.

## Red/Green Pin Record

| Status | Pins |
|---|---|
| Red on the pre-change compiler | _pending_ |
| Green on the feature head | _pending_ |
| Exclusion guards green on both | _pending_ |

**Base evidence:** the exact `RDC_CLC` command, compiler identity, and failing
pin names. **After the change:** the passing suite counts.

## Emission Audit

The isolated-base compiler and the head compiler emit the same sources except
for the `lookup` initializer template. The audit covers:

- the seven corpus programs;
- the six benchmark programs (five scored plus calibration);
- every `examples/amiga/*.clo` program, with its profile.

The cumulative diff additionally contains the already-landed nested-fill and
volatile-fill changes; those are listed separately and are not attributed to
this feature.

**Benchmark-source audit:** `git diff v1.1.0..<feature-head> -- benchmarks/`
shows the `lookup` addition; `git diff <benchmark-commit>..<feature-head> --
benchmarks/` is empty. Record the source/expected SHA-256 values.

## Accepted Limits

| Limit | Reason | Requirement |
|---|---|---|
| Module-level `let` only | A `const` explicit list is `E10224`; supporting it is a checker/language change | R2.1 |
| Complete explicit list only | The current stores leave an omitted tail at the global’s existing state; the extent-sized template would zero-fill it, and the tail policy is unmeasured | R2.1 |
| No fill or multi-extent shape | Existing fill compaction and multi-extent paths cover them | R2.4 |
| No float, struct, handle, owned, or view element | Separate semantics; no measured need | R2.4 |
| Guard-free elements only | `MIN / -1`, float casts, and short-circuit forms carry guards or unevaluated operands | R2.8 |
| At least 16 elements and 64 bytes | Provisional measured lower bound; Phase 1 finalizes it | R2.1 |
| `freestanding` keeps the stores | Byte-wise shim `memcpy`; no measured freestanding win | R2.1 |
| Read-only template beside the writable global | The total binary is still smaller than the store run; the scored `.text`/instructions improve | R2.3 |
| Host time is advisory | Run-to-run spread is tens of percent | R4.2 |

## Rejected Candidates

| Candidate | Evidence | Decision |
|---|---|---|
| Static declaration initializer that skips the init function | Changes init timing and address-taken mutation; larger semantic proof | Rejected |
| Linker `-ffunction-sections`/`--gc-sections` | Toolchain change; breaks the same-flags comparison; targets the runtime shim | Rejected |
| Whole-program dead-function elimination | Large call-graph/FFI surface | Rejected |
| Function-local constant tables | Separate declaration path; no committed benchmark | Residual |
| Short/empty list templates | The tail policy is unmeasured; an extent-sized template would zero-fill a tail the current stores leave at the existing state | Rejected for this cut |
| Module `const` arrays | `E10224`; checker/language change | Out of scope |
| Freestanding multi-byte templates | Byte-wise shim `memcpy`; unmeasured | Out of scope |
| C-byte-only gate | Runtime and `.text` both improve; C bytes are advisory | Rejected |

## Review Findings and Resolutions

Filled after each phase review. List the reviewer, verdict, and every
critical/major finding with its resolution. The final focused re-review must be
clean of critical/major findings.

## Final Acceptance

| Gate | Result |
|---|---|
| `make check` | _pending_ |
| `make asan` | _pending_ |
| `make ubsan` | _pending_ |
| `make corpus-amiga` | _pending_ |
| `make differential` | _pending_ |
| `make bench-shape` | _pending_ |
| `make bench` (full suite, head commit, `dirty=false`) | _pending_ |
| isolated `bench.py compare --require ...` | _pending_ |
| `make bench-compare REQUIRE="lookup:instructions:10 lookup:text:10"` | _pending_ |
| `make verify` | _pending_ |
| targeted `lookup` smoke (`build/perf/lookup-smoke.json`) | _pending_ |

## Residual Opportunities

| Opportunity | Evidence | Disposition |
|---|---|---|
| Function-local constant tables | Separate path; no benchmark | Residual |
| Short/empty lists | Tail policy needed | Residual |
| Fill forms with constant values | Existing compaction | Residual |
| Module `const` arrays | `E10224` | Out of scope |
| Freestanding multi-byte templates | Byte-wise `memcpy` | Out of scope |
| Linker DCE | Toolchain change | Out of scope |
