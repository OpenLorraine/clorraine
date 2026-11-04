# Execution Plan: Amiga SDK Baseline (Bundle 0)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-04 15:35
> **Progress**: 9/9 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Three phases: seed the registry and programme map, add the A500/1.3 profile and
shim compatibility, then prove the A500 runtime and close out. Specification
tests are written before implementation in each phase.

## Phase 1: Registry and programme map

> **Reasoning**: medium — data schema plus shell tooling; no compiler semantics.

**Tasks**: 3

- [x] 1.1.1 Write `tools/amiga-sdk/registry_test.sh` for ST-8–ST-10 (malformed rows, deterministic coverage, issue bodies); run it red because `registry.sh` does not exist
- [x] 1.1.2 Add `docs/amiga-sdk/{bundles.tsv,registry.tsv,README.md}`, `tools/amiga-sdk/registry.sh`, and `make sdk-registry` / `make sdk-issues`; run the test green and generate `coverage.md`
- [x] 1.1.3 Add `plans/amiga-sdk/00-index.md` (programme map, bundle order, baseline matrix); generate and create the ten tracking issues; verify with `gh issue list`

**Verify**: `tools/amiga-sdk/registry_test.sh`; `make sdk-registry`; `gh issue list --limit 20`

## Phase 2: A500/1.3 profile

> **Reasoning**: high — compiler profile, runtime flags, module gating, and a shared C shim.

**Tasks**: 4

- [x] 2.1.1 Write `compiler/tests/amiga_a500_spec_test.rs` for ST-3–ST-6 and extend the `profile.rs`/`toolchain.rs` unit tests; run red
- [x] 2.1.2 Add `Profile::HostedAmiga13` and the toolchain/runner/ABI/CLI/driver mappings; implement the 1.3 module whitelist and `E11200` gating; run the profile tests green
- [x] 2.1.3 Make the shared hosted-amiga shim 1.3-safe (`CreatePort`/`DeletePort`, `AllocMem`/`FreeMem`, `OpenLibrary(...,33)`); cross-build `examples/hello/hello.clo` with `-mcrt=nix13`; run green
- [x] 2.1.4 Update `spec/15-profiles.md`, `spec/appendix-amiga.md`, `AGENTS.md`, and `README.md`; run `make check`

**Verify**: `make check`; `cargo test --test amiga_a500_spec_test`; `cargo test --lib profile::tests toolchain::tests`

## Phase 3: Runtime and close-out

> **Reasoning**: high — real 1.3 emulator run plus the full acceptance gate.

**Tasks**: 2

- [x] 3.1.1 Add explicit `AMIGA_MODEL`/`KICKSTART` overrides to `docker/scripts/run.sh` and add `docker/scripts/a500-smoke.sh`; run the A500 hello under `vamos`; record the manual FS-UAE A500/KS1.3 leg
- [x] 3.1.2 Run `make verify` and `./spec/check.sh`; record findings; update the roadmap and the programme index; close the plan and commit/push

**Verify**: `make verify`; `./spec/check.sh`; `docker/scripts/a500-smoke.sh`

## Dependencies

- Phase 1 is independent of the compiler change.
- Phase 2 depends on the registry schema only for the coverage rows; the profile
  work can proceed in parallel in principle, but executes in order.
- Phase 3 depends on Phase 2.

## Success Criteria

1. All 9 tasks complete.
2. `hosted-amiga-13` builds the A500 hello with `nix13` and runs it under `vamos`; the FS-UAE A500/KS1.3 visual leg is manual (AR-17).
3. `hosted-amiga` (3.1) remains green.
4. Registry validates, coverage is generated, and ten tracking issues exist.
5. `make check`, `make verify`, and `./spec/check.sh` pass.
6. No language, grammar, diagnostic, or ABI change.
