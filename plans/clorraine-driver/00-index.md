# Clorraine build/run driver (`clorraine`) Implementation Plan

> **Feature**: A developer driver that builds and runs Clorraine programs, hiding the C toolchain by default
> **Status**: Done — plan and roadmap rows complete (2026-11-09 closeout).
> **Created**: 2026-09-23 00:11
> **Implements**: (none — standalone plan; no upstream RD exists)
> **CodeOps Artifact Schema**: 1

## Overview

The compiler MVP ships `clc`, an emit-only compiler: it turns Clorraine into one
C99 translation unit and writes it with `-o`, and nothing else. Building and
running a program therefore means hand-driving the target C compiler and the
emulator (`m68k-amigaos-gcc` + `vamos`/FS-UAE), which is a compiler-shaped
interface rather than a language-shaped one.

This plan adds **`clorraine`**, a second binary in the same crate that fronts
`clc` and owns the build/run orchestration. `clorraine run hello.clo` emits the
C into a gitignored artifact directory, compiles it with the profile's
toolchain, and runs it — printing only the program's output. The C is an
implementation detail: it appears only through `clorraine emit`, `--emit-c`, or
an internal-compiler-error report.

`clc` is untouched: its CLI contract and its emit-only, no-spawn design stay as
they are (AR-22). The driver is a separate binary that reuses the compiler's
public `profile` and `driver` modules in-process.

## Minimum-Sufficient Baseline

**Original goal:** Make writing, building, and running a Clorraine program a
one-command experience, with the C step invisible unless something is wrong or
explicitly asked for.

**Smallest viable design:** A second `[[bin]]` (`clorraine`) in the existing
`compiler/` crate whose logic lives in a testable lib module, reusing
`clc::driver::run` in-process and invoking the target toolchain from one
driver-owned table of the Ch 17 §13 commands.

**Excluded machinery:** A separate crate; giving the SDK container a Docker
socket so it can launch FS-UAE; converting `clc` into a multi-command toolchain
(these were considered and rejected — AR-2, AR-6).

**Approved complexity:** None beyond the driver binary and its toolchain table
(ordinary feature code invoking existing tools).

## Document Index

| #   | Document                                       | Description                                 |
| --- | ---------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                           | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)             | Requirements and scope                      |
| 02  | [Current State](02-current-state.md)           | Analysis of the current implementation      |
| 03-01 | [Driver CLI](03-01-driver-cli.md)            | Commands, flags, dispatch, exit codes       |
| 03-02 | [Build and Run](03-02-build-run.md)          | Toolchain table, artifacts, runner, ICE      |
| 07  | [Testing Strategy](07-testing-strategy.md)     | ST cases and verification                   |
| 08  | [Phase Review](08-review.md)                   | Independent correctness/security review      |
| 99  | [Execution Plan](99-execution-plan.md)         | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

```sh
# Build and run on the fast host profile (default)
clorraine run hello.clo

# Build and run on the real target under vamos, capturing output
clorraine run --profile hosted-amiga --capture hello.clo

# Print the exact host command that boots FS-UAE
clorraine run --profile hosted-amiga --emulator hello.clo

# Build only (no run)
clorraine build hello.clo

# The escape hatch: the generated C, on stdout or to a file
clorraine emit hello.clo
clorraine emit -o hello.c hello.clo
```

### Key Decisions

| Decision            | Outcome                                                        |
| ------------------- | -------------------------------------------------------------- |
| Where the driver lives | Separate `clorraine` bin in `compiler/`, `clc` unchanged (AR-2) |
| Command surface     | `run`, `build`, `emit` (AR-3)                                  |
| Default profile     | `host`; `--profile hosted-amiga` opt-in (AR-8)                 |
| Runners             | host exec; `vamos` default; `--emulator` FS-UAE (handoff) (AR-5, AR-6) |
| C visibility        | Hidden; artifacts in `build/clorraine/<stem>/<profile>/` (AR-9) |
| C compiler failure  | Reported as an internal compiler error (AR-10)                 |
| Environment         | SDK-container tool; outside → guidance, exit 2 (AR-7, AR-23)   |

## Related Files

Created: `compiler/src/clorraine/` (module tree), `compiler/src/clorraine_main.rs`
(the thin bin), `compiler/tests/clorraine_driver_spec_test.rs`,
`plans/clorraine-driver/**`.

Modified: `compiler/Cargo.toml` (add `[[bin]]`), `compiler/src/lib.rs` (add
`pub mod clorraine;`), `Makefile` (passthrough targets), `AGENTS.md`
(driver commands), `plans/00-roadmap.md` (`PLAN-02` row).

Read-only dependencies: `compiler/src/driver.rs`, `compiler/src/profile.rs`,
`spec/17-codegen.md` §13, `docker/scripts/run.sh`, `docker/Makefile`.
