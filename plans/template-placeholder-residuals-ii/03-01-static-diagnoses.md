# Static Diagnoses: text, bare null, and null-arm parity

> **Document**: 03-01-static-diagnoses.md
> **Parent**: [Index](00-index.md)
> **Governing requirements**: R1–R5, R10, R11 (see [01-requirements.md](01-requirements.md))

## 1. Explicit `text` Formattability

### 1.1 Rule

`text(v)` is the explicit spelling of the conversion a template placeholder
performs. The frozen formattable set (`str`, `static str`, `String`, integer,
float, `bool`; everything else E10500) applies to its argument:

- A null-shaped argument reports E10500 with `type 'null' is not formattable`.
- A concrete non-formattable argument reports E10500 with its displayed type.
- An opaque projection over a parameter (for example `text(x.n)`) reports
  E10500 at the projection, exactly as the template placeholder's opaque gate
  does.
- A nullable formattable argument at a concrete conversion reports E10010
  unless the flow narrowed it (the ordinary nullable-use rule).
- A generic argument records a deferred duty; each instance reports E10500 or
  E10010 as its substituted shape requires.
- No translation unit is written for a rejected argument.

### 1.2 Implementation

`compiler/src/sema/call.rs:276` (the compiler-known `text` branch) already
walks the arguments. Add the formattability judgment there, mirroring
`check_template` (`compiler/src/sema/expr.rs:1780`):

```rust
let null_shaped = crate::types::is_null_shaped(&ty);
let opaque = ty.is_unknown()
    && !ty.nullable
    && matches!(arg.value.kind, ExprKind::Member { .. }
        | ExprKind::Index { .. } | ExprKind::Qualified(_))
    && self.opaque_projections.iter().any(|projection| {
        projection.span.start >= arg.value.span.start
            && projection.span.end <= arg.value.span.end
    });
if null_shaped {
    self.error("E10500", "type 'null' is not formattable",
               "convert with `text`, `toHex`, or `toDecimal`", arg.value.span);
} else if (!ty.is_unknown() && !ty.is_param() && !formattable(&ty)) || opaque {
    self.error("E10500", format!("type '{}' is not formattable", ty.display()),
               "convert with `text`, `toHex`, or `toDecimal`", arg.value.span);
}
```

The help is the frozen registry help, unchanged for the explicit converter.

The own stage (`compiler/src/own/expr.rs`, the `ExprKind::Call` arm) applies
the nullable-use rule to a formattable argument:

```rust
let expected = Ty { kind: TyKind::Str, nullable: false };
self.check_value_into(&arg.value, &expected, arg.value.span);
```

This reports E10010 for an unnarrowed nullable argument and accepts a narrowed
one. It runs only when `crate::sema::formattable(&ty)` is true, so the
E10500 case is not duplicated.

### 1.3 Generic Argument Duty

A parameter type cannot be judged at the declaration. The own stage records
the existing `RequiresTemplatePlaceholder` duty for the argument when its type
is a parameter or contains one. The duty is reused because the rule and the
frozen E10500/E10010 surface are identical for a placeholder and an explicit
converter; its doc comment is widened to say so. The existing instance audit
(`compiler/src/own/instance_check.rs:2510`) substitutes the binding and
reports E10500 or E10010.

The record reuses the placeholder site set, so one argument records once. The
own stage recognizes the compiler-known call by the same predicate sema uses:
a plain `text` name with no local function binding, whose `resolved_callee`
is `Callee::Stdlib { name: "text", .. }`. A user function, an `ffi` declaration,
or a func-typed local named `text` resolves to `Callee::User`/`Callee::Ffi` or
a local and is skipped. A conditional argument records its arms, so the same
null-arm parity applies.

## 2. Bare Null Binary Operand

### 2.1 Rule

A null literal has the context type `T?` (`spec/04-expressions.md` §6). With
no expected type it has no operator surface. The frozen null rule
(E10012) reports when the operator is not `==`/`!=` with a null-capable or
null-shaped sibling:

| Expression | Result |
|------------|--------|
| `x == null`, `x != null` with a nullable-capable `x` | accepted |
| `null == null` | accepted (both null-shaped) |
| `null == 5` | E10012 at the null |
| `null + 1`, `null + null`, `null < null` | E10012 at the null |
| `null && flag`, `null || flag` | E10218 (the condition rule owns the report; the frozen spec names `null && true` as E10218) |

### 2.2 Implementation

`compiler/src/sema/ops.rs:115` (`check_binary`) excludes a literal
`ExprKind::Null` from the null-operand rule. Remove both
`&& !matches!(…, ExprKind::Null)` exclusions, keep the `Eq`/`Ne` null-capable
carve-out, and keep `LogicalAnd`/`LogicalOr` out of this rule so the frozen
E10218 condition report stays. Add a both-null branch: when both operands are
null-shaped and the operator is neither `==`/`!=` nor `&&`/`||`, report E10012
once at the left null. The returned type on the report path stays the current boolean/unknown so
later checks do not cascade.

## 3. Null-Arm Settlement

### 3.1 Rule

A conditional evaluates one arm. A null-shaped arm settles against its
sibling:

| Settled sibling result | Null arm | Diagnostic at the declaration |
|------------------------|----------|-------------------------------|
| view / `raw` / `cstring` / slice (non-nullable) | empty state | none |
| handle / `Box` (nullable `T?`) | legal null value | none; a use at a concrete position is E10010 |
| scalar / struct / other concrete non-nullable | illegal | E10012 |
| still a parameter or unknown | deferred | judged per instance |

