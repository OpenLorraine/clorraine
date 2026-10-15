# Current State: Residual Major Closeout

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The planning baseline is head `80433a6` (the batch-J close-out commit). Batches F–J built the
machinery each row extends; the line numbers below are that tree and the executor re-resolves
them before editing.

- **Initializer lowering and release** (`compiler/src/codegen/program.rs`): `lower_global`'s
  call arms render `callee: callee_name(callee).unwrap_or_default()` (:2394, :2418) with
  `callee_name` (:2938) returning `None` for an expression callee; the inferred-type-argument
  path is not consulted. `emit_init_functions` (:961) binds init locals through
  `__cl_itmp{N}` (:2217), pushes owners (:1476), and releases every owner at the init
  function's end after the guard blocks (:978-984) — the guarded `&&` bind's release lands
  outside its declaration block.
- **Monomorphization walk** (`compiler/src/own/generics.rs`): `collect` (:30) walks function
  bodies and global initializers; its header comment (:27) states the rule "only explicit type
  arguments are recorded", and `record_call` (:378) returns early for `type_args.is_empty()`
  (:383). The body path's inference helpers exist beside it: `infer_call_types`
  (`own/call.rs:421`) plus `sema::call::match_param` (`sema/call.rs:881`).
- **Element placeholder and substitution** (`compiler/src/own/loops.rs:531`,
  `compiler/src/codegen.rs:749`, `compiler/src/types/mod.rs:700`, `compiler/src/types/keys.rs:112`):
  the ownership stage writes the element as `TyKind::Param(element_param(name))`
  (`#element of T`), the codegen instance substitution maps it to `arg.iterated_element()`
  (`codegen.rs:749-752`), and the key/view-element derivation uses `iterated_element()`
  (`keys.rs:112`). The type stage's loop check (`sema/stmt.rs:210-218`, `check_for_in`) binds
  `iterable.iterated_element()`, which is `unknown` for a bare `Param(T)`; the opaque member
  read is recorded (`sema/expr.rs:587-596`) and the per-instance audit resolves it
  (`own/instance_check.rs:946-991`), but the inferred call argument's instance key derives
  before the element mapping applies.
- **Bounds guards** (`compiler/src/sema/expr.rs:604-628`, `compiler/src/codegen/guards.rs:1157-1204`):
  `check_index` folds a constant index against a fixed-array extent (`E10216`) and returns the
  row type; `bounds_guard_condition` (:1173) covers fixed arrays, slices, and `str` views — an
  opaque (`Param`) base produces no condition, so a substituted row element's index gets no
  guard.
- **Member dispatch** (`compiler/src/sema/expr.rs`): `member_type` (:553) types struct fields,
  enum members (:569-579), raw fields, and opaque projections; every other base — views
  included — falls to `Ty::unknown()` (:598). The qualified enum/type path is handled before
  it (`check_qualified` :485-498). `own/expr.rs::member_type` (:784) mirrors the struct/raw
  cases only. The instance audit's `projection_resolves` (`own/instance_check.rs:1000-1011`)
  mirrors the value-form checker, enum members included (:1003).
- **Pointer-member conversion** (`compiler/src/codegen/function.rs:2046-2078`,
  `names_pointer_member` :2647-2659): a `cstring` destination recognizes a `.ptr` member by
  shape and spells `(char *)(...)`, because the member carries no checked type.
- **Call conversions** (`compiler/src/sema/call.rs`): `check_ffi_arguments` (:719) and the
  direct-call checks skip conversion judgments while a type mentions a parameter
  (`contains_param` at :741/:765; `sema/expr.rs:345`); the `ffi` call branch types every
  `ffi` result as `Ty::unknown()` (:76-83). The per-instance duty pattern lives in
  `own/instance_check.rs` (`check_iterable_requirements` :897, the storable-slot and out-slot
  duties).
- **Null-shape rendering** (`compiler/src/codegen/expr.rs:344-345`, `compiler/src/types/mod.rs:688`,
  `codegen/types.rs:315`): `is_null_shaped` recognizes the unresolved nullable shape and
  `null_value` renders the destination's null where the destination is known; at a nullable
  view/handle destination the store/argument/return render the shape's own `void *`.
- **Lend/capture store judgment** (`compiler/src/own/expr.rs:1881-1899`): an out-slot target
  returns before the lend/capture judgment (the carve-out); `is_caller_slot` (:1578) treats an
  `EnvField` base as caller storage; the capture move rule lives in `sema/expr.rs:265-300`
  (`E10401`) with the per-instance audit in `own/instance_check.rs:65-115`.
- **Standalone sites**: the func-global resolution requires `id.module == current_module`
  (`codegen/expr.rs:778`, `codegen/function.rs:953`); the `sizeof` intrinsic renders `c_type`
  of the operand type (`codegen/calls.rs:343-344`) and `c_type` folds a fixed array to its
  element (`codegen/types.rs:98-103`); `lift_lambda`'s expression arm (`own/lower.rs:954`),
  the intrinsic renderer (`codegen/calls.rs:275`), and `emit_return`
  (`codegen/cleanup.rs:373`) carry the void-lambda shape; the freestanding `_start` calls
  `main()` without realigning the stack (`compiler/runtime/shim_freestanding.c:304-307`).

