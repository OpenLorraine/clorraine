# Implementation Design

> **Document**: 03-implementation.md
> **Parent**: [Index](00-index.md)
> **Governing requirements**: R1–R10 (see [01-requirements.md](01-requirements.md))

## 1. Aliased and Qualified `text`

### 1.1 Rule

`text` is compiler-known but imported by name. Three spellings name the same
helper:

| Spelling | Resolution |
|----------|------------|
| `text(v)` with `import { text } from std.fmt` | intrinsic (existing) |
| `t(v)` with `import { text as t } from std.fmt` | intrinsic |
| `std.fmt.text(v)` | intrinsic only when no real declaration is reached |

A lexical or module-level value binding of the spelling still shadows it. An
import of the same local name from another module is not the intrinsic. A real
function, `ffi` declaration, or module global reached through
`std.fmt.text` wins over the intrinsic.

### 1.2 Sema

Factor the body of the compiler-known branch
(`compiler/src/sema/call.rs:276`) into a method:

```rust
/// Checks the compiler-known `text(v)` conversion.
///
/// The arity, formattability, and nullable-use judgments are the same for the
/// bare, aliased, and module-qualified spellings; only the written name
/// differs.
fn check_text_call(&mut self, type_args: &[Type], args: &[Arg], span: Span) -> Ty
```

Add two predicates:

```rust
/// True when a written local name denotes the `text` intrinsic.
fn is_text_intrinsic_name(&self, name: &str) -> bool {
    if self.text_name_is_shadowed(name) {
        return false;
    }
    if name == "text" {
        // A user `std.fmt` may declare `text` as an `ffi` function. The
        // imported-ffi path below does not reach `ffi_signature` (which reads
        // only the current module), so the intrinsic gate must exclude it.
        let imported_ffi = self.module.import(name).is_some_and(|binding| {
            !binding.poisoned
                && self
                    .qualified_ffi_signature(&binding.module, &binding.original)
                    .is_some()
        });
        return !imported_ffi
            && self.qualified_ffi_signature("std.fmt", "text").is_none()
            && self.qualified_decl_kind("std.fmt", "text").is_none()
            && self.table.global("std.fmt", "text").is_none();
    }
    self.module.import(name).is_some_and(|binding| {
        !binding.poisoned
            && binding.module == "std.fmt"
            && binding.original == "text"
            && self.qualified_ffi_signature("std.fmt", "text").is_none()
            && self.qualified_decl_kind("std.fmt", "text").is_none()
            && self.table.global("std.fmt", "text").is_none()
    })
}

/// True when a qualified path is the `std.fmt.text` intrinsic.
fn is_std_fmt_text_path(&self, segments: &[String]) -> bool {
    segments.len() == 3
        && segments[0] == "std"
        && segments[1] == "fmt"
        && segments[2] == "text"
        && !self.name_is_value("std")
        && self.qualified_decl_kind("std.fmt", "text").is_none()
        && self.table.global("std.fmt", "text").is_none()
}
```

`name_is_value("std")` covers lexical scopes, module declarations, and imports,
so a value head makes the path an ordinary dot/member access. The
`qualified_decl_kind` gate covers a user module named `std.fmt` that declares a
function, `ffi`, or global; the ordinary route then handles the call. The bare
branch becomes `if self.is_text_intrinsic_name(name) { return
self.check_text_call(...); }`. In the `ExprKind::Qualified` arm, after the
ordinary `table.function`/intrinsic/`ffi` handling and before the dot/module
fallbacks, add `if self.is_std_fmt_text_path(&name.segments) { return
self.check_text_call(...); }`.

### 1.3 Ownership

Extend `is_builtin_text_call` (`compiler/src/own/expr.rs:1613`):

- `ExprKind::Name(name)`: keep the local and module-global shadow checks. A
  current-module `func` declaration or an actual `ffi` *signature* of the
  spelling wins; an `ffi` block *name* is not callable and does not shadow. If
  an unpoisoned import binding has `module == "std.fmt"` and
  `original == "text"`, reject a real `std.fmt.text` declaration and otherwise
  return true. The import binding alone decides the alias; the program-wide
  simple-name fallback is not consulted.
- `ExprKind::Qualified(path)`: return true for the exact `std.fmt.text` path
  when `!self.is_dot_call(callee)`, no `Callee::User`/`Callee::Ffi` resolves
  through the path, and the target module has no `text` function or global.

`check_text_argument` then applies the nullable-use rule and the deferred
placeholder duty for all three spellings.

### 1.4 Lowering

In `lower_call` (`compiler/src/own/call.rs:336`), after the local/global
callee resolution and mode computation, canonicalize every intrinsic spelling
to the sentinel. `is_builtin_text_call` becomes `pub(super)` so the sibling
module can call it:

```rust
// The alias and module-qualified spellings name the same converter. The
// `$text` sentinel is rendered before any user-symbol resolution, so an
// unrelated function named `text` cannot capture the call.
let name = if self.is_builtin_text_call(callee) {
    "$text".to_owned()
} else {
    name
};
```

`render_call` already handles `$text` at its top
(`compiler/src/codegen/expr.rs:1170`) and dispatches to the same `cl_text_*`
selector as the template placeholder. The old name-based `match "text"` arm is
removed: it would otherwise convert a user `std.fmt` `ffi text` call that sema
correctly left as an ordinary call. The bare intrinsic, alias, and qualified
forms emit the same `cl_text_*` shape as before.

A module initializer has its own AST walk, so `lower_global` mirrors the same
recognition (`init_text_intrinsic_name`) before it builds the call: the bare,
alias, and qualified spellings canonicalize to `$text`, while a real
function/`ffi` signature/value declaration wins. The `std` value-head gate and
the `ffi`-block-name distinction match the type stage.

## 2. Generic Condition Context

### 2.1 Rule

A generic conditional used directly as a condition follows its concrete twin:
a non-bool settled result reports E10218 at the conditional span and does not
report the null arm. A bool settlement keeps the null-arm rule (E10012 for a
concrete null at a bool position). An arm-agreement failure keeps E10200.

### 2.2 Record

Add `pub condition: bool` and `pub judged_operand: bool` to
`ConditionalArms` (`compiler/src/sema/mod.rs:375`) and `OpaqueConditional`
(`compiler/src/sema/mod.rs:332`). `check_conditional` records both from its
one-shot flags; `check_module` copies them to the opaque record. A
`&&`/`||` operand is checked through `check_judged_condition_value`, which sets
the second flag.

### 2.3 Instance Audit

In `check_opaque_conditionals` (`compiler/src/own/instance_check.rs:2661`),
before the null-arm block:

```rust
// A conditional used as a condition follows the concrete E10218 rule: a
// non-bool settled type reports at the conditional, and the null arm is not
// also reported. A still-open pair defers; a bool settlement keeps the
// ordinary null-arm rule.
if conditional.condition {
    let settled = crate::types::conditional_result(&then_ty, &else_ty);
    let non_bool = settled.as_ref().is_some_and(|result| {
        !result.is_bool()
            && !result.is_param()
            && !crate::types::contains_param(result)
            && !(result.is_unknown() && !crate::types::is_null_shaped(result))
    });
    if non_bool {
        diagnostics.push(
            Diagnostic::error("E10218", "condition must be 'bool'", conditional.span)
                .with_help("compare explicitly, for example `x != 0`"),
        );
        break;
    }
}
```

A direct condition lets the condition rule own the site and breaks. A
`&&`/`||` operand also carries the operand judge's null-arm E10012, so its
E10218 is reported after the null block and then breaks; this reproduces the
concrete twin's pair and reports once across instances. The recorded
`conditional.span` is the whole conditional expression; its start equals the
concrete `require_bool` span, which is what the diagnostic and IT-4 pin.

## 3. Both-Null Conditional

### 3.1 Rule

A conditional whose two arms are the null literal has no concrete value type;
its value is the null value. It is accepted exactly where a bare `let z = null;`
is accepted, and every concrete use reports through the ordinary null rules.

| Context | Result |
|---------|--------|
| inferred binding / nullable destination | accepted as the null value |
| concrete destination or typed call argument | E10012 at each written null arm (existing) |
| placeholder | E10500 `type 'null' is not formattable` (existing) |
| condition | E10218 (existing condition rule) |

### 3.2 Why No New Settlement Variant

`settle_null_arm` returns `Deferred` for two null arms because there is no
sibling type to classify. A no-destination both-null site stays non-opaque:
`ConditionalArms::opaque` needs an open destination or a parameter
(`compiler/src/sema/ops.rs:539-543`), so the site is never recorded as an
`OpaqueConditional`. The `Deferred` return therefore has no consumer and no
instance duty exists. Adding a `NullValue` variant would be dead code: no
reporting branch distinguishes it from `Deferred`. The plan documents the
decision in the `settle_null_arm` rustdoc and pins the behavior with guard
tests. No behavior or emission changes.

### 3.3 Rustdoc

```rust
/// ...
/// Two null-shaped arms settle to `Deferred` because there is no sibling type
/// to classify. A no-destination conditional of that pair is not recorded as
/// an opaque duty (the opaque flag needs an open destination or a parameter),
/// so the value is accepted exactly like a bare `null` binding; a concrete
/// destination still reports each arm through [`null_arm_needs_report`].
```

## 4. Deferred Null Comparison

### 4.1 Rule

A left-literal `null == x` / `null != x` compared with a still-open parameter
defers to the instance. A substituted sibling that is not null-capable reports
E10012 at the null; a null-capable substitution (handle, `raw`, `cstring`,
view, slice, `Box`) is accepted. The right-literal `x == null` / `x != null`
form is the generic narrowing idiom and keeps the declaration's acceptance;
the conformance oracle pins that a generic body adds no report when the
instance is already judged at its null argument.

