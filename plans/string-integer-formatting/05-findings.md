# Findings: String Integer Formatting Throughput

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

This document is the evidence record. It is filled in phases: the pre-plan
probe in Phase 1, the red/green record in Phase 2–3, and the final base/head
measurement in Phase 4. Values are never estimated; a missing value is
`pending`.

## 1. Base Identity

| Item | Value |
|---|---|
| Base commit | `8f07e408975d9ad090392712030d2e2761d19437` |
| Compiler version | `clc 1.1.2` |
| Base compiler SHA-256 | `232698a3dc89e8b96b9d51a7258542830d0d1c18a6f262366494c449c3b8f295` |
| Pre-change head | `828453a854341634f348f8a12def879fd96e27f8`, SHA-256 `7b7ab88b7b4ae65dddd26fe3b1e6d411fcf73c9c4d1c368a4ac711476362f3bc` |
| Worktree | clean for both measured runs; harness `dirty=false` |
| Toolchain | pinned SDK container; m68k GCC 16.2.0b, libnix `nix20`, `vamos` 0.8.1 |
| Base document | `build/perf/base.json` (commit `8f07e40`, 30 runs) |
| Pre-change document | `build/perf/pre-change-head.json` (commit `828453a`) |
| Base vs pre-change compare | passed, every metric `+0.0%` (the source is unchanged) |
| `make check` on the plan commit | passed (2026-11-11 09:50; spec, registry, setup, workspace tests, bench-test, fuzz-test, fuzz-smoke 28/0) |

## 2. Pre-Plan Probe

The baseline, attribution, and probe tables are owned by
`02-current-state.md` §Measurement Audit and §Probes. In one line: the base is
2,964,051 net instructions with 6,003 allocations; the selected direct-write
shim probe is 1,464,668 (−50.6%), 24,080 `.text` (−0.5%), and 6,003
allocations; the out-of-scope codegen probe is 1,132,051 (−61.8%) with 2,002
allocations.

Commands and artifacts (the `build/perf/audit` scratch is removed at
closeout):

```
target/debug/clc --profile hosted-amiga -o build/perf/audit/strings.c benchmarks/strings/program.clo
m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 \
    -ffunction-sections -fdata-sections -Wl,--gc-sections \
    -o build/perf/audit/strings-shimonly2 build/perf/audit/strings-shimonly2.c -lamiga -lm
vamos --instr-trace build/perf/audit/strings-shimonly2 2> build/perf/audit/shimonly2.trace
m68k-amigaos-size build/perf/audit/strings-shimonly2
```

The allocation counter was a host build of the same emission with
`cl_alloc_calls`/`cl_free_calls` counters and an `atexit` report; it printed
`AUDIT alloc=6003 free=6003` for the base and `alloc=2002` for the codegen
probe.

## 3. Red/Green Record

| Case | Base result | Head result | Note |
|---|---|---|---|
| ST-21 (`snprintf` count) | `hex=0x4` — failed as expected | `hex=0x0` — passed | the red case; zero formatter calls |
| ST-1 – ST-20, ST-22 – ST-24 | 8 tests passed | 8 tests passed | intentional no-change oracle |
| Impl pins (written after the implementation) | 2 passed, 3 failed | 5 tests passed | the three new-helper pins fail on the base; the literal-only and determinism pins pass |

Green commands: `cargo test -p clc --test
string_integer_formatting_spec_test --test
string_integer_formatting_impl_test -- --test-threads=1` (9 + 5 passed) and
`cargo test --workspace` (all suites, including the new ones).

Two existing implementation suites search the whole translation unit for the
spellings `do {` and `} else {` to pin a program-body shape. The decimal
writer avoids both spellings (the digit loop is a `while` with an
unconditional first pass, and the sign branch has no `else`), so those suites
keep their intended scope without a test change. No existing test was
modified.

Red command: `cargo test -p clc --test
string_integer_formatting_spec_test -- --test-threads=1` against the
compiler built from `828453a`; the suite reported 8 passed, 1 failed, with
`left: "hex=0x4"` against `right: "hex=0x0"`.

`make asan`/`make ubsan` discovery: both markers are present in the suite
(`grep -l RDC_ASAN compiler/tests/*.rs` and `RDC_UBSAN` include it).

## 4. Final Base/Head Measurement

Commands:

```
make bench-base COMMIT=8f07e40
make bench
make bench-compare
```

Head `be2f9d7` (`dirty=false`), base `8f07e40` (`dirty=false`).

| Benchmark | Base instructions | Head | Delta | Base `.text` | Head | Delta | Output |
|---|---:|---:|---:|---:|---:|---:|---|
| sieve | 1,040,581 | 1,040,152 | −0.04% | 23,832 | 23,720 | −0.5% | equal |
| matrix | 9,908 | 9,431 | −4.8% | 23,608 | 23,496 | −0.5% | equal |
| strings | 2,964,051 | 1,489,187 | **−49.8%** | 24,196 | 24,020 | −0.7% | equal |
| moves | 45,205 | 44,680 | −1.2% | 23,632 | 23,520 | −0.5% | equal |
| lookup | 9,470 | 8,849 | −6.6% | 39,904 | 39,792 | −0.3% | equal |

