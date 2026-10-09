# Current State: Borrow/Generic Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The ownership pass (`compiler/src/own/`) checks assignments and calls over the typed IR; the
emitter renders stores by releasing the target's old value before the store. Borrow parameters
are classified at declaration (`own.rs declare_param`: a nullable `borrow` parameter is an
`OutSlot`, otherwise a `Param`). Generic bodies defer substitution-dependent judgments to
per-instance duties (`ir::Requires*` records) audited in `own/instance_check.rs`. Type keys
(`types/keys.rs type_key`) name C symbols; codegen adds a nullability marker only on the
function-instance argument path (`codegen/types.rs instance_arg_key`). The sema operator rules
(`sema/ops.rs comparable`) gate `==`/`!=`. The may-lend gate (`own/lower.rs value_may_lend` +
`lowered_lend_source`) resolves a lowered callee by name through `own/program.rs callee`.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/expr.rs` | `check_assign` / store preparation | DEF-78 target-side check; DEF-82 self-assign rejection |
| `compiler/src/own/place.rs` | `move_place`, `mark_moved_if_owning`, duty recorders | Reference for the existing store/release rule; only if the dedupe needs the shared helper (AR-13) |
| `compiler/src/own/instance_check.rs` | Per-instance audits | New audit for the DEF-78 duty |
| `compiler/src/ir/mono.rs`, `compiler/src/ir.rs` | Duty record types and their re-export | New `RequiresStorableSlot` record (AR-5) |
| `compiler/src/codegen/function.rs` | Local-name table, guards | DEF-77: carry masked parameter names (PF-001) |
| `compiler/src/own/call_check.rs` | Call-argument checks, `W10001` | DEF-83 exemption; consult the effective (substituted or inferred) parameter type |
| `compiler/src/own/call.rs` | Call lowering and type inference | DEF-83: share the inferred substitutions with the warning decision |
| `compiler/src/own.rs` | `declare_param`, `unique_ir_name`, place naming | DEF-77 masking guard + parameter-name plumbing |
| `compiler/src/own/lower.rs` | `value_may_lend`, `lowered_lend_source` | Consumes the fixed callee names |
| `compiler/src/types/keys.rs` | `type_key` | DEF-79 nullability (AR-4) |
| `compiler/src/codegen/types.rs` | `c_type`, typedefs, `instance_arg_key` | DEF-79; `instance_arg_key` removal (AR-6) |
| `compiler/src/codegen.rs`, `compiler/src/codegen/expr.rs` | Instance call keys, text conversion | Follow the `type_key` change; text-call carve-out |
| `compiler/src/sema/ops.rs` | `comparable`, binary result | DEF-81 view-versus-`null` pairing |

### Code Analysis

- Store path: `own/expr.rs check_assign` handles whole-binding assignment; `own/statements.rs
  emit_store`/`prepare_store` emits `Statement::Move`/`Assign`, and the emitter drops the old
  target value. `own/place.rs move_place` rejects *moves out of* lend parameters (`E10034`) but
  nothing checks a store *into* a lend parameter.
- Out-slot classification: `own.rs declare_param` — `param.mode == Borrow && ty.nullable` is an
  `OutSlot`; anything else is a `Param`. The `OutSlot` kind is what makes out-param writes legal.
- Duty pattern: `own/place.rs mark_moved_if_owning` records `RequiresNotLend` for lend moves;
  `own/instance_check.rs check_not_lend_requirements` reports `E10034` when the substituted type
  is non-duplicable.
- Keys: `types/keys.rs type_key` renders `TyKind::Struct(name, args)` arguments through
  `type_key(arg)` with no nullability; `codegen/types.rs instance_arg_key` appends `Q` for
  nullable arguments on the function-instance path only.
- Comparison: `sema/ops.rs comparable` lists scalars, enums, `raw`, handles, `Str`, `CString`,
  `StaticStr` — arrays are excluded, so every slice comparison is `E10222`.
- Emission support for DEF-81 already exists: `codegen/expr.rs view_null_test` lowers a
  view-versus-`null` comparison to `(view).ptr == NULL`; the narrowing machinery
  (`own/place.rs narrowing`, `own/statements.rs apply_narrowing`) is type-agnostic.
- May-lend: `own.rs unique_ir_name` masks a module-callee name collision only for
  `BindingKind::Local`; `own/call.rs lower_call` uses a func-typed binding's IR name as the
  callee. A func-typed *parameter* keeps its plain name, so `program.callee(name)` resolves the
  module function and the gate treats the call as lending. Name-based consumers that re-resolve
  the lowered callee: `value_may_lend`/`lowered_lend_source`, `collect_lowered_temp_owners`,
  `take_move_out_places`, and `materialized_ty` (`own/lower.rs`). The C definition signature and
  `local_names` are built from the AST source names (`codegen.rs`, `codegen/function.rs`), while
  places and callees use `binding.ir_name` (`own.rs`) — masking a parameter therefore needs the
  name carried through the IR and the emitter (PF-001).

## Verified Reproductions (plan baseline)

All six defects were reproduced at the plan baseline with `target/debug/clc` and the host C
compiler under AddressSanitizer where the failure is a memory error.

| # | Repro (source shape) | Observed behavior |
| - | -------------------- | ----------------- |
| DEF-78 | `func fill<T>(take v: T, borrow slot: T): void { slot = v; }` called with `String`; the concrete spelling `func fill(take v: String, borrow slot: String)`; and a fresh-value store `slot = stringOf("x")` | All accepted; the callee releases the caller's shared storage and the caller drops it again — ASan `attempting double-free`. A duplicable slot (`borrow i32`) stays accepted with `W10001` (legal, must remain so) |
| DEF-82 | `func self(borrow slot: String?): void { slot = slot; }` called with `borrow s` | Accepted; the caller's `s` is emptied (probe exit 1 where the value should survive). A local `s = s;` alone compiles; `E10020` fires at the later use of the moved binding |
| DEF-83 | `func set(borrow x: raw u8?): void { x = null; }` called with `borrow r` | Accepted with `warning W10001: unnecessary lend of duplicable 'r'` |
| DEF-79 | `struct Wrap<T> { v: T; }` used as both `Wrap<String>` and `Wrap<String?>` | Accepted; the emitted C defines `struct ClWrap__String` twice — strict build fails: `error: redefinition of 'struct ClWrap__String'` |
| DEF-81 | `func demo(xs: u8[]?): bool { return xs == null; }` | `error E10222: operator '==' is not defined for 'u8[]?'` |
| DEF-77 | A module lender `give(borrow String): str` plus `func demo(give: func(borrow String): String): i32 { let s: String = stringOf("x"); return i32(len(give(s))); }` | Accepted; ASan `LeakSanitizer: detected memory leaks` (the owned result of the call through the parameter is never bound) |

Repro scratch files live under the session temporary directory and are not retained; the shapes
above are the full programs.

## Gaps Identified

### Gap 1: A store into a lend parameter releases caller storage (DEF-78)

**Current Behavior:** the store is accepted for concrete and generic spellings; the emitted drop
of the parameter's old value frees storage the caller still owns (double free).
**Required Behavior:** hostile-free — reject the store with `E10034` when the (substituted) slot
is non-nullable and owns storage; keep nullable out-slots and duplicable lends accepted.
**Fix Required:** the ownership assignment path rejects the concrete owning case and records a
per-instance duty for parameter-typed slots; the target-side judgment is skipped when the right
side is already judged (one report per site); the sibling not-lend audit exempts nullable
substitutions so the generic nullable spelling stays legal (03-01; AR-2, AR-13, PF-004, PF-006).

### Gap 2: Out-slot self-assignment destroys the caller's value (DEF-82)

**Current Behavior:** accepted; the callee drops the old value and stores the emptied read, so
the caller's slot loses its value.
**Required Behavior:** rejected `E10020`, consistent with the local self-assignment rule.
**Fix Required:** whole-binding out-slot self-assignment is rejected (03-01; AR-8).

### Gap 3: Spurious `W10001` on a nullable out-slot (DEF-83)

**Current Behavior:** `borrow raw?` warns `unnecessary lend of duplicable`, although the
out-slot writes through to the caller's storage.
**Required Behavior:** no warning for any nullable borrow (out-slot) parameter.
**Fix Required:** the warning path exempts out-slot parameters (03-01; AR-9).

### Gap 4: Struct instance keys ignore nullability (DEF-79)

**Current Behavior:** `type_key` drops the nullable flag, so `Wrap<String>` and `Wrap<String?>`
share one symbol and the emitted C redefines the struct.
**Required Behavior:** keyed spellings that carry a nullable type argument are distinct; function
instance symbols stay unchanged.
**Fix Required:** nullability folds into `type_key`; the text-conversion spelling is preserved;
nested nullable function-instance arguments change spelling (an expected fixing diff; PF-003)
(03-02; AR-4, AR-6).

### Gap 5: A nullable view cannot be narrowed (DEF-81)

**Current Behavior:** `xs == null` on a slice reports `E10222`; the narrowing path is unreachable.
**Required Behavior:** view-versus-`null` equality in either order is accepted (any view
nullability); slice-view `==` slice-view and ordering stay `E10222`.
**Fix Required:** the sema equality rule pairs a view operand with the null operand (03-03;
AR-7).

### Gap 6: A func-typed parameter named like a module lender leaks (DEF-77)

**Current Behavior:** the may-lend gate resolves the callee by name, finds the module function,
treats the result as aliasing, and skips binding the owned result — a leak.
**Required Behavior:** a call through a binding is never mistaken for a module call; the owned
result is bound and released once.
**Fix Required:** the callee-name masking guard covers every binding kind, with the masked
parameter spelling carried through the IR and the emitter; the two existing pinned tests update
(03-04; AR-3, PF-001).

## Dependencies

### Internal Dependencies

- P1's three fixes share `own/expr.rs`'s assignment path; P3 depends only on sema; P4 carries
  the masked parameter spelling through `codegen/expr.rs`, which P2 also touches (sequential
  phases; PF-001).
- The P2 and P4 emission changes are verified by the existing emission sweep and determinism
  tests; P2 also pins the nested function-instance spelling change (PF-003).

### External Dependencies

None — the workspace is std-only; the probes reuse the host C compiler and AddressSanitizer
already used by the repo's test tooling.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The DEF-78 check double-reports with the existing lend-move duty at one site | Med | Low | The one-report-per-site invariant is pinned by ST-3 (concrete and generic both-borrow clauses); AR-13 names the recording-time skip (PF-004) |
| The `type_key` change perturbs emissions beyond struct instances | Med | Med | P2 runs the base-versus-head emission sweep and the byte-identical re-emission pins (ST-12) with the emission-integrity review; nested nullable function-instance spellings change by design (PF-003) |
| The DEF-77 masking change alters emitted parameter names for colliding names | Low | Low | The diff is limited to bindings whose source name equals a module callee; the spelling is carried through the emitter (PF-001); ST-20 pins the module-call case, ST-17..19 the call behavior; the two existing pinned tests update |
| The projected out-slot self-assign clause depends on P3 (the shape is blocked at baseline) | Low | Low | ST-7's projected clause runs at P3, where the check becomes expressible; the runtime-indexed variant is a filed residual (PF-005) |
| View narrowing exposes an untested path end-to-end | Low | Med | Code inspection shows the narrowing machinery is type-agnostic; ST-14 pins the run behavior |
