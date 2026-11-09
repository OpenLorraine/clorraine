# Testing Strategy: Constant Array Initialization (PLAN-25)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The feature is a codegen change with a strict behavior-preservation obligation.
The test strategy has three layers:

1. **Behaviour specification tests** prove that every eligible table produces
   the same values as the current stores, and that every excluded or
   front-end-invalid shape keeps its current behavior or diagnostic.
2. **Emission implementation tests** pin the new C shape and prove that every
   exclusion keeps the old shape.
3. **Measurement and gate evidence** proves the `lookup` win and no scored
   regression.

No new test harness is introduced. The behaviour suite uses the same
compile-build-run pattern as `nested_fill_compaction_spec_test.rs`; the
emission suite uses the same compile-and-inspect pattern as
`nested_fill_compaction_impl_test.rs`. Both suites support the existing
`RDC_CLC` seam. A relative `RDC_CLC` value is resolved against
`CARGO_MANIFEST_DIR/..` (the workspace root) before it is used; the red-run
command therefore works from the Cargo test working directory.

## 🚨 Specification Test Cases

> These cases derive from [Requirements](01-requirements.md) R2/R3, the
> [Design](03-design.md), and the [Ambiguity Register](00-ambiguity-register.md).
> They are the immutable oracle: a failing case means the implementation is
> wrong, not the test.
>
> **Eligibility floor:** every positive template case uses a module-level
> `let` with at least 16 elements and at least 64 bytes under the checked type
> table; 1-byte `i8`/`u8`/`bool` tables are old-store controls because the
> measured u8 template cost exceeds the 1% guard (`i32`/`u32`/`usize`/`isize` 4 bytes, `i16`/`u16` 2 bytes,
> `i8`/`u8`/`bool` 1 byte). The small-table cases are deliberately labelled as
> old-store controls. This keeps the positive and exclusion cases above the
> threshold so each exclusion, not the threshold, is the reason a pin passes.

