# Defect Fixes: Compiler Robustness Closeout

> **Document**: 03-01-defect-fixes.md
> **Parent**: [Index](00-index.md)

## Shared Principles

Every fix makes the emitter agree with a rule the compiler already has:

- the constant folder is the semantic authority for constant contexts;
- generated C must be valid C99 under host GCC, `m68k-amigaos-gcc` (both CRT
  variants), vbcc, and the freestanding profile;
- a guard is emitted on the evaluated path and never moved across a condition;
- the generated C uses the compiler's own scalar typedefs (`u32`,
  `unsigned int`), never `uint32_t`, which the freestanding shim does not
  define;
- no fix changes the grammar, an ABI, a diagnostic code/message, or the frozen
  corpus. The one diagnostic trigger added by this plan is the user-ruled
  E10202 enum fit check (D6).

The tests that prove each fix live in
`compiler/tests/compiler_robustness_spec_test.rs` (behavior) and
`compiler/tests/compiler_robustness_impl_test.rs` (emission shape).

## D1 — Fill operand guards

### Root Cause

`Emitter::emit_array_assign_to` has a "pure fill" branch at
`compiler/src/codegen/function.rs:4222`. Purity means the fill expression has
no side effect to repeat, so the branch emits the value once per element. The
branch returns before the guard step that the impure branch runs at `:4238`.
Every operand guard is therefore dropped: an index bounds check, a division
zero check, and a float-to-integer range check.

The struct-literal field and conditional-arm paths reach the same branch, so
they share the defect. The module-initializer path is different: it emits a
guard per element after its preparation already emitted one, so it repeats the
first-element guard and also repeats guards for prefix elements.

### Fix

1. In the pure branch, emit the operand guards before the first store:

   ```rust
   if super::program::init_operand_is_pure(fill) {
       if !self.init_fill_guards_emitted {
           self.emit_guards(fill);
       }
       // existing per-element stores
   }
   ```

   `init_fill_guards_emitted` is a new flag, set by
   `emit_global_array_fill` (and the module struct-literal walk) when the
   preparation has already emitted the fill operand's guards. It is separate
   from `init_walked_guards`, which is false on the global-array path. The
   flag also gates the impure branch so it does not emit a second guard. Every
   set is paired with a save/restore around the walk, so a module fill cannot
   leave the flag true for a later statement fill.

2. In `emit_filled_array` (`compiler/src/codegen/program.rs:1821`), skip the
   per-element guard for every element whose operand the preparation already
   guarded, including prefix elements. The observable result: one fill guard
   plus one guard per prefix element, with no duplicate per-element guard.

3. In the module-level conditional struct-literal walk, set the flag false for
   the arm stores. The walk leaves arm guards to the store core; the store core
   must run the selected arm's guard. A module-level conditional struct case is
   added to prove it.

4. Do not change the call-fill branch or the conditional-condition site; they
   already emit exactly one guard.

### Invariants

- A non-conditional fill expression's operand is guarded once before the first
  store. A guard that faults stops the fill before any element is written.
- A conditional pure fill keeps its existing shape: the condition's guard is
  emitted at the condition site and each selected arm is guarded by the value
  renderer. The statement pure-fill path repeats the value expression per
  element, including its condition; hoisting a volatile condition is a
  separate, pre-existing evaluation-order limitation recorded in
  `docs/known-limitations.md`, not part of this guard fix.
- The module initializer runs the guard in declaration order, after the
  preceding initializers.
- The runtime shim's own trap helpers are not counted when a test counts
  guards.

### Tests

