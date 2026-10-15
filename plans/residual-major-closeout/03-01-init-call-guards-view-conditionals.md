# Initializer Call Resolution, Guards, and Nullable View Conditionals: Residual Major Closeout

> **Document**: 03-01-init-call-guards-view-conditionals.md
> **Parent**: [Index](00-index.md)

## Overview

Five rows in the module-initializer emission batch F left open (and the batch G memory audit
filed): the expression callee, the inferred generic instance (two spellings), the nullable view
conditional's null arm, and the guarded right-side owned bind's release scope. `DEF-209`'s
projection-root shape is the phase's green control (already fixed at the head, AR-3). The
mechanism is the second, diminished lowering path batch F began converging on the body path: the
initializer call arms (`codegen/program.rs`), the monomorphization walk's explicit-only rule
(`own/generics.rs`), the conditional view arms (`codegen/expr.rs`/`function.rs`), and the
init-owner release pass. Every fix mirrors a verified body-path contract into the init path or
scopes an existing release; none adds a pass (AR-14).

## Design

### 1. The expression callee at an initializer (`DEF-196`)

`let h: i32 = mk()(5);` emits `h = (5);` — the call collapses to its argument because
`callee_name(callee)` returns `None` for a call-result callee and the lowering carries the empty
name (`codegen/program.rs:2394`/`:2418`). The body path lowers a call-result callee into the
call's callable value; the conditional spelling `(c ? f : g)(5)` collapses the same way.

Mirror the body's expression-callee resolution in the initializer lowering: when the callee is
not a name, qualified name, or member, lower it as a callable value and attach it as the call's
callable/receiver (the `ValueKind::Call` shape the emitter already understands for
func-typed-global and field callees), so the emitted C carries the callee value and the
arguments. The two probed spellings (`mk()(5)`, `(c ? addOne : addOne)(5)`) both store the
call's result (run exit 0, `h = 6`).

### 2. The inferred generic instance at an initializer (`DEF-197`, `DEF-220`)

`let g: i32 = pick(1, 2);` and `let e: i32 = viewOf(slice(xs, 0, 4))[1];` emit the plain
symbol with no instance (strict build fails). `own/generics.rs::collect` records only explicit
type arguments (`record_call` returns early for `type_args.is_empty()`,
`own/generics.rs:378-395`), while the type stage inferred `T = i32` and the body path's
`infer_call_types` (`own/call.rs:421`) derives the arguments from the checked argument types
via `sema::call::match_param`.

Extend the initializer call recording with the body path's bounded inference: when an
initializer call (top level and the positions the walk reaches) writes no type arguments and
the callee is a user generic, derive the inferred arguments with the same helpers and record
the instance, so the monomorphization declares the keyed function and the emitter spells the
keyed callee. The walk never guesses: a call whose arguments do not determine every parameter
keeps the current path (and the type stage's own judgment governs). The explicit-type-argument
path and the initializer-lambda body (both already working) stay byte-identical controls.

### 3. The nullable view conditional (`DEF-207`)

`let s: str? = c ? stringOf("abc") : null;` emits one `cl_string_view(__cl_owned0)` after the
conditional; on the null arm `__cl_owned0 = NULL` and `cl_string_view(NULL)` reaches
`cl_string_ptr(NULL)` — a null dereference (run segfault 139, body and initializer alike).

Each arm constructs its own view: the owned arm wraps the handle (`cl_string_view(...)`), the
null arm carries the zero view (`cl_zero_ClStr()` or the equivalent zero struct), so the chosen
arm's nullness is preserved and the destination's null test works. A conditional at a nullable
view destination keeps the arms' own nullness (an owned arm is non-null, a null arm null);
the `widen_text_view`/store paths (`codegen/expr.rs:472`, `codegen/function.rs:2069-2093`)
and the initializer spelling (`codegen/program.rs:1058`) take the per-arm shape. The null test
and the length read then run correctly on both selections (run 9 on the null arm, 3 on the
taken arm), ASan-clean.

### 4. The guarded right-side owned bind (`DEF-219`)

`let hit: bool = yes && (firstByte(stringOf("ab")) == 97);` declares the owned bind
`ClString __cl_itmp1` inside the `if` block that guards the right side, but the init-owner
release pass (`codegen/program.rs:978-984`) emits its release after the block, so the strict
build fails (`'__cl_itmp1' undeclared`).

Keep the owned bind's release inside the block that declares it, or declare the bind outside
the decision: the init-owner pass gains a scope-aware form so a bind created inside a guard
block releases at that block's end (the ordinary `emit_drop_expr` shape, null-guarded), while
the top-level binds keep the reverse-order end-of-init release. The accepted program builds and
runs `hit == true`; the untaken path releases nothing.

### 5. The green control (`DEF-209`)

The head binds `__cl_owned0 = mkBox()` for `mkBox().count` and releases the owned `name` member
at the statement's end (the let, return, argument, and module-initializer spellings verified
strict-build/run/ASan-clean). The phase pins these shapes as controls so the P1 changes cannot
regress them; no implementation work is expected. If a close-out probe shows a regression, the
row becomes a red case in this phase with the new evidence (AR-5).

## Integration Points

- `codegen/program.rs`: the init call arms (`:2380-2440`), `callee_name` (`:2938`),
  `emit_init_functions` (`:961-984`), the guarded bind (`:1465-1480`).
- `own/generics.rs`: `collect` (`:30`), `record_call` (`:378-395`); `own/call.rs`
  `infer_call_types` (`:421`); `sema/call.rs::match_param` (`:881`).
- `codegen/expr.rs:472`, `codegen/function.rs:2069-2093` (`widen_text_view`),
  `codegen/program.rs:1058` — the per-arm view construction.
- The body contracts to mirror: `own/call.rs::lower_call` (`:203`), the store/arm shapes the
  function path already spells.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| An expression callee with no callable value (an unresolved name) | The existing unresolved-callee diagnostic; the probed spellings resolve | — |
| An inferred call whose arguments do not determine every parameter | No instance is guessed; the type stage's judgment governs | — |
| A null arm at a non-nullable view destination | The existing possibly-null conversion judgment | — |
| A guarded bind that owns nothing drop-needing | No release registers (the existing owner predicate) | — |
| A `DEF-209` shape regression | The row turns red with the fresh probe and is fixed in this phase | AR-5 |

## Testing Requirements

- Spec: the expression callee (both spellings; run value proves the call ran), the inferred
  instance (both spellings; the strict build proves the keyed instance), the nullable
  conditional (both selections, the null test, the length read; ASan), the guarded bind
  (strict build, run value, ASan), and the `DEF-209` control shapes (strict build, run, ASan).
- Impl: the emitted callee/instance spellings (`cl_..._pick`-style keyed symbols), the per-arm
  view construction (a zero-view arm instead of one post-conditional wrap), the release inside
  the guard block, the explicit-type-argument control byte-identical, and a re-emission
  determinism pin.
