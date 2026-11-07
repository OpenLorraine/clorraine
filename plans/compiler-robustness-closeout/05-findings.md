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
| Green | pending |

### D5 — `f32` rounding and constant float overflow

| Item | Evidence |
|---|---|
| Red (cast) | base: `117 116` |
| Red (declared) | base: `117 116` |
| Red (NaN) | base: accepted, `#define E_A 0` |
| Red (range) | base: accepted; fractional boundary accepted |
| Fix | `round_float`, untruncated range/NaN `E10202` with the registered message, float result scalar, declared width, per-op rounding |
| Green | spec 54/54, impl 22/22, consts unit 35/35, `make check` green |
| Review | pending with D6 (semantics lens) |

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
| Green | spec 54/54, impl 22/22, `make check` green |

## Gate Evidence

| Gate | Result | Log |
|---|---|---|
| `make check` baseline | green, 123.78 s | `build/robustness/logs/make-check-baseline.log` |
| `make asan` | pending | |
| `make ubsan` | green, 2 suites, activation self-test, 9.91 s | `build/robustness/logs/ubsan-d1.log`, `ubsan-d1b.log` |
| `make corpus-amiga` | pending | |
| `make differential` | pending | |
| `make bench-shape` | pending | |
| `make verify` | pending | |

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

## Review Verdicts

| Phase | Reviewers | Verdict |
|---|---|---|
| Requirements | 2 independent auditors | 36 findings resolved; user E10202 ruling |
| Plan | 2 independent auditors + focused re-scan | 45 + 14 findings resolved |
| D1 | correctness reviewer + memory-safety auditor | RV-001 fixed; MS-001/MS-002 fixed; re-review clean; 87 ASan/LSan probes, no report |
| D2/D3 | correctness + memory-safety + emission-integrity | RV-001/EI-001 fixed; re-review clean; 3,760-case matrix, UBSan clean, corpus diff attributable |
| D4/D6 | pending | |
| Fuzzer | pending | |
| Final | pending | |

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