| Test | Kind | Red evidence |
|---|---|---|
| `should_guard_a_runtime_index_in_a_pure_fill` | spec, ASan | ASan `stack-buffer-overflow` before the fix |
| `should_trap_on_a_runtime_zero_divisor_in_a_pure_fill` | spec | `SIGFPE` before the fix |
| `should_trap_on_an_out_of_range_float_fill` | spec | `44` before the fix |
| `should_guard_a_struct_literal_array_field` | spec, ASan | `s: 0` before the fix |
| `should_guard_a_conditional_struct_array_field` | spec, ASan | unguarded arm before the fix |
| `should_guard_a_module_conditional_struct_array_field` | spec, ASan | unguarded arm before the fix |
| `should_keep_the_scalar_conditional_fill_guard` | spec, ASan | control: the arm is already guarded inline; the condition guard is emitted once |
| `should_emit_one_fill_guard_for_a_module_array` | impl | five guards for four elements before the fix |
| `should_emit_one_guard_per_prefix_element_and_fill` | impl | duplicated prefix guard before the fix |
| `should_restore_the_fill_guard_flag_after_a_module_fill` | impl | a module fill followed by a statement fill must keep the statement guard |
| `should_emit_one_guard_per_pure_statement_fill` | impl | zero guards before the fix |

The impl count is scoped to the generated function or initializer body: the
test extracts the body text between the function/init header and its closing
brace, then counts `cl_trap("E1121`. A guard-free fill is the zero control.

### Rejected Alternatives

| Alternative | Why not |
|---|---|
| Emit the fill into a temporary and copy it | Changes the emitted shape and the ownership path |
| Guard only the bounds case | The same branch drops `E11210` and `E11212` |
| Hoist a conditional arm's guard above the condition | The arm is evaluated only on its path; hoisting changes semantics |
| Guard inside `emit_compact_scalar_fill` | The compact form only sees a rendered value; the guard must precede the first store for every form |

## D2 and D4 — Narrow result wrapping

### Root Cause

`Emitter::binary` (`compiler/src/codegen/expr.rs:968`) renders `+`, `-`, and
`*` through a signed-only wrap path: a signed result is computed in the
same-width unsigned type and cast back. An unsigned result returns the promoted
C expression, so `u8(200) + u8(100)` is an `int` 300. The shift path
(`:1008`) masks the count but never casts the result to the declared width, so
`u8(128) << 1` is 256 and `u16(65535) << 15` is 2,147,450,880.

The same renderer feeds module `const` initializers, so
`const U8W: u8 = u8(200) + u8(100)` assigns 300 to a `u8` storage and GCC
reports `-Werror=overflow`. `u16(65535) * u16(65535)` is a signed `int`
overflow in the promoted C expression; it fails the strict build.

### Fix

Replace the signed-only arithmetic block with a width-aware block for results
narrower than 32 bits:

```text
if op in {Add, Sub, Mul} and result type is an integer scalar (not bool):
    if width < 32:
        (result)((unsigned int)left OP (unsigned int)right)
    else:
        keep the existing spelling
```

`unsigned int` is 32 bits on both the host and m68k. Every product of two
16-bit values fits in 32 bits (`65535 * 65535 = 4,294,836,225 < 2^32`), so the
intermediate is defined; the outer cast performs the declared wrap. A 32-bit
result keeps the existing spelling: unsigned `u32` arithmetic is defined and
wraps in `unsigned int`, and the signed path already casts through `u32`.

For `<<`, keep the count mask and, when the left operand's type is narrower
than 32 bits, wrap the result in that type:

```text
unsigned: (u8)((u8)left << (count & 7))
signed:   (i8)((u8)left << (count & 7))     // existing outer cast
```

A 32-bit `<<` keeps the existing spelling; the signed path already casts
through `u32` and the unsigned path is defined. `>>` needs no result cast: a
right shift never leaves the operand's range.
`/`, `%`, and the bitwise operators keep their spelling; the agreement matrix
pins them. `~` and unary `-` already wrap.

The generated C uses `unsigned int` and the emitted `u32` typedef. It must not
spell `uint32_t`: the freestanding profile defines only its own scalar
typedefs and includes no `<stdint.h>`.

### Invariants

- Every expression's run-time value equals the folder's value at the same
  result type.
- No generated expression relies on signed `int` overflow.
- A module `const` initializer carries the same typed value as an inline
  expression.
- The const stays a runtime place; only the initializer expression changes.
- The C is valid under the freestanding profile (no `uint32_t`).

### Tests

