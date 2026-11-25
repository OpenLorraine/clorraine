# Current State: Independent Assessment of Clorraine

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

Facts recorded at planning time (commit `8b7ed4c0`, 2026-11-25). Phase 0 re-records the
inventory at the pinned baseline SHA; where the two differ, Phase 0 wins.

## Existing Implementation

### What Exists

`compiler/src/` holds 95,788 lines of Rust in one std-only crate. The pipeline is
`lex → parse → load → resolve → sema → own → ir → codegen`, emitting one C99 translation unit
per program. Per-stage size (the `.rs` root file plus its directory):

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
| `own` | 37,963 | ownership, borrow (NLL v2), drop, lowering; `lower.rs` 7,565, `expr.rs` 5,648, `place.rs` 5,118 |
| `ir` | 2,355 | monomorphization, places, guards, control; no optimization passes |
| `codegen` | 20,308 | `function.rs` 5,648, `program.rs` 5,339, `guards.rs` 2,285, `expr.rs` 2,291 |
| `borrow.rs`, `init_graph.rs`, `abi.rs`, `ffi.rs`, `profile.rs`, `stdlib.rs`, `driver.rs` | 3,572 | |
| `clorraine/` (driver) | 4,685 | only `toolchain.rs` flag tables are in scope (AR #9) |

### Toolchain flags (`compiler/src/clorraine/toolchain.rs`)

| Profile | Base | Debug | Release | Size |
|---------|------|-------|---------|------|
| hosted-amiga | `-std=c99 -mcrt=nix20 -m68000 -fwrapv` (line 60) | `-O0 -g` + `-Wall -Wextra -Werror` | `-O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s` | same with `-Os` |
| hosted-amiga-13 | `-std=c99 -mcrt=nix13 -m68000 -fwrapv` (line 63) | as above | as above | as above |
| freestanding | `-std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv` | as above | `-O2` | `-Os` |
| host | `-std=c99` | as above | `-O2` + section GC where the linker accepts it | `-Os` + section GC |

Every Amiga profile targets the plain 68000, including A1200 use. There is no 68020 variant.

### Runtime shims (`compiler/runtime/`, 11,610 lines)

| File | Lines | Role |
|------|------:|------|
| `shim_hosted_amiga.c` | 6,029 | KS1.3-compatible core shim |
| `shim_hosted_amiga_20.c` | 497 | 2.0+ additions |
| `shim_hosted_amiga_optional.c` | 1,216 | optional add-on base |
| `shim_hosted_amiga_optional_services.c` | 1,065 | services add-ons |
| `shim_hosted_amiga_optional_ui.c` | 635 | UI add-ons |
| `shim_hosted_amiga_optional_rtg.c` | 600 | Picasso96 RTG |
| `shim_hosted_amiga_optional_net.c` | 361 | bsdsocket |
| `shim_host.c` | 671 | host |
| `shim_freestanding.c` | 328 | freestanding |
| `picasso96.h`, `emulator_runner.c` | 208 | vendored definitions, runner |

Raw token counts across the hosted-amiga shims, as pointers for WS5 and **not** findings:
`OpenLibrary` 27 vs `CloseLibrary` 1 (closing may be centralized), `AllocMem` 97,
`MEMF_CHIP` 6, `Forbid` 3, `CreatePort` 7, `OpenDevice` 11, `SIGBREAKF_CTRL_C` 0.

The hosted-amiga shims never run under ASan or UBSan; only `shim_host.c` does (AGENTS.md
§Testing).

### Standard library (`stdlib/`)

45 modules: `std.fmt`, `std.io`, `std.iff`, `std.math`, `std.mem`, `std.text`, and 39
`std.amiga.*` modules covering exec, dos, graphics, intuition, gadtools, layers, audio,
input, timer, sprite, video, custom, ilbm, icon, workbench, expansion, picasso96, bsdsocket,
datatypes, amigaguide, arexx, asl, boopsi, clipboard, commodities, devices, eclock,
interrupt, locale, lowlevel, nonvolatile, pool, regions, semaphore, text, utility, args.

### Verification assets

| Asset | Facts |
|-------|-------|
| `spec/` | 18 chapters, grammar, diagnostics registry, `decisions.md`; goldens: 115 compile-fail, 7 run, 4 staged, 7 corpus |
| `compiler/tests/` | 156 entries; spec/impl suite pairs per feature |
| `benchmarks/` | `calibration`, `container`, `local-table`, `lookup`, `matrix`, `moves`, `sieve`, `strings`, `text-scan` |
| `tools/perf/` | `bench.py` (size, m68k sections, host time, `vamos` instruction count), `compile_time.py`, `flag_audit.py` |
| `tools/fuzz/` | typed template generator, 24 seeds |
| `build/perf/` | earlier `head.json`, `base.json`, `flags.json`, `compile-*.json` from previous plans (stale for this baseline) |

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
**Required Behavior:** a hand-written C reference per benchmark (R4).
**Fix Required:** Phase 0 references, WS4 and WS7 measurement.

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
- The pinned SDK container (`make -C docker exec CMD=...`), `m68k-amigaos-gcc`, vbcc, `vamos`, FS-UAE with the licensed media under `~/amiga`.
- The existing agents listed in 00-index §Specialist Agents.

### External Dependencies
- None.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Plausible but false findings | High | High | Evidence standard and falsifiability (03-01), adversarial verification (Phase 9) |
| Re-litigating settled prior decisions | Medium | Medium | Prior conclusions are re-tested with new evidence or cited, never reopened by opinion (03-01 §Independence) |
| Baseline drift during a multi-session run | Medium | High | Every finding cites the pinned SHA; Phase 0 pins it |
| `vamos` lacks a 68020 mode | Low | Medium | Phase 0 checks it; fall back to FS-UAE A1200 timing for the WS7 probes |
| Context loss across sessions | Medium | Medium | Findings are written to disk per task; the execution plan is the progress authority |
| Budget overrun | Medium | Medium | Effort per AR #19; workstreams run in separate sessions |
