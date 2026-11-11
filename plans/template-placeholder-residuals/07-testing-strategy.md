# Testing Strategy: Template Placeholder Residuals

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## 1. Test Layers

| Layer | File | Oracle |
|-------|------|--------|
| Specification behavior | `compiler/tests/template_placeholder_residuals_spec_test.rs` | R1–R9 and the frozen spec |
| Implementation emission | `compiler/tests/template_placeholder_residuals_impl_test.rs` | The intended diagnostic/emission shapes |
| #26 regression | `compiler/tests/template_placeholder_integrity_{spec,impl}_test.rs` | Unchanged #26 contract |
| #25 regression | `compiler/tests/template_direct_formatting_{spec,impl}_test.rs` | Unchanged #25 allocation/emission contract |
| Nullability regression | `compiler/tests/def_zero_null_spec_test.rs` | Unchanged E10010/E10012 rules |
| Sanitizer | `make asan`, `make ubsan` | No ASan/UBSan report |
| Benchmark | `make bench` + `make bench-compare` | Equal outputs; 1% scored guard |

The spec suite follows the harness of
`template_placeholder_integrity_spec_test.rs`: emit host C with the test
`clc`, build with `cc -std=c99 -O0 -g -Wall -Wextra -Werror`, run, and
compare stdout/exit. Under `RDC_ASAN`/`RDC_UBSAN` it adds the sanitizer flags
and sets `ASAN_OPTIONS=detect_leaks=1:symbolize=0`. A `run_leak_free` helper
builds the emitted C with ASan in the plain run, so the leak cases are a normal
red/green signal even outside the sanitizer leg.

## 2. Specification Test Cases

Each case is an input→expected pair. "Rejected" means exit 1, the named
diagnostic, and no output `.c` file. "Runs" means exit 0 and exact stdout.
"Leak-free" means the ASan-instrumented binary exits 0 with no leak report.

The aggregate cases share this scaffold:

```clorraine
let seq: i32 = 0;
func mark(n: i32): i32 { seq = seq * 10 + n; return n; }
func mkString(): String { return stringOf($"{mark(1)}"); }   // "1", length 1
func mkOther(): String { return stringOf("z"); }             // "z", length 1
func mkPair(): String[2] { return [stringOf($"{mark(1)}"), stringOf("x")]; }
struct Inner { a: String; }
struct P { inner: Inner; b: String; }
struct InnerA { a: String[2]; }
struct PA { inner: InnerA; b: String; }
struct InnerC { a: String; }
struct PC { inner: InnerC; b: String; }
```

The loop cases declare `let sum: usize = 0;` before the loop, add the stated
lengths each iteration, and print `sum` after the loop. `mkPair` marks 1 so the
`12` order is observable; `mkString` marks 1 and returns `"1"` (length 1);
`mkOther` returns `"z"` (length 1).

