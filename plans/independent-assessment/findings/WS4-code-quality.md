# WS4 — Generated-code quality and the optimizer question

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-02-compiler-workstreams.md §WS4 (WS4-01..WS4-10)

## Scope and limits

Components: `codegen/`, `ir/`, `consts/`, the release flags, the emitted C of the
benchmarks. Measurements cite the Phase 0 matrix (`records/WS0-matrix.json`); WS4 adds
attribution traces, patched-C estimates (diffs against the baseline A500 emission), and
flag variants. Optimizer estimates patch emitted C only; no compiler is changed (AR #17).

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | high | 2026-11-25 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (4.1.1) | `probes/WS4/400`–`430` | done |
| Red check (4.1.2) | seeded symbol, seeded mismatch; attribution amended | done |
| Overhead and attribution (4.2.1) | matrix, `400-attribute.py` | done |
| Guard, drop, temporary probes (4.2.2) | patches 410–415 | done |
| Static state, size, flags (4.2.3) | 416, 417, `425-size.sh`, `420-flags.py` | done |
| Pass table and answer (4.2.4) | WS4-09, WS4-10 | done |
| Self-refutation (4.3.1) | against `real-workload-audit`, `generated-c-optimization`, `performance-levers-closeout` | done |

## Checklist outcomes

Records (ST-12): `records/WS0-matrix.json` (overhead matrix), `records/WS4-attribution.log`,
`records/WS4-patches.log`, `records/WS4-flags.log`, `records/WS4-size.log`; commands in each
probe's header, run 2026-11-25 in the SDK container.

| ID | Outcome | Evidence |
|----|---------|----------|
| WS4-01 | ASM-WS4-001, ASM-WS4-002 | Phase 0 matrix, net instructions Clorraine vs C reference (each netted against its own calibration). A500 -O2: container +38.5%, strings +32.6%, text-scan +7.1%, local-table +4.0%, lookup +1.5%, sieve +0.0%, matrix −8.9%, moves −16.5%. A500 -Os: container +57.8%, strings +19.4%, local-table +5.2%, text-scan −1.9%, matrix −9.4%, moves −13.0%, lookup −30.3%. A1200 shipped: container +38.8%, strings +51.3%, text-scan +8.5%. 68020 probe: container +64.9%, strings +71.5%. `.text` above calibration: +1.2% to +8.4% on every lens (no benchmark reaches 10%) |
| WS4-02 | ASM-WS4-001, ASM-WS4-002 | `400-attribute.py` (A500 -O2, unstripped, per function): strings — `cl_string_concat` 326,086, `cl_string_new` 32,032, extra `memcpy` 36,045 instructions on the Clorraine side, while digit formatting (`cl_format_u32`/`___udivsi4`) is identical on both sides; container — `cl_buffer_reserve` 8,465 (one call per `push`), `memset` 1,028, `main` +5,987 over C (the one-element view and its second check). text-scan: the 4.6% gap is inside `main` (below threshold, not attributed further) |
| WS4-03 | `clean` | patch 410 removes every range-provable `E11211` check in `sieve`: output identical, instructions **+13.0%** (1,043,170 → 1,178,532) and `.text` −36 B — GCC schedules the guarded loop better; a range-fact pass gives no gain on this shape. The C references carry the same checks, so guards are not part of any gap above |
| WS4-04 | `clean` | patch 415 removes the dead null-checked release at the `strings` loop head: +12 instructions (noise); drop flags and cleanup ladders that survive `-O2` cost nothing measurable here |
| WS4-05 | ASM-WS4-001, ASM-WS4-002 | patches 411 (−4,072, −7.1% of net), 412 (−13,157, −23.0%), 413 (−401,990, −23.8%); 414 (move the owned `toHex` placeholder instead of copying it): −127 instructions, −32 B |
| WS4-06 | ASM-WS4-004 | patch 416: a module `let` table with a constant initializer is a zeroed static plus a `const` template copied by `memcpy` at start-up; emitting it as an initialized static saves 16,384 B of `.bss` (and 7 instructions). The `if (sizeof(&x)) { }` statements are folded away by GCC (no code; code reading of the `-O2` assembly) |
| WS4-07 | ASM-WS4-003, ASM-WS4-005 | `425-size.sh` and patch 417: fixed cost +776 B (`calibration` 5,900 vs 5,124 B `.text`); `cl_trap`'s `printf` pulls `___vfprintf_total_size` (10,582 B) and soft-float long-double helpers (`___addxf3` 2,452, `___mulxf3` 1,124, `___cmpxf2` 776, …) into every program with a runtime check |
| WS4-08 | ASM-WS4-006 | `420-flags.py`, all variants print the goldens: `-fwrapv` and `-fomit-frame-pointer` change nothing (identical counts and size); `-flto` −280 to −512 B `.text` on A500 (container −7.3% instructions); `-O3` container −18.3%, strings −25.1%, moves −98.5%, local-table −5.1% at +272 to +2,216 B; `-Os` is not uniformly smaller or slower (container +25.1% instructions, +76 B). The 68020 probe build shows the same pattern (labelled as a probe build) |
| WS4-09 | table below | |
| WS4-10 | answer below | |

### WS4-09 — pass candidates

| Candidate | Removes | Measured gain (A500 -O2) | Implementation size | Correctness risk |
|-----------|---------|--------------------------|---------------------|------------------|
| Template specialization: a fixed-shape template lowered to an inline builder | the generic `ClStrPart` array and two-pass `cl_string_concat` | strings −23.8% net (patch 413) | M: one lowering path in the template emitter; the shim already has the primitives | low — output is byte-identical; the reserve size is already computed per part |
| `Array<T>` intrinsic lowering: `push` with an inline capacity test; `get(i)[0]` as one check and an indexed read | a runtime call per `push`, a one-element view, a redundant check | container −23.0% (412) and −7.1% (411) | M: the `std.mem` `Array` API becomes compiler-known | medium — element store and drop of the old slot must keep the ownership semantics |
| Constant module initializers as initialized statics | a zeroed static, a `const` template, a start-up `memcpy` | −16,384 B RAM for `lookup` (416) | S | low |
| Fault writer without `printf` (runtime, not a pass) | `vfprintf` and soft-float helpers in every checked program | −16,652 B `.text` on sieve (417) | S | low — the fault line is identical |
| Range-check elimination | provable bounds checks | none (sieve +13.0% slower without them, 410) | M | medium — not recommended |
| Static drop elaboration | dead release before an assignment | none measurable (415) | M | medium — not recommended |
| Placeholder move instead of copy | one allocation and copy per owned placeholder | −127 instructions (414) | S | low — only worth it with the template work |

### WS4-10 — the optimizer question

**Answer: targeted passes, not an optimizer stage.** GCC already optimizes the emitted
scalar code as well as it optimizes the hand-written C: sieve (+0.0%), matrix (−8.9%), moves
(−16.5%), lookup (+1.5%), local-table (+4.0%), and text-scan (+7.1%) are within the 10% line
or faster than the reference on the A500 -O2 lens, and removing provable bounds checks or
dead releases does not help (410, 415). The two measured gaps are both lowering shapes of
runtime-library calls, not missed scalar optimizations: the generic template builder
(ASM-WS4-001) and the `Array` API through runtime calls and one-element views
(ASM-WS4-002). Each is closed most of the way by one targeted lowering (413: −23.8%; 412+411:
about −30%). The largest size wins are in the runtime and data lowering, not in code
generation (ASM-WS4-003, ASM-WS4-004). An IR optimizer stage would duplicate what GCC does on
the scalar benchmarks.

## Findings

### ASM-WS4-001 — Templates cost 33% more instructions than equivalent C on the A500

- **Severity**: Major
- **Confidence**: verified
- **Lens**: A500 shipped -O2 (also A500 -Os +19.4%, A1200 shipped +51.3%, 68020 probe +71.5%)
- **Location**: template lowering (`cl_string_concat((ClStrPart[]){…}, n)` per template) in `compiler/src/codegen/`; `cl_string_concat` in `compiler/runtime/shim_hosted_amiga.c:1046-1094`, at e24a8752
- **Checklist item**: WS4-01
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — Phase 0 matrix (`records/WS0-matrix.json`, A500 shipped -O2 (hosted-amiga-13, `-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`, `vamos` 68000), pinned `clc` sha256 `0c2fd66e…`, baseline e24a8752): `strings` Clorraine 1,685,808 vs C reference 1,271,509 net instructions (+32.6%; raw 1,688,727 / 1,274,427; calibrations 2,919 / 2,918). Attribution (`400-attribute.py`, `records/WS4-attribution.log`): `cl_string_concat` 326,086, `cl_string_new` 32,032, and 36,045 extra `memcpy` instructions; formatting is identical on both sides. Patch 413 (`probes/WS4/413-strings-template-inline.diff`, `430-patch-measure.py`, `records/WS4-patches.log`) builds the same string inline: 1,688,727 → 1,286,737 (−401,990, −23.8% of net), output identical.
- **Impact**: every program that formats text in a loop (status lines, logs, UI labels) pays about a third more CPU on the 68000 than hand-written C for the string building.
- **Root cause**: each template fills a stack array of tagged `ClStrPart` structs and calls a generic builder that walks the parts twice (sizing, then writing) and copies literal pieces with `memcpy`, instead of emitting the fixed sequence of copies and formats the template's shape already determines.
- **Recommendation**: specialize templates with a fixed shape at lowering time (reserve the sum of the part maxima, copy literals with known lengths, format scalars in place), keeping `cl_string_concat` for the general case.
- **Effort**: M
- **Related**: WS4-09 template specialization; WS6-03 (hidden costs)
- **Self-refutation (4.3.1)**: earlier plans measured Clorraine against earlier Clorraine, not against C; none settled the template builder's cost. The fairness of the C reference was checked: it allocates one heap block per string, formats with the same least-significant-first writer, and frees it — the gap is the generic builder only (attribution). Stays Major (≥10% on a shipped 68000 lens, ST-4).
- **DEF**: DEF-612 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Major, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3): net +32.6% reproduced; patch 413 −23.8% of net, output identical.

