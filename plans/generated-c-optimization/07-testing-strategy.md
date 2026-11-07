# Testing Strategy: Generated-C Optimization

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The feature is behavior-preserving for accepted programs. The original spec
suite is a green-before/green-after regression oracle: its R4–R7 cases pass on
the base compiler and must keep passing. The fix-pass trap cases pin defects
the independent reviews exposed in the first implementation and in the base
compiler's constant handling; those cases are red on the pre-fix compiler and
are bug-fix oracles, not behavior-preservation claims. The impl suite is the
red-first driver only for the pins whose shape is absent at the base; the pins
that already pass are regression pins. The benchmark harness proves the
measured win and guards against a runtime regression.

| Layer | File | Role |
|---|---|---|
| Behavior | `compiler/tests/generated_c_optimization_spec_test.rs` | Immutable behavior oracle for R4–R7 |
| Emission | `compiler/tests/generated_c_optimization_impl_test.rs` | Red-first shape pins and green regression pins |
| Harness | `tools/perf/tests/test_bench.py` | Schema, comparison, thresholds, failure paths |
| Benchmark | `benchmarks/` + `make bench-base`/`bench`/`bench-compare` | Runtime and code-size evidence |
| Existing | `make check`, `make asan`, `make corpus-amiga`, `make differential`, `make verify` | No regression across the repository |

## Source Conventions for the ST Cases

- A module-level `const` needs an explicit type:
  `const A: i32 = 5;`. A local `const` may infer its type.
- An imported module-level const needs `export`:
  `export const B: i32 = A * 2;`.
- A function with no result writes `: void`.
- `slice` is an intrinsic from `std.mem`; an ST that uses it imports
  `{ slice } from std.mem;`.
- A bool interpolates as `true`/`false`; use `i32(x < 0)` when the expected
  line needs `1`.
- Trap cases are built as host C and run; the assertion is the exact first
  stderr line (`error E11211: index out of range` / `error E11210: integer
  division by zero`) and a non-zero exit, following the existing
  `run_fault_spec_test.rs` pattern. An emit-only assertion is not enough.

## Specification Test Cases

The cases are derived from R4–R7 and the design in
[Generated-C Changes](03-02-generated-c-changes.md). Expected outputs are
computed from the language rules, not from the current emitter.

### R4 — Scalar Module `const` Folding

| # | Input / scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-1 | `const N: i32 = 8;` loop `i < N`, sum 0..7; print sum | `const: 28` | R4.1, R4.2 |
| ST-2 | `const N: i32 = -3;` print `N`, `i32(N < 0)`, `10 / N` | `-3 1 -3` | R4.1, R4.5 |
| ST-3 | `const A: f32 = f32(16777217.0);` compare to `let b: f32 = f32(16777217.0);`; also a finite `f64` literal | `f32: equal` and `f64: equal` | R4.1, R4.5 |
| ST-4 | `const A: i32 = 5; const B: i32 = A * 2 + 1;` print `B` | `11` | R4.4 |
| ST-5 | Module `m`: `export const A: i32 = 5; export const B: i32 = A * 2;`; module `n` imports `B` and prints it | `10` | R4.4 |
| ST-6 | Local `let N: i32 = 3;` shadows `const N: i32 = 5`; print `N` | `3` | R4.2 |
| ST-7 | `const N: i32 = 5; let p: raw i32 = unsafe &N; unsafe { *p = 6; } return N - 5;` | exits 1 (storage visible, not folded) | R4.3 |
| ST-8 | `const N: i32 = 5;` print `N` with no address taken | `5` | R4.1 |
| ST-9 | `const M: i32 = 5; const N: i32 = M;` read `N` in a function | `5`; the fold uses the declaring module | R4.4 |
| ST-10 | `const S: static str = "abc";` read `S` | unchanged behavior; `S` stays a place | R4.1 |
| ST-11 | Module initializer `let X: i32 = N;` with `const N: i32 = 5;` | `X` is 5; the module initializer path is unchanged | R4.7 |
| ST-36 | `const S: i32 = 1 << 31;` print `i32(S < 0)`; `const T: i32 = 1 << 32;` print `T` | `1` and `1` (the count masks; the result wraps at i32) | R4.5 |
| ST-37 | `const A: i32 = 2147483647 + 1;` print `A` | `-2147483648` (wraps at i32) | R4.5 |
| ST-38 | `const INF: f64 = 1.0 / 0.0;` print `INF` | runtime prints `inf`; the const stays a place | R4.1 |
| ST-39 | A const read inside an address-taken `&N` nested in a call argument, a lambda body, and a `defer`; raw-write one and read it | each read observes the raw mutation | R4.3 |
| ST-42 | A bare module-qualified const `m.N` in an expression, and a qualified member `obj.N` where `obj` is a global struct | `m.N` folds; `obj.N` stays a field read and is unchanged | R4.2 |

