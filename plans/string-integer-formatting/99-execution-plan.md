# Execution Plan: String Integer Formatting Throughput

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-11 11:48
> **Progress**: 30/30 tasks (100%)
> **CodeOps Artifact Schema**: 1
> **Commit mode**: `--auto-commit --auto-design`; pushes are held to the final
> gate and performed with the git-commit skill in push mode.

## Overview

The plan records the baseline, writes the behavior and implementation tests
before touching the shims, replaces the two integer converters in both
`String` shims, measures the scored suite, runs the sanitizer and strict-C
legs, takes the two phase reviews, runs the full gate, and closes out with the
findings, roadmap, and follow-up issue.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Baseline and pre-change gate | 5 |
| 2 | Specification tests and red run | 4 |
| 3 | Implementation, green run, and implementation tests | 5 |
| 4 | Measurement, sanitizers, and cross-profile proof | 7 |
| 5 | Reviews and full gate | 4 |
| 6 | Documentation, follow-up issue, and closeout | 5 |

**Total: 30 tasks across 6 phases.** No fabricated hour estimates.

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the single source of truth for progress. Every
> task line appears exactly once. Mark `[~]` with a timestamp on
> implementation, promote to `[x]` only after the task's verify passes, and
> update the Progress header and Last Updated stamp after every task. A
> blocker is `[!]` with a short reason on the same line. Timestamps come from
> `date '+%Y-%m-%d %H:%M'`.

---

## Phase 1: Baseline and pre-change gate

> **Phase baseline tree**: recorded by the exec-plan skill.
> **Lenses**: compiler and language; performance engineering.
> **Reasoning**: high — the measurement identity and the clean pre-change
> gate decide whether the later deltas are attributable.

### Step 1.1: Commit the plan and fix the baseline

**Reference**: R5, AR-12, AR-13, AR-18 · `02-current-state.md` §Measurement Audit
**Objective**: make the plan durable and measure the clean base before any
shim change.

- [x] 1.1.1 Commit the plan documents (index, requirements, register, current state, component spec, findings, testing strategy, execution plan) with the git-commit skill; no compiler or runtime file may change. ✅ (completed: 2026-11-11 09:46)
- [x] 1.1.2 Confirm `git status --porcelain` is empty, `main == origin/main`, and run `make check` on the plan commit; record the result in `05-findings.md` §1. ✅ (completed: 2026-11-11 09:50)
- [x] 1.1.3 Build the base compiler from `8f07e40` with `make bench-base COMMIT=8f07e40`; record the compiler path, SHA-256, version, and `dirty` flag, and confirm `build/perf/base.json` names the expected commit. ✅ (completed: 2026-11-11 09:57)
- [x] 1.1.4 Run `make bench` on the plan commit and copy the document to a retained pre-change file; confirm every scored output matches and that the base and pre-change instruction/text values agree within the harness's determinism (otherwise record the difference and re-run once). ✅ (completed: 2026-11-11 10:03)
- [x] 1.1.5 Record the pre-plan probe (attribution, selected and rejected probe tables, allocation count, the out-of-scope codegen probe) in `05-findings.md` §2 with the exact commands and artifact paths; commit the findings with the git-commit skill. ✅ (completed: 2026-11-11 10:05)

**Deliverables**:
- The plan is committed and the tree is clean.
- `build/perf/base.json` and the pre-change head document exist and name `8f07e40`.
- `05-findings.md` carries the real baseline, not placeholders.

**Verify**: `make check` green on the plan commit; the harness documents have `dirty=false`; the pre-change outputs match.

---

## Phase 2: Specification tests and red run

> **Phase baseline tree**: the plan commit (no shim change).
> **Lenses**: compiler and language; test integrity.
> **Reasoning**: high — the new tests are the immutable oracle for a
> behavior-preserving replacement.

### Step 2.1: Write the tests first

**Reference**: R1–R3, R6, AR-03, AR-08, AR-14 · `07-testing-strategy.md` ST-1 – ST-24
**Objective**: encode every boundary and the zero-formatter-call contract
before changing a shim.

- [x] 2.1.1 [spec-author] Write the behavior half of `compiler/tests/string_integer_formatting_spec_test.rs` from ST-1 – ST-19; build each case with the strict host flags, assert the exact output, and support `RDC_CLC` for the later pre-change comparison; do not modify compiler or runtime code. ✅ (completed: 2026-11-11 10:15)
- [x] 2.1.2 [spec-author] Add ST-20 – ST-24 to the same suite: the 2,000-iteration checksum, the `--wrap=snprintf` counter stub, and the ASan/UBSan cases with their `RDC_ASAN`/`RDC_UBSAN` markers and scratch-directory cleanup. ✅ (completed: 2026-11-11 10:15)
- [x] 2.1.3 Run the suite against the pre-change compiler (the plan commit) and record the exact command, compiler identity, and every failure in `05-findings.md` §3; ST-21 must report a nonzero count. ✅ (completed: 2026-11-11 10:15)
- [x] 2.1.4 Record the intentional no-change oracle: ST-1 – ST-20 and ST-22 – ST-24 pass on the base; explain why in `05-findings.md` §3. Do not weaken a failing red case; confirm `make asan` and `make ubsan` discovery includes the new suite (the markers are present). ✅ (completed: 2026-11-11 10:15)

