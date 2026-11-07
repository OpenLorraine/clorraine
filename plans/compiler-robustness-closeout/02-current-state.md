# Current State: Compiler Robustness Closeout

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)
> **Baseline commit**: `96ae0d4` (the starting tree; code-identical to `02877f2`,
> only the roadmap commit differs)
> **Pre-PLAN-14 base**: `0ac87d3e` (retained as `build/perf/base-clc` for D3)
> **Environment**: pinned SDK container; host GCC 12.2.0; `m68k-amigaos-gcc`;
> `vamos`; Python 3.11

## Scope of the Study

The six defects come from the PLAN-14 findings record. Each was reproduced at
the starting head before any change. Reproductions live under the ignored
`build/robustness/probes/` directory; the commands below are the exact ones
used.

The starting head is `02877f2`; the documentation commit `96ae0d4` (the
roadmap parking change) precedes implementation and does not change compiler
code. The emission audit therefore uses `96ae0d4` as its base.

## D1 — Fill operand guards are not emitted

**Reproduction A (bounds):**

```clorraine
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let a: u8[8] = [; src[i]];
```

At the starting head the run prints `a: 0` and exits zero. The emitted C is
eight unguarded reads:

```c
a[0] = src[i];
a[1] = src[i];
...
a[7] = src[i];
```

AddressSanitizer reports `stack-buffer-overflow` with a one-byte `READ` just
before the `a` frame object. Exact commands:

```text
target/debug/clorraine emit -o build/robustness/emit/fill_runtime_index.c \
    build/robustness/probes/fill_runtime_index.clo
cc -std=c99 -O0 -g -Wall -Wextra -Werror -fsanitize=address -no-pie \
    -o build/robustness/fill_runtime_index_asan \
    build/robustness/emit/fill_runtime_index.c
ASAN_OPTIONS=detect_leaks=1:symbolize=0 build/robustness/fill_runtime_index_asan
```

The report ends with:

```text
ERROR: AddressSanitizer: stack-buffer-overflow ... READ of size 1
```

**Reproduction B (division):**

```clorraine
let d: u8 = 0;
let a: u8[4] = [; u8(10) / d];
```

The program dies with `Floating point exception (core dumped)` instead of the
required `error E11210: integer division by zero`.

**Reproduction C (float-to-integer):**

```clorraine
let f: f32 = f32(300.0);
let a: i8[4] = [; i8(f)];
```

The program prints `f: 44` and exits zero instead of reporting
`error E11212: float-to-integer conversion out of range`.

**Reproduction D (struct field):**

```clorraine
struct S { a: u8[4]; }
let s: S = S { a: [; src[i]] };
```

The struct-literal field reaches the same fill lowering and prints `s: 0`
without a guard. The conditional struct arm reaches it too.

**Reproduction E (module initializer):**

A module-level `let plain: u8[4] = [; src[i]];` does trap at `E11211`, but it
emits two identical guards before element 0 and one before each later element
(five guards for four elements). The module path already guards the fill; the
duplication is a code-size defect in the same area.

**Root cause.** `compiler/src/codegen/function.rs:4222` takes the "pure fill"
branch and returns after spelling the per-element stores. The impure branch at
`compiler/src/codegen/function.rs:4238` calls `self.emit_guards(fill)` first;
the pure branch never does. A fill whose operand is a place read or a scalar
conversion is pure by design, so its bounds, division, and float-range guards
are lost. The module-initializer path
(`compiler/src/codegen/program.rs:1842`) emits a guard per element after the
preparation already emitted one.

**Severity.** Critical — a memory-safety violation and a broken fault
contract in accepted source.

## D2 — Narrow arithmetic does not wrap at its result type

**Reproduction:**

```clorraine
println($"inline: {i32(u8(200) + u8(100))}");
```

At the starting head the output is `inline: 300`. The same expression inside a
module `const` prints `300` too. The full probe adds multiplication,
subtraction, a shift, and wider types:

