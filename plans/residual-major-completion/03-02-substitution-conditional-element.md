# Substitution Edges I: Conditional and Element Keys: Residual Major Completion

> **Document**: 03-02-substitution-conditional-element.md
> **Parent**: [Index](00-index.md)

## Overview

Three rows are per-instance substitution gaps: an opaque conditional loses both the arm-agreement
judgment and the null-arm rule, the nested element placeholder is never bound, and a substituted
element's member read at a view destination skips the concrete twin's widening. The first two are
checker/judgment gaps (one rejects, one keys), the third is an accepted emission that must match
its concrete twin.

## Design

### 1. The opaque conditional's per-instance judgments (`DEF-364`)

Two shapes are accepted and fail the strict build:

- `id(c ? a : b)` inside `choose<T, U>` with `T = i32`, `U = str`: the conditional's arms are
  opaque at the declaration; `unify_numeric` (`types/mod.rs:839`) returns the other side when one
  side is a type parameter, so the conditional gets a type and no disagreement is judged. At the
  instance the emitted C is a conditional with mismatched arms (`type mismatch in conditional
  expression`).
- `id(c ? x : null)` with `x: i32`: the null arm's type is swallowed by the same unification, so
  no `E10012` fires; the emitted C is a pointer/integer conditional and passes `void *` where
  `i32` is declared.

Frozen rules: Ch 04 §5 (`spec/04-expressions.md:108`) requires `?:` to have "same-typed branches"
under the usual implicit widenings; Ch 02 §5 TY-7 (`spec/02-types.md:127`) makes `null` where a
non-nullable type is required `E10012`; Ch 04 §14 (`:327`) makes an illegal implicit conversion
`E10200`. Ch 06 G-6 (`spec/06-functions.md:272`) and Ch 17 §7 (`spec/17-codegen.md:242`) make the
judgment per instantiation.

