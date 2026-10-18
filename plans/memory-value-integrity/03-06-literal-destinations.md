# Literal Destinations and Conditional Arrays: Memory and Value Integrity

> **Document**: 03-06-literal-destinations.md
> **Parent**: [Index](00-index.md)
> **Rows**: `DEF-445` (major), `DEF-446` (major), `DEF-448` (major)
> **Phase**: P6 · **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-31)

## Overview

Three accepted literal shapes fail the strict C build: a conditional-of-literals read base keeps a
brace list instead of binding the hidden view-array destination (`DEF-445`), a take conditional
literal in an eager ternary arm keeps the same brace form (`DEF-446`), and a take of a conditional
array literal of view places emits a bare `{ a, b }` expression (`DEF-448`). The fixes teach the
read/store predicates to recurse into conditional arms and route the ternary/conditional array
through the existing element-wise store form. No new ABI and no new diagnostic (AR-24–AR-26).

## Frozen Rule

- Ch 04 §3 (`spec/04-expressions.md:53`): evaluation is strictly left-to-right; a call's
  arguments evaluate in order.
- Ch 04 §5 (`:108`): only the chosen branch of `?:` evaluates.
- Ch 04 §13 (`:291`): a literal is typed by its context.
- Ch 04 §15: a literal's elements are typed by the expected type; an array literal has a C
  initializer form only when every element does.
- Ch 17 §5 (`spec/17-codegen.md:184-193`): a fixed view array is returned through a hidden
  destination parameter; the destination must be bound at every literal position.

## Root Cause

1. **`DEF-445`:** `literal_read_needs_stores` (`own/lower.rs:4839`) only accepts a `StructLit`;
   it returns false for a `Conditional` value at its own level.
   `literal_field_read_needs_stores` (`:4849`) already returns true for a fixed-array field that is
   not an `ArrayLit` (a conditional field included), and `array_element_read_needs_stores`
   (`:4864`) recurses through nested literals. A conditional-of-literals read base therefore
   returns false only at the top-level predicate; the codegen read path keeps the brace list, and
   a call-valued `str[2]` field is emitted as `n1()` with no destination (`too few arguments`).
   The mixed `ok ? place : literal` form is red for the same reason: the literal arm's hidden
   destination is unbound.
2. **`DEF-446`:** the `if`-condition/guarded path materializes the literal through
   `materialize_guard_literal` (`own/lower.rs:1726`); the eager ternary arm goes through the
   ordinary argument/ternary path (`materialize_owned_argument`, `:5238`), which keeps the
   brace-list literal; the same hidden destination is unbound.
3. **`DEF-448`:** the conditional array literal of `str[2]` view places lowers to a bare
   `ValueKind::Conditional` of `ArrayLit` arms; the C renderer spells a conditional expression as
   `(c ? { ... } : { ... })`, which is not a C expression. The fixed-array conditional assign
   machinery (`codegen/function.rs:1723`) exists for a target; the take-argument value position
   never enters it.

## Fix Approach

1. **`DEF-445`:** apply the **OR** rule to the literal read/store predicates: if any conditional
   arm needs the store form, the whole conditional uses it. Only `literal_read_needs_stores`
   (`own/lower.rs:4839`) returns false for a `Conditional` at its own level;
   `literal_field_read_needs_stores` (`:4849`) already returns true for a fixed-array field that is
   not an `ArrayLit`, and `array_element_read_needs_stores` (`:4864`) recurses through nested
   literals. Add the conditional-arm recursion to `literal_read_needs_stores` and name
   `struct_literal_needs_stores` (`codegen/function.rs:2071`) in the codegen step. The existing
   element-wise store path already binds the destination and calls into it; the change is the
   predicate/call site, not a new form. The mixed `ok ? place : literal` read base is red and must
   be covered: the literal arm's hidden destination is unbound even when the place arm is fine.
2. **`DEF-446`:** route the ternary-arm literal through the guarded literal store. When the take
   argument is a conditional whose arms are literals with a call-valued fixed-array field, bind
   the destination and emit the per-arm store form (the same mechanism the `if`-condition path
   uses). The chosen branch still evaluates only; each call runs once.
3. **`DEF-448`:** route the conditional array literal of places through the element-wise store
   form: bind a destination array at body/statement scope, branch on the condition, and store the
   chosen arm's elements into it (the `emit_fixed_array_conditional_assign` core). The take
   argument then passes the bound destination. No conditional expression of brace lists is
   emitted.

## Specification-First Cases (ST-18–ST-20)

All go to `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (AR-30).

| ST | Row | Case | Expected |
|----|-----|------|----------|
| ST-18 | `DEF-445` | The read base `(ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0]`; the mixed `(ok ? place : Inner { row: n2() }).row[0][0]`; the mixed nested field and nested element; the concrete brace-list control | All build strictly and run 0 (OR rule: any arm needing stores selects the store form); each call runs once; the concrete control is byte-identical |
| ST-19 | `DEF-446` | `g ? consumeO(ok ? Outer {...n1()...} : Outer {...n2()...}) : 0`; both selections; the `if`-condition control | The ternary form builds strictly and runs 0; each call runs once; the `if`-condition control stays green |
| ST-20 | `DEF-448` | `consume(take (ok ? [a, b] : [c, d]))` with `str[2]` places; both selections; the unconditional `[a, b]` control; the `String[2]`/`Box[2]` controls | The conditional form builds strictly and reads the chosen arm's values (run 0); the unconditional and other-type controls stay green |

The impl pins assert the bound destination array, the call-with-destination form, the copy into
the field/element, the per-arm store, the call counter, the mixed read base's store form, and a
re-emission determinism pin.

## Verification

- `make check`; the residue suite plain and with `RDC_ASAN=1`.
- The strict builds (the red evidence is the C compiler) and the hosted-amiga compile-only leg for
  the accepted forms.
- The emission differential over the P6 literal probe set; the batch-M `m378_*`/`m379_*` and
  phase-7 literal controls stay green.

## Risks and Lenses

- **Semantics** owns the evaluation order and the chosen-branch rule: only the chosen arm runs;
  each call once.
- **Emission-integrity** owns the destination binding, the per-arm store form, the call count, and
  the differential.
- **Memory-safety** owns the owned literal fields: the destination must release the unchosen/
  trailing owners once (the phase-7 body-scope rule).
- **Correctness** owns the blast radius: every `literal_*_needs_stores` caller, the unguarded
  codegen read path, the take-argument carrier, and the conditional fixed-array assign.
- **Risk:** recursing into conditional arms may force the store form for a literal whose arms both
  have complete C initializers; the concrete brace-list control is the byte-identity oracle.
- **Risk:** the conditional array destination must have a lifetime long enough for the take call
  and must release once; the ASan/LSan leg and the trailing-owner read-back are the oracle.