| # | Source shape | Expected | Traces |
|---|--------------|----------|--------|
| ST-1 | `println($"{null}");` | rejected: E10500, `type 'null' is not formattable`, no C | R1, AR-1 |
| ST-2 | module `let s: str = $"{null}";` | rejected: E10500, no C | R1, AR-1 |
| ST-3 | `let v: str = null; println($"[{v}]");` | runs: `[]` (empty view) | R1, AR-17 |
| ST-4 | `let flag = true; println($"{flag ? null : "x"}");` | runs: an empty line (empty view) | R1, AR-17 |
| ST-5 | `let flag = false; let a: String = stringOf("alpha"); let b: String? = null; println($"{flag ? a : b}");` | rejected: E10010 at `b`, no C | R2, AR-2 |
| ST-6 | `let s: String = flag ? a : b;` with the same `a`/`b` | rejected: E10010 at `b`, no C | R2, AR-2 |
| ST-7 | the same conditional inside `if (b != null)`, with `b = stringOf("beta")`, `flag = true` | runs: `alpha` | R2, AR-11 |
| ST-8 | the same conditional inside `if (b != null)`, with `b = stringOf("beta")`, `flag = false` | runs: `beta` | R2, AR-11 |
| ST-9 | `let flag = true; let x: str = null; let y: str? = "v"; println($"{flag ? x : y}");` unnarrowed | rejected: E10010 at `y` | R2, AR-2 |
| ST-10 | the same `str`/`str?` arms inside `if (y != null)`, `flag = false` | runs: `v` | R2, AR-11 |
| ST-11 | `struct H { name: String; }; let h: H = H {}; println($"{h.name}");` | runs: empty line | R3, AR-3 |
| ST-12 | the same `h`; `println(h.name);` (implicit `String`→`str`) | runs: empty line | R3, AR-3 |
| ST-13 | the same `h`; `println($"{len(h.name)}");` | runs: `0` | R3, AR-3 |
| ST-14 | the same `h`; `println($"{clone(h.name)}");` | runs: empty line | R3, AR-3 |
| ST-15 | the same `h`; `import { text } from std.fmt; println($"{text(h.name)}");` | runs: empty line | R3, AR-3 |
| ST-16 | a bare `$"{1}";` statement in a block | runs; leak-free | R4, AR-4 |
| ST-17 | `for (let i: i32 = 0; i < 3; $"{i}", i = i + 1) { println($"{i}"); }` | runs: `0`, `1`, `2`; leak-free | R4, AR-4 |
| ST-18 | a for step discarding an owning call `mkString()`; print `done` after the loop | runs: `done`; leak-free | R4, AR-4 |
| ST-19 | a for step with a template, `if (i == 1) { continue; }` in the body | runs: `0`, `2`; leak-free | R4, AR-18 |
| ST-20 | the scaffold: `P { inner: Inner { a: mkString() }, b: $"{mark(2)}" }` once; print `seq` | runs: `12` | R5, AR-5 |
| ST-21 | the scaffold: `PA { inner: InnerA { a: mkPair() }, b: $"{mark(2)}" }` once; print `seq` | runs: `12`; valid C under `-Werror` | R5, AR-5 |
| ST-22 | ST-20 inside a 3-iteration loop; `let sum: usize = 0;` and `sum = sum + len(p.inner.a) + len(p.b);` print `sum` | runs: `6`; leak-free | R5, AR-19 |
| ST-23 | ST-21 inside a 3-iteration loop; `sum = sum + len(p.inner.a[0]) + len(p.b);` | runs: `6`; leak-free, no double free | R5, AR-24 |
| ST-24 | `func pick<T>(flag: bool, x: T, y: T): String { if (x != null && y != null) { return $"{flag ? x : y}"; } return stringOf("null"); }` with `pick<String?>(true, stringOf("alpha"), stringOf("beta"))` | runs: `alpha` (unchanged) | R2, R6 |
| ST-25 | `func show<T>(x: T): String { return $"{null}"; }` with no instantiation | rejected at the declaration with E10500 | R1, AR-1 |
| ST-26 | `let flag = true; println($"{flag ? null : stringOf("x")}");` | rejected: E10010 at the null arm (the conditional is `String?`) | R1, AR-23 |
| ST-27 | the scaffold: `let flag = true;` and `PC { inner: InnerC { a: flag ? mkString() : mkOther() }, b: $"{i}" }` in a 3-iteration loop; `sum = sum + len(p.inner.a) + len(p.b);` | runs: `6`; leak-free | R5, AR-24 |
| ST-28 | `let flag = true; let x: String? = stringOf("alpha"); if (x != null) { println($"{flag ? x : null}"); }` | rejected: E10010 at `null` (matches #26 ST-37) | R2, R8, AR-23 |
| ST-29 | `let c: bool = true; let a: str = "a"; let y: str? = "y"; if (y != null) { println($"{i32((c ? a : y)[0])}"); println($"{len(c ? a : y)}"); }` | runs: `97`, `1` (a narrowed conditional as an index and a `len` operand) | R2, AR-11 |
| ST-30 | `func f(c: bool, p: cstring?, q: cstring): i32 { if (p != null) { return i32(len(c ? p : q)); } return 0; }` with `f(true, toCString("x"), toCString("y"))` | rejected: E10010 at `q` (a bare cstring arm is empty-capable) | R2, AR-11 |

### 2.1 Red Evidence

The spec tests are written before the implementation and run against
`b6f12d8`:

- ST-1, ST-2, ST-25 must be rejected but the base accepts ST-25 and reports
  the internal "emitted C failed to compile" error (`cl_text_unknown`) for the
  instantiated cases.
- ST-5, ST-6, ST-9, ST-26 must be rejected but the base accepts them; ST-5,
  ST-6, and ST-26 crash when the null arm is selected.- ST-11 through ST-15 must run but the base segfaults on the zero state.
- ST-16 through ST-19 must be leak-free but the base leaks under ASan.
- ST-20, ST-21 must print `12` but the base prints `21` (and ST-21 may
  emit invalid C).
- ST-3, ST-4, ST-7, ST-8, ST-10, ST-22, ST-23, ST-24, ST-27, ST-28, ST-29,
  ST-30 are **behavior guards** that pass on the base and must keep passing;
  ST-23 and ST-27 are the double-free guards for the fix, ST-28 is the null-arm
  guard that must keep reporting E10010, ST-29 is the narrowed-operand guard,
  and ST-30 is the bare-cstring-arm guard.

The execution plan records the exact failing names and the observed base
output for every red case.

## 3. Implementation Tests

| # | Assertion |
|---|-----------|
| IT-1 | The null-literal diagnostic is E10500 with the frozen message and help at the placeholder span and writes no C |
| IT-2 | The mixed-conditional diagnostic is E10010 at the nullable arm's span and names its root/call; a null arm with a handle sibling settles to `String?` and reports E10010 at the null arm |
| IT-3 | A zero-state placeholder emits the normal `cl_string_copy` path; no compiler special case or new helper name |
| IT-4 | The host and Amiga shims each contain the NULL branch in `cl_string_ptr`, `cl_string_len`, and `cl_string_copy` |
| IT-5 | A discarded template statement emits an owning temporary and a release in the same block |
| IT-6 | A for-step template emits the release inside the step block |
| IT-7 | The adopted aggregate emits `cl_zero` then a fieldwise store into the fresh temporary and a plain struct assignment; no invalid aggregate assignment |
| IT-8 | The aggregate shape is the same on host and hosted-amiga; the host twin builds under `-Werror` (ST-21) |
| IT-9 | The #26 ordered-lowering pins are unchanged: one copy per String/str placeholder, the part adopts it, `$text` never appears |
| IT-10 | The literal-only and pure-scalar template shapes are unchanged |

## 4. Sanitizer Matrix

- `make asan` discovers both new suites through `RDC_ASAN`; the zero-state,
  discarded-template, for-step, and aggregate-loop cases must run with no ASan
  report and no leak.
- `make ubsan` does the same through `RDC_UBSAN`; the null view and zero-state
  view paths must have no runtime error.
- The plain spec run also builds the leak cases with ASan so the leak is a
  normal red/green signal, not only a sanitizer-leg signal.

The Amiga coverage for R3 is the shim parity pin (IT-4) plus the hosted-amiga
build/emission review; the runtime zero-state cases run on host, which is where
the language behavior is defined. `make verify` builds the Amiga legs.

## 5. Benchmark Plan

1. `make bench-base COMMIT=b6f12d8` writes `build/perf/base.json`.
2. `make bench` writes `build/perf/head.json` (host + m68k trace).
3. `make bench-compare` checks every scored output for equality and the 1%
   no-regression guard.
4. `make bench-shape` builds the generated-C shape fixture for m68k GCC,
   vbcc, and freestanding.

The String accessor null checks may add a few instructions to a benchmark that
widens a `String` to `str`. The `strings` benchmark widens in its hot loop, so
the expected delta is small but non-zero; it must stay within 1%. If it does
not, the safe fallback is one inlined NULL test in the generated
`cl_string_view` (keeping all three accessors safe), then re-measure; removing
an accessor check is not allowed. The aggregate, discard, and step changes do
not touch the benchmark programs.

## 6. Traceability

| Requirement | Cases |
|-------------|-------|
| R1 (null placeholder) | ST-1–ST-4, ST-25, ST-26, IT-1 |
| R2 (mixed conditional) | ST-5–ST-10, ST-24, ST-28, ST-29, ST-30, IT-2 |
| R3 (zero state) | ST-11–ST-15, IT-3, IT-4 |
| R4 (discarded template / step) | ST-16–ST-19, IT-5, IT-6 |
| R5 (aggregate order) | ST-20–ST-23, ST-27, IT-7, IT-8 |
| R6 (no language/ABI change) | ST-24, the #25/#26 suites, `make bench-shape` |
| R7 (gate/budget) | §5 and the execution plan's acceptance phase |
| R8 (diagnostic consistency) | ST-1, ST-2, ST-5, ST-9, ST-25, ST-26, ST-28, IT-1, IT-2 |
| R9 (emission clarity) | IT-3–IT-10 |

## 7. Test Integrity

Each fix assertion must fail without the fix:

- ST-1/ST-2/ST-25 fail on `b6f12d8` on the missing E10500.
- ST-5/ST-6/ST-9/ST-26 fail on `b6f12d8` on the missing E10010 (the base
  accepts and crashes when the null arm is selected).
- ST-11–ST-15 fail on `b6f12d8` on the crash.
- ST-16–ST-19 fail on `b6f12d8` under ASan.
- ST-20/ST-21 fail on `b6f12d8` on `21 != 12`.
- ST-23/ST-27 pass on `b6f12d8` but fail under ASan if the adopted aggregate
  omits the zero-fill; they guard the fix.
- ST-28 passes on `b6f12d8` and must keep reporting E10010 at the null arm.
- ST-29 passes on `b6f12d8` and must keep accepting a narrowed conditional as
  an index and a `len` operand.
- ST-30 passes on `b6f12d8` and must keep rejecting the bare cstring arm.
- The impl pins fail on `b6f12d8` on the missing diagnostic/release/store.

A test that passes on `b6f12d8` is classified as a behavior guard in the
findings ledger, not as a red fix case.