`make bench-compare`: passed, every output hash equal, every metric within the
1% guard. Emitted C bytes are +0.6% on every benchmark (advisory): the new
shim body is slightly longer than the two removed `snprintf` call sites, while
`.text` shrinks because the formatter no longer pulls in more of the library.

Allocation count on `strings`: **6,003** (unchanged), split as 3 per iteration
(two converter blocks + one builder block) and 3 outside the loop. The final
host counter build printed `AUDIT alloc=6003 free=6003` and
`strings: 0x3F9A6`.

The final `strings` count is 24,519 instructions above the pre-plan probe
(1,489,187 vs 1,464,668). The difference is the clearer loop shape: the digit
pass uses `while (value != 0 || count == 0)` and the sign branch initializes
the magnitude before the test. The measured win remains far above the 30%
floor.

## 5. Sanitizers and Cross-Profile

| Leg | Result |
|---|---|
| New suites, plain `cargo test` | spec 9/9, impl 5/5 |
| `cargo test --workspace` | passed |
| New suite under `make asan` | passed; the run discovered 31 suites including this one |
| New suite under `make ubsan` | passed; the run discovered 5 suites including this one |
| `make bench-shape` (GCC, vbcc, freestanding, nix13) | passed, all legs warning-free |
| Target boundary program under `vamos` | equal to the host output |
| Emitted-C diff base vs head | one hunk per profile; only the two converter bodies changed: `build/perf/audit/emit-base.c` vs `emit-head.c` (Amiga) at line 966, and the host emission at line 349 |
| Base-compiler attribution | spec 8 passed / ST-21 failed `hex=0x4`; impl 2 passed / 3 new-helper pins failed |
| `make check` | passed (spec, registry, setup, workspace tests, bench-test, fuzz-test, fuzz-smoke 28/0) |
| `make verify` | passed, `verify: all legs passed` (all FS-UAE, corpus, differential, fault, and shape legs) |
| Correctness review | no phase-blocking findings; RV-001 resolved; pre-existing issue #24 filed |
| Memory-safety review | no findings; bounds, ownership, ASan, and UBSan clean |

The emitted-C diff proves the program, template calls, and every other shim
section are byte-identical. The target boundary program covered `INT_MIN`,
`INT_MAX`, `UINT_MAX`, `0`, adjacent placeholders, `usize` max, and
`toDecimal`/`toHex`; its `hosted-amiga` output under `vamos` matched the host
output exactly.

## 6. Review Outcomes

| Review | Verdict | Findings |
|---|---|---|
| Correctness reviewer | no phase-blocking findings | RV-001 (🟡 MINOR, bookkeeping) resolved: the progress line now matches the checked task count, and the emission evidence names both profiles and line ranges. One pre-existing defect (P-1, `text` arity) was reproduced on the base compiler and filed as issue #24; it is not caused by this phase. |
| Memory-safety auditor | no findings | Direct-write bounds, `len`/`cap`, `INT_MIN`, ownership, ASan/LSan, and UBSan probes are clean; three negative controls (out-of-bounds, double free, leak) prove the instrumentation is active. |

The correctness reviewer independently reproduced the benchmark table, the
base red attribution, the 200,000-value `snprintf` differential, the target
boundary output, and the allocation count. The memory-safety auditor measured
the i32 block as 20 bytes and the u32 block as 19 bytes, proving the NUL sits
on the last valid byte for `INT_MIN`/`UINT_MAX`; `cl_string_append` still
traps after a conversion because `cap == len`.

## 7. Residual and Follow-Up

- Allocations per `strings` run stay at 6,003 in this plan (one builder plus
  two converter temporaries per iteration, three outside the loop). The
  logical capacity of each converter block is exact (`cap == len`); the
  physical allocation may carry up to 7 spare payload bytes, which is not
  observable through the block API.
- The codegen direct-to-destination lowering would remove two of the three
  per-iteration allocations and measured 1,132,051 net instructions on the
  probe. It is outside issue #19's scope (AR-01, AR-05) and is recorded as
  follow-up issue **#25**
  (https://github.com/OpenLorraine/clorraine/issues/25).
- The correctness review found a pre-existing `text` arity defect (zero and
  two arguments are accepted; a second argument's side effect is dropped on
  the base compiler). It is not caused by this change and is tracked as
  **#24** (https://github.com/OpenLorraine/clorraine/issues/24).
- `docs/known-limitations.md`: no change. The allocation count and the
  physical slack are not user-visible language behavior, and every observable
  conversion output is unchanged.
