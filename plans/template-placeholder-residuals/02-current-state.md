# Current State: Template Placeholder Residuals

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## 1. Baseline

All reproductions below were run on `b6f12d8` (clean `main == origin/main`)
with `target/debug/clc` / `target/debug/clorraine` and, for the leak case, the
emitted C built with `-fsanitize=address`.

| Case | Source shape | Observed on `b6f12d8` | Expected |
|------|--------------|-----------------------|----------|
| Null literal | `println($"{null}");` | internal compiler error; emitted `cl_text_unknown(NULL)` | E10500, no C |
| Mixed conditional | `flag ? a : b` with `a: String`, `b: String?`, `flag = false` | exit 139 (SIGSEGV) | E10010 at `b` |
| Mixed store | `let s: String = flag ? a : b;` | accepted; unsafe when the null arm is chosen | E10010 at `b` |
| Null arm with a handle | `flag ? null : stringOf("x")` in a placeholder | accepted; exit 139 when the null arm is chosen | E10010 at the null arm (`String?`) |
| Zero-state field | `struct H { name: String; }; let h: H = H {}; println($"{h.name}");` | exit 139; emitted `cl_string_copy(h.name)` | empty line |
| Zero-state direct | the same `h`; `println(h.name);` | exit 139; `cl_string_view` dereferences NULL | empty line |
| For-step template | `for (let i = 0; i < 3; $"{i}", i = i + 1) { … }` | ASan: 60 bytes leaked in 3 allocations | no leak |
| Discarded template | `$"{1}";` in a block | ASan: 20 bytes leaked in 1 allocation | no leak |
| Aggregate order | `P { inner: Inner { a: mkString() }, b: $"{mark(2)}" }` | prints `seq=21` | `seq=12` |

The null-literal, mixed-conditional, and zero-state cases build a host binary
and then crash at run time or in the C compile. The leak cases are clean
without ASan, which is why the #26 suite did not see them.

## 2. Root Cause per Residual

### 2.1 Null literal placeholder

`compiler/src/sema/expr.rs:1770` (`check_template`) types each placeholder and
rejects a concrete non-formattable type with E10500. Its `opaque` gate is
`ty.is_unknown() && !ty.nullable && …` (`:1787`), so a null-shaped type
(`TyKind::Unknown` with `nullable`, `compiler/src/types/mod.rs:711`) falls
through. `compiler/src/codegen/expr.rs::text_call` then has no converter for
an unknown type and emits `cl_text_unknown`.

### 2.2 Mixed-nullability conditional

`compiler/src/types/mod.rs:1055` (`conditional_result`) returns
`then_ty.clone()` as soon as the two kinds match, dropping `else_ty.nullable`.
A conditional of `String` and `String?` is therefore `String`. The ownership
store check (`compiler/src/own/expr.rs:1441`, `check_value_into`) trusts the
type and does not see a possible null. A bare assignment `let s: String = b;`
already reports E10010, so the conditional is the outlier. A null-shaped arm
with a `String` sibling has the same gap: the type stage has no expected
destination to report against, so the null arm is accepted and crashes when
selected (`compiler/src/sema/ops.rs:449`, AR-23).

### 2.3 Zero-state String handle

A struct zero value (`spec/07-structs.md` STR-7) fills the `String` field with
the all-zero handle. `spec/03-ownership.md` O7 makes the zero state the empty
state for every droppable type, and `FreeString` is already null-safe. The
read accessors are not:

- `cl_string_ptr` (`compiler/runtime/shim_host.c:276`) dereferences the block.
- `cl_string_len` (`:282`) reads `handle->len`.
- `cl_string_copy` (`:305`) reads the block and copies it.

The Amiga shim mirrors all three (`shim_hosted_amiga.c:893`, `:899`, `:922`).
The generated `cl_string_view`
(`compiler/src/codegen/program.rs:221`) calls ptr and len, so a direct
`println(h.name)` crashes as well as a template placeholder.

### 2.4 Discarded template and for step