| Test | Kind | Red evidence |
|---|---|---|
| `should_wrap_narrow_arithmetic_in_inline_and_const_positions` | spec | `300 400 -1 256 65537` before the fix |
| `should_wrap_a_16_bit_multiply_and_shift` | spec + UBSan | `u16*u16` strict build fails; `u16<<15` prints 2147450880 |
| `should_build_a_narrow_const_under_the_strict_flags` | spec | `-Werror=overflow` before the fix |
| `should_build_a_narrow_local_a_fill_and_a_field` | spec | `-Werror=overflow` before the fix |
| `should_keep_the_32_bit_controls` | spec | `i32(MAX)+1` already wraps; control |
| `should_agree_with_the_folder_for_the_other_operators` | spec | `>> / % & | ^ ~` constant-context agreement |
| `should_emit_a_result_cast_for_unsigned_arithmetic` | impl | no cast before the fix |
| `should_emit_a_result_cast_for_an_unsigned_shift` | impl | no cast before the fix |
| `should_build_under_the_freestanding_profile` | spec | `uint32_t` would fail the freestanding shape build |

The existing `fill-shift` impl pin changes from
`a[31] = ((u8)(1) << (8 & 7));` to the wrapped
`a[31] = (u8)((u8)(1) << (8 & 7));` spelling. It is updated in the same task.

### Rejected Alternatives

| Alternative | Why not |
|---|---|
| Materialize every narrow constant as a literal | A storage change outside the defect; the place must keep its behavior |
| Reject narrow constant arithmetic | The language defines it (wrapping); rejection would be a language change |
| Cast only the outermost expression | The defect is per-operation; `i32(a + b + c)` needs each result wrapped |
| Use signed `int` as the intermediate | `u16 * u16` is undefined at `int` width |
| Cast 32-bit results too | Broader than the recorded defect; the existing 32-bit spelling already wraps where it must, and changing every 32-bit expression would move the emission audit without a measured need |

## D3 — Inline constant shift divisor

No code change. The pre-PLAN-14 base traps; the starting head prints `0`. The
regression test pins the shape in a local, a module `const`, a fill, and a call
argument, plus a named-const divisor. The helper accepts `RDC_CLC`, so the same
test binary can run against `build/robustness/base-0ac87d3e-clc` and the output
recorded.

### Test

| Test | Kind | Evidence |
|---|---|---|
| `should_keep_an_inline_shift_divisor_out_of_the_zero_trap` | spec | base `0ac87d3e` traps; head prints `0` |
| `should_emit_the_masked_count_for_an_inline_shift_divisor` | impl | `(8 & 7)` in the emitted C |

## D5 — `f32` rounding and constant float-to-integer overflow

### Root Cause

`ConstFolder::fold_cast` (`compiler/src/consts/mod.rs:292`) copies the `f64`
value for an `F32` target without rounding and casts a float to an integer with
`v as i128`, which saturates and maps NaN to 0. `fold_float`
(`compiler/src/consts/arithmetic.rs:180`) returns an unrounded `f64` and
`fold_binary` drops the float type (`scalar: None`). `fold_module_const` only
derives an integer context, so a declared `f32` reference is never rounded.

### Fix

1. Add a `round_float(value: f64, scalar: Option<ScalarType>) -> f64` helper.
   `F32` rounds through `value as f32 as f64`; `F64` or `None` returns the
   value unchanged.
2. `fold_cast` to `F32` rounds; to `F64` widens. A float-to-integer cast
   checks `is_finite` and the target range (`signed`: min..max, `unsigned`:
   0..max) before truncating. A NaN or out-of-range value returns
   `ConstError::FloatOutOfRange(Span, ScalarType)`.
3. `report` maps that error to `E10202` with the registered message and help:
   `literal does not fit '{type}'` and `use a wider type or a hex/binary form`.
   The target scalar supplies `{type}`. This is the existing code and text with
   a new trigger.
4. `fold_binary` over floats chooses the result scalar from the operand
   scalars: any `F64` makes it `F64`; otherwise an `F32` operand makes it
   `F32`; none stays `None`. An `F32` result is rounded.
5. `fold_unary` negation keeps the operand's scalar.
6. `fold_module_const` reads the declared scalar type. For a float declaration
   it sets the folded value's scalar to that type and rounds the value; for an
   integer declaration it keeps the existing context behavior.

### Invariants

