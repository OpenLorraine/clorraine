# Plan Preflight Report: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 00-preflight-report.md
> **Parent**: [Index](00-index.md)
> **Audit target**: the complete plan folder
> `plans/performance-levers-closeout/`
> **Context documents**: [`00-ambiguity-register.md`](00-ambiguity-register.md),
> [`01-requirements.md`](01-requirements.md),
> [`00-preflight-report-requirements.md`](00-preflight-report-requirements.md),
> `plans/00-roadmap.md`, `AGENTS.md`
> **Scope mode**: strict
> **Final target hashes**:
> - `00-index.md`: `d2ad694f2e5ed0d50e4532e5d1b1e9baf9cc18f4249ba3c4e780934bc3821fd7`
> - `01-requirements.md`: `a51b79cce0328bbbf2acd84bb608ad66daf6fea8bb3c2d6ef65a4b00376e3246`
> - `03-01-release-flags.md`: `f27fb7d146403b68eb89801d513f91b9b34bcc822f58b152b0631dddc9e5530b`
> - `03-02-measurement-and-gates.md`: `a0d0b7a0fceee309b3f104e6c54a99f4b0eca8a33f6ea6332e2e0823cb1c0e84`
> - `07-testing-strategy.md`: `b8a34020e7f35982c2dd70136c836daa7b77b63c48fdeeb6b8e1e9489dd085ca`
> - `99-execution-plan.md`: `691c2a484fa04c2cf4986cdefc25314d0ff00457a3b1f1468dbb0642339c847c`
> **Verdict**: ✅ **PASSED** — every critical/major finding is resolved; every
> minor finding was fixed or explicitly recorded.

> ⚠️ **Same-session review.** The plan was authored in this session. Two
> independent read-only auditors scanned it against the real code; a third
> re-scanned the major fixes. The user's only reserved decision (the frozen
> spec rows) was made during the requirements preflight and is recorded in
> AR-19.

## Codebase Context Summary

| Fact | Evidence |
|---|---|
| The driver owns the compile-matrix commands | `compiler/src/clorraine/toolchain.rs:36-78,144-157` |
| `Tool` is also constructed by two test doubles | `compiler/src/clorraine/build.rs:159-185` |
| The spec has eight matrix rows; two are GCC release | `spec/17-codegen.md:384-397` |
| The harness hard-codes release flags and records them | `tools/perf/bench.py:36-41,493-498`; compare `:625-630` |
| The shape gate keeps warning flags | `docker/scripts/bench-shape.sh:27-28` |
| `sdk-advanced-build` builds debug at the default paths | `Makefile:294-303` |
| The 1.3 portable corpus is four programs; `interrupt-counter` is FS-UAE-only | `docker/scripts/corpus-amiga.sh:36,167-169` |
| `file-copier` writes in the current directory | `spec/tests/corpus/file-copier/program.clo:12-13` |
| The base `f9d5be8` contains the whole PLAN-25 closeout | git history; `build/perf/head.json` |

## Scan 1 — Soundness, Grounding, Delivery, and Risk

### Major findings

