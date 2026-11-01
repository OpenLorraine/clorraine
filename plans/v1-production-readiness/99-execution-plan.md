# Execution Plan: Clorraine v1.0 Production Readiness

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-01 00:45
> **Progress**: 0/45 tasks (0%); 0/2 conditional publish tasks
> **CodeOps Artifact Schema**: 1

## Overview

Seven phases close the production gap: residual hardening, specification stabilisation,
verification infrastructure, the existing DX plan, the bounded PLAN-13 acceptance slice, release
readiness, and the final production gate. The residual phase is specification-first; the DX and
PLAN-13 phases execute their own plans without restating their tasks.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Release-blocking residual hardening | 10 |
| 2 | Stable v1.0 specification | 9 |
| 3 | Verification infrastructure | 7 |
| 4 | DX slice execution | 3 |
| 5 | PLAN-13 acceptance slice | 4 |
| 6 | Release readiness | 5 |
| 7 | Final production gate | 7 |

**Total: 45 tasks across 7 phases**, plus a **2-task conditional publish sub-plan** that is
counted separately (no fabricated hour estimates — scope is bounded by the task-size criteria in
the make-plan quality checklist).

Phases 4 and 5 are **plan-gated**: Phase 4 executes the 28-task DX plan, and Phase 5 creates and
executes a new PLAN-13 plan. The 45 count is this plan's own checkboxes only; the delegated plans
report their own progress.

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated
>    stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Release-blocking residual hardening

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, memory-safety, emission-integrity, semantics
> **Reasoning**: high — the change touches ownership, codegen, release counts, and a frozen diagnostic

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-residual-hardening.md) §Proposed Changes · [AR-4](00-ambiguity-register.md)
**Objective**: Pin the reproduced residual cases before any implementation. ST-1 is three
independent compilations; the other cases use the exact ST-2 – ST-6 fixtures in
`07-testing-strategy.md`.

- [ ] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-6 using the exact fixtures — `compiler/tests/v1_production_readiness_spec_test.rs`
- [ ] 1.1.2 Run the spec suite with `RDC_ASAN=1` and `--test-threads=1`, and record the red phase (expected: ST-1 control reports E10012, cast reports E10201, condition reports E10218; ST-2 – ST-4 each leak 11 B at base; ST-5 leaks 11 B at base; ST-6 passes as the control)

### Step 1.2: Implementation

**Reference**: [03-01](03-01-residual-hardening.md) §Proposed Changes
**Objective**: Apply the null rule, extend the initializer retention walk, and carry the run-time index through the projection rebind.

- [ ] 1.2.1 Report E10012 for `bool(null)`/`!null`; keep `let b: bool = null;` as the accepted control — `compiler/src/sema/expr.rs`
- [ ] 1.2.2 Recurse into nested fill rows and retain every nested fill root — `compiler/src/codegen/program.rs`
- [ ] 1.2.3 Retain every multi-element view-array root and bind the whole initializer call root for field/element projections — `compiler/src/codegen/program.rs`
- [ ] 1.2.4 Carry the run-time index through the module-initializer view-root binding — `compiler/src/codegen/program.rs`, `compiler/src/own/lower.rs`
- [ ] 1.2.5 Run the spec suite with `RDC_ASAN=1` and `--test-threads=1` to green; on any failure STOP and fix the implementation, never the test

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-residual-hardening.md) §Testing Requirements · [07](07-testing-strategy.md) ST-1 – ST-6
**Objective**: Pin the release counts and emitted shapes, probe the fixed memory paths, and record the phase review.

- [ ] 1.3.1 Write the implementation tests (root binding, release counts, index carry-through, nested-array recursion, byte-identity where pinned) — `compiler/tests/v1_production_readiness_impl_test.rs`
- [ ] 1.3.2 Run every new memory case under `RDC_ASAN=1` with `--test-threads=1` and a bounded timeout; record the evidence
- [ ] 1.3.3 Phase review (correctness, memory-safety, emission-integrity, semantics), record findings in `plans/v1-production-readiness/05-findings.md`, and run `make check`; the emission lens includes the base/head corpus diff and strict C build

