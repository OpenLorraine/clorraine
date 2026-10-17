# Current State: Residual Major Completion (Batch M)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The planning baseline is head `86d6f51` (the batch-L follow-up close-out). Batches I/J/K/L built
the machinery each row extends; the line numbers below are that tree and the executor
re-resolves them before editing.

- **Call typing** (`compiler/src/sema/call.rs`): `check_dot_call` (:465) resolves a dot call
  through `dot_call` (:570) and checks the receiver's conversion; the written arguments go
  through `check_args_impl` (:671) against the **declared** parameter types, so a parameter that
  still names a type parameter accepts any written argument. The plain call path
  (`check_args` :629) runs the same check after the explicit or inferred type arguments are
  substituted. The dot-call route must infer the callee's type arguments at the call site; the
  deferred record (`sema/expr.rs:491` `record_deferred_conversion`) stores `self.function_name`,
  so a valid generic caller `outer<i32>(b, c)` with `b.pick(c)` is falsely rejected (`E10200`,
  observed at 8:12).
- **Module loading and identity** (`compiler/src/load/mod.rs`, `compiler/src/own/call.rs`,
  `compiler/src/codegen.rs`): `resolve_imports` (:386) resolves each import and reports
  `E11103` only when the same local name was bound by an **earlier import** (:413-423); a
  same-module declaration is not consulted. `declarations_of` (:293) enumerates every declaration
  kind (`func`, `comptime`, `struct`, `enum`, `handle`, `ffi` block/function, `intrinsic`, module
  `let`, module `const`). `lower_call` (`own/call.rs:279`) computes the callee name at :308 via
  `callee_name` (`own/call_check.rs:291-298`), which reduces a qualified path to its last segment;
  the qualified **func-field** arm is `own/call.rs:255`. `resolve_function` (`codegen.rs:701`)
  resolves own-first by the last segment, and the mode/parameter/return lookups in
  `codegen/calls.rs`, `codegen/expr.rs`, and `codegen/function.rs` are separate from the symbol.
- **Conditional typing** (`compiler/src/sema/ops.rs:318` `check_conditional`): the arm types are
  checked with no expected type (`sema/expr.rs:67-71` drops `expected` before the call);
  `unify_numeric` (`types/mod.rs:839`) returns the other side when one side is unknown or a type
  parameter, so an opaque conditional gets the other arm's type and the per-instance disagreement
  is never judged. The direct `let y: i32 = c ? x : null;` is accepted and the C fails
  (`m364_direct`); the null arm's `E10012` fires only when the conditional is checked into a known
  non-nullable slot.
- **Instance substitution** (`compiler/src/types/mod.rs`): `element_param` (:702) mints
  `#element of T`; `instance_bindings` (:712) binds the parameter and one placeholder per nested
  level, but only pushes the next level when the element is an `Array` (:730-732), so a `str`
  element's nested placeholder stays unbound. `index_element` (:745) maps an array row, a `str`
  byte (Ch 08 §4, `:100-111`), and a raw pointee. The per-instance duties
  (`own/instance_check.rs`) substitute the source and destination for conversions
  (`DEF-345`/`DEF-351` precedent).
- **Type resolution** (`compiler/src/types/table.rs`): `resolve_ty_with` (:602) resolves an AST
  type; `resolve_named` (:659) resolves only a **single-segment** name (`name.segments.len() ==
  1`), so a qualified type path becomes unknown and the declaration renders as the element type.
  `validate_type` (`sema/items.rs:131`) recurses into type arguments for extents and arity but
  does not check nullability; `report_illegal_nullable` (`own/program.rs:279`) checks declared
  **fields** only, and `check_nullable_decl` (`own/place.rs:1931`) checks declared bindings. The
  new TY-4 check is scoped to nested type-argument positions, leaving those two checks unchanged.