**Deliverables**:
- Both suites exist and build.
- The red record names the exact command, compiler identity, and failing cases.
- The no-change oracle is justified.

**Verify**: the red run is recorded; no runtime shim file changed; `cargo test` compiles both suites.

---

## Phase 3: Implementation, green run, and implementation tests

> **Phase baseline tree**: the plan commit with the red tests uncommitted.
> **Lenses**: compiler and language; memory safety; performance.
> **Reasoning**: high — the direct write bypasses the append capacity check
> and must preserve the block invariant.

### Step 3.1: Replace the integer converters

**Reference**: R1–R4, AR-02, AR-03, AR-04, AR-06, AR-09, AR-10 · `03-runtime-formatting.md`
**Objective**: implement the shared decimal writer and the two converter
bodies in both `String` shims.

- [x] 3.1.1 Add the `INT_MAX`/`UINT_MAX` width guard, `cl_format_u32`, and `cl_format_i32` to `compiler/runtime/shim_host.c`, and replace `cl_text_i32`/`cl_text_u32` with the direct-write bodies that set `len`/`cap` and the NUL; keep the doc comments factual about the maximum width and the `INT_MIN` magnitude. ✅ (completed: 2026-11-11 10:35)
- [x] 3.1.2 Apply the same guard, helper, and bodies to `compiler/runtime/shim_hosted_amiga.c` with `LONG`/`ULONG` parameters; keep the freestanding shim untouched. ✅ (completed: 2026-11-11 10:35)
- [x] 3.1.3 Run the new spec suite; it must pass (green). If a behavior case fails, fix the shim, never the expectation. ✅ (completed: 2026-11-11 10:36)
- [x] 3.1.4 Write `compiler/tests/string_integer_formatting_impl_test.rs` after the implementation: host emission contains `cl_format_i32`/`cl_format_u32` and no integer `snprintf(buffer` in the converter bodies; the float converter still calls `snprintf`; the Amiga emission has the same helpers; a literal-only template still emits `cl_str(...)`; two emissions are byte-identical. ✅ (completed: 2026-11-11 10:38)
- [x] 3.1.5 Run the implementation suite green and update `05-findings.md` §3 with the result and the exact head commit. ✅ (completed: 2026-11-11 10:42)

**Deliverables**:
- Both shims compile warning-free.
- ST-1 – ST-24 pass; the impl suite passes.
- The findings red/green table is complete.

**Verify**: `cargo test --workspace` green; the new suites green.

---

## Phase 4: Measurement, sanitizers, and cross-profile proof

> **Phase baseline tree**: the implementation commit.
> **Lenses**: performance; memory safety; cross-profile.
> **Reasoning**: high — the benchmark and sanitizer legs are the acceptance
> evidence, not a formality.

### Step 4.1: Measure and harden

**Reference**: R5, R6, AR-12, AR-13, AR-14 · `05-findings.md` §4–§5
**Objective**: prove the win, the no-regression guard, and the safety legs.

- [x] 4.1.1 Run `make bench` and `make bench-compare`; record the full base/head table, output equality, and the `strings` delta in `05-findings.md` §4. The change is rejected if `strings` misses −30% or any scored metric exceeds the 1% guard. ✅ (completed: 2026-11-11 10:50)
- [x] 4.1.2 Re-run the allocation-count probe on the head and record 6,003 (or the measured value) and the per-iteration split in `05-findings.md` §4 and §7. ✅ (completed: 2026-11-11 10:52)
- [x] 4.1.3 Run `make asan` and `make ubsan`; confirm the new suite runs in both legs and record the results in `05-findings.md` §5. ✅ (completed: 2026-11-11 11:05)
- [x] 4.1.4 Run `make bench-shape`; confirm the host, hosted-amiga, hosted-amiga-13, vbcc, and freestanding legs are clean; record in `05-findings.md` §5. ✅ (completed: 2026-11-11 11:07)
- [x] 4.1.5 Record the emitted-C diff evidence: the host and Amiga emissions differ only in the shim converter bodies, no template call or program-body line changed; record the exact files and line ranges. ✅ (completed: 2026-11-11 11:09)
- [x] 4.1.6 Build a boundary program (`INT_MIN`, `INT_MAX`, `UINT_MAX`, `0`, adjacent placeholders) for `hosted-amiga`, run it under `vamos`, and compare its output with the host build; record the result, so the target-side boundary path is exercised rather than only compiled. ✅ (completed: 2026-11-11 11:10)
- [x] 4.1.7 Run the spec suite against the recorded pre-change compiler one more time; confirm ST-21 still reports a nonzero count and the no-change cases still pass. Run the impl suite there too and record that it fails only on the new-helper pins (attributability evidence, not a red-phase requirement). ✅ (completed: 2026-11-11 11:11)