| ID | Finding | Resolution |
|---|---|---|
| PP-001/PP-112 | Adding `Tool.release` breaks the two `build.rs` test doubles; no task included `build.rs`. | `02` now lists `build.rs`; `03-01` gives the `release: &["-O2"]` fields; task 2.1.1 includes it. |
| PP-002 | The audit JSON was keyed by program name, so calibration/sieve collided across profiles. | Key is `"<profile>/<program>"`; the example and data model use it. |
| PP-003 | The driver constants are release-only slices; the cross-check compared them to full harness vectors. | ST-11, 03-02, AR-22/AR-25 compare `base + release` compositions, including `-mcrt=nix13`. |
| PP-004/PP-110 | The `--release` help omitted the freestanding/non-Linux exclusions required by R6.1a. | The help line and `Options.release` doc name both exclusions; ST-22 checks them. |
| PP-101 | Phase 1 tests referenced APIs/modules that did not exist and the cargo command filtered the wrong package. | Added a behavior-neutral test-seam task (1.1.2); the red set uses `-p clc --lib toolchain::tests` and `--test clorraine_driver_spec_test`; 1.1.7 records the skeleton boundary. |
| PP-102 | The audit matrix had only old/new; R1.1/R1.5 require base/GC/GC+strip and the rejected candidates. | Added the `gc` variant and complete arrays; rejected candidates are measured once in Phase 1 and recorded; the repeat gate is old/GC/new. |
| PP-103 | Reviews ran only after the audit and measurement, so a finding would force rework. | Added end-of-Phase-2 (2.1.8) and end-of-Phase-3 (3.1.12) reviews; 4.1.7 is the final cross-check. |
| PP-104 | The 1.3 corpus would run from the repo root, dirtying the tree, and `bench.expected_output` does not read corpus outputs. | The audit uses a private per-program run directory and the corpus expected-output path; a clean-tree check is implied by the base identity. |
| PP-105 | One task added the whole audit script. | Split into skeleton/CLI (3.1.2), build/run (3.1.3), floors (3.1.4), and repeat/Makefile (3.1.5). |

### Minor and observational findings

| ID | Finding | Resolution |
|---|---|---|
| PP-005 | `spec/check.sh` does not inspect the matrix. | Added the ST-8 driver spec test that reads the table and pins all eight rows. |
| PP-006 | Corpus expected-output and cwd were unspecified. | Private run directory + `spec/tests/corpus/<name>/expected-output.txt`. |
| PP-007 | The FS-UAE citation was at `:36`, not `:167-169`; AR-20 overstated the probe coverage. | Citations corrected; AR-20 says the audit is the first full run. |
| PP-008 | The 1.3 repeat set was base/GC, while the shipped build is GC+strip; AR-09 omitted the 1.3 set. | Repeat set and AR-09 now use base/GC/GC+strip on the 1.3 programs. |
| PP-009 | `make bench-flags --check` passes `--check` to make. | All references use `make bench-flags`; the target hardcodes the script's `--check`. |
| PP-010 | Freestanding was "do not grow", which permits a shrink. | The shipped pair must be exactly equal; the separate GC probe records the 208→232 growth. |
| PP-011 | The freestanding spec row says "same line with -O2", which retains warnings, while the driver omits them. | Recorded as a pre-existing discrepancy; only the two authorized rows change. |
| PP-012 | AR-03 said the post-feature commits were only docs/tests; `fd7fe30` is a codegen fix. | AR-03 now names the whole PLAN-25 closeout and the branch head. |
| PP-013 | AR-22 cited a nonexistent PF-106. | Replaced with PF-020. |
| PP-014 | ST-21 (`nm`/`readelf`) was mapped to the synthetic harness tests. | Moved to the real `make bench-flags` gate; ST-21 now uses `readelf -S` and fails on a missing tool. |
| PP-015 | "Exit on the first failed group" contradicted "list every miss". | The audit checks every group, prints every miss, writes JSON, then exits 1. |
| PP-016 | Several line citations were one row/range short. | Citations tightened (`:384-397`, `:27-28`, `:60-78`, `artifacts.rs:32-34,55-62`). |
| PP-017 | ST-10 conflated `measure()` with the document's calibration summary. | R4.2/ST-10 distinguish the full `measure()` record from the summary. |
| PP-018 | The requirements status was still Draft and the roadmap stage not advanced. | Requirements status Approved; the plan preflight advances the row to Plan Preflighted. |
| PP-019 | `binary_bytes` raised `FileNotFoundError`; the audit JSON had no toolchain identity. | It wraps `HarnessError`; the JSON includes `toolchain`. |
| PP-020 | `host_has_gnu_linker` was named for a GNU probe but checks Linux. | Renamed `host_is_linux()`; the pure `host_release_flags(bool)` is the test seam. |
| PP-021 | `dirty` was described as base-commit provenance. | It is a harness-tree cleanliness check; the full hash is the base identity. |
| PP-022 | Command-vector unit tests were labeled specification tests. | Labeled unit/implementation tests; ST-8/ST-22 are the driver spec tests. |
| PP-106 | `make bench-flags` was missing from R4.7 and the findings gate evidence. | Added to R4.7, the acceptance summary, task 4.1.10, and `05-findings`. |
| PP-107 | The execution plan did not carry the floor/rerun/KEEP contingency. | Added the Contingency section with AR-21 and AR-13. |
| PP-108 | ST-16 omitted the 1.3 floor; ST-13 omitted freestanding growth. | Both added. |
| PP-109 | Real repeat/symbol checks were mapped to fast unit suites. | Moved to the real audit gate; the synthetic tests cover the logic. |
| PP-111 | The audit JSON had no toolchain identity. | Added `toolchain`. |
| PP-113 | `f9d5be8` was called "v1.1.1-era". | Called the post-PLAN-25 `clc 1.1.1` compiler. |
| PP-114 | Stage transitions were not explicit. | Requirements Approved; the plan preflight sets Plan Preflighted; the exec-plan sets Executing/Done. |
| PP-115 | Floor arithmetic was not specified. | Exact integer cross-multiplication with boundary tests. |
| PP-116 | No runtime budget for the repeat set. | Default 2 runs; `--repeat 1` for development; the full gate cost is documented. |
| PP-117 | The debug-copy directory may not exist; the copies have no consumer. | `mkdir -p build/amiga-sdk`; the copies are documented developer artifacts; the warning compile is the point. |
| PP-118 | No explicit constructor/safety-helper check in the review packet. | Added to R7.2; the no-memory-safety-review exclusion stays justified. |
| PP-119 | `make verify` repeats some standalone legs. | Accepted; the verify log is the record for the duplicates. |