**Deliverables**:
- ST-1 – ST-6 and their implementation pins
- E10012 for the explicit `bool(null)`/`!null` spellings
- Nested fill, multi-element view-array, initializer field call-root, and indexed module-view roots released once

**Verify**: `make -C docker exec CMD="RDC_ASAN=1 cargo test --test v1_production_readiness_spec_test -- --test-threads=1"`; `make -C docker exec CMD="RDC_ASAN=1 cargo test --test v1_production_readiness_impl_test -- --test-threads=1"`; `make check`

---

## Phase 2: Stable v1.0 specification

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, semantics
> **Reasoning**: medium — a review-and-status pass over the frozen text, not a semantic rewrite

### Step 2.1: Chapter Review and Status

**Reference**: [03-02](03-02-spec-stabilization.md) §Proposed Changes · [AR-6](00-ambiguity-register.md)
**Objective**: Review every v1.0 chapter, correct stale text, and mark the v1.0 rule set stable.

- [ ] 2.1.1 Review chapters `00`–`02`, correct stale text, and mark the chapter status `stable (v1.0)` — `spec/00-introduction.md`, `spec/01-lexical.md`, `spec/02-types.md`
- [ ] 2.1.2 Review chapters `03`–`05`, correct stale text, record the E10012 null-cast exception, and mark the chapter status `stable (v1.0)` — `spec/03-ownership.md`, `spec/04-expressions.md`, `spec/05-statements.md`
- [ ] 2.1.3 Review chapters `06`–`10`, correct stale text, and mark the chapter status `stable (v1.0)` — `spec/06-functions.md`, `spec/07-structs.md`, `spec/08-arrays-strings.md`, `spec/09-enums-unions.md`, `spec/10-modules.md`
- [ ] 2.1.4 Review chapters `11`–`13`, correct stale text, and mark the chapter status `stable (v1.0)` — `spec/11-memory.md`, `spec/12-ffi-contracts.md`, `spec/13-data-inclusion.md`
- [ ] 2.1.5 Review chapters `14`–`15` and appendices A/B; keep the staged markers and mark the v1.0 chapters stable — `spec/14-diagnostics.md`, `spec/15-profiles.md`, `spec/appendix-amiga.md`, `spec/appendix-stdlib.md`
- [ ] 2.1.6 Review chapters `16`–`17` plus the grammar, grammar deltas, lineage, ledger, and specification README; update the README table and keep the anchor statuses — `spec/16-borrow-rules.md`, `spec/17-codegen.md`, `spec/grammar.ebnf.md`, `spec/grammar-deltas.md`, `spec/lineage.md`, `spec/decisions.md`, `spec/README.md`
- [ ] 2.1.7 Update the root README status table so NLL is described as implemented and the production-readiness status is current — `README.md`
- [ ] 2.1.8 Run `./spec/check.sh` and fix every status, link, or diagnostic-coverage failure; confirm the anchor/report statuses match the target table
- [ ] 2.1.9 Phase review (correctness, semantics) and record the findings

**Deliverables**:
- All v1.0 chapters and appendices `stable (v1.0)`
- Staged features still marked `E90xxx`/`W10005`
- Anchors and the preflight report keep their existing statuses and are listed as outside the chapter freeze
- Root README status table current
- `./spec/check.sh` green

**Verify**: `./spec/check.sh`; `make check`

---

## Phase 3: Verification infrastructure

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness; the change is infrastructure, not compiler semantics
> **Reasoning**: medium — two scripts, two Makefile targets, and one workflow

### Step 3.1: Sanitizer, Differential, and CI

**Reference**: [03-03](03-03-verification-infrastructure.md) §Proposed Changes · [AR-2, AR-9, AR-10, AR-11](00-ambiguity-register.md)
**Objective**: Add the simplified verification surface approved by the complexity gate.

