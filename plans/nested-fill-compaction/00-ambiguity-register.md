# Ambiguity Register: Nested Fill Compaction (PLAN-22)

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)

All entries are resolved. No material ambiguity is open. Entries marked
`(user)` were decided by the user before requirements; entries marked
`(technical)` were resolved under the auto-design policy with the evidence
cited.

| ID | Decision | Kind | Resolution | Evidence |
|---|---|---|---|---|
| AR-1 | Base commit for the base/head comparison | Process (user) | `b68dae5`, the current HEAD at plan start. It isolates this optimization from the 64 commits since PLAN-14's closeout. | User decision 2026-11-08; `git merge-base --is-ancestor 02877f2 HEAD` |
| AR-2 | Push cadence under `--auto-commit` | Process (user) | Commit at each verified phase boundary (see AR-23); push only after the final full gate. | User decision 2026-11-08; goal commit rules |
| AR-3 | How to gate a win that is smaller than PLAN-14's sieve target | Technical | Parameterize the comparator with explicit `--require` entries; keep the >1% no-regression rule. Do not change benchmark sources. | `tools/perf/bench.py:610-626`; flat baseline cannot pass the old rule |
| AR-4 | Requirement syntax | Technical | `NAME:METRIC:PERCENT`, repeatable; metric is `instructions` or `text`; percent is a positive finite number. | [Comparison Gate](03-01-comparison-gate.md) |
| AR-5 | Compaction threshold | Technical | 64 covered leaf slots, reusing `FILL_LOOP_THRESHOLD` as a conservative floor. It is not a measured nested-`memset` optimum; the measured case covers 1,024 slots and the risk is recorded. | PLAN-14 measured unrolled stores faster than a loop at 16-63 slots; [Design](03-02-nested-fill-compaction.md) |
| AR-6 | Float leaf types | Technical | Excluded in this cut. The measured shape is `i32`; float folding has its own accepted limits. | `05-findings.md` accepted limits; PLAN-14 float rule |
| AR-7 | Prefix rows | Technical | Compact only `[prefix..extent)`; keep the prefix stores. | Same byte semantics, no extra temporary |
| AR-8 | Dirty main checkout | Process | Work in a separate git worktree at `b68dae5`; leave the user's `examples/hello/hello.clo` and `.vscode/` untouched. | `git status --porcelain` on the main checkout |
| AR-9 | How `make bench-compare` passes expectations | Technical | `REQUIRE` make variable expanded with `$(addprefix --require ,...)`; empty means invariants only. | [Comparison Gate](03-01-comparison-gate.md) |
| AR-10 | Whether to add a benchmark | Technical | No. The existing matrix benchmark already exposes the selected shape; a new benchmark would not add evidence and would risk tuning. | [Current State](02-current-state.md) |
| AR-11 | Where the compaction lives | Technical | Nested-fill branch of `emit_array_assign_to`; reuse `compact`, `constant_fill_integer`, and `ArrayTarget`. | `compiler/src/codegen/function.rs:4138-4173`, `:4284-4321` |
| AR-12 | Zero detection helper | Technical | `constant_fill_integer(...) == Some(0)`, the width-correct restricted helper. Never the untyped evaluator. | PLAN-14 critical finding on fill constants |
| AR-13 | Result when no row is covered | Technical | Emit nothing; no temporary and no marker. The fill is a constant zero, so skipping its evaluation is unobservable. `nested_zero_fill` returns true for a uniform zero fill with `covered_rows == 0`, so the early return is reachable; the 64-slot gate applies only to a non-empty region. | [Design](03-02-nested-fill-compaction.md); IP-17 |
| AR-14 | Top-level prefix versus the fill chain | Technical | A top-level prefix before the fill is allowed; the "empty explicit-element list" rule applies to the fill chain only. | R3.1; ST-03, IP-04 |
| AR-15 | Slice views | Technical | Require every extent, including the first, to be `Some`; a missing extent does not compact. | `TyKind::Array.extents`; IP-18 |
| AR-16 | Slot-count overflow | Technical | Use `checked_mul` for the row size and the covered leaf count; overflow rejects. | R3.1 |
| AR-17 | Exact zero predicate | Technical | `constant_fill_integer(...) == Some(0)`, optionally through `Materialize` at every level; float-to-integer casts are rejected by the helper and do not compact. | `codegen/expr.rs:1788-1806` |
| AR-18 | Closure-environment and compiler-temp targets | Technical | Allowed: the existing `compact` gate admits them and they name real storage. No nested-fill fixture exists in those positions, so no separate pin; recorded as an accepted limit. | `fill_target_compactable`; `05-findings.md` accepted limits |
| AR-19 | Provisional 5%/3% gate | Technical | Provisional from the exploratory probe. Phase 1 task 1.3.3 re-runs the probe with `build/perf/base-clc` (emit, patch, build, trace, record) and records the final requirement before implementation; a change is a ruling. | R5.1; PF-REQ-013/018 |
| AR-20 | Plan documents and the dirty flag | Process | Commit the plan documents before the baseline; the head compiler is built from that docs commit and no compiler file differs from `b68dae5`. | `bench.py:133-137`; PF-REQ-011 |
| AR-21 | Corpus audit scope | Technical | Audit all seven corpus programs (four portable, three hosted-only), the five benchmark programs, and the Amiga examples. | `spec/tests/corpus/`; PF-REQ-024 |
| AR-22 | Benchmark comparison versus the codegen commit | Process | Commit the codegen change first (a benchmark needs a clean tree), then run the head benchmark and record the comparison immediately; a failed comparison blocks the closeout and is fixed by a follow-up commit. This satisfies the comparison-recorded rule without benchmarking a dirty tree. | Goal commit rules; N5 |
| AR-23 | Commit cadence | Process | Commit at explicit phase boundaries: plan docs, baseline record, tooling, codegen (with suites and shape fixture), pre-gate findings, and post-gate evidence. "Per verified task" is replaced by these boundaries and each boundary is listed in the commit plan. | PF-PLAN-002/003/021 |
| AR-24 | Probe and emission-audit reproducibility | Technical | Phase 1 and Phase 5 retain emitted C and trace buckets under `build/perf/audit` and record the exact shell commands in `05-findings.md`. No new committed measurement tool is needed. | PF-PLAN-004/005 |
| AR-25 | Zero-test variants | Technical | Pin `0u`, `i32(0)`, `-0`, `false`, `Char(0)`, and reject `i32(f32(0.0))` in the helper unit tests; one nested spec case uses an integer-cast zero. | PF-PLAN-016 |
| AR-26 | Exclusion shapes | Technical | Exclusion guards use at least 72 slots and the same-size positive control IP-21 so the threshold cannot explain the guard; the boundary itself is pinned by 63/64 cases. | PF-PLAN-009 |
| AR-27 | Post-measurement codegen fixes | Technical | Any codegen change after the head measurement forces a fresh clean-tree `make bench`, `bench-compare`, and emission audit. A stale head document blocks closeout. | PF-PLAN-020 |
| AR-28 | Conditional hidden-return arms | Technical | A conditional hidden-return arm passes `compact=true` and may compact; conditional assignment stores pass false. The enumeration in R3.1 records both. | `cleanup.rs:754-780`; PF-PLAN-011 |

## Reserved Decisions

None. No ABI, ownership, language-rule, or public-surface change is proposed.
