# Current State: Clorraine Compiler MVP

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)
> **Analysis baseline**: commit `585074e` (2026-09-17), working tree clean

## Existing Implementation

### What Exists

| Artifact | State | Relevance to this plan |
|---|---|---|
| `spec/` (Ch 00-17, appendices, ledger, grammar, lineage) | Frozen, preflight-passed (27 files, `spec check: OK`) | The oracle: every rule, diagnostic, and codegen contract the MVP implements |
| `spec/check.sh` | Working bash validator | Structural gate; stays the spec-side check |
| `spec/tests/compile-fail/` | 95+ `.clor` golden files, structure-validated only | Becomes the compile-fail harness input |
| `spec/tests/run/` | 6 runtime-fault cases, not executable | Becomes the run-fault harness input |
| `spec/tests/staged/` | 4 staged intent files | Excluded from coverage (AR-10) |
| `docker/` SDK (GCC 16.2, vbcc, libnix, NDK, gdb, vamos, FS-UAE, Rust stable, host cc) | Ready (`docker/README.md`) | The build/test environment and the implementation toolchain |
| `examples/hosted-skeleton/` | Phase 0 walking skeleton, C | Pattern source for the cleanup ladder, shim, and run matrix |
| `docs/findings-phase0.md`, `docs/findings-phase1.md` | Complete | Open items this plan executes (spikes, run suite, corpus) |
| `docs/ergonomics-corpus.md` | Friction sketches + rulings | Raw material for the corpus programs |
| `.gitignore` | Covers `*.o`, `*.prg`, `/build/`, `/target/` | Root `target/` already reserved for the future Rust workspace |

### Relevant Files

| File | Purpose | Changes Needed |
|---|---|---|
| `spec/tests/corpus/` | Positive golden corpus | Create (four program folders) |
| `spikes/` | Phase 2 spike fixtures | Create |
| `Cargo.toml`, `compiler/` | Compiler workspace and crate | Create |
| `stdlib/` | Clorraine standard-library slice | Create |
| `Makefile` (root) | `check` / `verify` orchestration | Create |
| `docs/findings-phase2.md` | Spike results | Create |
| `plans/00-roadmap.md`, `AGENTS.md` | Tracking and project guidance | Create (AR-38, AR-40) |
| `spec/tests/compile-fail/W10000_deny_warnings.clor` | Warning escalation golden | Create (AR-34) |

### Code Analysis

There is no compiler code. The closest executable artifacts are
`examples/hosted-skeleton/skeleton.c` (hand-written emitted-C shape: nullable
acquires, null-safe drops, single-exit ladder, counters) and
`spec/check.sh` (the shell-script validation style the project already uses).
The specification fixes every interface this plan implements: the grammar
(`grammar.ebnf.md` sections 2-10 with disambiguation in section 11), the
diagnostics registry (Ch 14), the typed control-flow IR the borrow checker
consumes (Ch 16 §6, `16-borrow-rules.md:213`), and the C emission contract
(Ch 17).

## Gaps Identified

### Gap 1: No compiler pipeline

**Current Behavior:** Clorraine source cannot be compiled; the golden suites
are structure-validated data only (`spec/tests/compile-fail/README.md:21-24`).
**Required Behavior:** `clc` lexes, parses, resolves, type-checks,
borrow-checks, and emits C for the corpus subset.
**Fix Required:** Phases 2-6 of `99-execution-plan.md`.

### Gap 2: No standard library

**Current Behavior:** `appendix-stdlib.md` is normative surface with no
implementation; output is marked planned (`appendix-amiga.md:28`).
**Required Behavior:** The AR-15/AR-46/AR-47 slice under `stdlib/` plus the
per-profile C shim.
**Fix Required:** Phase 7.

### Gap 3: Accepted Phase 2 verification items not executed

**Current Behavior:** The vbcc `#line` (PF-042), generated-C FFI (PF-002), and
arithmetic (PF-008) spikes plus the `tests/run/` execution are accepted but
undone (`preflight-report.md:121-126`, `findings1:83-92`).
**Required Behavior:** Fixtures under `spikes/` and results in
`docs/findings-phase2.md` before codegen work relies on them.
**Fix Required:** Phase 1.

### Gap 4: No positive corpus

**Current Behavior:** Only runtime-fault cases exist under `spec/tests/run/`;
the positive corpus is an open item (`findings1:90`).
**Required Behavior:** Four programs with expected outputs and hand-check
records.
**Fix Required:** Phase 0 (Task 0).

### Gap 5: No project guidance or orchestration

**Current Behavior:** No `AGENTS.md`, no root `Makefile`, no roadmap.
**Required Behavior:** AR-7, AR-38, AR-40.
**Fix Required:** Phases 6 and 8.

## Dependencies

### Internal Dependencies

- The frozen specification is the sole semantic authority; where the plan and
  the spec disagree, the spec wins (AR-31 pause rule).
- Phase 0 (corpus) and Phase 1 (spikes) gate the front-end and codegen phases.
- The harness skeleton (Phase 2) gates all later golden-suite work.
- The stdlib slice (Phase 7) gates the corpus E2E legs (Phase 8).

### External Dependencies

- Docker SDK image `clorraine-sdk:dev` (GCC, vbcc, libnix, NDK, gdb, vamos,
  FS-UAE, Rust stable, host cc) — no new tools are added (AR-20).
- No network at test time; `cargo` runs with zero external crates.
- Kickstart ROMs are not required: `vamos` and the FS-UAE AROS ROM cover the
  corpus (`docker/README.md`).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Corpus hand-check exposes a spec seam (V9a copy-on-lend, pre-NLL approximation, view elision) | Medium | High | AR-31: pause and take an explicit user ruling; lineage §10 says amend the rule, never silently reopen the model |
| vbcc does not honor `#line` in debug info | Medium | Medium | Spike S1 measures it; if unsupported, findings record the fallback and Ch 17's claim is revisited with the user |
| vbcc signed arithmetic does not wrap as assumed | Low | High | Spike S3 decides between `-fwrapv` (GCC) and the unsigned idiom (vbcc, PF-008) before codegen |
| Emitted FFI prototypes mismatch the NDK ABI | Low | High | Spike S2 compiles exact `BPTR`/`APTR`/`LONG` prototypes against the NDK and runs them |
| Interrupts unavailable under `vamos` | Medium | Medium | AR-4/AR-30: the counter's home leg is FS-UAE; vamos is at most a build leg |
| Float ABI unverified (P15) | Low | Medium | Only two E11212 run cases need floats; both execute on host and vamos; results recorded |
| Compiler scope grows past the corpus | Medium | High | Corpus-driven staging (AR-2) + E90000 for everything else + the user checkpoint (AR-31) |
| Long plan execution drifts | Medium | Medium | The execution plan is the single progress authority; `make check` stays green at every task boundary |
| Rust toolchain unavailable offline | Low | High | std-only, zero crates (AR-20); the SDK image already contains stable Rust |
