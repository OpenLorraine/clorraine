# Amiga SDK Baseline (Bundle 0)

> **Feature**: A500/Kickstart 1.3 profile, SDK coverage registry, and bundle tracking issues
> **Status**: Blocked — A500/KS1.3 runtime acceptance (AR-17)
> **Created**: 2026-11-04 14:33
> **CodeOps Artifact Schema**: 1

## Overview

Bundle 0 establishes the lowest supported Amiga baseline and the long-term backlog
for the Amiga SDK programme. It adds a `hosted-amiga-13` compiler profile for a
stock A500 (68000, OCS, 512 KB chip, Kickstart 1.3), keeps the existing
`hosted-amiga` profile as the advanced 2.0+/A1200/AGA target, and pre-checks the 1.3 build and load/run path with a
hello-world under `vamos`; the FS-UAE A500/KS1.3 visible acceptance is blocked.

It also creates the durable registry of missing Amiga capabilities. The registry
is the source of truth; a generated Markdown coverage view and one GitHub tracking
issue per bundle make the long implementation timeline visible without turning
issues into the database.

## Minimum-Sufficient Baseline

**Original goal:** make the A500/1.3 baseline real and start a registry for the
full Amiga SDK surface.
**Smallest viable design:** one new profile variant, one shared shim with 1.3-safe
exec/library calls, a TSV registry with a POSIX shell/awk validator and generator,
and generated per-bundle tracking issues.
**Excluded machinery:** a separate 1.3 shim, a JSON/YAML parser, a two-way issue
sync, per-capability issues, and any native graphics implementation.
**Approved complexity:** the registry and issue tracking are the user-requested
deliverables (AR-1, AR-6, AR-7); the shell/awk helpers are the smallest direct
implementation, not a new framework.

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions |
| 00  | [Index](00-index.md) | This document |
| 01  | [Requirements](01-requirements.md) | Scope and acceptance |
| 02  | [Current State](02-current-state.md) | Existing profile, shim, and harness |
| 03-01 | [A500 Profile](03-01-a500-profile.md) | Profile, toolchain, gating, shim, runtime |
| 03-02 | [SDK Registry](03-02-registry.md) | Registry schema, tooling, issues |
| 07  | [Testing Strategy](07-testing-strategy.md) | ST-cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Tasks and phases |

## Quick Reference

### Usage

```text
clc --profile hosted-amiga-13 -o out.c hello.clo
clorraine build --profile hosted-amiga-13 examples/hello/hello.clo
clorraine run   --profile hosted-amiga-13 --emulator examples/hello/hello.clo
make sdk-registry
```

### Key Decisions

| Decision | Outcome |
|---|---|
| Lowest baseline | A500, OCS, 68000, 512 KB chip, Kickstart 1.3 |
| Advanced baseline | 2.0/ECS/68000/1 MB minimum; A1200/AGA/3.1 is the high end |
| Profile split | `hosted-amiga-13` (1.3) and `hosted-amiga` (3.1) |
| 1.3 module surface | Text/IO/mem + DOS/graphics/intuition/interrupt/custom; no `std.amiga.exec` yet |
| Shim strategy | Shared hosted-amiga shim with 1.3-safe exec/library calls |
| Registry | TSV source, generated Markdown coverage, shell/awk validator |
| Runtime proof | FS-UAE A500/KS1.3 acceptance pending; `nix13` build + `vamos` load/run is the pre-check |
| Issues | One generated tracking issue per bundle |

## Specialist Agents

**None** — the capability-gap check found no gap a standing specialist would close.
The work is a profile/toolchain change plus repository tooling; the existing
correctness, memory-safety, and emission-integrity reviewers cover the changed
surfaces.

_Detection evidence: `compiler/src/profile.rs`, `compiler/src/clorraine/toolchain.rs`,
`compiler/runtime/shim_hosted_amiga.c`, and shell-only registry tooling. No new domain,
framework, concurrency model, or security boundary is introduced._

## Related Files

- `compiler/src/profile.rs`, `compiler/src/clorraine/toolchain.rs`
- `compiler/src/stdlib.rs`, `compiler/src/codegen.rs`, `compiler/src/types/layout.rs`
- `compiler/src/clorraine/{cli,mod}.rs`, `compiler/src/driver.rs`
- `compiler/runtime/shim_hosted_amiga.c`
- `compiler/tests/amiga_a500_spec_test.rs` (new)
- `docker/scripts/run.sh`, `docker/scripts/a500-smoke.sh` (new)
- `docs/amiga-sdk/{registry.tsv,bundles.tsv,coverage.md,README.md}` (new)
- `tools/amiga-sdk/registry.sh` (new)
- `plans/amiga-sdk/00-index.md` (programme map, new)
