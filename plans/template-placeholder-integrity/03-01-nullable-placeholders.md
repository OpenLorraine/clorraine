# Nullable and Generic Placeholders

> **Document**: 03-01-nullable-placeholders.md
> **Parent**: [Index](00-index.md)
> **Governing requirements**: R1, R4, R7 (see [01-requirements.md](01-requirements.md))

## 1. Rule

A template placeholder requires a concrete formattable value, so the existing
nullable-use rule applies:

- A `String?`, `str?`, or other nullable formattable placeholder that is not
  proven non-null reports **E10010**:
  `value '{name}' may be null here`, help `check `!= null` before the use`.
- A placeholder that is narrowed by an `== null` / `!= null` test or an
  early-return guard formats normally.
- A non-nullable placeholder is unaffected.
- No translation unit is written for a rejected program.

This is not a new language rule. `spec/02-types.md` TY-6 and
`spec/14-diagnostics.md` define E10010; the module-initializer path already
enforces it for templates (`compiler/src/own/lower.rs:1365`).

## 2. Implementation

The function-body ownership walk in `compiler/src/own/expr.rs:1123` becomes:

```rust
ExprKind::Template(pieces) => {
    for piece in pieces {
        if let TemplatePiece::Placeholder(inner) = piece {
            self.eval(inner);
            let expected = Ty { kind: TyKind::Str, nullable: false };
            self.check_value_into(inner, &expected, inner.span);
        }
    }
}
```

`check_value_into` is the existing nullability judgment
(`compiler/src/own/expr.rs:1427`). For a nullable value it calls
`check_nonnull_expr`, which:

- resolves a plain place or a place-rooted projection to its narrowed state
  (`check_nonnull_place`, which names the root binding),
- resolves a module path to its narrowed global state
  (`check_nonnull_global`),
- names a call result as `name()`,
- names a member rooted at a call by its field.

The expected type is `str` because that is the common non-null formattable
surface; the check only inspects nullability, never conversions or
formattability (sema already owns E10500).

## 3. Narrowing

The check must not reject a narrowed placeholder. Example:

```clorraine
let s: String? = stringOf("alpha");
if (s != null) {
    println($"{s}");       // accepted, prints alpha
}
```

`check_value_into` calls `check_nonnull_expr`, which consults
`state.place(id).is_narrowed(&projections)`. The existing `if` fact
application runs before the branch body is evaluated, so the placeholder sees
the narrowed state. A field path narrowed by `if (h.name != null)` behaves
the same.

A `String?` placeholder in an `unsafe` block is still E10010: `unsafe` governs
raw dereference, address-of, and volatile access, not the nullable-use rule.
This matches the module-initializer path.

## 3a. Generic Placeholder Duties

A placeholder whose declared type is a type parameter (`x: T`) has neither
formattability nor nullability at the declaration site, so neither judgment
can be made there. The function body records one per-instance duty with arm
records:

```rust
pub struct RequiresTemplatePlaceholder {
    pub function: String,
    pub ty: Ty,              // the written placeholder type, e.g. `T`
    pub span: Span,          // the placeholder expression
    pub narrowed: bool,      // the declaration's flow (non-conditional)
    pub name: String,        // `x`, `get()`, `name`, or `value`
    /// For a conditional placeholder, the two arms and their declaration
    /// narrowing; empty for a non-conditional placeholder.
    pub arms: Vec<TemplatePlaceholderArm>,
}

pub struct TemplatePlaceholderArm {
    pub ty: Ty,
    pub span: Span,
    pub narrowed: bool,
    pub name: String,
}
```

