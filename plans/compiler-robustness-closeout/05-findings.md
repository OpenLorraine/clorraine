# Findings: Compiler Robustness Closeout

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Baseline**: `96ae0d4` (code-identical to `02877f2`)
> **Last Updated**: 2026-11-07

## Baseline Evidence

| Item | Value |
|---|---|
| `make check` baseline | `123.78 s` (log: `build/robustness/logs/make-check-baseline.log`) |
| Pre-PLAN-14 base | `build/robustness/base-0ac87d3e-clc`, sha256 `7ce743a80a6e08ffbbfc4cd649826a41b717d63b00cac25d57f512cf613021f8` |
| Plan baseline | `build/robustness/base-96ae0d4-clc`, sha256 `06ae4eda3cf0029f5ef610f21c00e39f498a1d2c1eb1fa8358a8992c4403ece0` |
| ASan suites at baseline | 26 |
| Added fast-gate targets | `fuzz-test`, `fuzz-smoke` |

## Defect Evidence

### D1 — Fill operand guards

| Item | Evidence |
|---|---|
| Red (bounds) | `a: 0`; ASan `stack-buffer-overflow` |
| Red (division) | `Floating point exception` |
| Red (float) | `f: 44` |
| Red (struct) | `s: 0` |
| Red (module count) | two guards before `[0]`, five for four elements |
| Fix | `init_fill_guards_emitted`; pure branch; module array dedup; struct-literal walk; conditional arm re-enable |
| Green | spec 13/13, impl 11/11; focused `RDC_ASAN` 13/13; `make check` green |
| Review | correctness RV-001 (module struct duplicate) fixed and re-reviewed clean; memory-safety MS-001/MS-002 fixed, 87 probes, 0 ASan/LSan reports |

### D2/D4 — Narrow wrapping

| Item | Evidence |
|---|---|
| Red | base: `const: 300 400 40000 -1 256 65537`; `u16*u16` strict build fails; `u16<<15` is 2147450880; signed `i16` negative products fail the strict build |
| Fix | width-aware `unsigned int` intermediate for signed and unsigned narrow `+ - *`; narrow `<<` result cast; 32-bit spelling unchanged |
| Green | spec 23/23, impl 16/16; `make check` and `make differential` green; corpus diff attributable (string-parser, volatile-mmio, imageviewer, paddle) |
| Review | RV-001/EI-001 signed `i16` overflow fixed; re-review clean; RV3-001/RV3-002/ EI2-001 test gaps closed with const cases; 3,760-case matrix, host + m68k + vbcc + freestanding clean |

### D3 — Inline shift divisor

| Item | Evidence |
|---|---|
| Red | `build/robustness/base-0ac87d3e-clc` traps `E11210` for the literal call-argument and local shapes |
| Head | `q: 0 0 0 0 0 0` |
| Fix | none; regression test and evidence; `RDC_CLC` helper |
| Green | spec case green; genuinely-zero divisor still traps |
| Review | included in the D2/D3 phase review; D3 call-argument case red on base, green on head |

### D4 — Narrow const strict build

| Item | Evidence |
|---|---|
| Red | `-Werror=overflow` on `const U8W: u8 = u8(200) + u8(100)` |
| Fix | shared with D2 |
| Green | strict host/m68k/vbcc/freestanding builds green; `const U8W` reads 44 |

### D5 — `f32` rounding and constant float overflow

| Item | Evidence |
|---|---|
| Red (cast) | base: `117 116` |
| Red (declared) | base: `117 116` |
| Red (NaN) | base: accepted, `#define E_A 0` |
| Red (range) | base: accepted; fractional boundary accepted |
| Fix | `round_float`, untruncated range/NaN `E10202` with the registered message, float result scalar, declared width, per-op rounding |
| Green | spec 55/55, impl 23/23, consts unit 35/35, `make check` green |
| Review | Reviewed with D6; correctness/emission/semantics findings fixed |

### D6 — Enum values and conversion

| Item | Evidence |
|---|---|
| User ruling | E10202 for an auto-increment beyond the backing type (AR-11) |
| Red (explicit) | `#define E_A 0` |
| Red (overflow) | accepted with `E_B 256` / folder 0 |
| Red (conversion) | `Direction(2)` C build error |
| Red (cross-module) | consumer module folds the declaring const |
| Red (ABI sizeof) | the macro and the runtime `sizeof` disagree; a cached value-context `sizeof` hides the overflow |
| Fix | shared `enum_member_values` (declaring module, cached prefix, ABI sizeof, layout-keyed const cache), sema fit check, checker-recorded cast sites, conversion lowering |
| Green | spec 55/55, impl 23/23, `make check` green |

## Emission Audit

