# Verification packet 9.2.5 — WS7, WS8

## Scope

- Whole components at the baseline `e24a8752b08827c3a5d7c77b6e2827dac708b629`; pre-existing defects are in scope.
- Finding IDs use the `ASM-WS<n>-<NNN>` scheme.
- Read-only: do not edit any tracked file; write scratch output only under `build/tmp/assessment/verify/`.
- Before each reproduction run the drift check: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment e24a8752b08827c3a5d7c77b6e2827dac708b629` (it must pass). Builds and runs go through the SDK container: `make -C docker exec CMD="…"` (no nested quotes in CMD). The pinned compiler is `build/tmp/assessment/baseline/clc` (release; `clc-debug` alongside).
- Judge each finding from its text, the committed probes, the code, and `spec/`. Do not rely on the workstream files' other sections; they are the author's reasoning.

## Checklist briefs

- `plans/independent-assessment/03-03-runtime-performance-workstreams.md (§WS7 checklist)`
- `.opencode/agents/correctness-reviewer.md`

## Protocol

For every finding, the verifier:

1. Re-runs the reproduction or measurement from the committed sources with the drift check
   passing; for every `probable` Major or Critical it attempts a reproduction.
2. Tries to refute it: checks `spec/`, `docs/known-limitations.md`, earlier plans, and
   whether the observed behavior is intended.
3. Checks the severity against §Severity and the confidence against §Confidence.
4. Checks for duplicates across workstreams and merges them under the lowest ID.
5. Records one verdict: `confirmed`, `refuted`, `downgraded <from→to>`, `upgraded <from→to>`,
   or `merged into <ID>`, with a one-line reason.
6. Escalates any verdict that results in a Critical, per §Escalation.

The verifier also re-checks at least two `clean` outcomes per workstream, preferring items on
the A500 lens and memory-safety checklist items (WS2, WS5-03, WS6-05), and records a verdict
for each. Findings raised by a verifier are
added to the workstream file and receive their verdict from a different verification session.
Refuted findings stay in `05-findings.md` with their verdict; they never reach the report's
ranked list.

## Severity

| Severity | Definition | Ref |
|----------|------------|-----|
| 🔴 Critical | An accepted program miscompiles, is memory-unsafe, or emits invalid C; a crash or hang on a real Kickstart; data loss | AR #15 |
| 🟠 Major | Latent undefined behavior in emitted C; a shim leak or missing failure path; a wrongly rejected valid program; ≥10% net m68k instructions or `.text` against the equivalent C reference on a shipped 68000 build (§Measurement rules) | AR #15, PF-009 |
| 🟡 Minor | Edge-case divergence with low impact; maintainability debt that has already caused a recorded DEF | AR #15, PF-020 |
| 🟢 Opportunity | A measured improvement with no defect behind it below the Major threshold, or any gap measured only on a 68020 probe build | AR #15, PF-009 |

A `hypothesis` is never above Minor. A `clc` crash or hang on any input is Critical; a compile
time or peak RSS above 10× the Phase 0 `bench-compile` value for the same input size is Major
(PF-020). For emitted C (WS3-11), a C-compiler crash on valid emitted C, or a C compile time
above 10× the Phase 0 C compile time of the nearest-size emitted program, is Major (PB-005).

### Confidence

`verified` is only this confidence label; it never means "passed Phase 9" (PF-002).

| Label | Meaning |
|-------|---------|
| `verified` | Reproduced from committed probe sources at the baseline (drift check passing), or measured |
| `probable` | Strong code-reading evidence, no reproduction yet |
| `hypothesis` | Plausible from reading; never ranked above Minor (AR #15) |

A finding is **surviving** when its Phase 9 verdict is `confirmed`, `downgraded`, or
`upgraded`. Only surviving findings reach the report and the DEF ledger (AR #13 as clarified
by PF-002).



## Output

Write `build/tmp/assessment/verify/9.2.5.md` and return its content: one row per finding `| ID | verdict | reason (one line) | what you re-ran and observed |`, one row per sampled clean outcome with a verdict (`confirmed` or `refuted` + reason), any duplicate you would merge, and any new finding you raise in the finding template (it will be verified by a different session).

## Findings to verify

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

### ASM-WS7-002 — Compile time grows faster than quadratically for call-heavy and generic code

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

### ASM-WS8-001 — The fuzzer cannot generate the constructs where the defects are

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: all
- **Location**: `tools/fuzz/fuzz.py:155-283` (`GENERATORS = [_narrow, _fill, _enum, _float, _shift, _struct_loop]`), at e24a8752
- **Checklist item**: WS8-04, WS8-01
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading of the six generators (each emits one `main`), measured against the DEF history (`records/WS8-defs.tsv`): 353 of 582 rows are in the ownership component and 144 in codegen, while no generator produces a call with arguments, an owned value, a move or drop, a borrow, a generic, a closure, a module-level array, or a chosen identifier. Four verified Criticals and Majors of this assessment are in reach of such generators: ASM-WS1-002 (two effectful arguments), ASM-WS3-001 (identifier choice), ASM-WS7-001 (module-level fill), ASM-WS2-001 (drop timing printed by a deleter).
- **Impact**: the differential fuzzer guards arithmetic and layout, where few defects were found; the classes that recur (memory safety, invalid C, miscompile) are outside its reach.
- **Root cause**: the generators were written for the arithmetic and fill fixes they guarded.
- **Recommendation**: add typed generators for calls with effectful arguments (print order), owned values with instrumented deleters (drop order), module-level data, identifiers from a C keyword/macro list, generics and closures; run them under ASan/UBSan as the seeds already are.
- **Effort**: M
- **Related**: ASM-WS8-003

### ASM-WS8-002 — Tests and gates assert weaker properties than the behavior they cover

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: all; A500/A1200 gates
- **Location**: the suites and gates named in the gap table (B and C rows): `docker/scripts/run-fault-amiga.sh`, `docs/amiga-sdk/a1200-workbench.png`, `docker/scripts/a600-addons-smoke.sh:235-236`, the conditional-projection "statement end" tests, at e24a8752
- **Checklist item**: WS8-03, WS8-02
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: The 800 census (`records/WS8-assertions.tsv`): 282 spec tests check only a clean sanitizer run and 187 only acceptance or a zero exit; 11 gap-table rows are reason B and one is C. Concretely: drop-timing tests assert leak-freedom, not order (ASM-WS2-001); the fault gate asserts the message, not the system state (ASM-WS5-001); a golden made by the defective capture (ASM-WS5-008); a colour count stands in for the gadget list (ASM-WS5-012).
- **Impact**: a defect that keeps memory clean and the exit status right passes; a self-captured golden turns a defect into the expected value.
- **Root cause**: tests were written to close a specific crash or leak, so they assert its absence rather than the behavior.
- **Recommendation**: assert the observable behavior (deleter print order, the walker's system state after a fault, colours computed from the known palette, the gadget count from the window's list); regenerate goldens only from an independent oracle.
- **Effort**: M
- **Related**: ASM-WS5-008, ASM-WS2-001, ASM-WS5-001

### ASM-WS8-003 — Memory-safety and invalid-C defects recur because drop and emission decisions are made per expression shape, in two stages

- **Severity**: Minor
- **Confidence**: verified (the counts); `probable` (the structural cause)
- **Lens**: all
- **Location**: `compiler/src/own/` (137 `duplicable`/`maybe_owns` decisions) and `compiler/src/codegen/` (87 `needs_drop`/`duplicable` decisions re-derived, 709 `format!`-built C fragments), at e24a8752
- **Checklist item**: WS8-07, WS8-08
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement (810, `records/WS8-defs.tsv`): memory-safety 163 rows across 38 plans, invalid-C 118 across 26, false-accept 100 across 26, miscompile 51 across 22; 527 of 582 rows in October 2026, almost all in closeout, residue, and integrity plans that each fixed one more shape (projection roots, conditional roots, call results, view arrays). Still present at the baseline: ASM-WS2-003 (about 240 decision sites in two stages), new invalid-C instances ASM-WS3-001 and -002, new miscompiles ASM-WS1-002 and ASM-WS2-001, the emission blow-up ASM-WS7-001, and false accepts ASM-WS1-005 and ASM-WS2-002.
- **Impact**: each new expression shape needs its own move, drop, store, and emission path, so defects arrive shape by shape; closeout plans reduce the count without changing the rate at which new shapes fail.
- **Root cause**: (1) memory safety — the drop obligation of a value is decided in the ownership pass and decided again in codegen with a different predicate; (2) invalid C — C text is composed per shape from strings, with no typed C representation that would make an rvalue address, a missing parenthesis, or an unescaped name unrepresentable; (3) false accept — checks are attached to particular syntactic routes (inferred versus explicit type arguments, a local versus a call-result field) instead of a normalized form; (4) miscompile — effects are left inside C expressions whose order C leaves unspecified, instead of being sequenced into temporaries.
- **Recommendation**: one structural change addresses all four: lower to an A-normal-form IR (every effectful subexpression bound to a temporary in source order) before ownership checking; let the ownership pass record every move and drop in the IR and have codegen emit only recorded drops; emit through a small typed C syntax tree and printer that escapes names and parenthesizes by construction; run the semantic checks on the normalized form so every spelling reaches the same check. Then extend the fuzzer (ASM-WS8-001) to the shapes the IR must normalize.
- **Effort**: XL
- **Related**: ASM-WS2-003, ASM-WS8-001; the `non-lexical-lifetimes` and closeout plans

### ASM-WS8-004 — The Amiga shims never run under a sanitizer, and UBSan covers a third of the ASan suites

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: host (sanitizer legs); A500/A1200 (the unsanitized shims)
- **Location**: `compiler/runtime/shim_hosted_amiga*.c` (10,404 lines); the `RDC_ASAN` (38 suites) and `RDC_UBSAN` (12 suites) markers in `compiler/tests/`, at e24a8752
- **Checklist item**: WS8-05
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `grep -l RDC_ASAN compiler/tests/*.rs | wc -l` = 38, `grep -l RDC_UBSAN …` = 12; AGENTS.md: the optional C shim "is m68k-only, so it is not covered by host AddressSanitizer". The substitute evidence checks output only (`vamos` corpus and fault runs, FS-UAE report lines); WS5's leak detector and walker were written for this assessment.
- **Impact**: the 22 WS5 findings were invisible to the existing checks; the host-leg UB of ASM-WS3-003 and ASM-WS5-021 sits in the UBSan gap.
- **Root cause**: the shims depend on AmigaOS and cannot link on the host.
- **Recommendation**: keep the `vamos --vamos-leaks` detector and the FS-UAE system-state walker as standing gates; extract the OS-independent shim parts (buffers, strings, formatting, IFF, colour decoding) into a host-testable file under ASan/UBSan; run every ASan suite under UBSan as well.
- **Effort**: M
- **Related**: ASM-WS5-001, ASM-WS5-013

## Clean outcomes to sample

From `findings/WS7-performance.md` §Checklist outcomes:

| ID | Outcome | Evidence |
|----|---------|----------|
| WS7-04 | `clean` (with ASM-WS5-014 cited) | Chip placement: bitplanes through `AllocRaster` (`shim_hosted_amiga.c:3807-3830`), sprite images (`:4806`, `:4808`, `:5035`), audio samples (`:5511`), the pointer image (`:2673`), user `ChipBuffer`s (`:3700`), copper lists through graphics.library; descriptors and handles are `MEMF_ANY` (46 other `AllocMem` sites), and the C heap is libnix `malloc` (any memory). Nothing that needs no DMA is in chip memory, so no A1200 slowdown from placement; the reverse case is the 1.3 device buffer (ASM-WS5-014) |
| WS7-07 | `clean` | Peak RSS grows linearly (exponents 0.85–1.02): at 8,000 the series use 52–490 MB; `ownership` costs about 61 KB per function (490 MB), below 10× the Phase 0 `functions-8000` 82 MB (820 MB). The samples show per-call `Vec<Ty>` clones (`sample-gen.log`), a constant-factor cost, not growth |

From `findings/WS8-test-integrity.md` §Checklist outcomes:

| ID | Outcome | Evidence |
|----|---------|----------|
| WS8-06 | `clean` | Text-pinning tests: 580 (15%), of which 569 are impl tests (43% of impl tests) — the repository policy lets an impl pin change with an intended emission change; the 11 spec text pins (`stage_spec_test.rs`) check shapes spec/17 mandates (cleanup ladder, ABI prototypes, `#line`, init order). The WS4 remedies would update `cl_string_concat` pins (11 in 5 template impl tests) and bounds-guard pins (`cl_trap("E11211"`, 35) — maintenance, not a block |

