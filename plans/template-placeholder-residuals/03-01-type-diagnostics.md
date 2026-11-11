# Type Diagnostics: Null Literal and Conditional Nullability

> **Document**: 03-01-type-diagnostics.md
> **Parent**: [Index](00-index.md)
> **Governing requirements**: R1, R2, R6, R8 (see [01-requirements.md](01-requirements.md))

## 1. Null Literal Placeholder

### 1.1 Rule

A placeholder must hold a value with a formattable type
(`spec/04-expressions.md` §12: `str`, `String`, integer, float, `bool`;
any other type is E10500). The `null` literal has the null shape: `TyKind::Unknown`
with `nullable = true` (`compiler/src/types/mod.rs:711`). It has no concrete
formattable type in a context that supplies no expected type, so:

- A placeholder whose recorded type is null-shaped reports **E10500** at the
  placeholder with `type 'null' is not formattable`.
- No translation unit is written.
- A null literal in a conditional with a view arm is unaffected when the
  placeholder has no expected type: that conditional is typed as a `str` empty
  view and prints empty (AR-17). A written or resolved view destination
  (including a generic instantiation) keeps the existing E10012 rule. A null
  arm with a handle sibling makes the result nullable and reports E10010 at
  the null arm (AR-23).

This is the formattability rule applied to the one type shape it previously
skipped, not a new rule (AR-1, AR-21).

### 1.2 Implementation

`compiler/src/sema/expr.rs::check_template` (`:1770`) already computes each
placeholder's type and the `opaque` gate. Add a null-shape branch before the
opaque gate:

```rust
let null_shaped = crate::types::is_null_shaped(&ty);
if null_shaped {
    self.error(
        "E10500",
        "type 'null' is not formattable",
        "convert with `text`, `toHex`, or `toDecimal`",
        expr.span,
    );
} else if (!ty.is_unknown() && !ty.is_param() && !formattable(&ty)) || opaque {
    self.error(
        "E10500",
        format!("type '{}' is not formattable", ty.display()),
        "convert with `text`, `toHex`, or `toDecimal`",
        expr.span,
    );
}
```

The null branch reports once per placeholder. The opaque gate keeps excluding
null-shaped types (`!ty.nullable`), so a single placeholder cannot report twice.
The help is the frozen E10500 help from the registry; no new help text is
introduced (AR-21, PF-5).

### 1.3 Duplicate Reports

- A null literal has no other type diagnostic in a template; the type stage is
  the only judge.
- A conditional with two null arms is one null-shaped placeholder and reports
  once.
- A generic body that writes `$"{null}"` is checked at the declaration, so an
  uninstantiated body is rejected; the instance duty never sees it.
- The module-initializer path shares `check_template`, so it reports the same
  diagnostic.

## 2. Conditional Nullability

### 2.1 Rule

A conditional evaluates exactly one arm (`spec/04-expressions.md` §3). Its
result therefore has the null shape whenever any arm may be null. The nullable
flag is part of the type (`spec/02-types.md` TY-6), so the result keeps it:

```text
result = the settled arm type
result.nullable = result.nullable
    || (then.nullable && then is not null-shaped)
    || (else.nullable && else is not null-shaped)
    || (any arm is null-shaped && result is a handle/Box)
```

The null-shaped arm contributes in two ways:

- With a **view/`raw`/`cstring`/slice** sibling, the null is the empty state of
  a null-capable type (`spec/03-ownership.md` O7), and the result stays
  non-nullable. `flag ? null : "x"` remains an empty `str` and prints empty.
- With a **handle/`Box`** sibling (`String`), the result has the concrete
  nullable type `String?`; the null arm is a legal null, and a placeholder or
  store at a non-nullable context reports E10010 at the null arm. This is the
  issue's "type it as a concrete nullable type and E10010 applies" direction
  (AR-23). The concrete twin of #26 ST-37 (`flag ? x : null` with a narrowed
  `x`) reports E10010 at the null arm; ST-28 pins it.
- An **annotated** non-nullable handle destination
  (`let s: String = flag ? null : stringOf("x");`) keeps the existing
  `report_null_conditional_arm` E10012 at the null arm, because the written
  destination supplies the rule before the settled type is used. #27 does not
  change that pre-existing distinction.

A narrowed arm is judged by the flow state, so a program that checks a nullable
arm stays accepted; only an unnarrowed arm reports E10010 (AR-2, AR-11).

### 2.2 Implementation

`compiler/src/types/mod.rs::conditional_result` (`:1055`) has four success
returns. Settle each through one helper:

```rust
fn settle_conditional(then_ty: &Ty, else_ty: &Ty, mut result: Ty) -> Ty {
    let null_arm = is_null_shaped(then_ty) || is_null_shaped(else_ty);
    let then_nullable = then_ty.nullable && !is_null_shaped(then_ty);
    let else_nullable = else_ty.nullable && !is_null_shaped(else_ty);
    let handle_result = matches!(result.kind, TyKind::Handle(_) | TyKind::Box(_));
    result.nullable =
        result.nullable || then_nullable || else_nullable || (null_arm && handle_result);
    result
}
```

The settled kind must still be a legal conditional result; the flag does not
change the kind or the conversion checks. `check_conditional`
(`compiler/src/sema/ops.rs:449`) needs no new branch: it records the arm types
and returns the settled result, and the own-stage `check_value_into` applies
the E10010 rule to the nullable null arm. The written-expected
`report_null_conditional_arm` path is unchanged.

### 2.3 Interaction with Existing Checks

| Check | Interaction |
|-------|-------------|
| `check_conditional` (`compiler/src/sema/ops.rs:449`) | The result type now carries the flag; the written-expected null arm rule is unchanged; the sibling rule runs only when no expected type was written |
| `check_value_into` (`compiler/src/own/expr.rs:1441`) | `nullable_value_needs_narrowing` becomes true; the conditional path reports E10010 per unnarrowed nullable arm, including a null-shaped arm whose result is nullable (matching #26 ST-37) |
| Direct operand checks (`check_nonnull_expr`, `compiler/src/own/expr.rs:1270`) | An index base, a `len` operand, and any other direct nullable use judge a conditional's arms, so a flow-narrowed arm stays accepted; a bare `cstring` arm is empty-capable even without `?`, so it is judged too |
| Generic conditional duty (#26 `check_opaque_conditionals`, `compiler/src/own/instance_check.rs`) | The per-arm judgment is unchanged; the settled result may now be nullable, which the destination conversion sees |
| Narrowed concrete arm | `check_nonnull_expr` consults the flow state and accepts |
| Narrowed generic arm | `RequiresConditionalArmNarrowing` keeps `then_narrowed`/`else_narrowed`; #26 ST-34 must stay green |

### 2.4 Examples

```clorraine
let a: String = stringOf("alpha");
let b: String? = null;

println($"{flag ? a : b}");          // E10010 at `b` when not narrowed
let s: String = flag ? a : b;        // E10010 at `b` when not narrowed

if (b != null) {
    println($"{flag ? a : b}");      // accepted; the flow proves `b` non-null
}

println($"{flag ? null : "x"}");     // accepted; the result is a `str` empty view
println($"{flag ? null : stringOf("x")}"); // E10010 at the null arm (String?)
```

## 3. What Is Not Changed

- The E10500 registry entry, the E10010/E10012 messages/help, and their spans.
- The formattable-type list and the E10500 opaque-member rejection from #26.
- The generic per-instance duty and its per-arm records. A generic conditional
  whose null arm resolves to a view destination keeps its existing E10012
  behavior; the concrete placeholder with no expected type is the case this
  plan accepts as an empty view.
- The conditional arm conversion rules (E10200).
- The explicit `text(null)` path (AR-10, follow-up).

## 4. Invariants

1. No null-shaped placeholder reaches lowering or emission.
2. A conditional's static type is nullable whenever a non-null-shaped arm is
   nullable, and when a null-shaped arm meets a handle/`Box` result.
3. A null-shaped arm at a view/`raw`/`cstring`/slice result is the empty state
   when no expected type was written; a written or resolved view destination
   keeps its existing E10012 rule.
4. A null-shaped arm with a handle/`Box` sibling makes the result nullable; a
   placeholder reports E10010 at the null arm. An annotated non-nullable handle
   destination keeps E10012.
5. An unnarrowed nullable arm at a non-nullable context reports E10010
   exactly once per arm; a null-shaped arm at a nullable result is one of those
   arms, matching #26 ST-37.
6. A narrowed concrete or generic arm remains accepted.
7. A written expected destination and the generic instance duty keep their
   existing null-arm rules.
8. The checks add no runtime helper or allocation; an accepted nullable view
   may render the existing guarded view widening, not a new ABI surface.

## 5. Verification

The cases are ST-1 through ST-10, ST-24, ST-25, ST-26, and ST-28 in
[07-testing-strategy.md](07-testing-strategy.md). The phase also re-runs
`template_placeholder_integrity_spec_test` (#26), including ST-34 and ST-37,
and `def_zero_null_spec_test` to prove the existing nullable rules are intact.
