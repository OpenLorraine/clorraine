# Testing Strategy: Template Placeholder Residuals II

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## 1. Test Layers

| Layer | File | Oracle |
|-------|------|--------|
| Specification behavior | `compiler/tests/template_placeholder_residuals_ii_spec_test.rs` | R1–R11 and the frozen spec |
| Implementation emission | `compiler/tests/template_placeholder_residuals_ii_impl_test.rs` | The intended diagnostic/emission shapes |
| #27 regression | `compiler/tests/template_placeholder_residuals_{spec,impl}_test.rs` | Unchanged PLAN-31 contract |
| #26 regression | `compiler/tests/template_placeholder_integrity_{spec,impl}_test.rs` | Unchanged #26 contract |
| #25 regression | `compiler/tests/template_direct_formatting_{spec,impl}_test.rs` | Unchanged allocation/emission contract |
| Nullability regression | `compiler/tests/def_zero_null_spec_test.rs`, `def_semantic_parity_integrity_spec_test.rs` | Unchanged E10010/E10012 rules |
| Sanitizer | `make asan`, `make ubsan` | No ASan/UBSan report |
| Benchmark | `make bench` + `make bench-compare` | Equal outputs; 1% scored guard |

The spec suite follows the PLAN-31 harness: emit host C with the test `clc`,
build with `cc -std=c99 -O0 -g -Wall -Wextra -Werror`, run, and compare
stdout/exit. Under `RDC_ASAN`/`RDC_UBSAN` it adds the sanitizer flags and sets
`ASAN_OPTIONS=detect_leaks=1:symbolize=0`, `UBSAN_OPTIONS=halt_on_error=1`.

`assert_panic_clean(tag, source)` is a forced-UBSan helper: it emits host C,
builds with `-fsanitize=undefined -fno-sanitize-recover=all -no-pie`, runs with
`UBSAN_OPTIONS=halt_on_error=1`, asserts a non-zero exit, asserts stderr
contains `panic: `, and asserts stderr contains no `runtime error:` or
`AddressSanitizer`. It does not reuse `run`, which asserts exit 0.

## 2. Specification Test Cases

"Rejected" means exit 1, the named diagnostic, and no output `.c` file.
"Runs" means exit 0 and exact stdout. "Leak-free" means an ASan build exits 0
with no leak report. "Panic-clean" is the helper above.

