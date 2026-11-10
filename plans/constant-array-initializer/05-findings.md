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
| lookup | 13,653 | 117,440 | 474,430 | 0.65 ms |
| sieve | 1,040,581 | 76,728 | 311,284 | 0.75 ms |
| matrix | 10,959 | 80,712 | 313,817 | 0.63 ms |
| strings | 2,964,051 | 77,604 | 312,685 | 1.20 ms |
| moves | 45,205 | 76,896 | 313,173 | 0.67 ms |

### Compiler identities

| Role | Path | SHA-256 | Commit | Version line | Dirty |
|---|---|---|---|---|---|
| Base (`v1.1.0`) | `build/perf/base-clc` | `eb0e2dae6049698dbd9aa5c77e7b2663c117893b13f66875257e44467199b0da` | `c0bd6d05fe2a985874fddcce9cd07d5100b89e2f` | `clc 1.1.0` | false |
| Isolated base (benchmark commit) | `build/perf/isolated-base-clc` | `9f57815412bc322935da1c3ad284ba52801a7f94b666efd4ea705fb29c77c7e8` | `e2d3c760bb8fa23d3434bbc3d7e7be5289867978` | `clc 1.1.1` | false |
| Final head (feature commit) | `target/debug/clc` | `08efe2e350bf179f862b0767663796cabf7fdb3aa55e866a171ee40b564fab37` | `82877aba573bd0d0619f7130270eecbcb71359a9` (implementation `cef0b91`, fix `fd7fe30`, exact pins `866104c`) | `clc 1.1.1` | false |

**Base compiler source:** `v1.1.0` commit `c0bd6d05fe2a985874fddcce9cd07d5100b89e2f` (annotated tag object `c0e34568f423a1ff2994dddd6f2f654e50f338d6`) — record the exact full commit
and the `clc --version` line.

**Isolated-base compiler:** the compiler built from the benchmark commit,
retained as `build/perf/isolated-base-clc`; the v1.1.0 `base-clc` is a
separate cumulative-base artifact.

**Commands:** `make bench-base COMMIT=v1.1.0`, `make bench`, the isolated
copy, and the isolated comparison.

**Baseline execution note:** `make bench-base COMMIT=v1.1.0` built and retained
`build/perf/base-clc` (`clc 1.1.0`). Its first full harness run then hit a
stale-worktree race while a previous worktree was being removed; the base
harness was rerun directly with the retained `build/perf/base-clc` and the
same `c0bd6d05...` commit identity, writing the recorded `base.json`. No
compiler or benchmark source changed between the two runs.

### Threshold measurement

Phase 1 measures a small-table boundary with the isolated-base compiler and
records the final lower bound here. The test matrix uses 32×`i32` (128 bytes),
64×`u8`/`i8`/`bool` (64 bytes), 32×`i16`/`u16` (64 bytes), 16×`u32`
(64 bytes), and 32×`usize`/`isize` (128 bytes), all at or above the
provisional 16-element / 64-byte floor.

| Shape | Elements | Bytes | `.text` vs stores | Instructions vs stores | Verdict |
|---|---:|---:|---:|---:|---|
| `i32[4]` | 4 | 16 | +20 | -1 | below floor |
| `i32[8]` | 8 | 32 | +44 | -1 | below floor |
| `i32[16]` | 16 | 64 | -20 | -14 | 4-byte floor |
| `i32[32]` | 32 | 128 | -80 | -27 | test floor |
| `i32[64]` | 64 | 256 | -212 | -57 | test floor |
| `u8[64]` | 64 | 64 | -48 | +55 | 1-byte exclusion |
| `u8[128]` | 128 | 128 | -148 | +107 | 1-byte exclusion |
| `u8[256]` | 256 | 256 | -340 | +211 | 1-byte exclusion |
| `u16[32]` | 32 | 64 | -52 | +2 | 2-byte floor |
| `u16[64]` | 64 | 128 | -148 | +5 | 2-byte included |
| `u16[128]` | 128 | 256 | -344 | +7 | 2-byte included |
| `i16[32]` | 32 | 64 | -56 | +2 | 2-byte floor |
| `i16[64]` | 64 | 128 | -148 | +5 | 2-byte included |
| `i32[256]` (all ones) | 256 | 1024 | -480 | +19 | sensitivity wash |
| `i32[1024]` | 1024 | 4096 | -6,148 | +63 | code-size-only band |
| `i32[1280]` | 1280 | 5120 | -22.75% | -1,268 | GCC bulk-memcpy win |