- [ ] 3.1.1 Record the pre-target red state (`make asan` is an unknown target), then add the `make asan` target and the per-suite runner with `--test-threads=1` — `Makefile`, `docker/scripts/asan-suite.sh`
- [ ] 3.1.2 Run `make asan`; confirm the discovered count is 21, fix the discovery/timeout handling, and record the clean serial result (ST-7)
- [ ] 3.1.3 Record the pre-target red state (`make differential` is an unknown target), then add the `make differential` target and the three-program host/m68k runner with build/guard/clean and `DIFF_ONLY=1`; include it in `make verify` after `check` and before `corpus-fs-uae` — `Makefile`, `docker/scripts/differential-seed.sh`
- [ ] 3.1.4 Run `make differential` on the clean tree and the `DIFF_ONLY=1` negative control (ST-8, ST-9)
- [ ] 3.1.5 Add the host-only GitHub Actions workflow with least privilege, a pinned checkout, no persisted credentials, a timeout, and concurrency; validate its two commands locally (ST-10) — `.github/workflows/check.yml`
- [ ] 3.1.6 Update `AGENTS.md` with the new targets, the workflow, and the host-only CI limitation — `AGENTS.md`
- [ ] 3.1.7 Phase review (correctness) and run `make check`; record the CI run URL if an authorized push triggers it, otherwise record the local evidence

**Deliverables**:
- `make asan` and `make differential`
- `make verify` includes the differential leg after `check` and before `corpus-fs-uae`
- Host-only, least-privilege CI workflow
- AGENTS.md commands and layout current
- CI local evidence, or a recorded run URL after an authorized push

**Verify**: `make asan`; `make differential`; `make check`

---

## Phase 4: DX slice execution

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: the DX plan's own lenses
> **Reasoning**: medium — execution of an already-reviewed plan, no new design

### Step 4.1: Execute the Existing DX Plan

**Reference**: [03-04](03-04-dx-and-acceptance.md) §DX Plan Boundary · [AR-1](00-ambiguity-register.md)
**Objective**: Bring `plans/clorraine-dx/` to 28/28 and verify its outputs.

- [ ] 4.1.1 Execute `plans/clorraine-dx/` through its own exec-plan flow to 28/28, preserving its specification-first ordering and review dispatch
- [ ] 4.1.2 Preflight the FS-UAE environment (SDK X display, `docker/.env`, ROM path), then run the DX verification: `make check`, `make verify`, and `make demo-check`; record the evidence (ST-15)
- [ ] 4.1.3 Phase review, re-review `spec/14-diagnostics.md` after its DX help-cell edit, run `./spec/check.sh`, re-record `stable (v1.0)`, and close the DX gate

**Deliverables**:
- DX plan 28/28
- `--explain`, `--diagnostics=json`, and the canonical grammar delivered
- DX verification green
- `spec/14-diagnostics.md` re-reviewed and stable after the DX edit

**Verify**: `make check`; `make verify`; `make demo-check`; `./spec/check.sh`

---

## Phase 5: PLAN-13 acceptance slice

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — new FFI wrappers and real Amiga programs can expose compiler gaps

### Step 5.1: Plan and Execute the Bounded Slice

**Reference**: [03-04](03-04-dx-and-acceptance.md) §PLAN-13 Boundary · [AR-8, AR-13](00-ambiguity-register.md)
**Objective**: Deliver `std.amiga.exec` plus two allowlisted console programs and fix every
implementation, porting, and FFI gap they expose.

> **Gate (user input):** Phase 5 may start only after the user records the two programme names
> and licence audit in AR-13. The gate is not a plan task. If it is deferred, Phase 5 is
> `Blocked`, the plan is not Done, and the roadmap stays at `Plan Created`.

