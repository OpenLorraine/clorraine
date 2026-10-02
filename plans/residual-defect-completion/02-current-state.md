# Current State: Residual Defect Completion

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The compiler lowers an ownership-annotated, typed IR to one C99 translation unit. Generic
functions are checked once and emitted once per recorded instance with a type substitution
installed; `type_key` renders the C symbol of an instance. Lifted lambdas are separate IR
functions with a `lambda_site` marker. Ownership decisions (drops, moves, materialization) are
made in the ownership stage and replayed by codegen, and conditionally evaluated guards embed
their side effects as C comma-expression operands.

The six defects are gaps in these existing mechanisms, not missing subsystems. Each was
reproduced against the committed compiler (baseline `d438347`) and the failure recorded.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/codegen/types.rs` | `type_key`, `c_type`, slice typedefs | Encode extents in the array arm (R1); per-instance callable-type spelling (R3) |
| `compiler/src/codegen/calls.rs` | Argument rendering and the pointer decision | Substitute the argument type before `is_pointer_param` (R2) |
| `compiler/src/own/generics.rs` | Instance walk and transitive closure | Walk a lifted function with its enclosing instance's bindings (R3) |
| `compiler/src/own.rs` | Lowering boundary; `lifted` grouping | Record which declared function each lifted function belongs to (R3) |
| `compiler/src/ir.rs` | `Function.lambda_site` | Carry the enclosing-function provenance (R3) |
| `compiler/src/codegen/program.rs` | Instance emission, lifted emission, closure declarations | Emit one lifted specialization per needed instance; per-instance environments; reference the specialized symbols (R3) |
| `compiler/src/codegen/function.rs` | `emit_lifted_function`, `lifted_signature` | Install the instance substitution and the specialized symbol (R3) |
| `compiler/src/codegen/expr.rs` | Closure value, call, and environment rendering | Render the active specialization's lambda/closure/env spellings (R3) |
| `compiler/src/own/lower.rs` | Materializer; ownerless temporaries | Bind an ownerless call result before a projection or owning element read, and emit the take element move-out (R4, R5) |
| `compiler/src/own/call_check.rs` | Take-argument place effects | No change: the maybe-owning suppression stays; lowering owns the binding (R5) |
| `compiler/src/own/expr.rs` | `is_ownerless_element_move`, read hooks | No change for the take path; the lowering binding owns it (R5) |
| `compiler/src/codegen/guards.rs` | `guard_expressions`, `emit_reassigned_temp_drop` | Hoist the statement-form drop for `drop_expr_text == None` operands (R6) |
| `compiler/src/codegen/drop.rs` | `drop_expr_text`, statement drop | No change expected; the `None` case stays (R6) |
| `compiler/tests/residual_defect_completion_{spec,impl}_test.rs` | New suites | Create (all) |
| `compiler/tests/call_value_integrity_impl_test.rs` | The DEF-51 skip pin | Rewrite the pin to the fixed shape (AR-10) |

### Code Analysis

**DEF-50.** `type_key` renders a struct instance as `Name__<arg keys>` and folds an array to its
element: `TyKind::Array { element, .. } => type_key(element)` (`compiler/src/codegen/types.rs:290`).
The key feeds the C struct name (`types.rs:49`), instance symbols (`compiler/src/codegen/program.rs:382`),
slice typedefs (`types.rs:56`), and the text helpers (`compiler/src/codegen/expr.rs:660`), so two
distinct fixed-array arguments collide on one symbol. The unit tests at `types.rs:374-381` pin only
scalar and struct arguments.

**DEF-54.** `call_arguments` decides whether a named-call argument is passed by pointer
(`compiler/src/codegen/calls.rs:165-175`). The function-type branch substitutes the expected type
before the decision (`:169-171`), but the named-call branch tests `&arg.ty` directly (`:174`). A
borrow argument whose written type is `T` still carries the parameter at codegen time, so
`is_pointer_param` sees a param and decides "by value", while the callee's C parameter is the
substituted `ClInner__String *`.

**DEF-52.** Lowering appends each lifted function to the function list right after its enclosing
declared function and marks it with `lambda_site` (`compiler/src/own.rs:114-117`,
`compiler/src/ir.rs:548-554`), but carries no link to the enclosing function. The transitive
instance walk re-walks only the declared function named by an instance
(`compiler/src/own/generics.rs:453-491`), and emission writes every lifted function exactly once
without a substitution (`compiler/src/codegen/program.rs:756-766`,
`compiler/src/codegen/function.rs:119-135`). A generic call inside the lifted body therefore keeps
the enclosing parameter and is never instantiated.

**DEF-53.** For a `take` argument, `check_argument_place` suppresses both the move marking and the
element-move diagnostic when the argument type is maybe-owning
(`compiler/src/own/call_check.rs:281-305`; `is_ownerless_element_move` at
`compiler/src/own/expr.rs:201-217`). Under an owning substitution the callee still releases the
element, while the ownerless call result releases the same storage, so the element frees twice.

**DEF-55.** An owning read of a projection from a fresh call result is emitted inline:
`return cl_repro_def55_mkv().a;`. Nothing binds the call result, so its other owning fields are
never released (the `let` and take-argument forms leak the same way).

**DEF-51.** Guards are built as comma-expression operands; the materialized temporary's drop is
pushed only when `drop_expr_text` returns `Some` (`compiler/src/codegen/guards.rs:680-684`). A
growable array whose element needs a drop has no expression form
(`compiler/src/codegen/drop.rs:160-164`), so the re-store skips the previous value. The statement
path already exists for the always-evaluated case: `emit_reassigned_temp_drop`
(`guards.rs:215-226`) uses the statement-form `emit_drop`, and every guarded loop has a
per-iteration slot (`compiler/src/ir/guards.rs:71-74`, `compiler/src/codegen/control.rs:346-352`).
Temps are zero-initialized (`compiler/src/codegen/function.rs:251-258`), so an early hoisted drop
is null-safe.

## Reproductions

All six were compiled with `target/debug/clc --profile host` and built with
`clang -std=c99 -O0 -g -fsanitize=address`; memory cases ran with
`ASAN_OPTIONS=detect_leaks=1`.

| Defect | Reproduction | Observed failure on baseline `d438347` |
| ------ | ------------ | -------------------------------------- |
| DEF-50 | `Grid<i32[2]>` and `Grid<i32[3]>` values in one `main` | C build: `error: redefinition of 'ClGrid__i32'` |
| DEF-54 | `probe<T>(borrow p: Outer<T>)` calling `sink<T>(borrow p.v)`, instantiated `String` | C build: passing `ClInner__String` where `ClInner__String *` is expected |
| DEF-52 | `outer<T>` with a `func(T): T` lambda calling `apply<T>`, instantiated `i32` | C build: `error: unknown type name 'ClT'` (twice) |
| DEF-53 | `at<T>(mk: func(): T[2], i)` calling `sink<T>(mk()[i])`, instantiated `String` | ASan: `attempting double-free` in `free` |
| DEF-55 | `return mkv().a`; `let s: String = mkv().a`; `sink(mkv().a)` over a two-`String` struct | LSan: one leak report per form (the sibling field) |
| DEF-51 | `while (i < 2 && grow()[0].count > 0)` with `grow(): Array<String>[2]` | LSan: two direct 16-byte leaks plus indirect string bytes |

## Gaps Identified

### Gap 1: array extents are absent from the symbol key (DEF-50)
**Current Behavior:** distinct fixed-array arguments share one C struct and instance symbol.
**Required Behavior:** each distinct extent list renders a distinct deterministic key.
**Fix Required:** encode `arr<dims>_<element>` (fixed) and `slice_<element>` (unknown extent) in
`type_key` (03-01).

### Gap 2: the pointer decision reads the unsubstituted argument type (DEF-54)
**Current Behavior:** a param-typed borrow projection is passed by value.
**Required Behavior:** the decision uses the substituted argument type.
**Fix Required:** substitute before `is_pointer_param` in `call_arguments` (03-01).

### Gap 3: lifted lambdas have no instance context (DEF-52)
**Current Behavior:** one unsubstituted body is emitted and the nested generic call is never
recorded.
**Required Behavior:** one specialization per enclosing instance, with nested instances recorded.
**Fix Required:** carry the enclosing-function provenance, walk lifted bodies in the instance
closure, and emit per-instance specializations (03-02).

### Gap 4: ownerless call-result projections are never bound (DEF-55)
**Current Behavior:** the projection reads inline and the result's siblings leak.
**Required Behavior:** the result is bound to an owned temporary and the projection is moved out.
**Fix Required:** materialize the call result before an owning projection read (03-03).

### Gap 5: the maybe-owning element take suppresses the caller's duty (DEF-53)
**Current Behavior:** the callee and the call temporary release the same element.
**Required Behavior:** the element is taken from a bound temporary exactly once.
**Fix Required:** bind the ownerless call result in lowering and emit the element move-out; the
checker's maybe-owning suppression stays, because checking runs before lowering (03-03).

### Gap 6: guard operands of drop-needing growable types skip the re-store drop (DEF-51)
**Current Behavior:** the previous value leaks on each re-materialization.
**Required Behavior:** the statement-form drop runs before the re-store.
**Fix Required:** hoist `emit_reassigned_temp_drop` for the `drop_expr_text == None` case in
`emit_guards` (03-04).

## Dependencies

### Internal Dependencies

- `generate` stage order: `collect_ir_instances` runs after lowering, before codegen; the lifted
  provenance must exist by then (`compiler/src/own.rs`).
- `ProgramInfo.functions` and `ownership.functions` both enumerate functions; the lifted
  specialization keying must stay consistent between them (`compiler/src/codegen/program.rs`).
- `E10026` is owned by the ownership stage (`compiler/src/own/expr.rs`); the bind-temporary fix
  must not change its reporting for concrete element takes.

### External Dependencies

None. The workspace stays std-only; no new crates, build scripts, or harnesses.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Lifted specialization changes emitted C for existing generic lambdas | Med | High | Specialize only when a lifted body or environment mentions an enclosing parameter; keep the shared emission otherwise; pin byte-identical re-emission |
| Bind-temporary changes the accepted element-move boundary (E10026) | Med | High | Keep the checker's concrete-take rejection; add a compatibility case pinning `E10026` unchanged |
| The guard hoist fires on short-circuited iterations | Low | Med | Hoist only for the `drop_expr_text == None` case; temps are zero-initialized and released after the loop, so a drop runs at most once per stored value (03-04) |
| Extent keys collide with user struct names | Low | Med | Encoding is structural and deterministic; existing sanitized-name mechanisms stay the last resort; unit tests cover multi-extent and slice keys |
| Full `make verify` cross-toolchain time | Med | Low | Run the cross-toolchain legs once at close-out (AR-7) |
