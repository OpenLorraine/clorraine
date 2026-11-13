# Requirements: Opt-in Size Build Mode (PLAN-37)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Status**: Approved for execution
> **Base**: `e65de1c` (post-PLAN-36; `clc 1.1.2`)

## Feature Overview

Issue #21 asks for a public size build mode without changing the default
release line. PLAN-26's original evidence was measured before the shipped
release line had section GC: calibration `.text` fell 8.7% and `strings`
executed 13.1% more instructions. Re-measuring the delivered line shows the
retained m68k `.text` is near-neutral after section GC, host `.text` falls by
up to about 12%, and the runtime change is workload-dependent (`moves` +31.3%
net is the worst case; `strings` is +0.3%). The product decision is still to
make the mode **opt-in**: a developer who asks for `--size` accepts a
workload-dependent trade, existing release builds keep `-O2`, and the default
build stays debug.

The feature is a `clorraine` driver surface. It does not change the compiler,
the emitted C, the language, the ABI, the standard library, or the runtime.

## Functional Requirements

### Must Have

- [ ] **R1 — Size mode.** `clorraine build --size FILE...` and
  `clorraine run --size FILE...` select a size build mode that compiles the
  emitted C with `-Os` in place of `-O2`.
- [ ] **R2 — Release processing.** A size build keeps the release-only
  processing of the selected profile: `-ffunction-sections`, `-fdata-sections`,
  `-Wl,--gc-sections`, and `-s` on the host Linux, `hosted-amiga`, and
  `hosted-amiga-13` profiles; the plain `-Os` line on the host fallback and
  freestanding profiles. `--size` does not require `--release`.
- [ ] **R3 — Mutual exclusion.** A line containing both `--release` and
  `--size` is a usage error that names the conflict and exits 2. Repeating the
  same mode flag (`--size --size`) is accepted.
- [ ] **R4 — Emit exclusion.** `clorraine emit --size FILE` is a usage error
  and exits 2, for the same reason as `--release`: the emitted C is identical
  in every build mode.
- [ ] **R5 — Defaults unchanged.** With no mode flag the debug line is
  `-O0 -g -Wall -Wextra -Werror`; with `--release` the existing `-O2` vectors
  are byte-identical to the base commit.
- [ ] **R6 — Per-profile size lines.** The toolchain table provides an exact
  `-Os` vector for:
  - host Linux (with section GC and strip),
  - non-Linux host (plain `-Os`),
  - `hosted-amiga` (with section GC and strip),
  - `hosted-amiga-13` (the same line with `-mcrt=nix13`),
  - freestanding m68k (plain `-Os`),
  - freestanding host run with native syscalls, and
  - freestanding host run without native syscalls.
- [ ] **R7 — Freestanding run.** `clorraine run --size --profile freestanding`
  uses the size line for both the m68k build and the host-run build.
- [ ] **R8 — Help.** `clorraine --help` lists `--size`, names `-Os`, and states
  that it cannot be combined with `--release`. The existing `--release` help
  keeps its meaning and its symbol-bearing-profile wording.
- [ ] **R9 — Documentation.** `docs/known-limitations.md` gains a size-mode row
  that states the opt-in trade-off and the profiles where release processing
  applies. The `AGENTS.md` driver command row lists `--size`.
- [ ] **R10 — Tests.** CLI parsing, per-profile command vectors, and the
  end-to-end `build`/`run` behaviour have tests. The existing driver, toolchain,
  and workspace suites stay green.
- [ ] **R11 — Acceptance.** `make check`, `make asan`, `make ubsan`,
  `make bench-shape`, `make bench-compare`, `make corpus-amiga`,
  `make differential`, and `make verify` pass. The default flags and every
  scored benchmark are unchanged.

### Should Have

- [ ] **R12 — Measurement.** Record a per-profile size comparison on the same
  emitted C and a runtime comparison for the profiles that can execute the
  artifact. Record the `-Os` trade honestly, including the `strings` cost.
- [ ] **R13 — No new surface.** No new dependency, script framework, profile,
  environment variable, or build mode beyond `--size`.
- [ ] **R14 — Issue closeout.** Close issue #21 with the delivered surface and
  the measured trade, and mark the plan and roadmap Done.

## Scope Boundary

**In scope:** the `BuildMode` model; the per-profile `-Os` vectors; the
`--size` parser, validation, help text, and documentation; the CLI, toolchain,
and end-to-end tests; the per-profile measurement record; the issue, plan, and
roadmap closeout.

**Out of scope:** making `-Os` the default or changing `--release`; a
`-Oz`/LTO/per-function mode; flags passthrough to the C compiler; changing the
frozen `spec/17-codegen.md` compile matrix; changing `clc` emission; changing
the scored benchmark harness; version bumps or release publication.

## Measurement Requirements

- [ ] **M1 — Same emission.** Size and release builds use the same emitted C;
  only the C compiler optimization flag differs.
- [ ] **M2 — Per profile.** Measure the host, `hosted-amiga`,
  `hosted-amiga-13`, and freestanding artifacts, recording `.text`, artifact
  file bytes, and the executed runtime signal where the profile can run.
- [ ] **M3 — Representative programs.** Use the committed `calibration` and
  `strings` benchmark programs: one arithmetic/loop workload and one
  allocation-heavy workload that exposes the measured cost.
- [ ] **M4 — Recorded evidence.** Put the exact commands, tool versions, and
  numbers in [05-findings.md](05-findings.md); keep temporary artifacts under
  `build/tmp`.

## Acceptance Criteria

- [ ] Every R1–R14 item has a test, a recorded measurement, or a documented
  decision.
- [ ] The new CLI/toolchain/end-to-end cases fail on the base driver for the
  expected reason and pass on the head.
- [ ] The default release command vectors and the scored benchmark flags are
  byte-identical to the base.
- [ ] The full `make verify` gate is green before the plan is marked Done.
