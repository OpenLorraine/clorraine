# Per-Instance Generic Judgments: Memory and Value Integrity

> **Document**: 03-05-per-instance-generic.md
> **Parent**: [Index](00-index.md)
> **Rows**: `DEF-410` (major), `DEF-425` (major), `DEF-426` (major)
> **Phase**: P5 · **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-31)

## Overview

Three generic bodies are accepted where their concrete or resolved-instance twins are not: a
conditional store of a by-value parameter moves an owning value with no lend duty (`DEF-410`), an
element move through a borrowed parameter is not recorded (`DEF-425`), and a compound division or
remainder loses its `E11210` guard because the operand type is still the parameter (`DEF-426`).
The fixes extend the existing per-instance lend audit and feed the resolved operand type into the
division guard. Two rows are checker rejections (`E10034`, the concrete-twin rule); one is an
emission fix (the guard). No new diagnostic, no new audit family (AR-21–AR-23).

## Frozen Rule

- Ch 16 BR-5 (`spec/16-borrow-rules.md:191`): a lend parameter is call-scoped; the callee may not
  release or store it (`E10034`), except a `borrow T?` out-parameter slot.
- Ch 06 parameter modes (`x: T` / `const x: T` are read-only shared lends; `borrow` is a writable
  lend; `take` moves).
- Ch 07 STR-11/12 (`spec/07-structs.md:106-107`): partial moves are top-level fields of owned
  locals and `take` parameters; element moves are the v1 limit (`E10026`). The take/local rule is
  **not** the borrow-parameter rule.
- Ch 04 EX-5 (`spec/04-expressions.md:378`): integer `/`/`%` by zero or MIN/-1 faults `E11210`.

## Root Cause

1. **`DEF-410`:** `let r: String? = c ? h : null;` with `h: T` moves `h` when `T` owns. The
   concrete twin reports one `E10034` at the store (`d410b`); the generic body is accepted because
   `conditional_move_sources` (`own/expr.rs:175-205`) skips an arm whose type mentions a type
   parameter (`:191`), so `prepare_binding_effect` (`own/statements.rs:816`) returns
   `BindingEffect::None` and no lend duty reaches `check_not_lend_requirements`
   (`own/instance_check.rs:210-260`). The simple generic whole move (`let r: T = h`) already
   records the duty (`d410c`), so the machinery exists; only the conditional arm is dropped.
2. **`DEF-425`:** `w.items[i].name = w.items[j].name;` with `borrow w: T` moves an owning element
   out of the borrow. The concrete borrow twin reports one `E10034` (`d425b`); the generic body is
   accepted because `note_borrowed_element_move` (`own/place.rs:629-666`) accepts only a
   `BindingKind::Local` borrow, and the parameter's projected type is opaque at the definition, so
   `mark_moved_if_owning_at` (`:541-590`) records no path-bearing duty. The take twin's `E10026`
   is the **different** element-move limit (`check_element_move_requirements`,
   `own/instance_check.rs:448-483`) and must not be reused for a borrow parameter.
3. **`DEF-426`:** `division_guard_condition` (`codegen/guards.rs:1444-1460`) reads `left.ty`; a
   generic compound operand records the parameter type, so `is_integer()` is false or the scalar
   is unknown and the guard returns `None`. The concrete twin emits three guards (`d426d`); the
   generic emits zero (`d426e`) and `/0` SIGFPEs (`d426c`).

## Fix Approach

1. **`DEF-410`:** in `conditional_move_sources`, when an arm's type mentions a type parameter,
   record the arm as a **maybe-owning** source through the existing `MoveIfOwning` effect (or the
   equivalent `mark_moved_if_owning` duty) instead of skipping it. The per-instance audit
   substitutes the instance type; an owning instance reports one `E10034` at the store, a
   duplicable instance stays accepted. Do not call `move_place` at the definition (it would report
   before substitution), and do not add a new effect kind.
2. **`DEF-425`:** extend `RequiresNotLend` with an optional projection path, record the element
   move through a `BindingKind::Param` with `Mode::Borrow`, and have the audit substitute the
   parameter type, project the path (`project_place_path`), and report one `E10034` when the
   selected element owns. A duplicable element accepts. Do not reuse the `E10026` element-move
   audit and do not add a second element audit (AR-35(f)).
3. **`DEF-426`:** feed the resolved operand type into the guard: the compound store already
   resolves the destination/element type per instance (`emit_indexed_store`/`emit_assigned`); pass
   that type (or resolve `left.ty` through the instance substitution) into
   `division_guard_condition`. Emit the guard at the instance, exactly as the concrete twin.

## Specification-First Cases (ST-16–ST-18)

`DEF-410`/`DEF-425` go to `compiler/tests/borrow_generic_integrity_{spec,impl}_test.rs`;
`DEF-426` goes to `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (AR-30).

| ST | Row | Case | Expected |
|----|-----|------|----------|
| ST-16 | `DEF-410` | `pick<T>(c, h) { let r: String? = c ? h : null; }` at `T = String`; the duplicable instance; the concrete twin; the simple generic whole move | One `E10034` at the store for the owning instance; no TU; the duplicable instance runs; the concrete twin keeps its report; the simple generic move keeps its report |
| ST-17 | `DEF-425` | `copy<T>(borrow w: T, i, j) { w.items[i].name = w.items[j].name; }` at owning `T`; the duplicable instance; the concrete borrow and take twins | One `E10034` at the RHS element read for the owning instance; the duplicable instance runs; the concrete twin keeps `E10034`; the take twin keeps `E10026` |
| ST-18 | `DEF-426` | The generic and concrete `/=`, `%=` member and element twins; `/0`; `INT_MIN / -1`; `INT_MIN % -1` | The same guards in both twins; each fault traps `E11210` on the plain and ASan builds (inline `assert_traps`), no SIGFPE; the value results are unchanged |

The impl pins assert the `E10034` record/count/span, the duplicable acceptance, the guard count
and text in the generic and concrete emissions, and a re-emission determinism pin.

## Verification

- `make check`; the borrow-generic and residue suites plain and with `RDC_ASAN=1`.
- The host ASan runs of the owning-instance probes (before the fix) and the accepted duplicable
  controls (after).
- The `assert_traps` cases for `E11210`; the frozen `run_fault_spec_test` data is untouched.
- The emission differential over the P5 generic/concrete probe pairs.

## Risks and Lenses

- **Semantics** owns the parameter-mode reading: by-value/`const`/`borrow` are lends; `E10034`;
  `take` is the element-move limit (`E10026`). The concrete twins are the oracle.
- **Memory-safety** owns the accepted-program double free that the rejection prevents; the owning
  instance must report before any TU is produced.
- **Emission-integrity** owns the generic/concrete guard parity and the resolved operand type.
- **Correctness** owns the blast radius: every `conditional_move_sources` arm, every
  `mark_moved_if_owning_at`/`note_borrowed_element_move` caller, every `not_lend`/element duty
  consumer, and every division guard site.
- **Risk:** recording a generic conditional arm must not report at the definition; the
  per-instance audit is the only reporter. The `E10034` record dedupe is one per site.
- **Risk:** extending the element duty to a borrow parameter must not change the local
  borrow-element `E10026` family or the `take` twin; those controls are the oracle.