### ASM-WS4-002 — `Array<T>` push and get cost 39% more instructions than equivalent C on the A500

- **Severity**: Major
- **Confidence**: verified
- **Lens**: A500 shipped -O2 (also A500 -Os +57.8%, A1200 shipped +38.8%, 68020 probe +64.9%)
- **Location**: `stdlib/std.mem.clo` `push`/`get` (lines 110-132 per the emission `#line`s) lowered to `cl_buffer_reserve` per push and a one-element view per get; `compiler/runtime/shim_hosted_amiga.c:803-818` (`cl_buffer_reserve`), at e24a8752
- **Checklist item**: WS4-01
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — Phase 0 matrix (A500 shipped -O2 (hosted-amiga-13, `-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`, `vamos` 68000), pinned `clc` sha256 `0c2fd66e…`, baseline e24a8752): `container` Clorraine 57,198 vs C reference 41,312 net instructions (+38.5%; raw 60,117 / 44,230). Attribution: `cl_buffer_reserve` 8,465 instructions (called on every one of 1,024 pushes), `memset` 1,028 (zeroing each grown region), `main` +5,987 (the view and its second check per element). Patches: 412 (`push` with an inline capacity test and a direct store) 60,117 → 46,960 (−23.0% of net); 411 (`get(i)[0]` as one check and an indexed read) → 56,045 (−7.1%); outputs identical.
- **Impact**: container-heavy code (lists of game objects, parsed records) pays about 40% more CPU on the 68000 than the equivalent C growable array.
- **Root cause**: `push` always calls the runtime reserve (which returns at once when the capacity suffices) and then builds a one-element view checked twice; `get` returns a view that the caller indexes with a second check.
- **Recommendation**: make the `Array` element access compiler-known: inline the capacity test with the reserve call on the slow path only, store directly, and lower `get(i)[0]` to one bounds check and an indexed read.
- **Effort**: M
- **Related**: WS4-09; DEF history of `Array` ownership (WS8); `plans/real-workload-audit/99-execution-plan.md` (concluded the `get`/`push` instances are inlined into `main` and left them unchanged; amortized reserve growth followed in `growable-array-growth`)
- **Self-refutation (4.3.1)**: the earlier audit compared shapes, not C; this new measurement (patches 411/412: −7.1% and −23.0% with identical output) contradicts its "no change" conclusion for `get`/`push`. The C reference grows by the same doubling from 8 bytes and keeps the spec-required check, so the gap is the per-push call, the zeroing `memset`, and the view. Stays Major.
- **DEF**: DEF-611 (`plans/00-roadmap.md`)
- **Verdict**: confirmed (Major, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3): net +38.5%; patches 412 −23.0% and 411 −7.1%.