Base `build/robustness/base-96ae0d4-clc` (sha256 `06ae4eda…`) vs head
`target/debug/clc`, over `spec/tests/corpus`, `benchmarks`, `examples`,
`tools/fuzz/seeds`, and `tools/perf/fixtures`, host and hosted-amiga. Command:
`build/robustness/emission-audit.sh`; log `build/robustness/logs/emission-audit.log` and the identical `emission-audit-final2.log`.
Result: **0 status differences**; every emitted-C difference is one of the
attributed fixes.

| Program | Profile | Difference | Attribution |
|---|---|---|---|
| `spec/tests/corpus/string-parser` | host, hosted-amiga | `u8` subtraction result cast | D2 |
| `spec/tests/corpus/volatile-mmio` | hosted-amiga | `u16` subtraction result cast | D2 |
| `examples/amiga/imageviewer` | hosted-amiga | `u8` shift result cast | D2 |
| `examples/amiga/paddle` | hosted-amiga | `u8` addition result cast | D2 |
| `examples/robustness/fill-bounds` | host, hosted-amiga | bounds guard added before the fill | D1 |
| `examples/robustness/fill-divisor` | host, hosted-amiga | division guard added before the fill | D1 |
| `tools/fuzz/seeds/narrow_arith`, `narrow_fill` | both | narrow result casts | D2 |
| `tools/fuzz/seeds/fill_bounds`, `fill_divisor`, `fill_float`, `struct_fill` | both | fill guard before the stores | D1 |
| `tools/fuzz/seeds/enum_extent` | both | `E_A 0/E_B 1` → `E_A 5/E_B 6` | D6 |
| `tools/fuzz/seeds/f32_round` | both | rounded `f32` extent | D5 |
| `tools/perf/fixtures/shapes` | both | narrow consts/fill (D2), fill guard (D1), `f32`/enum shapes (D5/D6) | D1/D2/D5/D6 |

No change appears in `benchmarks/`, `spec/tests/staged/`, the frozen compile-fail
set, or any unrelated example. The differential gate confirms the corpus
stdout is unchanged.

## Gate Evidence

| Gate | Result | Log |
|---|---|---|
| `make check` baseline | green, 123.78 s | `build/robustness/logs/make-check-baseline.log` |
| `make asan` | green, 27 suites | `build/robustness/logs/asan-final2.log` |
| `make ubsan` | green, 2 suites, activation self-test, 9.91 s | `build/robustness/logs/ubsan-d1.log`, `ubsan-d1b.log` |
| `make corpus-amiga` | green, all portable programs match | `build/robustness/logs/corpus-amiga-final.log` |
| `make differential` | green, 3/3 match | `build/robustness/logs/differential-d6.log` |
| `make bench-shape` | green: GCC nix20/nix13, vbcc, freestanding | `build/robustness/logs/bench-shape-d7d.log` |
| `make verify` | green: `verify: all legs passed` (re-gated after the final semantics fix) | `build/robustness/logs/verify-final2.log` |

## Fuzzer Evidence

| Item | Result |
|---|---|
| `make fuzz-test` | 28 unit tests green (determinism across processes, SHA-256 vector, family distribution, extents, fill shapes, seeds/goldens, classifier, sanitizer leak, safety) |
| `make fuzz-smoke` | 28 cases: host 28, sanitizer 16, target 16, 0 failures, 30.19 s |
| `make check` with the smoke | green; 151.03 s total in the measured run (+27.25 s over the 123.78 s baseline); within the 60 s budget |
| `make fuzz` (full) | 268 cases: host 268, sanitizer 268, target 44, 0 failures, 154.29 s |
| Base negative control | `--clc base-96ae0d4-clc` seed run: 8 failures covering D1 (4 seeds), D2 (invalid-C), D5 (`117 116`), D6 (`v: 0 1 6 1`); D3 passes because that base already has the D3 fix |
| Matrix | smoke: all seeds host+sanitizer+m68k, generated 16 host / 4 sanitizer / 4 m68k; full: all seeds + 256 generated host+sanitizer, every 8th m68k |
| Artifacts | successes removed; failures retained under `build/fuzz/failures/<class>-<case>/` with source, C, per-leg logs (`command:`, exit, streams), and `case.json` |

`make ubsan` measured 9.91 seconds on the reference container (warm build)
including the activation probe and two suites. It is kept verify-only per
R6.5/AR-16: the fast gate is already dominated by the Rust suite, and the
measured cost, while bounded, is not needed on every edit. The suite-level
controls prove the child builds are sanitized even when the runner's compiler
works.

## Final Verdict

All six defects are closed:

| Defect | Fix | Test |
|---|---|---|
| D1 fill guards | guard emitted before the first store on every path; module fill dedup | spec/impl + ASan/UBSan + target fault probes |
| D2/D4 narrow wrapping | defined `unsigned int` intermediate; narrow shift result cast | value matrix, strict builds, UBSan, emission audit |
| D3 inline shift divisor | no code change; regression and base evidence | spec/impl + pre-PLAN-14 trap |
| D5 f32 and constant float overflow | per-op/cast rounding, untruncated E10202 range check | spec + unit + message + fractional boundaries |
| D6 enum values and conversion | shared declaring-module ABI walk, E10202 fit, checker-recorded cast sites | spec/impl + shadow, backward, cross-module, ABI sizeof |
| Known limits | imported type names, inexact casts, non-integer members, runtime handle, case-label consts, MIN/-1, and the D1-adjacent shapes are documented | `docs/known-limitations.md` §8 |

`make verify` prints `verify: all legs passed`; `make asan` is 27 suites;
`make ubsan` is 2 suites; the fuzzer smoke is 28 cases / 0 failures in
`make check`; the full fuzzer run is 268 cases / 0 failures. The working tree
is clean and `origin/main` is up to date after the closeout push.

## Review Verdicts

| Phase | Reviewers | Verdict |
|---|---|---|
| Requirements | 2 independent auditors | 36 findings resolved; user E10202 ruling |
| Plan | 2 independent auditors + focused re-scan | 45 + 14 findings resolved |
| D1 | correctness reviewer + memory-safety auditor | RV-001 fixed; MS-001/MS-002 fixed; re-review clean; 87 ASan/LSan probes, no report |
| D2/D3 | correctness + memory-safety + emission-integrity | RV-001/EI-001 fixed; re-review clean; 3,760-case matrix, UBSan clean, corpus diff attributable |
| D4/D6 | correctness + emission-integrity + semantics | 4 review rounds; the ffi/global/import shadow, same-enum backward ref, cross-module const, ABI sizeof, layout cache, fit order, and inexact-cast findings fixed; re-review clean |
| Fuzzer | correctness reviewer | 3 majors (leg counters/matrix tests, float bound, diversity test) and the minors fixed; 28 unit tests, smoke 28/0, full 268/0 |
| Final | correctness + memory-safety + emission-integrity + semantics | No critical/major code finding; the final semantics inexact-cast finding was fixed; docs/evidence finalized; `make verify` green |

## Residual Limitations

| Limitation | Reason |
|---|---|
| A statement pure conditional fill repeats its condition expression per element | Pre-existing evaluation-order shape; the guard fix does not hoist it. The module path hoists the condition. Recorded in `docs/known-limitations.md`. |
| A module fill that covers no slot with an impure or struct value emits an unused prepared local | Pre-existing strict-build limitation found during the D1 review (base and head); outside the guard fix. Recorded in `docs/known-limitations.md`. |
| A fixed-array row-place field emits the row bounds guard twice | Pre-existing duplicate guard in the row-copy path (base and head); no memory-safety impact. Recorded as a code-size limit. |
| Imported enum and handle type names are not recognized as casts (`Buffer(0)`, imported `E(2)`) | Pre-existing sema name-resolution limit; the enum conversion path is fixed for a name the sema types as a cast. Recorded in `docs/known-limitations.md`. |
| Type-name calls with zero, multiple, or explicit type arguments are accepted (`E()`, `E(1,2)`, `E<T>(2)`) | Pre-existing cast-arity validation gap; recorded in `docs/known-limitations.md`. |
| A non-integer explicit enum member (`A = true`, `A = 1.5`) is accepted and emitted as its ordinal | Pre-existing EN-3 validation gap; recorded in `docs/known-limitations.md`. |
| A runtime integer converted to a handle fails the host strict build | Pre-existing host `int-to-pointer-cast` limit; recorded in `docs/known-limitations.md`. |
| A computed module `const` used as a `case` label emits the mangled symbol | Pre-existing constant-label limit; recorded in `docs/known-limitations.md`. |
| A constant `MIN / -1` leaves the undefined operation after the guard | Pre-existing emission limit; the guard traps first. Recorded in `docs/known-limitations.md`. |
| The fuzzer's differential cannot detect a shared host/m68k wrong value | The committed seed goldens and the spec suites are the value oracle. |
| The fuzzer is a bounded template grammar, not coverage-guided | No new dependency; determinism is required for the gate. |

## Rejected Candidates

| Candidate | Why rejected |
|---|---|
| Materializing float constants | A storage/perf change outside the correctness closeout |
| Wrapping enum auto-increment instead of E10202 | User ruled E10202 |
| Keeping 256 for an out-of-range enum member | Unrepresentable in the backing type; invalid C99 store |
| Adding ASan to `make verify` | It is already a separate recorded gate; adding 27 suites changes acceptance runtime without a measured need |
