# Preflight Report: Clorraine v1.0 Production Readiness

> **Status**: ✅ PASSED WITH NOTES — 40 findings resolved across 3 iterations; the final two fixes were directly verified; a fresh-session audit is recommended before execution.
> **Iteration**: 3 (iteration 1 scan; iteration 2 re-assessment and fixes; iteration 3 scoped verification and fixes)
> **Artifact**: Implementation plan directory `plans/v1-production-readiness/` (11 documents, 1,562 lines)
> **Artifact manifest**: `168af3f787047a8e36935cfecbfcb97818a368e3d0dd01ab474f13f48926bffe` (SHA-256 of the `sha256sum` manifest over all 11 files)
> **Repository revision at scan start**: `9b05927` (`main`); the plan directory is untracked and `plans/00-roadmap.md` is modified in the working tree
> **Codebase grounded**: all referenced compiler paths, test commands, and corpus scripts were opened; the plan's key programs were executed against `target/debug/clc` (built 2026-10-30 from the same compiler sources)
> **Last Updated**: 2026-10-31 21:17

**Same-session note:** the artifact was not created in this session; it was authored earlier on 2026-10-31. Same-model bias cannot be excluded. This scan used five independent auditor contexts, direct compiler probes, and one independent challenger; every finding below was re-checked against the artifact or the code.

**Scope mode:** strict. Optional additions are not reported. The audit target is the plan directory only. Context documents are read for evidence and are not passed or modified: `plans/00-roadmap.md`, `plans/def-zero-closeout/05-findings.md`, `plans/clorraine-dx/99-execution-plan.md`, `spec/`, `Makefile`, `compiler/src/`, `compiler/tests/`, and `docker/scripts/`. The authorized modification set is empty: preflight is review-only and no artifact edits were applied.

**Auto-design:** active. Eligible technical resolutions are recorded as delegated decisions with provenance. Product, scope, acceptance-criteria, legal, and complexity-escalation choices remain reserved for the user. Delegated decisions select a remediation; they do not authorize applying it.

**`--auto-commit` note:** `--auto-commit` is an `exec-plan` commit mode, not a preflight option. It does not authorize file edits, commits, or pushes under the preflight protocol. No files were changed by this run.

---

## Codebase Context Summary

**Tech stack:** a std-only Rust 2021 workspace with one member, `compiler/`; `clc` emits one C99 translation unit and `clorraine` builds/runs programs. The SDK container supplies `m68k-amigaos-gcc`, `vamos`, and FS-UAE. The demo tooling is a separate npm package under `tools/demo/`.

**Architecture:** `compiler/src/` has `lex`, `parse`, `ast`, `load`, `resolve`, `sema`, `own`, `ir`, `codegen`, `types`, `consts`, plus `driver`, `diag`, `abi`, `profile`, `stdlib`, `borrow`, `ffi`, `span`, `token`, `init_graph`, and the `clorraine` driver. The runtime shims are `compiler/runtime/shim_{host,hosted_amiga,freestanding}.c`. The emitter is deterministic and already has initializer-owner, projection-root, and cleanup-ladder machinery.

**Key files examined:** `Makefile`; `spec/check.sh`; `spec/README.md`; `spec/02-types.md`; `spec/03-ownership.md`; `spec/07-structs.md`; `spec/08-arrays-strings.md`; `spec/14-diagnostics.md`; `compiler/src/sema/expr.rs`; `compiler/src/codegen/program.rs`; `compiler/src/own/lower.rs`; `compiler/src/own/expr.rs`; `compiler/src/driver.rs`; `compiler/Cargo.toml`; `compiler/tests/*.rs`; `docker/scripts/corpus-amiga.sh`; `plans/def-zero-closeout/05-findings.md`; `plans/clorraine-dx/99-execution-plan.md`; `plans/00-roadmap.md`.

**Observed facts used below:**

| Fact | Evidence |
|---|---|
| `make check` = `./spec/check.sh` + `cargo test --workspace` in the SDK container | `Makefile:43-45` |
| `make verify` = `check` + `corpus-fs-uae` + `run-fault-amiga`; `corpus-fs-uae` depends on `corpus-amiga`; FS-UAE needs X and local ROMs | `Makefile:80-91`, `Makefile:19-21` |
| `spec/check.sh` checks titles, status-line presence, relative links, diagnostic registration, and compile-fail coverage | `spec/check.sh:20-31`, `:71-196` |
| 81 integration test files; 21 reference `RDC_ASAN`; some suites contain 21 or 230 tests | `ls compiler/tests/*.rs`, `grep -l RDC_ASAN`, `grep -c '#\[test\]'` |
| The proposal's `make asan` script does not set `--test-threads` | `03-03-verification-infrastructure.md:66-75` |
| There is no `.github/workflows/`, no `LICENSE`, no `RELEASE.md`, no `docs/known-limitations.md`, no git tag | `ls`, `git tag` |
| `spec/README.md` and every chapter are draft; `spec/check.sh` only checks status-line presence | `spec/README.md:3`, `spec/00-introduction.md:3`, `spec/check.sh:31-45` |
| The DX plan is 0/28; `plans/amiga-library-surface/` does not exist | `plans/clorraine-dx/99-execution-plan.md:6`, `ls plans` |
| `std.amiga.exec` is absent; `std.amiga.{custom,dos,graphics,interrupt,intuition}` exist | `ls stdlib` |
| `compiler/Cargo.toml` has no `license`, `description`, or `repository`; version is `0.1.0` | `compiler/Cargo.toml:2-6` |
| `tools/demo/package.json` declares `"license": "UNLICENSED"` | `tools/demo/package.json:24` |
| The binary reports `clc 0.1.0` and `clorraine 0.1.0` | `./target/debug/clc --version`, `./target/debug/clorraine --version` |

**Reference verification:** every code path, test command, Makefile target, and corpus script named by the plan was opened. The incorrect or unverifiable references are enumerated in PF-001, PF-002, PF-003, PF-005, PF-006, PF-013, PF-014, PF-015, and PF-023.

---

## Verification Probes

These probes were run against `target/debug/clc` with `--profile host`; ASan probes used `cc -fsanitize=address` and `ASAN_OPTIONS=detect_leaks=1`.

