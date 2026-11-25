# Current State: Independent Assessment of Clorraine

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

Facts recorded at commit `2eff956c` (2026-11-25). Phase 0 re-records the inventory at the
pinned baseline; where the two differ, Phase 0 wins (task 0.2.6).

## Existing Implementation

### What Exists

`compiler/src/` holds 96,229 lines of Rust in one std-only crate. The pipeline is
`lex → parse → load → resolve → sema → own → ir → codegen`, orchestrated by `analysis.rs`
(the shared front-end order used by `clc`, the LSP, and docgen) and `driver.rs`, emitting one
C99 translation unit per program. Per-stage size (the `.rs` root file plus its directory):

| Stage | Lines | Notes |
|-------|------:|-------|
| `lex` | 1,204 | |
| `parse` | 2,652 | grammar in `spec/grammar.ebnf.md` |
| `ast` | 939 | |
| `load` | 1,154 | module loading, embedded stdlib |
| `resolve` | 855 | |
| `sema` | 9,339 | `call.rs` 2,574, `expr.rs` 1,951 |
| `types` | 3,394 | layout and the type table |
| `consts` | 1,879 | the only constant folder; `const` evaluation only |
| `analysis.rs`, `driver.rs` | 801 | front-end stage order and the `clc` pipeline |
| `own` | 37,963 | ownership, borrow (NLL v2), drop, lowering; `lower.rs` 7,565, `expr.rs` 5,648, `place.rs` 5,118 |
| `ir` | 2,355 | monomorphization, places, guards, control; no optimization passes |
| `codegen` | 20,308 | `function.rs` 5,648, `program.rs` 5,339, `guards.rs` 2,285, `expr.rs` 2,291 |
| `borrow.rs`, `init_graph.rs`, `abi.rs`, `ffi.rs`, `profile.rs`, `stdlib.rs` | 3,224 | |
| `clorraine/` (driver) | 5,126 | only the `toolchain.rs` flag tables are in scope (AR #9) |

### Toolchain flags (`compiler/src/clorraine/toolchain.rs`)

| Profile | Base | Debug | Release | Size |
|---------|------|-------|---------|------|
| hosted-amiga | `-std=c99 -mcrt=nix20 -m68000 -fwrapv` (line 60) | `-O0 -g` + `-Wall -Wextra -Werror` | `-O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s` | same with `-Os` |
| hosted-amiga-13 | `-std=c99 -mcrt=nix13 -m68000 -fwrapv` (line 63) | as above | as above | as above |
| freestanding | `-std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv` | as above | `-O2` | `-Os` |
| host | `-std=c99` | as above | `-O2` + section GC where the linker accepts it | `-Os` + section GC |

Every Amiga profile targets the plain 68000, including A1200 use. There is no 68020 variant;
a 68020 binary has never been run in this repository (`docs/findings-phase0.md`).

### Runtime shims (`compiler/runtime/`, 11,611 lines)

`hosted-amiga-13` embeds only `shim_hosted_amiga.c`; `hosted-amiga` appends `_20`, the
optional shims, and `picasso96.h` (`compiler/src/profile.rs:131-146`).

| File | Lines | Role | Workstream |
|------|------:|------|-----------|
| `shim_hosted_amiga.c` | 6,030 | KS1.3-compatible core shim | WS5a |
| `shim_hosted_amiga_20.c` | 497 | 2.0+ additions | WS5a |
| `shim_hosted_amiga_optional.c` | 1,216 | optional add-on base | WS5b |
| `shim_hosted_amiga_optional_services.c` | 1,065 | services add-ons | WS5b |
| `shim_hosted_amiga_optional_ui.c` | 635 | UI add-ons | WS5b |
| `shim_hosted_amiga_optional_rtg.c` | 600 | Picasso96 RTG | WS5b |
| `shim_hosted_amiga_optional_net.c` | 361 | bsdsocket | WS5b |
| `shim_host.c` | 671 | host | WS5b |
| `shim_freestanding.c` | 328 | freestanding | WS5b |
| `picasso96.h` | 173 | vendored definitions | WS5b |
| `emulator_runner.c` | 35 | FS-UAE guest wrapper used by the driver | excluded — driver surface (AR #9) |

Pointers for WS5, **not** findings: raw token counts across the hosted-amiga shims —
`OpenLibrary` 27 vs `CloseLibrary` 1 (closing may be centralized), `AllocMem` 97,
`MEMF_CHIP` 6, `Forbid` 3, `CreatePort` 7, `OpenDevice` 11, `SIGBREAKF_CTRL_C` 0. Commit
`ede6823a` added a raster-DMA write to `cl_video_display_flip`
(`shim_hosted_amiga.c:4543`, write near lines 4579–4587), a direct custom-chip access for
WS5-09.

The hosted-amiga shims never run under ASan or UBSan; only `shim_host.c` does (AGENTS.md
§Testing).

### Standard library (`stdlib/`)

43 modules: six core modules (`std.fmt`, `std.io`, `std.iff`, `std.math`, `std.mem`,
`std.text`) and 37 `std.amiga.*` modules (amigaguide, arexx, args, asl, audio, boopsi,
bsdsocket, clipboard, commodities, custom, datatypes, devices, dos, eclock, exec, expansion,
gadtools, graphics, icon, ilbm, input, interrupt, intuition, layers, locale, lowlevel,
nonvolatile, picasso96, pool, regions, semaphore, sprite, text, timer, utility, video,
workbench).

### Verification and measurement assets

| Asset | Facts |
|-------|-------|
| `spec/` | 18 chapters, grammar, diagnostics registry, `decisions.md`, two appendices; goldens: 115 compile-fail, 7 run, 4 staged, 7 corpus |
| `compiler/tests/` | 157 entries; spec/impl suite pairs per feature |
| `benchmarks/` | `calibration` plus eight scored benchmarks: `container`, `local-table`, `lookup`, `matrix`, `moves`, `sieve`, `strings`, `text-scan`; each has `expected-output.txt` |
| `tools/perf/bench.py` | Builds `program.clo` for host and hosted-amiga only, hard-coded `-mcrt=nix20 -m68000 -O2` (`bench.py:37,48-54`); nets counts against the Clorraine `calibration` program (`:490-512`); compares output after `normalize_output` (`:171-178`); `make bench*` write `build/perf/*.json` |
| `tools/perf/compile_time.py` | Generated scaling inputs at 1,000–8,000 (`:59`) |
| `tools/perf/flag_audit.py` | `hosted-amiga-13` only for `sieve` and the corpus, `-O2` variants |
| `tools/fuzz/` | Typed template generator, 24 seeds |
| `docker/Dockerfile` | `vamos` installed from amitools git HEAD, unpinned (`:131-133`); no profiler |
| `docker/scripts/run.sh` | Generic host-side FS-UAE runner (`RUNNER=adf RUN_CAPTURE=1`), fixed outputs `build/run/out.txt`, `build/run.adf` |
| `build/perf/` | Earlier JSON from previous plans; stale for this baseline |

### Prior audits this assessment must not simply repeat

| Plan | What it settled |
|------|-----------------|
| `generated-c-optimization` | Benchmark harness and the first emitted-C shape changes |
| `performance-levers-closeout` | Release flags and measurement gates |
| `real-workload-audit` | Bounds checks, generic calls, and struct copies are not top costs on `text-scan`/`container`; the allocator growth was |
| `growable-array-growth` | Amortized reserve growth |
| `compiler-throughput` | `clc` wall time and RSS |
| `generated-c-robustness` | Strict-build robustness of emitted C |

These conclusions are inputs to re-test, not proofs (03-01 §Independence).

### Defect history

The roadmap tracks 570 `↳ DEF-` rows and the highest identifier is `DEF-581`. Plan names
show repeated residue series: `template-placeholder-residuals` i–iii,
`conditional-projection-residue` i–ii, `residual-major-closeout` i–ii,
`memory-critical-*`, `def-zero-*`. `docs/known-limitations.md` (282 lines) lists accepted
limitations.

## Gaps Identified

### Gap 1: No whole-system view
**Current Behavior:** every audit was scoped to one plan's defect or shape.
**Required Behavior:** one assessment across all in-scope components, with ranked findings.
**Fix Required:** this plan (R2, R8).

### Gap 2: Overhead against C is unmeasured
**Current Behavior:** benchmarks compare Clorraine against earlier Clorraine.
**Required Behavior:** a fair hand-written C reference per benchmark (R4).
**Fix Required:** Phase 0 references, calibration, and the measurement matrix (03-01 §Measurement rules).

### Gap 3: The optimizer question has no answer
**Current Behavior:** optimization is GCC's job; nothing measures what the emitted shapes cost it.
**Required Behavior:** a measured answer (R5).
**Fix Required:** WS4 patched-C probes.

### Gap 4: Amiga shims are outside sanitizer coverage
**Current Behavior:** only the host shim runs under ASan/UBSan.
**Required Behavior:** a manual review of every hosted-amiga path plus targeted `vamos`/FS-UAE probes (WS5).

### Gap 5: Recurring defect classes are not root-caused
**Current Behavior:** residue plans fix instances.
**Required Behavior:** a class-level analysis (R3).

## Dependencies

### Internal Dependencies
- The pinned SDK container (`make -C docker exec CMD=...`), `m68k-amigaos-gcc`, vbcc, `vamos`, the host-side FS-UAE runner with the licensed media under `~/amiga`.
- The specialist briefs listed in 03-01 §Agent routing, used as checklists.

### External Dependencies
- None.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Plausible but false findings | High | High | Evidence standard and falsifiability (03-01), adversarial verification (Phase 9) |
| False "clean" verdicts | Medium | High | Clean evidence rule and the Phase 9 clean sample (03-01 §Verification protocol) |
| Re-litigating settled prior decisions | Medium | Medium | Prior conclusions are reopened only by a new measurement (03-01 §Independence) |
| Baseline or tool drift during a multi-session run | Medium | High | Drift check at every gate, pinned `clc`, recorded image and amitools versions (03-01 §Ground rules) |
| `vamos` lacks a 68020 mode | Medium | Medium | Static 68020 section sizes plus a named limit (03-01 §Measurement rules) |
| Context loss inside large tasks | Medium | Medium | Per-file coverage tables in each workstream file as the resume point |
| Budget overrun | Medium | Medium | Effort per AR #19; sequential sessions |
