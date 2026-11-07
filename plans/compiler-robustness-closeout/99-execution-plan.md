# Execution Plan: Compiler Robustness Closeout

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Created**: 2026-11-07
> **Last Updated**: 2026-11-07
> **Progress**: 18/38 tasks (47%)
> **CodeOps Artifact Schema**: 1

## Overview

Close the six recorded defects in dependency order, then add the UBSan leg and
the fuzzer, then finish the cross-toolchain and fault gates, the full
acceptance matrix, independent reviews, a re-gate, and the push. Each committed
task leaves the tree buildable and the focused tests green. Red-first test
tasks are committed together with the fix they prove, so no commit is red.

**Update this document after each completed task.**

## Dependencies

```text
0.1.1 requirements ──> 0.1.2 plan gate ──> 0.1.3 baseline
0.1.3 ──> 1.1.1/1.1.2 D1 tests ──> 1.1.3 D1 fix ──> 1.1.4 D1 verify/commit ──> 1.1.5 review
1.1.5 ──> 2.1.1/2.1.2 D2 tests ──> 2.1.3 D2 fix ──> 2.1.4 D2 verify/commit
2.1.4 ──> 2.2.1/2.2.2 D3 tests + evidence ──> 2.3.1 D2/D3 review
2.3.1 ──> 3.1.1 D5 tests ──> 3.1.2 D5 fix ──> 3.1.3 D5 verify/commit
3.1.3 ──> 4.1.1 D6 tests ──> 4.1.2 D6 fix ──> 4.1.3 D6 verify/commit ──> 4.1.4 review
4.1.4 ──> 5.1.1 UBSan runner ──> 5.1.2 suites ──> 5.1.3 make ubsan verify/commit
4.1.4 ──> 6.1.1 fuzzer tests ──> 6.1.2 fuzzer ──> 6.1.3 seeds ──> 6.1.4 gates ──> 6.1.5 review
5.1.3, 6.1.5 ──> 7.1.1 shape gate ──> 7.1.2 fault parity
7.1.1, 7.1.2 ──> 7.1.3 docs ──> 7.1.4 emission audit ──> 7.1.5 full gates
7.1.5 ──> 7.1.6 final review ──> 7.1.7 re-gate ──> 7.1.8 push
```

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 0 | Gates and baseline | 3 |
| 1 | D1 fill guards | 5 |
| 2 | D2/D4 narrow wrapping + D3 regression | 7 |
| 3 | D5 float width | 3 |
| 4 | D6 enum values and conversion | 4 |
| 5 | UBSan leg | 3 |
| 6 | Fuzzer | 5 |
| 7 | Shape, faults, docs, acceptance, review | 8 |

**Total: 38 tasks across 8 phases.**

> **EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes are the single source of truth for progress. Mark `[~]`
> with a timestamp on start; promote to `[x]` with a timestamp on verify pass;
> update the Progress header after every task. On a blocker, mark `[!]` and
> append `Blocked: <reason>`. Timestamps come from `date '+%Y-%m-%d %H:%M'`.

---

## Phase 0: Gates and Baseline

> **Phase baseline tree**: `96ae0d4`
> **Reasoning**: high — the requirements and plan gates own the scope, the
> user-ruled enum overflow rule, and the fuzzer oracle.
> **Lenses**: compiler semantics · emission integrity · verification quality
> **Verify**: both preflight reports show every critical/major finding resolved.

### Step 0.1: Requirements, plan, baseline

**Reference**: [Requirements](01-requirements.md) ·
[Ambiguity Register](00-ambiguity-register.md) ·
[Requirements Preflight](00-preflight-report-requirements.md) ·
[Plan Preflight](00-preflight-report.md)

- [x] 0.1.1 Write the requirements, current-state study, ambiguity register, and requirements preflight; resolve all 36 findings; record the user E10202 ruling ✅ (completed: 2026-11-07 19:10)
- [x] 0.1.2 Write the fix design, verification-infrastructure design, testing strategy, and this execution plan; run the independent plan preflight; resolve every critical/major finding ✅ (completed: 2026-11-07 20:40)
- [x] 0.1.3 Record the `make check` wall time; copy the pre-PLAN-14 `0ac87d3e` binary to `build/robustness/base-0ac87d3e-clc`; build `96ae0d4` once at `build/robustness/base-96ae0d4-clc`; record both hashes ✅ (completed: 2026-11-07 20:52)

**Deliverables**:
- [x] R1–R9, AR-1–AR-48, and the requirements preflight verdict
- [x] The plan document set and the plan preflight verdict
- [x] The measured fast-gate baseline (`123.78 s`) and the two base binaries (`0ac87d3e` sha256 `7ce743a…`, `96ae0d4` sha256 `06ae4ed…`)