### R5 — Compact Large One-Extent Scalar Fills

| # | Input / scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-12 | `let a: u8[4096] = [; 1];` sum all | `4096` | R5.1, R5.2 |
| ST-13 | `let a: i32[4096] = [; 7];` sum all | `28672` | R5.1, R5.2 |
| ST-14 | `let a: bool[4096] = [; true];` count true | `4096` | R5.2 |
| ST-15 | `let a: u8[20] = [2, 3; 4];` sum all | `77` | R5.3 |
| ST-16 | Impure fill `[; u8(next())]` over 4096 slots; `next` increments a global | one call, sum `4096` | R5.4 |
| ST-16b | A pure fill from a `volatile` module place over 4096 slots | the volatile read is repeated per element; sum matches the constant read | R5.4 |
| ST-17 | `func fill(borrow a: u8[4096]): void { a = [; 1]; }`; sum after the call | `4096`; the adjusted parameter is filled, not the pointer | R5.5 |
| ST-18 | `func make(): u8[4096] { return [; 1]; }`; sum the result | `4096`; the hidden return slot is filled | R5.5 |
| ST-19 | A 15-slot fill and a 16-slot fill | both produce the correct sums; the shape difference is in IP-4/IP-5 | R5.1 |
| ST-20 | A nested-row fill and a struct-literal fill | behavior unchanged; the existing spelling is kept | R5.1 |

### R6 — Non-Negative Slice/String Constant Guard

| # | Input / scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-21 | `import { slice } from std.mem; let a: u8[4] = [1,2,3,4]; let s: u8[] = slice(a, 0, 4); let x = s[1];` | `x` is the second byte; no trap | R6.1 |
| ST-22 | `let t: str = "abcd"; let x = t[1];` | `x` is `b`; no trap | R6.1 |
| ST-23 | A slice `s[-1]` at run time | runtime `E11211` trap, non-zero exit | R6.2 |
| ST-24 | A string `t[-1]` at run time | runtime `E11211` trap, non-zero exit | R6.2 |
| ST-25 | A slice `s[4]` on a length-4 slice | runtime `E11211` trap, non-zero exit | R6.1 |
| ST-26 | A fixed array `a[-1]` | the existing compile-time rejection `E10216`, unchanged | R6.2 |
| ST-27 | A `usize` index into a slice | the unsigned guard is unchanged (no `< 0`) | R6.3 |
| ST-40 | `let i: i32 = 1; s[i]` | the non-constant signed index keeps the signed guard | R6.2 |

### R7 — Constant Division and Remainder Guard

| # | Input / scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-28 | `let r = 10 % 2;` print `r` | `0` | R7.1 |
| ST-29 | `let min: i32 = -2147483648; let r = min / -1;` | runtime `E11210` trap, non-zero exit | R7.2 |
| ST-30 | `let z: i32 = 0; let r = 10 / z;` | runtime `E11210` trap, non-zero exit | R7.4 |
| ST-31 | `let r = 10 / 0;` | runtime `E11210` trap through the existing trap expression | R7.3 |
| ST-32 | `let d: i32 = -1; let r = 10 / d;` | runtime `-10`; the non-constant divisor keeps the full guard | R7.4 |
| ST-41 | `let d: i32 = i32(2147483647) + i32(2147483647) + i32(1); let r = min / d;` | runtime `E11210`; a wrapping binary divisor keeps the full guard | R7.1 |

### Cross-Cutting

| # | Input / scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-33 | Emit the same source twice | byte-identical C | N3 |
| ST-34 | Emit a no-stdlib program with a compact fill and a folded const for `host`, `hosted-amiga`, and `freestanding`; the slice guard is host-only | all three are accepted; the m68k unit builds with the strict flags | N4 |
| ST-35 | Run the existing `spec/tests/run` fault programs | same codes and exit statuses | R8.3 |