| # | Source shape | Expected | Traces |
|---|--------------|----------|--------|
| ST-1 | `let t: String = text(null);` | rejected: E10500, `type 'null' is not formattable`, no C | R1, AR-1 |
| ST-2 | `struct Box { n: i32; } let t: String = text(Box { n: 1 });` | rejected: E10500, `type 'Box' is not formattable`, no C | R1, AR-2 |
| ST-3 | `func show<T>(x: T): String { return text(x); }` with `show<Box>(Box { n: 1 })` | rejected: E10500 at the argument, no C | R2, AR-3 |
| ST-4 | the same `show<T>` with `show<i32>(7)` | runs: `7` | R2, AR-3 (guard) |
| ST-5 | `let s: String? = stringOf("x"); println(text(s));` unnarrowed | rejected: E10010 at `s`, no C | R1, AR-20 |
| ST-6 | the same inside `if (s != null)` | runs: `x` | R1, AR-20 (guard) |
| ST-7 | `func rec<T>(x: T): String { return text(x.n); }` with `rec<P>(p)` where `P` has `n: i32` | rejected: E10500 at `x.n`, no C | R1, AR-21 |
| ST-8 | `let flag: bool = true; println($"{flag ? null : "x"}");` | runs: an empty line | R10, AR-5 (guard) |
| ST-9 | `println($"{null + 1}");` | rejected: E10012 at the null, no C | R3, AR-4 |
| ST-10 | `let v: str = null; if (v == null) { println("null"); }` | runs: `null` | R3, AR-18 (guard) |
| ST-30 | `let flag: bool = true; let c = null && flag;` | rejected: E10218 (the frozen logical-condition rule) | R3, AR-4 (guard) |
| ST-33 | `let c = null + null;` | rejected: E10012 at the left null, no C | R3, AR-4 |
| ST-34 | `let c: bool = null == null;` | runs: `true` (a null comparison stays legal) | R3, AR-4 (guard) |
| ST-35 | `println($"{flag ? null : toCString("x")}");` | rejected: E10500 at the placeholder (the cstring sibling is not formattable) | R4, AR-5 (guard) |
| ST-36 | `let flag = true; if (-(flag ? null : 5) > 0) { ... }` | rejected: E10012 at the null (a conditional nested in a value operand of a condition) | R5, AR-8 |
| ST-37 | `func impl(s: String?): String`; `let text: func(String?): String = impl; text(s)` | runs: the module value is called, not the intrinsic | R1, AR-1 (guard) |
| ST-38 | `func f<T>(x: T): bool { let z = null; return z == x; }` with `f<i32>(1)` | rejected: E10012 at `z` (a null binding vs an open parameter) | R3, AR-4 (guard) |
| ST-39 | `struct P { a: i32; b: i32; }`; `for (let i = 0; i < 1; p = P { b: mark(2), a: mark(1) }, i = i + 1) { }` | runs: `21` (the step's differing aggregate order) | R8, AR-12 |
| ST-11 | `show<str>(false, "x")` with `$"{f ? null : x}"` | runs: `x` | R4, AR-5 |
| ST-12 | `show<String>(true, stringOf("x"))` with the same placeholder | rejected: E10010 only at the null; no E10012 | R4, AR-6 |
| ST-13 | `show<i32>(true, 1)` with the same placeholder | rejected: E10012 at the null | R4, AR-7 |
| ST-14 | `let flag: bool = true; let x = flag ? null : 5;` | rejected: E10012 at the null, no C | R5, AR-8 |
| ST-15 | `struct Box { n: i32; } let x = flag ? null : Box { n: 1 };` | rejected: E10012 at the null | R5, AR-8 |
| ST-16 | `let flag: bool = true; let y = flag ? stringOf("x") : null; if (y != null) { println(y); }` | runs: `x` (inferred `String?`) | R5, AR-9 (guard) |
| ST-17 | `let flag: bool = true; let y: i32 = flag ? 1 : null;` | rejected: E10012 (written destination) | R5, AR-9 (guard) |
| ST-18 | `let s: String = stringOf(""); for (let i: i32 = 0; i < 3; s = $"{i}", i = i + 1) { println($"{i}"); }` | runs: `0`, `1`, `2`; leak-free | R6, AR-10 |
| ST-19 | ST-18 with `if (i == 1) { continue; }` in the body | runs: `0`, `2`; leak-free | R6, AR-10 |
| ST-20 | `let seq: i32 = 0; func tick(): String { seq = seq + 1; return stringOf("t"); } func main() { let i: i32 = 0; for ($"{tick()}", i = 0; i < 1; i = i + 1) { println($"{seq}"); } }` | runs: `1`; leak-free | R7, AR-11 |
| ST-21 | `let seq: i32 = 0; func mkString(): String { seq = seq + 1; return stringOf("x"); } let i: i32 = 0; for (mkString(), i = 0; i < 1; i = i + 1) { println($"{seq}"); }` | runs: `1`; leak-free | R7, AR-11 |
| ST-22 | `let seq: i32 = 0; func mark(n: i32): i32 { seq = seq * 10 + n; return n; } let p = P { a: mark(1), b: mark(2) };` print `seq` | runs: `12` | R8, AR-12 |
| ST-23 | the same with `P { b: mark(2), a: mark(1) }` | runs: `21` | R8, AR-12 |
| ST-24 | `let v: str = null; panic(v);` | panic-clean: `panic: ` and no sanitizer report | R9, AR-13 |
| ST-25 | `struct H { name: String; } let h = H {}; panic(h.name);` | panic-clean: `panic: ` and no sanitizer report | R9, AR-13 |
| ST-26 | `let flag: bool = true; let x: String? = stringOf("a"); if (x != null) { println($"{flag ? x : null}"); }` | rejected: E10010 at the null (the concrete twin of #26 ST-37) | R4, AR-6 (guard) |
| ST-27 | `let s: String? = stringOf("x"); if (s != null) { println(text(s)); }` | runs: `x` | R1, AR-20 (guard) |
| ST-28 | `func show<T>(x: T): String { return text(x); }` with `show<String?>(stringOf("x"))` unnarrowed | rejected: E10010 at the argument | R2, AR-3, AR-20 |
| ST-29 | the same `show<T>` with `if (x == null) { return stringOf("n"); } return text(x);` and `show<String?>(stringOf("x"))` | runs: `x` | R2, AR-3, AR-20 (guard) |
| ST-31 | `let c: bool = true; let v: str = "v"; let q: str = c ? null : v;` | rejected: E10012 at `null` (the written view destination keeps its pre-existing rule) | R4, AR-5 (guard) |
| ST-32 | `let i: i32 = 0; let n: i32 = 0; for (i++, n = 0; n < 1; n = n + 1) { println($"{i}"); }` | runs: `1` (an increment init now runs; the base prints `0`) | R7, AR-11 |

### 2.1 Red Evidence

On `bb3956c`:

- ST-1/ST-2/ST-3 fail on the missing front-end rejection (ST-1/ST-2 emit
  `cl_text_unknown`/`cl_text_Box`; ST-3's instance emits `cl_text_Box`).
- ST-5 fails because the nullable `text` argument is accepted.
- ST-28 fails because the generic nullable `text` instance is accepted.
- ST-7 fails because `text(x.n)` emits `cl_text_unknown`.
- ST-9 is accepted and emits invalid pointer arithmetic.
- ST-11 is rejected with the wrong E10012+E10010 pair; ST-12/ST-13 are
  rejected with the wrong pair.
- ST-14/ST-15 are accepted and emit `(flag ? NULL : ...)`.
- ST-33 is accepted and emits `(NULL + NULL)`; ST-36 is accepted and emits a
  pointer/integer conditional; ST-39 is accepted and emits `12` from the
  compact step header.
- ST-18/ST-19 leak 49 bytes under ASan.
- ST-20/ST-21/ST-32 print `0` (the effect never runs).
- ST-24/ST-25 report a UBSan null-pointer error.
- ST-4, ST-6, ST-8, ST-10, ST-16, ST-17, ST-22, ST-23, ST-26, ST-27, ST-29,
  ST-30, ST-31, ST-34, ST-35, ST-37, ST-38 are **behavior guards** that pass on
  the base and must keep passing.

The execution plan records the exact failing names and observed base output.

## 3. Implementation Tests

| # | Assertion |
|---|-----------|
| IT-1 | `text(null)` is E10500 with the frozen message/help at the argument span and writes no C |
| IT-2 | A generic `text` instance reports E10500 at the substituted argument, once |
| IT-3 | The one-argument `text(7)` emission is unchanged (`cl_text_i32`) |
| IT-4 | `$"{null + 1}"` reports E10012 at the null span and writes no C |
| IT-5 | The generic `String` null arm reports E10010 only (no E10012) |
| IT-6 | A scalar null arm reports E10012 at the null span |
| IT-7 | A for-step owning assignment emits the flat loop form and a release before the store |
| IT-8 | A non-owning for step keeps the compact C `for` header |
| IT-9 | A for-init template emits its store and release before the loop header |
| IT-10 | The same-order aggregate keeps the initializer; the differing-order aggregate uses the fieldwise store |
| IT-11 | The host and Amiga `cl_panic` bodies guard the message `fwrite` with `len > 0`; each keeps its own stream |
| IT-12 | The #26/#27 String/str copy shapes stay unchanged (existing suites) |
| IT-13 | The `accepts_bare_null` unit test accepts `str`/`static str`/`cstring`/`raw`/slice and rejects `String`/`Box`/scalar/struct/fixed array |
| IT-14 | `(c ? 1 : null) + 1` reports E10012 once (the declaration and binary-argument judge do not double-report) |
| IT-15 | `null + null` reports E10012 once at the left null (the both-null branch) |

## 4. Sanitizer Matrix

- `make asan` discovers both new suites through `RDC_ASAN`; the for-step,
  for-init, and panic cases must run with no ASan report and no leak.
- `make ubsan` does the same through `RDC_UBSAN`; the panic cases must have no
  runtime error.
- The plain spec run forces UBSan for the panic cases so the zero-length
  `fwrite` is a normal red/green signal.

## 5. Benchmark Plan

1. `make bench-base COMMIT=bb3956c` writes `build/perf/base.json`.
2. `make bench` writes `build/perf/head.json`.
3. `make bench-compare` checks every scored output for equality and the 1%
   no-regression guard.
4. `make bench-shape` builds the generated-C shape fixture for m68k GCC,
   vbcc, and freestanding.

No scored program uses a for-step owning assignment, a for-init expression
list, a non-formattable `text` argument, a scalar null arm, or a generic null
arm; the aggregate benchmark already takes the fieldwise store path because
of its fixed-array field. Instructions and `.text` are expected to stay equal;
the generated-C size may grow slightly from the two panic-field guards and
must stay within the 1% guard.

## 6. Traceability

| Requirement | Cases |
|-------------|-------|
| R1 (text formattability) | ST-1, ST-2, ST-5, ST-6, ST-7, ST-27, ST-37, IT-1, IT-3 |
| R2 (generic text) | ST-3, ST-4, ST-28, ST-29, IT-2 |
| R3 (bare null operand) | ST-9, ST-10, ST-30, ST-33, ST-34, ST-38, IT-4, IT-15 |
| R4 (generic null arm) | ST-11, ST-12, ST-13, ST-26, ST-35, IT-5, IT-13 |
| R5 (scalar null arm) | ST-14, ST-15, ST-16, ST-17, ST-36, IT-6 |
| R6 (for-step own assignment) | ST-18, ST-19, IT-7, IT-8 |
| R7 (for-init effects) | ST-20, ST-21, ST-32, IT-9 |
| R8 (aggregate order) | ST-22, ST-23, ST-39, IT-10 |
| R9 (panic safety) | ST-24, ST-25, IT-11 |
| R10 (diagnostic consistency) | ST-1, ST-2, ST-3, ST-5, ST-7, ST-9, ST-12, ST-13, ST-14, IT-1, IT-2, IT-4, IT-5, IT-6, IT-14, IT-15 |
| R11 (emission stability) | ST-8, IT-3, IT-8, IT-12 |

## 7. Test Integrity

Each fix assertion must fail without the fix:

- ST-1/ST-2/ST-3 fail on `bb3956c` on the missing E10500.
- ST-5 and ST-28 fail on the accepted nullable argument.
- ST-7 fails on the opaque projection's `cl_text_unknown`.
- ST-9, ST-33, ST-36, and ST-39 fail on the accepted invalid C/order; ST-30 guards the
  E10218 logical rule; ST-34/ST-35 guard the accepted comparison and the
  non-formattable placeholder arm; ST-37/ST-38 guard the module value shadow
  and the null binding against an open parameter.
- ST-11/ST-12/ST-13 fail on the wrong reports.
- ST-14/ST-15 fail on the accepted invalid C.
- ST-18/ST-19 fail under ASan on the leak.
- ST-20/ST-21/ST-32 fail on the missing effect.
- ST-24/ST-25 fail on the UBSan report.
- IT-13 fails on the base (no helper); IT-14 is a guard that the fix must
  not introduce a double report (the base reports once through the binary
  argument judge).
- The behavior guards pass on the base and must keep passing.
