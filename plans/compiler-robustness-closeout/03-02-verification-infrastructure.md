# Verification Infrastructure: UBSan and Fuzzing

> **Document**: 03-02-verification-infrastructure.md
> **Parent**: [Index](00-index.md)

## 1. Host UBSan leg

### 1.1 Runner

`docker/scripts/ubsan-suite.sh` mirrors `docker/scripts/asan-suite.sh`:

1. `cd` to the repository root.
2. `suite_timeout="${UBSAN_SUITE_TIMEOUT:-900}"`.
3. Install a temporary `cc` wrapper in a `mktemp -d`; the wrapper matches a
   command containing `-fsanitize=undefined` or
   `-fsanitize=address,undefined` and appends `-no-pie`, then execs the real
   compiler. Every other command passes through.
4. Run the activation self-test with `set +e` semantics: compile a known-UB C
   probe with `-fsanitize=undefined -fno-sanitize-recover=all -no-pie`, run it,
   capture stderr, and require `runtime error: signed integer overflow`. The
   probe exits 1 under `-fno-sanitize-recover=all`; a nonzero status is
   expected. A probe built without the sanitizer must not report. If the
   sanitized probe does not report, exit 2.
5. Discover `grep -l 'RDC_UBSAN' compiler/tests/*.rs | sort`. Require at least
   two suites (exit 2 otherwise).
6. Run each suite with
   `timeout --kill-after=30 "$suite_timeout" env RDC_UBSAN=1 cargo test --test "$suite" -- --test-threads=1`.
7. Print `ubsan: <suite>` per suite and `ubsan: <count> suites` at the end;
   return nonzero if any suite fails.

The known-UB probe is:

```c
#include <limits.h>
int main(void) {
    volatile int value = INT_MAX;
    value = value + 1;
    return value == 0;
}
```

`ulimit -c 0` is set in the runner so a fault or sanitizer abort cannot leave a
core file. The temporary directory is removed on exit.

### 1.2 Makefile

```make
ubsan:
	$(DOCKER_MAKE) exec PASSTHROUGH="UBSAN_SUITE_TIMEOUT" CMD="docker/scripts/ubsan-suite.sh"
```

`UBSAN_SUITE_TIMEOUT` is exported next to `ASAN_SUITE_TIMEOUT`. `make verify`
runs `ubsan` after `check`/`bench-shape`; `make asan` stays a separate gate.
`make check` does not run `ubsan`.

### 1.3 Suites

| Suite | Opt-in | Coverage |
|---|---|---|
| `compiler_robustness_spec_test` | `RDC_ASAN` + `RDC_UBSAN` | R1–R5 behavior, trap lines, ASan memory cleanliness, UBSan arithmetic |
| `ubsan_corpus_spec_test` | `RDC_UBSAN` | `buffer-container`, `file-copier`, `string-parser` built and run under UBSan |

The robustness suite's build helper reads both variables and selects the
sanitizer flags; the plain test run uses the strict host build. It also builds
two suite-level controls through the same helper:

| Control | Assertion |
|---|---|
| known out-of-bounds C probe under `RDC_ASAN` | an `AddressSanitizer` report |
| known signed-overflow C probe under `RDC_UBSAN` | a `runtime error:` report |

This proves the suite's child builds are sanitized, not only that the runner's
compiler can report. The corpus suite is a no-op unless `RDC_UBSAN` is set, so
`cargo test --workspace` stays fast.

The corpus sweep runs each program in a private scratch directory, strips
CRLF, asserts exit zero, and asserts stderr contains no `runtime error:`. It
does not re-assert the corpus stdout; the frozen corpus suite owns that.

GCC 12 does not enable `float-divide-by-zero` or `float-cast-overflow` in
`-fsanitize=undefined`; the runner does not add them, because the language
defines float division by zero and the conversion guard is the compiler's own
`E11212` path.

## 2. Deterministic grammar-based fuzzer

### 2.1 Layout