| Expression | Starting head | Required |
|---|---|---|
| `i32(u8(200) + u8(100))` | `300` | `44` |
| `i32(u8(200) * u8(2))` | `400` | `144` |
| `i32(u8(200) * u8(200))` | `40000` | `64` |
| `i32(u8(1) - u8(2))` | `-1` | `255` |
| `i32(u8(128) << 1)` | `256` | `0` |
| `i32(u16(65535) + u16(2))` | `65537` | `1` |
| `i32(u16(65535) * u16(65535))` | strict build fails (`-Werror=overflow`) | `1` |
| `i32(u16(65535) << 15)` | `2147450880` | `32768` |
| `i32(i16(32767) * i16(2))` | `-2` | `-2` (already correct; control) |
| `i32(i8(127) + i8(1))` | `-128` | `-128` (already correct; control) |
| `i32(-u8(1))` | `255` | `255` (already correct; control) |
| `i32(~u8(0))` | `255` | `255` (already correct; control) |
| `i32(i32(2147483647) + i32(1))` | `-2147483648` | `-2147483648` (already correct; control) |

**Root cause.** `compiler/src/codegen/expr.rs:968` renders a binary operation.
The signed `+`, `-`, `*` path casts through the same-width unsigned type and
casts back (`:1024`), so it wraps. The unsigned path and the shift path
(`:1008`) return the promoted C expression unchanged, so C's `int` promotion
carries a value wider than the operation's declared type.

`u16(65535) * u16(65535)` promotes to `int` and overflows; that is undefined
behavior in C, not only a wrong value. A UBSan run reports it. The 16-bit
signed products fit `int`, so `i16(32767) * i16(2)` is already correct; it is a
control. `u16(65535) << 15` also fits `int`, so its defect is the missing
declared-width wrap (2,147,450,880 instead of 32,768), not UB.

## D3 — Inline constant shift divisor

**Reproduction:**

```clorraine
let q: u8 = u8(10) / (u8(128) >> 8);
```

At the pre-PLAN-14 base `0ac87d3e` this aborts with
`error E11210: integer division by zero` (exit 134). At the starting head it
prints `q: 0` and exits zero.

**Evidence.**

| Compiler | Result |
|---|---|
| `build/perf/base-clc` (`0ac87d3e`) | `error E11210: integer division by zero`, exit 134 |
| `target/debug/clc` (`02877f2`) | `q: 0`, exit 0 |

The final PLAN-14 divisor commits fixed the inline form along the way. The
findings table was not updated and no dedicated regression test pins the form.
This plan adds the missing test and records the evidence; it does not claim a
new code change for D3.

## D4 — Narrow `const` fails the strict C build

**Reproduction:**

```clorraine
const U8W: u8 = u8(200) + u8(100);
func main(): i32 { return i32(U8W); }
```

The driver reports `internal compiler error: the emitted C failed to compile`.
The exact strict-build error is:

```text
error: unsigned conversion from 'int' to 'u8' {aka 'unsigned char'}
changes value from '300' to '44' [-Werror=overflow]
```

The emitted initializer is `cl_U8W = ((u8)(200) + (u8)(100));`. A local
initializer, a struct field, and a fill store with the same expression fail the
same way.

**Root cause.** The same missing result-type wrap as D2. The constant stays a
runtime place, so the narrowing happens in the assignment to the `u8` storage
and GCC diagnoses the implicit conversion.

## D5 — `ConstFolder` does not round `f32`

**Reproduction A (cast rounding):**

```clorraine
const N: usize = usize(f32(16777217)) - 16777100;
func main(): i32 { println($"n: {sizeof(u8[N])} {i32(N)}"); return 0; }
```

At the starting head the output is `n: 117 116`. `f32(16777217)` is `16777216`
in binary32, so both values must be `116`. The folder keeps the `f64` value
`16777217.0` and truncates it to `16777217`, while the run-time cast rounds to
binary32 first.

**Reproduction B (declared `f32` const):**

```clorraine
const A: f32 = 16777217.0;
const N: usize = usize(f64(A)) - 16777100;
```

`sizeof(u8[N])` is `117` and `i32(N)` is `116`; the declared place holds the
rounded value, but the folder reference does not.

**Reproduction C (constant float-to-integer overflow):**

