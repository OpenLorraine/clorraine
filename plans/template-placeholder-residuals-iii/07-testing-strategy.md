# Testing Strategy: Template Placeholder Residuals III

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## 1. Test Layers

| Layer | File | Oracle |
|-------|------|--------|
| Specification behavior | `compiler/tests/template_placeholder_residuals_iii_spec_test.rs` | R1–R10 and the frozen spec |
| Implementation emission | `compiler/tests/template_placeholder_residuals_iii_impl_test.rs` | The intended diagnostic/emission shapes |
| #28 regression | `compiler/tests/template_placeholder_residuals_ii_{spec,impl}_test.rs` | Unchanged PLAN-32 contract |
| #24 regression | `compiler/tests/text_arity_{spec,impl}_test.rs` | Unchanged PLAN-33 contract |
| #26/#27/#25 regression | `template_placeholder_integrity_*`, `template_placeholder_residuals_*`, `template_direct_formatting_*` | Unchanged earlier contracts |
| Nullability regression | `def_zero_null_spec_test`, `def_semantic_parity_integrity_spec_test`, `checker_conformance_*` | Unchanged E10010/E10012/E10218 rules |
| Sanitizer | `make asan`, `make ubsan` | No ASan/UBSan report |
| Benchmark | `make bench` + `make bench-compare` | Equal outputs; 1% scored guard |

The spec harness follows the PLAN-32 suites: emit host C, build with
`cc -std=c99 -O0 -g -Wall -Wextra -Werror`, run, and compare stdout/exit. Under
`RDC_ASAN`/`RDC_UBSAN` it adds the sanitizer flags and sets
`ASAN_OPTIONS=detect_leaks=1:symbolize=0`, `UBSAN_OPTIONS=halt_on_error=1`.
`assert_leak_free` builds with ASan and bounds the child at 30 seconds;
`assert_panic_clean` forces UBSan and asserts the prefix, no runtime error, and
a non-zero exit.

Two aliases need a declaring module beside the program. The harness adds
`run_pair(tag, holder, main)` / `reject_pair(...)`, writing `holder.clo` and
`main.clo` into the scratch directory and passing both paths to `clc`, exactly
as a build does.

## 2. Specification Test Cases

"Rejected" means exit 1, the named diagnostic, and no output `.c` file.
"Runs" means exit 0 and exact stdout. "Leak-free" means an ASan build exits 0
with no leak report. "Panic-clean" is the forced-UBSan helper.

### 2.1 Text spellings

| # | Source shape | Expected | Traces |
|---|--------------|----------|--------|
| ST-1 | `import { text as t } from std.fmt; println(t(7));` | runs: `7` | R1, AR-1 |
| ST-2 | the same alias with `t(null)` | rejected: E10500, `type 'null' is not formattable`, no C | R1, AR-1 |
| ST-3 | the same alias with `struct Box { n: i32; } t(Box { n: 1 })` | rejected: E10500, `type 'Box' is not formattable` | R1, AR-1 |
| ST-4 | `let s: String? = stringOf("x"); println(t(s));` unnarrowed | rejected: E10010 at `s` | R1, AR-1 |
| ST-5 | the same inside `if (s != null)` | runs: `x` | R1, AR-1 |
| ST-6 | `println(std.fmt.text(7));` | runs: `7` | R2, AR-2 |
| ST-7 | `std.fmt.text(null)` | rejected: E10500 | R2, AR-2 |
| ST-8 | `std.fmt.text(Box { n: 1 })` | rejected: E10500 | R2, AR-2 |
| ST-9 | the alias with `t()` and `t(1, 2)` | rejected: E10308, frozen help, no C | R1, AR-1 |
| ST-10 | a holder `my.other` exporting `helper(i32, i32): String`; `import { helper as t } from my.other; println(t(1, 2));` | runs: `two` (an unrelated alias is not the intrinsic) | R10, AR-1 (guard) |
| ST-11 | the alias import plus a lexical `let t: func(i32): String = userText;` | runs: `local` (the local wins) | R10, AR-1 (guard) |
| ST-12 | `func show<T>(x: T): String { return t(x); }` with `show<Box>` | rejected: E10500 at the instance | R1, AR-1 |
| ST-13 | the same with `show<i32>(7)` | runs: `7` | R1, AR-1 |
| ST-14 | a holder `module std.fmt; export func text(n: i32): String` and `println(std.fmt.text(7));` | runs: `user` (a real declaration wins) | R2, AR-2 (guard) |
| ST-15 | a local `std` value with a `fmt.text` field; `println(std.fmt.text(7));` | runs: `u` (a value head makes it a member path) | R2, AR-2 (guard) |
| ST-46 | a holder `lib.coll2` exporting `t(take s: String)`; main imports it as `other` and `text as t` from `std.fmt`; `t(7)` | runs: `7` (an unrelated same-named function cannot capture the alias or supply its modes) | R10, AR-1 |
| ST-47 | module initializers: `len(text(7))`, `len(t(78))`, `len(std.fmt.text(789))` with `text, text as t` imported | runs: `123` (every spelling canonicalizes in the initializer walk) | R1, R2, AR-3 |
| ST-48 | `ffi text { func foo(): i32; }` plus `println(text(7));` | runs: `7` (an `ffi` block name is not a callable signature) | R10, AR-1 (guard) |