### ASM-WS4-003 — The runtime fault writer pulls 16.6 KB of `printf` and soft-float code into every checked program

- **Severity**: Opportunity
- **Confidence**: verified
- **Lens**: A500 shipped -O2 (every Amiga lens links the same code)
- **Location**: `compiler/runtime/shim_hosted_amiga.c:544-549` (`cl_trap` uses `printf("error %s: %s\n", …)`), at e24a8752
- **Checklist item**: WS4-07
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `425-size.sh` (`records/WS4-size.log`, A500 -O2 without `-s`): the `sieve` binary retains `___vfprintf_total_size` 10,582 B, `___addxf3` 2,452 B, `___mulxf3` 1,124 B, `___cmpxf2` 776 B, `___xf_pack` 388 B, `___extenddfxf2` 288 B; `calibration` (no runtime check) retains none of them. Patch 417 (`probes/WS4/417-trap-no-printf.diff`) writes the same line with `fputs`/`fputc`: `.text` 23,772 → 7,120 B (−16,652 B, −70%), output identical, instructions +12.
- **Impact**: on a 512 KB A500 every program that indexes, divides, or converts a float (almost all) carries about 16 KB of formatted-output and long-double code it never uses outside a fault message; the same applies to the equivalent C written with `printf`.
- **Root cause**: the fault path formats with `printf`, which links libnix's full `vfprintf` including floating-point conversion.
- **Recommendation**: write the fault line with `fputs`/`fputc` (or `Write` to `Output()`), exactly as patch 417 does; check the other shim `printf`/`snprintf` users the same way.
- **Effort**: S
- **Related**: WS4-07; WS7-02 (A500 footprint)
- **Self-refutation (4.3.1)**: no earlier plan measured the fault writer's link cost. Not a defect and not a gap against the C reference (which also uses `printf`), so Opportunity; listed as a quick win (effort S, −70% `.text`).
- **Verdict**: confirmed (Opportunity, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3): patch 417 `.text` −16,652 B, +12 instructions.

