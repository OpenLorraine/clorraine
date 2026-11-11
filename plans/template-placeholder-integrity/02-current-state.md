# Current State: Template Placeholder Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## 1. Baseline

All reproductions below were run on `8b4bff2` (clean `main == origin/main`)
with `target/debug/clorraine build FILE.clo` and the built host binary. The
base compiler for the registered issue is `00eb6bc`; the behavior is
unchanged on `8b4bff2`.

| Case | Source shape | Observed on `8b4bff2` | Expected |
|------|--------------|-----------------------|----------|
| Nullable `String?` | `let s: String? = null; println($"{s}");` | exit 139 (SIGSEGV) | E10010 |
| Nullable `str?` | `let v: str? = null; println($"{v}");` | prints an empty line, exit 0 | E10010 |
| `str` call | `println($"{mk()} calls={calls}");` | `x calls=2` | `x calls=1` |
| `take` sibling | `println($"{s}{consume(s)}");` | exit 139 (SIGSEGV) | `alpha5` |
| View over move | `let v: str = s; println($"{v}{consume(s)}");` | prints corrupt bytes; ASan heap-use-after-free | `alpha5` |
| Reverse view order | `println($"{consume(s)}{v}");` | E10022 (correct) | E10022 |
| Generic sibling | `show<String>(s, s)` with a sibling `take` | exit 139 | `alpha5` |
| Generic nullable | `show<String?>(null)` through `$"{x}"` | exit 139 | E10010 |
| Generic non-formattable | `show<H>(h)` with a struct `H` | internal "emitted C failed to compile" (`cl_text_H` missing) | E10500 |
| Generic opaque member | `show<T>(x) { $"x={x.name}" }` | internal "emitted C failed to compile" (`cl_text_unknown`) | E10500 at the declaration |
| View conditional | `println($"x={(c ? v : mk())}")` with `mk(): str` | `x=z`, `calls=2` | `calls=1` |
| View projection | `struct H { name: static str; }; println($"{get().name}")` | `alpha`, `calls=2` | `calls=1` |
| `?:` arm | `c ? $"{v}{consume(s)}" : stringOf("none")` | prints corrupt bytes | `alpha5` |
| `&&` right operand | `c && len($"v={mk()}") > 0` | `2` | `1` |
| Loop condition | `while (i < 2 && len($"v={mk()}") > 0)` | `4` for two tests | `2` |
| Module initializer | `let s: str = $"v={mk()}";` | `2` | `1` |

The nullable, double-evaluation, and module-initializer cases were reproduced
with small standalone programs. The take-sibling and view-over-move cases
come from issue #26 and were re-confirmed; the view case reports ASan
`heap-use-after-free` when built with `-fsanitize=address`.

The `str?` case does not crash: the null view has a null pointer and zero
length, so the current lowering prints an empty line. It is still an
unnarrowed nullable use, so the same E10010 rule applies. The fix therefore
tightens an accepted program into the diagnostic the spec already requires.

A separate adjacent case, `$"{null}"`, is outside this plan (AR-21): sema
accepts the null literal placeholder and the driver reports an internal
"emitted C failed to compile" error. A follow-up issue records it.

## 2. Lowering Pipeline

A template passes through five stages:

1. **Parse** — `compiler/src/parse/expr.rs:535` builds
   `ExprKind::Template(Vec<TemplatePiece>)`.
2. **Sema** — `compiler/src/sema/expr.rs:1766` (`check_template`) checks each
   placeholder's formattability (E10500) and the freestanding rejection
   (E11200), and types the template as `String` with placeholders or `str`
   without.
3. **Ownership walk** — `compiler/src/own/expr.rs:1123` evaluates each
   placeholder with `self.eval(inner)` and performs no nullable check.
4. **Lowering** — `compiler/src/own/lower.rs:754` maps each placeholder with
   `materialize_owned_argument(value, false)` and embeds it in
   `ValueKind::Template`.
5. **Emission** — `compiler/src/codegen/expr.rs:1620` (`template`) renders
   `cl_string_concat((ClStrPart[]){ ... }, n)`. A `str` placeholder emits
   `cl_str_part_owned(cl_text_str((value).ptr, (value).len))`; a `String`
   placeholder emits `cl_str_part_owned(cl_string_copy(value))`
   (`compiler/src/codegen/expr.rs:1646-1653`).

Module initializers take a separate path: `compiler/src/codegen/program.rs`
lowers the initializer AST (`lower_global` at `:3619`, `ExprKind::Template`
at `:3749`) and materializes owned results in
`materialize_init_value` (`:2927`, Template arm at `:3381`).

## 3. Root Cause per Defect

### 3.1 Nullable placeholder