- **Literal typing** (`compiler/src/sema/ops.rs:18` `check_unary`): the operand is checked with no
  expected type, so an integer literal defaults to `i32` and a concrete destination reports
  `E10200`. `check_conditional`/`check_args` have expected-type plumbing the unary path lacks; the
  negated literal's fit must be judged on its **folded value** (`-128` fits `i8`, `-129` does
  not), not on the positive operand.
- **Store lowering** (`compiler/src/own/expr.rs`): `emit_assigned` (:2274) emits a plain store;
  a target with a runtime index routes to `emit_indexed_store` (:2417), whose `indexed_target`
  (:2354) accepts only a top-level `ExprKind::Index` (:2364-2366). A member chain rooted at a
  runtime index returns `None`, and the store (and its RHS) is silently dropped. `element_ty` is
  taken from `self.ty(target)` at :2360, which is unknown at a generic declaration, so
  `StorePointer`'s owning/fixed-array branch never runs (the generic owning-element store leaks,
  LSan `m367_gen_elemstore`). `emit_indexed_update` (:2491) shares `indexed_target` for `++`/`--`,
  and the compound path shares it for `+=`; the target's `contains_update` pin is insufficient to
  order the base/index before an RHS call that writes the index. The constant-index twin lowers
  through the member path.
- **Place judgment** (`compiler/src/sema/ops.rs:402` `is_place`): the `Member` arm (:423) admits a
  base whose **shallow** kind is `Name`, `Member`, `Index`, `Deref`, or `Unsafe`; it does not
  recurse, so `(c ? a : b).f.g` (outer base is a Member) and `(c?a:b)[i].x` (outer base is an
  Index) pass the place test while their root is a conditional. The `Index` arm (:412) recurses
  but carries a shallow `|| matches!(base.kind, ExprKind::Unsafe(_))` bypass. The prefix spelling
  `unsafe p.f = ...` reports `E10221` at the head; the block form `unsafe { p.f.g = 7; }` is a
  green control.
- **Ownership/loan transfer** (`compiler/src/own/statements.rs`, `own/place.rs`,
  `own/expr.rs`): the batch-L `DEF-315` fix transfers a whole view-array copy's element loans;
  the element-read store and the conditional/array-literal/single-view copies still record no
  loan. `move_element_loans` (`own/place.rs:1184`) snapshots the source and calls
  `replace_element_loans` (`:1103`) with an empty set at `:1202`, replacing the destination
  record; a per-arm call would lose the first arm's positions, so the fix writes the union once.
- **Deref reads and guarded move-outs** (`compiler/src/own/statements.rs`,
  `compiler/src/own/lower.rs`): the owning raw-deref read materializes its operand only when
  `owns_drop(ty)` (`own/statements.rs:490`, in `prepare_store`) is true of the **declaration-time**
  type, so a generic `T` substituted to `String` is not materialized and the pointer expression is
  emitted twice. The return (`own/statements.rs` `check_return`, the `lower_value` at :1446) and
  the struct-literal field (`own/lower.rs:307`) paths have no materialization at all; both probes
  (`m375_ret`, `m375_field`) double-free. The take-argument path
  (`own/lower.rs:4866-4888`) already materializes under `owns_drop || maybe_owns || unknown`, but
  it runs at statement entry even under `defer_move_outs` (`m376_impure_false`: calls = 1 with
  `g = false`). The `&&`/`||` right operand defers move-outs (`lower.rs:228-236`), but the
  deferred channel carries places only, and the `Materialize` value (`ir.rs:298`, `move_outs`)
  is where a non-place pointer fill must travel.
