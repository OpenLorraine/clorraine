# Findings: Constant-Array Codegen Residuals (PLAN-36)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Base**: `7071761` (clean `main == origin/main`)

## 1. Baseline

| Item | Value |
|---|---|
| Base commit | `7071761` |
| Compiler | `clc 1.1.2` |
| Calibration | 2,928 raw m68k instructions |
| Probe root | `build/tmp/probes` (temporary) |
| Preserved base compiler | `build/tmp/clc-base` |
| Band store compiler | `build/tmp/clc-band` |
| 1-byte template compiler | `build/tmp/clc-byte` |

The probe sources use the same `i32` sine values as the committed
`benchmarks/local-table` program. The comparison uses the shipped release
flags from `tools/perf/bench.py`; only the compiler or the eligibility rule
differs.

## 2. Measurements

### 2.1 Function-local template (head vs base)

| Probe | Base `.text` | Head `.text` | Δ `.text` | Base net | Head net | Δ net |
|---|---:|---:|---:|---:|---:|---:|
| `local-table` (`i32[256]`) | 25,644 | 24,640 | −1,004 | 1,074 | 814 | −260 |
| `local-1024` (`i32[1024]`) | 31,748 | 27,712 | −4,036 | 4,210 | 3,165 | −1,045 |
| `local-1280` (`i32[1280]`) | 33,964 | 28,708 | −5,256 | 4,273 | 2,601 | −1,672 |

The local template wins both metrics at every measured size. `local-table` is
the committed benchmark shape.

### 2.2 2–5 KB band (template vs store, same module target)

| Probe | Store `.text` | Template `.text` | Δ `.text` | Store net | Template net | Δ net |
|---|---:|---:|---:|---:|---:|---:|
| `module-1024` | 33,744 | 27,700 | −6,044 | 3,118 | 3,159 | +41 |
| `module-1280` | 36,320 | 28,700 | −7,620 | 3,893 | 2,600 | −1,293 |

At 1024 the template is a large size win for +0.4% instructions in this scan
workload; the PLAN-25 init-dominated measurement was +1.18%. At 1280 the
template wins both. **Decision: keep the template; no size-aware cap.**

### 2.3 1-byte table (template vs store, `u8[256]`)

| Form | `.text` | Net instructions | C bytes |
|---|---:|---:|---:|
| Stores | 24,200 | 1,111 | 325,694 |
| Template | 23,864 | 1,322 | 316,725 |
| Δ | −336 | **+211** | −8,969 |

The template costs 211 instructions for 336 `.text` bytes. **Decision: keep
the stores.**

### 2.4 Short-list tail

The short local list (`i32[64]`, 16 values) in a loop prints:

```
iter 0 tail 0
iter 1 tail 99
```

The declaration zeroes the local once at function entry; the stores cover only
the prefix, so a loop re-entry keeps the prior tail. The spec does not state a
tail policy for a short list without a fill. **Decision: keep the stores and
record the behavior; no semantic change in this bundle.**

### 2.5 Module-level path

The module probes are byte-identical between the base and head compilers:
`module-1024`, `module-1280`, `module-byte`, and `module-short` all show 0
`.text` and 0 instruction delta. The shared-helper refactor did not change the
module-level emission.

## 3. Decisions

| Residual | Decision | Evidence |
|---|---|---|
| Function-local tables | **Land the template** | −1,004/−4,036/−5,256 `.text`; −260/−1,045/−1,672 net instructions |
| 1-byte tables | Keep stores | +211 net instructions at `u8[256]`; only −336 `.text` |
| 2–5 KB 4-byte band | Keep the template | −6,044 `.text` for +41 instructions at 1024; wins both at 1280 |
| Short/empty lists | Keep stores; record the tail | Stale tail on loop re-entry; no spec tail rule |
| Module `const` arrays | Out of scope | `E10224` is a checker/language change |
| Freestanding | Keep stores | Byte-wise runtime `memcpy` |

## 4. Implementation

| File | Change |
|---|---|
| `compiler/src/codegen/program.rs` | `emit_constant_array_template_data` emits the predicate-checked block-scope `static const` and returns its name; it skips the current function's local C names as well as user symbols; `emit_constant_array_template` is the module-level copy wrapper |
| `compiler/src/codegen/function.rs` | The top-level `Statement::Assign` path checks the template before rendering the place, then copies with `memcpy(ArrayTarget.address, name, ArrayTarget.bytes)`; a runtime-indexed element target keeps the stores so GCC's `-Wstringop-overflow` cannot reject the strict build, while a constant-indexed element stays eligible |
| `compiler/tests/constant_array_residuals_spec_test.rs` | 13 behavior cases |
| `compiler/tests/constant_array_residuals_impl_test.rs` | 18 emission cases |
| `benchmarks/local-table/` | 256-entry function-local `i32` sine table, weighted `u32` checksum, expected output, README |
| `benchmarks/README.md` | The sixth scored row |
| `tools/perf/fixtures/shapes.clo` | A 64-entry function-local `i32` table, so the shape gate builds the new template under m68k GCC and vbcc |
| `docs/known-limitations.md` §13 | Function-local row, re-measured 1-byte and band rows, short-list tail row |

## 5. Test Evidence