### 2.2 Generic condition context

| # | Source shape | Expected | Traces |
|---|--------------|----------|--------|
| ST-16 | `func show<T>(c: bool, x: T): i32 { if (c ? null : x) { return 1; } return 0; }` with `show<i32>(false, 1)` | rejected: E10218 only, no E10012 | R3, AR-4 |
| ST-17 | the same with `show<String>(false, stringOf("x"))` | rejected: E10218 only (the base accepts) | R3, AR-4 |
| ST-18 | `func show<T>(c: bool, x: T): i32 { if (c ? x : null) { return 1; } return 0; }` with `show<bool>(true, false)` | rejected: E10012 only (a bool settlement keeps the null-arm rule) | R3, AR-4 (guard) |
| ST-19 | `func show<T>(flag: bool, c: bool, x: T): bool { return flag && (c ? null : x); }` with `show<i32>` | rejected: E10218 + E10012 (the logical operand also carries the operand judge) | R3, AR-4 |
| ST-20 | `func show<T>(c: bool, x: T): i32 { while (c ? null : x) { return 1; } return 0; }` with `show<i32>` | rejected: E10218 only | R3, AR-4 |
| ST-21 | `func show<T>(c: bool, x: T): i32 { assert(c ? null : x); return 0; }` with `show<i32>` | rejected: E10218 only | R3, AR-4 |

`do { } while (...)`, a for-C condition, and the `!` operand use the same
`check_condition_value` route as `if`/`while`/`assert`; the representative
cases above pin the route, and the existing semantic-parity suite keeps the
remaining spellings green.
| ST-22 | the generic conditional nested in a value operand (`if (-(c ? null : x) > 0)`, `T = i32`) | rejected: E10012 (the condition flag is one-shot) | R3, AR-4 (guard) |
| ST-23 | the concrete twin `if (c ? null : 5)` | rejected: E10218 only | R3, AR-4 (guard) |
| ST-49 | the generic `flag && (c ? null : x)` instantiated at `i32` and `String` | rejected: one E10218 and one E10012 across the site | R3, AR-4 (guard) |

### 2.3 Both-null conditional

| # | Source shape | Expected | Traces |
|---|--------------|----------|--------|
| ST-24 | `let z = c ? null : null; if (z == null) { println("n"); }` | runs: `n` | R4, AR-5 (guard) |
| ST-25 | `let z: i32 = c ? null : null;` | rejected: E10012 at both null arms | R4, AR-7 (guard) |
| ST-26 | `let s: String? = c ? null : null; if (s == null) { return 0; }` | runs | R4, AR-6 (guard) |
| ST-27 | `let z = c ? null : null; let y: i32 = z;` | rejected: E10012 at `z` | R4, AR-5 (guard) |
| ST-28 | `println($"{c ? null : null}");` | rejected: E10500 | R4, AR-8 (guard) |
| ST-29 | `if (c ? null : null) { }` | rejected: E10218 | R4, AR-5 (guard) |
| ST-30 | `let p: cstring? = c ? null : null;` | runs | R4, AR-6 (guard) |
| ST-31 | `func f(n: i32): i32`; `f(c ? null : null)` | rejected: E10012 at both null arms (a typed call argument has a destination) | R4, AR-7 (guard) |
| ST-32 | `func id<T>(x: T): T`; `let y: i32 = id<i32>(c ? null : null);` | rejected: E10012 at both null arms | R4, AR-7 (guard) |

