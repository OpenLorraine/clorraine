# WS7 — Runtime and compile-time performance

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-03-runtime-performance-workstreams.md §WS7 (WS7-01..WS7-07)

## Scope and limits

Runtime cost on the Phase 0 lenses (the matrix in `records/WS0-matrix.json` is the
source for instruction counts and section sizes, PF-008), host wall time (probe 700),
A500 footprint (730), and compile time and peak RSS of the pinned release `clc` (710 and
the Phase 0 `compile_time.py` record). The FS-UAE frame-timing probe is not written:
WS5 and WS6 recorded no suspected cost in the display paths (03-03 WS7-05 makes the
probe conditional on one).

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | high | 2026-11-26 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (7.1.1) | `probes/WS7/700`, `710`, `730`, `740` | done |
| Red check (7.1.2) | seeded detectors, 740 contrast | done |
| Matrix and host time (7.2.1) | Phase 0 matrix, 5 lenses × 9 programs × 2 sides; 700 | done |
| Footprint and placement (7.2.2) | 31 A500 programs (730); `MEMF_CHIP` sites | done |
| 68020 and frames (7.2.3) | Phase 0 `a1200`/`a1200-020` lenses; WS5/WS6 display-path reviews | done |
| Compile time and RSS (7.2.4) | 710 (5 series × 4 sizes), 750 (gdb samples), Phase 0 compile record | done |
| Self-refutation (7.3.1) | `compiler-throughput`, `performance-levers-closeout` | done |

## Checklist outcomes

Runs 2026-11-26 (logs under `build/tmp/assessment/WS7/`; committed record
`records/WS7-runs.log`; matrix `records/WS0-matrix.json`).

| ID | Outcome | Evidence |
|----|---------|----------|
| WS7-01 | ASM-WS4-001, ASM-WS4-002 (cited); host time `limit` | Matrix outliers per lens (net instructions, Clorraine vs C): A500 -O2 `strings` +71.5%, `container` +38.5%, `text-scan` +8.2%, `local-table` +4.0%, `lookup` +1.5%, `matrix`/`moves`/`sieve` ≤ 0% (WS4); A1200 shipped `strings` +51.3%, `container` +38.8%, `text-scan` +8.5%, `local-table` +4.5%, `matrix` −9.1%, `moves` −16.5%. Host time (700, 15 runs, all outputs correct): every benchmark runs about 1–1.6 ms, dominated by process start; spreads reach 9–468% and every net difference is below its spread, so host time cannot rank the code (a `limit`: removed by workloads scaled to ≥100 ms on the host, which the benchmark suite does not have) |
| WS7-02 | ASM-WS7-001 | 730: the 31 programs the 1.3 profile accepts occupy 5.9–56.6 KB static (`lookup` 56,616 B = 10% of 512 KB, the largest; the largest demo `gui` 39,476 B = 7%); none near the budget. Startup: the A500 calibration runs 2,919 instructions for Clorraine and 2,918 for C (Phase 0 matrix). The exception is a module-level fill, whose code grows per element (740, ASM-WS7-001) |
| WS7-03 | ASM-WS7-003 | 68020 probe build (`-m68020`, `vamos -C 68020`) vs. A1200 shipped, Clorraine net instructions: `strings` −16.3% (hardware divide), `local-table` −4.4%, `text-scan` −3.7%, `lookup` −1.3%, `matrix`/`moves`/`sieve` ±0.5%, `container` **+21.3%** (the C reference +2.1%); `.text` −2% everywhere. Flag results are WS4-08's. Unranked recommendation: a 68020 profile variant is not justified by these gains; the `strings` gain is better taken by ASM-WS5-022 on every CPU |
| WS7-04 | `clean` (with ASM-WS5-014 cited) | Chip placement: bitplanes through `AllocRaster` (`shim_hosted_amiga.c:3807-3830`), sprite images (`:4806`, `:4808`, `:5035`), audio samples (`:5511`), the pointer image (`:2673`), user `ChipBuffer`s (`:3700`), copper lists through graphics.library; descriptors and handles are `MEMF_ANY` (46 other `AllocMem` sites), and the C heap is libnix `malloc` (any memory). Nothing that needs no DMA is in chip memory, so no A1200 slowdown from placement; the reverse case is the 1.3 device buffer (ASM-WS5-014) |
| WS7-05 | `clean` | No suspected display-path cost in WS5/WS6: the video, sprite, copper, and blit paths were read function by function (WS5 coverage rows `shim_hosted_amiga.c` 3100–6030; findings there are correctness: ASM-WS5-017 `WaitBlit`, ASM-WS5-019 b/c/h), and WS4's attribution found no display-path cost in the benchmarks. No frame-timing probe written (03-03 WS7-05) |
| WS7-06 | ASM-WS7-002 | 710 (3-run medians, host): `functions` 61/93/191/441 ms (exponent 1.21); `locals` 58/131/379/1,287 ms (1.76); `closure` 228/564/1,857/6,973 ms (1.91); `ownership` 658/2,711/8,775/45,475 ms (2.37); `generic` 1,039/3,301/14,203/77,566 ms (2.45). Attribution (750, gdb samples, `probable`): own-pass call resolution and instance recording |
| WS7-07 | `clean` | Peak RSS grows linearly (exponents 0.85–1.02): at 8,000 the series use 52–490 MB; `ownership` costs about 61 KB per function (490 MB), below 10× the Phase 0 `functions-8000` 82 MB (820 MB). The samples show per-call `Vec<Ty>` clones (`sample-gen.log`), a constant-factor cost, not growth |