### Relevant Files

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `compiler/src/codegen/program.rs` | Initializer lowering, init-owner release, guarded binds | The expression callee, the inferred instance recording, the release scope |
| `compiler/src/own/generics.rs` | Monomorphization walk | The initializer inferred-type-argument rule |
| `compiler/src/sema/stmt.rs`, `own/loops.rs`, `codegen.rs`, `types/keys.rs` | Element placeholder, substitution, key derivation | The type-stage element binding and its mapping |
| `compiler/src/sema/expr.rs` | Member dispatch, bounds check | The enum value form, the view members, the deferred index guard |
| `compiler/src/own/expr.rs`, `own/instance_check.rs` | Member typing, audit, duties, store judgment | The projection audit, the conversion duty, the capture slot store |
| `compiler/src/sema/call.rs`, `own/call.rs`, `own/call_check.rs` | Call checks and inference | The conversion duty recording; the `ffi` return type |
| `compiler/src/codegen/calls.rs`, `codegen/expr.rs`, `codegen/guards.rs`, `codegen/function.rs` | Intrinsics, conditionals, guards, stores | `sizeof`, the view arm, the guard condition, the pointer-member conversion |
| `compiler/src/own/lower.rs`, `compiler/src/codegen/cleanup.rs` | Lambda lift, return emission | The void intrinsic expression body |
| `compiler/runtime/shim_freestanding.c` | Freestanding entry | The `_start` stack alignment |
| The origin suites (`module_initializer_integrity_*`, `checker_conformance_*`, `callable_release_integrity_*`, `def_backlog_closeout_*`, `freestanding_spec_test.rs`) | Regression oracles | Additions only (AR-9) |

## Gaps Identified

Each gap names the row, the observed behavior at the head, the required behavior, and the site.
The roadmap rows carry the reproductions; AR-3 carries the probe results; this table is the site
map.

### P1 — Initializer call resolution, guards, and view conditionals

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-196` | `let h: i32 = mk()(5);` and `(c ? f : g)(5)` emit `h = (5);` (the call collapses to its argument; run exit 255) | The callee expression resolves like the body path (a call-result callee becomes the call's callable value) and the result stores | `codegen/program.rs` the call arms (:2394, :2418), `callee_name` (:2938) |
| `DEF-197` | `let g: i32 = pick(1, 2);` emits `cl_..._pick(...)` with no instance (strict build fails) | The inferred type arguments are derived with the body path's inference and the instance records/keyed callee emits | `own/generics.rs` `collect`/`record_call` (:30, :378-395); `own/call.rs::infer_call_types` (:421); `codegen/program.rs` |
| `DEF-207` | The null arm reaches `cl_string_view(NULL)` and `cl_string_ptr` dereferences it (run segfault 139; body and initializer) | Each arm builds its own view; the null arm carries the zero view, so the destination's nullness holds | `codegen/expr.rs:472`, `codegen/function.rs:2069-2093` (`widen_text_view`), `codegen/program.rs:1058` |
| `DEF-219` | `let hit: bool = yes && (...);` declares `__cl_itmp1` inside the `if` block and releases it after it (strict build fails) | The owned bind's release stays in the block that declares it (or the bind is declared outside the decision) | `codegen/program.rs` `emit_init_functions` (:961-984), the guarded bind (:1465-1480) |
| `DEF-220` | `let e: i32 = viewOf(slice(xs, 0, 4))[1];` emits the plain `viewOf` symbol (strict build fails) | The inferred instance records like `DEF-197`, keyed on the view-builder result | same as `DEF-197` |
| `DEF-209` | Fixed at the head: the root binds (`__cl_owned0 = mkBox()`) and releases its owned member; ASan-clean | Keep the shape (the pinned control) | `own/lower.rs` root binders (already handling the shape) |

### P2 — Element placeholder, inference, and bounds

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-229` | `id(v.y)` inside `each<S[2]>` keys `cl_id__unknown` (strict build fails); the direct read is green | The element binds in the type stage and its member read maps through the instance substitution | `sema/stmt.rs:210-218`, `own/loops.rs:531`, `codegen.rs:749-752`, `types/keys.rs:112`, `own/instance_check.rs` |
| `DEF-230` | `[v] () => v.y` declares the env field `i32 v` for a `ClS` element (strict build fails) | The capture's field type comes from the element's checked type | same as `DEF-229` plus the capture declaration path |
| `DEF-231` | `inner(v)` inside `outer<i32[2]>` keys `cl_inner__unknown`; `each(x)` with `x = null` keys `cl_each__unknownQ` (strict builds fail) | The placeholder and the null shape resolve before the instance key derives | `codegen.rs:743-752`, `types/keys.rs:105-120`, the inference/key path |
| `DEF-232` | `row[5]`/`row[i]` inside `rowsum<i32[2][3]>` emit no guard and run to completion (exit 1); the concrete twin reports `E10216` | The constant index reports `E10216` at the instance; a run-time index emits the `E11211` trap on the substituted row | `sema/expr.rs:604-628`, `codegen/guards.rs:1157-1204` |

