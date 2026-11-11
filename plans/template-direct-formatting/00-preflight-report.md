# Preflight Report: Direct Template Formatting

> **Audit target**: `plans/template-direct-formatting/`
> **Date**: 2026-11-11
> **Mode**: auto-design (technical findings resolved under the delegated
> policy; the scope is directed by the active objective)
> **⚠️ SAME-SESSION REVIEW**: the plan and this report were produced in the
> same session; the probe evidence and the repository were inspected directly.

## Codebase Context Summary

| Area | Read | Establishes |
|---|---|---|
| Lowering | `compiler/src/codegen/expr.rs:1556`, `:1616` | `text_call`, `template`, the current owned-part emission |
| Builder | `compiler/runtime/shim_host.c:310`, `:333`; Amiga `:927`, `:950` | Current part, helpers, concat |
| Converters | `cl_text_i32/i32` bodies | The shim-only decimal writer this plan reuses |
| Tests | `string_integer_formatting_*`, `codegen_run_spec_test`, `module_initializer_integrity_impl_test` | The behavior oracle and the template-call pin |
| Harness | `tools/perf/bench.py`, `benchmarks/strings` | The scored metrics and the 1% guard |
| Reviews | `AGENTS.md` phase dispatch | Correctness + memory-safety + emission-integrity |

## Findings

### PF-001 — `.text` is close to the 1% guard on single-template benchmarks

- **Severity**: 🟡 MINOR
- **Dimension**: Non-functional / feasibility
- **Evidence**: the probe is +0.8% `.text` against the shim-only base for
  sieve/matrix/moves; `strings` shrinks. The guard is 1%.
- **Resolution**: the builder is kept to the 12-byte part and one direct-write
  pass; Phase 4 measures every benchmark and the change is rejected if any
  metric exceeds 1%. If needed, the scalar writer is moved behind one helper
  to reduce duplication. The probe already passes.
- **Status**: ✅ Resolved by the measured gate.

### PF-002 — Borrowed `String`/`str` lifetime

- **Severity**: 🟡 MINOR
- **Dimension**: Edge cases / memory safety
- **Evidence**: a borrowed part points at the source's bytes until the builder
  copies them in the same full expression. A source freed earlier would be a
  use-after-free.
- **Resolution**: the borrowed source is an operand of the same expression;
  compiler temporaries live to the end of the statement; `cl_str_part_string`
  evaluates the source once; ST-9 proves the result is independent after the
  source is reassigned; ASan/ownership probes cover the loop and temporary
  cases.
- **Status**: ✅ Resolved.

### PF-003 — Allocation test uses GNU `--wrap`

- **Severity**: 🔵 OBSERVATION
- **Dimension**: Testability
- **Evidence**: the host suite links with `-Wl,--wrap=malloc`; the repository's
  host CI is Linux/GNU ld and the suite is host-only.
- **Resolution**: accepted; the m68k leg is covered by the benchmark and the
  target boundary run.
- **Status**: ✅ Accepted.

### PF-004 — Float placeholders keep two allocations

- **Severity**: 🔵 OBSERVATION
- **Dimension**: Completeness
- **Evidence**: `cl_str_part_f64` wraps the existing `%g` converter, so a
  float template has the temporary plus the result.
- **Resolution**: accepted and documented; floats are not in the scored hot
  path and the `%g` contract is preserved (AR-04).
- **Status**: ✅ Accepted.

### PF-005 — Pre-existing `text` arity defect

- **Severity**: 🔵 OBSERVATION
- **Dimension**: Scope
- **Evidence**: issue #24; zero/two-argument `text` is accepted on the base.
- **Resolution**: out of scope; the explicit `text_call` path is unchanged.
- **Status**: ✅ Accepted.

### PF-006 — Three whole-translation-unit emission assertions

- **Severity**: 🟡 MINOR
- **Dimension**: Test integrity
- **Evidence**: `call_value_integrity_impl_test` searches for `do {` and
  `conditional_projection_residue_impl_test` for `} else {`; both search the
  whole emitted C.
- **Resolution**: the builder uses `while` and `continue`, no `do {` and no
  `} else {`; no existing assertion is changed. This is ordinary style, not
  evasion.
- **Status**: ✅ Resolved.

### PF-007 — Impl tests are written after the implementation

- **Severity**: 🟡 MINOR
- **Dimension**: Ordering
- **Evidence**: the specification-first protocol puts impl tests after the
  implementation; the red evidence is ST-1/2/3/4.
- **Resolution**: Phase 2 has the spec suite and the red run; Phase 3 writes
  the impl suite after the green spec run.
- **Status**: ✅ Resolved.

### PF-008 — No language or ABI change

- **Severity**: 🔵 OBSERVATION
- **Dimension**: Consistency
- **Evidence**: the result type, evaluation list, and converter signatures are
  unchanged; only the part contents and builder change.
- **Resolution**: no semantics reviewer; the emission-integrity auditor
  verifies the C diff (AR-06, AR-09).
- **Status**: ✅ Accepted.

## Verdict

`✅ PREFLIGHT PASSED WITH NOTES — 4 resolved, 4 accepted`

No 🔴/🟠 finding remains. The plan is ready for execution.
