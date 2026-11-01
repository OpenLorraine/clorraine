# Clorraine v1.0 Production Readiness Implementation Plan

> **Feature**: Make the v1.0 compiler production-ready: close the release-blocking residuals,
> stabilise the specification, add the simplified verification infrastructure, execute the
> already-planned DX slice, complete the bounded PLAN-13 acceptance slice, and prepare the
> MIT-licensed release.
> **Status**: Planning Complete
> **Created**: 2026-10-31
> **Implements**: PLAN-15 (standalone programme; no upstream RD)
> **CodeOps Artifact Schema**: 1

## Overview

The tracked defect backlog is complete: 546/546 rows are Done and zero rows are open. The
compiler passes `make check`, and the last full acceptance run reported every cross-toolchain
leg green. That is a strong v1.0-core milestone, not a production release: the specification
chapters are still draft, several report-only residuals remain, the ASan suites are opt-in, there
is no CI, and the repository has no licence or release artifact.

This programme closes that gap without reopening the language surface. It fixes only the
release-blocking residuals, freezes the v1.0 specification, adds a narrow sanitizer and
differential signal, runs the existing DX plan, completes a bounded PLAN-13 acceptance slice,
and prepares an MIT-licensed release. The deferred language features (`comptime`, pools/arenas,
`stores`/`transfers`, nested templates, user contract tables, C-hook trampolines) remain out of
scope per the roadmap's v1.0 boundary.

The verification infrastructure is deliberately smaller than a fuzzing programme. The
independent challenger judged the full generator, scheduled self-hosted `make verify`, and
release automation premature; the approved scope is `make asan`, a fixed host-vs-m68k
differential convenience runner over the already-pinned corpus outputs, and a least-privilege
host-only CI workflow (AR-2).

## Minimum-Sufficient Baseline

**Original goal:** turn the completed v1.0 core into a production-ready, MIT-licensed release
with a stable specification and a defensible verification story.

**Smallest viable design:** reuse the existing suites and scripts. Add one `make asan` target
that runs the suites already honouring `RDC_ASAN`, one fixed differential script over the three
host-portable corpus programs, one host-only GitHub Actions workflow, the MIT licence, the
specification status changes, and the already-planned DX and PLAN-13 work.

**Excluded machinery:** the full grammar-based generator, scheduled self-hosted `make verify`,
release automation, the Amiga SDK installer, and the deferred v1.1/v2 language features. These
are recorded in AR-2 and AR-12.

**Approved complexity:** AR-2 — simplified verification infrastructure; the independent
challenger returned `Simplify`, and the user chose the simplified scope.

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (15 entries: 14 resolved, 1 deferred) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Programme requirements, scope, and acceptance criteria |
| 02  | [Current State](02-current-state.md) | Evidence about the compiler, spec, tests, and release gaps |
| 03-01 | [Residual Hardening](03-01-residual-hardening.md) | Release-blocking residual design and diagnostics |
| 03-02 | [Spec Stabilisation](03-02-spec-stabilization.md) | Chapter review and `stable (v1.0)` status change |
| 03-03 | [Verification Infrastructure](03-03-verification-infrastructure.md) | `make asan`, `make differential`, and host-only CI |
| 03-04 | [DX and Acceptance](03-04-dx-and-acceptance.md) | Existing DX plan plus the bounded PLAN-13 slice |
| 03-05 | [Release Engineering](03-05-release-engineering.md) | MIT licence, metadata, release gate, and checklist |
| 07  | [Testing Strategy](07-testing-strategy.md) | ST-1 – ST-16 and verification commands |
| 99  | [Execution Plan](99-execution-plan.md) | Seven phases, 45 tasks, and a 2-task conditional publish sub-plan |

## Quick Reference

### Usage Examples

```sh
# Fast host and container gate
make check

# All suites that honour RDC_ASAN, one bounded process per suite, serial tests
make asan

# Host-vs-m68k output convenience diff over the three portable corpus programs
make differential

# Full acceptance, including the Amiga emulator legs
make verify
```

CI runs `./spec/check.sh` and `cargo test --workspace` on a GitHub-hosted Ubuntu runner. The
m68k-only tests skip when `m68k-amigaos-gcc` is absent; `make verify` remains the local
acceptance gate (AR-9).

### Key Decisions

| Decision | Outcome | AR Ref |
| -------- | ------- | ------ |
| Programme scope | Full v1.0 readiness: hardening, spec, DX, PLAN-13, release | AR-1 |
| Verification machinery | Simplified: `make asan`, fixed differential seed, host-only CI | AR-2 |
| Release target | Source tag plus Linux x86_64 binary tarball; v1.0.0; aarch64 deferred | AR-3, AR-12 |
| Residual policy | Fix release blockers; document the accepted rest; explicit null cast = E10012 | AR-4 |
| Licence | MIT | AR-5 |
| Specification | All v1.0 chapters marked `stable (v1.0)` after review | AR-6 |
| Roadmap | New `PLAN-15 — v1.0 production readiness` | AR-7 |
| PLAN-13 slice | `std.amiga.exec` plus two allowlisted console programs | AR-8, AR-13 |
| CI | Host-only GitHub Actions; no self-hosted runner | AR-9 |
| Differential seed | `buffer-container`, `file-copier`, `string-parser` (convenience gate, not a new signal) | AR-10 |
| `make asan` set | Every test file that references `RDC_ASAN` (21 today) | AR-11 |
| Accepted residuals | `docs/known-limitations.md` | AR-14 |

## Specialist Agents

**None** — the capability-gap check found no gap a standing specialist would close.

_Detection evidence: `codeops/specialist-check.json` (checked 2026-10-31, 51 plan folders) records
no candidate; this plan adds no new domain, so the next `analyze-agents` run will include it with
the same outcome. The four committed project agents under `.opencode/agents/` already cover the
compiler surfaces: ownership/drop and ASan (`memory-safety-auditor`), language rules and the
`spec/17-codegen` ABI (`semantics-reviewer`), emitted-C and strict builds
(`emission-integrity-auditor`), and correctness/test integrity (`correctness-reviewer`). Release
engineering and the fixed CI workflow are one-off infrastructure work with no recurring domain
invariant; the catalog roles plus a dynamic packet cover them. No new specialist is justified._

## Related Files

- `plans/00-roadmap.md` — new PLAN-15 row and stage sync.
- `plans/def-zero-closeout/05-findings.md` — residual evidence.
- `spec/README.md`, `spec/*.md`, `spec/14-diagnostics.md` — specification status and rules.
- `compiler/src/sema/expr.rs`, `compiler/src/own/lower.rs`, `compiler/src/codegen/program.rs` —
  residual fixes.
- `compiler/tests/v1_production_readiness_{spec,impl}_test.rs` — new regression suites.
- `Makefile`, `docker/scripts/asan-suite.sh`, `docker/scripts/differential-seed.sh` —
  verification targets.
- `.github/workflows/check.yml` — host-only CI.
- `LICENSE`, `compiler/Cargo.toml`, `README.md`, `RELEASE.md`, `docs/known-limitations.md` —
  release readiness.
- `plans/clorraine-dx/` — existing DX plan to execute.
- `plans/amiga-library-surface/` — PLAN-13 plan to create and execute.
