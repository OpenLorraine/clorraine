# WS8 — Test integrity and defect history

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-03-runtime-performance-workstreams.md §WS8 (WS8-01..WS8-08)

## Scope and limits

Static analysis only: no test is run or edited (03-03 WS8-03). Inputs: the 157 Rust test
files (3,888 `#[test]` functions) under `compiler/tests/`, the fuzzer templates
(`tools/fuzz/`), the sanitizer runners (`make asan`, `make ubsan`), the FS-UAE gates,
the 582 DEF rows of `plans/00-roadmap.md`, and the WS1–WS7 findings. Classification is
by the committed schemes (probes 800, 810); every class used in a finding is read by
hand and the scheme's error rate on the read rows is reported.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | high | 2026-11-26 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Schemes written (8.1.1) | `probes/WS8/800`, `810` | done |
| Red check (8.1.2) | two tests, two DEF rows | done |
| Coverage and gaps (8.2.1) | 15 areas × 3,888 tests; 46 findings of Minor or above | done |
| Assertions, fuzzer, sanitizers, pins (8.2.2) | 800 census and sample; `tools/fuzz/fuzz.py`; `make asan`/`make ubsan` suites; text pins | done |
| DEF history (8.2.3) | 582 rows (810), 50 read by hand | done |
| Root causes (8.2.4) | four recurring classes | done |
| Self-refutation (8.3.1) | every root-cause claim against the code at e24a8752 | done |

## Checklist outcomes

Records: `records/WS8-assertions.tsv` (census and the 75-test sample), `records/WS8-defs.tsv`
(582 classified DEF rows). Commands: `python3 plans/independent-assessment/probes/WS8/800-assertion-scheme.py census|sample`, `python3 plans/independent-assessment/probes/WS8/810-def-scheme.py --rows`.

| ID | Outcome | Evidence |
|----|---------|----------|
| WS8-01 | ASM-WS8-001, ASM-WS8-004 | Tests per area (name match; classes value/diagnostic/text/sanitizer/compiles): lexical 328, evaluation order 250, constants 210, profiles 63, ownership 547, borrows 318, generics 364, closures 206, C validity 1,210, arithmetic 74, names 35, determinism 6, runtime faults 166, stdlib 577, performance 75. Goldens: 114 compile-fail, 6 run, 3 staged, 7 corpus (Phase 0). Fuzzer: 6 single-`main` generators (narrow arithmetic, local fills, enums, float constants, shifts, a struct loop) and 12 seeds. Sanitizers: 38 suites under ASan, 12 under UBSan; no Amiga shim under either. Thin areas: names (35, 12 unrecognised `contains` checks), determinism (6), arithmetic UB (74, 3 sanitizer-only), and every Amiga shim fault and edge path (FS-UAE gates assert report lines and captures only) |
| WS8-02 | gap table below | One reason per finding of Minor or above |
| WS8-03 | ASM-WS8-002 | Census of 3,888 tests: value 1,400, diagnostic 810, text 580, sanitizer-only 282, compiles-only 192, no-panic-only 34, unrecognised 590 (read: generic `assert!(…contains…)` checks — text, stderr, or determinism). Spec suites (2,573): value 1,108, diagnostic 652, sanitizer-only 282, compiles-only 187. Five-per-area sample (75): value 27, diagnostic 22, text 9, other 14 (read by hand: 9 text pins on emitted C or scripts, 3 stderr-message checks, 2 determinism comparisons) |
| WS8-04 | ASM-WS8-001 | The generators never produce a second function or a call, an owned `String`/`Array`/`Box`, a move or drop, a borrow, a generic, a closure, nesting, a module-level array, or an unusual identifier; the oracle compares printed output across host, ASan/UBSan, and m68k only |
| WS8-05 | ASM-WS8-004 | Never under a sanitizer: `shim_hosted_amiga*.c` (10,404 lines) and every m68k-only path; substitute evidence: `vamos` corpus and fault runs (output and exit status), FS-UAE gates (report lines and captures), the WS5 leak detector (this assessment only) |
| WS8-06 | `clean` | Text-pinning tests: 580 (15%), of which 569 are impl tests (43% of impl tests) — the repository policy lets an impl pin change with an intended emission change; the 11 spec text pins (`stage_spec_test.rs`) check shapes spec/17 mandates (cleanup ladder, ABI prototypes, `#line`, init order). The WS4 remedies would update `cl_string_concat` pins (11 in 5 template impl tests) and bounds-guard pins (`cl_trap("E11211"`, 35) — maintenance, not a block |
| WS8-07 | ASM-WS8-003 | 582 rows: memory-safety 163 (38 plans), invalid-C 118 (26), false-accept 100 (26), miscompile 51 (22), false-reject 5 (3), platform 5 (5), other 140 (read: process and tooling rows DEF-562..576, plus defects the patterns miss — DEF-04, DEF-44, DEF-52, DEF-86). Every class but `other` is recurring by the 3-rows/2-plans rule. Components: ownership 353, codegen 144, sema 47. By month: September 31 rows, October 527 (the closeout and residue series: `def-zero-closeout` 40, `conditional-projection-residue` 35, `def-critical-major-closeout` 20, `non-lexical-lifetimes` 18, …), November 24 (process and SDK). Scheme accuracy on 25 hand-read classified rows: 20 correct (80%); the misses are loan/escape gaps filed as memory-safety (DEF-373, DEF-391, DEF-550) and two notes-driven matches (DEF-378, DEF-408) |
| WS8-08 | ASM-WS8-003 | Root causes per recurring class, presence at the baseline, and remedies in the finding |

