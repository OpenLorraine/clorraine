# Requirements: Compiler Robustness Closeout (PLAN-18)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The compiler must keep its generated C faithful to the language's defined
semantics and must make latent undefined behavior visible before release. This
feature closes five recorded defects, pins a sixth that was already fixed, adds
undefined-behavior sanitization to the acceptance path, and adds a deterministic
differential fuzzer that exercises accepted programs on host and m68k.

The defects are latent because the portable corpus does not use them. The
closeout therefore requires a test for every defect that fails on the starting
compiler and passes after the fix, plus a recorded base/head reproduction.
The one exception is the inline shift divisor, which the final PLAN-14 commits
already fixed; it is pinned by a regression test with pre-PLAN-14 base evidence.

The user ruled on the enum auto-increment ambiguity: a member value that does
not fit the backing type is `E10202`, the existing diagnostic, for both
explicit and auto-incremented values. The full ruling is in AR-11.

## Functional Requirements

### R1 — Fill operand guards

**R1.1 Guard emission.** When an array literal's fill operand needs a guard, a
statement fill emits that guard before the first copied element. The guard
classes are the fixed-array or view index bounds check (`E11211`), the integer
division/remainder zero and `MIN/-1` checks (`E11210`), and the
float-to-integer range and NaN check (`E11212`). A guard-free fill keeps its
current spelling.

**R1.2 Non-conditional operands.** For a non-conditional pure fill the guard is
emitted exactly once. For a conditional pure fill the value expression is
repeated per element by the existing pure-fill shape, so each selected arm's
guard is emitted at that arm's evaluated site; the condition is still
evaluated once. The implementation must not introduce a second guard for a
non-conditional operand, and must not remove a selected arm's guard.

**R1.3 Paths.** The emitted guard is present on every path that reaches the fill
lowering:

| Path | Required behavior |
|---|---|
| pure scalar statement fill | emits the missing guard once |
| conditional pure scalar fill | each selected arm has its guard; the condition once |
| impure scalar fill (call) | keeps its current guard before the temporary |
| struct-literal fixed-array field | emits the guard through the same fill lowering |
| conditional struct arm | emits the guard on the selected arm before the stores |
| module-initializer array fill | emits the fill guard once for the whole fill, including prefix elements; no per-element repetition |
| guarded nested row fill | emits each operand's guard on the evaluated path |

**R1.4 Fault contract.** A guarded fill that faults reports the established
runtime fault line (`error E11210: integer division by zero`,
`error E11211: index out of range`, or
`error E11212: float-to-integer conversion out of range`) and exits nonzero.
Under AddressSanitizer it reports no memory error.

**R1.5 Memory safety and value safety.** These programs must not read outside
their source array and must not silently truncate:

```clorraine
let src: u8[4] = [1, 2, 3, 4];
let i: i32 = 10;
let a: u8[8] = [; src[i]];          // E11211
let d: u8 = 0;
let b: u8[4] = [; u8(10) / d];      // E11210
let f: f32 = f32(300.0);
let c: i8[4] = [; i8(f)];           // E11212
```

**R1.6 Exactly-once proof.** An implementation test counts guard calls inside
the generated function or initializer body, excluding the runtime shim's own
`cl_trap` definition and helpers. The rule is: the text counted is
`cl_trap("E1121x"` between the function/init-body start and end. A guard-free
fill has zero; a fixed non-conditional fill has one; a module fill has one for
the fill plus one per prefix element that the preparation did not guard; a
conditional pure fill has one per selected arm site. The test includes a
negative control.

### R2 — Narrow arithmetic result wrapping

**R2.1 Result-type wrapping.** Generated C wraps an integer `+`, `-`, `*`, or
`<<` at the operation's own result type when that type is narrower than the C
`int` promotion. This applies in function bodies, local initializers, module
`let` initializers, module `const` initializers, struct fields, and array fill
stores.