- [ ] 5.1.1 Confirm the user has recorded the two programme names and the licence audit (AR-13); then create the PLAN-13 plan for `std.amiga.exec` plus those two allowlisted console programs — `plans/amiga-library-surface/`
- [ ] 5.1.2 Execute the PLAN-13 plan to Done; fix every implementation, porting, and FFI gap the programs expose. An accepted-program gap blocks PLAN-13 Done; a gap that needs a language-surface change stops for a user ruling and is outside the slice.
- [ ] 5.1.3 Verify both programs on hosted-amiga against an independent expected-output oracle and run `make verify` (ST-16)
- [ ] 5.1.4 Phase review (the four project reviewers) and record the findings

**Deliverables**:
- `std.amiga.exec` wrapper surface
- Two allowlisted console programs with an independent expected-output oracle
- Every implementation/porting/FFI gap fixed; accepted-program gaps block Done; language gaps stopped for a ruling
- PLAN-13 Done

**Verify**: `make verify`; the two program runs

---

## Phase 6: Release readiness

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, security
> **Reasoning**: medium — licence, metadata, known limitations, and the release checklist;
> publication is the separate conditional sub-plan

### Step 6.1: Licence, Metadata, and Release Gate

**Reference**: [03-05](03-05-release-engineering.md) §Proposed Changes · [AR-5, AR-12, AR-14](00-ambiguity-register.md)
**Objective**: Make the repository legally and operationally ready for the v1.0.0 MIT release.
Publication is the separate conditional sub-plan, so the 45-task main plan can close
production-ready even if the artifact cannot be published.

- [ ] 6.1.1 Add the MIT `LICENSE` with the OpenLorraine copyright line — `LICENSE`
- [ ] 6.1.2 Add the Cargo licence/description/repository metadata, set `version = "1.0.0"`, and update the README licence section; set `license = "MIT"` in `tools/demo/package.json` — `compiler/Cargo.toml`, `README.md`, `tools/demo/package.json`
- [ ] 6.1.3 Add the accepted-residual document, link it from the README and the spec's limitations section, and rerun `./spec/check.sh` — `docs/known-limitations.md`, `README.md`, `spec/` (ST-13)
- [ ] 6.1.4 Write the release checklist with the AR-12 decision (v1.0.0, source + x86_64 tarball), the four verification commands, the manifest licence audit, and the conditional publish steps — `RELEASE.md`
- [ ] 6.1.5 Phase review (correctness, security) and verify the licence/metadata/checklist (ST-14)

**Deliverables**:
- MIT LICENSE and Cargo metadata; demo manifest MIT
- `docs/known-limitations.md` linked from README and spec
- `RELEASE.md` with the AR-12 decision and the conditional publish checklist

**Verify**: `./spec/check.sh`; `make check`

---

## Conditional Publish Sub-Plan

> **Not counted in the 45-task main plan.** Execute after Phase 7 passes, or after the user
> explicitly authorizes publication. If it cannot run, the main plan still closes
> production-ready; the release remains blocked with the reason recorded.

- [ ] C.1.1 Confirm the AR-12 decision (v1.0.0; source tag + Linux x86_64 tarball + `SHA256SUMS`) and the Cargo `version` of record set in 6.1.2 — `compiler/Cargo.toml`, `RELEASE.md`
- [ ] C.1.2 Build and smoke-test the x86_64 artifact; assert `--version` = v1.0.0; create the source tag, record checksums, and attach `RELEASE-NOTES.md` and the `CHANGELOG.md` entry — `RELEASE-NOTES.md`, `CHANGELOG.md`

**Deliverables**:
- v1.0.0 source tag and Linux x86_64 binary tarball
- `SHA256SUMS`, release notes, and changelog entry
- Smoke-test evidence for the x86_64 binary; aarch64 release deferred to PLAN-07

**Verify**: `clc --version` reports v1.0.0; a hello-world compile and run on x86_64; checksums match

---

## Phase 7: Final production gate

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: full verification; no new implementation
> **Reasoning**: medium — the final acceptance and documentation sync