### Gap linkage (WS8-02)

Reasons: **A** no test of that shape; **B** tests assert a weaker property; **C** a golden
pins the defect; **D** the path never runs under test (Amiga shim, FS-UAE-only, fault
path); **E** documented or staged out; **F** sizes in tests are small; **G** the leg that
shows it (host UBSan, vbcc) is not tested for that shape.

| Finding | Reason |
|---------|--------|
| ASM-WS1-001 | A — compile-fail goldens cover annotated contexts (E10202); no unannotated out-of-range literal |
| ASM-WS1-002 | A — left-to-right tests pin syntactic updates inside arguments, not two effectful calls; the fuzzer has no calls |
| ASM-WS1-003 | E — the E10214 golden is excluded as staged (`compile_fail_spec_test.rs:51`) |
| ASM-WS1-004 | F — no deep-nesting generator; `compile_time.py` shapes are flat |
| ASM-WS1-005 | A — E10305 tests use inferred type arguments |
| ASM-WS2-001 | B — "releases when the statement ends" tests check leak-freedom and exit code, not when the drop runs |
| ASM-WS2-002 | A — E10026 tests move fields of locals, not of call results |
| ASM-WS2-003 | — structural; tests are per shape by design |
| ASM-WS3-001 | A — the keyword test pins `int` (`codegen_ownership_impl_test.rs::a_borrow_element_may_spell_a_c_keyword`); no C99 keyword or header macro |
| ASM-WS3-002 | A — collision tests cover shim names and helper prefixes, not generic instance keys |
| ASM-WS3-003 | G — UBSan suites do not reach a value-position `++` at the limit; m68k GCC builds with `-fwrapv` |
| ASM-WS3-004 | A, G — no host test of a `usize` pointer round trip |
| ASM-WS3-005 | B — strict-build tests check pass or fail, not where a diagnostic points |
| ASM-WS3-006 | A — `line_directives_name_the_logical_cli_path` passes a relative path only |
| ASM-WS3-007 | F — vbcc legs build small units |
| ASM-WS4-001, -002 | B — `bench-compare` guards regressions against the base, not the gap to C |
| ASM-WS5-001 | B, D — `run-fault-amiga` checks the message and exit status, not system state after exit |
| ASM-WS5-002 | D — every gate opens a screen or window before using Intuition |
| ASM-WS5-003 | D — `setCurrentDir` runs once in the imageviewer gate; nothing checks the launcher's lock |
| ASM-WS5-004 | A — no off-screen text test |
| ASM-WS5-005 | A — no fault inside a worker |
| ASM-WS5-006 | A — no free of an installed clip region or a closed guide's message |
| ASM-WS5-007 | B — the services demo calls `cardPresent` once |
| ASM-WS5-008 | C — the A1200 Workbench golden was captured by the defective code |
| ASM-WS5-009 | D — interrupt-counter checks its own ticks, not later servers |
| ASM-WS5-010 | B — the GUI capture is the bitplanes only (no sprites) |
| ASM-WS5-011 | D — `vamos` does not truncate a BPTR's low bits |
| ASM-WS5-012 | B — the A600 gate counts colours in the capture |
| ASM-WS5-013 | D — no gate checks open counts; `vamos` does not model the ROM libraries |
| ASM-WS5-014 | — the shipped A500 lens has no expansion memory |
| ASM-WS5-015 | B — the gate asserts broker creation |
| ASM-WS5-016 | B — the clipboard gate writes, then reads (the order that works) |
| ASM-WS5-017 | D — timing-dependent; captures follow `WaitTOF` |
| ASM-WS5-018 | B — the GUI gate drives menus by Amiga-key shortcuts |
| ASM-WS5-019..021 | A, D — per item |
| ASM-WS6-001 | A — no IO-error test |
| ASM-WS6-002 | A — the appendix row for `slice` of a `str` has no test |
| ASM-WS6-003 | A — float tests compare finite values |
| ASM-WS6-004, -005 | E — nothing cross-checks the appendix against the exports |
| ASM-WS7-001 | F — large fills in tests are function locals (`amiga_ilbm_spec_test.rs:826`, `stage_spec_test.rs:2517`) |
| ASM-WS7-002 | F — `compile_time.py` shapes have few calls per body and no generics |