**Deliverables**:
- The final benchmark table with output hashes.
- ASan, UBSan, and `bench-shape` results.
- The emission proof.

**Verify**: `make bench-compare` passed; sanitizer and shape legs green.

---

## Phase 5: Reviews and full gate

> **Phase baseline tree**: the implementation and measurement commit.
> **Lenses**: correctness; memory safety; emission integrity is not in scope
> (no codegen change).
> **Reasoning**: high — the review gate is mandatory for a heap write.

### Step 5.1: Independent review and acceptance

**Reference**: R5–R8, AR-19 · `AGENTS.md` "CodeOps"
**Objective**: close the correctness and memory-safety reviews, then run the
full acceptance gate.

- [x] 5.1.1 Dispatch the correctness reviewer with a packet covering the two shims, the helper algorithm, the boundary proofs, the template interactions, and a blast-radius step (every caller of `cl_text_i32`/`cl_text_u32`, both profiles, and the explicit-conversion paths). ✅ (completed: 2026-11-11 11:25)
- [x] 5.1.2 Dispatch the read-only memory-safety auditor with a packet covering the direct write, the capacity proof, `INT_MIN`, the NUL, and the ASan probes; require one reproduction per finding. ✅ (completed: 2026-11-11 11:25)
- [x] 5.1.3 Resolve every critical/major finding with an authorized ruling, apply the fix, and rerun the affected suites and measurement legs. ✅ (completed: 2026-11-11 11:27)
- [x] 5.1.4 Run `make check`, `make bench-compare`, and `make verify` on the final head; record the leg results in `05-findings.md` §5–§6. ✅ (completed: 2026-11-11 11:35)

**Deliverables**:
- Both reviews recorded with verdicts.
- Full gate green.

**Verify**: `make verify: all legs passed`; no open critical/major finding.

---

## Phase 6: Documentation, follow-up issue, and closeout

> **Phase baseline tree**: the reviewed head.
> **Lenses**: documentation; roadmap.
> **Reasoning**: medium — the evidence is stable; this phase records it.

### Step 6.1: Close out

**Reference**: R7, R8, AR-15, AR-16, AR-17
**Objective**: make the evidence durable and leave the residual visible.

- [x] 6.1.1 Complete `05-findings.md`: final table, allocation split, sanitizer results, review verdicts, residual. ✅ (completed: 2026-11-11 11:40)
- [x] 6.1.2 Update `plans/00-roadmap.md`: add the plan row with stage `Done`, the 30/30 count, the `strings` delta, and the gate summary. ✅ (completed: 2026-11-11 11:41)
- [x] 6.1.3 Open the follow-up GitHub issue for the codegen direct-to-destination lowering with the probe evidence and the scope note; record the issue number in `05-findings.md` §7 and the roadmap. ✅ (completed: 2026-11-11 11:42)
- [x] 6.1.4 Update `docs/known-limitations.md` only if the residual allocation behavior is user-visible; otherwise record "no change" in the findings. Do not add planning identifiers to `docs/`. ✅ (completed: 2026-11-11 11:43)
- [x] 6.1.5 Commit the documentation, roadmap, and findings with the git-commit skill; verify the tree is clean and push with the git-commit skill in push mode only after the final gate. ✅ (completed: 2026-11-11 11:48)

**Deliverables**:
- Findings, roadmap, and follow-up issue are complete.
- The final commit is pushed and `main == origin/main`.

**Verify**: `git status --porcelain` empty; roadmap row present; follow-up issue exists.

---

## Dependencies

```
Phase 1
    ↓
Phase 2
    ↓
Phase 3
    ↓
Phase 4
    ↓
Phase 5
    ↓
Phase 6
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All 30 tasks completed.
2. ✅ `make check`, `make bench-shape`, and `make verify` pass.
3. ✅ The benchmark comparison passes; `strings` improves by at least 30%
   and no scored metric regresses above 1%.
4. ✅ ASan and UBSan legs are clean.
5. ✅ Both reviews are recorded with no open critical/major finding.
6. ✅ `05-findings.md`, the roadmap, and the follow-up issue are complete.
7. ✅ No `spec/` or `stdlib/` file changed; the tree is clean and pushed.