---

## Phase 1: D1 Fill Guards

> **Phase baseline tree**: after 0.1.3
> **Reasoning**: high — the only memory-safety fix; a wrong guard placement is
> a use-after-free or a missed trap.
> **Lenses**: memory safety · evaluation order · guard duplication
> **Verify**: focused spec/impl cases, a focused `RDC_ASAN` run, and the pure
> fill probes; then an independent correctness + memory-safety review.

### Step 1.1: Guard tests and fix

**Reference**: [Defect Fixes](03-01-defect-fixes.md) D1 · R1.1–R1.6
**Objective**: Emit the operand guards a pure fill lost, on every path that
reaches it, with the documented once-per-site rule.

- [x] 1.1.1 [spec-author] Add the D1 spec cases (bounds, division, float,
  struct, scalar conditional, conditional struct, module conditional, module
  count, prefix count, laziness) with the expected fault lines and ASan
  opt-in; run red against the starting compiler ✅ (completed: 2026-11-07 21:20)
- [x] 1.1.2 [impl-author] Add the D1 guard-count pins scoped to the
  function/init body; run red ✅ (completed: 2026-11-07 21:20)
- [x] 1.1.3 Implement `init_fill_guards_emitted`, the pure-branch emission,
  the module dedup including prefix elements, the struct-literal walk, and the
  conditional arm handling ✅ (completed: 2026-11-07 21:30)
- [x] 1.1.4 Verify the focused cases, `RDC_ASAN=1 cargo test --test
  compiler_robustness_spec_test`, and `make check`; commit
  `fix(compiler): emit pure-fill operand guards` ✅ (completed: 2026-11-07 21:45)
- [x] 1.1.5 Independent review: correctness reviewer + memory-safety auditor
  (ASan probes). RV-001 (module struct duplicate) and MS-001/MS-002 fixed;
  re-review clean; 87 probes, 0 ASan/LSan reports ✅ (completed: 2026-11-07 21:55)

**Deliverables**:
- [x] Green D1 spec (13) and impl (11) cases
- [x] ASan-clean guard cases
- [x] One verified commit with red evidence in `05-findings.md`
- [x] A clean D1 review verdict

---

## Phase 2: D2/D4 Narrow Wrapping

> **Phase baseline tree**: after 1.1.5
> **Reasoning**: high — the fix touches every integer arithmetic emission and
> must not introduce signed overflow or change a correct value.
> **Lenses**: C integer promotion · wrap width · strict C99 · freestanding
> **Verify**: the value matrix, the 16-bit UB cases, the strict build matrix,
> `make differential`, the shifted `fill-shift` pin, and an independent
> correctness + memory-safety + emission-integrity review.

### Step 2.1: Wrap tests and fix

**Reference**: [Defect Fixes](03-01-defect-fixes.md) D2/D4 · R2.1–R2.6
**Objective**: Wrap `+`, `-`, `*`, and `<<` at the declared result type using a
defined `unsigned int` intermediate, without `uint32_t` in the generated C.

- [x] 2.1.1 [spec-author] Add the D2/D4 value matrix, the 16-bit cases, the
  32-bit controls, the operator agreement matrix, the signed narrow case, the
  freestanding case, and the const/local/fill/field strict-build cases; run red
  ✅ (completed: 2026-11-07 22:35)
- [x] 2.1.2 [impl-author] Add the result-cast emission pins and update the
  `fill-shift` pin; run red ✅ (completed: 2026-11-07 22:35)
- [x] 2.1.3 Implement the width-aware arithmetic and shift result casts,
  including signed narrow results through `unsigned int`
  ✅ (completed: 2026-11-07 22:40)
- [x] 2.1.4 Update the PLAN-14 `narrow` value pin, verify the focused cases,
  `make check`, `make differential`, and the base red evidence; commit
  `fix(compiler): wrap narrow arithmetic at its result type`
  ✅ (completed: 2026-11-07 22:55)

### Step 2.2: D3 regression and base evidence

**Reference**: [Defect Fixes](03-01-defect-fixes.md) D3 · R3.1–R3.3
**Objective**: Pin the already-correct inline shift divisor and record the
pre-PLAN-14 red evidence.

- [x] 2.2.1 [spec-author] Add the D3 spec case (local, module const, fill, call
  argument, named const) and the impl count pin; add the `RDC_CLC` helper
  ✅ (completed: 2026-11-07 22:50)
- [x] 2.2.2 Run the case against `build/robustness/base-0ac87d3e-clc` (traps)
  and the head (prints 0); record both outputs in `05-findings.md`; commit
  with the D2 arithmetic commit because both extend the same suites
  ✅ (completed: 2026-11-07 22:55)

### Step 2.3: Review

