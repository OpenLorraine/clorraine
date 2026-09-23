# Testing Strategy: Clorraine build/run driver (`clorraine`)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Driver behaviour (CLI, build, run) | High — every requirement has an ST case |
| Environment and ICE internals | High — unit-tested with injected inputs |
| Glue / formatting | Medium |

The repository split applies (`AGENTS.md`): end-to-end spec tests live in
`compiler/tests/*_spec_test.rs` and spawn the built binary; focused internals are
in-module `#[cfg(test)] mod tests`. The host and `hosted-amiga`/`vamos` cases run
inside the SDK container (where `cc`, `m68k-amigaos-gcc`, and `vamos` exist) via
`make check`; the FS-UAE handoff is asserted as text, never executed headlessly
(AR-20).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived from `01-requirements.md`, `03-01-driver-cli.md`, and
> `03-02-build-run.md`. They are the immutable oracle: if the implementation
> disagrees, the implementation is wrong.

### Driver behaviour

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | ---------------- | -------------------------- | ------ |
| ST-DRV-1 | `clorraine run hello.clo` (default profile), a program printing `Hello, Clorraine!` | stdout is exactly `Hello, Clorraine!`; exit 0; no C toolchain text shown | R2, R3, R4, R8 / AR-3, AR-8, AR-11 |
| ST-DRV-2 | `clorraine build hello.clo` | exit 0; executable exists at `build/clorraine/hello/host/hello`; nothing is run | R2 / AR-3, AR-9 |
| ST-DRV-3 | `clorraine emit hello.clo` | stdout is the C translation unit; identical to `clc --profile host -o -` output for the same file | R2 / AR-12, AR-17 |
| ST-DRV-4 | `clorraine emit -o out.c hello.clo` | `out.c` is written with the same C; exit 0 | R2 / AR-12 |
| ST-DRV-5 | `clorraine run --profile hosted-amiga hello.clo` (SDK container) | the program prints `Hello, Clorraine!` via `vamos`; exit 0 | R2, R4 / AR-5 |
| ST-DRV-6 | `clorraine run --capture hello.clo` | `build/clorraine/hello/host/out.txt` contains the program output; its path is printed; the output is not streamed | R8 / AR-25 |
| ST-DRV-7 | `clorraine run exit7.clo`, whose `main` returns 7 | `clorraine` exits 7 | R8 / AR-11 |
| ST-DRV-8 | `clorraine run --profile hosted-amiga --emulator hello.clo` | prints the exact host command `make -C docker run PROG=build/clorraine/hello/hosted-amiga/hello`; exit 0; with `--capture` the command is prefixed `RUN_CAPTURE=1 ` | R5 / AR-6, AR-25 |
| ST-DRV-9 | `clorraine run bad.clo` with a Clorraine syntax error | the compiler's `error …` diagnostic on stderr; exit 1; no artifact produced | R7 / AR-22 |
| ST-DRV-10 | `clorraine frobnicate hello.clo`; `clorraine run --nope hello.clo`; `clorraine run`; `clorraine run -o x.c hello.clo`; `clorraine run --emulator hello.clo` | each is a usage error on stderr; exit 2 | R10 / AR-26 |
| ST-DRV-11 | `clorraine --help`; `clorraine --version` | help/version text on stdout; exit 0 | R10 / AR-26 |
| ST-DRV-12 | `clorraine run a.clo b.clo` (two files in one program) | both files are passed to `clc`; the artifact stem is `a` | R13 / AR-18 |
| ST-DRV-17 | a source file whose name contains shell metacharacters and spaces (e.g. `weird ; name.clo`) | the path is passed as a literal argument and the command succeeds; nothing is interpreted by a shell | Security / AR-22 |

### Internals (unit-tested with injected inputs)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | ---------------- | -------------------------- | ------ |
| ST-DRV-13 | `sdk_present` with a marker that does not exist | false → the caller prints SDK guidance and returns exit 2 | R9 / AR-23 |
| ST-DRV-14 | `missing_tool(["m68k-amigaos-gcc"])` when it is absent | returns the tool name → the caller reports it and returns exit 1 | R9 / AR-24 |
| ST-DRV-15 | `compile` with an injected compiler command that always fails | returns an `IceError` whose message names the emitted-C path and the first error | R7 / AR-10 |
| ST-DRV-16 | the `hosted-amiga` toolchain command for debug and `--release` | base flags plus `-O0 -g` or `-O2` and `-lamiga -lm` | R12 / AR-19, AR-16 |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/clorraine_driver_spec_test.rs` | ST-DRV-1..12, ST-DRV-17 | CLI + build/run (spawns `clorraine`) |

### Implementation / Unit Tests (internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/src/clorraine/cli.rs` (`mod tests`) | argument parsing, every usage-error rule, defaults | High |
| `compiler/src/clorraine/toolchain.rs` (`mod tests`) | command assembly for both profiles and both modes; the handoff text | High |
| `compiler/src/clorraine/environment.rs` (`mod tests`) | `sdk_present`, `missing_tool` with injected inputs | High |
| `compiler/src/clorraine/build.rs` (`mod tests`) | `compile` ICE shape with an injected failing tool | High |

### Integration / End-to-End Tests

The `ST-DRV-1..12` spec tests are the end-to-end cases: they spawn the built
`clorraine` binary and exercise emit → compile → run on real profiles. Live
FS-UAE execution is out of the fast gate (AR-20); the existing
`make corpus-fs-uae` leg remains the emulator-run acceptance.

## Test Data

### Fixtures Needed

- `hello.clo` — prints `Hello, Clorraine!` and returns 0.
- `exit7.clo` — returns 7.
- `bad.clo` — a syntax error.
- A second file for the multi-file case.

Fixtures are written into a per-test temporary directory; artifacts are
redirected there so the repository tree is untouched.

### Mock Requirements

None. The tests use the real compiler, real toolchains, and the real runner
inside the SDK container. The environment and ICE tests inject a marker path and
a bogus tool path — no mocking framework.

## Verification Checklist

- [ ] All specification test cases (ST-*) are defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement or an AR entry
- [ ] Specification tests are written BEFORE implementation
- [ ] Specification tests are verified to FAIL before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Unit tests cover parsing, environment, command assembly, and the ICE shape
- [ ] The injection-prevention case (ST-DRV-17) passes
- [ ] `make check` and `make verify` remain green
- [ ] `clc`'s existing spec tests are unchanged and still pass (no regression)
