# Requirements: Constant-Array Codegen Residuals (PLAN-36)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Status**: Approved for execution
> **Base**: `7071761` (post-PLAN-35; `clc 1.1.2`)

## Feature Overview

Issue #18 records four measured residuals from the constant-array closeout
(PLAN-25) plus one out-of-scope language item. The feature measures each
residual on one probe set, lands only a strict win, adds a scored benchmark for
the new shape, and keeps the accepted limits and known limitations current. It
does not change the language, the grammar, the ABI, the standard library, or
the runtime shims.

## Functional Requirements

### Must Have

- [ ] **R1 — Function-local template.** A function-local fixed-array binding or
  assignment whose value is a complete explicit list of guard-free constant
  values emits one block-scope `static const` template plus one `memcpy` into
  the writable destination, on `host`, `hosted-amiga`, and
  `hosted-amiga-13`. The declaration remains writable and keeps its address.
- [ ] **R2 — Eligibility parity.** The local path applies the same narrow rule
  as the module-level path: one extent, a 2- or 4-byte scalar leaf (`i16`,
  `i32`, `u16`, `u32`, `usize`, `isize`), at least 16 elements and 64 bytes, a
  complete explicit list (`fill` absent), every element a guard-free constant
  integer, a non-volatile target, and a non-freestanding profile.
- [ ] **R3 — Ineligible shapes keep stores.** A short or empty list, a fill
  form, a 1-byte leaf, a multi-extent array, a float/struct/view/handle/owned
  element, a guarded element, a volatile target, or the freestanding profile
  keeps the existing element-wise stores.
- [ ] **R4 — Module-level path unchanged.** Module-level constant arrays emit
  the same template and copy as on the base commit; the `lookup` benchmark
  output and metrics are unchanged.
- [ ] **R5 — 1-byte decision.** Re-measure the `u8` template against the store
  form at the committed probe size; keep the stores when the template costs
  more instructions, and record the numbers in the known limitations.
- [ ] **R6 — 2–5 KB band decision.** Re-measure the `i32[1024]` and
  `i32[1280]` store and template forms; keep the template when it wins both
  metrics at 1280 and record the exact trade at 1024. No size-aware cap is
  added.
- [ ] **R7 — Short-list tail policy.** Measure the tail of a short local list
  and record the observed behavior. No semantic change is made in this
  performance bundle.
- [ ] **R8 — Benchmark.** Add `benchmarks/local-table/` with a deterministic
  program, expected output, and README, and add the scored row to
  `benchmarks/README.md`.
- [ ] **R9 — Tests.** Add a specification suite for the behavior and an
  implementation suite for the emission. The existing constant-array suites
  stay green.
- [ ] **R10 — Documentation.** Update `docs/known-limitations.md` §13 with the
  function-local row and the re-measured 1-byte and 2–5 KB rows; update the
  benchmark suite table; keep the plan and roadmap in sync.
- [ ] **R11 — Acceptance.** `make check`, `make asan`, `make ubsan`,
  `make bench-shape`, `make bench-base`/`make bench`/`make bench-compare`,
  `make corpus-amiga`, `make differential`, and `make verify` pass. The scored
  comparison shows no regression outside the new `local-table` win.

### Should Have

- [ ] **R12 — Emission stability.** The module-level and unrelated emitted C is
  byte-identical between the base and head compilers.
- [ ] **R13 — Collision safety.** A user `ffi` symbol named `__cl_ctmpl_0`
  is not shadowed by a local template; the counter advances past it.
- [ ] **R14 — Scoped reviews.** The phase reviews use the correctness and
  emission-integrity lenses; the change surface is codegen only, so no
  semantics or memory-safety review is required unless a review finds a
  reason.

## Scope Boundary

**In scope:** the shared template helper; the function-local call site; the
new spec and impl suites; the `local-table` benchmark; the known-limitations,
benchmark README, plan, and roadmap updates; the issue closeout; the required
reviews and full gate.

**Out of scope:** module `const` arrays (`E10224`); a store-packing-aware
1-byte rule; a size-aware cap; a freestanding template; any language, grammar,
diagnostic, ABI, standard-library, or runtime change; any new crate, script
framework, profile, or build mode; version bumps or release publication.

## Measurement Requirements

- [ ] **M1 — One probe set.** Every comparison uses the same source values and
  the same shipped release flags; only the compiler or the eligibility rule
  differs.
- [ ] **M2 — Deterministic metrics.** The m68k dynamic instruction count and
  `.text` are the decision metrics; host time is advisory.
- [ ] **M3 — Recorded baseline.** The base probe and benchmark JSON are kept
  under `build/perf`/`build/tmp` for the plan record, and the committed
  benchmark baseline is written into `05-findings.md`.
- [ ] **M4 — No scored regression.** The `bench-compare` gate enforces output
  equality and the 1% net-instruction and `.text` limits.

## Acceptance Criteria

- [ ] Every R1–R14 item has a test, a recorded measurement, or a documented
  decision.
- [ ] The new spec suite passes under `make check`, `make asan`, and
  `make ubsan`.
- [ ] The new impl suite fails its positive pins on the preserved base compiler
  and passes on the head compiler.
- [ ] The scored comparison shows `local-table` improving and every existing
  row within the 1% guard.
- [ ] The full `make verify` gate is green before the plan is marked Done.