### Behaviour and emission cases

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | Module-level `let t: i32[32] = [1, 2, ..., 32];` print the sum | prints `528`; template + `memcpy`, no `t[i] =` stores | R2.1, R2.2, AR-04 |
| ST-2 | `let t: i32[32] = [-1, -2, ..., -32];` print the sum | prints `-528`; negative values in the template | R2.1, R3.1 |
| ST-3 | `let t: u8[64] = [1, 2, ..., 64];` print `u32` sum | prints `2080`; old stores, no template (1-byte exclusion) | R2.4, R3.1 |
| ST-4 | `let t: u16[32]` with an explicit list of 32 `1000` values; print `u32` sum | prints `32000`; `u16` template | R2.1, R3.1 |
| ST-5 | `let t: u32[16] = [u32(4294967295), 0, ..., 0];` print `t[0]` | prints `4294967295`; exact unsigned rendering | R2.1, R3.2 |
| ST-6 | `let t: i8[64]` with an explicit list of 64 `1` values; print `i32` sum | prints `64`; old stores, no template (1-byte exclusion) | R2.4, R3.1 |
| ST-7 | `let t: i16[32]` with an explicit list of 32 `1` values; print `i32` sum | prints `32`; `i16` template | R2.1, R3.1 |
| ST-8 | `let t: usize[32]` with an explicit list of 32 `1` values; print `usize` sum | prints `32`; `usize` template (32-bit typedef) | R2.1, R3.1 |
| ST-9 | `let t: isize[32]` with an explicit list of 32 `-1` values; print `isize` sum | prints `-32`; `isize` template (32-bit typedef) | R2.1, R3.1 |
| ST-10 | `let t: bool[64] = [true, false, ...]` (32 true, 32 false); count `true` | prints `32`; old stores, no template (1-byte exclusion) | R2.4, R3.1 |
| ST-11 | `let t: u8[64]` with the first four values `u8(300), u8(255), u8(256), u8(0)` and the rest zero; print `u32` sum | prints `299`; old stores, no template (1-byte exclusion) | R2.4, R3.1 |
| ST-12 | `let t: i8[64]` with the first four values `i8(200), i8(-128), i8(127), i8(0)` and the rest zero; print `i32` sum | prints `-57`; old stores, no template (1-byte exclusion) | R2.4, R3.1 |
| ST-13 | A module-level `i32[32]` initialized `[1, ..., 32]`, then an `unsafe` raw-pointer write of `9` to `t[0]`; print the sum | prints `536`; the target is writable and the template does not alias it | R2.3, R2.5, AR-05 |
| ST-14 | Module A declares `let t: i32[32] = [1, ..., 32];`; module B reads `A.t[0] + A.t[31]` | prints `33`; A initializes before B | R2.5 |
| ST-15 | Module A declares the table; module B declares `let x: i32 = A.t[2];` | prints `3`; the dependency order is preserved | R2.5 |
| ST-16 | `let src: i32[32] = [1, ..., 32]; let copy: i32[32] = src;` print the sum | prints `528`; no template, old element stores | R2.4 |
| ST-17 | `let t: i32[32] = [; 7];` print the sum | prints `224`; no template, the existing fill spelling | R2.4 |
| ST-18 | `volatile let t: i32[32] = [1, ..., 32];` print the sum | prints `528`; no template, per-element volatile stores | R2.4, R2.8 |
| ST-19 | `let t: i32[4][4] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 16]];` print the sum | prints `136`; no template, the existing multi-extent path | R2.4 |
| ST-20 | `struct S { v: i32; }` and `let t: S[32] = [S { v: 1 }, ...];` print the sum | prints `32`; no template, the existing struct path | R2.4 |
| ST-21 | `let t: f32[32]` with an explicit list of 32 `f32(1.5)` values; print `i32` sum | prints `48`; no template, the existing float path | R2.4 |
| ST-22 | `let t: i32[32] = [1, 2];` print the sum | prints `3`; no template, the omitted tail stays zero | R2.1, R2.4 |
| ST-23 | `let t: i32[32] = [];` print the sum | prints `0`; no template, the current empty-list behavior | R2.4 |
| ST-24 | Module-level `let min: i32 = i32(u32(0x80000000)); let neg: i32 = -1;` and `let t: i32[32]` with the first element `min / neg` and the rest zero (a runtime operand avoids a constant-overflow compile warning) | the same `E11210` trap/emission as the base compiler; no template | R2.4, R2.8 |
| ST-25 | `let t: i32[32]` with the first element `i32(f32(1.5))` and the rest zero | prints `1`; the same in-range cast emission as the base compiler; no template | R2.4, R2.8 |
| ST-26 | `let t: i32[32]` with the first element `i32(f32(3.0e9))` and the rest zero | the same `E11212` out-of-range fault as the base compiler; no template | R2.4, R2.8 |
| ST-27 | `let t: bool[64]` with the first element `false && f()` (`f(): bool`, observable effect) and the rest false; count `true` | prints `0`; `f` is not called; no template | R2.4, R2.5 |
| ST-28 | `let t: bool[64]` with the first element `false && (x > 0)` and the rest false; count `true` | prints `0`; `x` is not read; no template | R2.4, R2.5 |
| ST-29 | `let t: i32[32]` with the first element `f()` (returns 7) and the rest zero; print the sum | prints `7`; `f` is called once; no template | R2.4 |
| ST-30 | `let t: i32[32]` with the first element `c ? 1 : 2` (with `c = true`) and the rest zero; print the sum | prints `1`; the conditional is evaluated once; no template | R2.4 |
| ST-31 | Module-level `let min: i32 = i32(u32(0x80000000)); let neg: i32 = -1;` and `let t: i32[32]` with the first element `min % neg` and the rest zero | the same `E11210` trap as the base compiler; no template | R2.4, R2.8 |
| ST-32 | `let t: i32[8] = [1, ..., 8];` print the sum | prints `36`; below the 64-byte floor, no template | R2.1, R3.2 |
| ST-33 | `let t: str[16] = ["aa", ..., "aa"];` print the sum of lengths | prints `32`; no template, the existing view path | R2.4 |
| ST-34 | `let t: i32[32] = [2147483647, -2147483647, 0, ...];` print `t[0]` and `t[1]` | prints `2147483647` and `-2147483647`; template | R3.1, R3.2 |
| ST-35 | `let t: usize[32]` with `usize(7)` first and zero elsewhere; `let u: isize[32]` with `isize(-7)` first and zero elsewhere; print both | prints `7` and `-7`; templates | R3.1, R3.2 |
| ST-36 | `let t: bool[64]` with `bool(1)` first and false elsewhere; count `true` | prints `1`; old stores, no template (1-byte exclusion) | R2.4, R3.2 |
| ST-37 | `let t: i32[0] = [];` | `E10226`; no codegen | R2.4 |
| ST-38 | `let t: i32[] = [1, 2, 3];` | `E10200`; no codegen | R2.4 |
| ST-39 | A module-level handle/owned array element, e.g. a `String[2]` global literal | `E11106`; no codegen | R2.4 |
| ST-40 | `const T: i32[32] = [1, ..., 32];` | `E10224`; no codegen | R2.1, R2.4 |
| ST-41 | `let t: i8[64] = [200, 0, ..., 0];` | `E10202`; no codegen | R2.4 |
| ST-42 | Compile an eligible 32-entry `i32` table for `host`, `hosted-amiga`, `hosted-amiga-13`, and `freestanding` | the first three emit the template; freestanding emits the old stores | R2.1, R2.4, R2.6 |
| ST-43 | Two eligible 32-entry globals in one module, plus an earlier call-initialized global that forces an init local | two distinct `__cl_ctmpl_0`/`__cl_ctmpl_1` templates; no collision with the target, the init local, or a user symbol | R2.2, R3.2 |
| ST-44 | A 64-entry template | the template is extent-sized, `static const`, and wrapped at eight values per line | R2.2, R3.2 |
| ST-45 | The global declaration and the `memcpy` position | the target remains `static`, writable, and the same type/size; the `memcpy` replaces the first store at the same position | R2.3, R2.5, R3.2 |
| ST-46 | The committed `lookup` benchmark host output | prints `lookup: 134217728`; the impl suite separately asserts the template | R1.3, R3.7 |
| ST-47 | A `make bench`/`bench.py run --bench lookup` targeted measurement | prints `lookup: 134217728` on host and m68k; the harness records nonzero instructions and `.text` | R3.7 |
| ST-48 | `make bench-compare` with `REQUIRE="lookup:instructions:10 lookup:text:10"` | the `lookup` row passes both requirements; every other scored row passes the 1% no-regression guard; outputs match | R4.1, R4.2 |

