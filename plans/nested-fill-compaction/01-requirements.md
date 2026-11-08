# Requirements: Nested Fill Compaction (PLAN-22)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

This feature continues the measurement-first optimization programme. It uses
the existing generated-C benchmark harness to select the next hot shape, then
lands one emitted-code change with a measured win and no scored regression.

The measurement audit selects one shape: a nested constant-zero fixed-array
fill. `let a: i32[32][32] = [; [; 0]];` currently emits a 32-slot row temporary,
32 scalar stores, and 32 `memcpy` calls. On m68k that is about 4 KB of inlined
code and about 1,051 extra guest instructions for the matrix benchmark. A
single `memset` over the covered rows is byte-equivalent and measurably
smaller and faster.

The audit also finds that the existing comparison gate is hard-wired to
PLAN-14's sieve target and to a generic improvement rule. It cannot validate
this closeout, and it cannot even validate a flat baseline. Parameterizing the
gate is therefore part of this feature, before the optimization is measured.

## Scope Boundary

In scope: the baseline audit, the comparison-gate parameterization, the nested
zero-fill compaction, its behaviour and emission tests, the measurement
evidence, the required reviews, and the documentation closeout.

Out of scope: language, grammar, diagnostic, ABI, runtime-shim, and standard
library changes; new language features; DX, installer, and LSP work; Amiga
wrapper expansion; v2 language work; v1.1 `Box<T>` and trampoline work;
compiler compile-time speed; host wall time; safety trade-offs; unchecked
bounds; trap removal; new external crates; and build scripts.

## Functional Requirements

### R1 — Baseline and measurement audit

- **R1.1 Clean baseline.** Benchmark only a clean isolated clone. The plan documents
  are committed first, because an untracked `plans/nested-fill-compaction/`
  makes the harness record `dirty=true`. The base compiler source is commit
  `b68dae5`; the head compiler is built from the plan-docs commit, which
  changes no compiler file. `git status --porcelain` must be empty at both
  runs. Record the compiler path, SHA-256, commit, version, and `dirty` flag
  for both documents.
- **R1.2 Pre-change gate.** Run `make check` before changing any compiler
  file and record its result. A failure blocks the feature.
- **R1.3 Baseline documents.** Run `make bench-base COMMIT=b68dae5` and
  `make bench`. Retain `build/perf/base-clc`, `base.json`, and `head.json`.
  The base document must name `b68dae5` and the head document the plan-docs
  commit. Record both compiler SHA-256 values. Do not require the hashes to be
  equal: the base build runs in a nested worktree and a debug binary can embed
  a different path. A source-level difference (a compiler file changed) is a
  blocker; a path-only hash difference is recorded.
- **R1.4 Attribution.** Produce a per-benchmark attribution table from the
  emitted C and the streamed `vamos` trace. The method may map trace PCs to
  functions or count known basic blocks; it must not change any scored
  benchmark source, expected output, run count, or trace flag.
- **R1.5 Stale-gate evidence.** Run the pre-change `make bench-compare` on the
  flat baseline and record its failure. This failure is the evidence that the
  gate parameterization is necessary, not a regression.

**Acceptance:** `05-findings.md` holds the baseline and head identity table,
the baseline metrics, the attribution table, and the exact stale-gate output.

### R2 — Parameterized comparison gate

- **R2.1 Explicit requirements.** `bench.py compare` accepts a repeatable
  `--require NAME:METRIC:PERCENT` option. `METRIC` is `instructions` or
  `text`. The requirement passes when the head value is less than or equal to
  the base value multiplied by `(1 - PERCENT/100)`. `PERCENT` is a positive
  number; zero and negative values are rejected.
- **R2.2 Default behaviour.** Without `--require`, the comparison enforces
  only output equality, the presence of every required metric, the structural
  checks, and the existing no-regression rule (head must not exceed base by
  more than 1% in `instructions_net` or m68k `-O2` `.text`).
