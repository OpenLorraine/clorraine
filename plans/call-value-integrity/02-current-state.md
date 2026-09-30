# Current State: Call Value Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The runtime-value-integrity plan built the machinery this plan extends:

- A **branch-store rule** for fixed-array conditionals: `emit_fixed_array_conditional_assign`
  (`compiler/src/codegen/function.rs:825`) and `emit_fixed_array_assign_arm` (`:877`), with a
  return-position twin `emit_fixed_array_return_arm` (`compiler/src/codegen/cleanup.rs:358`).
- **Materialization helpers** in `compiler/src/own/lower.rs`: `materialize_guard_operand` (`:579`),
  `materialize_value_type` (`:888`), `materialize_read_conditional` (`:874`),
  `materialize_owned_argument` (`:950`), and the forwarding rule `projects_from_place` (`:696`).
- A **hidden-destination call ABI** for by-value fixed-array arguments and returns:
  `emit_fixed_array_call_assign` (`compiler/src/codegen/function.rs:973`), `call_with_destination`
  (`compiler/src/codegen/expr.rs:399`), and the destination parameter in
  `func_pointer_signature` (`compiler/src/codegen/types.rs:78`).
- A **pending-release set** for destination temporaries: `mark_destination_temp`
  (`compiler/src/own.rs:943`), `register_pending_release` (`:963`), `drain_pending_releases`
  (`:979`), and the `Statement::ReleaseTemp` emission (`compiler/src/codegen/function.rs:423`).
- A **shared func-pointer declarator**: `func_pointer_declarator` (`compiler/src/codegen/types.rs:66`),
  reached through `declarator` (`:170`), used by the DEF-27 fix for callable fields.
