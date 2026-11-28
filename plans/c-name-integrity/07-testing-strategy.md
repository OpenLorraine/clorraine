# Testing Strategy: C Name Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Escape predicate, spelling, reservation | every class, position, and family has a case |
| Emission sites | every member-spelling form (03-01 rule) and every symbol site of 03-02 reached by at least one case |
| Driver rewrite | inside, outside, relative, absolute, Windows separators |

- Test names state behaviour (`should … when …`).
- "Builds and prints X on the three profiles" means: host built with the strict flags and run;
  hosted-amiga and hosted-amiga-13 built with m68k GCC and run under `vamos` (skipped with the
  suites' `skip:` message when the toolchain is absent), output compared exactly; hosted-amiga
  also built with vbcc where the case says so.
- A vbcc build skips when `vc` is absent, retries once when the log holds vbcc's transient
  `No config file!` message (the `bench-shape.sh:45-62` pattern), and asserts exit status 0 only;
  warnings from the NDK headers do not fail it.

## Test Slices & Groups

N/A — full verify only (no test map; phase gate `make check`).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived from spec/01 §2–§3, spec/17 §3, §7 and §11, issue #42 and its findings, and the register.
> **IMMUTABLE ORACLE RULE:** do not change these expectations to match the implementation.
> Test comments restate the behaviour in plain language, never the ST/AR/ASM ids.

### Source names (03-01)

| #    | Input / Scenario | Expected Output / Behavior | Source |
|------|------------------|----------------------------|--------|
| ST-1 | Probe 340 (`plans/independent-assessment/probes/WS3/340-c-keywords-as-names.clo`, copied as a fixture): locals `int char signed unsigned long short double float extern typedef goto restrict inline union _Bool`, function `register(auto)` | builds and prints `121` on the three profiles; vbcc builds hosted-amiga | spec/01 §2–§3; spec/17 §7; #61 |
| ST-2 | Probe 341 (copied as a fixture): locals `TRUE FALSE NULL EOF errno stdout INT_MAX CHAR_BIT EXIT_SUCCESS MEMF_CHIP BYTE APTR ULONG SysBase IntuitionBase` | builds and prints `120` on the three profiles; vbcc builds hosted-amiga | #61; AR-5 |
| ST-3 | Locals `__LINE__` = 1, `__func__` = 2, `__STDC__` = 3, `_Pragma` = 4, `__chip` = 5, `_Complex` = 6, `_Imaginary` = 7; print their sum | prints `28` on the three profiles; vbcc builds hosted-amiga | AR-5 class 1–2 |
| ST-4 | A function with locals `memcpy` = 1, `memset` = 2, `strlen` = 3, that also copies an `i32[4]` array `[1, 2, 3, 4]` and a struct holding it; prints `memcpy + memset + strlen + b[3] + q.b[2]` | prints `13` on the three profiles | AR-5 class 3 |
| ST-5 | Locals `textPen` = 1, `detailPen` = 2, `major` = 3, `minor` = 4, `libnix` = 5, `Close` = 6, `FreeString` = 7, in a function that also drops a `String`; print the sum | prints `28` on the three profiles | AR-5, AR-19 |
| ST-6 | `func f(EOF: i32, restrict: i32, errno: i32): i32` returning the sum, called `f(1, 2, 3)`; and a lambda `(TRUE: i32) => TRUE + 1` called with `4` | prints `6 5` on the three profiles | AR-2 |
| ST-7 | `struct Gps { lat: i32; long: i32; }`, `struct S { int: i32; EOF: i32; errno: i32; restrict: i32; detailPen: i32; int_0: i32; }`; build each, copy, read every field, hold one in a module-level `let`, and read `long` once more through a `raw Gps` pointer to a local `Gps` (`unsafe p.long`) | prints `3 21 2` (`1 + 2`; `1 + 2 + 3 + 4 + 5 + 6`; the `long` field read through the pointer) on the three profiles | AR-2, AR-18 |
| ST-8 | An immediately invoked lambda capturing locals `int` = 5 and `EOF` = 6: `([int, EOF] (x: i32) => x + int + EOF)(1)` (a stored closure is E10404) | prints `12` on the three profiles | AR-2 |
| ST-9 | `enum INT : i32 { MIN, MAX, }` and `enum MEMF : u32 { PUBLIC, CHIP, }`, and in the same function locals `INT_MAX: i32 = 7` and `MEMF_CHIP: i32 = 8`; print `i32(INT.MAX)`, `u32(MEMF.CHIP)`, `INT_MAX`, `MEMF_CHIP` | prints `1 1 7 8` on the three profiles; the hosted-amiga emission has no `#define` line whose macro name is exactly `INT_MAX` or `MEMF_CHIP` (match `#define INT_MAX ` with the trailing space) and no `#define INT_` or `#define MEMF_` line for the program's enums | AR-15, AR-16 |
| ST-10 | `enum Color : u8 { Red, }` and a local `Color_Red: i32 = 7`; also `enum A_B : i32 { C, }` and `enum A : i32 { X, B_C, }`; print the local, `u8(Color.Red)`, `i32(A_B.C)`, and `i32(A.B_C)` | prints `7 0 0 1` on the three profiles, in the default and the `--release` build | AR-15, AR-16 |
| ST-11 | A program with locals `count`, `bufferSize`, a parameter `width`, and a field `height`, none in a class | the emitted C spells `count`, `bufferSize`, `width`, `height` exactly; two emissions of the same file are byte-identical | spec/17 §7, §11 |
| ST-25 | `ffi c { func abs(x: i32): i32; }`, then `let g: func(i32): i32 = abs;` and print `g(-5)` | prints `5` on host (an `ffi` function value keeps its C symbol) | spec/17 §7; AR-17 |

### Generated symbols (03-02)

| #     | Input / Scenario | Expected Output / Behavior | Source |
|-------|------------------|----------------------------|--------|
| ST-12 | Probe 342's two files on the command line (`get.clo`, `main.clo`) | builds and prints `1 102` on the three profiles | #64 |
| ST-13 | `module get_` exporting `Point(x: i32): i32` returning `x + 100`, imported as `import { Point as viaGet } from get_;`, beside `struct Point { x: i32; }` and a generic `get<T>(v: T): T` returning `v`, called as `get<Point>(Point { x: 1 })`; print the result's `x`, then `viaGet(2)` | builds and prints `1 102` on the three profiles | AR-8 |
| ST-14 | `module box` exporting `alloc__i32(x: i32): i32` returning `x + 1`, beside `boxNew<i32>(41)` read back through `boxWith`; print the Box value, then `box.alloc__i32(1)` | prints `41 2` on the three profiles (not freestanding: Box is E11200 there) | AR-3, AR-20 |
| ST-15 | `module zero` exporting `ClRec(): i32` returning `5`, beside a moved `struct Rec { a: i32 }` holding `1` | prints `5 1` (the call, then the moved value's field) on the three profiles | AR-20 |
| ST-16 | Two fixtures. (a) A generic `box_alloc<T>(v: T): T` called with `7`, beside `boxNew<i32>(8)` read back through `boxWith`: prints `7 8`. (b) A program whose only lambda is inside a generic `wrap<T>(v: T): T` instantiated at `i32` (so the lambda is site 0 and spells `cl_lambda_0__i32` today), beside a generic `lambda_0<T>(v: T): T` called with `9`; the wrapper's result is not printed: prints `9` | (a) `7 8`, (b) `9`, on the three profiles; the red phase records the duplicate `cl_lambda_0__i32` in (b)'s emitted C | AR-20 |
| ST-17 | `module Main` with a non-generic `helper` and no collision | the emitted C names it `cl_Main_helper` | spec/17 §7; known-limitations §7 |
| ST-26 | `struct Holder<T> { v: T; }`; a generic `get<T>(x: T): i32` returning `1` instantiated at `Holder<i32>`, and a generic `get__Holder<T>(x: T): i32` returning `2` instantiated at `i32`; print both results | prints `1 2` on the three profiles | AR-20 |
| ST-27 | `module string` exporting `view(x: i32): i32` returning `x + 1`; print `string.view(1)` | prints `2` on the three profiles | AR-20 |

### Line mapping (03-03)

| #     | Input / Scenario | Expected Output / Behavior | Source |
|-------|------------------|----------------------------|--------|
| ST-18 | A hosted-amiga emission of a two-function program with a lambda | every top-level item (03-03 definition) after the first `#line` and before the first `#line 1 "compiler/runtime/` is immediately preceded (ignoring blank lines and comments) by a `#line`; the entry stub's names the root file and its `main` line; each function's names its `func` line; the lambda's names its line | spec/17 §11; AR-21 |
| ST-19 | The same emission | the suffix starts with `#line 1 "compiler/runtime/shim_hosted_amiga_20.c"`, and each later shim file and `picasso96.h` starts with its own `#line 1 "compiler/runtime/<file>"` | AR-21 |
| ST-20 | A hosted-amiga emission with one invalid C line (`int cl_bad = ;`) appended at its end, compiled with m68k GCC | the C compiler's error names `compiler/runtime/shim_hosted_amiga_optional_rtg.c` (the last suffix file), not a `.clo` file | AR-21 |

### Driver paths (03-04)

| #     | Input / Scenario | Expected Output / Behavior | Source |
|-------|------------------|----------------------------|--------|
| ST-21 | `clorraine build -p host <abs>/proj/main.clo` with working folder `<abs>/proj` | the emitted C (`build/clorraine/main/host/main.c` under `<abs>/proj`) has `#line N "main.clo"` and no occurrence of `<abs>` | AR-10, AR-24 |
| ST-22 | The same build run as `clorraine build -p host main.clo` from `<abs>/proj` | byte-identical emitted C to ST-21 | AR-24; spec/17 §11 |
| ST-23 | Working folder `<abs>/proj`; `main.clo` imports `sub/lib.clo` and `<abs>/other/x.clo` (outside the folder); command `clorraine build -p host <abs>/proj/main.clo <abs>/proj/sub/lib.clo <abs>/other/x.clo`; C read from `build/clorraine/main/host/main.c` | `#line` names `main.clo` and `sub/lib.clo`; `x.clo`'s lines keep `<abs>/other/x.clo` | AR-24 |
| ST-24 | `clc --profile host <abs>/main.clo` | `#line` keeps `<abs>/main.clo` (unchanged `clc` behaviour) | AR-10 |


## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/c_name_integrity_spec_test.rs` | ST-1..ST-27 | all |

Fixtures live under `compiler/tests/fixtures/c_name_integrity/` (new). Red-phase expectation:
ST-11, ST-17, ST-24, and ST-25 pass before the fix (guards of kept behaviour); every other case
fails.

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/c_name_integrity_impl_test.rs` | completeness test over every available leg (AR-19), the scanner's recall against `-aux-info` on hosted-amiga, sortedness, uniqueness, the `_0` invariant; with `CLORRAINE_RC24_ROOT` set and unusable it fails | High |
| same | the escape function: kept names, `int` → `int_0`, `int_0` → `int_0_0`, `EOF_2` → `EOF_2_0`, `""` → `_0`, injectivity over a generated name set; a triple-shadowed source name never gets a counter spelling that is in a class | High |
| same | generated names keep their spellings: `__cl_tmp0`, `__env`, `__cl_ret`, a generated owner `__cl_owned0`, while a source local `__cl_owned0` becomes `__cl_owned0_0`; every source local and parameter of a sample function is in the locals table | High |
| same | keyword-named fields (`int`, `register`) and a class-3 field (`EOF`) through every member form — value `.`, raw `->`, projection, nested path, global field, drop of an owned field, guard destructuring, `offsetof`, struct literal, capture environment — built with the strict host and m68k flags | High |
| same | every generated family is in the exact sets (instances, Box, zero per struct unit, lambda, hook, fixed prelude helpers) | High |
| same | every top-level item in the covered region has a directive, for one corpus program per profile; the suffix pieces join to `shim_suffix()` | Med |
| same | driver: the `platform.rs` relative-path function for both host families (Windows `\` and `/` separators, Unix `\` kept as a name byte), canonicalization failure passes the spelling | Med |
| `compiler/src/codegen.rs` unit tests | the retired `cl_` asserts (`:1300-1307`) rewritten for `_0` (`int` → `int_0`, `""` → `_0`) | Med |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| corpus and golden suites | all | unchanged outputs on every profile (`make check`, `make corpus-amiga`) |
| `make differential` | host vs hosted-amiga | stdout unchanged |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| rc24 product toolchain | `make toolchain-check TOOLCHAIN=<rc24>`; the rc24 completeness command of 99 (container-built test binary run on the host, AR-27) | both pass |

## Test Data

### Fixtures Needed
Copies of probes 340, 341, and the 342 folder; one sample program for the extraction (03-01).

### Mock Requirements
None; real compilers in the SDK image.

## Verification Checklist
- [ ] All ST cases defined with concrete input/output pairs
- [ ] Every ST case traces to a source
- [ ] Spec tests written before implementation; red phase recorded
- [ ] All spec tests pass after implementation
- [ ] Impl tests written; pins updated per AR-25 and listed
- [ ] Final gate of AR-12 passes
