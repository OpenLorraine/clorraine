# Execution Plan: Constant-Array Codegen Residuals (PLAN-36)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-13
> **Progress**: 30/31 tasks (97%)
> **CodeOps Artifact Schema**: 1

## Overview

Deliver issue #18 in four phases: measure the four residuals on one probe set,
land the function-local template, add the committed benchmark and the two test
suites, then run the full gate and close the record. The technical
specification is [03-design.md](03-design.md); the expected behavior is
[07-testing-strategy.md](07-testing-strategy.md).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Measurement and decisions | 6 |
| 2 | Implementation and focused tests | 12 |
| 3 | Verification and reviews | 8 |
| 4 | Documentation and closeout | 5 |

**Total: 31 tasks across 4 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the single source of truth for progress. On
> implementation mark `[~]`; on verify pass mark `[x]`; on blocker mark `[!]`
> with the reason. Update the Progress header after every task. Timestamps
> come from `date '+%Y-%m-%d %H:%M'`.

---

## Phase 1: Measurement and Decisions

> **Phase baseline**: `7071761` with the probe sources under `build/tmp`.

### Step 1.1: Baseline probes

- [x] 1.1.1 Preserve `build/tmp/clc-base` and generate the local, module, byte, short, and band probe sources under `build/tmp/probes`
- [x] 1.1.2 Measure the base compiler on the local and module probes and record `.text`, net instructions, and C bytes
- [x] 1.1.3 Build `build/tmp/clc-band` with the 2–5 KB store form and measure `module-1024`/`module-1280`
- [x] 1.1.4 Build `build/tmp/clc-byte` with the 1-byte template form and measure `module-byte`
- [x] 1.1.5 Measure the short-list tail behavior in a loop and record the observed stale tail
- [x] 1.1.6 Record the measurements and decisions in [02-current-state.md](02-current-state.md) and [05-findings.md](05-findings.md)

**Phase 1 gate**: every residual has a measured number or a documented
decision.

---

## Phase 2: Implementation and Focused Tests

> **Phase baseline**: the Phase 1 probe record.
> **Lenses**: correctness, emission integrity.

### Step 2.1: Shared helper and local call site

- [x] 2.1.1 Split `emit_constant_array_template` into `emit_constant_array_template_data` (data) and the module-level wrapper (copy) — `compiler/src/codegen/program.rs`
- [x] 2.1.2 Add the top-level `Statement::Assign` local path; check eligibility before rendering the place and copy with `ArrayTarget.address`/`bytes` — `compiler/src/codegen/function.rs`
- [x] 2.1.9 Close the local-name collision found by the emission review: pass the function's local C names to the template helper and add the ST-11/IT-15 pins — `compiler/src/codegen/program.rs`, `compiler/src/codegen/function.rs`
- [x] 2.1.10 Keep the stores for an element target with a run-time or unknown index after the memory audit found GCC's `-Wstringop-overflow` on an out-of-range `memcpy`; add ST-12/IT-16 — `compiler/src/codegen/function.rs`
- [x] 2.1.11 Narrow the exclusion to `ElementTemp`/`Element(None)` so a constant-index element keeps the template; add IT-17 — `compiler/src/codegen/function.rs`
- [x] 2.1.12 Add the adjusted-array-parameter spec/impl cases from the correctness review (ST-13/IT-18) — `compiler/tests/constant_array_residuals_{spec,impl}_test.rs`
- [x] 2.1.3 Add the specification suite `constant_array_residuals_spec_test.rs` (ST-1–ST-13) — `compiler/tests/constant_array_residuals_spec_test.rs`
- [x] 2.1.4 Add the implementation suite `constant_array_residuals_impl_test.rs` (IT-1–IT-18) — `compiler/tests/constant_array_residuals_impl_test.rs`
- [x] 2.1.5 Run the impl suite against `build/tmp/clc-base` and record the red positive pins
- [x] 2.1.6 Run both new suites on the head compiler and re-run the existing constant-array suites
- [x] 2.1.7 Add `benchmarks/local-table/` (program, expected output, README) and update `benchmarks/README.md`
- [x] 2.1.8 Run `make bench-base COMMIT=7071761` and record the base benchmark JSON

**Phase 2 gate**: the positive pins are red on the base and green on the head;
the exclusion pins are green on both; the benchmark source runs identically on
both compilers.

---

## Phase 3: Verification and Reviews

> **Phase baseline**: the Phase 2 head.
> **Lenses**: correctness, emission integrity; memory safety only if a review
> finds a reason.

### Step 3.1: Focused and full gates

- [x] 3.1.1 Run `make check`
- [x] 3.1.2 Run `make asan`
- [x] 3.1.3 Run `make ubsan`
- [x] 3.1.4 Run `make bench` and `make bench-compare`; record the `local-table` delta and every other row
- [x] 3.1.5 Run `make bench-shape`
- [x] 3.1.6 Run `make corpus-amiga` and `make differential`
- [x] 3.1.7 Run the full `make verify` gate
- [x] 3.1.8 Dispatch the correctness and emission-integrity reviews over the fix diff; resolve every critical/major finding

**Phase 3 gate**: every gate is green and the reviews are clean of
critical/major findings.

---

## Phase 4: Documentation and Closeout

### Step 4.1: The record

- [x] 4.1.1 Update `docs/known-limitations.md` §13 with the function-local row, the re-measured 1-byte row, the 2–5 KB row, and the short-list tail note
- [x] 4.1.2 Write the final [05-findings.md](05-findings.md) with the benchmark baseline, test counts, review verdicts, and acceptance evidence
- [x] 4.1.3 Mark this plan and the roadmap row Done; update the roadmap progress count
- [x] 4.1.4 Close issue #18 with the measured decisions and the delivered change
- [ ] 4.1.5 Commit the change set and push `main`

**Phase 4 gate**: the plan, roadmap, known limitations, and issue agree with
the delivered code and the gate evidence.