- `compiler/src/own/expr.rs` records one duty for every placeholder whose type
  is a parameter or contains one, before the generic instance is known. For a
  conditional placeholder, it records each arm's written type, span, and
  declaration narrowing (mirroring `check_value_into`'s conditional path);
  otherwise `narrowed` is the placeholder's own flow state. The record is
  deduplicated per source site, and the name is computed once from the same
  place/member/call rules the concrete check uses.
- The duty is transferred to `Monomorphization` with the other body duties
  (`compiler/src/own.rs`; add the field, the dedupe set, and the tuple entry),
  re-exported from `compiler/src/ir.rs`, and judged in
  `compiler/src/own/instance_check.rs`.
- For each instance, substitute the bindings and:
  1. If the substituted type is not **formattable** (`str`, `static str`,
     `String`, integer, float, or `bool`), report E10500
     (`type '{type}' is not formattable`) at the placeholder span. The
     `formattable` predicate is the sema rule
     (`compiler/src/sema/expr.rs:1864`), exposed to the instance check.
  2. For a conditional placeholder, judge each arm: if an arm's substituted
     type is not formattable, report E10500 at the arm's span; else if it is
     nullable and that arm was not narrowed, report E10010 at the arm's span.
     Accept when every arm is formattable and every nullable arm was
     narrowed.
  3. For a non-conditional placeholder, if the substituted type is nullable
     and the declaration did not narrow it, report E10010 at the placeholder
     span.
  4. A non-null, formattable instantiation reports nothing.
- Report once per offending site, like `check_len_operand_requirements`; a
  conditional arm is its own site.

An **opaque** placeholder (`x.name`, a conditional whose written type is
neither a bare parameter nor a named type) cannot be named at the declaration.
`sema::check_template` rejects it with E10500 at the declaration instead of
emitting a nonexistent converter at the instance. The rejection excludes a
**null-shaped** placeholder (`TyKind::Unknown` with `nullable`), which is the
`$"{null}"` case owned by AR-21 and is left unchanged. A bare `T` stays
per-instance, and an element parameter such as `x[0]` (`Param("#element of
T")`) is also per-instance and works.

Examples:

```clorraine
func show<T>(x: T): String { return $"{x}"; }
let s: String? = null;
show<String?>(s);            // E10010 at the $"{x}" site
show<H>(h);                  // E10500 at the $"{x}" site (H is a struct)
```

```clorraine
func show<T>(x: T): String {
    if (x != null) { return $"{x}"; }   // accepted for String?
    return stringOf("null");
}
```

```clorraine
func pick<T>(flag: bool, x: T, y: T): String {
    if (x != null && y != null) { return $"{flag ? x : y}"; }
    return stringOf("null");
}
// accepted for String? because both arms were narrowed
```

```clorraine
func name<T>(x: T): String { return $"{x.name}"; }
// rejected with E10500: the member type cannot be named at the declaration
```

This is the instance-level twin of the concrete checks; it uses the same duty
pattern as `RequiresLenOperand` (`compiler/src/ir/mono.rs`) and
`check_len_operand_requirements`
(`compiler/src/own/instance_check.rs:2442`). It is required for R1/R4: without
it, `show<String?>(null)` reaches `cl_string_copy(NULL)` and `show<H>(h)`
emits a nonexistent `cl_text_H`.

## 4. Duplicate Reports

- The module-initializer path runs `check_initializer_reads`, not the
  function-body `eval`, so the new check does not double-report a module
  initializer.
- Within one function body, each placeholder is a separate source position.
  `$"{s}{s}"` with an unnarrowed `s` reports at both positions, which is the
  existing "one report per source position" rule.
- A rejected concrete placeholder program stops before lowering. A rejected
  generic instance is found by the instance check after lowering the generic
  template but before emission, so no output C is written either.

## 5. What Is Not Changed

- The module-initializer check at `compiler/src/own/lower.rs:1365` stays as
  is; it already produces the required diagnostic.
- Sema's `check_template` gains only the opaque-type rejection (`is_unknown`
  and not null-shaped). It has no flow state and must not guess narrowing; the
  ownership stage owns the concrete judgment and the instance check owns the
  generic judgment.
- The null literal `$"{null}"` is **outside this plan** (AR-21). Its type is
  null-shaped (`TyKind::Unknown` with `nullable`), so the opaque rejection
  excludes it. It still reaches the driver as an internal "emitted C failed to
  compile" error; a separate follow-up issue is filed at closeout. Do not
  claim here that sema rejects it.
- The diagnostic registry and message text are unchanged (AR-20).

## 6. Invariants

1. No nullable placeholder reaches emission unless the flow state (concrete)
   or the per-instance duty (generic) proves it non-null.
2. A rejected placeholder program writes no C output.
3. A narrowed placeholder formats the runtime value; the nullable marker has
   no effect on the emitted C type or converter.
4. The check adds no runtime code, allocation, or ABI surface.

## 7. Verification

The cases are ST-1 through ST-5, ST-22 through ST-24, and ST-26/ST-27,
ST-32–ST-34 in [07-testing-strategy.md](07-testing-strategy.md). The phase
also re-runs the existing module-initializer test
(`def_zero_null_spec_test::should_judge_module_initializer_call_and_template_arguments`)
to prove the two paths agree.