- A folded float value carries the width its expression has at run time.
- An `f32` cast and every `f32` operation round at binary32.
- A constant float-to-integer conversion is either representable or a
  compile-time `E10202`; it is never a silently saturated value.
- Floats stay places; the emitted storage and initialization order do not
  change.
- The diagnostic first line and help are exactly the frozen E10202 text.

### Tests

| Test | Kind | Red evidence |
|---|---|---|
| `should_round_an_f32_cast_in_a_constant_context` | spec | `117 116` before the fix |
| `should_round_a_declared_f32_const_reference` | spec | `117 116` before the fix |
| `should_round_each_f32_operation` | unit | cancellation case: `0` at f32, `1` at f64 |
| `should_reject_a_constant_nan_float_conversion` | spec | `#define E_A 0` before the fix |
| `should_reject_a_constant_out_of_range_float_conversion` | spec | wrong extent before the fix |
| `should_use_the_registered_e10202_message` | spec | asserts `literal does not fit 'i8'` and the help |
| `should_keep_a_finite_f32_conversion` | spec | control |
| `should_keep_a_negative_zero_and_an_infinity_control` | spec | `i32(-0.0)` is 0; an infinity conversion is E10202 |
| `should_keep_a_float_const_as_a_place` | spec (existing) | unchanged |

### Rejected Alternatives

| Alternative | Why not |
|---|---|
| Materialize float constants | A storage/perf change; R4.6 keeps floats as places |
| Round only at the final cast | A chain of `f32` operations would keep `f64` intermediate precision |
| Keep `v as i128` for NaN | Rust maps NaN to 0; the frozen rule requires `E10202` |
| Report `E11212` at compile time | The frozen rule names `E10202` for a constant conversion |
| A new E10202 message | The registry pins the first line; use the registered text |

## D6 — Enum member values and conversion

### Root Cause

`fold_enum_member` (`compiler/src/consts/mod.rs:456`) wraps each
auto-increment at the backing type and folds explicit expressions.
`emit_enum` (`compiler/src/codegen/program.rs:911`) only recognizes an integer
literal and otherwise uses the running counter, so an explicit expression
emits `0` and an auto-increment past the backing range emits `256`. The two
paths disagree. `sema::check_enum` only applies the fit check to explicit
values, so the auto-increment case is accepted.

Separately, `lower_value` emits every `Call` as `ValueKind::Call`, so the sema
stage's type-cast resolution for `Direction(2)` is lost and the emitter spells
a C function call. A handle type import is a separate pre-existing limit: the
current sema resolver treats the import as a value, so `Buffer(0)` is not typed
as a cast.

### Fix

1. **User-ruled overflow rule.** In `sema::check_enum`, walk members with a
   running value. For an explicit member, fold at the backing type as today.
   For an auto member, use `previous + 1`. Check every value with `int_fits`
   and report `E10202` at `member.name_span` with the registered message and
   help when it does not fit. The folder stops wrapping auto-increments: it
   uses `previous + 1` and returns `None` for an out-of-range member (the
   declaration already reported E10202).
2. **One shared value walk.** Add
   `ConstFolder::enum_member_values(enum_name) -> Option<Vec<(String, i128)>>`
   that walks the members once: it binds the folder to the enum's declaring
   module, computes explicit values there, continues each auto member from the
   previous value plus one, caches the computed prefix so a later member can
   read an earlier one (`C = A | B`), and uses the target ABI `sizeof`
   policy so an extent and the emitted macro agree with the runtime C
   `sizeof`. It returns a value even when it
   does not fit; the declaration check reports E10202. `fold_enum_member`
   reads the cached prefix, returns `None` for an out-of-range member, and only
   starts the walk when no cached value exists (a forward or self reference is
   therefore a cycle). `sema::check_enum` consumes the same walk, zips it with
   `decl.members`, and applies `int_fits` at each member span. `emit_enum`
   consumes the same walk and emits the pairs. There is exactly one value
   sequence.
3. `EnumInfo` gains the declaring module name. `emit_enum_typedefs` passes it
   to `emit_enum`.
4. `emit_enum` calls `enum_member_values(decl.name)`; it does not re-fold a
   member or keep a second counter. If the folder declines the walk (already
   diagnosed), emission falls back to the declaration's literal values so it
   cannot crash. An out-of-range member never reaches emission because sema
   rejected it.