The settlement applies when the conditional supplies its own context (a
placeholder, an inferred binding, or an argument with no written destination).
A written destination keeps `report_null_conditional_arm` (E10012), exactly as
before, including a written view destination: `let q: str = c ? null : v;` is
E10012, while `let q: str = null;` is accepted. That pre-existing distinction
is not changed by this plan and is pinned by a guard case.

At a generic instance the same settlement runs after substitution. A
placeholder adds the possible-null rule: a nullable settled result reports
E10010 at the null arm.

### 3.2 Shared Helper

Add to `compiler/src/types/mod.rs`:

```rust
pub enum NullArmSettlement {
    EmptyState,
    Nullable,
    NotAllowed,
    Deferred,
}

pub fn settle_null_arm(arm: &Ty, sibling: &Ty) -> Option<NullArmSettlement> {
    if !is_null_shaped(arm) {
        return None;
    }
    let result = conditional_result(arm, sibling)?;
    if result.is_unknown() || contains_param(&result) {
        return Some(NullArmSettlement::Deferred);
    }
    if result.nullable {
        return Some(NullArmSettlement::Nullable);
    }
    if accepts_bare_null(&result) {
        return Some(NullArmSettlement::EmptyState);
    }
    Some(NullArmSettlement::NotAllowed)
}
```

`conditional_result` already joins an unknown arm with its sibling through
`unify_numeric` (`compiler/src/types/mod.rs:1001`), so the helper sees the
settled sibling type. Add the bare-null predicate as part of this plan:

```rust
pub fn accepts_bare_null(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Raw(_) | TyKind::CString | TyKind::Str | TyKind::StaticStr => true,
        TyKind::Array { .. } => ty.is_slice(),
        _ => false,
    }
}
```

It deliberately excludes `Handle`/`Box`: those are nullable-capable but a
bare null needs their `?` form. The helper has a unit test covering views,
`raw`, `cstring`, slices, handles, `Box`, scalars, structs, and fixed arrays.
The own stage's private `null_capable_destination`
(`compiler/src/own/expr.rs`) delegates to this helper so the two stages share
one predicate.

### 3.3 Concrete Declaration

`compiler/src/sema/ops.rs:449` (`check_conditional`) adds a no-expected-type
rule after the arms are typed:

```rust
if expected.is_none() {
    if matches!(settle_null_arm(&then_ty, &else_ty), Some(NotAllowed)) {
        self.report_null_literal(then_value.span);
    }
    if matches!(settle_null_arm(&else_ty, &then_ty), Some(NotAllowed)) {
        self.report_null_literal(else_value.span);
    }
}
```

The bool feeds `ConditionalArms::null_reported` so the instance duty cannot
repeat the report. `report_null_literal` is the frozen E10012 helper. A binary
operand's `judge_conditional_argument` (`compiler/src/sema/ops.rs:616`) also
skips a site whose `null_reported` is already set, so an expression such as
`(c ? 1 : null) + 1` reports E10012 once, not twice.

### 3.4 Generic Instance

`compiler/src/own/instance_check.rs:2819` (`null_conditional_report`) takes
the sibling separately from the optional destination:

- A written destination keeps `null_arm_needs_report(arm, destination)`.
- No destination uses `settle_null_arm(arm, sibling)` and reports E10012 only
  for `NotAllowed`.

`check_template_placeholder_requirements` (`:2510`) replaces its
null-shaped-arm E10010 branch with the settlement:

- `EmptyState` → accept.
- `Nullable` → E10010 at the null arm (the placeholder needs a non-null
  value).
- `NotAllowed` → no report here: the instance conversion duty
  (`check_opaque_conditionals`) owns the E10012 at the same arm, so the site
  reports once.
- `Deferred`/`None` → no report.

A non-null arm keeps its existing E10500/E10010 judgments. A placeholder over
a `raw`/`cstring`/slice/`Box` arm is still E10500 because those types are not
formattable; the settlement acceptance applies to the null arm, not to the
placeholder's whole value.

### 3.5 Examples

```clorraine
let s: String? = stringOf("x");
text(s);                                  // E10010 at `s`
if (s != null) { text(s); }                // accepted
text(rec.n);                               // E10500 if `n` is an opaque projection
println($"{flag ? null : "x"}");          // accepted, empty line
println($"{flag ? null : stringOf("x")}");// E10010 at null
let x = flag ? null : 5;                  // E10012 at null
let y = flag ? stringOf("x") : null;      // accepted, y: String?
if (y != null) { use(y); }                // accepted
```

## 4. What Is Not Changed

- The E10500/E10010/E10012 registry text and help.
- The written-destination `report_null_conditional_arm` path.
- The formattable set and the #26 generic per-arm duty.
- `text` arity (separate mini-plan).

## 5. Invariants

1. No null-shaped, non-formattable, opaque-projection, or unnarrowed nullable
   `text` argument reaches codegen.
2. A literal null is accepted only in `==`/`!=` with a null-capable or
   null-shaped sibling.
3. A null arm at a bare-null-capable result is the empty state in a
   no-expected-type context; a written view destination keeps its E10012.
4. A null arm at a nullable handle result is a legal value; a concrete use
   reports E10010.
5. A null arm at a scalar/struct result reports E10012 exactly once.
6. The concrete and generic no-expected-type paths use the same settlement
   helper; written destinations keep the destination rule.
