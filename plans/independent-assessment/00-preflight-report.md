## Preflight Report: Independent Assessment of Clorraine

> **Status**: ✅ PREFLIGHT PASSED — all 43 findings resolved (32 iteration 1, 10 iteration 2, 1 iteration 3)
> **Iteration**: 3 (bounded fix verification after iteration 2)
> **Artifact**: implementation plan (read-only assessment) at `plans/independent-assessment/`, commit `2eff956c`
> **Codebase Grounded**: ~45 source/config/script files examined, ~120 references verified
> **Independence**: uniform-effort — five fresh-context auditor sessions (one per dimension cluster) plus one hardening challenger; same model family as the author
> Effort: high→high (exact)
> **Last Updated**: 2026-11-25

> ⚠️ SAME-SESSION REVIEW: the lead session also authored the plan. The scan itself ran in five
> fresh-context sessions and the major recommendations were challenged by a sixth; the lead only
> merged and reconciled.

### Codebase Context Summary

**Tech Stack:** std-only Rust workspace (`compiler/`), C99 runtime shims, Python bench/fuzz harnesses, Docker SDK image (m68k GCC, vbcc, `vamos` from amitools git HEAD, FS-UAE).
**Architecture:** `clc` pipeline `lex → parse → load → resolve → sema → own → ir → codegen`, orchestrated by `analysis.rs`/`driver.rs`; no IR optimizer; release flags in `clorraine/toolchain.rs`; shims embedded per profile (`profile.rs:131-146`).
**Key Files Examined:** `compiler/src/clorraine/toolchain.rs`, `compiler/src/profile.rs`, `compiler/src/analysis.rs`, `compiler/runtime/*`, `tools/perf/bench.py`, `tools/perf/compile_time.py`, `tools/perf/flag_audit.py`, `benchmarks/*`, `build/perf/head.json`, `docker/Dockerfile`, `docker/scripts/{run.sh,a500-smoke.sh,asan-suite.sh,ubsan-suite.sh,corpus-amiga.sh}`, `docker/compose.yaml`, `Makefile`, `.opencode/agents/*`, `.opencode/skills/{exec-plan,git-commit}/*`, `plans/00-roadmap.md`, `docs/known-limitations.md`, `spec/` chapters 05–07, 12, 15.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|------------------|
| 1 | Ambiguities | 4 | 🟠 |
| 2 | Implicit Assumptions | 5 | 🟠 |
| 3 | Logical Contradictions | 4 | 🟠 |
| 4 | Completeness Gaps | 2 | 🟡 |
| 5 | Dependency Issues | 1 | 🟠 |
| 6 | Feasibility Concerns | 3 | 🟠 |
| 7 | Testability | 4 | 🟠 |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 2 | 🟡 |
| 10 | Scope Creep Indicators | 1 | 🟡 |
| 11 | Ordering & Sequencing | 4 | 🟠 |
| 12 | Consistency | 1 | 🟡 |
| 13 | Codebase Alignment | 1 | 🟡 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 12 | 12 resolved |
| MINOR | 19 | 19 resolved |
| OBSERVATION | 1 | 1 resolved |

---

## 🟠 MAJOR

### PF-001: Phase-gate ST lists disagree across documents; ST-11 missing at Phases 4, 7, 8 🟠 MAJOR
**Dimension:** 3 Logical Contradictions · **Location:** `07` line 10 and §Specification Tests table; every `99` Verify (gate) line; `03-02`/`03-03` §Testing Requirements
**The Problem:** 07 line 10 says every ST case is checked at each gate; its own table lists subsets; 99 lists different subsets again and drops ST-11 (critical escalation) at Phases 4, 7, 8, where Criticals can still arise; 03-03 cites ST-13, which applies to Phase 0 only.
**Options:** A) the 07 table is the sole authority with one row per phase, non-applicable cases pass as `n/a`, other docs point to the row · B) per-phase lists maintained in 99 and 07 both.
**Recommendation:** A — single source prevents the drift; ST-11 in every Phase 1–8 row; ST-14/ST-15 conditional `n/a`. Challenger: agrees (A).
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