- **Literal and guard lowering** (`compiler/src/own/lower.rs`, `compiler/src/codegen/guards.rs`):
  `literal_fields_need_stores` (:4536) and `literal_field_needs_stores` (:4563) route a view-array
  field to the element-wise store only when an element needs a binding; `view_element_needs_binding`
  (:4592) returns false for a call-valued view element, so a call that writes through the hidden
  fixed-array destination keeps the brace-list form. The **unguarded** literal builder does not
  consult the hidden-destination predicate at all, so the read position is red (`m378_readpos`).
  For the guarded owning conditional, the eager carrier is `materialize_owned_argument`'s by-value
  binding (`own/lower.rs:5109-5131`), which emits the `Assign` into `__cl_owned0` at statement
  entry; the fill position is `codegen/guards.rs:106-117` (statement form) and `:1016` (guarded
  expression form), and the drop/re-fill hoist is `guards.rs:273-307`. `materialize_guard_literal`
  (:1697) is the guarded-literal entry, not the by-value argument carrier.

### Relevant Files

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `compiler/src/sema/call.rs` | Dot-call resolution and written-argument checks | Infer the callee's type arguments at the call site; judge the written argument (`DEF-360`) |
| `compiler/src/sema/expr.rs` | Deferred conversion records, expected-type plumbing | The record's enclosing-function substitution (`DEF-360`); pass `expected` into the conditional (`DEF-364`) |
| `compiler/src/load/mod.rs` | Import binding and diagnostics | Compare an import's local name against every declaration kind (`DEF-363`) |
| `compiler/src/own/call.rs`, `own/call_check.rs`, `codegen.rs`, `codegen/calls.rs`, `codegen/expr.rs`, `codegen/function.rs` | Callee lowering, symbol, modes/params/return | Thread the module-qualified identity through every lookup (`DEF-390`) |
| `compiler/src/sema/ops.rs` | Conditional, assignment, place, unary | The expected-type conditional parameter; `is_place` `Unsafe` recursion and Index-bypass removal; unary context/folded fit |
| `compiler/src/own/instance_check.rs` | Per-instance duties and audits | The per-instance conditional duty; the element member view conversion |
| `compiler/src/types/mod.rs` | Element placeholders and substitution | The nested element level chain (`DEF-365`) |
| `compiler/src/types/table.rs` | Type resolution | Multi-segment named type resolution (`DEF-385`) |
| `compiler/src/sema/items.rs`, `compiler/src/own/program.rs`, `compiler/src/own/place.rs` | Type validation and illegal-nullable reporting | The nested type-argument check (`DEF-384`); keep the existing declaration checks |
| `compiler/src/own/expr.rs` | Store dispatch, indexed targets, updates | Per-instance element/member type; plain/compound/update/deep-chain target builder; target-order materialization; no silent drop (`DEF-367`) |
| `compiler/src/own/statements.rs` | `prepare_store`, `check_return`, the `let` transfer | The shared owning-deref materializer in store/return (`DEF-375`); the element-read store loan (`DEF-372`) |
| `compiler/src/own/place.rs` | Element loans | The loan union and mixed positions (`DEF-373`); the declaration check stays |
| `compiler/src/own/lower.rs`, `ir.rs`, `codegen/guards.rs` | Deref materialization, deferred move-outs, literal/guard lowering | The shared materializer and the deferred pointer fill (`DEF-375`/`376`); the unguarded hidden destination (`DEF-378`); the by-value carrier deferred into `Materialize` (`DEF-379`) |
| The origin suites (`checker_conformance_*`, `conditional_projection_residue_*`, `callable_release_integrity_*`) | Regression oracles | Additions only (AR-26); the `DEF-367` bounds case in the residue suite with `assert_traps` |

## Probe Evidence

Every row was rebuilt from a minimal reconstruction of its note and probed at `86d6f51` (AR-5),
and the preflight re-runs at the same head added the revised controls. The command shape is
`make -C docker exec CMD="target/debug/clc --profile host -o <out>.c <probe>.clo && cc -std=c99
-O0 -g -Wall -Wextra -Werror -o <out>.bin <out>.c && timeout 10 <out>.bin"` in the pinned
container. The memory rows add `-fsanitize=address -no-pie` and
`ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30`. The probe sources are under the gitignored
`build/batchM-plan/` tree (`probes/`, `preflight/`, `matrix/`, `extra/`). The full probe table
(the exact output, diagnostic, run code) is the AR-5 table; this document records the site map
and the fix direction.