### ASM-WS4-004 — A module-level table with a constant initializer occupies RAM twice

- **Severity**: Opportunity
- **Confidence**: verified
- **Lens**: A500 shipped -O2
- **Location**: module-value initializer lowering (zeroed `static` plus `static const __cl_ctmpl_<n>` and a start-up `memcpy` in `cl_init_<module>`), `compiler/src/codegen/program.rs`, at e24a8752
- **Checklist item**: WS4-06
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — patch 416 (`probes/WS4/416-lookup-static-init.diff`; `records/WS4-patches.log`, A500 shipped -O2 (hosted-amiga-13, `-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`, `vamos` 68000), pinned `clc` sha256 `0c2fd66e…`, baseline e24a8752): emitting `lookup`'s 4,096-entry table as an initialized static: `.bss` 16,504 → 120 B (−16,384), `.text` −24 B, instructions −7, output identical.
- **Impact**: each mutable module table with a constant initializer costs its size twice (the template in the code hunk and the zeroed copy in RAM), which matters for sine tables, palettes, and level data on a 512 KB A500.
- **Root cause**: module `let` initializers are lowered as run-time stores from a template even when every element is constant.
- **Recommendation**: emit a constant-initialized module `let` as an initialized static and drop the template and the start-up copy.
- **Effort**: S
- **Related**: WS7-02; `plans/constant-array-initializer` (introduced the template for module values)
- **Self-refutation (4.3.1)**: `plans/real-workload-audit` checked function-local tables only ("already the compact shape"); module values were not measured for RAM. Opportunity.
- **Verdict**: confirmed (Opportunity, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3): patch 416 `.bss` −16,384 B; a static initializer changes what a read before module init sees, so the fix needs an init-order argument.

