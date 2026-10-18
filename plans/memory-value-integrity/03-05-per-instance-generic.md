# Per-Instance Generic Judgments: Memory and Value Integrity

> **Document**: 03-05-per-instance-generic.md
> **Parent**: [Index](00-index.md)
> **Rows**: `DEF-410` (major), `DEF-426` (major); `DEF-425` (major) is **parked** pending the
> user's ruling (AR-22)
> **Phase**: P5 · **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-31)

## Overview

Two generic bodies are accepted where their concrete or resolved-instance twins are not: a
conditional store of a by-value parameter moves an owning value with no lend duty (`DEF-410`), and
a compound division or remainder loses its `E11210` guard because the operand type is still an
unresolved projection (`DEF-426`). The fixes extend the existing per-instance lend audit and
resolve the guard's operand type. One row is a checker rejection (`E10034`, the concrete-twin
rule); one is an emission fix (the guard). No new diagnostic, no new audit family (AR-21, AR-23).
The third filed row, `DEF-425`, is **parked**: the pinned batch-M spec cases accept the body while
frozen BR-5 and the concrete/take twins require rejection (AR-22).

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
2. **`DEF-426`:** `division_guard_condition` (`codegen/guards.rs:1444-1460`) already calls
   `self.substitute(&left.ty)`, but a generic compound operand records the **unresolved projection
   type**, not the parameter type, so `is_integer()` is false or the scalar is unknown and the
   guard returns `None`. The concrete twin emits three guards (`d426d`); the generic emits zero
   (`d426e`) and `/0` SIGFPEs (`d426c`). The resolving hooks are `resolved_place_ty`
   (`codegen/expr.rs:321`) and `resolved_value_ty` (`:397`).
3. **`DEF-425` (parked, not a fix).** `w.items[i].name = w.items[j].name` with `borrow w: T` moves
   an owning element out of the borrow. Frozen BR-5 (`spec/16-borrow-rules.md:191`) and the
   concrete borrow twin (`d425b`, one `E10034`) and take twin (`d425d`, one `E10026`) require
   rejection, but the batch-M origin cases pin the generic body accepted at exit 0. An immutable
   spec-oracle change needs a user ruling (AR-22), so this row is parked with its evidence.

## Fix Approach

1. **`DEF-410`:** in `prepare_binding_effect` (`own/statements.rs:816-887`), when the conditional
   arm's type mentions a type parameter, record the arm as a **maybe-owning** source through the
   existing `MoveIfOwning` effect (or the equivalent `mark_moved_if_owning` duty) instead of
   letting `conditional_move_sources` skip it. The per-instance audit substitutes the instance
   type; an owning instance reports one `E10034` at the store, and the separate duplicable
   generic whole-move control stays accepted. Do not call `move_place` at the definition (it would
   report before substitution), and do not add a new effect kind. The duplicable control is
   `let r: T = h;` at `T = P` (the duplicable struct used by `q410dup`); the `String?` body cannot
   take a duplicable `T`.
2. **`DEF-426`:** resolve the unknown projection type before the guard: pass/derive the
   `resolved_place_ty`/`resolved_value_ty` of the compound operand into `division_guard_condition`
   (`codegen/guards.rs:1444-1460`). Both resolvers already exist at the head
   (`codegen/expr.rs:321`, `:397`); this row does not depend on a P4 output. Emit the guard at the
   instance, exactly as the concrete twin.
3. **`DEF-425` (park):** record the conflict and the ruling request in the phase-5 ledger and the
   close-out; make no code, test, spec, or registry edit. The park task is a bookkeeping task, not
   a fix task.

## Specification-First Cases (ST-16–ST-17)

`DEF-410` goes to `compiler/tests/borrow_generic_integrity_{spec,impl}_test.rs`; `DEF-426` goes to
`compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (AR-30). `DEF-425` has no new
case; its park evidence is the existing batch-M cases and the `d425a`/`d425b`/`d425d` probes.

| ST | Row | Case | Expected |
|----|-----|------|----------|
| ST-16 | `DEF-410` | `pick<T>(c, h) { let r: String? = c ? h : null; }` at `T = String`; the separate duplicable generic whole move (`let r: T = h;` at `T = P`, the `q410dup` struct); the concrete twin; the simple generic whole move at `T = String` | One `E10034` at the store for the owning instance; no TU; the duplicable generic whole move runs; the concrete twin keeps its report; the simple generic move keeps its report; no report at the definition |
| ST-17 | `DEF-426` | The generic and concrete `/=`, `%=` member and element twins; `/0`; `INT_MIN / -1`; `INT_MIN % -1` | The same guards in both twins (resolved through `resolved_place_ty`/`resolved_value_ty`); each fault traps `E11210` on the plain and ASan builds (inline `assert_traps`), no SIGFPE; the value results are unchanged |

The impl pins assert the `E10034` record/count/span, the duplicable acceptance, the guard count
and text in the generic and concrete emissions, and a re-emission determinism pin. The phase-5
record also stores the `DEF-425` park evidence and the ruling request.

## Verification

- `make check`; the borrow-generic and residue suites plain and with `RDC_ASAN=1`.
- The host ASan runs of the owning-instance probes (before the fix) and the accepted duplicable
  controls (after).
- The `assert_traps` cases for `E11210`; the frozen `run_fault_spec_test` data is untouched.
- The emission differential over the P5 generic/concrete probe pairs.
- The `DEF-425` park record: the batch-M case paths, the `d425a`/`d425b`/`d425d` probes, and the
  explicit ruling request (no code change).

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
  per-instance audit is the only reporter. The `E10034` record dedupe is one per site. The
  duplicable control is the generic whole move at `T = P`, not the `String?` body.
- **Risk (park):** a future ruling may authorize the `DEF-425` fix; until then no code, test, or
  spec edit is made. A silent fix is forbidden.