### 2.4 Null compared with an open parameter

| # | Source shape | Expected | Traces |
|---|--------------|----------|--------|
| ST-33 | `func f<T>(x: T): bool { return null == x; } f<i32>(1)` | rejected: E10012 at the null, no C | R5, AR-9 |
| ST-34 | the same with `null != x` and `T = i32` | rejected: E10012 at the null | R5, AR-9 |
| ST-35 | the right-literal `x == null` with `T = String?` and a null argument | runs: `true` (the narrowing idiom stays accepted) | R5, AR-9 (guard) |
| ST-36 | the same with `x == null` and `T = String`, argument a non-null `String` | runs: `false` | R5, AR-9 (guard) |
| ST-37 | the same with `x != null` and `T = str`, argument `"x"` | runs: `true` | R5, AR-9 (guard) |
| ST-38 | `func show<T>(x: T): String { if (x == null) { return stringOf("n"); } return text(x); }` with `show<String?>` | runs: `x` | R5, AR-9 (guard) |
| ST-39 | the same comparison with `T = Box<i32>?` and a null argument | runs: `true` | R5, AR-9 (guard) |
| ST-40 | `func f<T>(x: T): bool { let z = null; return z == x; } f<i32>(1)` | rejected: E10012 at `z` (the binding form keeps its rule) | R5, AR-9 (guard) |

### 2.5 Runtime and lowering

| # | Source shape | Expected | Traces |
|---|--------------|----------|--------|
| ST-41 | `let p: cstring = null; panic(p);` | panic-clean: `panic: ` and no UBSan report | R6, AR-11 |
| ST-42 | `panic(toCString("boom"));` | panic-clean and the message `boom` appears | R6, AR-11 (guard) |
| ST-43 | `for (...; a = mk3(), ...)` with `a: i32[3]` | runs; leak-free | R7, AR-12 |
| ST-44 | `for (...; a = s.vals, ...)` with a fixed-array member | runs; leak-free | R7, AR-12 |
| ST-45 | `for (...; a = [1, 2, 3], ...)` | runs (the array-literal path stays flat) | R7, AR-12 (guard) |

### 2.6 Red Evidence

On `541ba27`:

- ST-1/ST-2/ST-3/ST-4/ST-5/ST-6/ST-7/ST-8/ST-9/ST-12/ST-13 fail on the alias
  or qualified spelling: accepted and emitting the undeclared name (ST-1, ST-5,
  ST-6, ST-9, ST-13) or accepted with the intrinsic judgment skipped (ST-2,
  ST-3, ST-4, ST-7, ST-8, ST-12).