### Preflight re-run evidence (the revised controls)

All commands below ran at `86d6f51` with the template above (the `preflight/` paths under the
gitignored `build/batchM-plan/` tree); the multi-file rows pass the module files on the command
line.

| Probe | Observed (host, pinned container) |
|-------|-----------------------------------|
| `m375_ret`, `m375_field` | clc 0, cc 0, run 134; ASan `attempting double-free` |
| `m375_dup` (concrete duplicable `T = i32`) | clc 0, cc 0, run 0; one `cl_alt__i32` call |
| `m367_compound`, `m367_update`, `m367_chain`, `m367_order` | clc/cc 0; runs 253 / 255 / 249 / 247 |
| `m367_gen_elemstore` (ASan/LSan) | clc/cc 0; run 0; LeakSanitizer reports a leak |
| `m367_bounds` (generic borrow, index 5) | clc/cc 0; run 0 (no `E11211`) |
| `m373_union` (ASan) | clc/cc 0; run varies (38–40 observed); ASan heap-use-after-free |
| `m384_annot`, `m384_callarg`, `m384_nested`, `m384_count` | clc/cc 0, run 0; the count case reports the three existing `E10011` sites only |
| `m386_matrix` | clc 1; eight `E10200` (`i32`→`i16`) |
| `m386_i8_neg128`, `m386_i8_neg129`, `m386_u8_neg5` | clc 1; one `E10200` each |
| `m360_genericcaller`, `m360_genericcaller2` | clc 1; one `E10200` at the receiver (8:12) — the false rejection |
| `m390_hook` (main + lib files) | clc/cc 0; run 0 (control) |
| `m376_impure_false`, `m376_impure_true` (ASan) | clc/cc 0; false run 10, true abort 134 + ASan double-free |
| `m377_unsafe_cond_member`, `m377_unsafe_cond_index` | clc/cc 0; run 0 (the stores are dropped) |
| `m377_controls`, `m377_unsafe_ptr` | clc/cc 0; run 0 (the green controls) |
| `m377_unsafe_prefix`, `m377_unsafe_prefix2`, `m377_unsafe_bare` | the prefix `unsafe p.f = ...` clc 1 `E10221`; the block forms clc/cc 0, run 0 |
| `m364_direct`, `m364_ctx` | clc 0; cc 1 (`pointer/integer type mismatch`, `void *` to `i32`) |
| `m364_concrete` | clc 1 `E10200` at 7:22 (the concrete control) |
| `m363_kinds` (main + three module files) | clc 0; three unused-import warnings (the collisions are unreported) |

