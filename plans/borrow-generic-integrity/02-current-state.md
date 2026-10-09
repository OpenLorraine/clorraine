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
| `compiler/src/own/place.rs` | `move_place`, `mark_moved_if_owning`, duty recorders | Reference for the existing store/release rule; possibly a shared helper |
| `compiler/src/own/instance_check.rs` | Per-instance audits | New audit for the DEF-78 duty |
| `compiler/src/ir/mono.rs` | Duty record types | New `RequiresStorableSlot` record (AR-5) |
| `compiler/src/own/call_check.rs` | Call-argument checks, `W10001` | DEF-83 exemption |
| `compiler/src/own.rs` | `declare_param`, `unique_ir_name` | DEF-77 masking guard |
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
  module function and the gate treats the call as lending.

## Verified Reproductions (plan baseline)

All six defects were reproduced at the plan baseline with `target/debug/clc` and the host C
compiler under AddressSanitizer where the failure is a memory error.

| # | Repro (source shape) | Observed behavior |
| - | -------------------- | ----------------- |
| DEF-78 | `func fill<T>(take v: T, borrow slot: T): void { slot = v; }` called with `String`; the concrete spelling `func fill(take v: String, borrow slot: String)`; and a fresh-value store `slot = stringOf("x")` | All accepted; the callee releases the caller's shared storage and the caller drops it again — ASan `attempting double-free`. A duplicable slot (`borrow i32`) stays accepted with `W10001` (legal, must remain so) |
| DEF-82 | `func self(borrow slot: String?): void { slot = slot; }` called with `borrow s` | Accepted; the caller's `s` is emptied (probe exit 1 where the value should survive). The local form `s = s` is rejected `E10020: 's' was moved and cannot be used` |
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
per-instance duty for parameter-typed slots (03-01; AR-2, AR-13).

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
**Fix Required:** nullability folds into `type_key`; the text-conversion spelling is preserved
(03-02; AR-4, AR-6).

### Gap 5: A nullable view cannot be narrowed (DEF-81)

**Current Behavior:** `xs == null` on a slice reports `E10222`; the narrowing path is unreachable.
**Required Behavior:** view-versus-`null` equality in either order is accepted (any view
nullability); `view == view` and ordering stay `E10222`.
**Fix Required:** the sema equality rule pairs a view operand with the null operand (03-03;
AR-7).

### Gap 6: A func-typed parameter named like a module lender leaks (DEF-77)

**Current Behavior:** the may-lend gate resolves the callee by name, finds the module function,
treats the result as aliasing, and skips binding the owned result — a leak.
**Required Behavior:** a call through a binding is never mistaken for a module call; the owned
result is bound and released once.
**Fix Required:** the callee-name masking guard covers every binding kind (03-04; AR-3).

## Dependencies

### Internal Dependencies

- P1's three fixes share `own/expr.rs`'s assignment path; P2 and P4 each stand alone; P3 depends
  only on sema.
- The P2 emission change is verified by the existing emission sweep and determinism tests; no
  other phase depends on it.

### External Dependencies

None — the workspace is std-only; the probes reuse the host C compiler and AddressSanitizer
already used by the repo's test tooling.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The DEF-78 check double-reports with the existing lend-move duty at one site | Med | Low | The one-report-per-site invariant is pinned by ST-1/ST-2; AR-13 names the dedupe rule |
| The `type_key` change perturbs emissions beyond struct instances | Med | Med | P2 runs the base-versus-head emission sweep and the byte-identical re-emission pins (ST-11) with the emission-integrity review |
| The DEF-77 masking change alters emitted parameter names for colliding names | Low | Low | The diff is limited to bindings whose source name equals a module callee; ST-19 pins the non-func case, ST-16..18 the call behavior |
| View narrowing exposes an untested path end-to-end | Low | Med | Code inspection shows the narrowing machinery is type-agnostic; ST-13 pins the run behavior |