- [x] 2.3.1 Independent review: correctness reviewer, memory-safety auditor,
  and emission-integrity auditor (base/head emission diff, strict build hunt).
  RV-001/EI-001 fixed; RV3-001/RV3-002/EI2-001 test gaps closed; re-review
  clean ✅ (completed: 2026-11-07 23:05)

**Deliverables**:
- [x] Green D2/D4 spec and impl cases
- [x] `const U8W` builds under the strict matrix
- [x] One verified commit with red evidence
- [x] Green D3 regression with pre-PLAN-14 base evidence
- [x] A clean D2/D3 review verdict

---

## Phase 3: D5 Float Width

> **Phase baseline tree**: after 2.3.1
> **Reasoning**: high — the folder change affects every constant float
> expression and the enum walk that follows.
> **Lenses**: IEEE-754 binary32 · constant-context diagnostics · no
> materialization
> **Verify**: cast, declared-const, per-operation cancellation, NaN,
> out-of-range, finite, and `-0.0`/infinity cases; the existing float pins stay
> green.

### Step 3.1: Float tests and fix

**Reference**: [Defect Fixes](03-01-defect-fixes.md) D5 · R4.1–R4.7
**Objective**: Round `f32` at casts and operations, carry the declared float
width, and reject a NaN or out-of-range constant conversion with the registered
E10202 message.

- [x] 3.1.1 [spec-author] Add the D5 spec cases, the per-operation unit case,
  and the registered-message assertion; run red ✅ (completed: 2026-11-07 23:25)
- [x] 3.1.2 Implement `round_float`, the range/NaN error payload, the float
  result scalar, and the declared-const width ✅ (completed: 2026-11-07 23:30)
- [x] 3.1.3 Verify the focused cases and `make check`; commit
  `fix(compiler): round f32 constants and reject constant float overflow`
  ✅ (completed: 2026-11-07 23:35)

**Deliverables**:
- [x] Green D5 cases including the cancellation and message tests
- [x] Floats still stay places
- [x] One verified commit with red evidence

---

## Phase 4: D6 Enum Values and Conversion

> **Phase baseline tree**: after 3.1.3
> **Reasoning**: high — the user-ruled overflow rejection, the shared enum
> walk, and the conversion lowering touch both sema and codegen.
> **Lenses**: enum value semantics · cross-module folding · cast spelling ·
> shadowing safety
> **Verify**: explicit, overflow reject, in-range control, stored, extent,
> cross-module, enum/handle cast, constant cast, and shadowing cases; the strict
> build matrix; an independent correctness + emission-integrity review.

### Step 4.1: Enum tests and fix

**Reference**: [Defect Fixes](03-01-defect-fixes.md) D6 · R5.1–R5.7
**Objective**: One enum value walk, the user-ruled E10202 overflow rejection,
and a shadow-safe type-name cast lowering.

- [ ] 4.1.1 [spec-author] Add the D6 spec, compile-fail, and impl cases; run red
- [ ] 4.1.2 Implement `ConstFolder::enum_member_values`, the sema auto-increment
  fit check, the emitter fold, the module name on `EnumInfo`, the enum/handle
  cast lowering, and the constant enum cast
- [ ] 4.1.3 Update the PLAN-14 enum spec and impl pins, verify the focused
  cases and `make check`; commit
  `fix(compiler): fold enum member values in the emitter`
- [ ] 4.1.4 Independent review: correctness reviewer + emission-integrity
  auditor. Resolve every critical/major finding and record the verdict

**Deliverables**:
- [ ] Green D6 cases
- [ ] `Direction(2)` and `Buffer(0)` compile and run
- [ ] One verified commit with red evidence
- [ ] A clean D6 review verdict

---

## Phase 5: UBSan Leg

> **Phase baseline tree**: after 4.1.4
> **Reasoning**: high — a sanitizer leg that silently does nothing is worse
> than none.
> **Lenses**: sanitizer activation · suite controls · false positives
> **Verify**: `make ubsan` runs the self-test and at least two suites; a
> deliberate UB probe fails; a compiler-guard trap passes; the 16-bit
> arithmetic cases are UBSan-clean.

### Step 5.1: Runner, suites, integration

**Reference**: [Verification Infrastructure](03-02-verification-infrastructure.md) §1 · R6.1–R6.7
**Objective**: Add the UBSan runner, the two opt-in suites, the suite-level
controls, and the verify placement.

- [ ] 5.1.1 Add `docker/scripts/ubsan-suite.sh` with the activation self-test
  (nonzero exit expected), discovery, minimum count, timeout, `ulimit -c 0`,
  and `-no-pie` wrapper
- [ ] 5.1.2 Add `RDC_UBSAN` to the robustness suite, the suite-level ASan/UBSan
  controls, and `ubsan_corpus_spec_test.rs`