**R2.2 Safe intermediate arithmetic.** For a result narrower than 32 bits, the
operation is computed in `unsigned int` and cast to the result type. Every
product of two 16-bit values fits in 32 bits, so the intermediate is defined;
the outer cast performs the wrap. A 32-bit result keeps the existing spelling:
`unsigned int`/`u32` arithmetic is already defined and wraps, and the signed
path already casts through `u32`. The generated C must not use `uint32_t`,
which the freestanding profile does not define; it uses the emitted `u32` /
`unsigned int` form.

**R2.3 Operators in scope.** `+`, `-`, `*`, and `<<` are fixed by R2.1. `>>`,
`/`, `%`, `&`, `|`, `^`, and `~` are pinned by constant-context agreement
cases (the same expression folded in an enum member or extent and evaluated at
run time); a pinned difference is a defect and is fixed the same way.

**R2.4 Observable values.** In both a `const` and an inline position:

| Expression | Value |
|---|---|
| `i32(u8(200) + u8(100))` | `44` |
| `i32(u8(200) * u8(2))` | `144` |
| `i32(u8(200) * u8(200))` | `64` |
| `i32(u8(1) - u8(2))` | `255` |
| `i32(u8(128) << 1)` | `0` |
| `i32(u16(65535) + u16(2))` | `1` |
| `i32(u16(65535) * u16(65535))` | `1` |
| `i32(u16(65535) << 15)` | `32768` |
| `i32(i16(32767) * i16(2))` | `-2` (already correct; control) |
| `i32(i8(127) + i8(1))` | `-128` (already correct; control) |
| `i32(-u8(1))` | `255` (already correct; control) |
| `i32(~u8(0))` | `255` (already correct; control) |
| `i32(i32(2147483647) + i32(1))` | `-2147483648` (32-bit control) |

**R2.5 Strict C build.** These forms compile without a warning under the host
strict flags, `m68k-amigaos-gcc` (`nix20` and `nix13`), vbcc, and the
freestanding profile:

```clorraine
const U8W: u8 = u8(200) + u8(100);        // reads 44
const U16W: u16 = u16(65535) * u16(65535); // reads 1
let local: u8 = u8(200) + u8(100);        // reads 44
let fill: u8[64] = [; u8(200) + u8(100)]; // every element is 44
struct S { w: u8; }
let s: S = S { w: u8(200) + u8(100) };    // reads 44
```

**R2.6 No language change.** No grammar, diagnostic, ABI, or frozen-spec edit.
The wrapping rule is the rule the constant folder already applies; the emitter
is made to agree. The 32-bit signed `i32(MAX) + 1` path is a control that the
current signed spelling already handles.

### R3 — Inline constant shift divisor

**R3.1 Correct classification.** A divisor of the form
`(<narrow-typed expression> >> <constant>)` is evaluated at the left operand's
width with the shift count masked before the zero-divisor decision.
`u8(10) / (u8(128) >> 8)` yields `0` and does not trap; a genuinely zero
divisor still traps.

**R3.2 Positions.** The regression case covers the same literal divisor shape
in a local initializer, a module `const` initializer, an array fill, and a
function-argument position. A named-const divisor is pinned separately.

**R3.3 Regression and evidence.** The form is already correct at the starting
head (`02877f2`). The pre-PLAN-14 base (`0ac87d3e`) traps with
`error E11210: integer division by zero`. The change records that base/head
evidence and adds the missing regression test; no code change is claimed where
none is needed. The test helper accepts a `RDC_CLC` override so the same case
can be run against the base binary and its output recorded.

### R4 — `f32` rounding and constant float-to-integer conversion

**R4.1 Cast rounding.** A fold of a cast to `f32` rounds the value to IEEE-754
binary32 (round-to-nearest-even) before any later operation consumes it. A cast
to `f64` widens exactly.

**R4.2 Arithmetic rounding.** A float binary operation whose operands are both
`f32` computes and rounds at binary32; an `f64` operand makes the operation
`f64`. Unary negation keeps the operand's width. The folded value carries its
float type so a later operation chooses the same width as the run time.

**R4.3 Declared float constants.** A reference to a module constant declared
`f32` carries the rounded binary32 value and the `f32` type, not an unrounded
`f64`. The recorded case is:

```clorraine
const A: f32 = 16777217.0;
const N: usize = usize(f64(A)) - 16777100;
```

`A` is `16777216`; `sizeof(u8[N])` and `i32(N)` agree at `116`.

**R4.4 Constant float-to-integer overflow.** A constant float-to-integer
conversion that is NaN or outside the target's range is `E10202` in every
constant context (array extent, enum member, `case`, module `const`
initializer). The range check uses the untruncated value, matching the runtime
`E11212` guard, so `i8(127.5)` is rejected even though its truncation is 127.
The diagnostic uses the registered E10202 message and help
(`literal does not fit '{type}'`, `use a wider type or a hex/binary form`) with
the target type; this is an existing code with a new trigger, not a new
diagnostic. `ConstError` carries the target scalar so the message is complete.

**R4.5 Observable agreement.** A constant context that consumes a
float-derived integer agrees with the run-time conversion. The recorded cast
case is `const N: usize = usize(f32(16777217)) - 16777100;` where
`sizeof(u8[N])` and `i32(N)` must print the same value (`116`).

**R4.6 No materialization.** Floats stay places; this change does not turn a
float constant into an emitted literal.

**R4.7 Finite-value contract.** Rounding covers finite values, `-0.0`, and
infinities. NaN is covered only through the `E11212`/`E10202` conversion checks
and the existing `cl_isnan` bit test. User-level NaN ordering on the m68k
soft-float leg keeps the divergence already recorded in
`spec/17-codegen.md:381-384`; this feature does not claim IEEE NaN comparison
agreement on that leg. A finite `-0.0` and an infinity control are included.

### R5 — Enum member values and conversion

**R5.1 Explicit expression values.** The emitter computes an explicit enum
member value from its constant expression, using the same folder as the
constraint checker. `enum E: u8 { A = u8(2) + u8(3), B }` emits `E_A 5` and
`E_B 6`, not `0` and `1`.

**R5.2 Fit rule and auto-increment.** A member value is representable in the
backing type. An auto-incremented member continues the C sequence
(`previous value plus one`); if that value does not fit the backing type, the
declaration reports `E10202` at the member. `enum E: u8 { A = 255, B }` is
rejected; `enum E: u8 { A = 254, B }` yields `255`. This applies the existing
explicit-value fit rule to auto-incremented values, per the user ruling in
AR-11. No new diagnostic code, message, or help text.

**R5.3 Runtime storage.** For an accepted enum, a member value can be stored in
its enum and read back unchanged under the strict build:

```clorraine
enum E: u8 { A = 254, B }
let x: E = E.B;      // 255
let y: u8 = u8(x);   // 255
```

**R5.4 Constant-context agreement.** A constant context that reads a member
agrees with the emitted member value:

```clorraine
enum E: u8 { A = u8(2) + u8(3), B }
const N: usize = usize(i32(E.A)) + 1;
```

Before the fix the folder reads `5` while the emitter writes `0`; after the fix
both are `5`.

**R5.5 Conversion emission.** A call whose callee names an enum type is a
conversion, not a C function call. `Direction(2)` lowers to the same C cast a
scalar conversion emits; the spec example compiles and runs under the strict
flags. The checker records the exact call site it resolved as a cast after its
value-shadow rule, and the body and module-initializer lowerings read that
record; a local, module function, global, or import with the same written name
keeps the call path. A handle or enum type import is not recognized as a cast
by the current sema resolver (the import is treated as a value), so `Buffer(0)`
and imported enum conversions remain pre-existing limitations recorded in
`docs/known-limitations.md`.

**R5.6 Cross-module members.** The emitter folds a member declared in another
loaded module with that module's constants; the folder and emitter agree there
too. A member may read an earlier member of the same enum (`C = A | B`); a
forward or self reference is still a cycle. The walk uses the target ABI
`sizeof` layout, so an extent that reads a member agrees with the emitted macro
and the runtime C `sizeof`. Enum names are resolved by the language's existing
bare-name rule; a same-name enum in two modules keeps that rule and is recorded
as a collision scope, not newly supported.

