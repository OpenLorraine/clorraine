# Freestanding profile Implementation Plan

> **Feature**: Implement the `freestanding` platform profile with a provisional, LorraineOS-adoptable ABI
> **Status**: Complete
> **Created**: 2026-09-23 18:43
> **Implements**: (none — standalone plan; no upstream RD exists)
> **CodeOps Artifact Schema**: 1

## Overview

The compiler ships two profiles: `hosted-amiga` (the real AmigaOS target) and
`host` (the fast test profile). The specification defines a third,
`freestanding`, for LorraineOS, but marks it "designed, later" and delegates its
toolchain, entry, and ABI to an operating system that does not exist yet. Today
selecting it is rejected with `E11200`.

This plan implements `freestanding` now, using only what exists: the SDK's
`m68k-amigaos-gcc`, `vamos`, the host C compiler, and our own runtime shims. It
defines a **provisional, LorraineOS-adoptable** ABI (m68k, 32-bit, big-endian,
no libc) so the profile is real and testable today, and LorraineOS can adopt it
later instead of inventing one.

A freestanding program is the same emitted C99 translation unit, but linked
against a no-libc runtime with its own `_start`. Because there is no OS to run
it on, the profile is verified two ways: the m68k leg is built and linked
(`-ffreestanding -nostdlib -lg`) and asserted to link cleanly, and a small
corpus is compiled with the raw-syscall host layer and executed on the
development machine to prove semantics.

The profile's standard-library surface is deliberately small: console output
works, while the heap and formatting modules (`std.mem`, `std.text`,
`std.fmt`) and the AmigaOS modules are rejected with `E11200`, and the file
functions of `std.io` trap at runtime. Floats are supported through the
compiler's soft-float runtime archive.

## Minimum-Sufficient Baseline

**Original goal:** Make the `freestanding` profile real, selectable, and
verifiable on the toolchain that exists today, without waiting for LorraineOS.

**Smallest viable design:** A third `Profile` variant and one new runtime shim
(`shim_freestanding.c`) with a portable runtime, an arch-guarded host syscall
layer, and m68k no-op stubs; the emitted `main` stub is unchanged and the shim
adds `_start`. The driver gains a freestanding row that builds the m68k artifact
and runs the host no-libc one.

**Excluded machinery:** a profile-scoped FFI contract table, a custom
soft-float library, a freestanding heap allocator, an ARM64 profile, and a
general refactor of per-profile facts into one table (AR-12, AR-25, AR-26).

**Approved complexity:** None beyond a new profile, a new runtime shim, and a
driver toolchain row (ordinary feature code using existing tools).

## Document Index

| #   | Document                                       | Description                                 |
| --- | ---------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                           | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)             | Feature requirements and scope              |
| 02  | [Current State](02-current-state.md)           | Analysis of the current implementation      |
| 03-01 | [Profile and Runtime](03-01-profile-runtime.md) | Profile facts, runtime shim, entry, gating |
| 03-02 | [Driver Integration](03-02-driver.md)        | Toolchain rows, build/run pipelines, make   |
| 03-03 | [Spec and Goldens](03-03-spec-goldens.md)    | Specification edits and golden test data    |
| 07  | [Testing Strategy](07-testing-strategy.md)     | ST cases and verification                   |
| 99  | [Execution Plan](99-execution-plan.md)         | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

```sh
# The compiler accepts the profile directly and emits the freestanding C
clc --profile freestanding -o hello.c hello.clo

# Build the m68k freestanding artifact (build only; no OS to run it)
clorraine build --profile freestanding hello.clo

# Run the same program through the no-libc host layer to observe semantics
clorraine run --profile freestanding hello.clo

# From the repository root, inside the SDK container
make run FILE=hello.clo PROFILE=freestanding
```

### Key Decisions

| Decision            | Outcome                                                        |
| ------------------- | -------------------------------------------------------------- |
| Profile name        | `freestanding` (unchanged); Rust `Profile::Freestanding` (AR-4) |
| ABI                 | Provisional, m68k/32-bit/big-endian, LorraineOS-adoptable (AR-3) |
| Verification        | m68k build+link cleanly; host no-libc run (AR-7, AR-27)        |
| Entry               | Emitted `main` unchanged; shim `_start` (AR-13)                |
| stdlib surface      | Console only; heap/format/Amiga modules rejected `E11200` (AR-11) |
| File I/O            | Traps at runtime with the new `E11214` (AR-12, AR-28)          |
| Floats              | Supported; m68k links the self-contained `-lg` soft-float runtime (AR-10) |
| Driver              | `build` = m68k artifact; `run` = host no-libc (AR-20)          |
| Profile facts       | `Profile` is the single source of truth (AR-19)                |

## Related Files

Created: `compiler/runtime/shim_freestanding.c`,
`compiler/tests/freestanding_spec_test.rs`,
`spec/tests/corpus/freestanding/`, `plans/clorraine-freestanding-profile/**`.

Modified: `compiler/src/profile.rs`, `compiler/src/driver.rs`,
`compiler/src/codegen.rs`, `compiler/src/codegen/program.rs`,
`compiler/src/stdlib.rs`, `compiler/src/clorraine/{mod,cli,toolchain,build,run,artifacts}.rs`,
`Makefile`, `AGENTS.md`, `plans/00-roadmap.md`, and the authorized `spec/`
chapters listed in AR-21.

Read-only dependencies: `compiler/src/types/mod.rs` (`usize` width),
`spec/17-codegen.md` §12-§13, `stdlib/std.io.clo`, `docker/scripts/run.sh`.
