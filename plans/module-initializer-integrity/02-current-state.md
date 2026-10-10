# Current State: Module Initializer Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

Module-level `let`/`const` initializers are parsed as `Item::GlobalLet`/`GlobalConst`
(`parse/items.rs:366`, `:387`), type-checked (`sema/mod.rs:217-223`), cycle-checked
(`sema/init_cycle.rs:46-49`), and then handed to codegen as **raw AST**: `codegen/init.rs:69-120`
returns `Vec<(String, &'a Expr)>` in dependency order, and `emit_init_functions`
(`codegen/program.rs:756-770`) writes one `static void cl_init_<module>(void)` per module whose
body is a sequence of `emit_global_init` calls (`:779-824`). The own stage's contract,
`OwnershipOutput { functions, monomorphization }` (`own.rs:70-75`), contains no initializer IR:
`check()` walks `Item::Func` only (`own.rs:107-138`), and `check_globals`
(`own/program.rs:229-258`) inspects declared types only.

The initializer value lowering is a second lowerer: `lower_global` (`program.rs:1560-1691`) plus
`lower_global_intrinsic`/`global_intrinsic_call`/`global_len_receiver` (`:1700-1826`) duplicate a
subset of `own/lower.rs`'s `lower_value` and `own/call.rs`'s call/intrinsic lowering. The
materialization walk is `materialize_init_value`/`materialize_init_local`
(`program.rs:1085-1400`, `:991-1056`), minting inline-declared `__cl_itmp<N>` locals
(`program.rs:1546-1550`) with no registry and no cleanup. The function-body contracts live in
`codegen/function.rs` (stores at `:975`/`:1489-1545`, the `str` wrap at `:1937-1941`, drops via
`codegen/drop.rs` and `codegen/cleanup.rs`).

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/codegen/program.rs` | init emission, `lower_global`, init intrinsics, init slice collection | call resolution, aggregate stores, view wrap, len fold, release registry, slice typedefs |
| `compiler/src/codegen/init.rs` | init order + `(name, expr)` list | unchanged (input to the fixes) |
| `compiler/src/codegen/function.rs` | body contracts to mirror | unchanged (contract source; predicates are `pub(super)`) |
| `compiler/src/codegen/expr.rs` | value renderer | unchanged except where a mirrored helper is exported |
| `compiler/src/own/generics.rs` | AST walk recording instances | walk `GlobalLet`/`GlobalConst` values too |
| `compiler/src/own/lower.rs` | `lower_lambda`/`lift_lambda` | reused by the init-lambda lift context |
| `compiler/src/own/expr.rs`, `own/place.rs` | mutability predicates | predicates extracted for the check-only walk |
| `compiler/src/codegen/types.rs` | `collect_slice_types` | unchanged; the body collector gains the intrinsic `type_arg` walk |
| `compiler/tests/module_initializer_integrity_{spec,impl}_test.rs` | new suites | created |

### Code Analysis

The defects fall out of the duplicated lowerer and the missing surroundings:

- **Calls (DEF-131/132/133).** `lower_global`'s `Call` arm (`program.rs:1668-1688`) spells
  `callee_name(callee).unwrap_or_default()` — empty for a lambda — and hard-codes
  `receiver: None`; `lift_lambda` (`own/lower.rs:458-592`) is reachable only from a
  `FunctionChecker`; `generics::collect` walks `Item::Func` bodies only (`generics.rs:36-45`),
  and `collect_struct_instances` reads declared global types only (`:71-76`), so initializer
  calls record no instance.
- **Aggregates (DEF-134/135/143).** The init renderer uses the expression renderer directly
  (`program.rs:821-823`), which has no destination context: struct literals become compound
  literals (`codegen/expr.rs:99-101`, `:968-995`) where C rejects an expression initializer for
  an array member, and no `store_text` wrap is applied. The init `.len` receiver uses the head
  binding's type (`program.rs:1807-1822`), so a qualified `b.arr` folds against `Box` and the
  renderer's non-array fallback yields `"0"` (`codegen/calls.rs:277-305`).
- **Release (DEF-130).** `emit_init_functions` (`:756-770`) never emits any drop;
  `materialize_init_local` records nothing. Owning intermediates (`__cl_itmp0 = mkBox()`) leak
  per program start.
- **Checks (DEF-152).** The own stage never visits initializer expressions, so
  `const G: i32 = 0; let H: i32 = i32(G++);` is accepted although the body spelling reports
  `E10023` (`own/expr.rs:907-970`, `:986-1020`).
- **Slice typedefs (DEF-137).** The init side collects written type arguments
  (`program.rs:323-396`, `:374-386`); the body side scans locals, temps, signatures, and
  instances (`:268-312`) but not the IR `ValueKind::Intrinsic { type_arg }`
  (`own/lower.rs:310-337`), so `sizeof(i32[])` in a body misses `ClSlice_i32`.

## Gaps Identified

### Gap 1: Initializer calls lack callable resolution (DEF-131/132/133)
**Current Behavior:** lambda callee → empty callee name and an unresolved callable; func-typed
global → the written name, not the mangled symbol; generic call → an instance-less symbol;
dot call → a free call without its receiver. Strict builds fail.
**Required Behavior:** R1 – R4 — the body path's resolutions, verbatim.
**Fix Required:** an own-stage init walk (instance recording + lambda lifting) and the
`lower_global` Call arm mirroring `lower_dot_receiver`/`func_field_receiver` (03-01).

### Gap 2: Initializer aggregates emit invalid or wrong C (DEF-134/135/143)
**Current Behavior:** compound literal with an array-field expression; an unwrapped concat at a
`str` destination; a qualified `.len` folded against the head type (`0`).
**Required Behavior:** R5 – R7 — the body contracts (03-02).
**Fix Required:** destination-aware init stores, the `str` wrap, and the structural fold.

### Gap 3: No release and no checks for initializers (DEF-130/152)
**Current Behavior:** owning init locals leak at program start; const writes are accepted
(silently mutating a const at startup, or emitting invalid C).
**Required Behavior:** R8, R9 (03-03).
**Fix Required:** an init-owner registry with null-guarded releases at the init function's end;
a check-only mutability walk over initializer expressions.

### Gap 4: Body written slice type arguments miss their typedef (DEF-137)
**Current Behavior:** `let n: usize = sizeof(i32[]);` in a body emits `sizeof(ClSlice_i32)` with
no typedef; the strict build fails.
**Required Behavior:** R10 (03-04).
**Fix Required:** collect `ValueKind::Intrinsic { type_arg }` from body IR values.

## Dependencies

### Internal Dependencies

- `codegen/program.rs`'s init emission is the batch's main surface; `codegen/init.rs` supplies
  the ordered `(name, expr)` list unchanged.
- The lambda lift reuses `own/lower.rs lift_lambda`; the instance walk extends
  `own/generics.rs collect`; the checks reuse `own/expr.rs`/`own/place.rs` predicates.
- The existing init pins (`eval_emission_integrity_*`, `store_emission_integrity_*`) are the
  byte-identity oracle for unaffected shapes.

### External Dependencies

- None (std-only workspace, pinned Docker toolchain).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Byte-identity drift for unaffected initializers | Med | High | Shape-gate every change; the corpus differential per phase |
| Two lowerers stay in sync by discipline only | Med | Med | Mirror verified body contracts; name the duplication; Design A recorded as a future option |
| The synthetic checker context for the lambda lift drags emission side effects | Med | Med | Lift-only use; discard the synthetic body IR; the emission auditor diffs the corpus |
| The check walker misses a write form the initializer grammar admits | Low | Med | The semantics reviewer probes every write form in both spellings |
| Release double-fires on conditional fills | Low | High | Null-guarded reverse-order drops; the memory auditor probes multi-path fills |