## Implementation Test Cases

The impl suite pins the emitted text and the threshold boundaries. The
`Base` column records the expected status before implementation.

| # | Scenario | Pin | Base |
|---|---|---|---|
| IP-1 | `const N: i32 = 8192; while (i < N)` | the declaration and `cl_init` assignment remain; the loop condition uses `8192`; no read of `cl_..._N` in the loop | 🔴 red |
| IP-2 | `u8[4096] = [; 1]` | contains `memset(`; fewer than 16 direct stores | 🔴 red |
| IP-3 | `i32[4096] = [; 7]` | contains one `for (` over the covered range; fewer than 16 direct stores | 🔴 red |
| IP-4 | `u8[15] = [; 1]` | no `memset`, no compaction loop; 15 direct stores | 🟢 green |
| IP-5 | `u8[16] = [; 1]` | `memset` (or a compact loop), not 16 direct stores | 🔴 red |
| IP-6 | `func fill(borrow a: u8[4096]): void { a = [; 1]; }` | the fill uses the parameter address and the element count; no `sizeof(parameter)` | 🔴 red |
| IP-7 | `let s: u8[] = slice(a, 0, 4); s[1]` | the guard has no `< 0` | 🔴 red |
| IP-8 | `let s: u8[] = slice(a, 0, 4); s[-1]` | the guard retains the signed check and the `E11211` trap | 🟢 green |
| IP-9 | `let r = x % 2;` | no `2 == 0` and no `2 == -1` in the guard | 🔴 red |
| IP-10 | `let min = -2147483648; let r = min / -1;` | only the `MIN` check, no zero check | 🔴 red |
| IP-11 | `const N: i32 = 5; let p = unsafe &N;` | the C contains `&(cl_..._N)`, not `&(5)`; the read stays a place | 🟢 green |
| IP-12 | A shadowing local `N` | the local is used; no folded module literal | 🟢 green |
| IP-13 | Two emits of the same source | byte-identical C | 🟢 green |
| IP-14 | `let a: u8[4096] = [; unsafe readVolatile8(unsafe &v)];` | one `__cl_fill` assignment before the loop, then the loop copies it; the intrinsic is not repeated | 🔴 red (shape) |
| IP-15 | A pure `volatile` place fill over 4096 slots | the loop contains the volatile read expression; no `memset` | 🔴 red (shape) |
| IP-16 | A materialized/guarded fixed-array literal with a large fill | the materialized form stays unrolled; the statement form compacts | 🟢 green (scope regression) |
| IP-17 | `const S: i32 = 1 << 31; const T: i32 = 1 << 32;` | the folded literals are `-2147483648`/`1` in the C; no bare `2147483648u` compare | 🔴 red |
| IP-18 | `const INF: f64 = 1.0 / 0.0;` | the read stays `cl_..._INF`; the C never contains `inf.0` or `NaN.0` | 🟢 green |
| IP-19 | `&N` nested in a call argument, a lambda, and a `defer` | the reads stay places; the C contains `&(cl_..._N)` | 🟢 green |
| IP-20 | A bare `m.N` read and a projected `obj.N` read | `m.N` is a literal; `obj.N` keeps the field read | 🔴 red (bare-qualified fold) |

## Fix-Pass Additions

The independent reviews and the fix pass added these oracle cases:

| # | Input / scenario | Expected behavior | Source |
|---|---|---|---|
| ST-43 | `const A: i32 = i32(f32(16777217.0));` and a `f32(0.99999999)` twin; compare read to storage | both read `16777216`/`1`; the const stays a place | R4.5 |
| ST-44 | `let m: i32 = -2147483648; m / i32(-u8(1)); m / i32(~u8(0));` | `-8421504` both; the unsigned unary renders at `u8` | R4.5, R7 |
| ST-45 | `x % (u32(0) + u32(0));` and `x / i32(0.0);` | strict C build; runtime `E11210` trap | R7.3 |
| ST-46 | `volatile let G: u8[20]` filled by a named prefix, a conditional, and a volatile struct field | compiles strict; no `memset` on the volatile target; values correct | R5.1 |
| ST-47 | A materialized conditional `eat(c ? [;1] : [;2])` in a `while` | compiles and runs; the fill stays unrolled | R5.1 |
| ST-48 | `i32[63]` and `i32[64]` fills | 63 unrolled; 64 one loop | R5.1 |
| ST-49 | `u8[32] = [; u8(1) << 8]` | unrolled (non-constant byte), value 1 per element | R5.2 |
| ST-50 | `const B = A` where `&A` is taken; a lambda address; a huge negative slice; a bitnot divisor | place reads, trap codes preserved | R4.3, R6.2 |

