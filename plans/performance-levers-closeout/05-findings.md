# Findings and Evidence: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Baseline pending — filled during execution

## Purpose

This document owns the measured evidence for PLAN-26: the baseline, the
same-compiler old-vs-new flag audit, the constructor/section audit, the scored
base/head comparison, the exact acceptance values, the gate evidence, the
review rulings, and the accepted limits.

## Baseline

**Base:** the post-PLAN-25 `main` head `f9d5be8` (`clc 1.1.1`); the plan
baseline commit is `bb4645a`. The measured compiler binary SHA-256 is
`08efe2e350bf179f862b0767663796cabf7fdb3aa55e866a171ee40b564fab37`.

**Old release flags:** host `-std=c99 -O2`; hosted-amiga
`-std=c99 -mcrt=nix20 -m68000 -fwrapv -O2`; hosted-amiga-13 the same with
`-mcrt=nix13`; freestanding `-std=c99 -ffreestanding -fno-builtin -nostdlib
-m68000 -fwrapv -O2`. Debug flags are unchanged.

**Preserved evidence:** `build/perf/base-v1.1.0.json` (the previous
cumulative baseline) and `build/perf/head-constant-array.json` (the PLAN-25
head) are retained; `build/perf/plan26-baseline.txt` records the identity.

## Red Phase

The specification tests were written before the implementation and failed for
the expected reasons.

| Command | Result | Expected failure |
|---|---|---|
| `cargo test -p clc --lib toolchain::tests` | 8 passed, 4 failed | host, hosted-amiga, 1.3, and `host_release_flags` vectors still have the old `-O2` |
| `cargo test -p clc --test clorraine_driver_spec_test` | 13 passed, 2 failed | the spec release rows and the `--release` help still have the old text |
| `make bench-test` | 56 tests, 2 failures + 14 errors | the harness flags and `binary_bytes` are absent; the audit floors/repeat are skeleton stubs |

The freestanding release vectors and every debug vector passed in the red run:
those lines are intentionally unchanged.

## Audit Matrix

_Pending Phase 3._ The matrix is produced by `make bench-flags` and recorded
here; the JSON stays under `build/perf/` (gitignored).

## Constructor and Section Audit

_Pending Phase 1/3._ The removed `__amigapath.o`/`environ.o` entries and the
remaining `___INIT_LIST__` entries are recorded here.

## Scored Comparison

_Pending Phase 4._ `base.json` from `f9d5be8` and `head.json` from the final
head, with the full compiler hash, `dirty == false`, and the compare table.

## Gate Evidence

_Pending Phase 4._ `make check`, `make asan`, `make ubsan`, `make
corpus-amiga`, `make differential`, `make bench-shape`, `make bench-base`,
`make bench`, `make bench-compare`, `make bench-flags`, `make verify`, the
release Workbench gates, and the 1.3 release corpus.

## Review Rulings

_Pending Phase 4._ Correctness, emission integrity, and design challenge
findings with their rulings.

## Accepted Limits

_Pending Phase 5._ Freestanding exclusion, non-Linux fallback, stripped
release symbols, and any residual risk.