### ASM-WS4-005 — Every Amiga program links libamiga DOS stubs and `CreatePort` it does not use

- **Severity**: Opportunity
- **Confidence**: verified
- **Lens**: A500 shipped -O2
- **Location**: references from the embedded shim (`shim_hosted_amiga.c`) to `CreateDir`, `CurrentDir`, `DateStamp`, `Delay`, `DeleteFile`, `Examine`, `Info`, `Lock`, `ParentDir`, `Rename`, `UnLock`, `CreatePort`, at e24a8752
- **Checklist item**: WS4-07
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `425-size.sh` with a link map (`build/tmp/assessment/WS4/size/calibration.clo.map`): the `calibration` link includes `libamiga.a(createdir.o … unlock.o)` stubs (20–28 B each) and `CreatePort.o` (188 B) although the program's own retained `.text` is 412 B; Clorraine `calibration` `.text` 5,900 B vs C 5,124 B (+776 B, +15%).
- **Impact**: a fixed ~0.8 KB per program; small, but it is the whole fixed overhead over C.
- **Root cause**: the archive members are extracted for references in shim functions that `--gc-sections` later removes, and the stub objects stay linked.
- **Recommendation**: move rarely used shim functions to the optional suffixes (as already done for 2.0+ services), or reference the DOS calls through the profile-specific suffix only when the module is imported.
- **Effort**: M
- **Related**: ASM-WS4-003
- **Self-refutation (4.3.1)**: `performance-levers-closeout` measured the GC gain (calibration 53,432 → 5,908 B on hosted-amiga-13) but did not attribute the remainder; consistent with this listing. Opportunity.
- **Verdict**: confirmed (Opportunity, verified), magnitude corrected — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3): the unused libamiga stubs and `CreatePort` are 440 B of the 776 B fixed `.text` gap, not the whole gap.

### ASM-WS4-006 — `-O3` and `-flto` beat the shipped release flags on the A500

- **Severity**: Opportunity
- **Confidence**: verified
- **Lens**: A500 (hosted-amiga-13, nix13, -m68000) and the 68020 probe build (labelled)
- **Location**: `compiler/src/clorraine/toolchain.rs:94-109` (shipped `-O2`/`-Os` lines), at e24a8752
- **Checklist item**: WS4-08
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `420-flags.py` (`records/WS4-flags.log`). Flag set A (shipped): `-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`. Flag set B: the same with `-O3`: net instructions container −18.3%, strings −25.1%, moves −98.5%, local-table −5.1%, others within ±1.4%, at +272 to +2,216 B `.text`. Flag set C: `-O2 -flto`: −280 to −512 B `.text` on every benchmark, container −7.3% instructions. `-fwrapv` removed: identical code (it costs nothing); `-fomit-frame-pointer`: identical (already implied).
- **Impact**: users get up to a quarter fewer instructions on string- and container-heavy code with `-O3`, or about 0.5 KB smaller binaries with LTO, without source changes.
- **Root cause**: the release line is fixed to `-O2`/`-Os` and does not offer `-O3` or LTO.
- **Recommendation**: evaluate an `-O3` release option (speed) and `-flto` for the size line; keep `-fwrapv` (free).
- **Effort**: S
- **Related**: WS7-03; `plans/performance-levers-closeout/00-ambiguity-register.md:146-156` (rejected `-O3` as "`.text` +14.9% on calibration; runtime equal; no win" and LTO+GC as "saves only 2–11% more than GC" with link-time risk)
- **Self-refutation (4.3.1)**: the earlier rejection of `-O3` rested on calibration size and an equal runtime; the new A500 instruction traces contradict "runtime equal" on three benchmarks (−18.3%, −25.1%, −98.5%), which 03-01 §Independence allows to reopen. The size cost and the LTO toolchain risk the earlier plan named remain real, so this stays an Opportunity, not a defect.
- **Verdict**: confirmed (Opportunity, verified) — verifier ses_dea9a6112ffeMR984F2rpPTplg (9.2.3): `-O3` −25.1% strings, −18.3% container; `-flto` −7.3% container; `moves` −98.5% not re-measured.