| Probe | Result |
|---|---|
| `let b: bool = null;` | `error E10012: 'null' is not allowed here` (already the claimed ST-1 result) |
| `bool(null)` | `error E10201: cannot cast 'null' to 'bool'` |
| `if (null)` / `!null` | `E10218` condition diagnostic |
| `let rows: str[2][2] = [[mk(), "aa"], ["bb", "cc"]];` (ST-3) | emits, strict-builds, runs; no ASan/LSan report |
| `Box { names: [; stringOf("x")] }` with `names: String[2]` (ST-2) | `error E10215: array fill requires a fixed extent` (move-only fill) |
| `let s: String = mk()[pick()];` (ST-5) | `error E10026: partial moves are limited to top-level fields in v1` |
| `Box { n: sink(mk().xs[0]) }` | emits, strict-builds, runs; **11-byte LeakSanitizer report** (the sibling `xs[1]` is never freed) |
| `freestanding` corpus under `--profile host` | exit 0 (so the plan's "only three host-portable programs" claim is inaccurate) |

---

## Summary by Dimension

| # | Dimension | Findings | Highest severity |
|---|---|---|---|
| 1 | Ambiguities | 0 | — |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 2 | 🔴 |
| 4 | Completeness Gaps | 4 | 🟠 |
| 5 | Dependency Issues | 4 | 🟠 |
| 6 | Feasibility Concerns | 1 | 🟠 |
| 7 | Testability | 5 | 🔴 |
| 8 | Security Blind Spots | 2 | 🟠 |
| 9 | Edge Cases | 0 | — |
| 10 | Scope Creep Indicators | 2 | 🟠 |
| 11 | Ordering & Sequencing | 1 | 🟠 |
| 12 | Consistency | 2 | 🟠 |
| 13 | Codebase Alignment | 2 | 🟡 |

## Summary by Severity

| Severity | Count | Status |
|---|---|---|
| CRITICAL | 2 | 0 applied; 2 pending (1 delegated decision, 1 reserved) |
| MAJOR | 12 | 0 applied; 12 pending (7 delegated decisions, 5 reserved) |
| MINOR | 10 | 0 applied; 9 delegated decisions, 1 reserved (PF-021) |
| OBSERVATION | 1 | acknowledged (PF-025) |

---

## Findings

### PF-001: ST-1 tests a case that already passes; the real null residual is the explicit cast 🔴 CRITICAL

**Dimension:** 7 Testability (also 13 Codebase Alignment)
**Location:** `03-01-residual-hardening.md` §Code Examples "Null-to-bool"; `02-current-state.md:53-56`; `07-testing-strategy.md:32` (ST-1); `99-execution-plan.md:67,74`
**Codebase evidence:** `target/debug/clc --profile host` on the plan's exact ST-1 source prints `error E10012: 'null' is not allowed here` at the `let`. The actual residual recorded at `plans/def-zero-closeout/05-findings.md:279` is the explicit cast spelling: `bool(null)` prints `E10201`, and `if (null)`/`!null` print `E10218`. The cast path is `compiler/src/sema/expr.rs:1109-1125`; the annotated-let null rule is enforced separately in `compiler/src/own/expr.rs` (null-capable destination check).
**The problem:** Task 1.1.2 claims the red phase will show "ST-1 reports the wrong code"; it does not. Task 1.2.1 may change no behavior while ST-1 passes, and the actual `bool(null)` path is left untested. The artifact also calls the current result an "integer-cast diagnostic", which is not what the compiler emits for this input.
**Options:**
- **A.** Rebase ST-1 on `bool(null)` (and decide whether `!null` is in scope), keep `let b: bool = null;` as an accepted control, and correct the `02-current-state` claim. This requires a user ruling on whether `bool(null)` is E10012 (TY-7) or E10201 (TY-14), because `spec/04-expressions.md` currently lists unsupported casts as E10201.
- **B.** Drop the null item from Phase 1 and record the cast policy as an accepted limitation in `docs/known-limitations.md`.
- **C.** Change `check_cast` to E10012 for a null destination and update `spec/04-expressions.md` and `spec/02-types.md` to match (a language-rule change).

**Recommendation:** Option A, with the user ruling E10012 for `bool(null)` so AR-4 and TY-7 agree, then updating the cast table to name the exception. Option C is the same code change but needs the spec edit made explicit; Option B abandons a release-blocking item the user already selected.
**Decision:** **Pending — reserved.** Language-rule/diagnostic semantics are user authority under `--auto-design`; the delegated part (rewriting the test once the rule is fixed) can proceed immediately after the ruling.

---

### PF-002: ST-2 and ST-5 require programs the frozen specification rejects 🔴 CRITICAL

**Dimension:** 3 Logical Contradictions (also 13 Codebase Alignment)
**Location:** `07-testing-strategy.md:33,36`; `03-01-residual-hardening.md:39-41,89-105`; `99-execution-plan.md:75-77`
**Codebase evidence:** ST-2's `Box { names: [; stringOf("x")] }` with `names: String[2]` reports `E10215` because AL-1 requires a duplicable fill value (`spec/08-arrays-strings.md:44-50`; `compiler/src/sema/expr.rs:1283-1335`). ST-5's `let s: String = mk()[pick()];` reports `E10026` because v1 permits partial moves only for top-level fields of owned locals/`take` parameters; element moves are explicitly deferred (`spec/03-ownership.md:125-130`, `spec/07-structs.md:106-107`, `spec/14-diagnostics.md:57`).
**The problem:** The tests are written as "builds and runs" oracles, but the frozen language rejects the inputs. Passing them would require accepting move-only fills and element moves out of call results — a language-surface change, not an AR-4 residual fix. R2 and the index both say the programme does not reopen the language surface.
**Options:**
- **A.** Rewrite ST-2 as a `str[2]` view fill and ST-5 as an accepted view or discard shape; if the claimed leak does not reproduce, drop the item from AR-4. This keeps the frozen rules.
- **B.** Record a user ruling to change AL-1/OW-12 and treat both as language work, with the corresponding spec, checker, and emission changes.
- **C.** Delete the fill and indexed items from AR-4 and document the current rejection as correct.

**Recommendation:** Option A. It is the only option that preserves the approved scope. A valid `str[2]` fill and a discarded `mk()[pick()]` both compile today; use them as the starting point and only keep the item if a base leak reproduces.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Test-design rewrite inside the frozen rules; no product scope or acceptance change. Apply after PF-001/PF-003 rebaselining so the suite has one coherent red/green contract.

---

### PF-003: ST-3 and ST-4 are not red; the reproduced initializer leak is a field call root, not the described shape 🟠 MAJOR

**Dimension:** 7 Testability (also 13 Codebase Alignment)
**Location:** `07-testing-strategy.md:34-35`; `02-current-state.md:57-59`; `99-execution-plan.md:76`
**Codebase evidence:** ST-3's exact `str[2][2]` program emits, strict-builds, runs, and is ASan/LSan clean (probe above). ST-4 is prose only ("a view-array initializer whose element calls through a func-typed `take` callee with a fresh projection") and gives no program; the `take` element move is itself E10026. A real leak exists at an initializer field call root: `struct H { xs: String[2]; } struct Box { n: i32; } Box { n: sink(mk().xs[0]) }` leaks 11 bytes because the emitted initializer binds only the selected `xs[0]` and never frees `xs[1]`. The same emitter path is in `compiler/src/codegen/program.rs:1133-1430`; the plan's `retain_view_array_roots` walk (`:1399-1423`) is not the path this probe takes.
**The problem:** Two of the six specification cases cannot drive the implementation: ST-3 already passes and ST-4 has no concrete input. The one reproducible leak is a different shape, so the plan's "nested rows" and "func-typed take" fix may be a no-op while the real accepted leak survives.
**Options:**
- **A.** Re-baseline ST-3/ST-4 on verified HEAD repros: keep ST-3 as a clean control, add the field-call-root probe (direct call and func-typed twin) as the red case, and require an ASan assertion. Record the exact base leak size.
- **B.** Keep the current ST-3/ST-4 text; they will pass without a fix.
- **C.** Expand Phase 1 to every initializer field call root, adding a new residual class and more work.

**Recommendation:** Option A. It is the smallest change that produces a genuine red test and targets the emitter path that actually leaks. Option C is scope growth not authorized by AR-4.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Test-design and target-shape correction inside the approved residual class.

---

### PF-004: `make asan` does not serialise tests inside a suite; the accepted flake remains 🟠 MAJOR

**Dimension:** 6 Feasibility Concerns (also 7 Testability)
**Location:** `03-03-verification-infrastructure.md:29-32,66-75`; `99-execution-plan.md:141-142`; `07-testing-strategy.md:43`; acceptance criterion 4
**Codebase evidence:** The proposed script runs `timeout 900 env RDC_ASAN=1 cargo test --test "${suite}"` with no `--test-threads=1` or `RUST_TEST_THREADS=1`. libtest runs a suite's tests in parallel by default; `conditional_projection_residue_spec_test.rs` has 230 tests and several other ASan suites have 21-45. The closeout records the residual as a "parallel ASAN child-run flake" where "a handful of tests hang in LSan and pass in isolation" (`plans/def-zero-closeout/05-findings.md:307-312`). One suite per process isolates suites, not tests.
**The problem:** ST-7 and acceptance criterion 4 require `make asan` to exit 0. The proposed target can randomly fail or hang for the same environmental reason the plan is trying to avoid, so it is not a reliable production gate.
**Options:**
- **A.** Add `--test-threads=1` (and keep the existing temp-file capture) to each suite invocation; measure the 900 s cap against the largest suite.
- **B.** Keep per-suite isolation but add a bounded retry/quarantine policy and record the flake as an accepted gate exception.
- **C.** Fix the underlying LSan child-run hang and keep parallel tests.

**Recommendation:** Option A. It directly matches the recorded failure mode and is a one-line change; Option C is a separate investigation with no current repro.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Test-execution mechanism within the approved verification scope.

---

### PF-005: The deferred AR-12 decision makes the 47/47 completion rule impossible 🟠 MAJOR

**Dimension:** 3 Logical Contradictions (also 5 Dependency Issues, 11 Ordering)
**Location:** `99-execution-plan.md:224-225,294`; `01-requirements.md:45-47,120,131-132`; `00-ambiguity-register.md:19`
**Codebase evidence:** AR-12 is `⏸ Deferred` with the trigger "before the release-engineering phase or the first public release". Task 6.1.5 says "Resolve AR-12 with the user"; 6.1.6 is explicitly "blocked until 6.1.5"; success criterion 1 and AC1 require all 47 tasks complete. The plan also says it can close "production-ready but not released" when AR-12 remains deferred.
**The problem:** The two outcomes cannot both be true. Either AR-12 must be resolved before Phase 6 (contradicting the approved deferral), or the plan closes with unfinished tasks (contradicting 47/47 and the `Done` lifecycle). The revisit trigger also points at "before the release-engineering phase", while the resolution task sits inside Phase 6.
**Options:**
- **A.** Split Phase 6 into always-done release readiness (licence, metadata, known limitations, checklist) and a conditional publish sub-plan outside the 47; define a `Blocked`-release close-out that still counts the always-done tasks.
- **B.** Require AR-12 resolution before Phase 6 starts.
- **C.** Allow `[!]` tasks in the close-out and treat the plan as Done with a blocked release.

**Recommendation:** Option A. It preserves the approved deferral, keeps the licence/metadata work unblocked, and makes the completion count honest. Update the AR-12 revisit trigger to "before the conditional publish sub-plan".
**Decision:** **Pending — reserved.** Acceptance criteria and release scope are user authority.

---

### PF-006: AR-13 has no resolution gate before Phase 5; the closure rule only covers AR-12 🟠 MAJOR

**Dimension:** 5 Dependency Issues (also 11 Ordering)
**Location:** `99-execution-plan.md:194`; `00-ambiguity-register.md:20,62-65`; `03-04-dx-and-acceptance.md:104`; `01-requirements.md:40-42,127`
**Codebase evidence:** AR-13 is deferred ("the two programme selections", revisit "before the PLAN-13 plan is executed"). Task 5.1.1 requires recording the two names and the licence audit, but no earlier task resolves AR-13. `03-04` says "Phase 5 stays blocked; record the AR-13 deferral"; success criterion 5 and AC8 require PLAN-13 Done.
**The problem:** Phase 5 cannot start without a user selection, but the completion rule mentions only AR-12. If AR-13 remains deferred, the plan is blocked; if the user defers it again, the plan should not claim 47/47 or PLAN-13 Done. The current text leaves both interpretations open.
**Options:**
- **A.** Add an explicit AR-13 resolution gate before 5.1.1; if the user defers, mark Phase 5 blocked and exclude it from Done.
- **B.** Make Phase 5 conditional and outside the 47-task count.
- **C.** Require AR-13 resolution at programme start.

**Recommendation:** Option A. It makes the gate explicit, keeps the selection with the user, and closes the same loophole as PF-005.
**Decision:** **Pending — reserved.** The programme selection is a user decision.

---

### PF-007: Phase 5's "fix every compiler gap" is unbounded and conflicts with the no-language-change boundary 🟠 MAJOR

**Dimension:** 10 Scope Creep Indicators (also 6 Feasibility)
**Location:** `99-execution-plan.md:195,202`; `03-04-dx-and-acceptance.md:35-37,103-105`; `00-index.md:20-21`; `01-requirements.md:40-42`
**Codebase evidence:** R7/AR-8 say "every compiler gap they expose is fixed"; 03-04 says "fixed in the same plan or a bounded follow-up"; 99's deliverable says "fixed or explicitly filed"; Phase 5's objective says "fix every compiler gap". The index says the programme does not reopen the language surface; 03-04 says "no language feature".
**The problem:** "Every gap" is open-ended and can include a missing language feature, which would reopen the surface the programme promised not to change. The four formulations also disagree on whether a filed gap blocks PLAN-13 Done.
**Options:**
- **A.** Bound Phase 5 to implementation, porting, and FFI/wrapper gaps; stop and ask on any language-surface change; require fixes or named user-approved exceptions before PLAN-13 Done.
- **B.** Fix every gap, accepting that the language surface may expand.
- **C.** Allow gaps to be filed and leave PLAN-13 Done even when an accepted program still fails.

**Recommendation:** Option A. It preserves the bounded acceptance slice and the no-language-change promise while still requiring the two programs to work.
**Decision:** **Pending — reserved.** This changes acceptance scope and the meaning of PLAN-13 Done.

---

### PF-008: Phase 1 omits the emission-integrity reviewer required by the change surface 🟠 MAJOR

**Dimension:** 12 Consistency (also 13 Codebase Alignment)
**Location:** `03-01-residual-hardening.md:59,73-74`; `99-execution-plan.md:55-59,87`; `AGENTS.md` §CodeOps "Phase-review dispatch follows the change surface"
**Codebase evidence:** Phase 1 modifies `compiler/src/codegen/program.rs` (initializer retention and projection rebinding) and `compiler/src/own/lower.rs` (ownerless projection binding). `AGENTS.md` requires "correctness reviewer + read-only memory-safety auditor + read-only emission-integrity auditor" for IR/codegen/symbol-name changes, plus the `spec/17-codegen` ABI lens where lowering changes. The plan dispatches only correctness, memory-safety, and semantics.
**The problem:** The phase review would miss the base/head corpus emission diff and the strict C build hunt that the repository's own quality policy requires. Emission changes are exactly where an accepted source can start emitting invalid C or a changed shape.
**Options:**
- **A.** Add the emission-integrity auditor, with the base/head corpus emission diff and strict C build, and add the `spec/17-codegen` ABI lens to the semantics review.
- **B.** Fold emission checks into the semantics reviewer.
- **C.** Keep the three reviewers.

**Recommendation:** Option A. It follows the documented dispatch rule and covers the actual change surface.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Review-dispatch mechanism required by the repository policy.

---

### PF-009: Phase 1's Verify can pass without running ASan 🟠 MAJOR

**Dimension:** 7 Testability (also 11 Ordering)
**Location:** `99-execution-plan.md:67,78,95`; `07-testing-strategy.md:19`; `03-01-residual-hardening.md:125`
**Codebase evidence:** The residual suite helpers choose ASan only when `RDC_ASAN` is set; otherwise they run an ordinary strict host build (for example `compiler/tests/residual_defect_completion_impl_test.rs:259-275`, `compiler/tests/def_backlog_closeout_spec_test.rs:267-275`). The Phase 1 Verify line is `cargo test --test v1_production_readiness_spec_test` and `..._impl_test` with no `RDC_ASAN`. Task 1.3.2 is a separate manual evidence step, not the green gate.
**The problem:** Task 1.2.5 can turn green while the memory cases still leak, because the leak assertions are skipped in a plain build. The red-phase task 1.1.2 also claims "leak or fail under ASan" without setting the variable.
**Options:**
- **A.** Put `RDC_ASAN=1` and `--test-threads=1` in the 1.1.2, 1.2.5, and Phase 1 Verify commands; keep ST-6 as the non-ASan control.
- **B.** Rely on task 1.3.2 and leave the phase Verify fast.
- **C.** Add a separate ASan-only command for the new suite.

**Recommendation:** Option A. The memory cases are the point of the phase; the gate must run them under the sanitizer.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Test-command correction inside the approved residual work.

---

### PF-010: Phase 2 freezes `spec/14-diagnostics.md`, then the DX plan edits it in Phase 4 🟠 MAJOR

**Dimension:** 11 Ordering & Sequencing (also 12 Consistency)
**Location:** `99-execution-plan.md:114,170`; `03-02-spec-stabilization.md:32-33`; `plans/clorraine-dx/99-execution-plan.md:79`
**Codebase evidence:** Phase 2 task 2.1.5 marks `spec/14-diagnostics.md` `stable (v1.0)`. Phase 4 executes the existing DX plan unchanged; its task 1.2.1 adds twelve approved help cells to the same file (`plans/clorraine-dx/99-execution-plan.md:79`). The production plan's Phase 4 verification is `make check`, which runs `spec/check.sh` but does not re-record the stable review.
**The problem:** The "stable" status would be recorded before a post-freeze edit to the normative diagnostic registry, so the release's spec is not actually frozen at the claimed revision. The DX edit is authorized and non-semantic, but the audit trail no longer matches the text.
**Options:**
- **A.** Re-review `spec/14-diagnostics.md` after Phase 4, run `./spec/check.sh`, and re-record its stable status.
- **B.** Move the twelve help cells into Phase 2 task 2.1.5 (a cross-plan edit to the DX plan).
- **C.** Accept the post-freeze edit and note it in the release record.

**Recommendation:** Option A. It preserves the DX plan's ownership and the phase graph; Option B is cleaner in theory but violates "do not edit its tasks".
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Sequencing and verification correction inside the approved plan.

---

### PF-011: Phase 2 links `docs/known-limitations.md` before Phase 6 creates it 🟠 MAJOR

**Dimension:** 5 Dependency Issues (also 11 Ordering)
**Location:** `03-02-spec-stabilization.md:36-37,64`; `99-execution-plan.md:117,222`; `spec/check.sh:29-30`
**Codebase evidence:** `03-02` says "Link `docs/known-limitations.md` from the README and the spec's limitations section where appropriate." The file is created in Phase 6 task 6.1.3. `spec/check.sh` resolves every relative markdown link in `spec/*.md` and fails on a missing target. `03-02` also says the document is owned by 03-05.
**The problem:** If Phase 2 adds the spec link as written, `./spec/check.sh` fails and Phase 2 cannot close. If it defers the spec link, the proposed change is not actually implemented in the phase that claims it. The ownership split (03-01 proposes the content, 03-02 links it, 03-05 owns it) makes the dependency hard to see.
**Options:**
- **A.** Defer the spec link to Phase 6 and re-run `./spec/check.sh` there; add the README link only after the file exists.
- **B.** Create the real document after Phase 1 (when the residual facts are known) and link it in Phase 2; keep 03-05 as the content owner.
- **C.** Drop the spec link and keep only the README link.

**Recommendation:** Option A. It is the smallest change and does not create a stub or move ownership. Option B is acceptable if the user wants the stable spec to carry the link.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Ordering correction inside the approved documentation work.

---

### PF-012: The CI workflow is not least-privilege despite the security requirement 🟠 MAJOR

**Dimension:** 8 Security Blind Spots (also 13 Codebase Alignment)
**Location:** `03-03-verification-infrastructure.md:119-135`; `01-requirements.md:85-86`; `99-execution-plan.md:306`
**Codebase evidence:** The proposed YAML has no `permissions:` block, uses `actions/checkout@v4` by mutable tag, leaves `persist-credentials` at its default, and has no `timeout-minutes` or `concurrency` group. `01-requirements.md:85` requires "least privilege, no repository secrets". The workflow runs untrusted pull-request test code.
**The problem:** The repository's default token permissions apply to the workflow, the token is persisted in the checkout's git config by default, and a compromised or runaway job can hold a runner for the default timeout. This contradicts the plan's own security requirement and the coding standards' least-privilege rule.
**Options:**
- **A.** Add `permissions: contents: read`, pin checkout by commit SHA, set `persist-credentials: false`, add `timeout-minutes: 30` and a concurrency group.
- **B.** Keep the draft YAML; the workflow only runs tests.
- **C.** Use a reusable workflow with the repository's standard hardened settings.

**Recommendation:** Option A. It is a small, reviewable workflow change and directly satisfies the stated requirement. SHA pinning adds maintenance; use Dependabot or a comment if the project does not want that churn.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Security mechanism inside the approved CI scope.

---

### PF-013: The release artifact would still report version 0.1.0 🟠 MAJOR

**Dimension:** 4 Completeness Gaps (also 13 Codebase Alignment)
**Location:** `compiler/Cargo.toml:2-6`; `03-05-release-engineering.md:72-80`; `01-requirements.md:58-59`; `99-execution-plan.md:225`
**Codebase evidence:** `clc --version` prints `clc 0.1.0` and `clorraine --version` prints `clorraine 0.1.0`; both read the package version. The plan's Cargo metadata example keeps `version = "0.1.0"` and no task updates it. Task 6.1.5 only "records" the AR-12 version; the smoke test runs `--version` but does not assert the release version.
**The problem:** If AR-12 resolves to v1.0.0, the source tarball and both binaries would identify themselves as 0.1.0. The release identity is wrong and the smoke test would not catch it.
**Options:**
- **A.** Set the Cargo package version from the AR-12 decision in 6.1.5 and assert the exact `--version` output in the smoke test.
- **B.** Leave 0.1.0 and document the mismatch as an accepted release limitation.
- **C.** Inject the version at build time from a tag.

**Recommendation:** Option A. It is the smallest correct fix and keeps one version of record. The exact value remains reserved under AR-12.
**Decision:** **Pending — reserved.** The release version and artifact format are the user's AR-12 decision; the test assertion can be added once it is known.

---

### PF-014: The x86_64/aarch64 release matrix has no aarch64 build or smoke path 🟠 MAJOR

**Dimension:** 5 Dependency Issues (also 6 Feasibility)
**Location:** `01-requirements.md:58-59`; `03-05-release-engineering.md:96`; `07-testing-strategy.md:95`; `03-03-verification-infrastructure.md:119-135`
**Codebase evidence:** AR-3 and R13 promise source plus Linux x86_64 and aarch64 host binaries. The pinned SDK image installs only the host C toolchain and the m68k cross toolchain; there is no `aarch64-unknown-linux-gnu` Rust target, cross linker, qemu, or arm64 runner. The only CI job is `runs-on: ubuntu-latest` (x86_64). No task names the two architectures.
**The problem:** The release checklist cannot be executed as written. Adding a cross toolchain, qemu, or native runner is a new support surface and a complexity escalation, so it cannot be silently assumed.
**Options:**
- **A.** Add an approved cross-build path (`rustup target add aarch64-unknown-linux-gnu`, a cross linker, and a qemu-user `--version`/emit smoke) or a native arm64 runner.
- **B.** Narrow AR-3/R13 to source plus x86_64 with an explicit user ruling.
- **C.** Leave the architecture set to AR-12 and remove the aarch64 claim until then.

**Recommendation:** Option B is the smallest honest release if native aarch64 testing is unavailable. Option A is required if the two-architecture promise is kept; it needs explicit complexity approval. Option C postpones the decision but still leaves the acceptance criteria inconsistent.
**Decision:** **Pending — reserved.** This is a complexity escalation and a release-scope choice.

---

### PF-015: `make differential` adds no new signal beyond the existing corpus legs 🟡 MINOR

**Dimension:** 13 Codebase Alignment (redundancy)
**Location:** `03-03-verification-infrastructure.md:8-11,33-41`; `01-requirements.md:32-34`; `07-testing-strategy.md:44`
**Codebase evidence:** The host corpus test asserts stdout equals `expected-output.txt` for `buffer-container`, `file-copier`, and `string-parser` (`compiler/tests/corpus_spec_test.rs:104-118`). `docker/scripts/corpus-amiga.sh` asserts m68k stdout equals the same files (`:52-69`). If both existing legs pass, host output equals m68k output by transitivity; the new diff cannot detect a divergence the existing legs miss.
**The problem:** The plan justifies the target as a missing "host-vs-m68k differential signal", but it adds a third run over the same pinned outputs. It is still useful as a one-command convenience gate; the rationale is what is wrong.
**Options:**
- **A.** Keep the target as a convenience gate, correct the rationale, and state that it is not a new signal.
- **B.** Drop the target and reopen AR-10/R4.
- **C.** Change the seed to a program/input not already pinned on both profiles, with a new AR-10 decision.

**Recommendation:** Option A. It preserves the approved AR-10 scope and makes the document truthful. Option C is the only way to get a genuinely independent signal, but it reopens an approved decision.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Documentation correction inside the approved AR-10 scope; the seed remains unchanged.

---

### PF-016: The differential negative control cannot run against the proposed script, and the script is not robust for ST-8 🟡 MINOR

**Dimension:** 7 Testability (also 9 Edge Cases)
**Location:** `07-testing-strategy.md:45`; `03-03-verification-infrastructure.md:82-110,161,173-174`
**Codebase evidence:** ST-9 says a throwaway copy of a host output is corrupted, "then the differential diff runs". The proposed script always regenerates both `out.txt` files immediately before `diff`, so the corruption is overwritten; there is no diff-only or host-output override. The script also lacks the `[ -x "${CLC}" ]` guard, `rm -rf` clean, and `cargo build` step that `corpus-amiga.sh` uses (`docker/scripts/corpus-amiga.sh:39-49`), and it does not normalise the host side's line endings.
**The problem:** The negative control cannot exercise the script's failure path, and a stale or missing `target/debug/clc` can produce a false green or a confusing failure on a "clean tree". The "one negative control per script" goal also has no `asan-suite.sh` control.
**Options:**
- **A.** Add a build/guard/clean step and a `DIFF_ONLY` or host-output override seam; make ST-9 use it, and add a failing-suite/timeout control for `asan-suite.sh`.
- **B.** Make ST-9 a manual `diff` outside the script and accept that the script's failure path is untested.
- **C.** Drop the negative controls.

**Recommendation:** Option A. It makes the control real and aligns the script with the existing corpus script's robustness.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Test-harness and script robustness inside the approved verification work.

---

### PF-017: `docs/known-limitations.md` has conflicting content and ownership, and the multi-element leak is only conditional 🟡 MINOR

**Dimension:** 12 Consistency (also 4 Completeness)
**Location:** `00-ambiguity-register.md:81-83`; `03-01-residual-hardening.md:48-50`; `03-02-spec-stabilization.md:64`; `03-05-release-engineering.md:33-38`; `07-testing-strategy.md:54`
**Codebase evidence:** AR-14 lists the ASan child-run flake and the top-level-`while` loop split; 03-01 adds "any base-identical shapes outside the release-blocking list"; 03-05 adds "the multi-element view-array initializer's unselected read-root leak, if it remains" and the host-only CI limitation; ST-13 requires "any remaining base-identical view-array leak". 03-01 says the document is written in Phase 1; 03-02 says 03-05 owns it.
**The problem:** The release document can be written with different contents depending on which section is read, and the conditional "if it remains" leaves an accepted leak out of the release record. The multi-element leak is in the ledger (`plans/def-zero-closeout/05-findings.md:295-298`) and is not in AR-4's fix list.
**Options:**
- **A.** Make 03-05 the sole owner; pin one complete list (ASan flake, loop split, multi-element view-array leak, host-only CI limitation) and remove "any remaining"; 03-01 supplies the verified facts.
- **B.** Split ownership between 03-01 and 03-05 with a cross-reference.
- **C.** Leave the conditional wording and let the release phase decide.

**Recommendation:** Option A. It gives the release one authoritative list and does not claim the view-array class is fully closed.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Documentation ownership and completeness; the accepted-risk policy is already AR-4/AR-14.

---

### PF-018: The stabilisation tasks do not define the status of the grammar, lineage, and preflight-report anchors 🟡 MINOR

**Dimension:** 4 Completeness Gaps (also 12 Consistency)
**Location:** `03-02-spec-stabilization.md:44-50`; `99-execution-plan.md:115`; `spec/README.md:63-80`
**Codebase evidence:** `03-02`'s chapter groups include `spec/grammar-deltas.md` and `spec/lineage.md`; task 2.1.6 names only `spec/grammar.ebnf.md`, `spec/decisions.md`, and `spec/README.md`. `spec/preflight-report.md` also has a `> **Status:**` line and is checked by `spec/check.sh`. The README table still shows the grammar/lineage rows as `draft`, and the anchors' own status lines are `draft (Phase B anchor)`.
**The problem:** "Stable v1.0 specification" is ambiguous for the three anchor files. They are neither reviewed by a named task nor explicitly excluded, so the final status scan can pass while the README shows draft rows in a supposedly stable set.
**Options:**
- **A.** Add a per-file target-status table: chapters and appendices `stable (v1.0)`; anchors keep their `Phase B anchor`/`passed with notes` status and are explicitly out of the chapter freeze; update the README rows and include every status-bearing file in the final scan.
- **B.** Mark every spec file stable, including the anchors.
- **C.** Exclude the anchors from the README status table entirely.

**Recommendation:** Option A. It preserves the anchor markers' meaning and makes the freeze boundary explicit.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Documentation status bookkeeping within AR-6.

---

### PF-019: "CI is green" has no task that triggers or records a workflow run 🟡 MINOR

**Dimension:** 4 Completeness Gaps (also 7 Testability)
**Location:** `01-requirements.md:128`; `99-execution-plan.md:145,254`; `03-03-verification-infrastructure.md:42-47`
**Codebase evidence:** Task 3.1.5 only "validates its two commands locally". Phase 7 verifies "CI is green" but no task pushes a branch, opens a pull request, or records a workflow run. The repository has an `origin` remote and `main` is the integration branch.
**The problem:** The final gate cannot evidence a green workflow unless an external push happens; a local command run is not a CI run. The requirement is untestable as written.
**Options:**
- **A.** Add a task to push/trigger the workflow on the final commit and record the run URL and conclusion; include the run ID in the final evidence.
- **B.** Redefine acceptance criterion 9 as "workflow file validated and both commands green locally", and note that the first real CI run happens on the release push.
- **C.** Defer CI-green evidence to the first release.

**Recommendation:** Option A if the user authorizes the external push; otherwise Option B with a corrected acceptance criterion. Since the push is an external action, the plan should not assume it silently.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Evidence/acceptance wording; the actual push/PR remains a user-authorized external action.

---

### PF-020: The FS-UAE/X/ROM dependency is not in the plan's dependency or preflight tasks 🟡 MINOR

**Dimension:** 5 Dependency Issues (also 11 Ordering)
**Location:** `02-current-state.md:129-133`; `99-execution-plan.md:171,196,250`; `Makefile:19-21,87-91`
**Codebase evidence:** `make verify` runs `corpus-fs-uae`, which needs the SDK X display, FS-UAE, and local Kickstart ROMs; `docker/.env` is local and gitignored. The external-dependencies list in 02 names only the Docker SDK, GitHub Actions, and `cc`. Phases 4, 5, and 7 call `make verify`.
**The problem:** A phase can fail for an environment reason the plan never preflights, and the failure looks like a code failure. The plan should either check the environment or use a narrower acceptance command for non-release phases.
**Options:**
- **A.** Add an environment preflight task (FS-UAE, X display, `docker/.env`, ROM path) and fail with a clear message.
- **B.** Use `make corpus-amiga` + `make run-fault-amiga` for non-release phases and reserve `make verify` for the final gate.
- **C.** Assume the environment and document the assumption.

**Recommendation:** Option A, with Option B as the fallback for intermediate phases. It turns an environmental surprise into a named blocker.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Dependency/readiness check inside the approved plan.

---

### PF-021: The MIT release would leave `tools/demo/package.json` marked `UNLICENSED` 🟡 MINOR

**Dimension:** 8 Security Blind Spots (compliance)
**Location:** `03-05-release-engineering.md:29-32`; `01-requirements.md:43-44`; `tools/demo/package.json:24`
**Codebase evidence:** `tools/demo/package.json` has `"license": "UNLICENSED"`. The plan's licence scope covers the root `LICENSE`, `compiler/Cargo.toml`, and `README.md` only. The demo package is part of the repository and the source release.
**The problem:** A user or distributor reading the package metadata sees a proprietary licence inside an MIT-licensed source tree. The root LICENSE may cover it legally, but the manifest contradicts the release claim.
**Options:**
- **A.** Set `"license": "MIT"` in `tools/demo/package.json` and add a manifest licence audit to the release checklist.
- **B.** Exclude `tools/demo/` from the source release and say so.
- **C.** Leave it and document the exception.

**Recommendation:** Option A if the demo package is part of the release; Option B if it is not. Either way, the release checklist should audit every manifest, not only the compiler package.
**Decision:** **Pending — reserved.** Licence scope and legal text are user authority.

---

### PF-022: R12's release notes and changelog have no owning task 🟡 MINOR

**Dimension:** 4 Completeness Gaps
**Location:** `01-requirements.md:56-57`; `03-05-release-engineering.md:45-46`; `99-execution-plan.md:225`
**Codebase evidence:** R12 requires "concise v1.0 release notes and a changelog entry" once AR-12 resolves. `03-05` mentions release notes but not a changelog; task 6.1.6 only says "record checksums and notes"; the repository has no `CHANGELOG`.
**The problem:** A Should-Have requirement has no executable task, so it can be silently dropped at release time.
**Options:**
- **A.** Add conditional tasks under 6.1.6 for a release-notes file and a `CHANGELOG.md` entry.
- **B.** Remove R12 from the requirements.
- **C.** Defer both to the first post-release patch.

**Recommendation:** Option A. The work is small and belongs with the release artifact.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Release-documentation task completion within R12.

---

### PF-023: Several references are stale or phantom 🟡 MINOR

**Dimension:** 13 Codebase Alignment
**Location:** `07-testing-strategy.md:70`; `02-current-state.md:68-70`; `00-ambiguity-register.md:73-75`; `02-current-state.md:22-24`; `01-requirements.md:64`; `03-05-release-engineering.md:22`
**Codebase evidence:** `compiler/tests/dx_tooling_spec_test.rs` does not exist (the DX plan is 0/28 and names it as new). `spec/tests/corpus/freestanding/program.clo` compiles under `--profile host` (exit 0), so the "only three corpus programs' imports compile on both profiles" claim is false; the real reason to exclude `freestanding` is that it is a freestanding-profile program. The roadmap citations are off by one: the v1.0-core sentence is line 589 (cited 588), the deferred-feature row is 594 (cited 593), and the licence-blocker sentence is 599 (cited 598).
**The problem:** The testing table promises a suite that does not exist yet, the differential-seed evidence is wrong, and four citations point at blank lines. These are small but they weaken the plan's evidence trail.
**Options:**
- **A.** Correct the references: mark the DX suite "created by the DX plan", state the real exclusion reason for `freestanding`, and fix the four line numbers.
- **B.** Leave them; they do not block execution.

**Recommendation:** Option A. It is a short edit and keeps the audit trail trustworthy.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Documentation correction within the existing scope.

---

### PF-024: ST-6, ST-11, ST-13, and ST-16 are not falsifiable as written 🟡 MINOR

**Dimension:** 7 Testability
**Location:** `07-testing-strategy.md:37,52,54,57`
**Codebase evidence:** ST-6 names "accepted controls" and "existing fixed-array and slice initializer forms" without programs or exit codes. ST-11 says "every v1.0 chapter" without a file list. ST-13 says "any remaining base-identical view-array leak". ST-16 says "recorded output" but the PLAN-13 plan that produces that output is itself new and its programs are deferred under AR-13. Repository spec tests use self-contained `main.clo` snippets with exact exit codes.
**The problem:** The cases cannot be implemented as written without the author inventing the oracle. ST-16 in particular risks testing the new plan against output produced by the same plan, with no independent expectation.
**Options:**
- **A.** Add exact programs, expected exit codes, and a fixed file list; for ST-16, require an independent expected-output oracle (the program's documented behavior or a hand-checked transcription) before the run.
- **B.** Accept prose and let the executor choose.
- **C.** Move the non-falsifiable cases to a manual release checklist.

**Recommendation:** Option A for ST-6/ST-11/ST-13; for ST-16, an independent oracle is mandatory if the output is part of the acceptance gate.
**Decision:** **Resolved — AI — delegated by `--auto-design`.** Test-oracle completion inside the approved testing strategy.

---

### PF-025: The 47-task count understates the DX and PLAN-13 execution work 🔵 OBSERVATION

**Dimension:** 10 Scope Creep Indicators (estimation)
**Location:** `99-execution-plan.md:22-33,159-206`; `03-04-dx-and-acceptance.md:17-19`
**Codebase evidence:** Phase 4 is three tasks but means executing a 28-task DX plan; Phase 5 is four tasks but means creating and executing an entirely new PLAN-13 plan. The progress header reads `0/47`, which makes the programme look closer to done than the delegated work implies.
**The problem:** The count is honest as a count of *this* plan's checkboxes but not as a measure of remaining work. It can mislead the final gate and the roadmap.
**Recommendation:** Mark Phases 4 and 5 as plan-gated and show their own progress (DX 0/28; PLAN-13 not created) next to the 47-task count. No change to the task definitions is required.
**Decision:** Acknowledged as an observation; no blocking action.

---

## Delegated Resolution Provenance

The following block applies to every finding marked **Resolved — AI — delegated by `--auto-design`**.

```text
Authority: AI — delegated by --auto-design
Eligibility: technical test design, sequencing, review dispatch, CI hardening, or
  documentation correction that stays inside the confirmed v1.0 readiness scope;
  no product behavior, acceptance criteria, legal text, release format, or
  complexity escalation.
Objective: make the v1.0 production-readiness plan executable, testable, and
  truthful about the frozen language and the real codebase.
Policy version: 1
Root invocation ID: preflight-v1-production-readiness-20261031T2117Z
```

Per-finding provenance:

| Finding | Decision | Evidence | Rejected alternatives | Strongest counterargument | Confidence | Reopen triggers |
|---|---|---|---|---|---|---|
| PF-002 | Rewrite ST-2/ST-5 on frozen-rule-valid shapes; drop an item if no red leak reproduces | AL-1 E10215; OW-12 E10026; probes | Change AL-1/OW-12 (reserved); delete the items (loses a user-selected residual) | Re-baselining can accidentally encode current behavior as the oracle; derive expectations from the spec, use HEAD only to confirm red | High | A user ruling changes AL-1 or OW-12 |
| PF-003 | Re-baseline on the 11 B field-call-root leak; keep clean ST-3 as a control; add direct and func-typed twins | ASan probe; `program.rs:1133-1430` | Keep the current green cases; expand to all initializer field roots | The field-call-root shape is not the array-element shape the plan named; confirm the fix covers both or narrow the claim | High | A red probe shows the original nested-row/take shape still leaks |
| PF-004 | Add `--test-threads=1` per suite, keep temp-file capture | closeout flake; no `--test-threads` in repo | Retry/quarantine; fix LSan hang | A 230-test serial suite may exceed 900 s; measure and raise the bound with a recorded reason | High | Serial runs still flake; the 900 s cap is exceeded |
| PF-008 | Add emission-integrity auditor + base/head diff + strict C build; add `spec/17-codegen` ABI lens | AGENTS.md dispatch rule; changed paths | Fold into semantics; keep three | Extra review cost; the change is small | High | The phase changes no codegen/IR file |
| PF-009 | Add `RDC_ASAN=1` and serial threads to 1.1.2, 1.2.5, and Phase 1 Verify | helper fallback; Verify line | Rely on 1.3.2; separate ASan command | The ASan run is slower and flaky; that is the accepted risk PF-004 addresses | High | The suite runs only on a sanitizer-free host by design |
| PF-010 | Re-review Ch14 after DX and re-record stable status | DX 1.2.1; Phase 2 2.1.5 | Move the 12 cells into Phase 2; accept post-freeze edit | The freeze is weaker if it is re-recorded; but the DX edit is authorized and non-semantic | High | The DX plan changes diagnostics semantics, not just help cells |
| PF-011 | Defer the spec link to Phase 6 and rerun `spec/check.sh` | `spec/check.sh:29-30`; file created in 6.1.3 | Create a stub early; drop the link | The stable spec temporarily lacks the cross-reference | High | The user requires the link in the stable spec |
| PF-012 | Add `permissions: contents: read`, SHA-pinned checkout, `persist-credentials: false`, timeout, concurrency | plan's own least-privilege requirement; YAML | Keep defaults; reusable workflow | SHA pinning adds maintenance | High | The workflow needs write permission for a release job (not this one) |
| PF-015 | Keep `make differential` as a convenience gate; correct the rationale; keep AR-10 seed | host/Amiga both diff same golden | Drop target; change seed | The target is still a redundant third run | High | The user reopens AR-10 or wants a true differential signal |
| PF-016 | Add build/guard/clean and a diff-only/fixture seam; add asan negative control | script has no seam; corpus script has build/guard/clean | Manual diff only; drop controls | Extra script surface for a small test | High | The control cannot be made deterministic without a new tool |
| PF-017 | 03-05 sole owner; complete fixed list including the multi-element leak; remove "any remaining" | conflicting lists; ledger 295-298 | Split ownership; leave conditional | The multi-element leak is an accepted residual; listing it is still required | High | The leak is fixed before release or the user changes AR-4 |
| PF-018 | Per-file status table; anchors keep anchor status; include every status-bearing file in the final scan | 03-02 groups vs 2.1.6; README rows | Mark all stable; exclude anchors | The anchor markers are not v1.0 chapter statuses | High | The user wants the anchors inside the stable set |
| PF-019 | Add a task to trigger/record a workflow run, or redefine AC9 as local evidence | 3.1.5 local only; 7.1.6 verifies | Defer to first release | Triggering CI is an external action the plan cannot assume | Med | The CI cannot be triggered before release |
| PF-020 | Add FS-UAE/X/ROM preflight; use narrower commands for non-release phases | Makefile 19-21, 87-91; 02 deps | Assume environment; skip intermediate verify | Extra setup check adds friction | High | The environment is provisioned outside the plan |
| PF-022 | Add release-notes and changelog tasks under 6.1.6 | R12; no CHANGELOG | Remove R12; defer | Small documentation work; belongs to release | High | AR-12 remains deferred and no release is cut |
| PF-023 | Correct the DX-suite label, freestanding evidence, and four roadmap citations | absent file; host probe; roadmap lines | Leave | Low impact but easy to fix | High | The referenced files move |
| PF-024 | Add exact programs/exit codes/file lists; independent oracle for ST-16 | ST text lacks inputs; PLAN-13 output is self-produced | Accept prose; move to manual checklist | The PLAN-13 plan is expected to define its own output; independence is still needed for a gate | High | The PLAN-13 plan supplies a verifiable external oracle |

**Challenger record:** one independent challenger received the CRITICAL/MAJOR batch without the parent's picks. It converged on 12 of 14 decisions. It diverged on PF-010 (preferred moving the 12 help cells into Phase 2; the parent keeps the post-DX re-review because it does not edit the DX plan) and PF-014 (preferred a cross-build/qemu path; the parent keeps the decision reserved because it is a complexity escalation). The parent adopted the challenger's PF-017 recommendation (03-05 sole owner, complete list) and its PF-011 preference (defer the link rather than create a stub). `Hardening: converged with two recorded divergences; no finding was waived.`

---

## Pass/Fail Determination

| Criterion (at scan time) | Result |
|---|---|
| All 13 dimensions scanned | Yes |
| Codebase grounding | Yes — compiler probes, test-tree counts, Makefile/spec/CI checks |
| All 🔴/🟠 resolved | **No at scan time; partial now** — 2 critical + 12 major; 8 critical/major delegated decisions were applied on 2026-11-01, 6 remain reserved (PF-001, PF-005, PF-006, PF-007, PF-013, PF-014) |
| All 🟡 accepted | **No at scan time; partial now** — 10 minor findings; 9 delegated decisions were applied on 2026-11-01, 1 remains reserved (PF-021) |
| Optional additions | None reported (strict scope) |

**Verdict at scan time:** `❌ PREFLIGHT BLOCKED — 14 critical/major unresolved.`

The plan has a strong overall shape and much of its evidence is real: the 81/21 test counts, the Makefile topology, the missing licence/CI/release documents, the 0/28 DX plan, and the 546/546 defect close-out all check out. The blocking problems are concentrated in Phase 1's test oracles, the reliability of the sanitizer gate, the deferred-decision arithmetic, and a small set of release/CI details.

---

## Fix Application (2026-11-01)

**Applied — 17 delegated findings:**

| Finding group | Change made |
|---|---|
| PF-002, PF-003 | Replaced the invalid/mis-targeted residual cases with verified HEAD repros: module-level nested fill, module-level multi-element view array with a `tag` sibling, local initializer field call root (direct and func-typed), and module-level indexed view global. Each was probed at 11 B LSan before the fix. |
| PF-004, PF-009 | `make asan` now runs each suite with `--test-threads=1`; Phase 1's red/green commands set `RDC_ASAN=1` and serial threads. |
| PF-008 | Added `emission-integrity` to Phase 1's lenses and phase review, with the base/head corpus diff and strict C build. |
| PF-010 | Phase 4 now re-reviews `spec/14-diagnostics.md` after the DX help-cell edit and re-records its stable status. |
| PF-011 | The spec link to `docs/known-limitations.md` is deferred to Phase 6; the README link remains in Phase 2. |
| PF-012 | CI YAML now has `permissions: contents: read`, a SHA-pinned checkout, `persist-credentials: false`, a 30-minute timeout, and a concurrency group. |
| PF-015, PF-016 | Differential rationale corrected to "convenience gate"; the script now builds/guards/cleans, normalises both outputs, and supports `DIFF_ONLY=1`; ST-9 uses it; the asan script has a negative control. |
| PF-017, PF-018 | `03-05` is the sole known-limitations owner with a fixed list; the spec target-status table now classifies chapters, appendices, and anchors. |
| PF-019 | CI evidence is explicitly a run URL when a push is authorized, otherwise local command evidence with the deferred run recorded. |
| PF-020 | FS-UAE/X/ROM preflight added to the verify phases and the dependency list. |
| PF-022, PF-023, PF-024 | Release notes/changelog tasks added; stale DX-suite/freestanding/roadmap references corrected; ST-6/ST-11/ST-13/ST-16 have concrete fixtures and an independent-oracle requirement. |

**Reserved rulings — recorded 2026-11-01 (user accepted all recommendations):**

| Finding | Recorded ruling |
|---|---|
| PF-001 | `bool(null)` and `!null` report E10012; `spec/04-expressions.md` records the null-cast exception |
| PF-005 | 45-task main plan plus a 2-task conditional publish sub-plan outside the count |
| PF-006 | Explicit AR-13 user gate before Phase 5 task 5.1.1; the plan records the names and does not choose them |
| PF-007 | Accepted-program gaps block PLAN-13 Done; language-surface gaps stop for a user ruling |
| PF-013 / PF-014 | AR-12 resolved to v1.0.0, source tag + Linux x86_64 tarball + SHA256SUMS; aarch64 release deferred to PLAN-07 |
| PF-021 | `tools/demo/package.json` becomes MIT |

**Iteration 2 re-assessment (independent auditor):** 13 new findings, all fixed. They are newly
numbered PF-026 – PF-038 in the finding sequence:

| New finding | Iteration-2 ID | Severity | Fix |
|---|---|---|---|
| PF-026 | PA2-01 | CRITICAL | Split ST-1 into three independent compilations |
| PF-027 | PA2-02 | MAJOR | Bounded the `03-04` overview to implementation/porting/FFI gaps |
| PF-028 | PA2-03 | MAJOR | Removed the "release format deferred" line; AR-12 is fixed, publication is conditional |
| PF-029 | PA2-04 | MAJOR | Removed the remaining "pending PF-001"/"blocked" text |
| PF-030 | PA2-05 | MAJOR | ST-14 now requires the demo package MIT, with no exclusion alternative |
| PF-031 | PA2-06 | MINOR | `03-01` now says five residual classes |
| PF-032 | PA2-07 | MINOR | Merged the duplicate conditional-publish risk rows |
| PF-033 | PA2-08 | MINOR | Differential script builds `clc` unless `DIFF_ONLY=1` |
| PF-034 | PA2-09 | MINOR | Added `workflow_dispatch` to the CI workflow |
| PF-035 | PA2-10 | MINOR | Phase 7 deliverables require all 45 main-plan tasks complete |
| PF-036 | PA2-11 | MINOR | Pinned the known-limitations list; removed the catch-all |
| PF-037 | PA2-12 | MINOR | Moved the README/spec known-limitations links to Phase 6 |
| PF-038 | PA2-13 | MINOR | Added the exact func-typed ST-4b fixture |

**Iteration 3 scoped verification (independent auditor):** 2 findings, both fixed. They are
PF-039 – PF-040:

| New finding | Iteration-3 ID | Severity | Fix |
|---|---|---|---|
| PF-039 | PA3-01 | MAJOR | PLAN-13 selection is now explicitly the user's Phase 5 gate decision; the new plan records the names |
| PF-040 | PA3-02 | MINOR | Removed the Phase-2 known-limitations-link claim |

No iteration 4 was started: the protocol requires a fresh-session audit or an explicit user
decision after iteration 3. The two iteration-3 fixes were directly verified against the current
text, links, and task counts.

**Post-fix verification performed:**

| Check | Result |
|---|---|
| ST-1 three independent compilations | Control E10012; cast E10201; condition E10218 at base |
| ST-2 nested fill fixture | 11 B LeakSanitizer report at base |
| ST-3 multi-element fixture (with `tag`) | 11 B LeakSanitizer report at base |
| ST-4/ST-4b direct and func-typed field call root | 11 B LeakSanitizer report for each at base |
| ST-5 indexed module view | 11 B LeakSanitizer report at base |
| ST-6 controls (`slice`, simple fill, clean nested literal) | Emits, strict-builds, runs exit 0, no sanitizer report |
| Relative links in the plan | All resolve |
| `./spec/check.sh` | OK — 27 files, 111 compile-fail tests |
| Task count | 45 main (10+9+7+3+4+5+7) + 2 conditional |
| `make check`, `make verify`, `make asan`, `make differential` | Not run in this session: the host has no `cargo`; these remain execution-time gates inside the SDK container |

**Final manifest:** `112e5a59a78910fe433851487883eb2633fc5038fe9a6a3e1c561523822b9aa8` (SHA-256 of the `sha256sum` manifest over the 11 plan files; the report itself is excluded).

**Final status:** `✅ PASSED WITH NOTES — 40 findings resolved; no independent iteration 4 was run because the protocol requires a fresh session or an explicit user decision.` The roadmap advances to `Plan Preflighted`.

---

## Next Steps

1. **Commit the plan and report** with the requested `--auto-commit` mode.
2. **Run a fresh-session audit before execution** if maximum independence is required; no independent iteration 4 was run because the protocol caps automatic iteration at 3.
3. **Execute PLAN-15** through `exec-plan`; Phase 1 starts with the three independent ST-1 compilations and the four 11-byte leak fixtures.
4. **The roadmap is advanced** to `Plan Preflighted`; it moves to `Executing` when the first phase starts.

---

## Appendix: Per-File Hashes at Scan Start

```
f998de40705fe4aa3848decce3435c721247fc8e614e0b9e6f80c8ca3b2a4429  00-ambiguity-register.md
511ccfe1806ba6653d8ee3c683f252405b02f658d38ebc95a8b914ec66338e17  00-index.md
1d679b9f550323aa2e2eb89ff270a4150da79efa76e2a693377ab7045ef136d3  01-requirements.md
944799fa1aeb186f2c1f77ccf2b03689e228552507f38007326aa905a763fab9  02-current-state.md
e4abf95586c1ef08635c42051dfc7f361b593e1581a899ba3f15834682c4a505  03-01-residual-hardening.md
818a7daa279f175745bbbfb8720d619ca633f55110a01332cc8d0df4ba54d95f  03-02-spec-stabilization.md
a95202b2121113c62469077b622ad59262f9d921ad60b1ec42e3a697a0586e59  03-03-verification-infrastructure.md
8e7f5d0030e413f36b4520ad273df9ff5e00bdaf8e77c2043537f0d17ab90461  03-04-dx-and-acceptance.md
401c043ab957412132697912848371e4aa03fe53f0b2608f8dfd08e15c6890f9  03-05-release-engineering.md
faedb83abfdb2a1b6bca4124bd39bbc50c58504b4f900ca42bbcea4c61773368  07-testing-strategy.md
baa9c22a5c671ecb3edd92158ef737614043e2b74c0d8494bae83a498687a9e2  99-execution-plan.md
```

## Appendix: Per-File Hashes After Fix Application

```
ecc05c913f85ece511a21ee5b3ccadec0eeb3b4362bb798e891bb2a0cc9c4431  00-ambiguity-register.md
a823e889a5375f6c94b202bab124697c5161694b19ff285dfa0d04ecd4647fd1  00-index.md
7c73d6c0b7396e553f38eaf0fed0d9a0fd0b56a94a1e5d5c77be103f9d5b75bf  01-requirements.md
7e0843ef452e5286ec2269a8b63aa1229334bf7b320a8a193f37e8119a9237a1  02-current-state.md
b51a6c7eb92b519c66106a417b6a2fb1cfa3d98ad9b0a030601ccfb5d9eaf830  03-01-residual-hardening.md
686de615091832de23e021e71719006a3b7b0c52b6db7c75a2f6eb670e19d370  03-02-spec-stabilization.md
d5bce5a4a9d00d8e58e22581187f6e7e26b3724e1f3efb059db1db4e3fa47652  03-03-verification-infrastructure.md
e9144bc6bb93241f4557a3fa09f7c78a3b6088f2c58934932929cb48708bfcd7  03-04-dx-and-acceptance.md
92d459c62ea7476a2158e1b332659ed5363c7b7b05d07f1300375f632ebc04cf  03-05-release-engineering.md
b8f4b70ce77c6c83f6d75739370571e33ca4ec5467cb8974ba1698c181a06187  07-testing-strategy.md
b907b6e04b0c5b53ef2976105014665997ccc81c314af8f63acfd59f403fb780  99-execution-plan.md
```

Post-fix manifest: `112e5a59a78910fe433851487883eb2633fc5038fe9a6a3e1c561523822b9aa8`.
