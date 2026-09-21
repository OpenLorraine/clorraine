# Requirements: Clorraine Compiler MVP

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The MVP turns the frozen specification (`spec/`, v0.4) into a working compiler
for one bounded acceptance target: the four-program validation corpus. The
compiler emits a single strict-C99 translation unit per program (Ch 17 §1,
P10), validated on two profiles: `host` for fast per-commit tests and
`hosted-amiga` for the real target through GCC, vbcc, and vamos (Ch 15 §2).

This document is the owning requirements document for the plan (no upstream RD
exists). Every requirement traces to a user decision in the
[Ambiguity Register](00-ambiguity-register.md) or to a normative specification
artifact.

## Functional Requirements

### Must Have

| # | Requirement | Source |
|---|---|---|
| R1 | The four corpus programs (file copier, interrupt counter, string/parser, buffer/container) exist under `spec/tests/corpus/` with `program.clo`, `expected-output.txt`, and a `README.md` rule-coverage record | AR-3, AR-28, `spec/lineage.md:169-175` |
| R2 | Each corpus program is hand-checked line-by-line against the frozen specification; the user accepts the corpus before front-end work starts | AR-31, AR-43 |
| R3 | Four spikes are executed and recorded: vbcc `#line`, generated-C FFI ABI, arithmetic wrapping/shifts, and the run-fault patterns | AR-8, AR-41; `decisions.md:330,336,370`, `preflight-report.md:121-126` |
| R4 | `clc` lexes and parses the frozen grammar (Ch 01, grammar §9-§11), including the targeted parser-only diagnostics E10204, E10208, E10501, E20008 | AR-6, AR-21 |
| R5 | Name resolution, module merging, imports, and module initialization order follow Ch 10 | AR-25 |
| R6 | Type checking, conversions, operators, constant evaluation, structs, and enums follow Ch 02, Ch 04, Ch 07, Ch 09 | AR-11, AR-13 |
| R7 | Definite assignment, moves, partial moves, nullability, drop planning, and the v1 lexical loan checker follow Ch 03, Ch 16, and `lineage.md` §6 | AR-18 |
| R8 | FFI declarations are checked as contracts (Ch 12) and lower through the ABI mapping of Ch 17 §3 (PF-002) | AR-19 |
| R9 | Codegen emits the single C99 translation unit of Ch 17: cleanup ladder, zero-as-empty moves, `#line` mapping, stable naming, byte-identical rebuilds | AR-35 |
| R10 | Both profiles are implemented: `host` (libc shim) and `hosted-amiga` (exec/dos shim, GCC/vbcc C subset) | AR-5 |
| R11 | The standard-library slice of AR-15/AR-46/AR-47 exists under `stdlib/`, compiles warning-free, and supports the corpus | AR-15, AR-42, AR-46, AR-47 |
| R12 | The compile-fail harness runs every in-scope golden file under `spec/tests/compile-fail/`, asserts the annotated code and the Ch 14 first-line shape, and reports skipped files with reasons | AR-10, AR-23, AR-33 |
| R13 | The run-fault harness compiles and executes all six `spec/tests/run/` cases and asserts the fault first line and abort | AR-32, AR-36 |
| R14 | The corpus harness compiles and runs the corpus on `host` and `hosted-amiga` and diffs stdout against `expected-output.txt`; the interrupt counter runs under FS-UAE | AR-29, AR-30, AR-32 |
| R15 | `make check` and `make verify` orchestrate the suites (AR-7); `AGENTS.md` records the commands | AR-38 |

### Should Have

| # | Requirement | Source |
|---|---|---|
| R16 | A warning-escalation golden case exercises `--deny-warnings` | AR-34, `preflight-report.md:56` |
| R17 | A vbcc build-only leg compiles the emitted C for one corpus program | AR-5, Ch 17 §13 |
| R18 | The compiler reports all recoverable diagnostics (N5) and honors `--max-errors=N` | AR-21 |

### Won't Have (Out of Scope)

Data inclusion (`embed`); lambda semantics (parsed, reported E90000/E90002);
callable bounds; `stores`/`transfers` contracts; pools and arenas;
user-provided contract tables (`--contracts`, W10005); the `freestanding`
profile; `comptime` semantics; nested templates; per-chip MMIO wrappers;
`std.amiga.exec` message/signal APIs; a library-build entry (MOD-29); 64-bit
fixed integers; tuple/multiple returns (AR-10, AR-14, AR-17, AR-47); and a
compiler that runs on AmigaOS/LorraineOS or a self-hosted compiler — `clc`
is a host-side development tool (AR-20; user scope statement, 2026-09-18).

