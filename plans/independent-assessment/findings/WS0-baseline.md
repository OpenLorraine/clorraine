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
| Emission record (0.2.2) | done |
| Harness numbers (0.2.3) | done |
| vamos 68020 support (0.2.4) | done |
| Measurement matrix (0.2.5) | done |
| Inventory (0.2.6) | done |

## Checklist outcomes

WS0 has no checklist IDs; its observations are filed as `ASM-WS0-<NNN>` findings with
checklist item `WS0`.

## Findings

### ASM-WS0-001 — The flag audit's host binary floor fails on `lookup` at the baseline

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: host
- **Location**: `tools/perf/flag_audit.py:444-446` at e24a8752
- **Checklist item**: WS0
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `python3 tools/perf/flag_audit.py run --clc build/tmp/assessment/baseline/clc --check --out build/perf/assessment-baseline-flags.json` (container, 2026-11-25; record `records/assessment-baseline-flags.json`, `checks[0]`). Expected (documented harness invariant: `make bench-flags` "assert[s] the size/runtime floors", AGENTS.md §Commands; `check_floors` requires a scored host binary ≤ 85% of the old-flag binary): every row passes. Observed: `host/lookup: host binary 30912 > 34880 * 85%` (88.6%); exit status 1. The earlier `build/perf/flags.json` (2026-11-15, `clc 1.1.2`) records the same failure with the same byte counts, so it predates this release.
- **Impact**: `make bench-flags` exits non-zero on a clean tree, so the size/runtime floor gate is red for every run and cannot signal a real regression on the other rows; no shipped program is affected.
- **Root cause**: the 85% host-binary floor is uniform, but `lookup` (added in `a886a345`, 2026-11-09) is dominated by its 16 KB static table, which section GC and stripping cannot shrink; its `.text` falls 19.5% while its file falls only 11.4%.
- **Recommendation**: give data-dominated benchmarks a floor on `.text` instead of the file size, or exclude the table bytes from the host-binary floor; alternatively record `lookup` as exempt from the host floor with its reason.
- **Effort**: S
- **Related**: `plans/constant-array-initializer` (added `lookup`); WS8-05/WS8-06 (gate integrity)
- **Verdict**: confirmed (Minor) — verifier ses_dea9a6131ffesU3qOxzrQQoINm (9.2.1): the floor check fails with the same byte counts (`host/lookup` 30,912 > 85% of 34,880); `bench-flags` is outside `check`/`verify`.

## Spec questions

None.

## Falsifiability log

| Probe | Form | Result |
|-------|------|--------|
| `probes/WS0/drift-check.sh` | seeded defect: an untracked file at the repository root, and an untracked folder `plans/zz-drift-probe/` | both rejected with exit 1 and the path named; clean tree passes with exit 0 (2026-11-25 19:08) |
| `probes/WS0/measure.py` comparator | mutated expectation (comparator form, ST-5): `sieve.expected` copied to `build/tmp/assessment/WS0/redcheck/expect/` with `10280` changed to `10281`; run `--bench sieve --lens host a500-O2 --side c clo --no-trace --expect-dir …` | all four sieve cells reported `MISMATCH`, exit 1; calibration cells matched; the control run with the committed expectations matched every cell, exit 0 (2026-11-25 19:16; logs `build/tmp/assessment/WS0/redcheck/{mutated,control}.log`) |

| `probes/WS0/cpu68020.{c,expected,sh}` | contrasting input: the same `-m68020` binary on the default 68000 CPU, plus a `-m68000` control build | 68020 CPU printed the expected line; 68000 CPU refused the binary (`ALERT 00068020`, rc 20); the control printed the line (2026-11-25 19:21) |
| `probes/WS0/ref/*.c` (ST-13) | output check against the committed benchmark goldens on every lens; discrimination of the comparator is shown by the `measure.py` row above | 90/90 cells matched (0.2.5) |
| `probes/WS0/emit-all.sh` | none — measurement recorder with no expectation (it records acceptance, determinism, and compile time; WS1-09, WS3-09, and WS3-11 write the expectations those records are judged against) | — |

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

## Emission record (0.2.2)