## Re-scan — Fix Verification

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| RS2-001 | 🟠 MAJOR | The freestanding GC probe was set to `-O2`, so it could not reproduce the 208→232 growth. | The matrix's GC column is the GC flags; the shipped `new` stays `-O2` and must equal the old pair; 1.1.8 and 3.1.10 record the growth. |
| RS2-002 | 🟠 MAJOR | ST-21 had no implementation in the audit. | `--check` now runs `readelf -S` on every scored host release binary and fails if `.symtab` is present or `readelf` is missing; task 3.1.10 includes it. |
| RS2-003 | 🟠 MAJOR | The "remaining init-list entries still run" claim had no proof method. | 03-02 now specifies `nm`/`objdump` plus a `vamos` trace mapped through the HUNK load base; 1.1.8/3.1.10 record it, and ST-15 asserts it. |
| RS2-004 | 🟡 MINOR | The dependency block used stale task numbers. | Updated to 3.1.8/3.1.10/3.1.11. |
| RS2-005 | 🟡 MINOR | The roadmap note still said 38 tasks. | Now 44. |
| RS2-006 | 🟡 MINOR | The acceptance summary and success criteria omitted `bench-flags`/`bench-base`/`bench`. | Added. |
| RS2-007 | 🟡 MINOR | The audit flag arrays were ambiguous (release-only vs full command). | Defined as complete compiler arguments before `-o`, with a separate `libs` field; the cross-check uses the composition. |

## Final Verification

- All nine distinct critical/major root causes are resolved in the owning
  documents and traced into tasks.
- Every minor and observational finding is fixed or explicitly accepted
  (PP-119 is the only no-change acceptance).
- The plan has 44 tasks in 5 phases, with specification tests before
  implementation, phase-end reviews before the scored measurement, and a
  complete acceptance/closeout path.
- The requirements, design, testing, execution, register, and roadmap agree on
  the base, the flags, the spec scope, the floors, the release-path gates, and
  the four deferred issues.
- No new critical/major issue was introduced by the fixes.

**Verdict: ✅ PREFLIGHT PASSED — 9 distinct critical/major findings and 32
minor/observational findings resolved; 1 accepted observation; 0 open.**
