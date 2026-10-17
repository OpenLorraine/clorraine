# Substitution Edges II: Context Typing: Residual Major Completion

> **Document**: 03-03-substitution-context-typing.md
> **Parent**: [Index](00-index.md)

## Overview

Four rows are concrete context-typing gaps: a literal at a generic callee's concrete parameter,
the nullable scalar in a type-argument position, a qualified type path that loses its declaration,
and a negated literal at concrete destinations. Three are acceptances that must build; one
(`DEF-384`) is an illegal type argument that must report the frozen `E10011`. All four reuse the
existing context/introduction rules; no new conversion is added.

## Design

### 1. The literal at a generic concrete parameter (`DEF-371`)

`set<S[1]>(borrow a, 0)` reports `E10200: cannot convert 'i32' to 'usize'` while the concrete
`setS(borrow a, 0)` accepts: the literal is typed `i32` (its default) before the parameter check.
Frozen: Ch 01 LEX-14 (`spec/01-lexical.md:181`: "A literal is typed by its context; a decimal
literal defaults to `i32`"), Ch 04 §13 (`spec/04-expressions.md:291`: "A literal is typed by its
context"), and Ch 04 §5's literal rule ("A literal operand takes the other operand's type when its
value fits, otherwise E10202"). The concrete parameter `n: usize` is a context, even when the
callee has type parameters.

Fix: in the call argument check, when the declared parameter type is concrete (it does not
mention the callee's type parameters), check a literal argument **into** that parameter type
(pass it as the expected type) instead of checking it with no context and converting afterwards;
a literal that does not fit reports `E10202` at the literal. The generic callee's opaque
parameters keep the deferred route. Expected: `set<S[1]>(borrow a, 0)` accepted, build-clean,
run 0; `setS(borrow a, 0)` stays green; `set<S[1]>(borrow a, -1)` (or `1000000000000`) reports
`E10202` at the literal (the control). The same hook serves `DEF-364`'s call-site conditional
re-check (P2) and `DEF-360`'s written argument (P1); the phase keeps one implementation.

### 2. The nullable scalar type argument (`DEF-384`)

`Array<i32?>` is accepted and emits `cl_push__i32Q` with an `i32` parameter and a `void *`
argument (`passing argument 2 ... makes integer from pointer`); hosted-amiga emits the same.
Frozen: Ch 02 §5 TY-4 (`spec/02-types.md:124`: "`T?` is allowed only on handles, `raw`,
`cstring`, and views | E10011"). The existing helper `report_illegal_nullable`
(`own/program.rs:279`) checks declared fields only, and `validate_type`
(`sema/items.rs:131`) does not check nullability at all.

Fix: apply the TY-4 check to **every** resolved type node, including type arguments, with the
node's own span: extend `validate_type`'s recursion (or a parallel resolved-type walk called from
`resolve_type`, `sema/mod.rs:482`) so `ty.nullable && !is_nullable_capable(ty)` reports one
`E10011` at the offending node. `Array<String?>` (a handle) is the verified control, and the
phase also probes the legal nullable `raw`/`cstring`/view spellings; the probe shows
`Array<String?>` already keys (`ClArray__StringQ`), builds, and runs 0, so **no ABI key change is
made** — the row's key half is the control (AR-14). Expected: `Array<i32?>` one `E10011` at the
`i32?` span, no TU; the nullable-capable controls run 0. If the extension rejects a legal
nullable type (`Array<String?>` or a nullable view), the phase stops and narrows the check to the
invalid non-capable kinds (AR-34).

### 3. The qualified type path (`DEF-385`)

`let g: std.mem.Array<i32> = std.mem.arrayNew<i32>();` is accepted and emits `i32 g = ...`; the
method call then fails (`incompatible types ... 'i32' from 'ClArray__i32'`). The resolver
`resolve_named` (`types/table.rs:659`) resolves only a single-segment name, so the multi-segment
path becomes unknown and the declaration renders as the element type. Frozen: Ch 10 §5 MOD-13
(`spec/10-modules.md:112`: "Qualified access `A.B.f()` is always available, with or without an
import") and MOD-17 (module-level declarations resolve globally).

Fix: resolve a multi-segment named type through its declaration's own name (the last segment) in
the same table, exactly as the unqualified spelling: `std.mem.Array<i32>` is
`Struct("Array", [i32])`, emits `ClArray__i32`, and its methods resolve to the same instances. The
type table is keyed by simple name; a cross-module last-segment collision is a pre-existing table
limitation and is out of scope (AR-34), recorded as a residual if a probe shows it. Expected: the
qualified and unqualified spellings build strictly and run 0; the unqualified control's emission
is byte-identical.

### 4. The negated literal at a concrete context (`DEF-386`)

`let delta: i16 = -5;` (the spec's own example, `spec/02-types.md:49`) and `let a: i8 = -5;`
report `E10200: cannot convert 'i32' to 'i16'/'i8'`; the positive literals accept. Frozen: Ch 01
LEX-14, Ch 04 §13 (`:291`: "A literal is typed by its context"), and Ch 04 §5 (`:110`: unary `-`
yields the operand's "same type", wrapping for the minimum value).

Fix: `check_unary` (`sema/ops.rs:18`) accepts the expected context type from its `check_value`
caller and propagates it into the operand, so the integer literal takes the destination type and
the unary minus preserves it. Expected: `i16 = -5` and `i8 = -5` accepted, build-clean, run 0;
the positive literals stay green; a literal that does not fit the context reports `E10202` at the
literal (`let a: i8 = -1000;`); the `u8` spelling is accepted with wrapping (`-5` = 251; an
implementation pin, AR-16). The plan adds no new conversion and no new fit rule; the existing
literal-context and fit rules are applied to the unary operand.

## Integration Points

- `compiler/src/sema/call.rs:629` (`check_args`), `:671` (`check_args_impl`), the literal typing
  in `compiler/src/sema/expr.rs` / `compiler/src/sema/ops.rs`.
- `compiler/src/sema/items.rs:131` (`validate_type`), `compiler/src/own/program.rs:279`
  (`report_illegal_nullable`), `compiler/src/sema/mod.rs:482` (`resolve_type`).
- `compiler/src/types/table.rs:659` (`resolve_named`), `:602` (`resolve_ty_with`).
- `compiler/src/sema/ops.rs:18` (`check_unary`), the `check_value` expected-type plumbing.
- Controls: `probes/m371_control.clo`, `m384_nullablehandle.clo`, the unqualified `Array<i32>`
  shape, `probes/m386_control.clo`; the origin conversion and type-resolution cases.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A literal at a concrete generic parameter that does not fit | One `E10202` at the literal (the frozen fit rule) | AR-13 |
| A literal at an opaque parameter | The existing deferred/instance route (no new report at the declaration) | AR-13 |
| `T?` on a non-capable type in any type position | One `E10011` at the offending node; no TU | AR-14 |
| A nullable handle/view type argument | Accepted; the existing key and ABI | AR-14 |
| A qualified type path whose declaration is absent | The existing unknown-type/name error (`E10219`/`E10212`) | — |
| A negated literal out of range | One `E10202` at the literal (fit rule) | AR-16 |
| A negated literal in an unsigned context | Accepted with wrapping (the Ch 04 §5 rule); the pin records the value | AR-16 |

## Testing Requirements

- **Spec cases** (`checker_conformance_spec_test.rs`):
  - `should_context_type_a_literal_at_a_generic_concrete_parameter` (`DEF-371`): the accepted
    spelling runs 0; the concrete twin; the out-of-range literal reports one `E10202`.
  - `should_report_a_nullable_type_argument_on_a_non_capable_type` (`DEF-384`): one `E10011` at
    `Array<i32?>`; the verified `Array<String?>` control and the legal nullable `raw`/`cstring`/
    view spellings the phase probes run 0.
  - `should_resolve_a_qualified_type_path_to_its_instance` (`DEF-385`): the qualified and
    unqualified spellings build and run 0; the unqualified control is byte-identical.
  - `should_context_type_a_negated_literal` (`DEF-386`): the `i16`/`i8` shapes build and run 0;
    the positive control; the out-of-range `E10202`; the `u8` wrapping pin.
- **Impl pins** (`checker_conformance_impl_test.rs`): the literal's resolved type and the
  accepted emission; the `E10011` span at the type argument; the qualified type's emitted symbol;
  the unary operand's context; a re-emission determinism pin.
- **Verification**: `make check`; the focused suite; the strict builds; the hosted-amiga leg for
  `DEF-384`/`385`; the origin type-resolution and conversion cases.
- **Risks/lenses**: correctness + semantics + emission-integrity (AR-27). The semantics lens owns
  the TY-4/LEX-14 readings; the emission lens verifies the qualified instance and the byte
  identity.