**Command.** `make -C docker exec CMD="bash plans/independent-assessment/probes/WS0/emit-all.sh build/tmp/assessment/baseline/clc build/tmp/assessment/baseline/clc-debug build/tmp/assessment/baseline/emit"`
(measurement only; 2026-11-25 19:12–19:40; pinned `clc` sha256 `0c2fd66e…`). Inputs: the 7 spec
corpus programs, the 9 benchmarks (calibration included), and the 30 `examples/amiga/*.clo`
programs — 46 programs × 4 profiles = 184 cells. Full per-cell record committed as
[`WS0-emission-record.tsv`](WS0-emission-record.tsv); emitted files and logs under
`build/tmp/assessment/baseline/emit/<program>/<profile>/`.

**Acceptance.**

| Profile | Accepted | Rejected | Rejection diagnostic |
|---------|---------:|---------:|----------------------|
| `host` | 14 | 32 | all `E11200` (`'std.amiga.*' is not available on profile 'host'`; spec 15 PR-3) |
| `hosted-amiga` | 46 | 0 | — |
| `hosted-amiga-13` | 31 | 15 | all `E11200` (2.0+-only modules) |
| `freestanding` | 2 | 44 | all `E11200`; accepted: `spec/tests/corpus/freestanding`, `benchmarks/calibration` |

Every rejection is the profile gate `E11200`; no accepted-elsewhere program failed with any
other code. Whether each rejection is *correct* is WS1-09's question.

**Determinism (input to WS3-09).** For all 93 accepted cells the two release emissions are
byte-identical, and the debug-`clc` emission is byte-identical to the release emission
(columns `deterministic`, `debug_equal`: 93/93 `yes`).

**C compile time (input to WS3-11).** Compile-only (`-c`) wall time of each accepted
emission, shipped optimization flags without link (host: `-std=c99 -O2 -ffunction-sections
-fdata-sections`; GCC m68k: the profile base line plus the same; vbcc: `+aos68k -c99 -O2
-cpu=68000 -I…/ndk-include`). Every one of the 170 compiles succeeded.

| Compiler | Profile | Builds | C lines (min–max) | Compile-only ms (min–max) | Largest program |
|---|---|---|---|---|---|
| cc | host | 14 | 830–1375 | 213–376 | `benchmarks/lookup/program.clo` (1375 lines, 321 ms) |
| gcc | hosted-amiga | 46 | 10728–14114 | 3353–4679 | `examples/amiga/hello-intro.clo` (14114 lines, 4679 ms) |
| gcc | hosted-amiga-13 | 31 | 6178–9564 | 1903–3192 | `examples/amiga/hello-intro.clo` (9564 lines, 3084 ms) |
| gcc | freestanding | 2 | 489–491 | 66–71 | `spec/tests/corpus/freestanding/program.clo` (491 lines, 66 ms) |
| vbcc | hosted-amiga | 46 | 10728–14114 | 887–1370 | `examples/amiga/hello-intro.clo` (14114 lines, 1223 ms) |
| vbcc | hosted-amiga-13 | 31 | 6178–9564 | 460–798 | `examples/amiga/hello-intro.clo` (9564 lines, 654 ms) |

Observation for WS4-07/WS7-02 (not a finding): every `hosted-amiga` emission is at least
10,728 lines and every `hosted-amiga-13` emission at least 6,178 lines because the whole shim
is embedded in each translation unit; the m68k GCC compile of even `calibration` takes
3.4 s (hosted-amiga) and 1.9 s (hosted-amiga-13).

## Harness numbers (0.2.3)

**Commands** (one run each, 2026-11-25 18:20–18:41 UTC, inside the SDK container; script
`build/tmp/assessment/baseline/harness.sh`; log `build/tmp/assessment/baseline/harness.log`):

```sh
python3 tools/perf/bench.py run --clc build/tmp/assessment/baseline/clc --compiler-git e24a8752b08827c3a5d7c77b6e2827dac708b629 --label assessment-baseline --out build/perf/assessment-baseline-bench.json
python3 tools/perf/compile_time.py run --clc build/tmp/assessment/baseline/clc --compiler-git e24a8752b08827c3a5d7c77b6e2827dac708b629 --label assessment-baseline-compile --out build/perf/assessment-baseline-compile.json
python3 tools/perf/flag_audit.py run --clc build/tmp/assessment/baseline/clc --check --out build/perf/assessment-baseline-flags.json
```