- **R2.3 Remove plan-specific rules.** Delete the hardcoded sieve instruction,
  `.text`, and C-byte targets and the generic "any benchmark improves by 20% or
  10%" rule. The active improvement expectation comes only from `--require`.
  The existing `test_compare_fails_on_c_byte_target` test is retired or
  rewritten as an invariant case; a C-byte value is advisory, not gated.
- **R2.4 Make target.** `make bench-compare` forwards a `REQUIRE` variable to
  the comparison command. An empty `REQUIRE` performs the invariants only
  (output, metric presence, structure, and the 1% no-regression guard). The
  initial invocation is
  `make bench-compare REQUIRE="matrix:instructions:5 matrix:text:3"`; Phase 1
  records the final values.
- **R2.5 Unknown names and metrics.** An unknown benchmark name or an unknown
  metric fails with a clear diagnostic before any comparison output.
- **R2.6 Unit tests.** Cover a met requirement, a missed instruction
  requirement, a missed text requirement, a flat comparison with no
  requirement, an invalid metric, an invalid percentage, the retired C-byte
  case, and the existing output, regression, missing-metric, and structural
  cases.
- **R2.7 Emission neutrality.** This change touches no benchmark source and no
  measurement path (emit, build, run, trace, or JSON schema). It changes only
  comparison interpretation. A base/head emission run over the corpus is
  byte-identical for this commit.
- **R2.8 Documentation.** The `AGENTS.md` command table describes the
  parameterized `make bench-compare` behaviour.

**Acceptance:** `make bench-test` passes; a flat comparison passes with an
empty `REQUIRE`; a deliberately missed requirement fails on the named metric;
the emission audit for the tooling commit is empty.

### R3 — Nested constant-zero fill compaction

- **R3.1 Compaction rule.** When all of the following hold, emit one `memset`
  for the covered rows instead of a row temporary plus one copy per row:
  - the array type has at least two extents and every extent is `Some` (a
    fixed array, not a slice view);
  - the fill chain is an `ArrayLit` at every nested level with an empty
    explicit-element list, ending in a fill value whose width-correct constant
    value is zero. A top-level prefix before the fill is allowed and is not
    part of the fill chain;
  - the zero test is `constant_fill_integer(value) == Some(0)`, optionally
    through a `Materialize` wrapper. A float-to-integer cast is not accepted
    by that helper and therefore does not compact;
  - the leaf element type is a non-float scalar (`i8`, `i16`, `i32`, `u8`,
    `u16`, `u32`, `usize`, `isize`, or `bool`);
  - the fill target has `compact == true`. The flag is false for a field
    projection, a `DerefTemp` alias, a volatile module array, and the
    materialized, conditional-assignment, and self-read callers; it is true
    for a local, an adjusted array parameter, a hidden return slot, a closure
    environment field, a compiler temporary, and a hidden-return conditional
    arm (a real return slot);
  - for a non-empty covered region, the covered region contains at least 64
    leaf slots, computed with `checked_mul`; a missing extent or an
    overflowing product does not compact. When `prefix >= extent`, a uniform
    zero fill emits nothing and the threshold does not apply.
- **R3.2 Spelling.** The emitted statement is
  `memset(<first covered row address>, 0, <covered rows> * <row size>);` with
  compile-time constants for the count and the row size. The row temporary is
  not declared and no per-row `memcpy` is emitted.
- **R3.3 Prefix.** With a prefix of `n` rows, the prefix rows keep their
  existing stores and the `memset` covers rows `n..extent` only. When
  `n >= extent`, a uniform zero fill emits nothing: no temporary and no
  unused marker, because a constant zero has no side effect to preserve.
- **R3.4 Exclusions.** Keep the current spelling for volatile targets, field
  projections, `DerefTemp` aliases, non-zero constants, float leaf types,
  aggregate or struct rows, rows with explicit prefix elements inside the
  fill chain, one-extent arrays, slice views with a missing extent, and
  covered regions below the threshold.
- **R3.5 Evaluation order.** Only constant zero fills compact. An impure fill
  keeps the evaluate-once temporary, its guards, and its current order.