The spec suite now has 43 cases; the impl suite has 49 pins. The red/green
record is in `05-findings.md`.

## The Harness Self-Tests

`tools/perf/tests/test_bench.py` covers:

1. schema validation (`schema`, required fields, benchmark set);
2. delta arithmetic and the 1%/20%/10% rules, including the `sieve`
   `instructions_net`, `.text`, and `c_bytes` hard rows;
3. each hard-fail case (output, instructions, `.text`, `c_bytes`);
4. a structural mismatch (toolchain, runs, flags, benchmark set, missing
   trace);
5. an unparsable size line;
6. a fake compiler that exits non-zero;
7. a fake compiler that emits a program whose output differs;
8. `--out` outside `build/perf/`, an unknown `--bench`, and a `--label`
   traversal;
9. a timeout through a stub process;
10. `\r\n` and lone-`\r` normalization;
11. two runs of the same fixture produce the same instruction count and output
    hash (synthetic trace data).

The tests feed recorded or synthetic data into the parsing and comparison
functions. The fake-compiler cases use a stub tool directory on `PATH` whose
`clc` exits non-zero (or emits a program with a different fixed output); the
run stops at the emit failure, so no real toolchain is needed. The integration
proof is `make bench` itself.

## Existing Emission Pins Updated

Two existing implementation pins change with the intended emission, and both
keep their behavior checks:

- `checker_conformance_impl_test::should_store_an_impure_dividend_once_in_an_initializer`
  no longer expects the `MIN` guard for a constant `1` divisor; the impure
dividend is still stored once and the division is still tested.
- `def_backlog_closeout_impl_test::should_spell_module_names_inside_a_captureless_lambda`
  now expects the folded `40` instead of a module-const read; the lambda still
  calls the same function and the program still exits 42.

## Test Data and Fixtures

- The spec and impl suites use the existing `ScratchDir` pattern and
  `env!("CARGO_BIN_EXE_clc")`; no new fixture directory is needed.
- The two-module case writes `holder.clo` and `main.clo` and passes both paths
  to `clc`, following `module_initializer_integrity_spec_test.rs`.
- The trap cases build the host C with `cc` and run it, asserting the exact
  first stderr line and a non-zero exit, following `run_fault_spec_test.rs`.
- The benchmark suite is the runtime fixture set; each program has its
  expected output and a README, including the calibration.
- The base compiler fixture is `build/perf/base-clc`, produced by
  `make bench-base` from the recorded commit; it is not committed.
- The new spec suite references `RDC_ASAN` and includes the fill and const
  cases under `make asan`.

## Verification Checklist

- [ ] Every ST case traces to R4–R7 or a cross-cutting requirement.
- [ ] The spec suite passes on the base compiler before implementation.
- [ ] The fix-pass additions (ST-43–ST-50) are green; the impl suite has 34 pins.
- [ ] The impl suite's red pins (IP-1, IP-2, IP-3, IP-5, IP-6, IP-7, IP-9,
      IP-10, IP-14, IP-15, IP-17, IP-20) fail on the base compiler.
- [ ] The green pins (IP-4, IP-8, IP-11, IP-12, IP-13, IP-16, IP-18, IP-19)
      pass on the base and after the change.
- [ ] The impl suite is green after implementation.
- [ ] `make bench-base`, `make bench`, and `make bench-compare` produce the
      recorded evidence and pass R1.5 plus the R4.8/R5.7 sieve targets.
- [ ] `make bench-test` passes in the pinned container and is part of
      `make check`.
- [ ] `make check`, `make asan`, `make corpus-amiga`, `make differential`, and
      `make verify` are green.
- [ ] No file under `spec/` is edited.