5. **Conversions.** The checker records every call node it resolves through
   `callee_type` in `SemaOutput::type_casts`. In `lower_value`'s
   `ExprKind::Call` branch and in `lower_global`'s Call branch, a call in that
   set lowers to `ValueKind::Cast`; the first argument takes the same
   float-guard materialization a scalar cast uses. Every other call keeps the
   call path. This is precise for a local, module function, global, or import
   that shadows the type name, because the checker already applied its
   `name_is_value` rule before recording the site. An imported enum or handle
   type name is not recorded as a cast and remains the pre-existing limitation.

### Invariants

- For an accepted enum, the folder and emitter produce the same value for
  every member, including a member that reads an earlier member and a member
  declared in another module.
- An auto-incremented member that does not fit the backing type is E10202; an
  accepted member is representable and storable.
- An enum conversion emits the same C cast a scalar conversion does. A local,
  module function, global, or import with the same written name keeps the call
  path.
- A constant context that reads a member agrees with the emitted macro,
  including the `sizeof` policy.
- A constant-context enum conversion stays E10224; the frozen expression list
  is not extended.

### Tests

| Test | Kind | Red evidence |
|---|---|---|
| `should_fold_an_explicit_enum_member_expression` | spec + impl | `#define E_A 0` before the fix |
| `should_reject_an_enum_auto_increment_that_does_not_fit` | compile-fail spec | accepted with `E_B 256` / folder 0 before the fix |
| `should_keep_an_in_range_enum_auto_increment` | spec | control |
| `should_store_an_enum_member_in_its_enum` | spec | control after the rejection rule |
| `should_emit_an_enum_conversion_as_a_cast` | spec + impl | `Direction(2)` C build error before the fix |
| `should_convert_a_runtime_value_to_an_enum` | spec + impl | a runtime argument takes the same cast path |
| `should_keep_a_shadowing_binding_a_call` | spec | a func-typed parameter named `E` keeps the call |
| `should_keep_a_module_function_named_like_a_type` | spec | a module function named `String` keeps the call |
| `should_keep_an_imported_function_call_before_an_enum_type` | spec | an imported function named `E` keeps the call (the pre-fix lowering stole it) |
| `should_accept_a_backward_enum_member_reference` | spec | `C = A | B` folds; the first shared walk broke it |
| `should_fold_a_cross_module_enum_member_with_a_declaring_const` | spec | the consumer module folded the declaring module's const |
| `should_keep_the_abi_sizeof_for_an_enum_member` | spec | the macro and the runtime `sizeof` disagreed |
| `should_reject_a_const_mediated_sizeof_enum_overflow` | spec | a cached value-context `sizeof` hid the ABI overflow |
| `should_check_an_explicit_member_before_a_later_non_integer_member` | spec | a later non-integer member hid an earlier overflow |
| `should_agree_on_a_cross_module_enum_member` | spec | cross-module literal coverage |

### Rejected Alternatives

| Alternative | Why not |
|---|---|
| Wrap at the backing type | The user ruled E10202; wrapping silently changes 255→0 |
| Keep `256` in both evaluators | An unrepresentable enum value; the spec store shape fails the strict build |
| Leave the emitter on literals only | Explicit constant expressions are valid per EN-3 and currently emit the wrong value |
| Emit a C function call for `Direction(2)` | The sema rule already treats it as a cast; the spec example must compile |
| A bare-name enum check in the lowering | A func-typed local or parameter named `E` would be miscompiled |

## Ordering

1. D1 (guards) first: the only memory-safety fix; independent.
2. D2/D4 (wrapping) next: removes the strict-build failure and is the base
   for the UBSan leg.
3. D5 (float width and E10202) before D6: the enum walk uses the folder.
4. D6 (enum values, overflow rejection, conversions) next.
5. UBSan and the fuzzer after the compiler is green.
6. Shape fixture, fault parity, documentation, emission audit, full gate,
   independent reviews, re-gate, and push.

Each committed task leaves the tree buildable and the focused tests green.
Red-first test tasks are committed together with the fix they prove.