### 4.2 Record

Add to `compiler/src/sema/mod.rs`:

```rust
/// A literal-null comparison whose other operand still names a parameter.
///
/// The declaration cannot tell whether the instance's type is null-capable, so
/// the per-instance audit substitutes the sibling and applies the concrete
/// E10012 rule at the null literal.
pub struct OpaqueNullComparison {
    /// The enclosing function's name.
    pub function: String,
    /// The non-null operand's declared type.
    pub other_ty: Ty,
    /// The literal null's span.
    pub null_span: Span,
}
```

Add the vector to `SemaOutput` and `Checker`, and drain it in the per-module
loop in `sema::check` beside the other outputs
(`compiler/src/sema/mod.rs:478-486`):

```rust
opaque_null_comparisons.extend(checker.opaque_null_comparisons.drain(..));
```

`check_module` itself is unchanged. Record from `check_binary` after the
operands are typed and adapted:

```rust
if comparison {
    if let Some(function) = self.function_name.clone() {
        if matches!(left.kind, ExprKind::Null)
            && crate::types::contains_param(&right_ty)
        {
            self.opaque_null_comparisons.push(OpaqueNullComparison {
                function,
                other_ty: right_ty.clone(),
                null_span: left.span,
            });
        }
    }
}
```

### 4.3 Instance Audit

Add `check_opaque_null_comparisons(program, sema, mono, diagnostics)` next to
`check_opaque_conditionals`. For each requirement, resolve the function's type
parameter names, iterate its instances, substitute `other_ty`, skip a still-open
result, accept `is_nullable_capable`, and otherwise report E10012 at
`null_span` once per site:

```rust
diagnostics.push(
    Diagnostic::error("E10012", "'null' is not allowed here", requirement.null_span)
        .with_help("declare the type `T?` or avoid the empty state"),
);
```

Skip a still-open substitution with `other.is_unknown() || other.is_param() ||
contains_param(&other)` (an unknown type is nullable-capable but carries no
concrete instance). Use a reported-site set keyed by `(function, file, start,
end)` so the same literal cannot report twice if the site is recorded more than
once. The loop continues to later instances until the first report; a report
ends the site.

## 5. Null-Safe Cstring Length

### 5.1 Rule

`cstring` is nullable-capable, so a null cstring is its empty state and its
length is 0 (O7). `cl_cstring_len(NULL)` returns 0 in every shim; `strlen` is
called only for a non-null pointer. The panic then passes a zero length and the
shipped `cl_panic` guard skips the message write. A language-level
`len(cstring)` already reports E10010 for a possibly-null operand, so this
guard is observable through the panic path.

### 5.2 Implementation

In `compiler/runtime/shim_host.c:48` and
`compiler/runtime/shim_hosted_amiga.c:552`:

```c
static inline size_t cl_cstring_len(const char *text)
{
    /* A null cstring is the empty state; strlen has no null contract. */
    return text == NULL ? 0 : strlen(text);
}
```

`compiler/runtime/shim_freestanding.c:230` has no `NULL` definition and uses
`unsigned long`, so its guard is `text == 0` with the same comment.

Each shim keeps its own return type (`size_t`/`unsigned long`) and its own
`strlen` implementation. `emit_panic` needs no change.

## 6. For-Step Fixed-Array Assignment

### 6.1 Rule

A fixed array has no C assignment or value-return form. The compact `for`
header cannot carry the hidden destination argument or the element copy, so
any `Statement::Assign` whose target is a fixed array takes the flat
label/`goto` loop. The ordinary store path then emits the valid form.

### 6.2 Implementation

In `for_step_is_structured` (`compiler/src/own/loops.rs:1005`), add
`&& !target.ty.is_fixed_array()` to the `Assign` arm. A non-array target keeps
the compact header; an array literal already took the flat path.

## 7. Invariants

1. Every spelling of the intrinsic lowers through `$text` to `cl_text_*`; an
   alias or qualified path never reaches C as a function name, and an unrelated
   user function named `text` is never captured.
2. A user `std.fmt.text` declaration wins over the intrinsic.
3. A generic conditional in a condition reports exactly one of E10218 (non-bool
   settlement) or the ordinary null-arm rule (bool settlement), matching the
   concrete twin.
4. A both-null conditional is accepted as the null value; its behavior matches
   a bare null value and its settlement remains consumer-free.
5. A left-literal null comparison reaches the instance and reports E10012
   exactly once for a non-null-capable substitution; the right-literal
   narrowing form is unchanged.
6. `cl_cstring_len(NULL)` is 0 in every shim; no zero-length panic writes a
   null buffer.
7. A fixed-array for-step assignment emits the flat store, never the compact
   header.
8. The #24/#28 arity, formattability, condition, settlement, loop, and panic
   contracts stay green.
