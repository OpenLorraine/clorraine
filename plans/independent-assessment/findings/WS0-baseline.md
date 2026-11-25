# WS0 — Baseline, pins, C references, measurement matrix

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629
> **Baseline date**: 2026-11-25 (commit date 2026-11-25 18:50 +0100)
> **Pinned on**: 2026-11-25 19:07, after the user confirmed the working tree was settled (AR #7, AR #30)

## Scope and limits

Phase 0 pins the baseline and the tools, writes the equivalent C references and the
measurement script, captures the emission record, harness numbers, and the measurement
matrix that every later phase cites (03-01 §Ground rules, §Measurement rules).

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | medium | 2026-11-25 |

## Coverage table

| Item | Status |
|------|--------|
| Baseline and drift check (0.1.1) | done |
| C references and calibration (0.1.2) | done |
| `measure.py` and its red check (0.1.3) | done |
| Pinned tools (0.2.1) | done |

## Checklist outcomes

WS0 has no checklist IDs; its observations are filed as `ASM-WS0-<NNN>` findings with
checklist item `WS0`.

## Findings

None yet.

## Spec questions

None.

## Falsifiability log

| Probe | Form | Result |
|-------|------|--------|
| `probes/WS0/drift-check.sh` | seeded defect: an untracked file at the repository root, and an untracked folder `plans/zz-drift-probe/` | both rejected with exit 1 and the path named; clean tree passes with exit 0 (2026-11-25 19:08) |
| `probes/WS0/measure.py` comparator | mutated expectation (comparator form, ST-5): `sieve.expected` copied to `build/tmp/assessment/WS0/redcheck/expect/` with `10280` changed to `10281`; run `--bench sieve --lens host a500-O2 --side c clo --no-trace --expect-dir …` | all four sieve cells reported `MISMATCH`, exit 1; calibration cells matched; the control run with the committed expectations matched every cell, exit 0 (2026-11-25 19:16; logs `build/tmp/assessment/WS0/redcheck/{mutated,control}.log`) |

## Escalations

None.

## Pinned tools (0.2.1)

Recorded 2026-11-25 inside the SDK container (`make -C docker exec`), image not rebuilt
during the assessment (03-01 §Ground rules).

| Item | Value |
|------|-------|
| Release `clc` | `cargo build --workspace --release` (up to date at the baseline); copied to `build/tmp/assessment/baseline/clc`, sha256 `0c2fd66e7346acd0f3c01bff7f09f51b54605c0a974f6c1d2b00cd68aa29bba6`, `clc 1.2.0` |
| Debug `clc` | `cargo build --workspace`; copied to `build/tmp/assessment/baseline/clc-debug`, sha256 `9b126b8620f9dad4e059c714fa1fa6545cb1cce0d52dca3c76242d6a6349d1e5` (for WS3-09) |
| Docker image | `clorraine-sdk:dev`, ID `sha256:3835c03b273e3d71b5ae3790c6c5706a511722690c9e67a6316420746880a782`, created 2026-10-08T01:04:09+02:00 |
| amitools (`vamos`) | `0.8.1.dev274+gd0b9727da`, commit `d0b9727da3069328231784d2d5fdcf44816c95b1` (`https://github.com/AmigaPorts/amitools.git`) |
| Host GCC | `cc (Debian 12.2.0-14+deb12u1) 12.2.0` |
| `m68k-amigaos-gcc` | `(GCC) 16.2.0b 20260825082934`, toolchain repo `AmigaPorts/m68k-amigaos-gcc` at `e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c` (`docker/versions.env`) |
| vbcc | built by the same toolchain commit (`TOOLCHAIN_TARGETS` includes `vbcc vlink vasm`); the binaries print no version banner |
| FS-UAE | `3.1.66` |
| Python / rustc | `Python 3.11.2` / `rustc 1.99.0 (b940084d7 2026-09-28)` |

Raw record: `build/tmp/assessment/baseline/versions.txt`, `clc.sha256`.

## Drift check

`probes/WS0/drift-check.sh <plan-folder> [<baseline-sha>]` implements 03-01 §Ground rules.
