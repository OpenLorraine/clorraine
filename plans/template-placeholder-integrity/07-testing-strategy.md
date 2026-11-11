# Testing Strategy: Template Placeholder Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## 1. Test Layers

| Layer | File | Oracle |
|-------|------|--------|
| Specification behavior | `compiler/tests/template_placeholder_integrity_spec_test.rs` | R1–R8 and the frozen spec |
| Implementation emission | `compiler/tests/template_placeholder_integrity_impl_test.rs` | The intended lowering shapes |
| Existing #25 behavior | `compiler/tests/template_direct_formatting_spec_test.rs` | Unchanged output/allocation contract |
| Existing #25 emission | `compiler/tests/template_direct_formatting_impl_test.rs` | Updated intended pins |
| Existing null module case | `compiler/tests/def_zero_null_spec_test.rs:1510` | Unchanged E10010 |
| Sanitizer | `make asan`, `make ubsan` | No ASan/UBSan report |
| Benchmark | `make bench`, `make bench-compare` | Equal outputs; 1% scored guard |

The spec suite follows the harness pattern of
`template_direct_formatting_spec_test.rs`: emit host C with the test `clc`,
build with `cc -std=c99 -O0 -g -Wall -Wextra -Werror`, run, and compare
stdout/exit. Under `RDC_ASAN`/`RDC_UBSAN` it adds the sanitizer flags and sets
`ASAN_OPTIONS=detect_leaks=1:symbolize=0`. The allocation wrapper is skipped
under a sanitizer.

## 2. Specification Test Cases

Each case is an input→expected pair. "Rejected" means exit 1, the named
diagnostic, and no output `.c` file. "Runs" means exit 0 and exact stdout.

