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
| Red | `300 400 -1 256 65537`; `u16*u16` strict build fails; `u16<<15` is 2147450880 |
| Fix | width-aware `unsigned int` intermediate; shift result cast |
| Green | pending |
| Review | pending |

### D3 — Inline shift divisor

| Item | Evidence |
|---|---|
| Red | `build/robustness/base-0ac87d3e-clc` traps `E11210` |
| Head | `q: 0` |
| Fix | none; regression test and evidence |
| Green | pending |
| Review | pending |

### D4 — Narrow const strict build

| Item | Evidence |
|---|---|
| Red | `-Werror=overflow` on `const U8W: u8 = u8(200) + u8(100)` |
| Fix | shared with D2 |
| Green | pending |

### D5 — `f32` rounding and constant float overflow

| Item | Evidence |
|---|---|
| Red (cast) | `117 116` |
| Red (declared) | `117 116` |
| Red (NaN) | `#define E_A 0` |
| Fix | `round_float`, range/NaN `E10202`, float result scalar, declared width |
| Green | pending |

### D6 — Enum values and conversion

| Item | Evidence |
|---|---|
| User ruling | E10202 for an auto-increment beyond the backing type (AR-11) |
| Red (explicit) | `#define E_A 0` |
| Red (overflow) | accepted with `E_B 256` / folder 0 |
| Red (conversion) | `Direction(2)` C build error; `Buffer(0)` same |
| Fix | shared `enum_member_values`, sema fit check, conversion lowering |
| Green | pending |

## Gate Evidence

| Gate | Result | Log |
|---|---|---|
| `make check` baseline | green, 123.78 s | `build/robustness/logs/make-check-baseline.log` |
| `make asan` | pending | |
| `make ubsan` | pending | |
| `make corpus-amiga` | pending | |
| `make differential` | pending | |
| `make bench-shape` | pending | |
| `make verify` | pending | |

## Review Verdicts

| Phase | Reviewers | Verdict |
|---|---|---|
| Requirements | 2 independent auditors | 36 findings resolved; user E10202 ruling |
| Plan | 2 independent auditors + focused re-scan | 45 + 14 findings resolved |
| D1 | correctness reviewer + memory-safety auditor | RV-001 fixed; MS-001/MS-002 fixed; re-review clean; 87 ASan/LSan probes, no report |
| D2/D3 | pending | |
| D4/D6 | pending | |
| Fuzzer | pending | |
| Final | pending | |

## Residual Limitations

| Limitation | Reason |
|---|---|
| A statement pure conditional fill repeats its condition expression per element | Pre-existing evaluation-order shape; the guard fix does not hoist it. The module path hoists the condition. Recorded in `docs/known-limitations.md`. |
| A module fill that covers no slot with an impure or struct value emits an unused prepared local | Pre-existing strict-build limitation found during the D1 review (base and head); outside the guard fix. Recorded in `docs/known-limitations.md`. |
| A fixed-array row-place field emits the row bounds guard twice | Pre-existing duplicate guard in the row-copy path (base and head); no memory-safety impact. Recorded as a code-size limit. |
| The fuzzer's differential cannot detect a shared host/m68k wrong value | The committed seed goldens and the spec suites are the value oracle. |
| The fuzzer is a bounded template grammar, not coverage-guided | No new dependency; determinism is required for the gate. |

## Rejected Candidates

| Candidate | Why rejected |
|---|---|
| Materializing float constants | A storage/perf change outside the correctness closeout |
| Wrapping enum auto-increment instead of E10202 | User ruled E10202 |
| Keeping 256 for an out-of-range enum member | Unrepresentable in the backing type; invalid C99 store |
| Adding ASan to `make verify` | It is already a separate recorded gate; adding 27 suites changes acceptance runtime without a measured need |