- **Callable-field receiver guards** on the call paths (`compiler/src/codegen/guards.rs`).
- A committed two-argument `cl_panic(pointer, length)` runtime boundary in all three shims.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/statements.rs` | Store preparation, conditional move effect, discards | Owned conditional branch-store rule (03-01) |
| `compiler/src/own/expr.rs` | `conditional_move_sources`, indexed store targets | Accept call arms; element stores (03-01, 03-02) |
| `compiler/src/own/lower.rs` | Conditional lowering, materialization, transfer temps | Duplicable materialization gate; transfer registration (03-01, 03-02) |
| `compiler/src/own.rs` | Release bookkeeping, destination temps | Transfer temps in the release set (03-01) |
| `compiler/src/own/call.rs` | Callee resolution for calls | Captures and globals by name (03-03) |
| `compiler/src/own/loops.rs` | Do-while lowering | Condition block entry (03-04) |
| `compiler/src/codegen/function.rs` | Statement emission, assign, stores, calls | Owned branch store; zero walk (03-01, 03-04) |
| `compiler/src/codegen/expr.rs` | Value rendering, projections, call spellings | Projections; callee spelling (03-02, 03-03) |
| `compiler/src/codegen/types.rs` | Declarators, func-pointer signatures | Reuse in remaining positions (03-03) |
| `compiler/src/codegen/guards.rs` | Statement/embedded guard spellings | Growable drop expression; do-while (03-04) |
| `compiler/src/codegen/drop.rs` | Drop emission | Growable drop expression form (03-04) |
| `compiler/src/codegen/calls.rs` | Argument widening and modes | Func-value expected types (03-03) |
| `compiler/src/types/{mod,table}.rs` | `FuncTy`, resolution | Parameter modes (03-03) |
| `compiler/src/codegen.rs`, `program.rs`, `cleanup.rs` | Declarations, closure structs, cleanup | Declarator reuse; array parameter zeroing (03-03, 03-04) |

### Code Analysis

Two structural facts dominate the defects:

1. **The duplicable skip.** Lowering materializes an owned call argument only when the value is
   non-duplicable (`materialize_owned_argument`, `compiler/src/own/lower.rs:950-953`). A duplicable
   fixed array (`i32[2]`) therefore reaches C as a value expression in positions that need a place
   (conditional arms, nested literals, projections), and C rejects the emission.
2. **The forwarding rule treats every `Materialize` as place-rooted.**
   `projects_from_place` (`compiler/src/own/lower.rs:696-717`) returns true for `Materialize`,
   including a deferred take-call transfer temporary (`compiler/src/own/lower.rs:99-118`), and no
   pending release is registered for it. The scalar conditional then copies a handle without moving
   it, and the transfer temporary is never released.

## Gaps Identified

### Gap 1: Scalar owned conditionals (DEF-34)

**Current Behavior:** `let c: String = ok ? a : b;` emits `c = (ok ? a : b);` with no move of the
chosen arm; `c` and both arms free the same storage (ASan double-free). The call-arm form
`let c: String = ok ? mk() : b;` has the same shape, grounded by a host probe on the committed
compiler.
**Required Behavior:** the chosen arm moves into the destination per path (R1, AR #2, AR #19).
**Fix Required:** scalar branch store plus call arms — see [03-01](03-01-scalar-conditional-ownership.md).

### Gap 2: Deferred take-call transfer temporary (DEF-45)

**Current Behavior:** `ok ? sink(s) : t;` materializes `sink(s)` into a transfer temporary that
owns the result and is never released; a later binding copies the handle without zeroing the
temporary, so a blind release would double-free.
**Required Behavior:** discard releases exactly once; adoption transfers ownership (R2, AR #2).
**Fix Required:** register transfer temps for release and move out on adoption — see
[03-01](03-01-scalar-conditional-ownership.md).

### Gap 3: Element arms (spec pin)

**Current Behavior:** `let c: String = ok ? a[0] : a[1];` reports `E10026` today; the rejection is
spec-correct (`spec/03-ownership.md:126-128`); the filing's "same shape" is potential, not an active
miscompile.
**Required Behavior:** keep `E10026`; duplicable element arms copy and run (R3, AR #3).
**Fix Required:** regression pins only — see [03-01](03-01-scalar-conditional-ownership.md).

### Gap 4: Indexed fixed-array element stores (DEF-31)

**Current Behavior:** `rows[i] = value;` on `i32[2][2]` routes through
`Statement::StorePointer` (`compiler/src/codegen/function.rs:390-399`) and emits
`*(&(rows[i])) = a;`; the scalar branch is chosen because the place type for `rows[i]` resolves to
the base scalar (`compiler/src/own.rs:774-776`), so `emit_fixed_array_assign`
(`compiler/src/codegen/function.rs:760`) is not reached. The constant-index sibling
`rows[0] = a;` emits `rows[0] = a;`, which C rejects for an array element type.
**Required Behavior:** element stores byte-copy or store element-wise with valid C (R4, AR #13).
**Fix Required:** route element stores and fix the projected element type — see
[03-02](03-02-fixed-array-value-positions.md).

### Gap 5: Conditional and nested fixed-array arguments (DEF-32, DEF-33)

**Current Behavior:** `eat(ok ? mkA() : mkB())` emits `eat((ok ? mkA() : mkB()))`, but `mkA` is
emitted as `void mkA(i32 *__cl_ret)` (hidden destination); the call results name no destination.
`eat(ok ? [1, 2] : [3, 4])` emits `(ok ? { 1, 2 } : { 3, 4 })`. Non-duplicable `String[2]` already
works through `materialize_owned_argument` and the fixed-array call assign; the gap is the
duplicable skip and the literal arms.
**Required Behavior:** the chosen value materializes per path for both duplicable and non-duplicable
element types (R5, R6, AR #11).
**Fix Required:** materialize duplicable fixed-array values where a place is required — see
[03-02](03-02-fixed-array-value-positions.md).

### Gap 6: Array-literal projections (DEF-35)

**Current Behavior:** `[a, b][0]` in value position emits `cl_string_view({ a, b }[0])` and in
argument position `__cl_owned0 = { a, b }[0];`; C rejects indexing a compound literal that has no
place in these positions.
**Required Behavior:** the literal materializes and the element read uses a valid place (R7).
**Fix Required:** materialize at the projection and argument positions — see
[03-02](03-02-fixed-array-value-positions.md).

### Gap 7: Struct-literal array fields (DEF-38)

**Current Behavior:** `Box { arr: ok ? a : b, tag: 1 }` emits `(ClBox){ (ok ? a : b), 1 }`; C cannot
copy an array into an aggregate initializer from an expression.
**Required Behavior:** the array field initializes through a store (R8, AR #7).
**Fix Required:** zero-init plus per-field store — see [03-02](03-02-fixed-array-value-positions.md).

### Gap 8: Func-pointer declarator reuse (DEF-30, DEF-43)

**Current Behavior:** `func(): func(i32): i32` emits an unnamed pointer prefix
(`compiler/src/codegen.rs:401-428`); closure struct `fn` fields and lifted lambda signatures build
their types inline with `c_type` (`compiler/src/codegen/program.rs:267-281`,
`compiler/src/codegen/function.rs:198-219`), so a fixed-array return loses the hidden destination.
**Required Behavior:** every position uses the shared declarator, including the environment-prefixed
lifted-lambda signature (R9).
**Fix Required:** declarator reuse — see [03-03](03-03-func-pointers-and-callable-resolution.md).

### Gap 9: Callable resolution through captures and globals (DEF-40, DEF-41)

**Current Behavior:** a call by name inside a lambda body renders the source name
(`compiler/src/own/call.rs:148-158`, `compiler/src/codegen/expr.rs:431-448`), and a module-level
func-typed global is not consulted in either lowering or emission.
**Required Behavior:** captures resolve to the environment field; globals to their symbol (R10).
**Fix Required:** resolution in lowering and emission — see
[03-03](03-03-func-pointers-and-callable-resolution.md).

### Gap 10: Func-value argument widening (DEF-42)

**Current Behavior:** widening keys off declared function signatures
(`compiler/src/codegen/calls.rs:129-189`), so a func-typed parameter receives `ClString` where
`ClStr` is expected.
**Required Behavior:** expected types and modes come from the callee's function type (R11, AR #12).
**Fix Required:** resolve from `TyKind::Func` — see [03-03](03-03-func-pointers-and-callable-resolution.md).

### Gap 11: Borrow mode in func types (DEF-44)

**Current Behavior:** the parser accepts `borrow` in a `function_type_param`
(`compiler/src/parse/types.rs:166-192`; grammar line 201) and resolution drops it
(`compiler/src/types/table.rs:558-564`); `FuncTy` has no mode
(`compiler/src/types/mod.rs:78-85`), so the pointer spells the handle by value.
**Required Behavior:** `FuncTy` carries the mode; declarator and call site pass the pointer (R12,
AR #4, AR #14).
**Fix Required:** parameter-mode data model — see [03-03](03-03-func-pointers-and-callable-resolution.md).

### Gap 12: Array-parameter zeroing (DEF-36)

**Current Behavior:** `func f(take a: String[2])` with `let b = a;` emits
`memcpy(&(b), &(a), sizeof(b)); cl_zero(&a);` on the C-adjusted pointer parameter `a`; the copy
over-reads and the later cleanup dereferences NULL.
**Required Behavior:** by-reference array parameters zero their elements; locals keep `cl_zero`
(R13, AR #10).
**Fix Required:** element-walk zeroing — see [03-04](03-04-guards-and-cleanup.md).

### Gap 13: Do-while guarded conditions (DEF-37)

**Current Behavior:** `do { i = i + 1; } while (vlen(mk()[0]) == 2 && i < 3);` emits the body's
fall-through as `continue;` before the condition block's guard/store, so the test reads the initial
empty temporary (`compiler/src/codegen/control.rs:362-375`).
**Required Behavior:** the condition block evaluates after every iteration, guards included (R14,
AR #8).
**Fix Required:** a goto-free flag-first loop when the condition carries guards — see
[03-04](03-04-guards-and-cleanup.md).

### Gap 14: Growable drops in embedded guards (DEF-39)

**Current Behavior:** `while (i < 3 && mk()[0].count > 0)` with `mk(): Array<i32>[2]` re-materializes
`__cl_tmp0` without dropping the old value; `drop_expr_text` has no growable-array form
(`compiler/src/codegen/drop.rs:139-160`), unlike the statement spelling which drops correctly.
**Required Behavior:** the embedded spelling drops before re-materializing (R15, AR #9).
**Fix Required:** growable drop expression form — see [03-04](03-04-guards-and-cleanup.md).

## Dependencies

### Internal Dependencies

- The runtime-value-integrity machinery listed under "What Exists" (all committed at `15cf339`).
- The frozen specification `spec/` as the test oracle; no spec edits.
- The pinned SDK container for `make check` and `make verify`.

### External Dependencies

- None added. The plan adds no crate, tool, or service; the workspace stays std-only.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Owned branch store interacts with move-state joins | Medium | High | The owned rule mirrors the proven fixed-array branch store; keep the conservative join (any-path move counts) |
| Transfer-aware release regresses the deferred-take cases | Medium | High | Update the old pins deliberately (AR #15); run the deferred-take ST cases and ASan |
| Func-type mode touches shared type resolution | Medium | High | The change reaches `FuncSig::ty`, type equality, and every `TyKind::Func` consumer (AR #4, #14); task 3.2.4 lists them, and a mode mismatch stays `E10210` |
| Duplicable materialization changes emitted shape for passing tests | Low | Medium | New materialization only where a C place is required; run the full suite per phase |
| A defect's spec test passes before implementation | Low | Low | Red-phase task records any pre-pass with justification (07-testing-strategy) |
