# Current State: Template Placeholder Residuals II

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## 1. Baseline

All reproductions were run on `bb3956c` (clean `main == origin/main`) with
`target/debug/clc` and, for runtime cases, host C builds under ASan/UBSan.

| Case | Source shape | Observed on `bb3956c` | Expected |
|------|--------------|-----------------------|----------|
| `text(null)` | `let t: String = text(null);` | accepted; `t = cl_text_unknown(NULL)`; C build fails | E10500, no C |
| `text(struct)` | `let t: String = text(Box { n: 1 });` | accepted; `cl_text_Box` is undeclared | E10500, no C |
| generic `text` | `func show<T>(x: T): String { return text(x); }` with `show<Box>` | accepted; `cl_text_Box` is undeclared | E10500 at the instance |
| `null + 1` | `println($"{null + 1}");` | accepted; invalid pointer arithmetic C | E10012 at the null |
| generic null arm (`str`) | `show<str>(true, "x")` with `$"{f ? null : x}"` | E10012 + E10010 at the null | accepted, prints `x` |
| generic null arm (`String`) | the same with `show<String>` | E10012 + E10010 at the null | E10010 only |
| scalar null arm | `let x = flag ? null : 5;` | accepted; `x = (flag ? NULL : 5)` | E10012 at the null |
| nullable `text` | `let s: String? = stringOf("x"); text(s);` | accepted | E10010 at `s` |
| opaque `text` projection | `func show<P>(p: P): String { return text(p.n); }` | accepted; `cl_text_unknown` | E10500 at `p.n` |
| for-step owning assignment | `for (...; s = $"{i}", ...)` | ASan: 49 bytes leaked in 3 allocations | leak-free |
| for-init template | `for ($"{tick()}", i = 0; ...)` | prints `0`; `tick()` never runs | prints `1` |
| same-order aggregate | `P { a: mark(1), b: mark(2) }` | prints `12` (supported toolchain) | `12` by contract |
| differing-order aggregate | `P { b: mark(2), a: mark(1) }` | prints `21` (fieldwise store) | `21` |
| panic null view | `let v: str = null; panic(v);` | UBSan: null pointer passed to `fwrite` | clean panic, no UBSan report |

## 2. Root Cause per Residual

### 2.1 Explicit `text` formattability

`compiler/src/sema/call.rs:276` handles the compiler-known `text` name by
checking the type arguments and walking every argument, then returning
`String`. It never applies the placeholder formattability rule from
`compiler/src/sema/expr.rs:1763`. `compiler/src/codegen/expr.rs:1561`
(`text_call`) has a generic fallback that spells `cl_text_<type_key>`; an
unknown or non-formattable type therefore emits a helper the shim does not
define. A generic argument is typed as a parameter at the declaration, so the
concrete check cannot judge it; no deferred duty exists.

### 2.2 Bare null binary operand

`compiler/src/sema/ops.rs:119-170` deliberately excludes a literal
`ExprKind::Null` from the null-operand rule (`is_null_shaped(&left_ty) &&
!matches!(left.kind, ExprKind::Null)`), so `null + 1` reaches
`binary_result` with an `Unknown` operand and is accepted. The emitted C
combines `NULL` with an integer.

### 2.3 Conditional null-arm settlement

`compiler/src/types/mod.rs:1055` (`conditional_result`) settles a null arm with
a handle sibling to `T?` (from PLAN-31) and leaves a view/slice sibling as the
empty state. Two gaps remain:

- The concrete declaration does not report a null arm at a scalar or struct
  sibling, because no expected type is written. `let x = flag ? null : 5;`
  emits `(flag ? NULL : 5)`.
- `compiler/src/own/instance_check.rs:2819` (`null_conditional_report`)
  judges a null arm against the raw sibling type instead of the settled result.
  At `T = str` it reports E10012 even though the concrete twin is an accepted
  empty view; at `T = String` it adds the same wrong E10012 beside the correct
  E10010. The placeholder duty at
  `compiler/src/own/instance_check.rs:2510` also reports E10010 for a null arm
  without checking whether the settled sibling is a bare-null view.

### 2.4 For-step owning assignment

`compiler/src/own/loops.rs:999` (`for_step_is_structured`) accepts any
`Statement::Assign` whose value needs no guards and is not an `ArrayLit`.
`compiler/src/codegen/control.rs:549` (`for_step_text`) renders the assignment
as a comma expression in the C `for` header. The ordinary store path
(`compiler/src/codegen/function.rs:1721`, `emit_assign`) releases the old
owned value through a scratch local; a header expression has no place for that
statement, so `s = $"..."` leaks the previous `s` on every iteration.

### 2.5 For-init effects

`compiler/src/own/loops.rs:280-287` handles `ForInit::Expr` with
`self.eval(expression)` for each expression. `eval` checks and emits an
assignment's own store, but it does not lower a template, call, or increment
into its run-time effect, so those effects do not run. The loop preheader has
no statement for them.

### 2.6 Aggregate initializer order

`compiler/src/codegen/function.rs:3003` (`struct_literal_needs_stores`) forces
the fieldwise store when the written order differs from the declaration order
(`compiler/src/codegen/guards.rs:1413`). When the orders agree, the literal is
a C initializer list. C99 6.7.8p23 leaves the evaluation order of initializer
expressions unspecified. GCC executes them in order, which the corpus and
benchmarks observe; vbcc is build-only in this repository (`bench-shape`). The
codegen comments assert declaration order but do not state the C99 caveat or
the supported-toolchain contract, and no test pins the same-order behavior.

### 2.7 Panic message

`compiler/runtime/shim_host.c:56` and
`compiler/runtime/shim_hosted_amiga.c:560` call
`fwrite(message, 1, len, stream)` unconditionally. A null view or zero-state
String is a NULL pointer with length zero; UBSan reports the null argument.
The freestanding shim copies byte by byte and is already safe.

## 3. Existing Contracts Reused

| Contract | Location | Use in this plan |
|----------|----------|------------------|
| Placeholder formattability | `compiler/src/sema/expr.rs:1763` | Explicit `text` argument |
| Deferred placeholder duty | `compiler/src/own/instance_check.rs:2510` | Generic `text` argument and generic null arms |
| Conditional settlement | `compiler/src/types/mod.rs:1055` | Shared null-arm helper |
| Null operand rule | `compiler/src/sema/ops.rs:119` | Bare null binary operand |
| Null-arm destination rule | `compiler/src/types/mod.rs:1094` | Written-expected destinations |
| For-step shape | `compiler/src/own/loops.rs:999` | Force flat form for an owning assignment |
| Statement release floor | `compiler/src/own/statements.rs:159` | For-init drain |
| Fieldwise store core | `compiler/src/codegen/function.rs:3056` | Differing-order aggregate |
| Panic shim | `compiler/runtime/shim_host.c:56`, Amiga `:560` | Zero-length guard |

## 4. Constraints

- `spec/` is the immutable oracle. No chapter or golden changes.
- The emitted C must stay valid C99 on host and m68k; host and Amiga shims
  stay in agreement.
- The freestanding profile rejects templates at sema; the panic guard there is
  already safe.
- The #26/#27 ordered-lowering and zero-state contracts stay green.
- #24 is a separate mini-plan; this plan does not change `text` arity.
