# Runtime and Performance Workstreams: Independent Assessment of Clorraine

> **Document**: 03-03-runtime-performance-workstreams.md
> **Parent**: [Index](00-index.md)

## Overview

Checklists for WS5–WS8 and the defect-history analysis. Item IDs, outcome rules, and evidence
follow 03-01 and ST-8. Partition per AR #18; effort per AR #19.

---

## WS5 — Runtime shims

**Components:** every shim file in `compiler/runtime/` (AR #9), assigned per the table in
`02-current-state.md` §Runtime shims:
WS5a — `shim_hosted_amiga.c`, `shim_hosted_amiga_20.c`;
WS5b — the five `shim_hosted_amiga_optional*.c` files, `shim_host.c`, `shim_freestanding.c`,
`picasso96.h`. `emulator_runner.c` is excluded as driver surface (AR #9).
**Spec:** chapters 11, 12, 15, `appendix-amiga.md`, `appendix-stdlib.md`; NDK autodocs and
include prototypes as expectation sources.
**Method:** function-by-function review of every exported function, tracked in the coverage
table; `vamos` probes for DOS/exec paths; FS-UAE probes (03-01 §Ground rules) only where
`vamos` cannot model the behavior (graphics, devices, Workbench, chip memory, Kickstart 1.3
ROM differences) (AR #6); host-shim probes under the 03-01 sanitizer builds.

| ID | Check |
|----|-------|
| WS5-01 | Kickstart availability: every library call, structure field, and tag used on the KS1.3 path exists in V34; every 2.0+ call is reachable only behind a version check or the `_20` shim; cross-check `appendix-amiga.md` |
| WS5-02 | Library lifecycle: every `OpenLibrary`/`OpenDevice` has a version argument that matches the calls made, a failure path, and exactly one matching close on every exit, including fault exits (the raw count is 27 opens vs. 1 `CloseLibrary` token; verify the central close mechanism) |
| WS5-03 | Memory: every `AllocMem` has a matching `FreeMem` of the same size on every path; `MEMF_CHIP` for every buffer the custom chips DMA (bitplanes, sprites, audio, copper lists, blitter sources); `MEMF_PUBLIC` where shared; alignment of DMA buffers |
| WS5-04 | Failure paths: allocation failure, missing library, missing device, out-of-range arguments; each fails cleanly with resources released and a defined result to the Clorraine caller |
| WS5-05 | Process context: default stack size on KS1.3 (4 KB) vs. stack use of the shim and emitted code; recursion; `Forbid`/`Permit` pairing; interrupt-handler code using only interrupt-safe calls |
| WS5-06 | Signals and messages: signal-bit allocation and release; port creation and deletion (`CreatePort` on 1.3, `CreateMsgPort` on 2.0+); IORequest lifecycle including `AbortIO`/`WaitIO` before close; reply of every received message |
| WS5-07 | Break handling: Ctrl-C (no `SIGBREAKF_CTRL_C` token in the shims): what happens to a long-running Clorraine program on Ctrl-C, and whether owned resources are released on exit |
| WS5-08 | Exit and cleanup: `exit` paths, cleanup ordering, cleanup after a runtime fault, Workbench-start (WBStartup message) handling, `CurrentDir`/lock restoration |
| WS5-09 | Hardware access: custom-chip register writes respect OCS vs. AGA differences (including the raster-DMA write in `cl_video_display_flip`, `shim_hosted_amiga.c:4543`), `OwnBlitter`/`WaitBlit` discipline, `LoadView`/`WaitTOF` restoration of the system display, `Disable` usage |
| WS5-10 | Data-model crossings: 16-bit vs. 32-bit `int` assumptions, signed `char`, BPTR/BSTR conversions, big-endian IFF parsing, NDK prototype agreement (the DEF-559 class) |
| WS5-11 | Host shim (`shim_host.c`): ASan/UBSan probes over allocation, growth, string, and formatting routines; parity of observable behavior with the hosted-amiga shim (differential) |
| WS5-12 | Freestanding shim: correctness of the console-only surface; no hidden libc dependence; 32-bit big-endian assumptions |
| WS5-13 | Hot paths: cost of allocation, buffer growth, string building, integer formatting (68000 has no 32×32 multiply or 32/32 divide; check reliance on libgcc helpers), memory copy/fill on 68000 and the 68020 probe |
| WS5-14 | Duplication and drift: logic duplicated between shims (host vs. Amiga, 1.3 vs. 2.0) that can diverge; list every pair |

---

## WS6 — Standard library

**Components:** the 43 modules in `stdlib/` (6 core, 37 `std.amiga.*`), their shim entry
points, `compiler/src/stdlib.rs`; coverage table with one row per module.
**Spec:** `appendix-stdlib.md`, `appendix-amiga.md`.

| ID | Check |
|----|-------|
| WS6-01 | API conformance: every public function against `appendix-stdlib.md` and `appendix-amiga.md`: signature, ownership of arguments and results, nullability, fault behavior |
| WS6-02 | Core modules (`std.fmt`, `std.io`, `std.text`, `std.mem`, `std.math`, `std.iff`): boundary cases (empty, maximum length, invalid UTF-8 or IFF data, overflowing numbers, NaN/inf formatting) with host ASan/UBSan probes |
| WS6-03 | Hidden costs: allocations, copies, and quadratic loops in common operations (string building, formatting, searching, array growth) |
| WS6-04 | Amiga modules: the Kickstart version each module requires is declared and enforced; modules that silently fail on 1.3 |
| WS6-05 | Resource ownership: every Amiga handle wrapper (screens, windows, ports, requests, locks, files, fonts, BOOPSI objects) releases on drop and on fault paths; no double close when the user closes explicitly |
| WS6-06 | Consistency: naming, error-reporting style, and ownership conventions across the existing modules; missing pieces observed while assessing them go to the report's unranked recommendations (03-04 §8) |
| WS6-07 | `stdlib.rs` embedding: module discovery, version skew between embedded stdlib and shims, profile gating of modules |

---

## WS7 — Runtime and compile-time performance

**Components:** emitted programs, shims, release flags, `clc` itself.
**Method:** cite the Phase 0 measurement matrix and compile-time baseline; add only WS7's own
probes; FS-UAE frame timing only for display-bound demos (AR #14, AR #17).

| ID | Check |
|----|-------|
| WS7-01 | Benchmark matrix analysis: read the Phase 0 matrix (every benchmark × every lens × {Clorraine, C reference}) and add host time with ≥5 runs; identify the outliers per lens |
| WS7-02 | Realistic programs: the corpus programs and in-scope examples (`examples/amiga/`): `.text`, chip-memory footprint, startup instruction count, against the 512 KB A500 budget |
| WS7-03 | A1200 shipped and A1200 68020 probe lenses: cite WS4-08 for flag results; the gain of the 68020 probe build per benchmark; whether a 68020 profile variant is worth adding (to the report's unranked recommendations) |
| WS7-04 | Chip vs. fast memory: buffers placed in chip memory that need not be, and the reverse; effect on A1200 performance |
| WS7-05 | Display-bound demos: frame time of Boing and the sprite demo on A500 and A1200 under FS-UAE when WS5 or WS6 recorded a suspected shim cost; otherwise `clean` citing the WS5/WS6 probe IDs or `file:line` site lists that examined the Boing and sprite display paths |
| WS7-06 | Compile time: the `tools/perf/compile_time.py` series (1,000–8,000) plus ownership-heavy, generic, and closure generators under `probes/WS7/`; hotspots attributed by scaling exponent and code reading (confidence `probable`); host `perf` on the pinned `clc` only if it runs, otherwise a `limit` |
| WS7-07 | Compile memory: peak RSS growth with program size; data structures cloned per function or per instance |

---

## WS8 — Test and verification integrity, and defect history

**Components:** `compiler/tests/`, `spec/tests/`, `tools/fuzz/`, `tools/perf/tests/`, the
sanitizer runners, the roadmap DEF ledger. Limited to the confirmed scope (AR #18).

| ID | Check |
|----|-------|
| WS8-01 | Coverage map: for each WS1–WS7 checklist area, which spec/impl suites, goldens, fuzz templates, and sanitizer runs exercise it; areas with none |
| WS8-02 | Gap linkage: for every WS1–WS7 finding of Minor or above, why the existing tests did not catch it (re-pointed in task 9.2.5 for refuted or merged findings) |
| WS8-03 | Assertion strength: a static classification of five tests per checklist area — each assertion checks a value, a diagnostic code, only "compiles", or only "no panic"; no test is run or edited |
| WS8-04 | Fuzzer: which language constructs the templates generate and which they never generate; whether the differential compares enough state |
| WS8-05 | Sanitizer holes: code that never runs under ASan/UBSan (the hosted-amiga shims, m68k-only paths) and what substitute evidence exists |
| WS8-06 | Pinned-text tests: impl tests that pin emitted C text; their share and whether they block improvements without guarding behavior |
| WS8-07 | Defect history: classify every DEF row by component, class (miscompile, memory safety, invalid C, false accept, false reject, performance, platform), and introducing area; recurrence per class over time. A class is **recurring** with ≥3 DEF rows across ≥2 plans |
| WS8-08 | Root causes: for each recurring class (the residue series), the structural cause; whether it is still present at the baseline (cross-check WS2-12); a structural remedy proposal |

## Testing Requirements

Acceptance per the `07-testing-strategy.md` check-point row of each phase.