> **Notation:** an ellipsis in a list (`[1, 2, ..., 64]`) means an explicit
> comma-separated continuation, never the Clorraine `[value; count]` fill
> form. The only fill case is ST-17, which expects the old stores.

## Test Categories

### Specification Tests

| Test File | ST Cases | Component |
|---|---|---|
| `compiler/tests/constant_array_initializer_spec_test.rs` | ST-1..ST-41, ST-46 | Global initializer behavior, guards, diagnostics, benchmark host output |
| `compiler/tests/constant_array_initializer_impl_test.rs` | ST-42..ST-45, plus emission assertions for ST-1..ST-41 | Emission shape, profile gate, names, wrapping, exclusions |

The behaviour suite compiles each run case with the selected `clc`, builds the
host C with `cc` (adding `-fsanitize=address` when `RDC_ASAN` is set and
`-fsanitize=undefined -fno-sanitize-recover=all` when `RDC_UBSAN` is set),
runs it, and compares stdout. Trap cases assert the same nonzero exit and
fault code as the base compiler. Compile-fail cases assert the diagnostic code
and no emitted C.

The emission suite runs `clc --profile <profile> -o <temp>.c` and asserts the
template/`memcpy` shape or the old stores. It uses the `RDC_CLC` seam when set
(a relative value is resolved against the workspace root), otherwise
`env!("CARGO_BIN_EXE_clc")`.

