# Current State: Generated-C Robustness Bundle

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## 1. Baseline (`0fdb96c`)

| Case | Source | Observed | Expected |
|------|--------|----------|----------|
| init-local collision (#16) | `ffi probe { func __cl_itmp0(): i32; }` + call-initialized `i32[32]` + `__cl_itmp0()` | emitted `__cl_itmp0[32]` local shadows the ffi; cc rejects `called object '__cl_itmp0' is not a function` | generated local advanced past the taken symbol |
| text temporary leak (#30) | `println(text(stringOf("x")));` (with the `text` import) | ASan: 10 bytes leaked per call | leak-free |
| text template leak (#30) | `println(text($"{s}"));` | ASan: 12 bytes leaked | leak-free |
| body module-path precedence (#31) | module-level `let std: Std` + `println(std.fmt.text(7));` | emitted `std.fmt.text(7)`; cc rejects `'std' undeclared` | `cl_text_i32((i32)(7))` |
| initializer module-path precedence (#31) | module-level `let std: Std` (member `userText` returns `"u"`) + `let n = len(std.fmt.text(1234));` | initializer resolves the member field `std.fmt.text` and prints `1`; the module path should give `4` | `4` |
| initializer text argument | `let n = len(text(stringOf("ab")));` | already binds and frees the argument and the copy (ASan clean) | unchanged |

Confirmed controls:
- A local `std` value with a `fmt.text` field runs the member call (`u`).
- A non-module-path global field (`box.text(7)`) already emits
  `(cl_..._box.text)(7)` and runs.
- `text(s)` with a named place keeps `s` readable.

## 2. Root Causes

### 2.1 Init-local collision (#16)

`compiler/src/codegen/program.rs:3673` (`next_init_local`) formats
`__cl_itmp<n>` and advances the counter without checking user symbols. The
`__cl_ctmpl` template name already scans `is_taken_symbol`
(`compiler/src/codegen.rs:607`, used at `program.rs:1916`), so the init-local
path is the outlier.

### 2.2 Text temporary leak (#30)

`lower_call` (`compiler/src/own/call.rs:336`) derives `modes` from the
callee's declared signature. The `$text` sentinel has no signature, so `modes`
is `None` and the argument loop never calls `materialize_owned_argument`
(`compiler/src/own/lower.rs:6794`). A fresh `stringOf(...)`/template result is
therefore passed straight to `cl_string_copy` and never released. A named place
is already handled by the `projects_from_place` path. The module initializer
has its own walk and already binds/releases the argument, so #30 is body-only.

### 2.3 Body module-path precedence (#31)

`compiler/src/sema/call.rs` `is_std_fmt_text_path` gates the intrinsic on
`!self.name_is_value("std")`, and `compiler/src/own/expr.rs`
`is_builtin_text_call` has a matching `value_head_shadows("std")`. Both include
module-level and imported values. The rest of the compiler follows MOD-13: a
qualified module path is always available, and only a local shadows it
(MOD-16). With the intrinsic declined, the call falls to the qualified
member-call path, whose emission spells the literal head `std`; the global
member path is otherwise correct (`box.text` works).

### 2.4 Initializer module-path precedence (#31)

The module initializer has its own AST walk. `init_callable_value`
(`compiler/src/codegen/program.rs:4159`) resolves the whole
`std.fmt.text` path as a func-typed global field before
`init_text_intrinsic_name` runs, so the callable branch returns the member
field. The initializer must decide the intrinsic before the callable fallback.

## 3. Constraints

- `spec/` is the immutable oracle. MOD-13/MOD-16 decide the precedence.
- No new diagnostic, IR node, runtime helper, or ABI change.
- The #24/#28/#29 suites stay green, except the #29 IT-13 pin that #31
  supersedes.
- The module initializer has its own AST walk and must apply the same
  precedence.
