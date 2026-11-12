# Testing Strategy: Generated-C Robustness Bundle

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## 1. Test Layers

| Layer | File | Oracle |
|-------|------|--------|
| Specification behavior | `compiler/tests/generated_c_robustness_spec_test.rs` | R1–R6 and the frozen MOD-13/MOD-16 rules |
| Implementation emission | `compiler/tests/generated_c_robustness_impl_test.rs` | Generated names, call shapes, temp release |
| Prior contracts | #24/#26/#27/#28/#29 suites, `checker_conformance_*` | Unchanged except the intended #29 IT-13 update |
| Sanitizers | `make asan`, `make ubsan` | No report, no leak |
| Benchmark | `make bench` + `make bench-compare` | Equal outputs, 1% guard |

The spec harness follows the #29 suites: emit host C, build with
`cc -std=c99 -O0 -g -Wall -Wextra -Werror`, run, compare stdout; `assert_leak_free`
builds with ASan and asserts no leak; `assert_pair_output` writes a holder +
main and passes both files. Warnings (for example an unused imported `std`) are
allowed; the tests assert stdout/diagnostics, not an empty stderr.

## 2. Specification Cases

| # | Source | Expected | Traces |
|---|--------|----------|--------|
| ST-1 | lexical `let std: Std` with a `fmt.text` field; `println(std.fmt.text(7));` | runs: `u` | R2, AR-7 (guard) |
| ST-2 | module-level `let std: Std`; `println(std.fmt.text(7));` | runs: `7` (module path wins) | R1, AR-1 |
| ST-3 | holder exports `std`; main imports it and calls `std.fmt.text(7)` | runs: `7`; an unused-import warning is allowed | R1, AR-1 |
| ST-4 | module-level `std`; `std.fmt.text(null)` | rejected: E10500 (the baseline accepts and emits invalid `std.fmt.text(NULL)`) | R1, AR-1 |
| ST-5 | user `std.fmt` module with `export func text`; module-level `std`; `std.fmt.text(7)` | runs: `user` | R1, AR-8 (guard) |
| ST-6 | `import { text } from std.fmt; println(text(stringOf("x")));` | runs: `x`; leak-free | R3, AR-4 |
| ST-7 | `import { text } from std.fmt; println(text($"{s}"));` | runs: `abc`; leak-free | R3, AR-4 |
| ST-8 | `let s = stringOf("x"); text(s); println(s);` | runs: `x` | R3, AR-4 (guard) |
| ST-9 | `import { text } from std.fmt; text(stringOf("x"));` discarded | leak-free | R3, AR-4 |
| ST-10 | alias and qualified `text(stringOf("x"))` | leak-free | R3, AR-4 |
| ST-11 | module-level `std` (member `userText` returns `"u"`) and `let n = len(std.fmt.text(1234));` | runs: `4` (module path; the member path would give `1`) | R5, AR-3 |
| ST-12 | non-module-path global `box.text(7)` | runs: `u` | R2, AR-7 (guard) |
| ST-13 | holder exports `std`; main imports it and initializes `let n = len(std.fmt.text(1234));` | runs: `4` (the member path would give `1`) | R5, AR-3 |
| ST-14 | `import { text } from std.fmt; let n = len(text(stringOf("ab")));` in an initializer | runs: `2`; leak-free (the initializer path is already correct) | R3, R5 (guard) |
| ST-15 | holder `std.fmt` exports a func-typed global `text` returning `"gg"`; module-level `std`; initializer `len(std.fmt.text(1234))` | runs: `2` (a real global declaration wins over the intrinsic) | R1, AR-8 (guard) |

Red on `0fdb96c`: ST-2, ST-3, ST-4, ST-6, ST-7, ST-9, ST-10, ST-11, ST-13.
ST-4 currently accepts and emits invalid C rather than reporting E10500. Guards:
ST-1, ST-5, ST-8, ST-12, ST-14, ST-15.

## 3. Implementation Cases

| # | Assertion |
|---|-----------|
| IT-1 | Module-level `std` emits `cl_text_i32((i32)(7))`; no `std.fmt.text(7)` |
| IT-2 | Lexical `std` emits `(std.fmt.text)(7)`; no `cl_text_i32((i32)(7))` |
| IT-3 | An owned body-level text argument is materialized into a temp; a named place is not |
| IT-4 | A user `ffi __cl_itmp0` makes the init local `__cl_itmp1`; the C object-builds strict |
| IT-5 | An ordinary init keeps `__cl_itmp0` |
| IT-6 | A module initializer `std.fmt.text(7)` with module-level `std` emits `cl_text_i32` |
| IT-8 | A user `std.fmt` func-typed global `text` keeps its symbol (`(cl_std_fmt_text)(7)`), not `cl_text_*` |
| IT-9 | A real `std.fmt.text` `ffi` wins in an initializer (`text(1234)`, no `cl_text_*`); the C object-builds strict |
| IT-10 | A non-callable `std.fmt.text` declaration in an initializer is rejected with E10309 and writes no C |
| IT-11 | A real `std.fmt.text` ffi keeps the qualified spelling in an initializer, so a same-named function of the initializing module cannot capture it |
| IT-12 | An imported/bare non-callable `std.fmt.text` declaration is rejected with E10309, like the qualified spelling |

## 4. Sanitizers and Benchmarks

- `make asan` discovers the new suites; the leak cases must be clean.
- `make ubsan` must be clean; no runtime error is expected.
- `make bench-base COMMIT=0fdb96c`, `make bench`, `make bench-compare`;
  `make bench-shape`; `make verify`.

## 5. Traceability

| Requirement | Cases |
|-------------|-------|
| R1 (module path) | ST-2, ST-3, ST-4, ST-5, ST-11, ST-13, ST-15, IT-1, IT-6, IT-8–IT-12 |
| R2 (lexical/global member) | ST-1, ST-12, IT-2 |
| R3 (text arg) | ST-6, ST-7, ST-8, ST-9, ST-10, ST-14, IT-3 |
| R4 (init local) | IT-4, IT-5 |
| R5 (initializer parity) | ST-11, ST-13, ST-14, IT-6 |
| R6 (regressions) | #24/#26/#27/#28/#29 suites, conformance |