```clorraine
const B: f32 = f32(0.0) / f32(0.0);
enum E: i32 { A = i32(B) }
```

The emitted C contains `#define E_A 0` and the program exits zero. The frozen
rule EX-16 requires `E10202` for a constant float-to-integer overflow; NaN is
not representable.

**Root cause.** `compiler/src/consts/mod.rs:292` (`fold_cast`) copies the `f64`
value for an `F32` target without rounding and casts a float to an integer with
`v as i128` (Rust saturates and maps NaN to 0). `compiler/src/consts/arithmetic.rs:180`
(`fold_float`) returns an unrounded `f64` and drops the result's float type.
`compiler/src/consts/mod.rs:414` (`fold_module_const`) only derives an integer
context, so a declared `f32` reference never carries its rounded width.

## D6 — Enum member values disagree between folder and emitter

**Reproduction A (auto-increment):**

```clorraine
enum E: u8 { A = 255, B }
const N: usize = usize(i32(E.B)) + 1;
func main(): i32 { println($"n: {sizeof(u8[N])} {i32(N)}"); return 0; }
```

At the starting head the folder wraps `E_B` to `0` (`sizeof` prints `1`) while
the emitter writes `#define E_B 256` (`i32(N)` prints `257`). The two
evaluators disagree.

**Reproduction B (explicit expression):**

```clorraine
enum E: u8 { A = u8(2) + u8(3), B }
```

At the starting head the emitted C is `#define E_A 0` and `#define E_B 1`; the
program prints `enum: 0 0 1`. The explicit value is a constant expression per
the spec (EN-3), so `E_A` must be `5` and `E_B` must be `6`.

**Reproduction C (conversion emission):**

```clorraine
enum Direction: u8 { UP, DOWN, LEFT = 10, RIGHT }
let d: Direction = Direction(2);
```

The spec's own example fails the strict C build: the emitter spells
`Direction(2)` as a C function call instead of a cast.

**Root cause.** Three independent paths:

- `compiler/src/consts/mod.rs:456` (`fold_enum_member`) walks members, wraps
  each auto-increment at the backing type, and folds explicit expressions.
- `compiler/src/codegen/program.rs:911` (`emit_enum`) only recognizes an
  integer literal (`int_literal_value`) and otherwise falls back to the running
  counter. It never folds an explicit expression and never rejects an
  out-of-range auto-increment.
- `compiler/src/own/lower.rs` lowers every `Call` to `ValueKind::Call`, even
  when the sema stage typed the callee as a type name (`check_type_cast`).

**User ruling.** An auto-incremented member value must fit the backing type. A
value that does not fit is `E10202` at the member, using the registered
message `literal does not fit 'u8'`; there is no wrap and no `256`. This
applies the same rule the checker already uses for explicit values. The
emitter therefore never sees an out-of-range member.

**Remaining value defect.** For an accepted enum, the folder and emitter still
disagree on an explicit expression: `enum E: u8 { A = u8(2) + u8(3), B }`
folds `A` to `5` but emits `#define E_A 0`. The fix makes the emitter fold the
same expression with the folder. The conversion emission is also fixed so the
spec's `Direction(2)` example compiles.

## Verification Baseline

At the starting head, `make check` is the expected green baseline. The new
suites do not exist yet, so all six defects are unguarded. The UBSan leg does
not exist; the ASan runner covers 26 suites. The fuzzer does not exist.

## Measured Gate Costs (starting point)

| Command | Observation |
|---|---|
| `make check` | Rust workspace suite plus spec check; the existing fast gate |
| `make asan` | 26 suites, one process per suite (27 after the robustness suite opts in) |
| `make ubsan` | absent |
| `make corpus-amiga` | m68k GCC + `vamos` per portable program |
| `make verify` | full acceptance matrix; needs licensed media for FS-UAE legs; no `asan` step today |
| `bench-shape` | GCC `nix20`, vbcc, and freestanding build-only legs |

The first implementation task records the measured `make check` wall time so
the smoke budget in R7.10 has a baseline. The plan adds the `nix13` build leg;
`bench-shape` has no host leg, so the host strict build is credited to the spec
suites and `make check`.