### Implementation Tests

| Test File | Description | Priority |
|---|---|---|
| `compiler/tests/constant_array_initializer_impl_test.rs` | Template spelling, values, wrapping, name uniqueness, profile gate, target declaration/position, exclusions | High |
| `compiler/src/codegen/program.rs` `#[cfg(test)]` | The `guard_free_constant` predicate unit cases (literal/cast/unary accepted; Binary/float/conditional/materialize/call/place rejected) | High |

### Integration and Measurement Tests

| Test | Components | Description |
|---|---|---|
| `make check` | Workspace + spec suites | The new suites run with every other suite |
| `make bench-shape` | Emitted C + toolchains | The fixture compiles on hosted-amiga (m68k GCC), hosted-amiga-13 (nix13), vbcc, and freestanding |
| `make corpus-amiga` | Driver + runtime | The portable corpus still builds and runs on hosted-amiga |
| `make differential` | Host/m68k runtime | Host and m68k outputs still match |
| `make bench` | Harness + emitted C | The full suite's `lookup` output and metrics |
| `make bench-compare` | Harness + emitted C | The cumulative `lookup` requirement and the 1% no-regression guard |
| isolated `bench.py compare` | Harness + emitted C | The isolated `lookup` requirement and the 1% guard |
| `make verify` | Full acceptance | All of the above plus the Amiga/emulator legs |

## Test Data

### Fixtures

- The test cases build their own small `.clo` sources in a temporary
  directory; the helper is shared with the existing feature suites.
- The committed `benchmarks/lookup/program.clo` and
  `expected-output.txt` are the measurement fixture.
- `tools/perf/fixtures/shapes.clo` gains a **module-level** eligible
  `i32[64]` table (64 entries, 256 bytes) read by `main`; this exercises the
  template on the hosted legs and the old stores on freestanding.

### Mocks

None. The tests use the real `clc`, the real host C compiler, and the real
`vamos`/size tools for the measurement gates.

## Red/Green Record

| Phase | Run | Expectation |
|---|---|---|
| Red | New suites with `RDC_CLC="$PWD/build/perf/isolated-base-clc"` (absolute) or the pre-change `CARGO_BIN_EXE_clc` | Positive template pins fail; every exclusion pin passes |
| Red | New behaviour suite with the same base compiler | Every run case prints the expected value; trap cases report the expected fault; compile-fail cases report the expected diagnostic |
| Green | New suites with the feature `clc` | All pass |
| Green | `make check`, ASan/UBSan, shape, corpus, differential | All pass |

The red tests are not committed as a standalone red commit. They stay
uncommitted until the implementation makes them green; the red evidence is
recorded in `05-findings.md` and the tests land with the implementation commit.

## Verification Checklist

- [ ] Every ST case has a concrete input and expected output.
- [ ] Every positive and exclusion ST case meets the 16-element/64-byte floor
      unless the case is explicitly a below-threshold control (ST-32).
- [ ] Every ST case traces to R2/R3 or an AR entry.
- [ ] The spec tests are written before the implementation.
- [ ] The positive pins fail on the pre-change compiler and pass on the head.
- [ ] Each exclusion pin fails when its exclusion is removed.
- [ ] The behaviour suite carries `RDC_ASAN`/`RDC_UBSAN` markers and its C
      builder adds the matching flags.
- [ ] `make asan` and `make ubsan` include the behaviour suite.
- [ ] `make bench-shape` covers the template on hosted-amiga, nix13, and vbcc;
      the freestanding leg covers the unchanged stores.
- [ ] `make check`, `make corpus-amiga`, `make differential`,
      `make bench-compare`, and `make verify` pass.
- [ ] No existing test is weakened or deleted; no spec golden is changed.