### PF-002: "verified" means both a confidence label and "survived Phase 9" 🟠 MAJOR
**Dimension:** 1 Ambiguities · **Location:** `01` R9/AC #3, `03-04` §DEF rows and report §4/§7, `03-01` §Confidence/§Escalation, `03-03` WS8-02, `07` ST-10/ST-11
**The Problem:** A `probable` Major that survives Phase 9 needs a DEF row under ST-10 but not under a literal R9; WS8-02 runs before Phase 9 so its "verified" can only be the label; report §4/§7 say "confirmed" and would drop `downgraded`/`upgraded` survivors.
**Options:** A) `verified` = confidence label only; "surviving" = Phase 9 verdict confirmed/downgraded/upgraded; DEF rows for surviving Critical/Major of any confidence, the row noting its confidence; recorded as a clarification of AR #13 · B) rename the confidence label.
**Recommendation:** A — B renames a label the user accepted (AR #15). Phase 9 must attempt reproduction of every `probable` Major. WS8-02 covers every finding of Minor or above. Challenger: agrees (A).
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

### PF-003: Roadmap edit window conflicts with exec-plan's roadmap sync 🟠 MAJOR
**Dimension:** 3 Logical Contradictions · **Location:** `07` ST-7, `03-01` §Ground rules, `00-index` §Related Files, `99` assessment rules, AR #20 wording, task 10.2.3
**Codebase Evidence:** `.opencode/skills/exec-plan/SKILL.md:264-272` sets the row to Executing/Done and adds `↳ DEF` blocker rows.
**The Problem:** ST-7 fails at the first gate every time; the 99 rule also forbids 10.2.3's own edit.
**Options:** A) allow this plan's row status/progress updates and exec-plan blocker DEF rows in any phase (through the roadmap skill); finding-derived DEF rows only in Phase 10; ST-10's reverse check covers finding-derived rows only · B) suppress exec-plan's sync until Phase 10.
**Recommendation:** A — B overrides the skill's mandate. Challenger: agrees (A).
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

### PF-004: "Phases 1–7 independent, any order" is false and not executable 🟠 MAJOR
**Dimension:** 11 Ordering · **Location:** `99` Overview and §Dependencies; `02` §Risks
**Codebase Evidence:** tasks 3.3.1 (needs WS2), 6.3.1 (needs WS5), 7.2.3/WS7-05 (needs WS5/WS6); `exec-plan/SKILL.md:151-154` resumes in document order only.
**Options:** A) sequential in document order, diagram shows the true edges 0→all, 2→3, 5→6, {5,6}→7 · B) parallel sessions in separate worktrees with a merge procedure.
**Recommendation:** A — B is unapproved machinery. Challenger: agrees (A).
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

### PF-005: Baseline semantics break on the assessment's own commits; tools are unpinned; "expectation before run" is unprovable 🟠 MAJOR
**Dimension:** 2 Implicit Assumptions · **Location:** `03-01` §Ground rules, §Confidence, §Error Handling; `07` ST-2, ST-5, ST-7; `99` task 9.1.2
**Codebase Evidence:** `Makefile:168-193` (bench targets rebuild `target/` and stamp `--compiler-git $(git rev-parse HEAD)`); `docker/Dockerfile:131-133` (amitools from git HEAD).
**The Problem:** probes are committed after the baseline, so "at the baseline SHA" is impossible; every assessment commit moves HEAD and literally triggers "baseline SHA changes → stop"; `clc`, the image, and `vamos` can change mid-run; nothing proves expectations preceded observations.
**Recommendation (single viable path):** baseline = SHA X; drift check at every gate = `git diff --quiet X HEAD -- . ':(exclude)plans'` plus an empty `git status --porcelain -- . ':(exclude)plans'`; "reproduce at baseline" = at HEAD with that check passing; keep a pinned release `clc` copy and its sha256 under `build/tmp/assessment/baseline/`; record the image ID and amitools commit; no image rebuild during the assessment; harness reruns pass `--compiler-git X`; commit Step N.1 (probes + expectations) before Step N.2; an amended expectation keeps the original and a reason. Challenger: agrees, with the pathspec simplification adopted here.
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