A constant-context enum conversion (`E(2)` in an extent or `case`) remains
`E10224`: the frozen constant-expression list does not include enum casts, and
this closeout does not extend it. An imported enum or handle name is not
recognized as a cast by the current sema resolver; both are recorded as
pre-existing limitations in `docs/known-limitations.md`.

### R6 — Host UBSan leg

**R6.1 Target.** Add `make ubsan`, analogous to `make asan`: it discovers every
Rust test suite that references `RDC_UBSAN`, runs each suite in its own bounded
process with one test thread, exports `RDC_UBSAN=1`, and reports the discovered
count. It fails with exit 2 when fewer than two suites are discovered.

**R6.2 Flags.** A UBSan child build uses
`-fsanitize=undefined -fno-sanitize-recover=all`. The runner installs the same
temporary `cc` wrapper pattern as the ASan runner and adds `-no-pie` to any
command that requests `-fsanitize=undefined` or `-fsanitize=address,undefined`,
because the SDK image's GCC 12 runtime is unstable for PIE executables.

**R6.3 Activation self-test.** Before running the suites, the runner builds a
known-undefined-behavior C probe with the same flags, runs it, captures its
stderr, and requires the `runtime error:` report. The probe exits 1 under
`-fno-sanitize-recover=all`; the runner treats a nonzero status as expected and
checks the report text, so `set -e` cannot turn a correct probe into a failure.
A probe built without the sanitizer must not report. If the sanitized probe
does not report, the runner fails with exit 2.

**R6.4 Coverage.** Two suites opt in:

| Suite | Coverage |
|---|---|
| `compiler_robustness_spec_test` | R1–R5 behavior; builds under both `RDC_ASAN` and `RDC_UBSAN` |
| `ubsan_corpus_spec_test` | `buffer-container`, `file-copier`, and `string-parser` built and run under UBSan with no `runtime error:` |

The six intentional-fault programs under `spec/tests/run` are not the UBSan
corpus; they remain the frozen fault oracle. The robustness suite adds its own
`E11210`/`E11211`/`E11212` trap cases under UBSan and treats the compiler's own
fault line as a pass.

**R6.5 Acceptance placement.** `make ubsan` is part of `make verify`, placed
after `check`/`bench-shape`. `make asan` stays a separately recorded gate; this
plan does not add ASan to `verify`. `make ubsan` is not added to `make check`;
the measured runtime is recorded in `05-findings.md`.

**R6.6 No false positives.** A program that traps through the compiler's own
guard (`error E11210:`) is a pass, not a UBSan finding. GCC 12's UBSan does not
include `float-divide-by-zero` or `float-cast-overflow`; those checks are not
enabled, because the language defines float division by zero.

**R6.7 Suite-level control.** When `RDC_UBSAN` is set, the robustness suite
builds a known-UB C probe through its own flag helper and asserts the report;
when `RDC_ASAN` is set, it builds a known-out-of-bounds C probe and asserts the
ASan report. This proves the suite's child builds are sanitized, not only that
the runner's compiler can report.

### R7 — Deterministic grammar-based fuzzer

**R7.1 Location and technology.** `tools/fuzz/fuzz.py`, Python 3 standard
library only, with `tools/fuzz/__init__.py`,
`tools/fuzz/tests/__init__.py`, and `tools/fuzz/tests/test_fuzz.py` following
the `tools/perf/tests` conventions.

**R7.2 Deterministic generation.** Each generated choice is derived from a
SHA-256 digest of `(seed, decision index, name)`; the generator does not use
`random`, hash order, filesystem order, environment input, or wall-clock
input. The same seed and count produce byte-identical programs on every run.
Seed files are sorted by filename before use. The determinism test runs the
generator in two subprocesses with different `PYTHONHASHSEED` values and
compares bytes.

**R7.3 Accepted programs.** The generator is a typed template grammar, not
arbitrary EBNF fragments. It emits only the constructs it is known to accept:
integer and narrow arithmetic, shifts, consts, enums, arrays and fills, structs
with complete literals, bounded loops, and `std.io.println` text. It emits no
`embed`, `ffi`, `unsafe`, raw pointer, file I/O, `assert`, or trapping
operation. Each template family is validated once in `fuzz-test`; a
`compiler-rejected` result is a compiler finding, not a silent skip. The
fuzzer's unit tests scan the committed seeds for the same forbidden constructs.