## Spec questions

None.

## Falsifiability log

Red check 2026-11-25 after `34400fe4` (log `build/tmp/assessment/WS4/red.log`).

| Probe | Form | Red-check result |
|-------|------|------------------|
| `400-attribute.py` | seeded known symbol: `400-seeded.c`, whose `static spin()` runs a 20,000-iteration loop | **first run failed the check**: 96.2% was attributed to `___exitmalloc`, because the Amiga hunk symbol table keeps only global symbols and `spin` is `static`. The script was amended to read function addresses from the link map (`.text.<name>` sections) and committed before re-running (`probes/WS4/400-attribute.py`); the re-run attributes 96.1% to `spin.constprop.0`. Every emitted Clorraine function is `static`, so the amendment is what makes WS4-02 possible |
| `430-patch-measure.py` | comparator with a seeded wrong expected output (`--seeded-mismatch`, patch 414) | reported `FAIL` with both outputs rejected, exit 1 |
| `420-flags.py` | comparator: the same `bench.normalize_output` equality whose rejection of a mutated expectation was shown in Phase 0 | form recorded |
| `410`–`417` diffs | each patch must keep the golden output; the comparator above shows the check fires | observed in task 4.2.2 |

## Phase 4 gate (4.3.2)

| Case | Result |
|------|--------|
| Task verify | drift check PASS |
| ST-1 | six findings, every template field, IDs `ASM-WS4-001..006` |
| ST-2 | each finding cites committed probes or the committed matrix record and the command |
| ST-3 | no `hypothesis` |
| ST-4 | ASM-WS4-001 and -002: both numbers, same lens and compiler, each side netted against its own calibration, the equivalent C reference, the shipped A500 -O2 lens, deltas +32.6% and +38.5%; ASM-WS4-006 states both flag sets |
| ST-5 | probes committed in `34400fe4`; the attribution fix and the size-listing fix committed before re-running; falsifiability log complete |
| ST-6 | no spec disagreement in WS4 |
| ST-7 | drift check PASS |
| ST-8 | WS4-01..WS4-10 each have one outcome |
| ST-11 | no Critical |
| ST-12 | records listed under §Checklist outcomes |
| ST-14 | patches 410–417: each output identical to the golden after normalization, committed as a diff, gain reported against the unpatched build with the same lens and flags |
| ST-15 | 68020 numbers labelled "68020 probe build"; none rated above Opportunity |
| ST-16 | dispatch record `high` |

## Escalations

None (no Critical in WS4).

## Probe index

| Item | Probes |
|------|--------|
| WS4-01 overhead table | Phase 0 matrix |
| WS4-02 attribution | `400-attribute.py`, `400-seeded.c` |
| WS4-03 guards | `410-sieve-range-guards.diff` |
| WS4-04 drop flags | `415-strings-static-drop.diff` |
| WS4-05 temporaries and copies | `411`, `412`, `413`, `414` diffs |
| WS4-06 static state | `416-lookup-static-init.diff` |
| WS4-07 size | `417-trap-no-printf.diff`, `425-size.sh` |
| WS4-08 flags | `420-flags.py` |
| runner | `430-patch-measure.py` |