| Path | Role |
|---|---|
| `tools/fuzz/__init__.py` | Package marker, matching `tools/perf` |
| `tools/fuzz/fuzz.py` | Harness, generator, runner, classifier, CLI |
| `tools/fuzz/seeds/<name>.clo` | Curated accepted seed programs |
| `tools/fuzz/seeds/<name>.expected.txt` | Normalized stdout for a normal seed |
| `tools/fuzz/seeds/<name>.expected-fault.txt` | Normalized `error CODE:` line for a trap seed |
| `tools/fuzz/tests/__init__.py` | Test package marker |
| `tools/fuzz/tests/test_fuzz.py` | Unit tests |
| `build/fuzz/` | Ignored scratch, logs, and failure artifacts |

### 2.2 Deterministic decisions

`fuzz.py` derives every choice from a SHA-256 digest:

```python
def decide(seed: int, index: int, label: str, modulus: int) -> int:
    digest = hashlib.sha256(f"{seed}:{index}:{label}".encode("ascii")).digest()
    return int.from_bytes(digest[:8], "big") % modulus
```

The generator increments `index` for each decision, so the stream is stable
across Python versions and machines. Seed files are sorted by filename. No
`random`, set iteration, dictionary order, environment variable, or clock
reaches the generated text. The unit test runs the generator in two
subprocesses with `PYTHONHASHSEED=0` and `PYTHONHASHSEED=1` and compares bytes.

### 2.3 Generator

The generator is a typed template grammar. Each family returns a complete
`module`, a `main`, and a list of printable expressions:

| Family | Shapes |
|---|---|
| narrow arithmetic | `u8/i8/u16/i16/u32/i32` casts, `+ - * <<`, const and inline positions |
| shift divisor | masked shifts, integer division by a non-zero constant shift result |
| fill | fixed arrays 1–8 elements, fills from constants, in-range indices, and narrow arithmetic |
| enum | explicit values, in-range auto-increments, conversions, extent use |
| float | `f32`/`f64` casts and chains, finite integer conversions, `sizeof`/value agreement |
| struct and loop | struct with scalar and fixed-array fields, a bounded `while` loop, `std.io.println` |

Every generated program:

- has a unique `module fuzz.S<seed>` name;
- uses array extents in `1..=8` and loop bounds `0..=8`;
- performs no division by a possibly zero value, no out-of-range index, no
  `assert`, no `embed`, no `ffi`, no `unsafe`/raw access, and no file I/O;
- prints 1–5 lines of integer text and returns 0.

A template family is accepted by construction; `fuzz-test` instantiates each
family across its parameter domain and asserts `clc` accepts every program. A
`compiler-rejected` result is a compiler finding, not a silent skip.

### 2.4 Seeds

The committed seeds are hand-written regression inputs. The set is:

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

There are twelve seeds. A unit test scans every `.clo` for the forbidden
constructs (`embed`, `ffi`, `unsafe`, `raw`, file I/O, `assert`) and asserts
each has exactly one golden.

### 2.5 Matrix

| Mode | Host plain | Host sanitizer | m68k |
|---|---|---|---|
| smoke | all twelve seeds + generated `0..=15` | all seeds + generated `0..=3` | all seeds + generated `0..=3` |
| smoke-host | all twelve seeds + generated `0..=15` | all seeds + generated `0..=3` | none |
| full | all seeds + generated `0..=255` | all seeds + generated `0..=255` | all seeds + every 8th generated seed |

The host plain leg runs first and stops a seed on a golden mismatch. The
sanitizer leg is one combined ASan+UBSan build, not two builds, so the two
reports are distinguished by their text. The m68k leg uses `vamos` with a
120-second timeout.

### 2.6 Build and run commands