**R7.4 Differential execution.** Every generated program is compiled and run on
the host profile. A deterministic subset (R7.6) is also compiled and run on
`hosted-amiga` (m68k GCC + `vamos`). Host and target stdout are compared after
CRLF normalization and trailing-line normalization; the exit status is compared
for normal runs. The exact flags are:

| Leg | Flags |
|---|---|
| host | `-std=c99 -O0 -g -Wall -Wextra -Werror` |
| m68k | `-std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror` |
| sanitizer | host flags plus `-fsanitize=address,undefined -fno-sanitize-recover=all -no-pie` |

The sanitizer run sets `ASAN_OPTIONS=detect_leaks=1:symbolize=0` and
`UBSAN_OPTIONS=print_stacktrace=0`.

**R7.5 Sanitizers.** Host programs are built and run under AddressSanitizer and
UndefinedBehaviorSanitizer with leak detection. A report is a `sanitizer`
failure with the report text retained. The `-no-pie` link is mandatory. The
harness disables core dumps (`RLIMIT_CORE=0`) for every child.

**R7.6 m68k leg.** The smoke target runs the host matrix on all programs,
sanitizers on a fixed subset, and m68k on a fixed subset. The long target runs
the host and sanitizer matrices on all programs and m68k on every eighth seed.
The subsets are fixed integer lists in the harness, not wall-clock decisions.

**R7.7 Seeds.** Curated, hand-written programs under `tools/fuzz/seeds/`. Each
seed is named `<name>.clo` and ships one golden:

| File | Meaning |
|---|---|
| `<name>.expected.txt` | exact normalized stdout for a normal run |
| `<name>.expected-fault.txt` | the normalized `error CODE:` line for a trap seed |

A seed golden mismatch is a `golden-mismatch` failure. The committed set is:

| Seed | Covers | Golden |
|---|---|---|
| `narrow_arith.clo` | D2/D4 values const + inline | stdout |
| `narrow_fill.clo` | D2/D4 in a fill store | stdout |
| `fill_bounds.clo` | D1 bounds guard, out-of-range runtime index | fault |
| `fill_divisor.clo` | D1 division guard | fault |
| `fill_float.clo` | D1 float-to-integer guard | fault |
| `struct_fill.clo` | D1 struct-literal field guard, out-of-range runtime index | fault |
| `struct_value.clo` | a normal struct value | stdout |
| `enum_values.clo` | D6 explicit value and stored value, in range | stdout |
| `enum_extent.clo` | D6 explicit-expression constant-context agreement | stdout |
| `f32_round.clo` | D5 cast and declared const agreement | stdout |
| `shift_divisor.clo` | D3 inline shift divisor | stdout |
| `loop_strings.clo` | corpus pattern: strings, arrays, loop | stdout |

There are twelve seeds.

**R7.8 Trap normalization and precedence.** A correct runtime fault differs by
profile: the host writes `error CODE: message` to stderr and aborts; the m68k
shim writes the same line to stdout and exits 20. A seed with a
`expected-fault.txt` golden is classified first: if the extracted normalized
line matches, the seed passes regardless of the raw exit status or signal. Only
a missing or mismatched line is `golden-mismatch`. For a normal seed and every
generated program, an unexpected signal or nonzero status is `crash`, and
stdout must match exactly.

**R7.9 Classification.** Failures are classified as:

| Class | Meaning |
|---|---|
| `invalid-c` | accepted source emitted C that the selected toolchain rejected, including a warning under `-Werror` |
| `divergence` | host and target stdout or exit status differ |
| `sanitizer` | ASan or UBSan reported a finding |
| `crash` | the compiler or the built program died on a signal |
| `hang` | a bounded subprocess timeout expired |
| `compiler-rejected` | a grammar-valid generated program was rejected by `clc` |
| `internal` | the compiler reported an internal error or panicked |
| `golden-mismatch` | a committed seed did not match its golden |
| `harness` | the harness itself failed (missing tool, bad template, I/O); exit 2, not a compiler finding |