- **R3.6 No trap change.** The fill is a constant zero with no operand guards,
  so no reachable trap is removed. The one-extent guard and compaction logic
  is unchanged.
- **R3.7 C validity.** The emitted C remains valid C99 for host, m68k GCC,
  vbcc, and freestanding. `memset` is already used by the one-extent byte-fill
  path and is declared by the emitted translation unit.
- **R3.8 Locality.** The change stays in the nested-fill branch of
  `emit_array_assign_to` in `compiler/src/codegen/function.rs` and reuses the
  existing compaction gate and threshold constants.

**Acceptance:** matrix emits one `memset`; every excluded form keeps its old
spelling; every behaviour case prints the same output as the base compiler.

### R4 — Tests

- **R4.1 Behaviour suite.** `compiler/tests/nested_fill_compaction_spec_test.rs`
  covers: a 2D zero fill; a 3D zero fill; a prefix row followed by a zero fill;
  a non-zero fill; a volatile target; a struct row; an address-taken array; an
  array parameter; and a hidden return slot. Each case compiles, builds, runs,
  and compares against an expected output recorded from a run on the
  unmodified compiler at `b68dae5`. The suite itself spawns only the head
  compiler (`CARGO_BIN_EXE_clc`); it does not depend on the gitignored
  `build/perf/base-clc`.
- **R4.2 Emission suite.**
  `compiler/tests/nested_fill_compaction_impl_test.rs` pins: exactly one
  `memset`; no `__cl_fillrow`; the size expression; the 63/64-slot boundary;
  each excluded form at a size above the threshold; the prefix boundary; and
  an unchanged one-extent fill.
- **R4.3 Red first.** Positive pins (compactable shapes) fail on the base
  compiler and pass on the head. Exclusion guards (non-zero, float, struct,
  volatile, field, row prefix, below threshold) pass on both; they fail if the
  implementation wrongly compacts the excluded shape. Record both sets.
- **R4.4 Test integrity.** Each positive pin must fail without the fix; each
  exclusion guard must fail when its exclusion is removed while the shape
  otherwise compacts. Spot-check by disabling the compaction condition and by
  removing one exclusion at a time.
- **R4.5 Sanitizers.** The behaviour suite carries the `RDC_ASAN` and
  `RDC_UBSAN` discovery markers. Its builder adds `-fsanitize=address` when
  `RDC_ASAN` is set and
  `-fsanitize=undefined -fno-sanitize-recover=all` when `RDC_UBSAN` is set,
  following `compiler_robustness_spec_test.rs`. `make asan` and `make ubsan`
  include it. The implementation suite is emission-only and needs no
  sanitizer.

**Acceptance:** the red/green record exists, both suites pass, and every
positive pin has a recorded base failure and every exclusion guard has a
recorded gate-removal failure.

### R5 — Measurement and acceptance

- **R5.1 Declared win.** The initial expectation is
  `matrix:instructions:5` and `matrix:text:3`. The probe that produced it was
  exploratory and its absolute C values did not reproduce byte-for-byte;
  Phase 1 task 1.3.3 re-runs the probe with `build/perf/base-clc` and records
  the exact commands, the observed C, and the final requirement. A change from
  the initial numbers is a ruling recorded in `05-findings.md`, not a silent
  edit. After the head commit, run `make bench` and
  `make bench-compare REQUIRE="<final requirements>"`. If a review or
  sanitizer fix changes codegen after that measurement, rerun both on the new
  clean commit; a stale head document blocks the closeout.
- **R5.2 No regression.** No scored benchmark may regress by more than 1% in
  `instructions_net` or m68k `-O2` `.text`. Output hashes must match.
- **R5.3 Emission audit.** Diff base and head emitted C over all seven corpus
  programs (four portable and three hosted-only), the five benchmark programs,
  and `examples/amiga/*.clo`. Every changed unit must be a nested zero fill;
  an unexplained diff blocks the closeout.