### PF-006: No commit mode; every commit path falls back to the forbidden gates 🟠 MAJOR
**Dimension:** 5 Dependency Issues · **Location:** `99` Overview and every Verify (gate) line
**Codebase Evidence:** `git-commit/SKILL.md:17` and `exec-plan` execution protocol take the verify command from the plan's Verify line or fall back to AGENTS.md (`make check`/`make verify`, forbidden by AR #3).
**Recommendation (single viable path):** run with `--ask-commit`, commit only (no push unless the user asks) after each Step N.1 and each phase gate, and list those commit points; add a command-shaped `**Task verify**` line (`git diff --check` + the allowed-path check + the PF-005 drift check) and make each `**Verify (gate)**` that same command plus the phase's ST row. Challenger: agrees.
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

### PF-007: exec-plan's strict-mode quality loop is not switched off and collides with `05-findings.md`; `[spec-author]` markers misroute 🟠 MAJOR
**Dimension:** 11 Ordering · **Location:** AR #16, `00-index` §Minimum-Sufficient Baseline, `03-04` §`05-findings.md`, task markers in `99`
**Codebase Evidence:** no `codeops/codeops.json`; strict defaults apply and `independentReview:false` is "allowed only outside strict mode" (`.opencode/_shared/quality-profile.md:79`); the execution protocol creates `05-findings.md` with an RV schema; `[spec-author]` routes to `spec-test-author`, which writes `*.spec.test.*` files and expects RED.
**Options:** A) state in 99 that AR #16 replaces the post-phase quality step for Phases 0–9 as an explicit strict-profile exemption (needs your one-line confirmation), drop all `[spec-author]` markers, route 10.3.1's review rows to a `## Report review` section · B) let the loop run (reviewer-only on docs diffs) into a separate `## Execution reviews` section excluded from ST-10.
**Recommendation:** A — B contradicts AR #16 and adds eleven reviews; the bulk acceptance of AR #16 never showed the strict-mode conflict, hence the confirmation. Challenger: agrees (A).
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

### PF-008: The bench harness covers one cell of the planned measurement matrix 🟠 MAJOR
**Dimension:** 2 Implicit Assumptions / 13 · **Location:** `03-01` §Measurement rules; `03-02` WS4-01, WS4-08; `03-03` WS7-01, WS7-03; `99` 0.2.2–0.2.4
**Codebase Evidence:** `tools/perf/bench.py:37,48-54` hard-codes `-mcrt=nix20 -m68000 -O2`; builds only host and hosted-amiga from `program.clo` (`:381-403`); `make bench*` overwrite `build/perf/*.json`.
**The Problem:** the A500 lens needs `hosted-amiga-13` (nix13); `-Os`, `-m68020`, and C sources are not supported; the matrix is specified three times (0.2.3, WS4-01, WS7-01).
**Options:** A) Phase 0 owns the matrix through one committed probe script under `probes/WS0/` that only calls `bench.py`'s existing `emit`/`build_step`/`trace_vamos`/`size_sections` with explicit flag arrays — no new metrics or comparison logic, output under `build/tmp`; lenses A500 = `hosted-amiga-13`/nix13/68000, A1200 = `hosted-amiga`/nix20 + `-m68020` probe; WS4/WS7 cite it; harness runs use `--out` under `build/tmp`; `make bench` numbers labelled nix20 · B) shrink the matrix to the harness (contradicts AR #2/R4).
**Recommendation:** A. Complexity check: the challenger's verdict is **Simplify, not escalation** — a probe runner over existing functions is within the plan's "small scripts" probe layout provided the limits above are written into 03-01. Challenger: agrees (A).
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

### PF-009: C references lack fairness rules and their own calibration 🟠 MAJOR
**Dimension:** 6 Feasibility / 7 Testability · **Location:** `99` 0.1.2; `03-01` §Severity, §Measurement rules; `07` ST-4; WS4-01/02
**Codebase Evidence:** `bench.py:490-512` nets every count against the Clorraine `benchmarks/calibration` program (2,928 instructions); `local-table` is 5,855 raw; `.text` is 23.6–24.6 KB for seven benchmarks, mostly fixed shim cost (`build/perf/head.json`); `benchmarks/sieve/program.clo:2` exercises spec-mandated bounds checks.
**The Problem:** startup differences alone cross the 10% Major threshold on small benchmarks; one fixed cost yields eight Majors and eight DEF rows; an unchecked "idiomatic" reference grades spec-required checks as defects; "higher severity across lenses" invites cherry-picking.
**Recommendation (single viable path, simplified by the challenger):** fairness rules pinned in 03-01 before Phase 0 — same algorithm, integer widths, data layout, allocation pattern, output routine, and the spec-required checks written by hand; one such reference per benchmark (guard cost is measured separately by the WS4-03 patched-C probes, not by extra unchecked references); one C calibration program per lens cell, each side netted against its own calibration, raw counts reported too; per-benchmark `.text` compared above each side's calibration `.text`; the fixed cost reported once (WS4-07); Major only from the shipped 68000 builds — 68020-probe gaps are Opportunities.
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

