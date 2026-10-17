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
  substituted. `callee_name` (`own/call_check.rs:291`) reduces a qualified callee to its last
  segment.
- **Module loading and identity** (`compiler/src/load/mod.rs`, `compiler/src/own/call.rs`,
  `compiler/src/codegen.rs`): `resolve_imports` (:386) resolves each import and reports
  `E11103` only when the same local name was bound by an **earlier import** (:413-423); a
  same-module declaration is not consulted. `lower_call` (`own/call.rs:279`) lowers a qualified
  path through `global_head_segments` and carries the path value, but the function's emitted
  symbol is resolved by `resolve_function` (`codegen.rs:701`), which tries the caller's own
  module first and then the imports, i.e. by the last segment.
- **Conditional typing** (`compiler/src/sema/ops.rs:318` `check_conditional`): the arm types are
  checked with no expected type; `unify_numeric` (`types/mod.rs:839`) returns the other side when
  one side is unknown or a type parameter, so an opaque conditional gets the other arm's type and
  the per-instance disagreement is never judged. The null arm's `E10012` fires only when the
  conditional is checked into a known non-nullable slot.
- **Instance substitution** (`compiler/src/types/mod.rs`): `element_param` (:702) mints
  `#element of T`; `instance_bindings` (:712) binds the parameter and one placeholder per nested
  level, but only pushes the next level when the element is an `Array` (:731-734), so a `str`
  element's nested placeholder stays unbound. `index_element` (:745) maps an array row, a `str`
  byte, and a raw pointee. The per-instance duties (`own/instance_check.rs`) substitute the
  source and destination for conversions (`DEF-345`/`DEF-351` precedent).
- **Type resolution** (`compiler/src/types/table.rs`): `resolve_ty_with` (:602) resolves an AST
  type; `resolve_named` (:659) resolves only a **single-segment** name (`name.segments.len() ==
  1`), so a qualified type path becomes unknown and the declaration renders as the element type.
  `validate_type` (`sema/items.rs:131`) recurses into type arguments for extents and arity but
  does not check nullability; `report_illegal_nullable` (`own/program.rs:279`) checks declared
  **fields** only.
- **Literal typing** (`compiler/src/sema/ops.rs:18` `check_unary`): the operand is checked with no
  expected type, so an integer literal defaults to `i32` and a concrete destination reports
  `E10200`. `check_conditional`/`check_args` have the expected-type plumbing the unary path lacks.
- **Store lowering** (`compiler/src/own/expr.rs`): `emit_assigned` (:2274) emits a plain store;
  a target with a runtime index routes to `emit_indexed_store` (:2417), whose `indexed_target`
  (:2354) accepts only a top-level `ExprKind::Index` (:2363-2368). A member chain rooted at a
  runtime index returns `None`, and the store (and its RHS) is silently dropped. The
  constant-index twin lowers through the member path.
- **Place judgment** (`compiler/src/sema/ops.rs:402` `is_place`): the `Member` arm (:427) admits a
  base whose **shallow** kind is `Name`, `Member`, `Index`, `Deref`, or `Unsafe`; it does not
  recurse, so `(c ? a : b).f.g` (outer base is a Member) and `(c?a:b)[i].x` (outer base is an
  Index) pass the place test while their root is a conditional. The `Index` arm (:410) does
  recurse.
- **Ownership/loan transfer** (`compiler/src/own/statements.rs`, `own/place.rs`,
  `own/expr.rs`): the batch-L `DEF-315` fix transfers a whole view-array copy's element loans;
  the element-read store and the conditional/array-literal/single-view copies still record no
  loan. `move_element_loans` (`own/place.rs:1184`) is the transfer helper.