| Step | Command |
|---|---|
| host emit | `clc --profile host -o program.c program.clo` |
| host build | `cc -std=c99 -O0 -g -Wall -Wextra -Werror -o program program.c` |
| host run | private working directory, timeout 30 s |
| sanitizer emit | same emitted C as host |
| sanitizer build | `cc -std=c99 -O0 -g -Wall -Wextra -Werror -fsanitize=address,undefined -fno-sanitize-recover=all -no-pie -o program program.c` |
| sanitizer run | `ASAN_OPTIONS=detect_leaks=1:symbolize=0 UBSAN_OPTIONS=print_stacktrace=0`, timeout 60 s |
| m68k emit | `clc --profile hosted-amiga -o program.c program.clo` |
| m68k build | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror -o program program.c -lamiga -lm` |
| m68k run | `vamos program`, timeout 120 s, CRLF stripped |

Every command is a list passed to `subprocess.run`, never a shell string. Every
child sets `RLIMIT_CORE=0`; the harness asserts no `core` file appears in the
run directory.

### 2.7 Outcome normalization

```text
normalize(text) := text.replace("\r\n", "\n").replace("\r", "\n").rstrip("\n")
fault_line(host) := first line in stderr matching "error [A-Z0-9]+:"
fault_line(target) := first line in stdout matching "error [A-Z0-9]+:"
```

A normal outcome is `(stdout, exit 0)`. A fault outcome is
`(fault_code, nonzero)` where the exact normalized line is compared. A seed
declares which kind it is by the golden file it ships; both the golden and the
extracted line are normalized.

For generated programs the harness requires a normal outcome on both
profiles: no traps are generated. Host and target stdout are compared after
normalization, and both exit statuses must be zero.

### 2.8 Classification and precedence

The classifier applies these rules in order:

| Order | Check | Result |
|---|---|---|
| 1 | committed seed with `expected-fault.txt` | compare the normalized fault line; a mismatch is `golden-mismatch`, a match is a pass even with SIGABRT/exit 20 |
| 2 | committed seed with `expected.txt` | compare normalized stdout and exit status; a mismatch is `golden-mismatch` |
| 3 | compiler timed out | `hang` |
| 4 | compiler exited with a signal | `crash` |
| 5 | compiler stderr contains `internal compiler error` or a Rust panic | `internal` |
| 6 | compiler rejected a generated program | `compiler-rejected` |
| 7 | C compiler timed out | `hang` |
| 8 | C compiler nonzero | `invalid-c` |
| 9 | program timed out | `hang` |
| 10 | program exited with a signal or nonzero (normal seed) | `crash` |
| 11 | stderr contains `AddressSanitizer` or `runtime error:` | `sanitizer` |
| 12 | host and target stdout or status differ | `divergence` |
| 13 | harness/tool failure | `harness` (exit 2) |

A failure retains `program.clo`, the emitted `program.c` when produced,
`host.log`, `target.log`, `sanitizer.log`, and a `case.json` with the seed,
class, commands, and exit codes under
`build/fuzz/failures/<class>-<seed>/`. The process exits 1 after all scheduled
cases finish (2 for `harness`), so one run reports every finding it saw.

### 2.9 Makefile targets

```make
fuzz-test:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && python3 -m unittest discover -s tools/fuzz/tests -t tools/fuzz -v"

fuzz-smoke:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && python3 tools/fuzz/fuzz.py run --mode smoke"

fuzz-smoke-host:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && python3 tools/fuzz/fuzz.py run --mode smoke-host"