The three outputs are new files (none existed before), copied to
`build/tmp/assessment/baseline/` and committed as measurement records under
[`records/`](records/). The flag audit has no `--compiler-git` option; its document records
the HEAD of the run (`a7069030`, working tree dirty only with this plan's files), and the
compiler it measured is the pinned `clc` (sha256 `0c2fd66e…`), built from the baseline
`e24a8752`.

### Bench harness — nix20 numbers (`hosted-amiga`, `-mcrt=nix20 -m68000 -O2`, shipped release flags)

These are **nix20** numbers from `bench.py`'s hard-coded flags; they are never A500 numbers
(03-01 §Measurement rules). Calibration: 2,928 instructions raw, host release best 0.585 ms.
Host time: best and median of 30 runs on the container host.

| Benchmark | m68k `.text` | m68k file bytes | Instructions raw | Instructions net | Host best ms | Host median ms |
|-----------|-----------:|-----------:|-----------:|-----------:|-----------:|-----------:|
| `container` | 24,108 | 25,980 | 59,742 | 56,814 | 0.586 | 0.787 |
| `local-table` | 24,568 | 26,380 | 5,855 | 2,927 | 0.538 | 0.706 |
| `lookup` | 39,884 | 41,700 | 11,707 | 8,779 | 0.581 | 0.659 |
| `matrix` | 23,592 | 25,400 | 12,289 | 9,361 | 0.686 | 0.826 |
| `moves` | 23,684 | 25,512 | 47,558 | 44,630 | 0.673 | 0.774 |
| `sieve` | 23,816 | 25,640 | 1,043,009 | 1,040,081 | 0.816 | 0.934 |
| `strings` | 23,972 | 25,844 | 1,224,160 | 1,221,232 | 0.878 | 1.041 |
| `text-scan` | 24,152 | 25,992 | 378,606 | 375,678 | 0.705 | 0.846 |

### Compile-time harness (release `clc`, 5 runs each)

| Workload | Profile | Median ms | Min–max ms | Peak RSS KB |
|----------|---------|----------:|-----------:|------------:|
| `benchmark/calibration` | host | 4.9 | 4.7–5.3 | 17,264 |
| `benchmark/container` | host | 8.8 | 8.6–10.1 | 17,316 |
| `benchmark/local-table` | host | 5.8 | 5.7–6.1 | 17,264 |
| `benchmark/lookup` | host | 12.2 | 10.7–13.2 | 17,264 |
| `benchmark/matrix` | host | 6.3 | 6.1–6.8 | 17,264 |
| `benchmark/moves` | host | 5.9 | 5.4–6.9 | 17,264 |
| `benchmark/sieve` | host | 10.7 | 10.5–11.2 | 17,264 |
| `benchmark/strings` | host | 5.7 | 5.3–6.1 | 17,316 |
| `benchmark/text-scan` | host | 5.8 | 5.6–5.9 | 17,264 |
| `generated/calls-4000` | host | 118.8 | 113.6–121.6 | 27,976 |
| `generated/functions-1000` | host | 35.9 | 35.5–41.6 | 18,072 |
| `generated/functions-2000` | host | 74.0 | 69.9–77.9 | 24,828 |
| `generated/functions-4000` | host | 144.3 | 143.9–145.9 | 43,792 |
| `generated/functions-8000` | host | 299.6 | 295.7–308.3 | 81,512 |
| `generated/locals-4000` | host | 294.3 | 293.4–302.0 | 29,152 |
| `generated/mixed-4000` | host | 503.9 | 493.6–519.9 | 66,448 |
| `generated/statements-4000` | host | 51.5 | 50.6–53.3 | 18,664 |
| `real/boing` | hosted-amiga | 19.9 | 19.6–20.9 | 17,264 |
| `real/exec-demo` | hosted-amiga | 23.3 | 22.9–23.7 | 17,264 |
| `real/gui` | hosted-amiga | 24.1 | 23.6–24.5 | 17,264 |
| `real/imageviewer` | hosted-amiga | 49.6 | 48.4–53.2 | 17,264 |
| `real/services` | hosted-amiga | 8.4 | 8.2–10.9 | 17,264 |
| `real/workbench` | hosted-amiga | 18.7 | 18.5–18.8 | 17,264 |
| `corpus` (130 files, 19 accepted / 111 compile-fail goldens rejected) | host | 445.7 | 418.3–500.6 | 17,284 |

These are the "Phase 0 `bench-compile` value for the same input size" of 03-01 §Severity
(WS1-10, WS7-06/07). RSS floor reported by the harness: 18,112 KB.

### Flag audit (`--check`)

Constructors and repeat checks passed; **the floors check failed** on one row:
`host/lookup: host binary 30912 > 34880 * 85%`. Every other row passed its floor. Recorded
as [ASM-WS0-001](#asm-ws0-001--the-flag-audits-host-binary-floor-fails-on-lookup-at-the-baseline).
Old vs. shipped flags (the audit's "old" is no section GC/strip; "new" is the shipped line):
hosted-amiga `.text` falls 57–92%, hosted-amiga-13 50–89%, host 20–80%; full table in
`records/assessment-baseline-flags.json`.

## Measurement matrix (0.2.5)

**Command** (2026-11-25, inside the SDK container; log `build/tmp/assessment/WS0/matrix.log`):

```sh
python3 plans/independent-assessment/probes/WS0/measure.py plans/independent-assessment e24a8752b08827c3a5d7c77b6e2827dac708b629 --clc build/tmp/assessment/baseline/clc --out build/tmp/assessment/WS0/matrix --jobs 6
```

Raw output: `build/tmp/assessment/WS0/matrix/matrix.json`, committed as
[`records/WS0-matrix.json`](records/WS0-matrix.json) (per cell: command, CPU, output,
section sizes, raw and net instructions). Pinned `clc` sha256 `0c2fd66e…`; baseline
`e24a8752`.

**ST-13 (green).** All 90 cells (9 programs × 2 sides × 5 lenses) printed output identical
after `bench.normalize_output` to their committed expectation (`ref/<name>.expected`, copied
from `benchmarks/<name>/expected-output.txt`), including the trace runs; `failures: []`, exit
0. The C references follow the fairness rules in `ref/ref_io.h` (same algorithm, widths,
layout, allocation pattern including the heap-built output line, stdio path, and a bounds
check per index expression).

**Cross-check.** The A1200 shipped Clorraine column equals the bench harness's nix20 numbers
(for example `container` 59,742 raw / 56,814 net; `strings` 1,224,160 / 1,221,232), so the
script reproduces the harness on the shared cell.

**Reading the table.** Net = raw − the same side's calibration in the same lens; `.text`
net = `.text` − that side's calibration `.text`. Δ = (Clorraine − C) / C on the net values.
Interpretation, attribution, and severity belong to WS4-01/WS4-02 and WS7-01; this section is
the record they cite. 68020 numbers are a **68020 probe build** (`-m68020`, `vamos -C 68020`)
and are never rated above Opportunity (ST-15).

#### A500 shipped -O2 (hosted-amiga-13, nix13, 68000)

Flags: `-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s … -lamiga -lm`; `vamos` CPU 68000. Calibration raw instructions: Clorraine 2,919, C 2,918; calibration `.text`: Clorraine 5,900, C 5,124.

| Benchmark | Instr raw Clo | Instr raw C | Net Clo | Net C | Δ net | `.text` Clo | `.text` C | `.text` net Clo | `.text` net C | Δ `.text` net |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `container` | 60,117 | 44,230 | 57,198 | 41,312 | +38.5% | 24,056 | 22,552 | 18,156 | 17,428 | +4.2% |
| `local-table` | 6,166 | 6,039 | 3,247 | 3,121 | +4.0% | 24,520 | 22,320 | 18,620 | 17,196 | +8.3% |
| `lookup` | 11,985 | 11,852 | 9,066 | 8,934 | +1.5% | 39,832 | 38,656 | 33,932 | 33,532 | +1.2% |
| `matrix` | 12,472 | 13,407 | 9,553 | 10,489 | -8.9% | 23,544 | 22,368 | 17,644 | 17,244 | +2.3% |
| `moves` | 47,773 | 56,616 | 44,854 | 53,698 | -16.5% | 23,632 | 22,380 | 17,732 | 17,256 | +2.8% |
| `sieve` | 1,043,170 | 1,043,043 | 1,040,251 | 1,040,125 | +0.0% | 23,772 | 22,648 | 17,872 | 17,524 | +2.0% |
| `strings` | 1,688,727 | 1,274,427 | 1,685,808 | 1,271,509 | +32.6% | 23,920 | 22,588 | 18,020 | 17,464 | +3.2% |
| `text-scan` | 374,244 | 349,670 | 371,325 | 346,752 | +7.1% | 24,084 | 22,924 | 18,184 | 17,800 | +2.2% |

#### A500 shipped -Os (hosted-amiga-13, nix13, 68000)

Flags: `-std=c99 -mcrt=nix13 -m68000 -fwrapv -Os -ffunction-sections -fdata-sections -Wl,--gc-sections -s … -lamiga -lm`; `vamos` CPU 68000. Calibration raw instructions: Clorraine 2,927, C 2,918; calibration `.text`: Clorraine 5,924, C 5,124.

| Benchmark | Instr raw Clo | Instr raw C | Net Clo | Net C | Δ net | `.text` Clo | `.text` C | `.text` net Clo | `.text` net C | Δ `.text` net |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `container` | 74,508 | 48,284 | 71,581 | 45,366 | +57.8% | 24,132 | 22,540 | 18,208 | 17,416 | +4.5% |
| `local-table` | 5,951 | 5,793 | 3,024 | 2,875 | +5.2% | 24,528 | 22,288 | 18,604 | 17,164 | +8.4% |
| `lookup` | 11,980 | 15,914 | 9,053 | 12,996 | -30.3% | 39,860 | 38,640 | 33,936 | 33,516 | +1.3% |
| `matrix` | 11,792 | 12,698 | 8,865 | 9,780 | -9.4% | 23,708 | 22,500 | 17,784 | 17,376 | +2.3% |
| `moves` | 61,798 | 70,590 | 58,871 | 67,672 | -13.0% | 23,676 | 22,376 | 17,752 | 17,252 | +2.9% |
| `sieve` | 1,043,168 | 1,043,019 | 1,040,241 | 1,040,101 | +0.0% | 23,772 | 22,616 | 17,848 | 17,492 | +2.0% |
| `strings` | 1,692,733 | 1,418,421 | 1,689,806 | 1,415,503 | +19.4% | 23,904 | 22,616 | 17,980 | 17,492 | +2.8% |
| `text-scan` | 384,199 | 391,727 | 381,272 | 388,809 | -1.9% | 24,080 | 22,900 | 18,156 | 17,776 | +2.1% |

#### A1200 shipped (hosted-amiga, nix20, 68000)

Flags: `-std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s … -lamiga -lm`; `vamos` CPU 68000. Calibration raw instructions: Clorraine 2,928, C 2,927; calibration `.text`: Clorraine 5,952, C 4,740.

| Benchmark | Instr raw Clo | Instr raw C | Net Clo | Net C | Δ net | `.text` Clo | `.text` C | `.text` net Clo | `.text` net C | Δ `.text` net |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `container` | 59,742 | 43,855 | 56,814 | 40,928 | +38.8% | 24,108 | 22,168 | 18,156 | 17,428 | +4.2% |
| `local-table` | 5,855 | 5,728 | 2,927 | 2,801 | +4.5% | 24,568 | 21,936 | 18,616 | 17,196 | +8.3% |
| `lookup` | 11,707 | 11,573 | 8,779 | 8,646 | +1.5% | 39,884 | 38,272 | 33,932 | 33,532 | +1.2% |
| `matrix` | 12,289 | 13,224 | 9,361 | 10,297 | -9.1% | 23,592 | 21,984 | 17,640 | 17,244 | +2.3% |
| `moves` | 47,558 | 56,401 | 44,630 | 53,474 | -16.5% | 23,684 | 21,996 | 17,732 | 17,256 | +2.8% |
| `sieve` | 1,043,009 | 1,042,892 | 1,040,081 | 1,039,965 | +0.0% | 23,816 | 22,264 | 17,864 | 17,524 | +1.9% |
| `strings` | 1,224,160 | 809,860 | 1,221,232 | 806,933 | +51.3% | 23,972 | 22,204 | 18,020 | 17,464 | +3.2% |
| `text-scan` | 378,606 | 349,103 | 375,678 | 346,176 | +8.5% | 24,152 | 22,540 | 18,200 | 17,800 | +2.2% |

#### A1200 68020 probe build (hosted-amiga emission, -m68020, vamos -C 68020)

Flags: `-std=c99 -mcrt=nix20 -m68020 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s … -lamiga -lm`; `vamos` CPU 68020. Calibration raw instructions: Clorraine 3,042, C 2,882; calibration `.text`: Clorraine 5,884, C 4,656.

| Benchmark | Instr raw Clo | Instr raw C | Net Clo | Net C | Δ net | `.text` Clo | `.text` C | `.text` net Clo | `.text` net C | Δ `.text` net |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `container` | 71,970 | 44,675 | 68,928 | 41,793 | +64.9% | 23,612 | 21,648 | 17,728 | 16,992 | +4.3% |
| `local-table` | 5,841 | 5,528 | 2,799 | 2,646 | +5.8% | 24,080 | 21,408 | 18,196 | 16,752 | +8.6% |
| `lookup` | 11,703 | 11,372 | 8,661 | 8,490 | +2.0% | 39,392 | 37,740 | 33,508 | 33,084 | +1.3% |
| `matrix` | 12,356 | 13,115 | 9,314 | 10,233 | -9.0% | 23,092 | 21,456 | 17,208 | 16,800 | +2.4% |
| `moves` | 47,584 | 47,246 | 44,542 | 44,364 | +0.4% | 23,192 | 21,468 | 17,308 | 16,812 | +3.0% |
| `sieve` | 1,043,349 | 1,043,054 | 1,040,307 | 1,040,172 | +0.0% | 23,332 | 21,748 | 17,448 | 17,092 | +2.1% |
| `strings` | 1,025,545 | 599,082 | 1,022,503 | 596,200 | +71.5% | 23,476 | 21,664 | 17,592 | 17,008 | +3.4% |
| `text-scan` | 364,646 | 337,226 | 361,604 | 334,344 | +8.2% | 23,644 | 22,000 | 17,760 | 17,344 | +2.4% |

**Host lens.** Output check only (host time comes from WS7-01): all 18 host cells matched.

## vamos 68020 support (0.2.4)

**Method (ST-15).** `vamos` (amitools `d0b9727d`) accepts `-C 68020` (`vamos --help`: "Set
type of CPU to emulate (68000, 68020 or 68040)"); its default is 68000
(`vamos --config-dump`: `'machine': {'cpu': '68000', ...}`). The bench harness's
`trace_vamos` takes no CPU argument, so `measure.py` selects the CPU through a private
`HOME` holding `.vamosrc` with `[vamos] cpu=68020`; `--config-dump` under that `HOME` shows
`'cpu': '68020'`.

**Probe.** `probes/WS0/cpu68020.{c,expected,sh}`, committed in `a7069030` before the run;
command `make -C docker exec CMD="bash plans/independent-assessment/probes/WS0/cpu68020.sh build/tmp/assessment/WS0/cpu68020"`.

| Check | Expected (`cpu68020.expected`) | Observed |
|-------|-------------------------------|----------|
| 68020 build contains long multiply/divide | yes | yes — `m68k-amigaos-objdump -d -m m68k:68020` shows `divull sp@(12),d2,d2` at `0x10e` and `mulsl d0,d1` at `0x116` in `_main`. The script's own count printed 0 because plain `objdump -d` decodes as 68000 and shows `.short 0x4c6f`/`0x4c00`; the script's grep is too narrow, the expectation stands |
| `vamos -C 68020` on the 68020 build | prints `cpu68020: 1592766455 125082` | printed it, rc 0 |
| `vamos` default CPU on the 68020 build | does not print the line | `exec: ERROR: ALERT: code=00068020`, rc 20 (the nix20 start-up refuses a 68020 binary on a 68000) |
| Control: 68000 build on the default CPU | prints the line | printed it, rc 0 |

**Result.** `vamos` executes 68020 code, so the A1200 68020 probe lens reports dynamic
instruction counts (no static-only fallback, no named limit). Every 68020 number is labelled
"68020 probe build (`-m68020`, `vamos -C 68020`)" and is never rated above Opportunity
(ST-15). Log: `build/tmp/assessment/WS0/cpu68020.log`.

## Inventory at the baseline (0.2.6)

Re-recorded at `e24a8752` against `02-current-state.md` §What Exists (recorded at `2eff956c`).
Counts are `wc -l` of the `.rs` root file plus its directory, as in `02`.

| Item | `02` (at `2eff956c`) | Baseline `e24a8752` | Difference |
|------|---------------------|---------------------|------------|
| `compiler/src/` Rust lines | 96,229 | 96,229 | none |
| Stages `lex`/`parse`/`ast`/`load`/`resolve` | 1,204 / 2,652 / 939 / 1,154 / 855 | same | none |
| `sema`/`types`/`consts` | 9,339 / 3,394 / 1,879 | same | none |
| `own`/`ir`/`codegen` | 37,963 / 2,355 / 20,308 | same | none |
| `analysis.rs`+`driver.rs`; six support files; `clorraine/` | 801; 3,224; 5,126 | same | none |
| Largest files (`sema/call.rs` … `codegen/expr.rs`) | as listed | same | none |
| `compiler/runtime/` lines (11 files) | 11,611 (per-file as listed) | 11,611, per-file identical | none |
| `stdlib/` modules | 43 (6 core + 37 `std.amiga.*`) | 43 (6 + 37) | none |
| `compiler/tests/` entries | 157 | 157 | none |
| `spec/` goldens | 115 compile-fail, 7 run, 4 staged, 7 corpus | 114 compile-fail programs, 6 run programs, 3 staged programs (each folder also holds a `README.md`), 7 corpus | wording only: `02` counted the README in each folder |
| `spec/` Markdown files | 18 chapters, grammar, diagnostics, `decisions.md`, two appendices | 18 chapters (00–17), `grammar.ebnf.md`, `grammar-deltas.md`, `decisions.md`, two appendices, `README.md`, `lineage.md`, `preflight-report.md` | `02` omitted `lineage.md` and `preflight-report.md` |
| `tools/fuzz/` seeds | 24 | 24 | none |
| Roadmap `↳ DEF-` rows / highest ID | 570 / `DEF-581` | 570 / `DEF-581` | none |
| `docs/known-limitations.md` | 282 lines | 271 lines | changed by `a777415f` (installer documentation, after `2eff956c`); out of scope for the assessed components |

The inventory matches `02` for every assessed component; the only differences are counting
conventions and a documentation edit. Phase 0 wins where they differ (task 0.2.6).

## Phase 0 gate (0.3.2)

Checked 2026-11-25 against `07` §Check points, Phase 0 row.

| Case | Result |
|------|--------|
| Task verify | `drift-check.sh plans/independent-assessment` — PASS (baseline `e24a8752`) |
| ST-1 | ASM-WS0-001 has every template field including `Author session`; ID, severity, and effort valid — pass |
| ST-2 | ASM-WS0-001 (`verified`) cites the committed measurement record `records/assessment-baseline-flags.json` and its command — pass |
| ST-3 | no `hypothesis` finding — n/a |
| ST-5 | `ref/*.expected` committed in `c7097e66` before the matrix ran; `cpu68020.*` committed in `a7069030` before its run; falsifiability log has one line per probe with its form — pass |
| ST-7 | `git diff --name-only e24a8752 HEAD` touches only `plans/independent-assessment/` and `plans/00-roadmap.md` (this plan's row and the header counter); working tree adds only `tools/assessment/README.md` — pass |
| ST-11 | no Critical — n/a |
| ST-12 | every measured quantity above cites its command, lens, flags, CPU, baseline, and raw output path — pass |
| ST-13 | 90/90 cells identical after normalization; fairness rules in `ref/ref_io.h` — pass |
| ST-15 | every 68020 number labelled "68020 probe build" with `-m68020` and `vamos -C 68020` — pass |
| ST-16 | `Reasoning` lines are `medium`/`high`/`xhigh`; dispatch record `medium` — pass |
| ST-18 (stub) | `tools/assessment/README.md` documents the full mode with exact steps; `drift-check.sh` takes the plan folder and an optional baseline SHA (else read from the run's `WS0-baseline.md`); `measure.py` takes the plan folder and baseline SHA; neither (nor `emit-all.sh`) contains a run-specific path or SHA (`grep -nE 'independent-assessment|[0-9a-f]{40}'` finds nothing) — pass |

No Critical to escalate. Phase 0 gate: **passed**.

## Drift check

`probes/WS0/drift-check.sh <plan-folder> [<baseline-sha>]` implements 03-01 §Ground rules.
