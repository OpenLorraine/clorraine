# Current State: Evaluation and Emission Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

The batch's surfaces (with the fix levers mapped by three read-only recon passes at HEAD
`6548954`): the module-initializer emitter (`compiler/src/codegen/program.rs:619-684`) beside the
function-body materialization path (`own/lower.rs:999-1035`, `codegen/expr.rs:503-517`); the
emitter's type/declaration layer (`codegen/types.rs`, `codegen/cleanup.rs`, `codegen/function.rs`);
the checker (`sema/ops.rs`, `sema/stmt.rs`, `sema/call.rs`, `sema/expr.rs`); the ownership stage
(`own/lower.rs`, `own/call.rs`, `own/place.rs`, `own/statements.rs`, `own/loops.rs`); and the
per-instance duty audit (`own/instance_check.rs`).

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/codegen/program.rs` | init emission, slice typedefs, freestanding preamble, global lowering | P1 init materialization (R1–R3), R7 bool, R9 seeds, R10 typedef split |
| `compiler/src/codegen/expr.rs` | value rendering, hidden destinations, `len`/value typing | R1 destinations, R12 projection resolution |
| `compiler/src/codegen/calls.rs` | call arguments, intrinsics | R8 coercion, R11 fold |
| `compiler/src/codegen/function.rs` | function emission, name de-dup, unused analysis | R4/R5 markers, R9 seeds |
| `compiler/src/codegen/cleanup.rs` | returns, empty states, defer/supersede machinery | R6 empty helper, R20 preemption release |
| `compiler/src/sema/{ops,stmt,call,expr}.rs` | operator/statement/call/expression checks | P3 rejections, R22/R23 capture duties |
| `compiler/src/own/{lower,call,place,statements,loops,expr}.rs` | ownership lowering and checks | R4/R5 binding skip, R17/R18 writes, P5 releases |
| `spec/14-diagnostics.md` | registry | E10043 row, E10042 wording, E10212 trigger (AR-B6, AR-B11) |
| `spec/tests/compile-fail/` | goldens | `E10043_str_loop.clo`; existing E10042 golden covers the `cstring` wording |
| `compiler/tests/eval_emission_integrity_{spec,impl}_test.rs` | new suites | all phases (AR-B15) |

### Baseline Evidence

Each defect's repro and mechanism are recorded on its roadmap row (verified at the stated
baselines by the batch that filed it) and were re-mapped to `file:line` by the plan's three recon
passes. The rows needing a fresh reproduction at execution time: DEF-99 (no minimal shape is
recorded — reproduce first), DEF-97 (the recorded `codegen_run_spec_test__036` ordinal does not
map; reproduce from the suite), and DEF-76 (a code-proven mechanism without a live test pin).

## Gaps Identified

### Phase 1 — module-initializer integrity

| Gap | Current Behavior | Required Behavior |
| --- | ---------------- | ----------------- |
| DEF-117 | six accepted positions emit calls without the hidden fixed-array destination (`program.rs:677-683`; `fills_element_wise:1513` excludes them); strict build fails | every position binds through the destination (`call_with_destination`, `expr.rs:508`) and builds |
| DEF-121 | `emit_guards` and the store re-render the same expression (`program.rs:682-683`); an impure index runs twice (ASan global-buffer-overflow at base) | the scalar single-assignment path materializes impure sub-operands (`materialize_guarded_sub_operands`, `program.rs:854`) before the guards |
| DEF-122 | `check_array_literal` adopts a `None`-bearing expected extent (`sema/expr.rs:755-761`), so the literal silently becomes a slice and `x = { 1, 2 };` is emitted | the literal keeps its fixed type and `check_conversion` reports `E10200` |

### Phase 2 — strict-build emission shapes

| Gap | Current Behavior | Required Behavior |
| --- | ---------------- | ----------------- |
| DEF-76 / DEF-97 | a generic fixed-array `len` argument binds `__cl_owned0` that is never read (`own/lower.rs:1491`, `codegen/calls.rs:239`; `unread_locals` `function.rs:2441` counts the arg as a read) | the binding is skipped for a structural `len`, as the concrete non-owning path already drops it |
| DEF-80 | `empty_return_value` spells `(ClP){0}` (`cleanup.rs:922`) contrary to spec/17:19-23 | a `cl_zero_<ClType>` helper is returned (the view helpers are the precedent) |
| DEF-90 | the freestanding preamble defines no `bool` (`program.rs:88-99`); `scalar_c_type` spells it (`types.rs:267`) | the preamble provides the type (1-byte, matching `types/table.rs:497`) |
| DEF-98 | a view argument to an uncontracted extern passes the `ClSlice` struct where the prototype says `APTR` (`calls.rs:148-164,185`) | the call site passes the view's pointer; the prototype stays `APTR` |
| DEF-112 | generated spellings are not seeded into `used_names` (`codegen/function.rs:83,185`; `codegen.rs:525`) | a source binding colliding with `__env`/`__cl_ret`/`__cl_tmpN`/`__cl_ownedN`/`__cl_itmpN` de-duplicates |
| DEF-120 | slice typedefs precede the struct definitions (`codegen.rs:368-369`); a multi-extent row element needs the complete struct (`types.rs:238-254`) | pointer-element slices stay first; complete-row-element slices follow the definitions |
| DEF-128 | a callable-bound parameter's field call accepts (`sema/call.rs:444-465` skips the bounded case) but the emission spells an unqualified `call(h, 5)` (`own/call.rs:111-137` requires a concrete `Func` field; `codegen/expr.rs:575-588` falls through) | the field call resolves through the instance field/closure call path and builds |

### Phase 3 — value typing and checker conformance

| Gap | Current Behavior | Required Behavior |
| --- | ---------------- | ----------------- |
| DEF-99 / DEF-111 | a generic projection is Unknown (`own/expr.rs:448`), so `len` falls to `"0"` (`calls.rs:239`) and `takeBox<T>` returns the fallback | the projected field resolves through the substituted base |
| DEF-108 | `let x = null;` carries the literal's nullable-unknown type but emits `i32 x = NULL;` (`types.rs:94`, `expr.rs:325`) | the declaration and null spelling agree; the pairing stays |
| DEF-107 | `comparable` whitelists `Str`/`StaticStr` (`ops.rs:455-467`), so a pair emits a C struct comparison | views are outside the frozen table (spec/04:104); `E10222` reports; the null-pair exemption stays |
| DEF-118 | `iterated_element` returns Unknown for `str` (`types/mod.rs:375`), the element binds as `raw Unknown` → `i32 *` | the loop rejects with the approved `E10043` |
| DEF-129 | a bare-`Param` receiver falls through `check_dot_call` (`sema/call.rs:444-465`) and emits `foo(x, 5)` | the member callee reports E10212 |

### Phase 4 — write-mutability rules

| Gap | Current Behavior | Required Behavior |
| --- | ---------------- | ----------------- |
| DEF-119 | `check_update_value_write` (`own/expr.rs:898-915`) runs only the immutable-view predicate, so a value update bypasses E10023/E10024 (`xs[0]++` on `const`/read-only `u8[]`) | routed through `read_place`+`write_place` (locals) and the const/immutable checks (globals), one report per site |
| DEF-124 | `report_immutable_view_write` matches only `Str`/`StaticStr` (`own/place.rs:261-270`), so `cstring` element writes pass | `CString` joins the predicate; the registry row gains the wording |

### Phase 5 — ownership lifetimes and releases

| Gap | Current Behavior | Required Behavior |
| --- | ---------------- | ----------------- |
| DEF-95 | the adopting carrier is non-releasing by design (`own/lower.rs:921-950`) and the temp-owner walk has no `StructLit` arm (`:1130`), so the backing leaks per iteration | the view binding takes a scope-scoped owner (release at its scope end) |
| DEF-96 | the supersede gate arms only inside a deferred copy (`codegen.rs:143-151,691`), so a preempted ordinary result is never released | the gate arms for an ordinary store whose deferred scopes can preempt (`defer_region_has_out_edge`, `cleanup.rs:931`); byte-identity where unreachable |
| DEF-125 | a func-value call skips the modes block (`own/call.rs:211`), so an owned temporary argument is passed inline and leaks | the call takes the binding's own func-type modes and materializes the argument |
| DEF-126 | a block-bodied capture return routes to the not-lend duty (`own/place.rs:460`), reporting `E10034` | the duty is suppressed for captures (`base = Some(EnvField)`) and the ruled `E10401` records at the block return/take |
| DEF-127 | `place_root_name` has no `Conditional` arm (`sema/mod.rs:697`), so both capture checks miss it and the C double-frees | the conditional arms recurse in the rejection and the recorder, one report per site |

### Phase close — DEF-103

The `Q` marker (`types/keys.rs:70-74`, recursion `:47-54`) collides with a user `StringQ` in
nested instances; the top-level function-instance analogue is pre-existing. The collision fails
closed under the strict build (distinct `Ty::display` dedupe, `ir/mono.rs:63-80`, emits both
structs → redefinition). The limitation is documented in `keys.rs:31-37` and pinned by
`borrow_generic_integrity_impl_test.rs:322-333`; the row closes with this evidence (AR-B3).

## Dependencies

### Internal Dependencies

- The batch reuses the per-instance duty audit (R22/R23), the hidden-destination contract
  (R1), the materialized-operand contract (R2), the supersede gate (R20), and the owner
  machinery (R19/R21) — each with a working precedent cited per component doc.
- `spec/check.sh` plus the compile-fail driver own the new/edited codes; the freestanding and
  corpus legs own the profile and cross-toolchain checks.

### External Dependencies

- None. The workspace stays std-only.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| DEF-95's owner machinery touches the expression-temporaries workstream | Med | Med | The smallest lever (binding-scoped owner + `StructLit` descent) with probes per sub-step; the phase review carries the memory lens |
| DEF-99 has no recorded repro | Med | Low | Reproduce first inside the phase; if it is the DEF-111 class, fix together and record both |
| DEF-97's recorded ordinal does not map | Med | Low | Reproduce from the suite as the first step of its task |
| DEF-96's gate arming could churn ordinary-return emission | Low | Med | Byte-identity constraint pinned by the existing defer suites; arm only when a defer in scope has an out-edge |
| DEF-108's inference fix could shift accepted pairings | Low | Low | Pin the accepted `if (x == xs)` spelling as a control |
| DEF-120's typedef split could reorder closures | Low | Med | Pointer-element slices stay before closures; the store-emission order pins update deliberately |