### PF-010: Falsifiability is trivial; `clean`/`limit` outcomes need no evidence and are never checked 🟠 MAJOR
**Dimension:** 7 Testability · **Location:** `03-01` §Falsifiability, §Verification protocol; `07` ST-5, ST-8
**The Problem:** a mutated expectation only proves the diff works; detector-based probes (odd address, MMIO width, KS availability, Ctrl-C) may never fire; a "clean" verdict can rest on nothing, so false negatives — the main risk of a strict audit — are unreviewed.
**Recommendation (single viable path, scaled by the challenger):** probes behind a finding or a `clean` use a contrasting input or a seeded defect; mutated expectations only for comparator scripts; a detector that cannot be made to fire yields `limit`, not `clean`; `clean` cites probe IDs and/or a `file:line` site list (ST-8 amended); Phase 9 re-checks a sample of at least two `clean` outcomes per workstream chosen on the A500/memory-safety lenses, and report §2 states the sample.
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

### PF-011: The routed agents cannot author or record anything 🟠 MAJOR
**Dimension:** 13 Codebase Alignment (Architecture Mismatch) · **Location:** `03-01` §Agent routing; `99` 9.2.x; `07` ST-9; `00-index` §Specialist Agents
**Codebase Evidence:** all six routed agents are `edit: deny`, review one phase diff, treat pre-existing defects as out of scope (`memory-safety-auditor.md:65`, `correctness-reviewer.md:33`), stop without a diff (`semantics-reviewer.md:94`); `performance-auditor.md` is an untracked generic catalog file.
**Options:** A) agents as read-only verifiers via overriding packets · B) the executing session authors all probes and findings; Phase 9 uses fresh general sessions whose packets include the matching specialist brief as a checklist plus "whole component at X, pre-existing defects in scope, ASM IDs, read-only, writes only under `build/tmp`"; the lead records verdicts with verifier identity; ST-9 becomes "issued by"; `correctness-reviewer` stays for 10.3.1 (the report is a real diff); `performance-auditor` dropped.
**Recommendation:** B — A relies on fragile packet overrides of "stop if no diff". Challenger: B for verification, A-style authoring by the executing session (as stated in B).
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

### PF-012: Three checklist methods have no route inside the allowed runs 🟠 MAJOR
**Dimension:** 6 Feasibility · **Location:** `03-03` WS8-03, WS7-06; `03-02` WS3-09; `99` 8.1.1–8.1.2, 7.1.1, 7.2.4
**Codebase Evidence:** WS8-03 mutation needs tracked-test edits or `cargo test` runs (outside AR #14) and mutating an expectation is tautological; no profiler in the image (`docker/Dockerfile:96-120`); `tools/perf/compile_time.py:59` already generates 1k–8k inputs; no Rust on the host.
**Options:** A) WS8-03 = static assertion classification (value / diagnostic code / compiles-only / no-panic); WS7-06 = the `compile_time.py` series plus ownership-heavy probe generators, hotspots attributed by scaling exponent and code reading (`probable`), host `perf` on the pinned `clc` copy only if it runs, else a limit; WS3-09 = two container emissions plus debug-vs-release `clc` emission comparison · B) widen AR #14 for bounded mutation runs in a `build/tmp` worktree.
**Recommendation:** A — the challenger also removed the "instrumented `clc` copy" idea as too close to the prototype AR #17 excludes.
**User Decision:** Resolved — User accepted recommendation ("Accept all recommendations", 2026-11-25); applied in iteration 1 fixes

---

## 🟡 MINOR

