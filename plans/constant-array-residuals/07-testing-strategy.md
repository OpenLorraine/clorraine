# Testing Strategy: Constant-Array Codegen Residuals (PLAN-36)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## 1. Test Layers

| Layer | File | Oracle |
|---|---|---|
| Specification behaviour | `compiler/tests/constant_array_residuals_spec_test.rs` | Every element is written on each execution; the writable local, struct field, and parameter keep their values; ineligible shapes keep the store behaviour |
| Implementation emission | `compiler/tests/constant_array_residuals_impl_test.rs` | The block-scope template and `memcpy` appear exactly for the eligible local shapes and are absent for every exclusion |
| Existing constant-array contract | `compiler/tests/constant_array_initializer_{spec,impl}_test.rs` | The module-level rule and its 42 + 32 cases are unchanged |
| Sanitizer | `make asan`, `make ubsan` | No ASan/UBSan report on the new programs |
| Benchmark | `make bench-base`, `make bench`, `make bench-compare` | Equal outputs; the new `local-table` row improves; every existing row stays within the 1% guard |
| Shape gate | `make bench-shape` | The emitted C stays valid C99 under the host, m68k GCC, vbcc, and freestanding legs |

The spec harness emits host C, builds it with
`cc -std=c99 -O0 -g -Wall -Wextra -Werror`, runs it, and compares stdout. Under
`RDC_ASAN`/`RDC_UBSAN` it adds the sanitizer flags. The impl harness emits C
for `host`, `hosted-amiga`, `hosted-amiga-13`, or `freestanding` and pins the
text.

## 2. Specification Cases

| # | Case | Expected | Traces |
|---|---|---|---|
| ST-1 | function-local `i32[32]` full list, summed | `sum: 528` | R1 |
| ST-2 | function-local `u16[32]` full list, summed | `sum: 528` | R1, R2 |
| ST-3 | function-local `usize[32]` full list, summed | `sum: 528` | R1, R2 |
| ST-4 | the same local table in a loop body, two rounds, write the last element | both rounds read the original first/last values | R1, R3 |
| ST-5 | `let t: i32[32]; t = [...];` later assignment | `sum: 528` | R1 |
| ST-6 | `h.values = [...]` on a struct field | `sum: 528` | R1 |
| ST-7 | short local list `i32[32]` with 16 values, read the tail | `tail: 0` | R3, R7 |
| ST-8 | function-local `u8[64]` full list, summed | `sum: 2080` | R3, R5 |
| ST-9 | one call element in a local list, count calls | `calls: 1` | R3 |
| ST-10 | five elements for a `i32[4]` local list | rejected `E10215`, no C | R3 |
| ST-11 | a user local named `__cl_ctmpl_1` plus a local table | runs: the user local and the table both read correctly | R13 |
| ST-12 | a runtime-indexed struct member target with an out-of-range index | traps `E11211` after a strict C build | R3 |
| ST-13 | an adjusted array parameter assigned a full list | `param: 528` | R1 |

ST-1–ST-6 and ST-8–ST-13 pass on both the base compiler (stores) and the head
compiler (template), so they are behaviour guards, not red tests. ST-7 and
ST-10 also pass on both and guard the exclusions. The positive emission pins
in the impl suite are the red tests.

## 3. Implementation Cases

| # | Case | Assertion | Traces |
|---|---|---|---|
| IT-1 | local `i32[32]` | `static const i32 __cl_ctmpl_` + `memcpy(&(t), ...)` + `sizeof(t)` | R1 |
| IT-2 | local `u16[32]` | `static const u16 __cl_ctmpl_` + local copy | R1, R2 |
| IT-3 | local `usize[32]` | `static const usize __cl_ctmpl_` + local copy | R1, R2 |
| IT-4 | `host`, `hosted-amiga`, `hosted-amiga-13` | the template appears in all three | R1 |
| IT-5 | `freestanding` | no template | R3, R6 |
| IT-6 | short local list | no template; the first store is present | R3, R7 |
| IT-7 | local `u8[64]` | no template | R3, R5 |
| IT-8 | local fill `[prefix; 7]` | no template | R3 |
| IT-9 | local list with `1 / d` | no template | R3 |
| IT-10 | module-level `i32[32]` | the template and global copy still appear | R4 |
| IT-11 | `ffi __cl_ctmpl_0` + local table | the call survives; `static const i32 __cl_ctmpl_0[32]` is absent | R13 |
| IT-12 | struct field assignment | `memcpy(&(h.values), ...)` | R1 |
| IT-13 | volatile global assignment | no template | R3 |
| IT-14 | local declaration placement | the template is inside `main` and precedes its `memcpy` | R1, R12 |
| IT-15 | user local `__cl_ctmpl_1` | the user local declaration survives and the template name is not `_1` | R13 |
| IT-16 | runtime-indexed member target | no template; the store form is emitted | R3 |
| IT-17 | constant-indexed member target | the template is emitted | R1 |
| IT-18 | adjusted array parameter | the template and `memcpy(a, ...)` are emitted | R1 |

## 4. Red/Green Order

1. The impl suite runs against the preserved base compiler
   (`RDC_CLC=build/tmp/clc-base`). IT-1–IT-4, IT-11, IT-12, IT-14, IT-15,
   IT-17, and IT-18 fail on the missing local template; the exclusion cases
   pass.
2. The impl suite runs against the head compiler. Every case passes.
3. The spec suite runs against both compilers. Every case passes, proving the
   local template does not change the visible behavior.
4. The existing constant-array suites, the full workspace tests, and the
   sanitizer legs run before the benchmark gate.

## 5. Gate Coverage

| Gate | What it proves |
|---|---|
| `make check` | Spec check, workspace tests, fuzz smoke, and the new suites |
| `make asan` | No leak, use-after-free, or double-free in the new local programs |
| `make ubsan` | No undefined behavior in the new local programs |
| `make bench-shape` | The emitted C is valid C99 on every toolchain leg |
| `make bench-base` + `make bench` + `make bench-compare` | The baseline is the base compiler; `local-table` improves; every existing row is inside the 1% guard |
| `make corpus-amiga` + `make differential` | The m68k corpus output is unchanged |
| `make verify` | The full acceptance gate |