- [ ] 5.1.3 Add the `ubsan` target, `UBSAN_SUITE_TIMEOUT`, and the `verify`
  step after `check`; run `make ubsan`; commit
  `build: add a host UBSan verification leg`

**Deliverables**:
- [ ] `make ubsan` green with the measured runtime in `05-findings.md`
- [ ] The corpus sweep and suite controls green
- [ ] One verified commit

---

## Phase 6: Fuzzer

> **Phase baseline tree**: after 5.1.3
> **Reasoning**: high — the fuzzer is a committed verification tool; a
> non-deterministic or unsafe harness is worse than none.
> **Lenses**: determinism · oracle quality · sandboxing · bounded runtime
> **Verify**: unit tests, the smoke and smoke-host runs on the fixed compiler,
> the seed goldens, a deliberate failure injection per class, and an
> independent correctness review.

### Step 6.1: Tests, harness, seeds, gates

**Reference**: [Verification Infrastructure](03-02-verification-infrastructure.md) §2 · R7.1–R7.13
**Objective**: Add the deterministic generator, the host/sanitizer/m68k
runner, the seeded goldens, and the CI gates.

- [ ] 6.1.1 [spec-author] Add the fuzzer unit tests (two-process determinism,
  template acceptance, goldens, seed safety, normalization, classifier,
  argument lists, private cwd/core); run red
- [ ] 6.1.2 Implement `tools/fuzz/__init__.py`, `tests/__init__.py`,
  `fuzz.py`, and the five Makefile targets
- [ ] 6.1.3 Add the twelve seeds and their goldens; verify the smoke run
- [ ] 6.1.4 Run `make fuzz-test`, `make fuzz-smoke`, and a bounded
  `make fuzz`; measure `fuzz-test` and the smoke process separately; decide the
  `fuzz-smoke-host` fallback if the budget is exceeded; commit
  `feat(tools): add a deterministic differential fuzzer`
- [ ] 6.1.5 Independent review: correctness reviewer. Resolve every
  critical/major finding and record the verdict

**Deliverables**:
- [ ] Green fuzzer unit tests
- [ ] Green smoke matrix with the measured cost
- [ ] Twelve committed seeds
- [ ] One verified commit
- [ ] A clean fuzzer review verdict

---

## Phase 7: Shape, Faults, Docs, Acceptance, Review

> **Phase baseline tree**: after 6.1.5
> **Reasoning**: high — the last phase owns the cross-toolchain proof, the
> emission attribution, the release decision, and the final re-gate.
> **Lenses**: cross-toolchain C99 · emission attribution · acceptance honesty
> **Verify**: every gate in R9.1, the independent final review, and a re-gate
> on the exact pushed tree.

### Step 7.1: Gates, docs, audit, reviews, push

**Reference**: [Requirements](01-requirements.md) R8.4–R9.6 ·
[Testing Strategy](07-testing-strategy.md)

- [ ] 7.1.1 Extend the shape fixture with the narrow const, narrow fill,
  narrow arithmetic fill, f32 const, enum, and runtime-index fill; add the
  `nix13` leg; run `make bench-shape`; commit
- [ ] 7.1.2 Add the two fill fault probes and extend `run-fault-amiga.sh` with a
  per-case path; run `make run-fault-amiga`; commit
- [ ] 7.1.3 Update `AGENTS.md`, the roadmap PLAN-18 row, the specialist check,
  and `docs/known-limitations.md` where required; commit the docs
- [ ] 7.1.4 Reuse `build/robustness/base-96ae0d4-clc` and
  `base-0ac87d3e-clc`; emit the corpus and seeds for host and hosted-amiga with
  base and head; attribute every difference
- [ ] 7.1.5 Run `make check`, `make asan`, `make ubsan`, `make corpus-amiga`,
  `make differential`, `make bench-shape`, and `make verify`; record the exact
  output
- [ ] 7.1.6 Final independent review: correctness, memory-safety,
  emission-integrity, and semantics reviewers on the whole change. Resolve
  every critical/major finding
- [ ] 7.1.7 Re-run the affected gates and the full `make verify` on the exact
  tree after the final review; write the final report and `05-findings.md`
- [ ] 7.1.8 Commit the closeout; push `origin/main`; verify the tree is clean
  and `origin/main..HEAD` is 0

**Deliverables**:
- [ ] Green shape, fault, and full acceptance gates
- [ ] A clean emission audit
- [ ] Independent review verdicts
- [ ] The pushed closeout commit

---

## Evidence Log

| Task | Command | Result | Artifact |
|---|---|---|---|
| 0.1.1 | requirements preflight | 36 findings resolved; user E10202 ruling | `00-preflight-report-requirements.md`, AR-11 |
| | | | |
