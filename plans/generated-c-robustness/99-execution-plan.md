# Execution Plan: Generated-C Robustness Bundle

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-12 23:45
> **Progress**: 30/30 tasks (100%)
> **CodeOps Artifact Schema**: 1

**🚨 Update this document after EACH completed task!**

---

## Phases

| Phase | Title | Tasks |
|-------|-------|-------|
| 1 | Text ownership and module-path precedence | 14 |
| 2 | Init-local reservation and review | 8 |
| 3 | Full gate and closeout | 8 |

**Total: 30 tasks**

---

## Phase 1: Text Ownership and Module-Path Precedence

> **Baseline**: `0fdb96c`.
> **Lenses**: correctness, semantics, memory-safety, emission-integrity

### Step 1.1: Specification Tests

- [x] 1.1.1 [spec-author] Create `compiler/tests/generated_c_robustness_spec_test.rs` with ST-1–ST-15 — `compiler/tests/generated_c_robustness_spec_test.rs`
- [x] 1.1.2 Run against `0fdb96c`; record red evidence for ST-2–ST-4, ST-6, ST-7, ST-9–ST-11, ST-13 — verify: the failure log names each red case
- [x] 1.1.3 Confirm ST-1, ST-5, ST-8, ST-12, ST-14, ST-15 pass — verify: the suite output

### Step 1.2: Implementation

- [x] 1.2.1 Remove the module-level value-head gate from `is_std_fmt_text_path` and `is_builtin_text_call`; drop `value_head_shadows`; correct the doc comments — `compiler/src/sema/call.rs`, `compiler/src/own/expr.rs`, `compiler/src/codegen/program.rs`
- [x] 1.2.2 Decide `init_text_intrinsic_name` before `init_callable_value`; skip the callable/receiver for the intrinsic; add `Global`/`Const` to `declares_text`; remove the dead later canonicalization — `compiler/src/codegen/program.rs`
- [x] 1.2.3 Give the intrinsic text call a synthetic `Mode::Borrow` so an owned body argument is materialized and released — `compiler/src/own/call.rs`
- [x] 1.2.4 Update the #29 IT-13 pin and ST-15 comment; add the supersession note to #29 `01/03/05/07/99` — `compiler/tests/template_placeholder_residuals_iii_{spec,impl}_test.rs`, `plans/template-placeholder-residuals-iii/`
- [x] 1.2.5 Re-run the new spec suite; all Phase 1 cases green — verify: the suite output
- [x] 1.2.6 Re-run #24/#26/#27/#28/#29 and conformance — verify: the suite output

### Step 1.3: Impl Tests and Review

- [x] 1.3.1 Create `generated_c_robustness_impl_test.rs` with IT-1–IT-3, IT-6, IT-8–IT-12 — `compiler/tests/generated_c_robustness_impl_test.rs`
- [x] 1.3.2 Create `05-findings.md` — `plans/generated-c-robustness/05-findings.md`
- [x] 1.3.3 Run the four-lens review and record the verdicts — verify: `05-findings.md`
- [x] 1.3.4 Fix every critical/major finding and re-review — verify: the re-review verdict
- [x] 1.3.5 `make check` green — verify: `make check`

**Verify**: `make check`

---

## Phase 2: Init-Local Reservation and Review

> **Baseline**: the Phase 1 head.
> **Lenses**: correctness, memory-safety, emission-integrity, performance

### Step 2.1: Tests

- [x] 2.1.1 Add IT-4/IT-5 pins for the init-local reservation — `compiler/tests/generated_c_robustness_impl_test.rs`
- [x] 2.1.2 Record the red evidence against the Phase 1 head — verify: the failure log

### Step 2.2: Implementation

- [x] 2.2.1 Make `next_init_local` scan `is_taken_symbol` — `compiler/src/codegen/program.rs`
- [x] 2.2.2 Re-run the new suites; all green — verify: the suite output
- [x] 2.2.3 Re-run the prior suites — verify: the suite output

### Step 2.3: Review

- [x] 2.3.1 Run the correctness + memory-safety + emission review on the Phase 2 diff — verify: `05-findings.md`
- [x] 2.3.2 Fix every critical/major finding and re-review — verify: the re-review verdict
- [x] 2.3.3 `make check` green — verify: `make check`

## Phase 3: Full Gate and Closeout

### Step 3.1: Full Gate

- [x] 3.1.1 `make asan` green — verify: the runner summary
- [x] 3.1.2 `make ubsan` green — verify: the runner summary
- [x] 3.1.3 `make bench-base COMMIT=0fdb96c`, `make bench`, `make bench-compare` — verify: the comparison output
- [x] 3.1.4 `make bench-shape` green — verify: the four legs
- [x] 3.1.5 `make verify` green — all legs passed, including `sdk-a500-gui`, the advanced SDK gates, `differential`, `corpus-fs-uae`, and `run-fault-amiga`; the smoke gates run Xvfb inside the SDK image

### Step 3.2: Closeout

- [x] 3.2.1 Finalize `05-findings.md` — `plans/generated-c-robustness/05-findings.md`
- [x] 3.2.2 Update the PLAN-35 roadmap row — `plans/00-roadmap.md`
- [x] 3.2.3 Close #16, #30, #31 with evidence — GitHub

**Verify**: `make verify` and the recorded gate outputs

---

## Success Criteria

1. ✅ All phases completed
2. ✅ R1–R6 verified
3. ✅ `make check`, `make asan`, `make ubsan`, `make verify`, `make bench-shape`, `make bench-compare` green
4. ✅ No scored regression above 1%
5. ✅ #16, #30, #31 closed
6. ✅ No spec/stdlib/ABI change
7. ✅ The only prior-pin change is the intended #29 IT-13 update