### P3 — Member typing

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-233` | `e.A` on an enum value is accepted on both paths and the strict build fails (`request for member 'A'`) | Reject with `E10212` on the concrete and instance paths; the scoped `E.A` stays accepted (AR-6) | `sema/expr.rs:569-579`, `own/instance_check.rs:1003` |
| `DEF-238` | `return s.ptr;` into `usize` is accepted and fails `-Werror=int-conversion`; `.len` builds | `.ptr` carries the view pointer type, `.len` is `usize`; the conversions judge, the `cstring` destination keeps its pointer-member conversion | `sema/expr.rs:598`, `own/expr.rs:784`, `codegen/function.rs:2046-2078`/`:2647-2659` |

### P4 — Call conversions and `ffi` result typing

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-242` | `Open(x, 0)` at `call<str>` passes `ClStr` where `char *` is declared; the `cstring` twin, `return x` at `make<i32>()`, and `let y: T = x` at `f<i32>()` fail the strict build | The conversion duty records at the parameter-typed sites and reports `E10200` at the offending instance; legal instances stay accepted (AR-8) | `sema/call.rs:719-765`, `own/call_check.rs`, `own/instance_check.rs` |
| `DEF-243` | `Uses(Count())` fails `pointer from integer` (the `ffi` result is typed unknown) | The `ffi` call's type is the declaration's resolved return type | `sema/call.rs:76-83`, `ffi_signature` (:683) |

### P5 — Null destinations and capture slot stores

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-245` | `let s: str? = x;` / `show(x)` / `let fl: File? = x;` are accepted and fail the strict build (`void *` into the view/`BPTR`) | The null shape lowers to the destination's null value (the zero view / `NULL`) per Ch 17; the stop condition is AR-13 | `codegen/expr.rs:344-345`, `types/mod.rs:688`, `codegen/types.rs:315` |
| `DEF-251` | `([borrow x, borrow out] () => { out = x; })()` is accepted and double-frees (ASan `attempting double-free`) | The out-slot carve-out does not exempt a capture source; the capture move is judged at the slot store | `own/expr.rs:1881-1899`, `sema/expr.rs:265-300`, `own/instance_check.rs:65-115` |

### P6 — Standalone emission shapes

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-170` | `import { g } from lib; g(5)` emits `g(...)`; `lib.g(5)` the same; `h.call(6)` emits `h` (strict build fails) | The func global's identity resolves for every spelling; the struct global's field call spells its symbol | `codegen/expr.rs:773-782`, `codegen/function.rs:953`, `codegen/program.rs` `global_symbol` |
| `DEF-189` | The freestanding host no-libc run segfaults 139 at `main`'s 16-byte store; the m68k link is clean | `_start` aligns the stack before calling `main` | `compiler/runtime/shim_freestanding.c:304-307` |
| `DEF-211` | `sizeof(i32[2])` emits `sizeof(i32)` (4 instead of 8) | The fixed-array operand measures the whole type (AR-7) | `codegen/calls.rs:343-344`, `codegen/types.rs:98-103` |
| `DEF-255` | `() => assert(false, "nope")` emits `return 0;` in a void function (strict build fails; the abort is dropped) | A void intrinsic statement in the expression body emits the abort, not a value return | `own/lower.rs:954`, `codegen/calls.rs:275`, `codegen/cleanup.rs:373` |

## Dependencies

### Internal Dependencies

- The origin suites (`module_initializer_integrity_{spec,impl}_test.rs`,
  `checker_conformance_{spec,impl}_test.rs`, `callable_release_integrity_{spec,impl}_test.rs`,
  `def_backlog_closeout_{spec,impl}_test.rs`, `freestanding_spec_test.rs`) are the regression
  oracles; the new cases append to them (AR-9).
- The batch-H/I/J suites and the `callable_release_integrity` / `checker_conformance` oracles
  gate the phases that touch reports, emissions, and the element surface.
- `make check` (the container) and `make verify`'s Amiga legs gate every phase's green state.

### External Dependencies

- None — the workspace stays std-only.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| The `DEF-242` duty over-rejects a legally converting instance | Med | High | The phase measures the legal instances first and stops to park the row if the duty cannot be bounded (AR-8) |
| The `DEF-245` typed-null lowering breaks a pinned null-shape acceptance | Med | Med | The comparisons, the copies, and the literal stay pinned; the phase stops and parks on a conflict (AR-13) |
| The `DEF-229`/`231` type-stage element binding changes drop decisions for element reads | Med | High | The memory lens probes the element reads and captures on both paths; the origin suites re-run |
| The `DEF-233` rejection invalidates a pinned opaque-projection acceptance | Low | Med | The phase measures the existing pins before the change; a legitimate scoped spelling keeps a control |
| The `DEF-211` emission change alters non-array intrinsic bytes | Low | Low | The impl pin checks the scalar/struct spellings byte-identical |
| The freestanding shim change affects the m68k entry ABI | Low | High | The freestanding m68k link and the FS-UAE-less host legs both run; the shim change is stack-only |