| Suite | Base compiler | Head compiler |
|---|---|---|
| `constant_array_residuals_impl_test` | 10 failed / 8 passed (the positive pins are red) | 18 passed / 0 failed |
| `constant_array_residuals_spec_test` | 13 passed / 0 failed | 13 passed / 0 failed |
| `constant_array_initializer_impl_test` | unchanged | 32 passed / 0 failed |
| `constant_array_initializer_spec_test` | unchanged | 42 passed / 0 failed |

The ten base failures are IT-1–IT-4, IT-11, IT-12, IT-14, IT-15, IT-17,
and the adjusted-parameter pin (IT-18), exactly the cases that require a local
template. The
eight exclusion pins pass on both, as intended. The shadow case was checked
against a temporary pre-fix compiler (`build/tmp/clc-prefix`, the `local_taken`
check removed): it fails there with a `__cl_ctmpl_1` redefinition, so the pin
is red for the right reason.

## 6. Benchmark Baseline

`local-table` is the new scored row. The base compiler emits stores; the head
compiler emits the template.

| Metric | Base | Head | Δ |
|---|---:|---:|---:|
| `.text` | 25,572 | 24,568 | −1,004 (−3.9%) |
| Net instructions | 3,188 | 2,927 | −261 (−8.2%) |
| C bytes | 320,767 | 316,882 | −3,885 (−1.2%) |

The full `make bench-compare` table:

```
benchmark         base ins    head ins    delta  base .text  head .text    delta
local-table           3188        2927    -8.2%       25572       24568    -3.9%
lookup                8779        8779    +0.0%       39884       39884    +0.0%
matrix                9361        9361    +0.0%       23592       23592    +0.0%
moves                44630       44630    +0.0%       23684       23684    +0.0%
sieve              1040081     1040081    +0.0%       23816       23816    +0.0%
strings            1221232     1221232    +0.0%       23972       23972    +0.0%

require:
  local-table instructions     -8.2%  need <= -4%  PASS
  local-table text             -3.9%  need <= -2%  PASS

compare: passed
```

## 7. Acceptance

| Gate | Result |
|---|---|
| `make check` | passed: spec check, registry, 148 workspace test groups (18 impl + 13 spec new), fuzz smoke 28/28 host + 16/16 sanitizer/target, 0 failures |
| `make asan` | passed: 38 suites, including the 13-case residual spec suite |
| `make ubsan` | passed: 12 suites, including the 13-case residual spec suite |
| `make bench-shape` | passed: hosted-amiga GCC, nix13 compile, vbcc, and freestanding compile warning-free; the fixture now includes a 64-entry function-local table |
| `make bench-compare` | passed: `local-table` −8.2% instructions / −3.9% `.text`; every other row 0.0%; both requirements PASS |
| `make corpus-amiga` | passed: all portable programs match on hosted-amiga; vbcc/build-only legs warning-free |
| `make differential` | passed: buffer-container, file-copier, and string-parser all match (3/3) |
| `make verify` | passed: `verify: all legs passed` (check, bench-shape, all A500/A600/A1200/A4000 SDK legs, differential, corpus FS-UAE, run-fault-amiga). The host `:0` display had a GLX/DRI failure, so the host-display legs ran on a private Xvfb `:99`; the `hello` emitted C is byte-identical to the base commit. |
| Reviews | memory safety, correctness, and emission integrity clean of critical/major; the four correctness minors and the two routed non-memory issues are fixed and re-checked |

## 8. Reviews

| Reviewer | Verdict | Notes |
|---|---|---|
| Memory safety | **no findings** | ASan/LSan probes covered a loop entry, a struct field, an adjusted parameter, generic instantiations, runtime-indexed and constant-indexed member rows, a volatile global, a short list, a u8 table, and the ffi/local-name collisions; leak/OOB positive controls fired. It routed two non-memory issues — the runtime-indexed strict-build diagnostic and the shadow test's red case — both fixed and re-checked. |
| Correctness | **no critical/major** | RV-001–RV-004 minor: the ffi collision pin was vacuous, the array-parameter target lacked a case, the `init_templates`/`local_taken` docs were stale, and the plan red-list/task ranges lagged. All four are addressed above. |
| Emission integrity | **no findings** | 84 units diffed across host, hosted-amiga, hosted-amiga-13, and freestanding; 79 strict builds and runtime comparisons; only the intended local-table, shapes, boing, and cube units changed; the module-level path and all unrelated units are byte-identical. A pre-existing volatile `for (borrow)` alias defect is recorded separately and is not introduced here. |

## 9. Residual Opportunities

| Opportunity | Evidence | Disposition |
|---|---|---|
| Module `const` arrays | `E10224`; a checker/language change | Out of scope; needs its own scope ruling |
| 1-byte store packing | The template loses +211 instructions at 256 bytes; a pack-aware rule could revisit | Residual |
| Short-list tail policy | Loop re-entry keeps the prior tail; the spec leaves it open | Residual; needs a semantic ruling |
| 2–5 KB band | The template is a large `.text` win for a small instruction cost at 1024 | Accepted and documented |
| `__cl_itmp` user-FFI shadowing | Pre-existing collision class outside this change | Residual defect |
| Freestanding template | Byte-wise runtime `memcpy` | Out of scope |