- **Deref reads and guarded move-outs** (`compiler/src/own/statements.rs`,
  `compiler/src/own/lower.rs`): the owning raw-deref read materializes its operand only when
  `owns_drop(ty)` (:490) is true of the **declaration-time** type, so a generic `T` substituted to
  `String` is not materialized and the pointer expression is emitted twice. The `&&`/`||` right
  operand defers move-outs (`lower.rs:228-236`), but the deferred channel carries places only, so
  a guarded `take` of a non-place deref source keeps the inline call.
- **Literal and guard lowering** (`compiler/src/own/lower.rs`): `literal_fields_need_stores`
  (:4536) and `literal_field_needs_stores` (:4563) route a view-array field to the element-wise
  store only when an element needs a binding; `view_element_needs_binding` (:4592) returns false
  for a call-valued view element, so a call that writes through the hidden fixed-array
  destination keeps the brace-list form. `materialize_guard_literal` (:1697) materializes a
  guarded literal at statement entry, so an owning conditional's arm fill precedes
  `if (g && consume(...))`.

### Relevant Files

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `compiler/src/sema/call.rs` | Dot-call resolution and written-argument checks | Judge the written argument against the receiver-resolved parameter (`DEF-360`) |
| `compiler/src/load/mod.rs` | Import binding and diagnostics | Compare an import's local name against the module's declarations (`DEF-363`) |
| `compiler/src/own/call.rs`, `compiler/src/codegen.rs` | Callee lowering and symbol resolution | Thread the module-qualified callee identity (`DEF-390`) |
| `compiler/src/sema/ops.rs` | Conditional, assignment, place, unary | Per-instance conditional hook; `is_place` recursion; unary context typing |
| `compiler/src/own/instance_check.rs` | Per-instance duties and audits | The conditional arm judgment; the element member view conversion |
| `compiler/src/types/mod.rs` | Element placeholders and substitution | The nested element level chain (`DEF-365`) |
| `compiler/src/types/table.rs` | Type resolution | Multi-segment named type resolution (`DEF-385`) |
| `compiler/src/sema/items.rs`, `compiler/src/own/program.rs` | Type validation and illegal-nullable reporting | The nullable type-argument check (`DEF-384`) |
| `compiler/src/own/expr.rs` | Store dispatch and indexed targets | The member-over-runtime-index store (`DEF-367`) |
| `compiler/src/own/statements.rs` | Deref materialization, copy transfers | Deref-once under the substituted type (`DEF-375`) |
| `compiler/src/own/place.rs` | Element loans | The element-read store and copy-source loans (`DEF-372`, `373`) |
| `compiler/src/own/lower.rs` | Literal and guard lowering | The hidden destination at every position (`DEF-378`); the guarded arm fill (`DEF-379`) |
| The origin suites (`checker_conformance_*`, `conditional_projection_residue_*`, `callable_release_integrity_*`) | Regression oracles | Additions only (AR-26) |

## Probe Evidence

Every row was rebuilt from a minimal reconstruction of its note and probed at `86d6f51` (AR-5).
The command shape is `make -C docker exec CMD="target/debug/clc --profile host -o <out>.c
<probe>.clo"`, then `cc -std=c99 -O0 -g -Wall -Wextra -Werror -o <out>.bin <out>.c` and
`timeout 10 <out>.bin` in the pinned container. The memory rows add
`cc -std=c99 -O0 -g -fsanitize=address -no-pie` and
`ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30`. The probe sources are under the gitignored
`build/batchM-plan/` tree. The full probe table (command output, exact diagnostic, run code) is
the AR-5 table; this document records the site map and the fix direction.