| ID | Dimension | Location | Problem | Recommendation | User Decision |
|----|-----------|----------|---------|----------------|---------------|
| PF-013 | 9 Edge Cases | `03-01` §Escalation, §Verification; `07` ST-11 | A Critical upgraded or confirmed in Phase 9 has no escalator; a refuted escalated Critical is never retracted; the notification leaves no record | Verifier escalates any verdict yielding a Critical; the escalation entry records the message text; the Phase 9 verdict is recorded beside it and the user told; retractions listed in report §2; ST-11 amended | Resolved — User accepted recommendation; applied |
| PF-014 | 7 Testability | `07` ST-7, ST-9, ST-12, ST-16; `03-01` §Finding template | ST-9 needs an author-session field; ST-16 has no dispatch record; ST-12 "any number" covers line numbers and counts; ST-7 names no command | Add `Author session` to the template; one dispatch-effort line per workstream file; ST-12 = measured quantities only; ST-7 = the PF-005 drift check plus the allowed-path check | Resolved — User accepted recommendation; applied |
| PF-015 | 4 Completeness | `03-03` WS5 scope; `02`; `03-02` WS1 components | There are five optional shims, not four; `emulator_runner.c` unassigned; `driver.rs` and `analysis.rs` (the shared front-end pipeline, `analysis.rs:1-10`) in no workstream; stdlib is 43 modules (6 + 37), not 45; big tasks have no inner resume point | Say five; exclude `emulator_runner.c` as driver surface (AR #9); assign `driver.rs` and `analysis.rs` to WS1; correct to 43; each workstream file keeps a per-file / per-function-group coverage table, checked at the WS5/WS6 gates and used as the in-task resume point | Resolved — User accepted recommendation; applied |
| PF-016 | 13 Codebase Alignment | `02` §What Exists | Facts were taken from a working tree: `shim_hosted_amiga.c` is 6,030 lines at `2eff956c`, `compiler/src` 96,229, driver 5,126, tests 157; `ede6823a` added a raster-DMA write in `cl_video_display_flip` (`shim_hosted_amiga.c:4579-4587`) | Re-record the facts at `2eff956c`; add the DMA write as a WS5-09 pointer | Resolved — User accepted recommendation; applied |
| PF-017 | 12 Consistency | `03-02` WS2 header vs `99` 2.x | WS2a/WS2b split differs (NLL placement); WS2-12 points at WS8-07 instead of WS8-08 | Split by ID ranges WS2a = 01–06, WS2b = 07–12; point WS2-12 at WS8-08 | Resolved — User accepted recommendation; applied |
| PF-018 | 1 Ambiguities | `07` ST-5; AR #10 wording | Accepted expectation sources omit the C standard, NDK autodocs, and documented harness invariants needed by WS3/WS5/WS7/WS8 | Add those sources | Resolved — User accepted recommendation; applied |
| PF-019 | 3 Contradictions | `07` ST-4 vs `03-01` §Measurement rules | ST-4 forbids different flags on both sides, so flag findings (WS4-08, WS7-03) can never be Major | ST-4 exception for flag findings, which state both flag sets | Resolved — User accepted recommendation; applied |
| PF-020 | 1 Ambiguities | `03-04` §Scorecard, §Quick wins; `03-02` WS1-10, WS3-11; `03-03` WS8-03; `03-01` Minor | Grades D/E overlap; "recurring"/"systemic" undefined; quick-win metric unnamed; no time/RSS bounds; no sample size; area mapping unstated | D = exactly one Critical; recurring = ≥3 DEF rows of one WS8-07 class across ≥2 plans; systemic = ≥2 Criticals sharing one WS8-08 root cause; quick-win metric = net instructions or `.text`; WS1-10/WS3-11 bound = 10× the `bench-compile` baseline time or RSS for the same input size; WS8-03 sample = 5 tests per area; areas mapped by checklist prefix | Resolved — User accepted recommendation; applied |
| PF-021 | 10 Scope Creep | `03-03` WS6-06; `03-04` §Report structure | WS6-06 sends API gaps to an "opportunity list" that does not exist, and "gaps a typical Amiga program needs" drifts beyond assessing existing modules | Add an "Unranked recommendations" subsection to report §8 (also holds excluded-machinery recommendations and the WS7-03 profile-variant question); limit WS6-06 to gaps observed while assessing existing modules | Resolved — User accepted recommendation; applied |
| PF-022 | 3 Contradictions | `03-02` WS1 lens; `03-04` error row; `03-01` error rows; `03-04` line citing ST-9 | "Diagnostic wording is out of scope (AR #9)" — AR #9 says nothing about diagnostics; several other citations point at the wrong AR/ST | Keep wrong diagnostic **codes** in scope (wrong rejection) and drop the wording exclusion's false citation — wording stays out as a stated assessment focus, not an AR; fix the other citations | Resolved — User accepted recommendation; applied |
| PF-023 | 6 Feasibility | `03-01` §Measurement rules; `02` §Risks | The A1200 fallback ("FS-UAE timing") has no instruction counter and contradicts AR #17's limited FS-UAE use | If `vamos` lacks a 68020 mode: report static 68020 section sizes and record dynamic numbers as a named limit | Resolved — User accepted recommendation; applied |
| PF-024 | 6 Feasibility | `03-01` §Ground rules; WS5 method | No route for ad-hoc FS-UAE probes; decrypted KS1.3 ROM handling unstated; fixed `build/run/out.txt` paths | Ground-rule row: host-side `docker/scripts/run.sh` via the compose `emulator` service on Xvfb, one emulator run at a time, KS1.3 ROM decrypted into `build/tmp/assessment/` and deleted on exit as the gates do, output copied into `build/tmp/assessment/<WS>/` | Resolved — User accepted recommendation; applied |
| PF-025 | 2 Assumptions | `03-01` §Ground rules | Known container quirks absent: ASan needs `-no-pie`; UBSan needs `-fno-sanitize-recover=all`; `ASAN_OPTIONS=detect_leaks=1`; vbcc's transient "No config file!" on first run; `$` must be `$$` in `make -C docker exec CMD` | Write the exact flags and the single vbcc retry into the ground rules | Resolved — User accepted recommendation; applied |
| PF-026 | 11 Ordering | `03-03` WS8-02/08; `99` 9.2.5; `03-04` §5 | WS8 links and themes can cite findings Phase 9 later refutes | 9.2.5 runs last and re-points WS8 links for refuted/merged IDs; report §5 cites surviving findings only | Resolved — User accepted recommendation; applied |
| PF-027 | 11 Ordering | `99` Phase 10 | DEF IDs are allocated after the appendix that maps them and before the independent review | Order: 10.2.1 → 10.2.2 → review → DEF allocation and write-back (appendix included) → ST-10 re-check → gate | Resolved — User accepted recommendation; applied |
| PF-028 | 11 Ordering | `99` 4.1.1–4.1.2 | Phase 4's expectation-first step covers WS4-03..06 only | Extend to every Phase 4 probe; with PF-008 the matrix moves to Phase 0 | Resolved — User accepted recommendation; applied |
| PF-029 | 2 Assumptions | `99` 0.2.1 | "Emit … for every profile" — `host` rejects `std.amiga.*`, `hosted-amiga-13` accepts a subset, `freestanding` is console-only (`profile.rs:93-110`) | Emit for every profile that accepts the program; record rejections as expected | Resolved — User accepted recommendation; applied |
| PF-030 | 13 Test Impact | `07` ST-13, ST-14 | "Byte-identical" is stricter than the harness, which compares after `normalize_output` (`bench.py:171-178`) | Use "identical after `bench.normalize_output`" | Resolved — User accepted recommendation; applied |
| PF-031 | 13 Stale Assumptions | `03-02` WS1/WS2/WS3 spec lists | `defer` (ch. 05) and lambdas (ch. 06 §6.1) missing from WS2; volatile/MMIO (ch. 07 §7) missing from WS3; WS1-07's register conventions have no spec source | Add 05 and 06 to WS2, 07 to WS3; WS1-07 expectation source = NDK prototypes, or a spec question | Resolved — User accepted recommendation; applied |

## 🔵 OBSERVATION

| ID | Location | Note | Recommendation | User Decision |
|----|----------|------|----------------|---------------|
| PF-032 | `03-04` §DEF rows | The roadmap carries DEF IDs both as tracker `↳` rows and in a separate backlog table | State: tracker `↳` children under PLAN-47, newest first | Resolved — User accepted recommendation; applied |

### Adversarial self-check (lead)

- Assumption possibly confirmed by the author: that the existing agents fit — refuted by PF-011.
- External standard not cited: C99 and NDK autodocs — now named as expectation sources (PF-018).
- A domain expert's likely objection: comparing against unchecked C — addressed by PF-009.

---

## Preflight Report: Independent Assessment of Clorraine — Iteration 2

> **Status**: 10 new findings (1 major, 7 minor, 2 observations); all resolved
> **Previous Iteration**: 32 findings — 26 verified fixed, 6 partial (carried into PB-001..PB-009 by root cause)
> **This Iteration**: 10 findings (PB-001 🟠; PB-002..PB-008 🟡; PB-009, PB-010 🔵)
> **Independence**: one fresh-context auditor session; Effort: high→high

| ID | Severity | Problem | Resolution | User Decision |
|----|----------|---------|------------|---------------|
| PB-001 | 🟠 | Harnesses confine `--out` to `build/perf/` (`bench.py:476`, `compile_time.py:401`, `flag_audit.py:519`); `flag_audit.py` has no `--compiler-git`; runs did not pass the pinned `clc` | New uniquely named `build/perf/assessment-baseline-*.json` outputs copied to `build/tmp/assessment/baseline/`; never overwrite an existing result; `--clc` pinned for all three; `--compiler-git` where supported | Resolved — User accepted recommendation |
| PB-002 | 🟡 | Old "verified" meaning in four places; roadmap row said 0/89 | "Surviving"/"reviewed"; roadmap row 0/90 | Resolved — User accepted recommendation |
| PB-003 | 🟡 | `measure.py` function list could not check host output | Added `run_host`, `run_vamos`; Host cell is an output check | Resolved — User accepted recommendation |
| PB-004 | 🟡 | Red checks ran before expectations were committed | Commit at the end of the last Step N.1 writing task, before the red check | Resolved — User accepted recommendation |
| PB-005 | 🟡 | WS3-11 cited an unmeasured baseline | Task 0.2.2 records C compile times; 03-01 §Severity maps WS3-11 | Resolved — User accepted recommendation |
| PB-006 | 🟡 | Drift check allowed changes to other plans' folders | Drift check forbids any `plans/` change other than this folder and the roadmap | Resolved — User accepted recommendation |
| PB-007 | 🟡 | WS0 findings had no gate cases and no verifier | Phase 0 row gains ST-1..ST-3, ST-11; 9.2.1 verifies WS0 | Resolved — User accepted recommendation |
| PB-008 | 🟡 | WS7-05 `clean` lacked valid evidence | Cites WS5/WS6 probe IDs or site lists | Resolved — User accepted recommendation |
| PB-009 | 🔵 | Wording drift: task number, routing groups, lens names, probe layout, coverage-table wording | Aligned | Resolved — User accepted recommendation |
| PB-010 | 🔵 | DEF rows "under PLAN-47" would mix with PLAN-45's children | Insert directly below the PLAN-47 row; no reordering | Resolved — User accepted recommendation |

## Preflight Report: Independent Assessment of Clorraine — Iteration 3 (bounded)

> **Status**: ✅ PASSED — no unresolved 🔴/🟠
> **Previous Iteration**: 10 findings — 6 verified fixed, 4 partial (PB-002, PB-004, PB-009 leftovers)
> **This Iteration**: 1 new finding (PC-001)
> **Independence**: one fresh-context auditor session; Effort: high→high

The partial leftovers belonged to the same root causes and were completed under the accepted
PB-002, PB-004, and PB-009 recommendations: remaining "verified ledger/evidence" wording; the
Overview's Step 1 order, task 9.1.2's extra commit moved into 9.1.1, and a commit added to
10.1.1; the "A1200 lens" and "both Amiga lenses" names. Each edit was verified directly.

| ID | Severity | Problem | Resolution | User Decision |
|----|----------|---------|------------|---------------|
| PC-001 | 🔵 | ST-7's parenthetical understated the PB-006 drift check | Reworded to cite 03-01 §Ground rules; applied as part of the accepted PB-006 fix | Resolved — completes the accepted PB-006 recommendation |

No further iteration: every 🔴/🟠 finding is resolved and no 🟡 remains open.