### Step 7.1: Acceptance and Close

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria · [07](07-testing-strategy.md) §Verification Checklist
**Objective**: Prove every production gate and close the programme.

- [ ] 7.1.1 Run `make check`
- [ ] 7.1.2 Preflight the FS-UAE environment, run `make verify`, and record every leg
- [ ] 7.1.3 Run `make asan` and record the discovered suite count and serial result
- [ ] 7.1.4 Run `make differential`
- [ ] 7.1.5 Verify the DX plan is 28/28, `spec/14-diagnostics.md` was re-reviewed after the DX edit, and PLAN-13 is Done with both program outputs against an independent oracle
- [ ] 7.1.6 Verify the spec is stable (chapters/appendices plus the anchor table), the MIT licence and metadata are present (including the demo package), `docs/known-limitations.md` is current, and `RELEASE.md` records the AR-12 decision; record the CI run URL or the local evidence. The conditional publish sub-plan is complete or explicitly blocked.
- [ ] 7.1.7 Update `AGENTS.md` and the roadmap (PLAN-15 Done), record the final review evidence, and close the plan

**Deliverables**:
- All 45 main-plan tasks complete
- Roadmap and AGENTS.md current
- Programme closed with evidence; the conditional publish sub-plan complete or recorded as blocked

**Verify**: `make check`; `make verify`; `make asan`; `make differential`

---

## Dependencies

```
Phase 1 (residual hardening)
    ↓
Phase 2 (specification stable)
    ↓
Phase 3 (verification infrastructure)
    ↓
Phase 4 (DX slice) ──┐
    ↓                 │
Phase 5 (PLAN-13) ───┤
    ↓                 │
Phase 6 (release) ────┤
    ↓                 │
Phase 7 (final gate) ─┘
    ↓
Conditional publish (2 tasks, outside the 45)
```

Phase 4 and Phase 5 may run after Phase 3; they do not depend on each other. Phase 5 additionally
requires the AR-13 programme selection at its gate before task 5.1.1. Phase 6 is the release
readiness work; it does not wait for publication. The conditional publish sub-plan runs after
Phase 7 passes or after an explicit user authorization. Any phase that runs `make verify` first
checks the FS-UAE/X/ROM environment.

---

## Success Criteria

**Blocked-release branch:** if AR-13 (PLAN-13 programmes) remains deferred at the Phase 5 gate,
the plan is **Blocked**, not Done: it does not claim 45/45, the roadmap stays at `Plan Created`,
and the gate is recorded. If the conditional publish sub-plan cannot run, the 45-task main plan
may still close production-ready; the release sub-plan remains blocked with the reason recorded.
A separate user ruling is required to descope or defer either item.

**The programme is complete when:**

1. ✅ All 45 main-plan tasks are complete; the 2 conditional publish tasks are complete or explicitly blocked.
2. ✅ `make check`, `make verify`, `make asan`, and `make differential` pass.
3. ✅ The v1.0 specification is stable and `./spec/check.sh` passes.
4. ✅ The DX plan is 28/28 and its verification is green.
5. ✅ PLAN-13 is Done and both programs match an independent expected-output oracle.
6. ✅ The host-only CI workflow is green, or locally validated with the deferred run recorded.
7. ✅ MIT `LICENSE`, Cargo metadata, README licence text, and demo-package MIT metadata are present.
8. ✅ `docs/known-limitations.md` records the accepted residuals.
9. ✅ `RELEASE.md` records the AR-12 decision (v1.0.0, source + Linux x86_64 tarball); the
   conditional publish sub-plan is complete or explicitly blocked with the reason recorded.
10. ✅ `AGENTS.md` and the roadmap are current; the final review evidence is recorded.
11. ✅ No dead code, unused parameters, or commented-out blocks are introduced.
12. ✅ Security review is complete: CI uses no secrets, the release includes no NDK content,
    and the project has no new network, credential, or stored-data surface.