## Findings

### ASM-WS7-001 — A module-level fill emits one assignment per element: a 64 KB table needs 548 KB of code and 163 s of GCC

- **Severity**: Major
- **Confidence**: verified
- **Lens**: A500 (measured); A1200 shipped and host (the same emission)
- **Location**: module-initializer lowering of `let g: T[N] = [; v];` (emitted into the module's init function as `cl_<module>_<name>[i] = v;` for every `i`, after a static `= {0}`; the exact emitter site was not traced), at e24a8752
- **Checklist item**: WS7-02, WS7-06 (also WS3-11)
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — probe `probes/WS7/740-fill-emission.sh`, command `make -C docker exec CMD="bash plans/independent-assessment/probes/WS7/740-fill-emission.sh build/tmp/assessment/baseline/clc build/tmp/assessment/WS7/fill"`, A500 release line (`-mcrt=nix13 -m68000 -O2`, section GC, strip), output checked under `vamos` ("1" in every case). Module-level `u8` fill: 1,024 → 7,216 C lines, GCC 3.0 s, `.text` 25,932 B; 16,384 → 22,576 lines, 38.4 s, `.text` 154,460 B; 65,536 → 71,728 lines, 163.1 s, `.text` 547,676 B (image 613,612 B > 512 KB). The same fill as a function local: 6,193 lines and about 2.2 s at every size, `.text` 23,384 B at 65,536. A 614,400-element module fill (also with `[; 0]`) emits a 22 MB unit that GCC did not finish in 30 minutes (`records/WS7-runs.log`). Expected (`740-fill-emission.expected`: spec/08 §2 fill semantics; 03-01 §Severity C-compile limit about 32 s): size-independent C and GCC within 32 s.
- **Impact**: a demo or game that declares a module-level buffer with a fill (a 16 KB sample buffer, a 10 KB bitplane, a lookup table of a constant) gets about 8 bytes of 68000 code per element and runs that code at startup; at 64 KB the image no longer fits the A500, and the C build takes minutes. A zero fill pays the same cost although the static is already zero.
- **Root cause**: the module initializer expands the fill into per-element stores instead of a loop (or, for zero, nothing beyond the static `{0}`) as the function-local lowering does.
- **Recommendation**: lower a module-level fill like the local one (a loop, or `memset` for a byte fill); emit nothing for a zero fill; for a constant fill of a `const`-able value consider a static initializer.
- **Effort**: S
- **Related**: ASM-WS4-004 (module tables occupy RAM twice); DEF-309 (the fill expression evaluated per element — a different, fixed defect); WS3-11
- **Self-refutation (7.3.1)**: `performance-levers-closeout` and `constant-array-initializer` measured constant-initializer tables, not fills; no limitation entry covers fills at module level. Major (≥10% `.text` and C compile time above 10× on a shipped 68000 build).
- **Phase 9 additions**: emitter site traced by the verifier: `compiler/src/codegen/program.rs:1974`, `:2058-2085`; related to ASM-WS4-004 (both lower module initializers as run-time stores; different shapes and remedies).
- **DEF**: DEF-596 (`plans/00-roadmap.md`)
- **Issue**: #68 (finding issue)
- **Verdict**: confirmed (Major, verified) — verifier ses_dea685c4bffegV12VLXpl04mC6 (9.2.5): 740 reproduces (GCC 160 s at 65,536; `.text` 547,676 B); a `[; 0]` module fill is expanded too; emitter site `codegen/program.rs:2058-2085` `emit_filled_array` (called from `emit_global_array_fill`, `:1974`), while the function path compacts (`codegen/function.rs:4337-4675`).

### ASM-WS7-002 — Compile time grows about quadratically for call-heavy and generic code

- **Severity**: Major
- **Confidence**: verified (the measurement); the attribution is `probable`
- **Lens**: host (`clc` itself)
- **Location**: `compiler/src/own/call.rs:1261-1291` (`resolved_callee`: `declared_function` and `declared_ffi`, `:1453-1469`, scan the user module's `items` linearly before imports are consulted, and the resolver runs several times per call from `call_may_write_globals`, `infer_call_types`, `lower_intrinsic_call`, `call_view_source`, `check_dot_receiver`); `compiler/src/ir/mono.rs:690-711` (`Monomorphization::record` and `record_struct` compare each new instance against every recorded one), at e24a8752
- **Checklist item**: WS7-06
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — probe `probes/WS7/710-compile-scaling.py`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS7/710-compile-scaling.py build/tmp/assessment/baseline/clc build/tmp/assessment/WS7/m710"` (pinned release `clc`, host profile, 3-run medians): 8,000 one-line functions 441 ms; 8,000 functions with owned `String`s and stdlib calls 45,475 ms (time exponent 2.37, 103× the one-line series at the same function count, 152× the Phase 0 `functions-8000` 299 ms); 8,000 `Array<S_i>` instantiations 77,566 ms (exponent 2.45). Attribution — probe `750-sample.sh` (gdb stack samples once a second; host `perf` is locked by `perf_event_paranoid` 4): 7 of 8 `ownership-4000` samples and 4 of 10 `generic-4000` samples are in `resolved_callee` or `function_identity`; 2 `generic` samples are in `Monomorphization::record`/`record_struct` comparisons (`records/WS7-runs.log`).
- **Impact**: compile time is fine for today's examples (the largest compile in Phase 0 took 18.7 ms), but a program of a few thousand functions that use the stdlib — the size of a real game or a generated binding — takes tens of seconds and the LSP, which runs the same analysis on every edit, becomes unusable earlier. The closure (1.91) and locals (1.76) series are superlinear too; WS1 measured 53 s for 50,000 locals.
- **Root cause**: name resolution in the ownership pass and instance deduplication use linear scans inside per-call and per-instance loops, so cost is the product of the call count and the module size.
- **Recommendation**: build a per-module name index once (as `compiler-throughput` did for codegen bodies) and look callees up in it; key instances in a map (`BTreeMap`/`HashMap` by the instance key) instead of `iter().any`; add the 710 series to `compile_time.py` so regressions show.
- **Effort**: M
- **Related**: `plans/compiler-throughput/05-findings.md` §2 (codegen fixed; "the ownership pass … close to linear" on the mixed shape; "residual generic-instance … lookups still scan the function table"); WS1-10 robustness (locals)
- **Self-refutation (7.3.1)**: the earlier plan measured shapes with few calls per body and no generics; the new series contradict its "close to linear" statement only for call-heavy and generic bodies, which is the claim here. The 10× rule is applied at equal function count, stated explicitly; against each series' own linear extrapolation the 8,000 point is 8.6× (ownership) and 9.3× (generic) slower. Major.
- **Phase 9 corrections**: average exponent over 1,000→8,000 is about quadratic (ownership 2.22, generic 1.85), above quadratic on the top doubling; add `compiler/src/codegen.rs:1009-1020` (`resolve_function` fallback scan) to the location — the residual generic-instance lookup `compiler-throughput` left open.
- **DEF**: DEF-610 (`plans/00-roadmap.md`)
- **Issue**: #82 (finding issue)
- **Verdict**: confirmed (Major; measurement verified, attribution probable and reproduced) — verifier ses_dea685c4bffegV12VLXpl04mC6 (9.2.5): every superlinear series stays superlinear under load (ownership top-doubling exponent 2.94, average 2.22 over 1,000→8,000; generic 2.12/1.85); title correction: "faster than quadratically" holds for the top doubling, about quadratic on average; gdb samples add `codegen::Emitter::resolve_function`'s linear fallback (`compiler/src/codegen.rs:1009-1020`) to the location.

### ASM-WS7-003 — The 68020 probe build of `container` runs 21% more instructions than the 68000 build

- **Severity**: Opportunity
- **Confidence**: verified
- **Lens**: A1200 68020 probe build (`-m68020`, `vamos -C 68020`) — a 68020 probe build number, not a shipped-profile number
- **Location**: `benchmarks/container` under the `a1200-020` lens, at e24a8752
- **Checklist item**: WS7-03
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — Phase 0 matrix (`records/WS0-matrix.json`, `measure.py`): Clorraine net instructions 56,814 (A1200 shipped, 68000) → 68,928 (68020 probe build), +21.3%; the C reference 40,928 → 41,793, +2.1%; Clorraine/C ratio +38.8% → +64.9%.
- **Impact**: none on a shipped profile; it would matter if a 68020 variant were added.
- **Root cause**: not attributed (the WS4 attribution script runs the A500 lens only); a `limit` for the cause.
- **Recommendation**: attribute with `400-attribute.py` on the 68020 lens before any 68020 profile work.
- **Effort**: S
- **Related**: ASM-WS4-002 (the `container` gap on the 68000)
- **Self-refutation (7.3.1)**: an instruction count, not cycles; a 68020 executes fewer cycles per instruction, so the time effect is unknown. Opportunity by the 68020 rule.
- **Issue**: #15 (comment on the existing issue)
- **Verdict**: confirmed (Opportunity, verified) — verifier ses_dea685c4bffegV12VLXpl04mC6 (9.2.5): the counts reproduce exactly; `container` is the only 68020 outlier.

## Spec questions

None.

## Falsifiability log

Red check 2026-11-26 (logs `build/tmp/assessment/WS7/red.log`, `fill.log`).

| Probe | Form | Result |
|-------|------|--------|
| 700 | mutated expectation (the `sieve` golden replaced; a comparator script) | both `sieve` rows FAIL |
| 710 | seeded defect: a program with a type error | FAIL (exit 1, no C file) |
| 730 | seeded defect: a 600 KB static image | first seed (a Clorraine `[; 1]` module fill) did not build within 30 min — the observation behind probe 740; second seed (a zero fill) also unrolled to 22 MB; third (a C array) was folded away by `-O2` (1,920 B); fourth (`big[argc] = 1`) reported OVER at 616,344 B. Each change committed before its rerun (`8e49b2e6`, `cd722911`, and the follow-up) |
| 750 | attribution only (gdb stack samples stand in for the locked host `perf`); no pass/fail | 8 and 10 samples recorded |
| 740 | contrast: 1,024-element cases against 16,384 and 65,536, module level against local | discriminates: the local form is constant (6,193 lines), the module form grows per element |

## Self-refutation (7.3.1)

- **`compiler-throughput`**: its fix indexed codegen bodies and measured function and
  mixed farms; it recorded the ownership pass as "close to linear" and left generic and
  lambda lookups unmeasured. ASM-WS7-002 contradicts it only with new series (710) on
  shapes it did not run; the `functions` series reproduces its near-linear result
  (exponent 1.21).
- **`performance-levers-closeout`**: its flag and size conclusions are WS4's (ASM-WS4-006
  already reconciles them); its table work concerned constant initializers, not fills,
  so ASM-WS7-001 does not contradict it.
- **Host time**: recorded as a `limit`, not a finding — the measurement cannot separate
  the code from process start at these sizes.
- **No Critical.**

## Phase 7 gate (7.3.2)

| Case | Result |
|------|--------|
| Task verify | drift check PASS |
| ST-1 | three findings, every template field, IDs `ASM-WS7-001..003` |
| ST-2 | each verified finding names its probe or record and command |
| ST-3 | no `hypothesis` |
| ST-4 | ASM-WS7-001 states both sides (module vs. local fill, same lens and compiler) on the shipped A500 line; ASM-WS7-002 is a compile-time Major judged by the 10× rule, stated with both baselines |
| ST-5 | probes committed in `9a717210`; 740 in `8e49b2e6`; seed fixes `cd722911` and its follow-up; 750 before its run; falsifiability log complete |
| ST-6 | no spec disagreement |
| ST-7 | drift check PASS |
| ST-8 | WS7-01..WS7-07 each have one outcome; every coverage-table row complete |
| ST-11 | no Critical |
| ST-12 | every number cites `records/WS7-runs.log` or the Phase 0 records with the command |
| ST-14 | no patched-C estimate |
| ST-15 | ASM-WS7-003 and the WS7-03 row are labelled "68020 probe build" and rated Opportunity |
| ST-16 | dispatch record `high` |

## Escalations

None yet.

## Probe index

| Item | Probes |
|------|--------|
| WS7-01 host time | 700 |
| WS7-02 footprint | 730 |
| WS7-06/07 compile scaling | 710 (+ Phase 0 `records/assessment-baseline-compile.json`), 740 (fill emission), 750 (gdb samples) |