The module-initializer walk applies the nullable rule:
`compiler/src/own/lower.rs:1365` calls
`self.check_value_into(inner, &Ty::str, inner.span)` for every placeholder.
`compiler/src/own/expr.rs:1123` (the function-body walk) does not. The
module case therefore reports E10010, while the function body lowers
`cl_string_copy(NULL)` and crashes.

`check_value_into` (`compiler/src/own/expr.rs:1427`) is the existing
nullability check; for a nullable value whose destination is non-null it calls
`check_nonnull_expr`, which consults flow narrowing. Reusing it gives the
function body the module behavior without a new diagnostic.

### 3.2 Double-evaluated `str` call

The emitter renders two projections of the same value:

```c
cl_str_part_owned(cl_text_str((cl_repro_Double_eval_mk()).ptr,
                             (cl_repro_Double_eval_mk()).len))
```

`mk()` runs once for `.ptr` and once for `.len`. A plain `str` place is
harmless to read twice; a call is not.

### 3.3 Take sibling / view over move

The own stage hoists a `take` argument's result statement ahead of the
template expression. The emitted body for `println($"{s}{consume(s)}")` is:

```c
__cl_tmp0 = cl_repro_Take_sibling_consume(s);
s = NULL;
...
__cl_owned0 = cl_string_concat((ClStrPart[]){
    cl_str_part_owned(cl_string_copy(s)),
    cl_str_part_i32((i32)(__cl_tmp0))
}, 2u);
```

`consume` owns and releases `s`, then the part copy reads the freed block.
The same shape appears in a `?:` arm, in a short-circuit right operand (as a
comma expression), and in a loop condition (per test). The own-stage flow
analysis already rejects the reverse order (`consume(s)` then `v`) with
E10022, so the language intends `$"{v}{consume(s)}"` to be legal; only the
emission order is wrong.

### 3.4 Module-initializer double evaluation

`let s: str = $"v={mk()}";` emits the same duplicated `(mk()).ptr` /
`(mk()).len` pair inside `cl_init_*` and prints `2`. The global path has no
shared `String` owner (module `let`s must be duplicable), so only the
once-only part of the fix is reachable there.

## 4. Existing Contracts Reused

| Contract | Location | Use in this plan |
|----------|----------|------------------|
| Insert a value statement at the current position | `compiler/src/own/lower.rs:3438` (`materialize_value`) | Eager placeholder source/copy temporaries |
| Deferred guarded fill | `compiler/src/own/lower.rs:2646` (`materialize_guard_operand`), `compiler/src/codegen/guards.rs` (`Materialize`) | `?:`, `&&`/`||`, loop-condition fills |
| Owning a temporary without a release | `materialize_value`'s unnamed temp; `Materialize { adopting: true }` | The part builder adopts the copy |
| Init-local binding and release | `compiler/src/codegen/program.rs:2293` (`materialize_init_local`), `:2927` (`materialize_init_value`) | Module-initializer once-only and retained copy |
| Type-aware text conversion | `compiler/src/codegen/expr.rs:1556` (`text_call`) | The internal copy call's concrete rendering |
| Nullable check | `compiler/src/own/expr.rs:1427` (`check_value_into`) | Function-body placeholder rejection |

## 5. Baseline Metrics and Tests

The #25 closeout recorded `strings` at 1,221,232 net instructions, 23,972
`.text`, and 2,003 allocations, with every scored output equal. The hot loop
is `benchmarks/strings/program.clo:13`, a 2000-iteration template with two
pure scalar placeholders; it must keep its direct scalar parts (AR-9).

Existing test suites that touch this surface:

- `compiler/tests/template_direct_formatting_spec_test.rs` — 15 behavior and
  allocation cases from #25.
- `compiler/tests/template_direct_formatting_impl_test.rs` — 5 emission pins.
- `compiler/tests/def_zero_null_spec_test.rs:1510` — the module-initializer
  nullable template case, currently the only nullable-template coverage.
- `compiler/tests/string_integer_formatting_{spec,impl}_test.rs` — the #28
  converter oracle and pins.

No suite covers a function-body nullable placeholder, a `str` call count, a
take sibling, a view over a move, a generic placeholder, a guarded arm, a loop
condition, or a module-initializer call count. Those are the gaps the new
suite fills (07-testing-strategy.md).

## 6. Constraints

- `spec/` is the immutable oracle; the behavior is already specified by
  Ch 04 §3 (left-to-right), Ch 02 TY-6 (nullable use), and Ch 16 §2 (last-use
  loan). No chapter or golden changes.
- The generated C must stay valid C99 on the host and the m68k toolchains; the
  Amiga shim has its own `cl_string_copy` and `cl_text_str`.
- The freestanding profile rejects placeholders at sema (E11200), so the new
  path must not add a freestanding dependency.