### P1 — Call identity and written-argument checks

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-360` | `probes/m360_dotarg.clo`, `probes/m360_borrowdot.clo` | accepted; strict build fails at the written `7` (`cl_pick__Box` 11:25, `cl_put__Box` 11:20); the direct inferred/explicit twins report one `E10200`; control runs 0 | ❌ Red | Run the written-argument conversion check after the receiver fixes the first parameter (the dot-call route) | `sema/call.rs:465` `check_dot_call`, `:671` `check_args_impl`, `:491` `record_deferred_conversion` |
| `DEF-361` | `probes/m361_local.clo`, `probes/m361_param.clo`, `matrix/d_*.clo` | local, parameter, lambda-expression, lambda-block, module-own, and generic-body direct-name shadows all compile strictly and run 0 | ✅ Control | Pin the direct-name shapes; no code change | `own/generics.rs:34-52` (module initializer scope), `:400-410` (lambda body scope) |
| `DEF-363` | `probes/m363_importfirst.clo`, `m363_declfirst.clo`, `m363_direct.clo` | all accepted; strict builds run 0; the collision is unreported | ❌ Red | Extend the import pass's duplicate check to the module's own declarations; one `E11103` at the import item | `load/mod.rs:386` `resolve_imports`, the prior-import check :413-423 |
| `DEF-390` | `probes/m390_main.clo` + `m390_lib.clo`; `probes/m390_sigmain.clo` + `m390_siglib.clo` | `lib.f()` runs the caller's `f` (run 1, expected 0); the signature shape accepted, strict build fails (`incompatible type for argument 1 of 'cl_probe_m390_sigmain_f'`); the mismatch control reports `E10200` | ❌ Red | Thread the resolved `(module, name)` through the lowered callee; module path wins, a lexical local shadows | `own/call.rs:279` `lower_call`, `:255` the qualified callee arm, `own/call_check.rs:291` `callee_name`, `codegen.rs:701` `resolve_function` |

### P2 — Substitution edges I: conditional and element keys

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-364` | `probes/m364_condopaque.clo`, `probes/m364_nullarm.clo` | (a) accepted, strict build fails (`type mismatch in conditional expression`); (b) accepted, strict build fails (`pointer/integer type mismatch`, `void *` to `i32`); controls run 0 | ❌ Red | Judge the substituted arms per instance: `E10200` at the disagreeing arm, `E10012` at the null arm | `sema/ops.rs:318` `check_conditional`, `types/mod.rs:839` `unify_numeric`, `own/instance_check.rs` |
| `DEF-365` | `probes/m365_elemkey.clo` | accepted; emitted key `cl_id__#element of #element of T`; strict build fails (`stray '#'`); host, hosted-amiga, freestanding | ❌ Red | Bind the nested element level from the substituted element, not only an array level | `types/mod.rs:712` `instance_bindings` (:731-734), `:745` `index_element` |
| `DEF-368` | `probes/m368_elemview.clo`, `probes/m368_concrete.clo` | accepted; strict build fails (`incompatible types ... 'ClStr' from 'ClString'` at 8:9); concrete twin runs 0; hosted-amiga same assignment | ❌ Red | Per-instance member view conversion at a `str` destination (the concrete twin's `cl_string_view`) | `own/instance_check.rs` (conversion duty), `codegen/expr.rs`/`function.rs` (the emission) |

### P3 — Substitution edges II: context typing

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-371` | `probes/m371_literal.clo`, `probes/m371_control.clo` | one `E10200` (`i32`→`usize`) at 15:25; concrete twin runs 0 | ❌ Red | Type the literal by the concrete resolved parameter at a generic callee | `sema/call.rs:629`/`:671` the argument check, `sema/expr.rs` literal typing |
| `DEF-384` | `probes/m384_nullabletypearg.clo`, `probes/m384_nullablehandle.clo` | accepted; strict build fails (`void *` to `i32` in `cl_push__i32Q` at 8:25); hosted-amiga same; `Array<String?>` keys and runs 0 | ❌ Red | Apply TY-4 to every resolved type node, including type arguments; keep the nullable-handle control | `sema/items.rs:131` `validate_type`, `own/program.rs:279` `report_illegal_nullable`, `sema/mod.rs:482` `resolve_type` |
| `DEF-385` | `probes/m385_qualifiedtype.clo` | accepted; strict build fails (`i32` from `ClArray__i32` at 4:9); hosted-amiga emits `i32 g`; unqualified control runs 0 | ❌ Red | Resolve a multi-segment named type through the declaration's own name (MOD-13) | `types/table.rs:659` `resolve_named` (the single-segment gate) |
| `DEF-386` | `probes/m386_negated.clo`, `probes/m386_control.clo` | one `E10200` (`i32`→`i16`) at 4:22, one (`i32`→`i8`) at 5:17; positive control runs 0 | ❌ Red | Propagate the expected context into the unary operand (LEX-14, Ch 04 §13/§5) | `sema/ops.rs:18` `check_unary` |

### P4 — Dropped store targets

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-367` | `probes/m367_indexed.clo` | accepted; strict build ok; run 191 — the local, borrow-parameter, and generic-borrow stores and their RHS are dropped (calls = 1, expected 4); constant index emits; ASan clean | ❌ Red (critical) | Lower the member-over-runtime-index target through the element address and apply the member projection; keep the bounds guard and the drop | `own/expr.rs:2274` `emit_assigned`, `:2354` `indexed_target` (the `Index`-only gate), `:2417` `emit_indexed_store` |
| `DEF-377` | `probes/m377_member.clo`, `m377_index.clo`, `m377_array.clo`, `m377_indexbase.clo`, `m377_deep.clo` | all accepted; runs 249/255/249/249/255 — the stores and RHS dropped; ASan/LSan-clean (no memory error; the store is absent); read control runs 0 | ❌ Red | Recurse the `Member` arm's base test so a conditional-rooted member/Index chain is not a place; one `E10221` | `sema/ops.rs:402` `is_place` (the `Member` arm :427) |

### P5 — Element and store loans

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-372` | `probes/m372_elementread.clo`; `m372_direct.clo`, `m372_member.clo` | accepted; run varies (14) + ASan heap-use-after-free; the direct and member twins report one `E10030` | ❌ Red | Carry the source element's loan through the view-like store/let | `own/statements.rs` (the store transfer), `own/place.rs:1184` `move_element_loans` |
| `DEF-373` | `probes/m373_conditional.clo`, `m373_arrayliteral.clo`, `m373_singleview.clo`; `m373_direct.clo` | all three accepted; runs vary (56/193/145) + ASan heap-use-after-free; the direct twin reports one `E10030` | ❌ Red | Record each arm's / element's loans in the conditional, array-literal, and single-view copy transfer | `own/place.rs:1184`, `own/statements.rs` (the copy transfer), `own/expr.rs` (the conditional transfer) |
| `DEF-374` | `probes/m374_condelem.clo`, `probes/m374_concrete.clo` | both reject two `E10026` at 12:24 | ✅ Control | Pin the count and span parity; no code change | `own/expr.rs`/`own/lower.rs` (the per-arm move-out, already conformant) |

### P6 — Deref and take channels

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-375` | `probes/m375_derefimpure.clo` | accepted; abort 134 + ASan double-free; the C calls `cl_alt__String` twice (`m375_derefimpure.c:702-703`) | ❌ Red | Materialize the deref operand for every raw-deref read that will be moved/emptied, under the substituted type | `own/statements.rs:490` (the `owns_drop(ty)` gate) |
| `DEF-376` | `probes/m376_guardedtake.clo` | accepted; abort 134 + ASan double-free | ❌ Red | Carry the consumed non-place source's emptying in the guarded operand | `own/lower.rs:228-236` (the deferred right operand), the deferred move-out channel |

### P7 — Literal destinations and guards

| Row | Probe | Observed | Verdict | Fix direction | Site |
|-----|-------|----------|---------|---------------|------|
| `DEF-378` | `probes/m378_readpos.clo`, `m378_gridel.clo`, `m378_guardrowcond.clo` | accepted; strict build fails (`too few arguments ... expected 1, have 0` for `n1`/`r1`) | ❌ Red | Materialize the hidden view-array destination at every literal position | `own/lower.rs:4536` `literal_fields_need_stores`, `:4563` `literal_field_needs_stores`, `:4592` `view_element_needs_binding` |
| `DEF-379` | `probes/m379_guardnest.clo` | accepted; strict build ok; run 1 (expected 0); the arm fill precedes `if (g && consume(...))` (calls = 3, expected 2) | ❌ Red | Keep the guarded literal's fill in the guard's evaluated path | `own/lower.rs:1697` `materialize_guard_literal`, the guarded statement emission |

## Dependencies

### Internal Dependencies

- The origin suites (`checker_conformance_{spec,impl}_test.rs`,
  `conditional_projection_residue_{spec,impl}_test.rs`,
  `callable_release_integrity_{spec,impl}_test.rs`) are the regression oracles; the new cases
  append to them (AR-26). `module_initializer_integrity_{spec,impl}_test.rs`, the frozen spec
  suites, and `run_fault_spec_test.rs` are oracles (the last gains the `DEF-367` bounds case).
- The batch-I/J/K/L suites and the pinned oracles gate the phases that touch reports, emissions,
  and the ownership surface.
- `make check` (the container) and `make verify`'s Amiga legs gate every phase's green state.

### External Dependencies

- None — the workspace stays std-only.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| The `DEF-367` indexed-member store needs a C declarator form the emitter lacks | Low | High | The constant-index twin already emits the member store; the fix reuses its shape. The AR-34 stop condition parks the row with evidence if a profile refuses |
| The `DEF-367` store changes ownership/drop ordering for an overwritten element field | Med | High | The memory lens probes the store with an owning field, a read-back, and a bounds fault; the overwritten owner drops per O3 before the store |
| The `DEF-377` recursion rejects an accepted ordinary store | Low | High | The controls (`a[i].y`, `s.f.g`, `s.arr[i]`, `*p.f`) are pinned per base kind; the read spelling stays accepted |
| The `DEF-360` written-argument check reports at the generic declaration | Med | Med | The direct inferred twin's accepted/rejected sites are the oracle; the report is at the call's argument |
| The `DEF-363` collision check rejects a legal import that shadows nothing | Low | High | The renamed-import and unrelated-declaration controls; the check matches the local binding name only |
| The `DEF-364` per-instance judgment reports at the declaration or twice | Med | Med | The concrete twins' count and site are the oracle; the phase records the spans |
| The `DEF-365` level-chain extension changes a legal nested array key | Low | Med | The existing nested-array cases in the origin suites re-run; the new bind only fills a previously unbound placeholder |
| The `DEF-384` type-argument check rejects a legal nullable handle/view | Med | High | `Array<String?>` is the verified control; the phase also probes the legal nullable `raw`/`cstring`/view spellings; the AR-34 narrowing applies |
| The `DEF-385` multi-segment resolution collides across modules | Low | Med | The row's scope is the same-instance spelling; a collision is recorded as a residual (AR-34) |
| The `DEF-386` context typing changes a legal literal's type | Med | Med | The positive literals and the binary literal rule stay pinned; the fit rule (`E10202`) is the rejection control |
| The `DEF-372`/`373` loan transfer changes an accepted copy | Med | High | The direct twins and the batch-L `DEF-315` loan matrix are the oracle; ASan/LSan gates every shape |
| The `DEF-375` materialization changes a legal impure deref read's count | Low | Med | The impure pointer is pinned once; the read-back value and ASan are the proof |
| The `DEF-376` guarded take empties a pointee on the wrong path | Low | High | Both guard selections run under ASan; the false selection must leave the owner intact |
| The `DEF-378` destination fill changes a guarded literal's laziness | Med | High | The guarded/un-guarded call counters and the batch-L phase-5 laziness pins re-run |
| The `DEF-379` guard-path fill changes the owned-arm evaluation order | Med | High | The calls counter and both selections; the ASan/LSan run |
