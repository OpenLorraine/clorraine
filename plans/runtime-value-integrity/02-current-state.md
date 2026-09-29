# Current State: Runtime Value Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The ownership stage lowers expressions into IR statements and values; the codegen stage
renders IR to C. Three mechanisms matter here:

- **Materialized guard operands.** `FunctionChecker::materialize_guard_operand`
  (`compiler/src/own/lower.rs:523-553`) wraps a value in
  `ValueKind::Materialize { temp, value }` (`compiler/src/ir.rs:248-253`) so a guarded
  operand's call runs only on the guarded path. The temporary is a plain C local
  (`compiler/src/codegen/function.rs:249-263`, `__cl_tmpN`, empty-state initialized) and
  the guard emitter places its assignment inside the guard
  (`compiler/src/codegen/guards.rs:103-125`, `:286-307`).
- **Value stores.** A fixed-array value is stored through
  `emit_fixed_array_assign` (`compiler/src/codegen/function.rs:659-705`), which emits a
  byte `memcpy` for a duplicable element type and a save-and-release shape for an owned
  one.
- **Element loops.** `check_for_in` (`compiler/src/own/loops.rs:379-408`) lowers the
  iterable with `lower_value` and indexes it directly; a `borrow` element binds a
  `DerefTemp` alias (`:302-367`).

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/lower.rs` | Guard-operand materialization, call/take lowering, conditional arms | Owning-temp tracking, conditional materialization, deferred move-outs |
| `compiler/src/own/loops.rs` | Element-loop checking and lowering | Materialize a call-rooted iterable |
| `compiler/src/own/statements.rs` | Statement dispatch, stores, returns | Statement-end release drain (after each statement, before terminators) |
| `compiler/src/own/place.rs` / `own/liveness.rs` | View provenance, loans, and the loan-lifetime rule | Statement-scoped temporary ownership so a derived view's later use reports `E10030` |
| `compiler/src/own.rs` | Binding state and temp creation | Temp release bookkeeping |
| `compiler/src/ir.rs` | IR values and statements | Statement-position release representation |
| `compiler/src/codegen/function.rs` | Fixed-array assign, call emission, temp declarations | Branch-aware conditional assign, receiver guards, func-value destinations, inline release |
| `compiler/src/codegen/guards.rs` | Guard emission for values and statements | Deferred move-outs, conditional/receiver handling |
| `compiler/src/codegen/types.rs` | C type rendering | Func-pointer hidden out-param ABI |
| `compiler/src/codegen/cleanup.rs` | Scope/drop emission, fixed-array returns | Receiver guard and branch-aware conditional store on the return call path |
| `compiler/runtime/shim_host.c`, `shim_hosted_amiga.c`, `shim_freestanding.c` | Runtime | `cl_panic` takes pointer + length |

### Code Analysis

- `Materialize` is constructed at exactly one site (`own/lower.rs:545-552`); its callers
  are the index, division, and float-to-int operand materializations
  (`own/lower.rs:45-46`, `:122-123`, `:140`), the compound/indexed stores
  (`own/expr.rs:517-518`, `:568-569`), and slice operands (`own/call.rs:261`). No drop
  statement is ever emitted for a compiler temporary (`own.rs:400-408` emits drops only
  for bindings; `codegen/cleanup.rs:39-52` scans only existing `Drop`/`Defer` statements,
  which it renders at scope end). Two other temp producers — `materialize_value`
  (`own/lower.rs:560-570`, the take-call result) and `consume_place` (`:580-599`, the
  aggregate-literal element adoption) — emit their store immediately and are intentionally
  left aliased to their destination; they are transfer temps, not release candidates.
- `spec/17-codegen.md:80` (D5, O2) requires a temporary to be dropped at statement end in
  reverse creation order; the current emitter deviates for every owning temporary.
- The element loop takes any iterable the checker accepts (`own/loops.rs:379-381`;
  `sema/stmt.rs:198-200`) but lowers a call-rooted iterable inline, producing
  `mk()[i]` while the callee is emitted with the hidden out-parameter ABI
  (`codegen/cleanup.rs:301-361`), so the C compiler rejects the call.
- A fixed-array conditional (`ValueKind::Conditional`) is rendered as a C ternary
  (`codegen/expr.rs:73-82`) and then byte-copied (`function.rs:666-671`, `:688-704`); C
  decays the array operands to pointers, so `memcpy(&target, &(cond ? a : b), n)` does not
  compile. The return path is separate: `emit_fixed_array_return`
  (`codegen/cleanup.rs:308-361`) falls back to `__cl_ret = (cond ? a : b);`, and an indexed
  element target goes through `Statement::StorePointer`
  (`codegen/function.rs:390-399`), which emits an invalid array assignment — that broader
  element-store form is tracked as `DEF-31`.
- A call through a func-typed field renders the receiver inline
  (`codegen/expr.rs:408-411`) and guards only the arguments on the statement path
  (`function.rs:461-465`), the fixed-array assign path (`:730-732`), and the fixed-array
  return path (`cleanup.rs:339-341`); the expression-guard path already guards the
  receiver (`guards.rs:54-59`, `:240-249`).
- `func_pointer_type`/`func_pointer_declarator` (`codegen/types.rs:58-73`) ignore the
  hidden out-param ABI (`codegen.rs:403-424`), so a `func(i32): i32[2]` value renders
  `i32 (*)(i32)` while the callee is `void (...)(i32, i32 *)`; call destinations are only
  derived for named functions (`function.rs:471-476`, `:580-586`), so a discarded call
  through a local func value drops the destination. A closure struct's `fn` field and a
  lifted lambda's own signature are built inline with `c_type`
  (`codegen/program.rs:267-283`, `codegen/function.rs:198-220`), not through the
  func-pointer helpers; the accepted-but-invalid fixed-array-returning form is filed as
  `DEF-30`.
- `panic_message` (`function.rs:527-532`) passes a literal as a C string and every other
  value as its rendered spelling; `str` renders `ClStr` and `String` renders a `void *`
  handle, while `cl_panic` takes `const char *` in all three shims. `sema` does not type
  the message (`sema/expr.rs:802-805`).
- A `take` argument lowers to a materialized value plus an eagerly emitted
  `Statement::MoveOut` (`own/lower.rs:95-107`, `own/call.rs:308-322`); the take-call
  result is stored by `materialize_value` (`own/lower.rs:102`), whose `ValueKind::Temp`
  the guard emitter treats as an always-evaluated read (no `Temp` arm; the `_ => {}`
  catch-all at `codegen/guards.rs:319`), so the call and the move-out run even when the
  operand is not evaluated.

## Gaps Identified

### Gap 1: Owning temporaries are never released (DEF-23)

**Current Behavior:** A materialized owning temporary (`String[2]`, `AnyHandle`) is never
dropped; its elements leak and the statement-end rule of `spec/17` is unimplemented. A
view derived from such a temporary (`let v: str = mk()[0];`) is accepted today because
the temporary is not an owner in the view/loan model, so the leak is the only thing that
keeps the view valid.
**Required Behavior:** Every owning destination temporary is released at the end of its
statement in reverse creation order, only on the paths where it was evaluated. A view
derived from a released temporary is a loan of that temporary, so a use with no
release-free path reports the existing `E10030`. Transfer temps (the take-call result and
the aggregate-literal adoption temp) stay untouched because their destination owns the
value.
**Fix Required:** Track the destination materializations during statement lowering
(guard-operand temps, the iterable temp, the conditional read-position temp) and emit a
dedicated statement-position release after each statement and before every terminator; a
scope-collected `Statement::Drop` would fire at scope end instead. Register each released
temporary as a loan owner so the liveness pass reports `E10030` on a later derived-view
use; an unevaluated temp stays in its emitted empty state, so its release is a no-op.

### Gap 2: A call-rooted iterable lowers inline (DEF-24)

**Current Behavior:** `for (x in mk())` is accepted and emits `mk()[i]`, which fails the C
compile because the callee uses the hidden out-parameter ABI.
**Required Behavior:** The call result is materialized once into a compiler temporary
owned by the loop statement and released at its end; the loop indexes the temporary.
**Fix Required:** Materialize the iterable in `check_for_in` when it has no place.

### Gap 3: A fixed-array conditional emits invalid C (DEF-25)

**Current Behavior:** `let c: i32[2] = ok ? a : b;` and `s = ok ? s : t;` are accepted and
emit `memcpy` over a C ternary.
**Required Behavior:** Every routed position that stores a fixed-array conditional value
emits a branch that evaluates only the chosen arm: a byte copy for a duplicable arm, a
move (copy plus zeroed source) for an owned arm; an arm that names the assignment target
resolves to the saved old value. The return position uses the same branch shape into
`__cl_ret`, and a place-arm conditional argument stays a forwarding value.
**Fix Required:** Make the shared fixed-array assignment path branch-aware for
`ValueKind::Conditional`, teach `emit_fixed_array_return` the same branch shape, and
extend the place-projection predicate so a conditional argument whose arms are places
stays a forwarding value (no owning copy); a conditional argument with a call arm is
filed as `DEF-32`, and the indexed element target goes through `StorePointer` and is
filed as `DEF-31`.

### Gap 4: A runtime-index receiver is not bounds-guarded (DEF-26)

**Current Behavior:** `hs[i].run(1);` reads out of bounds before the call.
**Required Behavior:** The receiver's runtime index is guarded with the existing `E11211`
trap on every call path, exactly as arguments are.
**Fix Required:** Guard the receiver in the statement call path, the fixed-array call
assign path, and the fixed-array return path.

### Gap 5: A func pointer with a fixed-array return has the wrong C type (DEF-27)

**Current Behavior:** `func(i32): i32[2]` renders `i32 (*)(i32)`; the C compiler rejects
the assignment and the call.
**Required Behavior:** Every filed `func` spelling with a fixed-array return (struct
fields, locals, parameters, and discarded calls through those) renders the hidden
out-parameter ABI (`void (*)(params..., <destination>)`), and a call through a func value
passes its destination (a discard temporary for a discarded result). A discarded call
through a local func-typed callee resolves the local's declared return type.
**Fix Required:** Fix the declarator helpers and the call-side destination detection for
func values, including the local-callee lookup; closure and lifted-lambda signatures are
separate surfaces filed as `DEF-30`.

### Gap 6: A non-literal panic message fails the C compile (DEF-28)

**Current Behavior:** `panic(s)` with `s: str` or `String` renders a view/handle where
`cl_panic` takes `const char *`.
**Required Behavior:** A literal, `str`, or `String` message compiles and prints
`panic: <message>`; the runtime takes pointer and length so no NUL assumption is needed.
**Fix Required:** Change `cl_panic` in all three shims and render the message pair at the
call site.

### Gap 7: A `take` argument in a guarded operand runs eagerly (DEF-29)

**Current Behavior:** The call and its `MoveOut` are emitted before the guard, so a
short-circuited operand still moves the value.
**Required Behavior:** The move-out travels with the guarded operand's materialization and
runs only on the evaluated path.
**Fix Required:** Carry the move-out places on the materialized value and emit them in the
guard emitter.

## Dependencies

### Internal Dependencies

- Gap 1's release design and its statement-scoped temporary loan rule (AR-18) are a
  prerequisite for Gaps 2 and 3 (their temporaries need the release rule, and their
  element reads may derive views).
- Gap 7 shares the materialization value shape with Gap 1.
- The corpus and stdlib must keep compiling under `make check` and `make verify`; no
  source-language change is involved.

### External Dependencies

None (stdlib-only Rust workspace; the C runtime shims change in-repo).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| An emitted drop for an unevaluated temp drops garbage | Low | High | Temps are empty-state initialized; the drop machinery already relies on empty-state safety for moved-out bindings; pin with a guard-short-circuit run case and an ASan probe |
| The release frees storage a live derived view points into | Med | High | Statement-scoped temporary loan provenance plus the existing `E10030` rule; a positive same-statement case and a rejected later-use case pin it (ST-24, ST-25) |
| The release set includes a transfer temp and double-frees an adopted value | Med | High | The set is pinned to the destination materializations (AR-18); the take-call and literal-adoption temps are excluded; a unit test pins the registered sites and the existing double-free suites stay green |
| The return path is missed and ST-8 stays red | Med | High | Include `codegen/cleanup.rs` in the conditional-store task and pin the emitted branch shape in the impl tests |
| Branch-aware assignment changes the owned-array self-assignment path (`s = ok ? s : t`) | Med | High | Design resolves an arm that names the target to the saved old value; dedicated ST cases for both branches |
| Deferred move-outs change the emitted shape of existing guarded calls | Low | Med | The existing laziness tests (`array_value_integrity`, `codegen_run_spec_test`) stay green; impl tests pin the new shape |
| Closure and lifted-lambda fixed-array returns are already invalid and remain so | High | Low | Out of scope and filed as `DEF-30`; no existing test regresses because no test covers the form |
| Runtime shim signature change breaks an existing caller | Low | Med | `cl_panic` has exactly one emitter call site and the three shims; grep-verified in recon |