## Findings

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
- **Self-refutation (8.3.1)**: the generator list is read from the code at the baseline. Minor (test debt).
- **Verdict**: _(Phase 9)_

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
- **Self-refutation (8.3.1)**: sanitizer-only and compiles-only tests are useful as crash guards; the finding is that some of them are the only test for a behavior. Minor.
- **Verdict**: _(Phase 9)_

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
- **Self-refutation (8.3.1)**: checked against the code at e24a8752 — `codegen/drop.rs:283` `needs_drop` re-derives from `TypeTable::duplicable`, not from an ownership record; the emitter builds C with `format!` (709 sites); E10305 is reported only on the inference route (`sema/call.rs:792`, `:1325`), which is why explicit arguments bypass it; `own/call.rs` sequences arguments only when they contain a syntactic update. Rated Minor by the rubric (maintainability debt that has caused recorded DEF rows); it is the report's main structural recommendation.
- **Verdict**: _(Phase 9)_

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
- **Self-refutation (8.3.1)**: the core runtime already exists twice (host and Amiga) and the host copy is sanitized; the gap is the Amiga-only code. Minor.
- **Verdict**: _(Phase 9)_

## Spec questions

None.

## Falsifiability log

Red-check targets, chosen by reading before the classifiers run (task 8.1.1):

- 800 `value`: `amiga_ilbm_spec_test.rs::an_uncompressed_ilbm_decodes_every_pixel`
  (compares the decoded pixel report through `assert_stdout`).
- 800 `compiles`: `stage_spec_test.rs::lexical_literals_and_templates_are_accepted`
  (its only check is `assert_front_end_accepts`; the program returns 0).
- 810: DEF-13 (memory-safety) and DEF-22 (invalid-c).

Results (task 8.1.2, 2026-11-26): 800 classified the two tests `value` and `compiles`;
810 classified DEF-13 `memory-safety` and DEF-22 `invalid-c` — both schemes separate
the hand-picked pairs. (810's component for DEF-22 came out `ownership` from words in
its notes; components are read by hand before use.)

## Self-refutation (8.3.1)

Each root-cause claim was checked against the code at e24a8752 (see ASM-WS8-003's
self-refutation line). One earlier finding was corrected while building the gap table:
ASM-WS3-001's evidence named `char`/`int`/`long` as failing names, but the emitted C of
probe 340 escapes every C89 keyword; the failing names are `restrict`, `_Bool`, and
header/NDK macros. The finding stays Critical; its evidence, impact, and root cause are
corrected in `WS3-codegen.md`, and the correction is logged under its escalations.

## Phase 8 gate (8.3.2)

| Case | Result |
|------|--------|
| Task verify | drift check PASS |
| ST-1 | four findings, every template field, IDs `ASM-WS8-001..004` |
| ST-2 | verified findings cite the 800/810 records with commands, or code locations measured by grep |
| ST-3 | no `hypothesis` |
| ST-4 | no performance finding |
| ST-5 | schemes and red-check targets committed in `792c778e` before either classifier ran; red check logged |
| ST-6 | no spec disagreement |
| ST-7 | drift check PASS |
| ST-8 | WS8-01..WS8-08 each have one outcome; every coverage-table row complete |
| ST-11 | no Critical |
| ST-12 | counts cite `records/WS8-assertions.tsv` and `records/WS8-defs.tsv` |
| ST-14, ST-15 | not applicable |
| ST-16 | dispatch record `high` |

## Escalations

None yet.

## Probe index

| Item | Probes |
|------|--------|
| WS8-03, WS8-06 assertion strength, pinned text | 800 |
| WS8-07 defect history | 810 |