`compiler/src/own/lower.rs:1984` (`lower_discarded`) binds an owning result to
a statement temporary only for a direct `Call`/`ClosureCall`. A `Template`
value goes to `Statement::Discard`, whose rendered concat result is never
released. A bare `$"{1}";` therefore leaks one `String`.

`compiler/src/own/loops.rs` checks each `for` step expression with
`check_expr_statement` (`:332` in the speculative pass, `:394` in the real
pass) without the release-floor/drain every ordinary statement gets
(`compiler/src/own/statements.rs:159`). Even after the template result binds,
the pending release would stay pending; the step needs the same drain.

### 2.5 Owning aggregate literal order

`compiler/src/own/lower.rs:2830` (`pin_literal_operand`) pins a duplicable
operand, an owning call, and a fixed array, but deliberately leaves an owning
`StructLit`/`ArrayLit` inline because its structured store had no no-pre-drop
form. The template copy for a later field is emitted while that field is
lowered, before the outer C initializer evaluates the inline aggregate. The
aggregate's own field effects therefore run after the copy (`seq=21`).

`compiler/src/codegen/function.rs:3041` (`emit_struct_literal_store`) drops the
target before filling it, so reusing it for a compiler temporary would free a
value the literal already adopted on loop re-entry. A fresh temporary needs the
fieldwise fill without the pre-drop; `Statement::Adopt` (`:701`) already owns
the no-pre-drop store for a call.

## 3. Existing Contracts Reused

| Contract | Location | Use in this plan |
|----------|----------|------------------|
| Placeholder formattability | `compiler/src/sema/expr.rs:1770` | Reject a null-shaped placeholder with E10500 |
| Nullable store check | `compiler/src/own/expr.rs:1441` | Apply E10010 to a preserved nullable conditional |
| Conditional result | `compiler/src/types/mod.rs:1055` | Keep the nullable flag |
| Zero state | `compiler/runtime/shim_host.c` (`FreeString`), `shim_hosted_amiga.c` | Make the read accessors match the drop helper |
| Discard binding | `compiler/src/own/lower.rs:1984`, `bind_discarded_owner` | Bind an owned template result |
| Statement release floor | `compiler/src/own/statements.rs:159` | Drain after each for-step expression |
| Adopting store | `compiler/src/ir.rs` (`Statement::Adopt`), `compiler/src/codegen/function.rs:701` | Add the no-pre-drop structured store |
| Fieldwise store core | `compiler/src/codegen/function.rs:3078` (`emit_struct_literal_fields`) | Reuse for the adopted aggregate |
| Source-order literal pin | `compiler/src/own/lower.rs:2830` | Extend to an owning aggregate |

## 4. Baseline Metrics and Tests

The #26 closeout recorded `strings` at 1,221,232 net instructions, 23,972
`.text`, and 2,003 allocations on `96e7b1a`; `b6f12d8` is the same code plus
docs. The scored benchmark set is `lookup`, `matrix`, `moves`, `sieve`, and
`strings`.

Existing suites that touch this surface:

- `compiler/tests/template_placeholder_integrity_spec_test.rs` — 41 #26 cases.
- `compiler/tests/template_placeholder_integrity_impl_test.rs` — 15 #26 pins.
- `compiler/tests/template_direct_formatting_{spec,impl}_test.rs` — the #25
  allocation/emission oracle.
- `compiler/tests/def_zero_null_spec_test.rs` — nullability and module
  initializer behavior.

No suite covers a null literal placeholder, a mixed-nullability conditional,
a zero-state String, a discarded template, or an aggregate field before a
later placeholder. Those are the gaps the new suite fills.

## 5. Constraints

- `spec/` is the immutable oracle. O7, STR-7, TY-6, and Ch 04 §3 already state
  the required behavior, so no chapter or golden changes.
- The generated C must stay valid C99 on host and m68k; the host and Amiga
  shims must agree.
- The freestanding profile rejects templates at sema and has no `String`
  helpers; the new path must not add a freestanding dependency.
- The #26 ordered-lowering contract (`owned` pieces, `Statement::Adopt`, the
  module retained copy) must keep its guarantees.