## Technical Requirements

### Performance

The compiler is a development tool, not a runtime: no performance target is
imposed. Compilation is whole-program (P10) and single-pass per file, so the
corpus (well under 2,000 lines including the stdlib slice) must compile in
interactive time inside the SDK container.

### Compatibility

- Emitted C is the strict common subset of Ch 17 §1, accepted by
  `m68k-amigaos-gcc` and `vbcc` with the Ch 17 §13 commands (P1, `-std=c99`,
  `-fwrapv` on GCC; unsigned idioms for vbcc, PF-008).
- Integer semantics: wrapping arithmetic, masked shift counts, division faults
  (Ch 17 §10, B7a).
- The `host` profile uses only portable C99 plus libc.

### Security

A compiler's attack surface is untrusted source and paths. The MVP:

- treats every input file as untrusted data; the lexer/parser never evaluates
  or executes source content;
- imposes no path resolution from source (data inclusion is excluded, so
  `embed` paths cannot be traversed; AR-10);
- never invokes a shell; the harness spawns the compiler and C toolchain with
  argument arrays, never interpolated command strings;
- writes only the `-o` output file; no other filesystem writes;
- adds no network access, no external crates, and no secrets (AR-20);
- keeps `unsafe` semantics compile-time; the emitted runtime checks are always
  on (T12).

Items that do not apply are recorded as N/A rather than invented: there is no
network service (no auth, sessions, CSRF, rate limiting), no stored user data
(no encryption at rest), no credentials, and no deployment infrastructure in
this plan (AR-24 keeps CI/CD out of scope).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Acceptance target | corpus subset / corpus+full suite / full v1 | corpus subset | The corpus is the specification's own v1-seam validator; everything else is staged | AR-2, AR-3 |
| Compiler implementation | Rust / C / self-hosted | Rust (already pinned by the SDK) | `docker/Dockerfile:136` pins Rust; host tooling is ready | AR-20 |
| Compiler output | emit C only / drive the toolchain | emit C only | Matches P10 and keeps the driver small; Ch 17 fixes the commands | AR-6 |
| Stdlib shape | Clorraine source / C library | Clorraine source + C shim | `appendix-stdlib.md:9-11` says the library is ordinary Clorraine | AR-19, AR-46 |
| Test harness | Rust integration tests / dedicated harness / shell | Rust integration tests spawning `clc` | Smallest viable; challenger verdict `Unnecessary` for the larger option | AR-24 |
| Staged features | E90000 / partial implementations | E90000 + skipped tests | Honest MVP boundary; the spec's staged codes exist for this | AR-10, AR-14 |

> **Traceability:** every row cites the register entry that resolved it; see
> `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] The user has accepted the corpus and spike findings (AR-31).
2. [x] Every in-scope compile-fail golden file passes; skipped files are
       reported by name and reason (AR-10, AR-23).
3. [x] All six run-fault cases pass on `host` and on `hosted-amiga`/vamos.
4. [x] All four corpus programs compile with zero diagnostics and match their
       expected output (host for three; hosted-amiga for all; FS-UAE for the
       interrupt counter).
5. [x] Same source and flags produce byte-identical C (AR-35).
6. [x] The stdlib slice compiles warning-free (AR-42).
7. [x] `make verify` passes end to end in the SDK container.
8. [x] `docs/findings-phase2.md` records all four spike results; `AGENTS.md`
       and `plans/00-roadmap.md` are current (AR-38, AR-40, AR-41).

**Acceptance ruling (2026-09-21):** the user accepted the MVP against all eight
criteria after the full `make verify` run (spec check; 370 Rust tests across the
unit, codegen-run, compile-fail, corpus, determinism, run-fault, stage, and
stdlib suites; the hosted-amiga GCC + `vamos`, vbcc build-only, FS-UAE, and
run-fault legs). The recorded known limitations are the deferred `freestanding`
profile, the staged-out surfaces (data inclusion, lambda semantics, callable
bounds, `stores`/`transfers`, pools, user contract tables, nested templates,
`comptime`), vbcc debug decoding (D-1), and the planned `-m68881` FPU leg. Two
adjacent pre-existing minor gaps remain open for later triage: a local `func`
value that shares a global's name resolves to the global symbol, and `text()`
with zero arguments or a non-formattable argument emits invalid C.