- ST-16/ST-20/ST-21 fail on the extra E10012 report from the instance
  duty; ST-17 is accepted on the base because no instance duty reports at
  all; ST-19 is red because the base misses E10218 (it reports only the
  operand's E10012).
- ST-33/ST-34 fail on the accepted invalid C (the left-literal form).
- ST-46 fails on the captured alias; ST-47 fails on the initializer spellings;
  ST-49 fails on the missing E10218.
- ST-41 fails on the `strlen(NULL)` crash.
- ST-43/ST-44 fail on the invalid compact-header C.

The guards are ST-10, ST-11, ST-14, ST-15, ST-18, ST-22–ST-32, ST-35–ST-40,
ST-42, ST-45, and ST-48. They pass on the base and must keep passing.

## 3. Implementation Tests

| # | Assertion |
|---|-----------|
| IT-1 | The alias emits `cl_text_i32((i32)(7))` and no `t(` call |
| IT-2 | The qualified spelling emits `cl_text_i32(...)` and no `std.fmt.text` |
| IT-3 | The alias `t(null)` reports E10500 at the argument with the frozen help and writes no C |
| IT-4 | The generic condition instance reports E10218 at the conditional span and no E10012 |
| IT-5 | The inferred both-null conditional emits valid C and builds strict with no deferred diagnostic |
| IT-6 | The deferred null comparison reports E10012 at the literal-null span and writes no C |
| IT-7 | All three shims null-test the message in `cl_cstring_len` (`text == NULL` on host/Amiga, `text == 0` on freestanding) |
| IT-8 | The fixed-array step emits the flat loop and the store's hidden destination/element form |
| IT-9 | A non-owning step keeps the compact C `for` header |
| IT-10 | Bare `text(7)`, alias, and qualified intrinsics emit the same `cl_text_*` shape; no name-based `text` arm remains, so a user `ffi text` keeps its symbol |
| IT-11 | A user `std.fmt.text` pair emits the user symbol, not `cl_text_*`; an aliased or bare user `std.fmt.text` `ffi` declaration keeps its `ffi` symbol |
| IT-12 | The #24/#28 emission/diagnostic pins stay unchanged (existing suites) |
| IT-13 | A module-level value head keeps the member path and is not captured by the intrinsic (emit-only; the global member-path emission is a pre-existing limitation) |

## 4. Sanitizer Matrix

- `make asan` discovers both new suites through `RDC_ASAN`; the alias-text
  ownership, step-array, and both-null cases must run with no ASan report and
  no leak.
- `make ubsan` does the same through `RDC_UBSAN`; the cstring panic case must
  have no runtime error.
- The plain spec run forces UBSan for the panic cases so the null pointer is a
  normal red/green signal.

## 5. Benchmark Plan

1. `make bench-base COMMIT=541ba27` writes `build/perf/base.json`.
2. `make bench` writes `build/perf/head.json`.
3. `make bench-compare` checks every scored output for equality and the 1%
   no-regression guard.
4. `make bench-shape` builds the generated-C shape fixture for m68k GCC,
   vbcc, and freestanding.

No scored program uses a changed shape; the expected scored delta is zero. The
cstring helper guard adds a few bytes to the generated C, within the C-size
advisory (the scored guard is instructions and `.text`).

## 6. Traceability

| Requirement | Cases |
|-------------|-------|
| R1 (aliased text) | ST-1, ST-2, ST-3, ST-4, ST-5, ST-9, ST-12, ST-13, ST-46, ST-47, ST-48, IT-1, IT-3 |
| R2 (qualified text) | ST-6, ST-7, ST-8, ST-14, ST-15, ST-47, IT-2, IT-11, IT-13 |
| R3 (generic condition) | ST-16, ST-17, ST-18, ST-19, ST-20, ST-21, ST-22, ST-23, ST-49, IT-4 |
| R4 (both-null) | ST-24, ST-25, ST-26, ST-27, ST-28, ST-29, ST-30, ST-31, ST-32, IT-5 |
| R5 (null vs parameter) | ST-33, ST-34, ST-35, ST-36, ST-37, ST-38, ST-39, ST-40, IT-6 |
| R6 (cstring panic) | ST-41, ST-42, IT-7 |
| R7 (array step) | ST-43, ST-44, ST-45, IT-8, IT-9 |
| R8 (diagnostics) | ST-2, ST-3, ST-4, ST-7, ST-8, ST-9, ST-16, ST-25, ST-33, IT-3, IT-4, IT-6 |
| R9 (emission stability) | ST-18, ST-23, ST-36, ST-42, ST-45, IT-9, IT-10, IT-12 |
| R10 (shadow) | ST-10, ST-11, ST-14, ST-15, IT-11, IT-13 |

IT-11 emits two pairs: the user function pair and a bare/aliased `ffi text`
pair. The ffi pair is emit-only (the symbol has no C definition in the test),
so it asserts the emitted `text(...)` symbol and the absence of `cl_text_*`.

## 7. Test Integrity

Each fix assertion must fail without the fix:

- ST-1/ST-2/ST-3/ST-4/ST-5/ST-6/ST-7/ST-8/ST-9/ST-12/ST-13 fail on the base
  alias or qualified path.
- ST-16/ST-17/ST-20/ST-21 fail on the extra E10012; ST-19 and ST-49 fail
  on the missing E10218.
- ST-33/ST-34 fail on the accepted left-literal pointer/integer comparison.
- ST-41 fails on the null `strlen`.
- ST-43/ST-44 fail on the strict C build.
- The guards pass on the base and must keep passing; ST-18 and ST-22 prove the
  condition fix does not suppress a bool settlement or a nested value use.
