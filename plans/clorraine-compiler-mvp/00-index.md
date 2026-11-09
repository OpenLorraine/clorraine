# Clorraine Compiler MVP Implementation Plan

> **Feature**: The first Clorraine compiler: a Rust program (`clc`) that compiles the corpus subset of the frozen specification to one C99 translation unit, with the validation corpus, the four accepted Phase 2 spikes, the standard-library slice, and the golden test suites.
> **Status**: Done — plan and roadmap rows complete (2026-11-09 closeout).
> **Created**: 2026-09-17 23:56
> **Implements**: none (standalone plan; the requirements source is the frozen specification `spec/`, v0.4)
> **CodeOps Artifact Schema**: 1

## Overview

Clorraine is the systems language of LorraineOS: a safe, C-like language with
Rust-style ownership that compiles to plain C so it runs on the existing Amiga
toolchains (`README.md`). The specification is frozen and preflight-passed;
there is no compiler yet (`spec/preflight-report.md`, `docs/findings-phase1.md`).

This plan builds the compiler MVP on exactly one acceptance target: the
**four-program validation corpus** from `spec/lineage.md:169-175` — a file
copier, an interrupt-driven counter, a string/parser workload, and a
buffer/container workload — plus the six existing runtime-fault cases and the
four accepted Phase 2 verification spikes. The corpus defines the language
subset; everything outside it answers `E90000` (staged feature), so the
compiler stays small while the full specification and its golden suites remain
the oracle.

The plan follows the stage list already recorded in `spec/README.md:30-41`:
corpus and spikes first, then front end, name resolution, types, ownership,
codegen, standard library, and the suites. It uses only existing project
infrastructure: the Docker SDK (`docker/`), the golden files under
`spec/tests/`, and `spec/check.sh`.

## Minimum-Sufficient Baseline

**Original goal:** implement a working Clorraine compiler for the four-program
validation corpus, with the accepted Phase 2 spikes executed first, and commit
verified work automatically during execution.

**Smallest viable design:** one std-only Rust crate (`compiler/`) that emits a
single C99 translation unit per program; a Clorraine standard-library slice
under `stdlib/` compiled together with the program; a small per-profile C shim
prepended to the emitted unit; and golden-suite test drivers that spawn the
built `clc` binary and read the existing `spec/tests/` files (AR-19, AR-20,
AR-24, AR-25, AR-46).

**Excluded machinery:**

| Excluded | Why | Ref |
|---|---|---|
| Dedicated harness/xtask crate | No requirement beyond the four suites; `cargo test` + Make cover them | AR-24 |
| External Rust crates | Offline, deterministic, minimal supply chain | AR-20 |
| Per-chip MMIO wrappers | No corpus program needs them | AR-17 |
| Data inclusion (`embed`) | No corpus program needs it; tests excluded | AR-10 |
| Lambda semantics, callable bounds, pools, `stores`/`transfers`, user contract tables, `comptime`, nested templates | Staged beyond the corpus surface | AR-10, AR-14 |
| `freestanding` profile | Selecting it is E11200 per Ch 15 | AR-10 |