Each failure records the seed, class, source path, the emitted C when one was
produced, every log, and the exact commands. The artifacts stay under
`build/fuzz/failures/`; any failure makes the command exit 1 (or 2 for
`harness`).

**R7.10 Gate placement.** `make fuzz-test` runs the unit tests and
`make fuzz-smoke` runs the bounded smoke matrix; both are part of `make check`.
The smoke matrix is:

| Dimension | Count |
|---|---|
| committed seeds | all twelve, host + sanitizer + m68k |
| generated seeds | `0..=15`, host |
| generated sanitizer | `0..=3` |
| generated m68k | `0..=3` |

`make fuzz` is the longer local target: generated seeds `0..=255` on the host
and sanitizer matrices, m68k on every eighth seed. It is not part of
`make check` or `make verify`. If the smoke target exceeds its 60-second
budget after the `cargo build` step, the fallback is `make fuzz-smoke-host`,
which runs only the host and sanitizer matrices, and the m68k subset moves to
`make verify`; the decision and measured times are recorded. The budget covers
the fuzzer process after the workspace build.

**R7.11 Safety.** Every program runs in a private working directory under the
ignored build tree. The generator emits no file I/O. The harness invokes tools
through argument lists, never a shell, and confines every output path under
`build/fuzz/`. No generated program, failure artifact, or log is committed.
The committed seeds are scanned by a unit test for the forbidden constructs.

**R7.12 No new dependencies.** No pip package, no Rust crate, no build script.

**R7.13 Bounded programs.** Every generated program has array extents in
`1..=8`, loop bounds in `0..=8`, no division by a possibly zero value, no
assertion, and no trap. A hang is therefore a compiler or harness finding, not
a normal generated outcome.

### R8 — Tests and documentation

**R8.1 Spec and impl suites.** Add
`compiler/tests/compiler_robustness_spec_test.rs` (behavior) and
`compiler/tests/compiler_robustness_impl_test.rs` (emission and C-shape pins).
Each defect has at least one case that fails on the starting compiler and
passes after the fix. D3 is the exception defined by R3.3: its red evidence is
the pre-PLAN-14 base and its regression test passes at head.

**R8.2 Test integrity.** Every assertion added for a fix fails when the fix is
reverted. The red evidence is recorded per task in `05-findings.md`, including
the exact command and the failing output. The new suite carries both
`RDC_ASAN` and `RDC_UBSAN` so the memory-safety claim is executable, and it has
the suite-level sanitizer controls in R6.7.

**R8.3 Historical pins.** The PLAN-14 cases that encoded the old accepted
behavior are updated exactly where the expected value or the accepted input
changes:

| Case | Old | New |
|---|---|---|
| `should_keep_a_narrow_arithmetic_const_and_its_storage_consistent` | `narrow: 300 300` | `narrow: 44 44` |
| `should_keep_an_enum_member_const_consistent_with_the_emitter` | `enum: 256 256` | `enum: 255 255` with `A = 254, B` |
| `should_keep_an_enum_member_const_as_a_place` | `A = 255, B` | `A = 254, B`; the place pin stays |
| `should_keep_a_shifting_fill_on_the_unrolled_path` (`fill-shift`) | uncast `<<` | the wrapped `(u8)(...)` spelling |
| `should_fold_a_shift_at_the_declared_width` | 32-bit shift pin | unchanged; R2.2 keeps the 32-bit spelling |
| `should_keep_a_narrow_arithmetic_const_as_a_place` | place pin | unchanged; the const still stays a place |

A new compile-fail spec case asserts `enum E: u8 { A = 255, B }` reports
`E10202` with the registered message. No frozen `spec/` golden is edited. The
reason for each change is recorded in `05-findings.md`.

