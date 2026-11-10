# Build-Flag and Binary-Size Closeout (PLAN-26)

> **Feature**: Ship the measured build-flag win — section garbage collection
> plus release stripping on `host` (Linux), `hosted-amiga`, and
> `hosted-amiga-13` — without changing emitted C, runtime behavior, or safety.
> **Status**: Plan preflighted — ready for execution
> **Created**: 2026-11-10
> **Last Updated**: 2026-11-10
> **Base**: current `main` head `f9d5be8` (post-PLAN-25; `clc 1.1.1`)
> **Implements**: R1–R7
> **CodeOps Artifact Schema**: 1

**Original goal:** run a performance-levers audit and close out the
binary-size, dead-code-elimination, and build-flag opportunities with measured
wins, no scored regression, and no safety trade-off.

## Overview

`clc` emits one C translation unit containing the program and the runtime
shim. The linker cannot drop an unused shim function because it sees one
object. `-ffunction-sections -fdata-sections` plus `-Wl,--gc-sections` lets
the linker drop unreachable sections; `-s` removes the release symbol table.

The exploratory probe on the current tree measures 57.1–69.3% m68k `.text`
reductions on the scored programs (92.2% on calibration-only startup),
64.2–74.7% stripped-binary reductions (92.7% on calibration),
20.6–36.6% host release binary reductions, and no runtime change. The raw calibration count falls by
444 instructions because the `__amigapath.o` and `environ.o` constructor
bodies are collected. Freestanding is excluded: section GC grows its 208-byte
image to 232 bytes.

The feature also audits the removed constructor sections, aligns the
benchmark harness with the shipped release build, and proves the no-regression
guard with a `f9d5be8` base. The flag win is attributed by a same-compiler
old-vs-new audit; the scored comparison is the emission guard. The user ruled
on 2026-11-10 to update the two GCC release rows in `spec/17-codegen.md` §13,
so the frozen compile matrix stays the exact acceptance test (AR-19). Release
Workbench/services and the four 1.3 corpus programs are exercised under
GC+strip (AR-20).

## Minimum-Sufficient Baseline

**Smallest viable design:** change the release command in
`compiler/src/clorraine/toolchain.rs`, align `tools/perf/bench.py` and the nix20
shape leg, pin both command vectors with unit tests, run the audit and the
full gate, and record the evidence. No new crate, service, script framework,
or public build mode.

**Excluded machinery:** LTO, an `-Os` size mode, a 68020/FPU profile, macOS
host flags, freestanding GC/strip, shim splitting, a new benchmark, and any
safety-flag change.

**Approved complexity:** none. This is a flag/measurement change inside the
existing toolchain and harness.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Every material decision, including the auto-design records |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Functional, measurement, acceptance, and scope requirements |
| 02 | [Current State](02-current-state.md) | The build model, current flags, harness, spec, and gate gaps |
| 03-01 | [Release Flags](03-01-release-flags.md) | Per-profile release vectors, host predicate, spec rows, CLI text |
| 03-02 | [Measurement and Gates](03-02-measurement-and-gates.md) | Harness, `flag_audit.py`, shape gate, release Workbench, 1.3 corpus |
| 05 | [Findings](05-findings.md) | Baseline, audit matrix, gate evidence, reviews, acceptance |
| 07 | [Testing Strategy](07-testing-strategy.md) | ST cases, red/green order, gate coverage |
| 99 | [Execution Plan](99-execution-plan.md) | 44 tasks across 5 phases |
| PF-R | [Requirements Preflight](00-preflight-report-requirements.md) | Adversarial requirements audit and rulings |
| PF-P | [Plan Preflight](00-preflight-report.md) | Adversarial plan audit and rulings |

## Decisions

| Decision | Value |
|---|---|
| Base commit | `f9d5be8` (current `main`, post-PLAN-25; `clc 1.1.1`); assert the full hash and `dirty == false` |
| Shipped release flags | `-O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s` on host Linux, hosted-amiga, hosted-amiga-13 |
| Spec | Update only the two GCC release rows in `spec/17-codegen.md` §13 (user ruling 2026-11-10, AR-19) |
| Excluded | Freestanding (m68k and host-run), non-Linux host, LTO, `-Os`, frame-pointer, 68020/FPU |
| Harness | Builds the shipped release flags; records `m68k_release_bytes` for scored programs; cross-checks the driver vectors |
| Release path | Workbench/services release build and the four 1.3 corpus programs under GC+strip (AR-20) |
| Comparison | Scored `f9d5be8` → head is the emission guard; same-compiler old/new flags is the attribution |
| Floor policy | A missed floor stops; one audited rerun for a flake (AR-21) |
| Execution mode | `--auto-commit --auto-design`; push only when the full gate is green |
| Branch | `main` |

## Specialist Agents

**None** — the capability-gap check found no recurring domain a new specialist
would cover. The existing `correctness-reviewer`, `emission-integrity-auditor`,
and `design-challenger` cover this build/measurement change; the hot-path
`performance-auditor` is not applicable because no runtime code changes.

_Detection evidence: the change surface is `compiler/src/clorraine/toolchain.rs`,
`tools/perf/bench.py`, `docker/scripts/bench-shape.sh`, `Makefile`, tests, and
docs; no ownership/drop/IR/codegen path changes; no new recurring review area._

## Scope Boundary

In scope: the repeatable flag audit; the release flag change; the two GCC
release rows in `spec/17-codegen.md` §13 (user ruling AR-19); the harness and
shape-gate alignment; command-vector tests; the constructor/section audit; the
release Workbench/services build and the four 1.3 corpus programs (AR-20); the
documentation, issue registration, roadmap closeout; the required reviews and
the full gate.

Out of scope: language, grammar, diagnostics, ABI, stdlib, runtime-shim
behavior, emitted C, any other spec row, new target profiles, public build
modes, LTO, macOS host flags, freestanding GC/strip, shim splitting,
safety-flag removal, benchmark changes, new dependencies, and version
publication.

## Related Files

- `compiler/src/clorraine/toolchain.rs` — release slices, host predicate,
  freestanding host-run, vectors.
- `compiler/src/clorraine/cli.rs` — `--release` help.
- `spec/17-codegen.md` — the two authorized release rows.
- `tools/perf/bench.py` — harness flags and `m68k_release_bytes`.
- `tools/perf/flag_audit.py` — the new floor gate.
- `tools/perf/tests/test_bench.py`, `tools/perf/tests/test_flag_audit.py` —
  harness and audit tests.
- `docker/scripts/bench-shape.sh` — warning-bearing shape legs.
- `Makefile` — `bench-flags` and the release Workbench build.
- `benchmarks/README.md`, `docs/known-limitations.md`,
  `plans/v1-production-readiness/00-ambiguity-register.md` — the docs and
  accepted-limitation record.
- `AGENTS.md` — one command-table row.