**Final threshold:** at least 16 elements and 64 bytes for 2-byte and 4-byte
elements. The 4-byte shape wins both metrics at 64 bytes; the 2-byte shape
wins `.text` with a ≤0.2% instruction cost; the 1-byte shape (`u8`/`i8`/
`bool`) is excluded because its measured runtime cost is +55/+107/+211
instructions at 64/128/256 bytes (above the 1% guard). The ST cases use
`i32[32]` (128 bytes), `u32[16]` (64), `u16[32]` (64), `i16[32]` (64),
`usize[32]` (128), and `isize[32]` (128); the 1-byte tables are old-store
controls.

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
the in-repo `cube` example uses a 256-entry display table. The Phase 1
sensitivity probe (an `i32[256]` table, all ones) measures `.text` -480 and
instructions +19: the code-size win exists at the cube’s scale, while the
runtime is a sub-1% wash; the committed 4,096-entry sine table wins both
metrics (-24,464 `.text`, -4,183 instructions). The benchmark is committed
before the codegen change and its source is not modified afterward.

The checksum is a plain sum of an offset sine table. It detects a skipped,
zeroed, or partially copied template, but it is not permutation-sensitive; the
behaviour and emission suites cover the exact element values and order.

## Head and Delta

Filled after the implementation and each fix pass.

### Instructions

| Benchmark | Base `v1.1.0` | Isolated base | Head | Cumulative delta | Isolated delta | Output |
|---|---:|---:|---:|---:|---:|---|
| lookup | 13,653 | 13,653 | 9,470 | -30.6% | -30.6% | equal |
| sieve | 1,040,581 | 1,040,581 | 1,040,581 | 0.0% | 0.0% | equal |
| matrix | 10,959 | 9,908 | 9,908 | -9.6% | 0.0% | equal |
| strings | 2,964,051 | 2,964,051 | 2,964,051 | 0.0% | 0.0% | equal |
| moves | 45,205 | 45,205 | 45,205 | 0.0% | 0.0% | equal |

### `.text` and C bytes

| Benchmark | Base `.text` | Isolated `.text` | Head `.text` | Isolated delta | Base C | Isolated C | Head C | Isolated C delta |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| lookup | 117,440 | 117,440 | 92,976 | -20.8% | 474,430 | 474,430 | 340,503 | -28.2% |
| sieve | 76,728 | 76,728 | 76,728 | 0.0% | 311,284 | 311,284 | 311,284 | 0.0% |
| matrix | 80,712 | 76,676 | 76,676 | 0.0% | 313,817 | 311,168 | 311,168 | 0.0% |
| strings | 77,604 | 77,604 | 77,604 | 0.0% | 312,685 | 312,685 | 312,685 | 0.0% |
| moves | 76,896 | 76,896 | 76,896 | 0.0% | 313,173 | 313,173 | 313,173 | 0.0% |

**Head identity:** record the feature commit, the clean/dirty flag, and the
`clc` SHA-256. The head document is valid only if its commit equals the
feature head and `dirty` is false.

**Comparison requirements:** `lookup:instructions:10 lookup:text:10`
(re-derived in Phase 1 if the committed measurements differ). Both the
cumulative and isolated comparisons pass the same requirement.

**Comparisons:** isolated compare passed (`lookup` -30.6% instructions,
-20.8% `.text`, C bytes -28.2%; every other row 0.0%). Cumulative compare
passed (`lookup` -30.6% / -20.8%; `matrix` -9.6% / -5.0% from the already-landed
nested-fill work; no scored regression). Both requirement rows pass. The head
document records the feature head `82877ab` with `dirty=false`; the compiler
source is implementation `cef0b91` plus the collision fix `fd7fe30` and the
exact-cast pins `866104c`.

## Red/Green Pin Record

| Status | Pins |
|---|---|
| Red on the pre-change compiler | `constant_array_initializer_spec_test`: 42 passed. `constant_array_initializer_impl_test`: 16 passed, 8 failed: `should_emit_one_template_and_memcpy_for_i32`, `should_emit_negative_and_typed_values`, `should_emit_u16_and_i16_templates`, `should_emit_usize_and_isize_templates`, `should_emit_the_template_for_each_hosted_profile`, `should_use_distinct_templates_and_keep_the_target_declaration`, `should_keep_the_memcpy_at_the_initializer_position`, `should_wrap_the_template_at_eight_values_per_line`. |
| Green on the feature head | `constant_array_initializer_spec_test`: 42 passed. `constant_array_initializer_impl_test`: 32 passed. `guard_free_constant_tests`: 3 passed. The head `lookup` probe is 12,842 instructions and 92,976 `.text` with equal output. |
| Exclusion guards green on both | 16 implementation exclusions passed on the pre-change compiler; the 42 behaviour cases passed. |

