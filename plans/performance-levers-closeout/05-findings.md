# Findings and Evidence: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Executing — Phases 1–2 complete; measurement pending

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

### Probe Matrix (Phase 1)

The exploratory probe used the same `clc` binary and the pinned toolchain; only
the flags differ. The committed audit re-runs the old/GC/new set in Phase 3.

**m68k old flags (`-O2`, hosted-amiga)**

| Program | `.text` | file bytes | raw instructions |
|---|---:|---:|---:|
| calibration | 76,368 | 99,948 | 3,372 |
| sieve | 76,728 | 100,332 | 1,043,953 |
| matrix | 76,676 | 100,276 | 13,280 |
| strings | 77,604 | 101,500 | 2,967,423 |
| moves | 76,896 | 100,632 | 48,577 |
| lookup | 92,976 | 116,584 | 12,842 |

**host old flags (`-std=c99 -O2`)**

| Program | file bytes |
|---|---:|
| calibration | 18,384 |
| sieve | 22,528 |
| matrix | 22,520 |
| strings | 22,912 |
| moves | 22,648 |
| lookup | 38,944 |

The host byte counts are path-sensitive while symbols are present; the audit
rebuilds each variant in one fixed path and the shipped build is stripped.

**hosted-amiga-13 old flags (`-O2`, nix13)**

| Program | `.text` |
|---|---:|
| calibration | 53,432 |
| sieve | 53,792 |
| buffer-container | 54,540 |
| file-copier | 55,296 |
| string-parser | 55,072 |
| volatile-mmio | 53,512 |

**freestanding**

| Variant | `.text` | file bytes |
|---|---:|---:|
| old `-O2` | 208 | 1,552 |
| GC without strip | 232 | 1,576 |

**Rejected candidates (m68k `.text`, calibration/sieve/matrix/strings/moves/lookup)**

| Candidate | `.text` sequence | Runtime finding |
|---|---|---|
| `-O3` | 87,764 / 88,296 / 89,580 / 89,580 / 88,332 / 104,532 | equal, no win |
| `-Os` | 69,756 / 69,984 / 70,204 / 70,204 / 70,060 / 86,260 | strings raw 2,967,423 → 3,357,540 (+13.1%) |
| `-fomit-frame-pointer` | 76,296 / 76,656 / 76,604 / 77,528 / 76,820 / 92,900 | equal |
| `-flto` | 10,640 / 28,288 / 28,204 / 28,636 / 28,228 / 44,496 | worse than GC |
| `-flto` + GC | 5,284 / 23,208 / — | 2–11% smaller than GC, extra link risk |
| host `-fno-(asynchronous-)unwind-tables` | no host size change | — |

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

## Release Flag Change

The release command is per-profile after Phase 2:

| Profile / mode | Vector |
|---|---|
| host Linux release | `cc -std=c99 -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c` |
| host non-Linux release | `cc -std=c99 -O2 -o prog out.c` |
| host debug | unchanged `-O0 -g -Wall -Wextra -Werror` |
| hosted-amiga release | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c -lamiga -lm` |
| hosted-amiga-13 release | the same with `-mcrt=nix13` |
| freestanding m68k release | `m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O2 -o prog out.c -lg` |
| freestanding host-run release | `cc -std=c99 -ffreestanding -fno-builtin [-nostdlib] -O2 -o prog out.c` |

The frozen compile matrix changed exactly two rows (`spec/17-codegen.md:391`
and `:393`); the debug, vbcc, and freestanding rows are byte-identical. A
driver spec test pins all eight rows.

The `--release` help and `Options.release` doc now name the Linux/Amiga
strip+GC set and the freestanding/non-Linux/debug symbol exceptions.

**Pre-existing discrepancy recorded, not edited:** the freestanding release
row says "the same line with `-O2` in place of `-O0 -g`", which would retain
the debug warning flags, while the driver's release line omits warnings. The
user authorized only the two GCC release rows; the freestanding row stays as
it is.

**Green:** `cargo test -p clc --lib toolchain::tests` 12 passed;
`cargo test -p clc --test clorraine_driver_spec_test` 15 passed.

## Audit Matrix

_Pending Phase 3._ The matrix is produced by `make bench-flags` and recorded
here; the JSON stays under `build/perf/` (gitignored).

## Constructor and Section Audit

`--gc-sections` removes the `__amigapath.o` and `environ.o` objects and their
`.list___INIT_LIST__` entries. The linker log names:

- `__amigapath.o`: `.text`, `.bss`, `.list___INIT_LIST__`
- `environ.o`: `.text`, `.data`, `.bss`, `.list___INIT_LIST__`

The remaining `___INIT_LIST__` walk and constructor bodies still run. The GC
calibration trace has 2,928 instructions and load base `0x260c`; mapping the
`nm` symbols through that base gives:

| Symbol | File address | Trace hits |
|---|---:|---:|
| `__Exit` (init-list loop) | 0x7c | 823 |
| `___initstdio` | 0x802 | 82 |
| `___initcpp` | 0xef8 | 233 |
| `___initlibraries` | 0x104e | 113 |
| `___initmalloc` | 0x132e | 97 |

The 444-instruction reduction is the two removed constructor bodies, not a
skipped loop. The remaining constructors execute before `main`.

## Scored Comparison

_Pending Phase 4._ `base.json` from `f9d5be8` and `head.json` from the final
head, with the full compiler hash, `dirty == false`, and the compare table.

## Gate Evidence

_Pending Phase 4._ `make check`, `make asan`, `make ubsan`, `make
corpus-amiga`, `make differential`, `make bench-shape`, `make bench-base`,
`make bench`, `make bench-compare`, `make bench-flags`, `make verify`, the
release Workbench gates, and the 1.3 release corpus.

## Review Rulings

### Phase 2

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-201 | 2 | MINOR | toolchain doc comment | fixed — stale "filled in" clause removed |
| RV-202 | 2 | MINOR | host command test | fixed — platform-conditional host vector plus a composed non-Linux fallback test |
| RV-203 | 2 | MINOR | plan progress | fixed — Phase 2 tasks and progress updated |
| RV-204 | 2 | MINOR | driver spec test doc/name | fixed — scoped to the two GCC release rows |
| EM-2 | 2 | — | base/head emission diff | no findings — 235 sources and 248 profile runs had zero C/status/output differences |

The two reviewers found no critical or major issue. The emission auditor
confirmed the driver change produces byte-identical C and that the two spec
rows match the actual release vector.

## Accepted Limits

_Pending Phase 5._ Freestanding exclusion, non-Linux fallback, stripped
release symbols, and any residual risk.

**Recorded pre-existing limits (not caused by this feature):**

- A freestanding floating-point program needs target math bases; a minimal
  `f32` compare fails to link at both the base and the head with the same
  emitted C. `docker/scripts/bench-shape.sh` already treats freestanding as
  compile-only for this reason.
- The freestanding release spec row says "the same line with `-O2` in place
  of `-O0 -g`", which would retain warnings; the driver's freestanding release
  line omits warnings. Only the two GCC rows are authorized to change (AR-19),
  so the row is pinned as-is and the mismatch is recorded here.
- Host byte counts are path-sensitive while symbols are present; the audit
  rebuilds each variant in one fixed path and the shipped build is stripped.
- A non-Linux host was not available to boot; the fallback is pinned by unit
  tests only.
