# WS3 — IR and C codegen correctness

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-02-compiler-workstreams.md §WS3 (WS3-01..WS3-11)

## Scope and limits

Components: `ir/`, `codegen/`, the emitted C. Inputs: the Phase 0 emission (93 accepted
cells, `findings/WS0-emission-record.tsv`) and the WS3 probes. Lens: valid C99 on every
compile-matrix leg, no undefined behavior, 68000 correctness, single-width volatile access.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | xhigh | 2026-11-25 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (3.1.1) | `probes/WS3/300`–`380` | done |
| Red check (3.1.2) | seeded files, detector, comparators, contrasts | done |

## Checklist outcomes

_(filled in Step 3.2)_

## Findings

_(filled in Step 3.2)_

## Spec questions

_(filled in Step 3.2)_

## Falsifiability log

Red check 2026-11-25 after `06edeb98` (log `build/tmp/assessment/WS3/red.log`,
`contrast.log`; results `build/tmp/assessment/WS3/probes/contrast.json`).

| Probe | Form | Red-check result |
|-------|------|------------------|
| `300-strict-build.sh` | seeded defect `300-seeded.c` (old-style declaration, shadowing, narrowing, sign change) | 5 warnings recorded (`-Wstrict-prototypes` ×2, `-Wshadow`, `-Wconversion`, `-Wsign-conversion`) |
| `301-vbcc-scan.sh` | seeded defect `301-seeded.c` | vbcc warnings 85 ("assignment of different pointers") and 113 ("only 0 should be assigned to pointer") |
| `320-odd-address.sh` | seeded violation: a word read at an odd address on the 68000 | **detector cannot fire**: `vamos` (68000) printed `odd read 515` and exited 0, exactly like `-C 68020`; WS3-05's runtime part is a `limit` and rests on code reading (FS-UAE would be the remaining detector) |
| `380-shapes.py` | comparator with a seeded record (`380-seeded-record.tsv`, every baseline 1 ms) | every compile reported `FAIL` against the 10 ms limit |
| `360-determinism.sh` | comparator (mutated copy) | `cmp` reports a copy with one appended line as different |
| 310, 311, 340, 341, 342, 344, 370, 371, 372 | contrasting input: a changed operand or literal whose expected output differs | 311, 344, 370, 372 matched on host, hosted-amiga, hosted-amiga-13 (debug, release; host also ASan and UBSan). **310**: the host UBSan line reported `signed integer overflow: 2147483647 + 1` at `++y`; **340, 341, 342**: the emitted C does not compile on any profile; **371**: the host builds crash (SIGSEGV) and the host debug build fails (`int-to-pointer-cast`) while the Amiga profiles print the expected value. Each contrast still discriminates (its id or operand change is visible where it runs); the failures are evidence for WS3-03, WS3-07, WS3-10 confirmed by the main run |
| 350–358 | contrasting input: an in-range index or non-zero divisor; the fault probe's contrast must print | all printed the stated value on the three profiles |
| 330 | contrasting input: the second volatile read replaced by a copy (`w = v`); accepted, and `330-mmio-width.sh` must then show one 16-bit load instead of two | accepted on both Amiga profiles; the load count is read in task 3.2.3 |
| 343 | — | **probe design error**: both modules declare `ffi strdup`, which `clc` rejects as one C symbol declared twice (E11101) before the handle typedefs are reached; the handle-typedef question is answered by code reading in task 3.2.3 |

## Escalations

None yet.

## Probe index

| Item | Probes |
|------|--------|
| WS3-01 strict builds | `300-strict-build.sh`, `300-seeded.c` |
| WS3-02 vbcc | `301-vbcc-scan.sh`, `301-seeded.c` |
| WS3-03 UB | 310, 311; WS1 probes 039–043 rerun under UBSan; ASM-WS1-002 |
| WS3-04 aliasing | code reading |
| WS3-05 68000 odd address | `320-odd-address.{c,sh}` |
| WS3-06 volatile/MMIO | 330 + `330-mmio-width.sh` |
| WS3-07 names | 340–344 |
| WS3-08 guards | 350–358 |
| WS3-09 determinism | `360-determinism.sh`; Phase 0 record |
| WS3-10 ABI | 370–372 |
| WS3-11 shapes | `380-shapes.py` (+ `380-seeded-record.tsv`) |