**R8.4 Cross-toolchain shape gate.** Extend
`tools/perf/fixtures/shapes.clo` with the narrow-const, narrow-fill,
narrow-arithmetic-fill, `f32`-const, enum, and runtime-index fill shapes and
add a `hosted-amiga-13` (`-mcrt=nix13`) build leg to
`docker/scripts/bench-shape.sh`. The existing GCC `nix20`, vbcc, and
freestanding legs stay. The fixture's helper function is either called or
documented as an emitted-unused function with its `sizeof` use; `main` uses
every new value.

**R8.5 Documentation.** Update the `AGENTS.md` command and layout tables, add
the roadmap row for this plan, update the specialist-check record, and update
`docs/known-limitations.md` only where a limitation is removed or a new
accepted limit is created.

**R8.6 No plan references in source.** Source and doc comments state durable
rationale in plain language and never name this plan or its identifiers.

### R9 — Acceptance gates

**R9.1 Gate matrix.** Run and record: `make check`, `make asan`, `make ubsan`,
`make corpus-amiga`, `make differential`, `make bench-shape`, and `make verify`.
The final `make verify` must print `verify: all legs passed`.

**R9.2 Emission audit.** Build a base compiler once in task 0.1.3 from the
starting commit `96ae0d4` (code-identical to `02877f2`; the only difference is
the roadmap commit) at `build/robustness/base-96ae0d4-clc`, and copy the
pre-PLAN-14 `0ac87d3e` binary to `build/robustness/base-0ac87d3e-clc` before
any rebuild. Emit every portable corpus program plus every committed
robustness/fuzz seed with both the base and head compilers for the host and
hosted-amiga profiles, and diff the C. Every difference is attributed to
R1–R5; no unrelated emission change is accepted. The `0ac87d3e` binary is used
only for D3's red evidence. The base build is not repeated in Phase 7.

**R9.3 Fault parity.** Add two committed probes for the pure-fill bounds and
division guards under `examples/robustness/` and extend
`docker/scripts/run-fault-amiga.sh` with a per-case source path or a second loop
for those files, so the six frozen `spec/tests/run` cases keep their directory.
Assert the exact fault line and the Amiga trap exit for all eight, and report
the updated count. Do not edit `spec/tests/run`.

**R9.4 Repository state.** Leave the working tree clean and `origin/main`
containing every commit (`git rev-list --count origin/main..HEAD` is `0`).

**R9.5 Blocked media.** If a licensed-media FS-UAE leg cannot run, record the
exact blocker and do not claim a full `make verify`. Partial green legs are
reported as partial.

**R9.6 Review re-gate.** If an independent review finding requires a code or
test change after the full gate, re-run the affected gates and the final
`make verify` on the exact pushed tree.

## Non-Functional Requirements

| ID | Requirement |
|---|---|
| N1 | Host, `hosted-amiga`, `hosted-amiga-13`, and `freestanding` emitted C remain valid C99 and deterministic. The generated C uses the emitted `u32`/`unsigned int`, not `uint32_t`. |
| N2 | No new external crate, package, build script, or container image. |
| N3 | Every subprocess in the fuzzer and sanitizer runners has a timeout and a clear failure message. |
| N4 | The fuzzer never writes outside its ignored build directory; generated programs run in a private working directory with core dumps disabled. |
| N5 | Documentation is written for a junior developer: root cause, invariant, and evidence are explicit. |
| N6 | The added `make check` cost is bounded and measured; a regression in the fast gate is a defect. |

## Acceptance Criteria

1. All six defects are closed; D3 is explicitly shown already closed at the
   starting commit with a regression test and recorded base/head evidence.
2. Every fix has a failing-before/passing-after test; no frozen `spec/` edit.
3. `make ubsan` exists, runs two or more suites, has an activation self-test and
   a suite-level control, and is part of `make verify`.
4. The fuzzer is committed, deterministic, documented, covered by unit tests,
   and has a bounded smoke target in `make check` plus a longer `make fuzz`.
5. `make verify` is green (or every blocked leg is named), the working tree is
   clean, and `origin/main` is up to date.
6. The final report contains the six-defect table, fuzzer coverage and seed
   count, gate evidence, and residual limitations.
7. The enum auto-increment rule is the user's E10202 ruling, recorded in AR-11
   and pinned by a compile-fail test.