- **R5.4 Strict builds.** `make bench-shape` builds the shape fixture with
  m68k GCC, vbcc, and the freestanding profile. The fixture gains a nested
  zero-fill case, a nested `bool` zero-fill case, a prefix case, an
  array-parameter case, and a hidden-return case. `make corpus-amiga` builds
  and runs the portable corpus on
  hosted-amiga and builds the vbcc and FS-UAE-only programs; freestanding is
  build-only in `bench-shape` and its runtime corpus is covered by
  `freestanding_spec_test.rs` inside `make check`. No new warning.
- **R5.5 Gate set.** `make check`, `make asan`, `make ubsan`,
  `make corpus-amiga`, `make differential`, `make bench-shape`,
  `make bench-compare REQUIRE="<final requirements>"`, and `make verify` are
  green. The bare `make bench-compare` is used only for the flat baseline
  invariant check.

**Acceptance:** the measured matrix delta meets or exceeds the declared
requirement, no other benchmark moves by more than 1%, and every gate is
green.

### R6 — Reviews

- **R6.1 Codegen review set.** Dispatch `correctness-reviewer`, read-only
  `memory-safety-auditor`, read-only `emission-integrity-auditor`, and
  `performance-auditor` on the codegen phase.
- **R6.2 Packets.** Every packet includes a blast-radius step (every call site,
  exhaustive match, and parallel fill path) and a test-integrity step (the
  assertion fails without the fix).
- **R6.3 Finding gate.** Critical and major findings pause execution for a
  ruling in every commit mode. Minor findings are recorded.
- **R6.4 Tooling review.** The gate change receives a correctness review and a
  test-integrity check before the codegen phase starts. The review is a
  scheduled task in Phase 2 and its verdict is recorded in `05-findings.md`;
  Phase 3 does not start until the verdict is clean.

**Acceptance:** `05-findings.md` records every verdict with no open critical or
major finding.

### R7 — Documentation and closeout

- **R7.1 Findings ledger.** Record the baseline and head tables, the probe
  evidence, the red/green record, the review findings, the accepted limits,
  and the residual opportunities.
- **R7.2 Roadmap.** Add the new PLAN-22 row and move it to `Done` with the
  measured result; no row exists before this plan.
- **R7.3 AGENTS.** Update only the `make bench-compare` row for the
  parameterized requirement. No other command or layout fact changes.
- **R7.4 Known limitations.** Update only if the measurement boundary changes.
  The current boundary (guest instructions, not cycles; host time advisory)
  stays.
- **R7.5 No identifiers in code.** No plan, requirement, or task identifier
  appears in source or doc comments.

**Acceptance:** the docs match the measured result, the roadmap row is `Done`,
the tree is clean, and `git rev-list --count origin/main..HEAD` is zero after
the final push.

## Non-Functional Requirements

- **N1 Determinism.** Two runs of the same compiler produce identical
  instruction counts, section sizes, C sizes, and output hashes.
- **N2 Safety.** No ownership or drop change; no unchecked access; no removed
  trap; ASan and UBSan clean.
- **N3 Compatibility.** No language, ABI, runtime-shim, or stdlib change. The
  corpus and example programs build and run unchanged.
- **N4 Hygiene.** Temporary artifacts stay under `$CODEOPS_TMPDIR` or
  `/tmp/opencode`; none are committed. Generated C, benchmark JSON, captures,
  and licensed media stay untracked.
- **N5 Reviewability.** Each commit is buildable and its focused tests green.
  Each optimization commit is followed immediately by a clean-tree benchmark
  comparison recorded in the findings before the next phase; a failed
  comparison blocks the closeout. Docs commits are separate from
  implementation commits.

## Traceability

| Requirement | Evidence |
|---|---|
| R1 | `02-current-state.md`, `05-findings.md` baseline tables |
| R2 | `03-01-comparison-gate.md`, `tools/perf/tests/test_bench.py`, AGENTS row |
| R3 | `03-02-nested-fill-compaction.md`, emission diff |
| R4 | `07-testing-strategy.md`, red/green record |
| R5 | `05-findings.md` head table, gate logs |
| R6 | `05-findings.md` review tables |
| R7 | roadmap, AGENTS, index |