**Approved complexity:** AR-24 (test-harness support surface) — direct user
choice of the smaller solution after the visible packet and an independent
challenger verdict of `Unnecessary` for the larger option.

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Scope, acceptance criteria, and exclusions |
| 02  | [Current State](02-current-state.md) | What exists, gaps, dependencies, risks |
| 03-01 | [Validation Corpus](03-01-corpus.md) | The four golden programs, their outputs, and the hand-check |
| 03-02 | [Phase 2 Spikes](03-02-spikes.md) | VBCC `#line`, FFI ABI, arithmetic, run-fault spikes |
| 03-03 | [Front End](03-03-frontend.md) | Crate layout, diagnostics, lexer, parser, CLI skeleton |
| 03-04 | [Semantics](03-04-semantics.md) | Loading, name resolution, types, constant evaluation |
| 03-05 | [Ownership and Borrows](03-05-ownership.md) | IR, definite assignment, moves, loans, FFI contracts, generics |
| 03-06 | [Code Generation](03-06-codegen.md) | C emitter, cleanup ladder, ABI, profiles, runtime shim, driver |
| 03-07 | [Standard Library](03-07-stdlib.md) | Stdlib modules, signatures, shim API, layouts |
| 07  | [Testing Strategy](07-testing-strategy.md) | Spec test cases (ST-\*) and verification |
| 08  | [Phase 0 Review](08-phase0-review.md) | Independent corpus review findings and checkpoint rulings |
| 09  | [Phase 1 Review](09-phase1-review.md) | Independent spike review findings and fix batch |
| 10  | [Phase 2 Review](10-phase2-review.md) | Independent front-end review findings and fix batch |
| 11  | [Phase 3 Review](11-phase3-review.md) | Independent name/module review findings and fix batch |
| 12  | [Phase 4 Review](12-phase4-review.md) | Independent type/const review findings and fix batch |
| 13  | [Phase 5 Review](13-phase5-review.md) | Independent ownership/borrow review findings and fix batch |
| 14  | [Phase 6 Review](14-phase6-review.md) | Independent codegen/ABI review findings and fix batch |
| 15  | [Phase 7 Review](15-phase7-review.md) | Independent stdlib/intrinsic/memory review findings and fix batch |
| 16  | [Phase 8 Review](16-phase8-review.md) | Independent cross-toolchain/corpus review findings and fix batch |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, steps, and the task checklist |

## Quick Reference

### Usage examples

```sh
# Compile for the fast test profile (inside the SDK container)
clc --profile host -o out.c program.clo
cc -std=c99 -O0 -g -Wall -Wextra -Werror -o program out.c && ./program

# Compile for the Amiga target (inside the SDK container)
clc --profile hosted-amiga -o out.c program.clo
m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g \
    -Wall -Wextra -Werror -o demo out.c -lamiga
vamos ./demo

# The suites
make check     # spec/check.sh + host-profile cargo tests
make verify    # check + corpus and run-fault legs (host, hosted-amiga, FS-UAE)
```

### Key decisions

| Decision | Outcome | Ref |
| -------- | ------- | --- |
| Acceptance target | Four-program corpus + 6 run faults + 4 spikes | AR-2, AR-3 |
| Compiler | `compiler/` crate, binary `clc`, emits C only | AR-6, AR-20 |
| Implementation language | Rust (pinned by the SDK), std-only, zero crates; compiler is host-only — no AmigaOS/LorraineOS-hosted or self-hosted compiler in scope | AR-20 |
| Profiles | `host` + `hosted-amiga` | AR-5 |
| Stdlib | Clorraine sources under `stdlib/`, embedded; per-profile C shim | AR-19, AR-47 |
| Goldens | Existing `spec/tests/` files are the spec tests; suites spawn the binary | AR-23, AR-24, AR-44 |
| Staged features | Answer E90000; their tests are skipped and reported | AR-10, AR-14 |
| Execution | Later `exec-plan clorraine-compiler-mvp --auto-commit` | AR-1 |

## Related Files

Created by this plan: `Cargo.toml` (workspace), `compiler/` (crate),
`stdlib/` (Clorraine stdlib), `compiler/runtime/` (profile C shims),
`spec/tests/corpus/` (four programs), `spikes/` (four spike fixtures),
`docs/findings-phase2.md`, `Makefile` (root), `AGENTS.md`, `plans/00-roadmap.md`.

Read as the oracle: `spec/README.md`, `spec/decisions.md`,
`spec/grammar.ebnf.md`, Ch 00-17, `spec/appendix-amiga.md`,
`spec/appendix-stdlib.md`, `spec/tests/**`, `docs/findings-phase0.md`,
`docs/findings-phase1.md`, `docker/README.md`.