Fix: record a conditional site whose arm types still mention a type parameter (the enclosing
generic's instance carries it), and at each instance substitute both arms through
`instance_bindings` and apply the conditional rule with the **substituted** types: disagreeing
concrete arms report one `E10200` at the else arm (the concrete twin's site); a null arm at a
non-nullable substituted context reports one `E10012` at the `null`. For a conditional written
directly as an argument to a generic call, the call-site inference resolves the parameter
concretely, so the same re-check runs at the call site with the inferred parameter as the
expected type (the shared "judge the written argument at the resolved parameter" hook that
`DEF-360` and `DEF-371` use). The un-instantiated generic body keeps its current acceptance; the
report is once per site; the controls (identical arms, a non-null sibling, the legal nullable
instance) stay accepted.

### 2. The nested element placeholder (`DEF-365`)

`id(v[0])` inside a generic `f<T>` with `T = str[2]` emits `cl_id__#element of #element of T` and
fails the strict build (`stray '#' in program`) on host, hosted-amiga, and freestanding.
`instance_bindings` (`types/mod.rs:712`) binds `T` and `#element of T`, then pushes the next
level only when the element is an `Array` (:731-734). For `T = str[2]` the first element is `str`
(a text view), so the nested placeholder stays unbound. Ch 06 G-6 and Ch 17 §7 (`:242`, `:253`)
require one keyed copy per concrete instantiation; Ch 08 §3 makes a `str` index yield its byte.

Fix: extend the level chain whenever the element has an index surface — `Array`, `Str`,
`StaticStr`, `CString`, `Raw` — so `#element of #element of T` binds `index_element(str) = u8`
(`types/mod.rs:745`). The placeholder is only retained when the substituted element has no index
surface (the unknown case). Expected: the emitted key is `cl_id__u8`; the translation unit has no
`#`; the build is strict-clean and the run reads the byte (0).

### 3. The substituted element member at a view destination (`DEF-368`)

`let s: str = xs[0].name;` inside a generic `first<T>` with `T = S` (a `String` member) is
accepted and emits `s = xs[0].name` — `ClString` (a `void *`) into `ClStr` — while the concrete
`first(xs: S[1])` emits `s = cl_string_view(xs[0].name)` and runs 0. Ch 16 §2
(`spec/16-borrow-rules.md:51`) makes the implicit `String`→`str` widening a loan; Ch 06 G-6/Ch 17
§7 make the instance behave as its concrete twin; Ch 03 §2 makes the view lend copy the view and
share the storage. The accepted case is a conversion, not a rejection (the concrete twin accepts).

Fix: resolve the member read's type through the instance substitution (the `DEF-339`
`resolved_place_ty` machinery) and apply the destination conversion in the per-instance walk, so
a `String` member read into a `str` destination emits the concrete twin's `cl_string_view`
widening. Expected: the generic and concrete spellings build strictly and run the same value (2);
the emitted member assignment is byte-identical to the concrete twin's.

## Integration Points

- `compiler/src/sema/ops.rs:318` (`check_conditional`), `compiler/src/types/mod.rs:839`
  (`unify_numeric`), `compiler/src/sema/call.rs` (the call-site inferred-argument re-check),
  `compiler/src/own/instance_check.rs` (the per-instance conditional duty and audit).
- `compiler/src/types/mod.rs:702` (`element_param`), `:712` (`instance_bindings`), `:745`
  (`index_element`).
- `compiler/src/own/instance_check.rs` (the conversion duty), `compiler/src/own/call_check.rs`
  (the argument check), `compiler/src/codegen/expr.rs`/`codegen/function.rs` (the member
  emission), `compiler/src/codegen/expr.rs:290` (`resolved_place_ty`, the `DEF-339` helper).
- Controls: `probes/m364_control.clo`, `m365_elemkey.clo`'s `id(v.len)` shape,
  `m368_concrete.clo`; the origin checker/conversion cases.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| Disagreeing substituted conditional arms | One `E10200` at the else arm (the concrete twin's site); no TU | AR-10 |
| A null arm at a non-nullable substituted context | One `E10012` at the `null`; no TU | AR-10 |
| A conditional whose arms stay opaque (uninstantiated body) | No report here; the instance judgment owns it (the existing deferral) | AR-10 |
| A nested element of an unknown substitution | The placeholder stays; the existing unknown-shape rule governs | AR-11 |
| A member read that is not `String`→`str` | The existing conversion rules (no new conversion) | AR-12 |
| A member read at a non-view destination | No widening; the concrete twin's rule | AR-12 |

## Testing Requirements

- **Spec cases** (`checker_conformance_spec_test.rs`):
  - `should_judge_an_opaque_conditional_at_its_instance` (`DEF-364`): the two reds
    (`m364_condopaque`, `m364_nullarm`) report one `E10200` at the else arm and one `E10012` at
    the null; the identical-arm and non-null-sibling controls run 0.
  - `should_key_a_nested_element_from_the_substituted_element` (`DEF-365`): the `id(v[0])` case
    builds strictly and runs 0 on host; the emitted key is `cl_id__u8`; the `id(v.len)` control
    stays green; the hosted-amiga and freestanding emission checks are pin-level.
  - `should_widen_a_substituted_element_member_at_a_view_destination` (`DEF-368`): the generic
    and concrete spellings build and run 0; the emitted assignment matches the concrete twin.
- **Impl pins** (`checker_conformance_impl_test.rs`): the report code/count/span per conditional
  site; the keyed symbol and the absent `#`; the member widening text and its byte-identical
  concrete twin; a re-emission determinism pin.
- **Verification**: `make check`; the focused suite; the strict builds; the hosted-amiga leg for
  `DEF-365`/`368`; the freestanding compile for `DEF-365`.
- **Risks/lenses**: all four (AR-27). The memory lens probes the conditional values and the
  element member read (the ownership of the arm values and the view loan); the emission lens
  checks the keyed symbols and the concrete-twin byte identity.
