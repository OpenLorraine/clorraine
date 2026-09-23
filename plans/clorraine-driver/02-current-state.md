# Current State: Clorraine build/run driver (`clorraine`)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

- **An emit-only compiler.** `clc` parses, checks, and emits one C99 translation
  unit. Its CLI is fixed by the MVP plan: `--profile`, `--deny-warnings`,
  `--max-errors`, `-o`, `--version` (`compiler/src/driver.rs`). It writes only
  the `-o` file and never spawns a process (`plans/clorraine-compiler-mvp/01-requirements.md:86-91`).
- **A public compiler library.** `compiler/src/lib.rs` exports `driver` and
  `profile`, so a second binary can call the compiler in-process and reuse the
  profile facts.
- **Profile facts.** `Profile { Host, HostedAmiga }` exposes the name, the
  runtime shim, the module surface, and the stack-frame threshold
  (`compiler/src/profile.rs`). It deliberately holds no toolchain command.
- **The compile matrix as documentation.** The exact C-builder command lines are
  normative in `spec/17-codegen.md` §13; today they appear only in docs, the
  integration tests, and the `docker/scripts/*.sh` legs.
- **Host-side emulator orchestration.** `docker/scripts/run.sh` boots FS-UAE
  through `docker compose` with X11; `docker/Makefile` exposes `run` and
  `run-vamos`. These run on the **host**, not inside the `dev` container.
- **Test conventions.** `compiler/tests/*_spec_test.rs` are spec tests;
  `compiler/tests/stdlib_impl_test.rs` is an impl test; in-module
  `#[cfg(test)] mod tests` are unit tests (`AGENTS.md`).

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/Cargo.toml` | One `[[bin]]` (`clc`) | Add `[[bin]] name = "clorraine"` |
| `compiler/src/lib.rs` | Public module list | Add `pub mod clorraine;` |
| `compiler/src/driver.rs` | `pub fn run(args) -> i32` | None (called in-process) |
| `compiler/src/profile.rs` | `Profile` enum | None (read-only) |
| `compiler/src/clorraine/` | — | New module tree (CLI, toolchain, build/run) |
| `compiler/src/clorraine_main.rs` | — | New thin binary entry |
| `compiler/tests/clorraine_driver_spec_test.rs` | — | New spec tests |
| `Makefile` | `check`/`verify`/`corpus-*` | Add `run`/`build` passthrough |
| `AGENTS.md` | Commands + layout | Document `clorraine` |
| `plans/00-roadmap.md` | Tracker | Add `PLAN-02` row |

### Code Analysis

The compiler entry point is a plain function returning an exit code:

```rust
// compiler/src/driver.rs
pub fn run(args: Vec<String>) -> i32
```

The driver will call it with `["clc", "--profile", profile, "-o", c_path, file…]`,
check the returned code, and read `c_path`. Because it is a library call, no
second process is launched and the exit code already encodes diagnostics
(`1`) versus success (`0`); usage errors from `Options::parse` return `2`.

## Gaps Identified

### Gap 1: No build/run driver

**Current Behavior:** a developer runs `clc`, then a hand-typed
`m68k-amigaos-gcc … -lamiga -lm`, then `vamos`/FS-UAE.
**Required Behavior:** one command builds and runs, hiding C (R2–R8).
**Fix Required:** the new driver (see `03-01`, `03-02`).

### Gap 2: The compile matrix is not executable

**Current Behavior:** the §13 command lines live in docs and shell scripts.
**Required Behavior:** one driver-owned table produces the commands (AR-16).
**Fix Required:** a `toolchain` module carrying the profile→command data.

### Gap 3: No artifact convention

**Current Behavior:** outputs scatter across `build/` by hand.
**Required Behavior:** `build/clorraine/<stem>/<profile>/` (AR-9).
**Fix Required:** an artifact-path helper in the driver.

## Dependencies

### Internal Dependencies

- `clc::driver::run` (in-process compile), `clc::profile::Profile` (facts).
- The SDK container (for `m68k-amigaos-gcc`, `vamos`); the host wrapper for
  FS-UAE.

### External Dependencies

- `cc` (host C compiler), `m68k-amigaos-gcc`, `vamos` — inside the SDK image.
- No new crates: the workspace is std-only (`Cargo.toml`).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| In-process `clc::driver::run` writes to the process stdout/stderr | Low | Med | It already renders diagnostics there; the driver adds only its own post-build messages, so ordering stays readable |
| `hosted-amiga`/`vamos` tests need the SDK tools | Low | Med | `make check` already runs `cargo test` inside the container |
| FS-UAE behaviour cannot be asserted headlessly | High | Low | Assert the exact handoff command as text; live FS-UAE stays a `make verify`/manual leg (AR-20) |
| Artifact-directory collisions across programs | Low | Low | One subdirectory per stem, cleaned per build (AR-9) |