**Red command and identity:** `make -C docker exec CMD="RDC_CLC=/work/build/perf/isolated-base-clc
cargo test --test constant_array_initializer_impl_test -- --test-threads=1"`
with the retained pre-change `clc` (`e2d3c760...`, `clc 1.1.1`); the positive
pins fail because the base emits per-element stores. The same command on the
feature head is the green run recorded below.

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
benchmarks/` is empty. Source SHA-256:
`program.clo c888c782e06513ef685fe66d9d823f152dabb027bc8f6f40460a96d881d46f0f`;
`expected-output.txt 7675c31262c1d7a200445d4a25fc989f450e5c23b1e7f6b9adc2f45c0e78a445`.

## Accepted Limits

| Limit | Reason | Requirement |
|---|---|---|
| Module-level `let` only | A `const` explicit list is `E10224`; supporting it is a checker/language change | R2.1 |
| Complete explicit list only | The current stores leave an omitted tail at the global’s existing state; the extent-sized template would zero-fill it, and the tail policy is unmeasured | R2.1 |
| No fill or multi-extent shape | Existing fill compaction and multi-extent paths cover them | R2.4 |
| No float, struct, handle, owned, or view element | Separate semantics; no measured need | R2.4 |
| Guard-free elements only | `MIN / -1`, float casts, and short-circuit forms carry guards or unevaluated operands | R2.8 |
| At least 16 elements and 64 bytes for 2/4-byte elements | Measured lower bound; the 4-byte shape wins both metrics at 64 bytes, the 2-byte shape wins `.text` with a ≤0.2% instruction cost | R2.1, AR-11 |
| 1-byte elements keep the stores | The measured u8 template costs +55/+107/+211 instructions at 64/128/256 bytes, above the 1% guard | R2.4, AR-11 |
| `freestanding` keeps the stores | Byte-wise shim `memcpy`; no measured freestanding win | R2.1 |
| Read-only template beside the writable global | The total binary is still smaller than the store run; the scored `.text`/instructions improve | R2.3 |
| `__cl_itmp<N>` user-FFI shadowing is pre-existing | The same collision class exists for init locals; outside the constant-array template change and not reached by the repository programs | Recorded residual |
| 2–5 KB 4-byte tables are code-size-only | At i32[1024] the template is +63 instructions (+1.18% in an init-dominated run) for -6,148 `.text` (-7.1%); at i32[1280] it is -1,268 instructions and -22.75% `.text`; the scored `lookup` 4,096-entry table wins both, and the 1% guard applies to scored benchmarks | R2.1, AR-11 |
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

### Phase 3 reviews (implementation `cef0b91`)

| Review | Verdict | Findings |
|---|---|---|
| Correctness | clean of critical/major | RV-001 (MINOR): the division/remainder, conditional, full-prefix-fill, and byte-floor pins did not isolate their guards; RV-002 (MINOR): no exact typed-extreme or init-local pins, no Call/Intrinsic predicate cases; RV-003 (MINOR): the `init_templates` doc said `__cl_ctmpl<N>` instead of `__cl_ctmpl_<n>`. |
| Memory safety | **no findings** | Focused ASan: 42 spec + 24 impl + 66 module-initializer + 25 nested-fill + 42 array-value cases clean; 30 probes and 3 front-end rejects clean; leak/OOB controls fired; head/base output equal. Full `make asan`/`make ubsan` run in Phase 4. |
| Emission integrity | clean of critical/major | F-001 (MINOR): a user `ffi` symbol named `__cl_ctmpl_0` could be shadowed by the block-scope template; base compiles, head introduced the collision. All other 82 units byte-identical; only `lookup` and the shape fixture changed; 136/136 strict builds. |
| Performance | clean of critical/major | PE-001 (MINOR): the 4-byte threshold admits a 2–5 KB band that is code-size-only at i32[1024] (+63 instructions, -6,148 `.text`); PE-002 (MINOR): the threshold table omitted the 64-byte 2-byte boundary and denominators; PE-003 (MINOR): the benchmark-source hash/diff record was still a placeholder. |

**Resolutions:** F-001 fixed by advancing the template counter past any
`is_taken_symbol` collision, with the `should_not_shadow_a_user_ffi_template_name`
regression pin; RV-003 fixed in the `codegen.rs` doc; RV-001/RV-002 addressed
by the Phase 4 hardening pins (constant Binary/conditional, full-prefix fill,
`u16[16]` byte floor, exact cast spellings, init-local/template distinctness,
Call/Intrinsic predicate cases); PE-002/PE-003 recorded below. PE-001 is
accepted as a code-size-only trade-off for the 2–5 KB band, recorded in the
accepted limits; the scored `lookup` table wins both metrics and the 1% guard
applies to scored benchmarks.

**Phase 4 focused re-reviews (`cef0b91..fd7fe30`):** correctness, memory
safety, emission integrity, and performance each re-reviewed the fix diff.
No critical/major finding. The correctness reviewer added RV-004 (exact cast
spellings missing); the follow-up test commit `866104c` closes it. Memory
safety re-ran the focused ASan recipe and the collision probes clean; emission
integrity found 82/82 units byte-identical except the intended template and
confirmed the collision probe now compiles and runs on host/m68k/vbcc;
performance confirmed the benchmark emission and metrics are unchanged.

**Final focused re-review (Phase 5):** clean of critical/major. RV-005
(test completeness: the exact-cast case used short lists) was fixed by
`4e6e1d1`, making the u16/usize/isize tables full 32-entry templates and
asserting the template. The final acceptance evidence is green:
`make check`, `make asan` 30 suites, `make ubsan` 4 suites, `make corpus-amiga`,
`make differential` 3/3, `make bench-shape`, both benchmark comparisons, and
`make verify` (`verify: all legs passed`).

**Pre-existing outside this change:** `__cl_itmp<N>` init locals have the same
theoretical user-FFI shadowing class (`compiler/src/codegen/program.rs`); this
predates the feature, is not reached by the corpus/benchmarks/examples, and is
recorded as a residual defect rather than fixed here.

## Final Acceptance

| Gate | Result |
|---|---|
| `make check` | green: 130 test-result groups, including 32 impl + 42 spec; fuzz smoke 28/28 host, 16/16 sanitizer/target, 0 failures |
| `make asan` | green: 30 suites, including the 42-case constant-array suite |
| `make ubsan` | green: 4 suites, including the 42-case constant-array suite |
| `make corpus-amiga` | green: all portable programs match on hosted-amiga; vbcc/build-only legs warning-free |
| `make differential` | green: 3/3 programs match |
| `make bench-shape` | green: hosted-amiga, nix13, vbcc, and freestanding warning-free |
| `make bench` (full suite, head commit, `dirty=false`) | green: head `82877ab`, `dirty=false`; `lookup` 9,470 / 92,976 |
| isolated `bench.py compare --require ...` | passed: `lookup` -30.6% instructions, -20.8% `.text`; every other row 0.0% |
| `make bench-compare REQUIRE="lookup:instructions:10 lookup:text:10"` | passed: `lookup` -30.6% / -20.8%; `matrix` -9.6% / -5.0% from prior work; no scored regression |
| `make verify` | green on the final run: `verify: all legs passed`. Two earlier runs hit environmental FS-UAE screen-capture flakes (paddle, text); each direct rerun passed and the emitted C is byte-identical base/head; the corpus-amiga vbcc transient is fixed by the bounded retry `ac92308`. |
| targeted `lookup` smoke (`build/perf/lookup-smoke.json`) | passed on the pre-change compiler: output `lookup: 134217728`, 13,653 / 117,440; the final full-suite head run records the same output at 9,470 / 92,976 |

## Residual Opportunities

| Opportunity | Evidence | Disposition |
|---|---|---|
| Function-local constant tables | Separate path; no benchmark | Residual |
| 1-byte tables | The measured template cost is above 1%; a store-packing-aware rule could revisit | Residual |
| 2–5 KB 4-byte band | Code-size-only at i32[1024]; a size-aware cap could skip the band | Residual |
| `__cl_itmp` user-FFI shadowing | Pre-existing collision class outside this change; not reached by the repository programs | Residual defect |
| Short/empty lists | Tail policy needed | Residual |
| Fill forms with constant values | Existing compaction | Residual |
| Module `const` arrays | `E10224` | Out of scope |
| Freestanding multi-byte templates | Byte-wise `memcpy` | Out of scope |
| Linker DCE | Toolchain change | Out of scope |