| # | Source shape | Expected | Traces |
|---|--------------|----------|--------|
| ST-1 | `let s: String? = null; println($"{s}");` | rejected: E10010, names `s`, no C | R1, AR-1 |
| ST-2 | `let v: str? = null; println($"{v}");` | rejected: E10010, names `v` | R1, AR-2 |
| ST-3 | `let s: String? = stringOf("alpha"); if (s != null) { println($"{s}"); }` | runs: `alpha` | R1, AR-1 |
| ST-4 | `func get(): String? ...; println($"{get()}");` | rejected: E10010, names `get()` | R1, AR-1 |
| ST-5 | module `let s: str = $"x{get()}";` with `get(): String?` | rejected: E10010, names `get()` (existing behavior) | R1, AR-1 |
| ST-6 | `println($"{mk()} calls={calls}")` with `mk` incrementing `calls` | runs: `x calls=1` | R2, AR-3 |
| ST-7 | `let s: String = stringOf("alpha"); println($"{s}{consume(s)}");` | runs: `alpha5` | R3, AR-4 |
| ST-8 | `let v: str = s; println($"{v}{consume(s)}");` | runs: `alpha5`; ASan clean | R3, AR-5 |
| ST-9 | `println($"{consume(s)}{v}");` | rejected: E10022, names `s` | R3, AR-6 |
| ST-10 | `let x: i32 = 1; $"{x}{consumeSetsX(s)}"` where the callee sets `x = 2` | runs: the pre-call `x` value | R2, AR-9 |
| ST-11 | `$"{consumeSetsX(s)}{x}"` | runs: the post-call `x` value | R2, AR-9 |
| ST-12 | `let s: String = stringOf("alpha"); println($"{s}{s}");` | runs: `alphaalpha` | R2, R3 |
| ST-13 | `show<String>(s, s)` on `func show<T>(x: T, take y: String): String { return $"{x}{consume(y)}"; }` | runs: `alpha5`; ASan clean | R4, AR-7 |
| ST-14 | `show<i32>(7)` with the same generic template shape | runs: `7` | R4, AR-18 |
| ST-15 | `c ? $"{v}{consume(s)}" : stringOf("none")` with `c = true` | runs: `alpha5`; ASan clean | R2, R3, AR-11 |
| ST-16 | `let ok: bool = c && len($"v={mk()}") > 0;` with `c = true` | runs: `true` and `1` | R2, AR-11 |
| ST-17 | `c && len($"{v}{consume(s)}") > 0` with `c = true` | runs without a sanitizer report; the length is the pre-move length | R2, R3, AR-11 |
| ST-18 | `while (i < 2 && len($"v={mk()}") > 0) { ... }` | runs: `2` | R2, AR-11 |
| ST-19 | module `let s: str = $"v={mk()}";` with `mk` incrementing `calls` | runs: `1` | R2, AR-10 |
| ST-20 | module `let s: str = $"a={mk()} b={mk()}";` | runs: `2` (each placeholder once) | R2, AR-10 |
| ST-21 | `println($"literal {{ braces }}");` (no placeholder) | runs: `literal { braces }`; no allocation | R5, AR-17 |
| ST-22 | `let v: str? = stringOf("alpha"); if (v != null) { println($"{v}"); }` | runs: `alpha` | R1, AR-2 |
| ST-23 | `struct H { name: String?; }`; `println($"{h.name}")` unnarrowed | rejected: E10010, names the root `h` | R1, AR-1 |
| ST-24 | `if (h.name != null) { println($"{h.name}"); }` | runs: the field value | R1, AR-1 |
| ST-25 | `let c: bool = false; let s: String = c ? $"{mk()}" : stringOf("none");` | runs: `none`, `calls=0` (a skipped arm evaluates no placeholder) | R2, AR-11 |
| ST-26 | `func show<T>(x: T): String { return $"{x}"; }` with `let s: String? = null; show<String?>(s);` | rejected: E10010 at the placeholder, names `x`; ASan-free base would be 139 | R1, R4, AR-7 |
| ST-27 | `func show<T>(x: T): String { if (x != null) { return $"{x}"; } return stringOf("null"); }` with `show<String?>(stringOf("alpha"))` | runs: `alpha` | R1, R4, AR-7 |
| ST-28 | `let ok: bool = c && len($"{mkString()}") > 0;` with `c = true` and `mkString(): String` | runs: `true`, one call | R2, AR-3, AR-11 |
| ST-29 | module `let s: str = $"v={mkString()}";` with `mkString(): String` | runs: `1` | R2, AR-10 |
| ST-30 | `c ? $"{mkH().name}" : stringOf("none")` and a `&&` variant, with `struct H { name: String; }` and `mkH(): H` | runs: `alpha`; ASan clean | R2, R3, AR-11 |
| ST-31 | `println($"x={(c ? v : mk())}")` with `v: str` and `mk(): str`; `c = false` | runs: `x=z`, `calls=1`; ASan clean | R2, AR-3 |
| ST-32 | `func show<T>(x: T): String { return $"{x}"; }` with `struct H` and `show<H>(h)` | rejected: E10500 at the placeholder, names the type; no C | R4, AR-7 |
| ST-33 | `func name<T>(x: T): String { return $"{x.name}"; }` | rejected: E10500 at the declaration; no C | R4, AR-7 |
| ST-34 | `func pick<T>(flag: bool, x: T, y: T): String { if (x != null && y != null) { return $"{flag ? x : y}"; } return stringOf("null"); }` with `pick<String?>(true, stringOf("alpha"), stringOf("beta"))` | runs: `alpha` | R1, R4, AR-7 |
| ST-35 | `struct H { name: static str; }; func get(): H; println($"{get().name}");` | runs: `alpha`, `calls=1` | R2, AR-3, AR-11 |
| ST-36 | `func first<T>(x: T): String { return $"{x[0]}"; }` with `first<str>("a")` | runs: `97` (the element parameter resolves per instance) | R4, AR-7 |
| ST-37 | `func show<T>(flag: bool, x: T): String { if (x != null) { return $"v={flag ? x : null}"; } ... }` with `show<String?>(true, stringOf("alpha"))` | rejected: E10010 at the null arm | R1, R4, AR-7 |
| ST-38 | `Array<str>`; `println($"{get(xs, 0)[5]}")` | runtime trap: E11211; no out-of-range read | R2, AR-22 |
| ST-39 | `struct Pair { a: i32; b: String; }`; `Pair { a: mark(1), b: $"{txt(mark(2))}" }` | runs: `12` (the earlier field evaluates first) | R2, AR-22 |
| ST-40 | `struct P { a: String; b: String; }`; `P { a: mk(i), b: $"{mk(i + 10)}" }` in a 3-iteration loop | runs: `9`; ASan clean (the pinned owning field is adopted) | R2, AR-22 |
| ST-41 | `struct P { a: String; b: String; }`; `P { a: mk(get(xs, 0)[5]), b: $"{i}" }` with `Array<str>` | runtime trap: E11211 (the pinned call keeps its argument's guard) | R2, AR-22 |

### 2.1 Red Evidence

The spec tests are written before the implementation and run against the
phase-start compiler:

- ST-1, ST-2, ST-4, ST-23, ST-26, ST-32, ST-33 must be **rejected** but the
  phase-start compiler accepts them: ST-1, ST-4, ST-23, ST-26 crash (exit 139
  or invalid C); ST-2 prints an empty line and exits 0; ST-32 reports the
  internal "emitted C failed to compile" error; ST-33 is accepted because no
  instance is emitted (an instantiated member would fail the C build). The
  test fails on the missing diagnostic.
- ST-6, ST-7, ST-8, ST-10, ST-13, ST-15–ST-20, ST-31, ST-35, ST-39 are red
  fix cases: they fail on the wrong call count, corrupt output, or a crash.
- ST-3, ST-5, ST-9, ST-11, ST-12, ST-14, ST-21, ST-22, ST-24, ST-25, ST-27–ST-30,
  ST-34, ST-36, ST-38, ST-40, ST-41 are **behavior guards** that pass before
  the change and must keep passing; they are still written first so the whole
  contract is visible.
- The execution plan records the exact failing test names and the observed
  base output for every red case.

## 3. Implementation Tests

| # | Assertion |
|---|-----------|
| IT-1 | A concrete `String` placeholder emits one `cl_string_copy`, marks the piece `owned`, and emits one adopting `cl_str_part_owned`; no nested copy |
| IT-2 | A `str` call placeholder's emitted body contains exactly one call to the function |
| IT-3 | The copy assignment appears before the `take` call assignment in the host C |
| IT-4 | `$"token-{i}-{i * u32(3)}"` keeps two direct `cl_str_part_i32`/`u32` parts and adds no placeholder temporary |
| IT-5 | The emitted C never contains `$text` |
| IT-6 | A generic `String` instance emits the copy before the `take` call |
| IT-7 | A module initializer emits one call for a `str`/`String` call placeholder, releases an owned source at init end, and retains the copy local (no `FreeString` of it) |
| IT-8 | The `hosted-amiga` profile emits the same copy-before-call order as host |
| IT-9 | A short-circuit copy is an adopting fill: no `FreeString` of the copy temporary, and the C builds under `-Werror` |
| IT-10 | The part count, literal byte parts, and scalar kind codes are unchanged |
| IT-11 | A nullable placeholder reports E10010 with the placeholder's name/span and writes no output C |
| IT-12 | A nullable generic instance reports E10010 at the placeholder span; a narrowed instance emits the copy and no diagnostic; a non-formattable instance reports E10500 |
| IT-13 | An adopted copy is a plain `target = value;` with no previous-value release and its bounds guard emitted |

The #25 impl pins for a `String`/`str` placeholder change intentionally: the
copy now binds to a temporary before the concat and the part adopts it. Only
the intended emission change is updated; the `strings` benchmark's scalar-only
shape stays identical (IT-4).

## 4. Sanitizer Matrix

- `make asan` discovers both new suites through `RDC_ASAN`; each view/take,
  generic, guarded, and loop case must run with no ASan report and no leak.
- `make ubsan` does the same through `RDC_UBSAN`; the guarded and generic
  cases must have no runtime error.
- The allocation cases skip under a sanitizer, as in the #25 suite.

## 5. Benchmark Plan

1. `make bench-base COMMIT=8b4bff2` writes `build/perf/base.json`.
2. `make bench` writes `build/perf/head.json` (host + m68k trace).
3. `make bench-compare` checks every scored output for equality and the 1%
   no-regression guard.
4. `make bench-shape` builds the generated-C shape fixture for m68k GCC,
   vbcc, and freestanding.

The expected `strings` result is **neutral**: the hot loop's scalar
placeholders keep their direct parts. The final `toHex(total)` placeholder is
a `String`-returning call, which already binds once and copies once, so the
fix does not remove a duplicate there; the `str`-call win is exercised by
ST-6/ST-16/ST-18 instead. Any scored regression above 1% blocks acceptance and
reopens AR-9.

## 6. Traceability

| Requirement | Cases |
|-------------|-------|
| R1 (nullable) | ST-1–ST-5, ST-22–ST-24, ST-26, ST-27, ST-34, ST-37 |
| R2 (once, in order) | ST-6, ST-10–ST-20, ST-25, ST-28, ST-30, ST-31, ST-35, ST-38–ST-41 |
| R3 (copy before move) | ST-7–ST-9, ST-12, ST-15, ST-17, ST-30 |
| R4 (generic) | ST-13, ST-14, ST-26, ST-27, ST-32, ST-33, ST-34, ST-36 |
| R5 (no language change) | ST-21, the existing #25 suite, `make bench-shape` |
| R6 (gate/budget) | §5 and the execution plan's P3 tasks |
| R7 (diagnostic consistency) | ST-1, ST-2, ST-4, ST-5, ST-23, ST-26, ST-32, ST-33, IT-11, IT-12 |
| R8 (emission clarity) | IT-1–IT-10 |

## 7. Test Integrity

Each fix assertion must fail without the fix. The execution plan records:

- the exact failing test names on the phase-start compiler,
- the observed base exit code/stdout for each red case,
- the same test passing after the implementation,
- the emission pins failing on the base shape and passing on the intended
  shape.

A test that passes on the phase-start compiler is classified as a behavior
guard, not a red fix case, and is labeled as such in the findings ledger.