fuzz:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && python3 tools/fuzz/fuzz.py run --mode full"
```

`make check` adds `fuzz-test` and `fuzz-smoke`. If the full smoke exceeds its
budget, `make check` uses `fuzz-smoke-host` instead and `make verify` adds the
m68k subset on its own; the two are not duplicated. `make fuzz` is local-only.

The 60-second smoke budget covers the fuzzer process after the `cargo build`
step; `fuzz-test` is measured separately. If the full smoke exceeds it, the
fallback is `fuzz-smoke-host` in `make check` and the m68k subset in
`make verify`; the decision is recorded in `05-findings.md`.

### 2.10 Unit tests

| Test | Asserts |
|---|---|
| `test_generation_is_deterministic_across_processes` | two subprocesses with different `PYTHONHASHSEED` produce identical bytes |
| `test_a_different_seed_changes_the_program` | the stream is not constant |
| `test_every_template_family_is_accepted` | each family compiles with `clc`; the target builds the workspace first |
| `test_seed_set_and_goldens_are_complete` | twelve seeds and exactly one golden per seed |
| `test_seed_order_is_sorted` | filesystem order does not affect selection |
| `test_seeds_are_safe` | no forbidden construct in a committed seed |
| `test_normalization_strips_crlf` | CRLF and trailing newlines normalize |
| `test_fault_line_normalizes_crlf` | an m68k fault line matches its golden |
| `test_classifier_maps_each_failure` | one fabricated input per class, including a host SIGABRT fault pass |
| `test_commands_are_argument_lists` | no shell string is built |
| `test_private_working_directory` | a run does not write into the repository; no core file |

The template-acceptance test calls `clc`. `make fuzz-test` builds the workspace
first, so a missing binary is a failure, not a silent skip.

## 3. Shape and fault gates

### 3.1 Shape fixture

Extend `tools/perf/fixtures/shapes.clo` with:

```clorraine
const U8W: u8 = u8(200) + u8(100);
const U16W: u16 = u16(65535) * u16(65535);
const F: f32 = (f32(16777216.0) + f32(1.0)) - f32(16777216.0);
enum Mode: u8 { Off, On = 4, Ready }
const M: i32 = i32(Mode.Ready);

func pick(src: i32[], i: i32): i32 {
    let a: i32[4] = [; src[i]];
    return a[0];
}
```

`main` uses `U8W`, `U16W`, `F`, and `M`, and declares
`let nf: u8[4] = [; u8(200) + u8(100)];` so the narrow arithmetic reaches a
fill store. `pick` is emitted but not called (the fixture is build-only); the
emitter's existing `sizeof` use keeps the function defined. `bench-shape.sh`
adds a `hosted-amiga-13` leg:

```bash
GCC13_FLAGS=(-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -Wall -Wextra -Werror)
"${CLC}" --profile hosted-amiga-13 -o "${OUTDIR}/shapes13.c" "${FIXTURE}"
"${GCC}" "${GCC13_FLAGS[@]}" -o "${OUTDIR}/shapes13" "${OUTDIR}/shapes13.c" -lamiga -lm
```

The existing GCC `nix20`, vbcc, and freestanding legs stay. The host strict
build is covered by the spec suites and `make check`; `bench-shape` has no host
leg.

### 3.2 Fault parity

Add two committed programs under `examples/robustness/`:

| Program | Fault |
|---|---|
| `fill-bounds.clo` | pure fill with a runtime out-of-range index (`E11211`) |
| `fill-divisor.clo` | pure fill with a runtime zero divisor (`E11210`) |

Extend `docker/scripts/run-fault-amiga.sh` with a second `PROBES` array that
keeps `RUN_DIR=examples/robustness` for those two files, or a per-case source
path. For each, run under `vamos`, capture stdout+stderr, assert the exact
`error CODE:` line, assert exit 20, and assert the program did not die on a
signal. The six frozen `spec/tests/run` cases keep their directory and
assertions. The final message reports eight cases. The host side is covered by
the robustness spec suite under ASan.

## 4. Gate Summary

| Gate | New content |
|---|---|
| `make check` | `fuzz-test`, `fuzz-smoke` |
| `make asan` | the robustness suite now opts in (27 suites) |
| `make ubsan` | new; two suites; activation and suite-level controls |
| `make verify` | adds `ubsan` after `check`; `run-fault-amiga` gains the two fill probes; `fuzz-smoke-host` if the budget fallback applies |
| `make bench-shape` | new shapes and the `nix13` leg |
| `make fuzz` | local longer run |
| `make fuzz-smoke-host` | host-only budget fallback |