### P1 — Call identity and written-argument checks

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-360` | `probes/m360_dotarg.clo`, `probes/m360_borrowdot.clo`, `preflight/m360_genericcaller.clo` / `m360_genericcaller2.clo` | accepted; strict build fails at the written `7` (`cl_pick__Box` 11:25, `cl_put__Box` 11:20); the direct inferred/explicit twins report one `E10200`; control runs 0; the valid generic caller is falsely rejected with `E10200` at the receiver (8:12) | ❌ Red | Infer the callee's type arguments at the dot-call site and judge the written argument there (no enclosing-function deferred record) | `sema/call.rs:465` `check_dot_call`, `:671` `check_args_impl`; `sema/expr.rs:491` `record_deferred_conversion` |
| `DEF-361` | six fixtures: `probes/m361_local.clo` + holder, `m361_param.clo` + holder, `matrix/d_param_generic.clo`, `matrix/d_lambda_expr.clo`, `matrix/d_lambda_block.clo`, `m361_control.clo` (no-import baseline) | all six compile strictly and run 0; the module-own shape is removed (it is the `DEF-363` collision); `d_comp` is `E90001` and `d_capture` is `E10405` (unrelated) | ✅ Control | Pin the six direct-name fixtures; no code change | `own/generics.rs:34-52` (body scope), `:400-410` (lambda body scope) |
| `DEF-363` | `probes/m363_importfirst.clo`, `m363_declfirst.clo`, `m363_direct.clo`; `preflight/m363_kinds_main.clo` + `m363_kinds_{fmod,smod,cmod}.clo` | all accepted; strict builds run 0; the collision is unreported; the func/struct/const kind matrix is accepted with unused-import warnings | ❌ Red | Extend the import pass's duplicate check to every declaration kind; one `E11103` at the import item | `load/mod.rs:386` `resolve_imports`, `:293` `declarations_of`, the prior-import check :413-423 |
| `DEF-390` | `probes/m390_main.clo` + `m390_lib.clo`; `m390_sigmain.clo` + `m390_siglib.clo`; `preflight/m390_hook_main.clo` + `m390_hook_lib.clo` | `lib.f()` runs the caller's `f` (run 1, expected 0); the signature shape accepted, strict build fails (`incompatible type for argument 1 of 'cl_probe_m390_sigmain_f'`); the mismatch control reports `E10200`; the qualified func-typed global `m.hook()` is green (run 0) | ❌ Red | Thread the resolved `(module, name)` through the symbol and the mode/parameter/return lookups; module path wins, a lexical local shadows | `own/call.rs:279` `lower_call`, `:308` the callee name, `:255` the func-field arm; `own/call_check.rs:291-298`; `codegen.rs:701`; `codegen/calls.rs`, `codegen/expr.rs`, `codegen/function.rs` |

### P2 — Substitution edges I: conditional and element keys

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-364` | `probes/m364_condopaque.clo`, `m364_nullarm.clo`; `preflight/m364_direct.clo`, `m364_ctx.clo` | (a) accepted, strict build fails (`type mismatch in conditional expression`); (b) accepted, strict build fails (`pointer/integer type mismatch`, `void *` to `i32`); the direct let/return/field/argument contexts are accepted and fail the strict build; the concrete disagreeing call already reports `E10200` at 7:22; controls run 0 | ❌ Red | Pass the expected type into `check_conditional` for direct contexts; judge the substituted arms per instance: `E10200` at the disagreeing arm, `E10012` at the null arm | `sema/ops.rs:318` `check_conditional`, `sema/expr.rs:67-71` (the dropped expected), `types/mod.rs:839` `unify_numeric`, `own/instance_check.rs` |
| `DEF-365` | `probes/m365_elemkey.clo` | accepted; emitted key `cl_id__#element of #element of T`; strict build fails (`stray '#'`); host, hosted-amiga, freestanding | ❌ Red | Bind the nested element level from the substituted element, not only an array level | `types/mod.rs:712` `instance_bindings` (:730-732), `:745` `index_element`; Ch 08 §4 `:100-111` |
| `DEF-368` | `probes/m368_elemview.clo`, `probes/m368_concrete.clo` | accepted; strict build fails (`incompatible types ... 'ClStr' from 'ClString'` at 8:9); concrete twin runs 0; hosted-amiga same assignment | ❌ Red | Per-instance member view conversion at a `str` destination (the concrete twin's `cl_string_view`) | `own/instance_check.rs` (conversion duty), `codegen/expr.rs`/`function.rs` (the emission) |

### P3 — Substitution edges II: context typing

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-371` | `probes/m371_literal.clo`, `m371_control.clo` | one `E10200` (`i32`→`usize`) at 15:25; concrete twin runs 0; the out-of-range input is `1000000000000` (not `-1`) | ❌ Red | Type the literal by the concrete resolved parameter at a plain generic call | `sema/call.rs:629`/`:671` the argument check, `sema/expr.rs` literal typing |
| `DEF-384` | `preflight/m384_annot.clo`, `m384_callarg.clo`, `m384_nested.clo`, `m384_count.clo`; `probes/m384_nullabletypearg.clo`, `m384_nullablehandle.clo` | the annotation, explicit call type argument, and both nested cases are accepted/run 0; the count case reports the three existing `E10011` sites but not `Array<i32?>`; the combined probe fails the strict build (`void *` to `i32` in `cl_push__i32Q` at 8:25); hosted-amiga same; `Array<String?>` keys `ClArray__StringQ` and runs 0 | ❌ Red | Apply TY-4 at nested type-argument positions only; keep `check_nullable_decl` and `report_illegal_nullable`; one report per site | `sema/items.rs:131` `validate_type`, `own/program.rs:279` `report_illegal_nullable`, `own/place.rs:1931` `check_nullable_decl`, `sema/mod.rs:482` `resolve_type` |
| `DEF-385` | `probes/m385_qualifiedtype.clo` | accepted; strict build fails (`i32` from `ClArray__i32` at 4:9); hosted-amiga emits `i32 g`; unqualified control runs 0 | ❌ Red | Resolve a multi-segment named type through the declaration's own name (MOD-13) | `types/table.rs:659` `resolve_named` (the single-segment gate) |
| `DEF-386` | `preflight/m386_matrix.clo`, `m386_i8_neg128.clo`, `m386_i8_neg129.clo`, `m386_u8_neg5.clo`; `probes/m386_control.clo` | the matrix (let, module value, argument, return, field, array element, conditional arm, assignment) reports eight `E10200`; `i8 = -128`, `i8 = -129`, and `u8 = -5` report `E10200`; the positive control runs 0 | ❌ Red | Propagate the expected context into the unary operand and judge the fit on the folded negated value (`E10202` for `-129`/`u8 -5`) | `sema/ops.rs:18` `check_unary`; the shared `int_fits`/`int_literal_value` |

### P4 — Dropped store targets

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-367` | `probes/m367_indexed.clo`; `preflight/m367_compound.clo`, `m367_update.clo`, `m367_chain.clo`, `m367_order.clo`, `m367_gen_elemstore.clo`, `m367_bounds.clo` | accepted; strict build ok; `m367_indexed` run 191 (calls = 1, expected 4); compound run 253; update run 255; chain run 249; target order run 247; the generic owning member spelling is dropped (run 255) and the generic owning-element store runs 0 with a LeakSanitizer leak; the out-of-range generic-borrow index runs 0 (no trap); the constant index emits | ❌ Red (critical) | Resolve the element/member type per instance; generalize the target builder to plain/compound/update/deep chain; always materialize the base/index before the RHS; keep the bounds guard; the `None` fallback never silently drops | `own/expr.rs:2274` `emit_assigned`, `:2354` `indexed_target` (:2364-2366), `:2417` `emit_indexed_store`, `:2491` `emit_indexed_update`; `codegen/expr.rs:290` `resolved_place_ty`; `codegen/function.rs:714` (`StorePointer` reads `pointer_ty = substitute(&pointer.ty)`) |
| `DEF-377` | `probes/m377_member.clo`, `m377_index.clo`, `m377_array.clo`, `m377_indexbase.clo`, `m377_deep.clo`; `preflight/m377_unsafe_cond_member.clo`, `m377_unsafe_cond_index.clo`, `m377_controls.clo`, `m377_unsafe_ptr.clo` | all five forms accepted, runs 255/249/249/249/255; the `unsafe`-wrapped conditional member/index stores accepted/run 0; the green controls `s.f.g`, `s.arr[i]`, `p.f.g` inside an `unsafe { }` block run 0; the prefix `unsafe p.f = ...` reports `E10221` | ❌ Red | Add `Unsafe(inner) => is_place(inner)` and remove the Index arm's Unsafe bypass; recurse the Member arm; one `E10221` per conditional-rooted store | `sema/ops.rs:402` `is_place` (Index :412, Member :423) |

### P5 — Element and store loans

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-372` | `probes/m372_elementread.clo`; `m372_direct.clo`, `m372_member.clo` | accepted; run varies (14) + ASan heap-use-after-free; the direct and member twins report one `E10030` | ❌ Red | Carry the source element's loan through the view-like store/let | `own/statements.rs:391-399` (the `let` transfer), `own/place.rs:1184` `move_element_loans` |
| `DEF-373` | `probes/m373_conditional.clo`, `m373_arrayliteral.clo`, `m373_singleview.clo`; `preflight/m373_union.clo`; `m373_direct.clo` | all four accepted; runs vary (56/193/145/38–40 observed) + ASan heap-use-after-free; the direct twin reports one `E10030`; the mixed-position union reads freed storage | ❌ Red | Snapshot both arms and write the destination's loan **union** once; include the array-literal fill form and a conditional arm that is an array literal | `own/place.rs:1184` `move_element_loans`, the replace at `:1202`, `replace_element_loans` `:1103`; `own/statements.rs` (the copy transfer) |
| `DEF-374` | `probes/m374_condelem.clo`, `probes/m374_concrete.clo` | both reject two `E10026` at 12:24 | ✅ Control | Pin the count and span parity; no code change | `own/expr.rs`/`own/lower.rs` (the per-arm move-out, already conformant) |

### P6 — Deref and take channels

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-375` | `probes/m375_derefimpure.clo`; `preflight/m375_ret.clo`, `m375_field.clo` | accepted; all three abort 134 + ASan double-free; the store C calls `cl_alt__String` twice (`m375_derefimpure.c:702-703`); the return and struct-field shapes double-free identically | ❌ Red | One shared owning-deref materializer (take-path predicate) used by the store, the return, and the aggregate/struct-literal-field paths | `own/statements.rs:440` `prepare_store` (:490 gate), `check_return` (:1358, `lower_value` :1446); `own/lower.rs:4866-4888` (the take predicate) and the struct-literal field path (:307) |
| `DEF-376` | `probes/m376_guardedtake.clo`; `preflight/m376_impure_false.clo`, `m376_impure_true.clo` | accepted; `guardedtake`/`impure_true` abort 134 + ASan double-free; `impure_false` runs 10 (the statement-entry materialization calls the impure pointer once with `g = false`; expected 0) | ❌ Red | Gate the take-deref materialization on `defer_move_outs == 0` and carry the pointer fill on the deferred record (`Materialize`/`move_outs`); empty on the evaluated path | `own/lower.rs:4866-4888` (the take path), `:228-236` (the deferred right operand); `ir.rs:298`; `codegen/guards.rs`, `codegen/function.rs`, `codegen/cleanup.rs`; `own/generics.rs`, `own/instance_check.rs`, `own/loops.rs` |

### P7 — Literal destinations and guards

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-378` | `probes/m378_readpos.clo`, `m378_gridel.clo`, `m378_guardrowcond.clo` | accepted; strict build fails (`too few arguments ... expected 1, have 0` for `n1`/`r1`); the read and unguarded positions are red because the builder does not consult the predicate | ❌ Red | Treat a call-valued view-array element whose return uses the hidden destination as needing the binding store; make the **unguarded** builder consult it | `own/lower.rs:4536` `literal_fields_need_stores`, `:4563` `literal_field_needs_stores`, `:4592` `view_element_needs_binding` |
| `DEF-379` | `probes/m379_guardnest.clo` | accepted; strict build ok; run 1 (expected 0); the by-value argument carrier is filled before `if (g && consume(...))` (calls = 3, expected 2) | ❌ Red | Defer the final by-value argument binding under `defer_move_outs` into the `Materialize` channel; keep the fill in the guarded evaluated path | `own/lower.rs:5109-5131` (`materialize_owned_argument`'s carrier); `codegen/guards.rs:106-117`, `:1016`, the drop/re-fill `:273-307` |

## Dependencies

### Internal Dependencies

- The origin suites (`checker_conformance_{spec,impl}_test.rs`,
  `conditional_projection_residue_{spec,impl}_test.rs`,
  `callable_release_integrity_{spec,impl}_test.rs`) are the regression oracles; the new cases
  append to them (AR-26). `module_initializer_integrity_{spec,impl}_test.rs`, the frozen spec
  suites, and `run_fault_spec_test.rs` are oracles; the `DEF-367` bounds case is in the residue
  suite with the inline `assert_traps` helper, so the frozen `run_fault` data (and the Amiga fault
  list) stays untouched.
- The batch-I/J/K/L suites and the pinned oracles gate the phases that touch reports, emissions,
  and the ownership surface.
- `make check` (the container) and `make verify`'s Amiga legs gate every phase's green state.
- Execution is dependency-ordered: P1 → P2 and P1 → P3; P2 → P5; P6 → P7; P4 is independent and
  runs fourth (AR-36).

### External Dependencies

- None — the workspace stays std-only.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| The `DEF-367` indexed-member store needs a C declarator form the emitter lacks | Low | High | The constant-index twin already emits the member store; the fix reuses its shape. If a profile refuses, the phase is a blocker and asks for a user ruling (AR-34(a)); no park fallback |
| The `DEF-367` store changes ownership/drop ordering for an overwritten element field | Med | High | The memory lens probes the store with an owning field, a read-back, the generic owning store (LSan), and a bounds fault; the overwritten owner drops per O3 before the store |
| The `DEF-377` recursion rejects an accepted ordinary store | Low | High | The controls (`s.f.g`, `s.arr[i]`, `p.f.g` inside an `unsafe { }` block) are pinned per base kind; the read spelling stays accepted; the prefix `unsafe p.f` is already `E10221` and is not claimed |
| The `DEF-360` written-argument check reports at the generic declaration or on a valid generic caller | Med | Med | The call-site inference control (`outer<i32>(b, c)`) and the direct twin's accepted/rejected sites are the oracle; the report is at the call's argument |
| The `DEF-363` collision check rejects a legal import that shadows nothing | Low | High | The renamed-import and unrelated-declaration controls; the func/struct/const kind matrix; the check matches the local binding name only |
| The `DEF-364` per-instance judgment reports at the declaration, twice, or misses a direct context | Med | Med | The four direct context pins and the concrete twins' count/site are the oracle; the phase records the spans |
| The `DEF-365` level-chain extension changes a legal nested array key | Low | Med | The existing nested-array cases in the origin suites re-run; the new bind only fills a previously unbound placeholder |
| The `DEF-384` type-argument check rejects a legal nullable handle/view or duplicates an existing report | Med | High | The annotation/call/nested/count probes; `Array<String?>` is the verified control; the phase narrows per AR-34(b) if needed |
| The `DEF-385` multi-segment resolution collides across modules | Low | Med | The row's scope is the same-instance spelling; a collision is recorded as a residual (AR-34) |
| The `DEF-386` folded fit changes a legal literal's type | Med | Med | The position matrix, `-128`/`-129`, and the `u8 -5` rejection are the oracle; no wrapping pin |
| The `DEF-372`/`373` loan transfer changes an accepted copy or loses an arm's positions | Med | High | The direct twins, the mixed-position union, and the batch-L `DEF-315` loan matrix are the oracle; ASan/LSan gates every shape |
| The `DEF-375` materializer changes a legal impure deref read's count or a duplicable instance's emission | Low | Med | The store/return/field probes, the discarded/duplicable controls, and the named generic duplicable delta; ASan is the proof |
| The `DEF-376` guarded take empties a pointee on the wrong path | Low | High | Both guard selections run under ASan; the false selection must not call the impure pointer and must leave the owner intact |
| The `DEF-378` destination fill changes a guarded literal's laziness | Med | High | The guarded/un-guarded call counters and the batch-L phase-5 laziness pins re-run; the unguarded builder now consults the predicate |
| The `DEF-379` deferred carrier changes the owned-arm evaluation order or the guard drop/re-fill | Med | High | The calls counter and both selections; the memory lens analyzes `guards.rs:273-307`; the impl pin asserts the fill position |
