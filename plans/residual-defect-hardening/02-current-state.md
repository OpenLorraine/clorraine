# Current State: Residual Defect Hardening

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The compiler lowers an ownership-annotated, typed IR to one C99 translation unit. Fixed arrays are
inline storage; a slice is a pointer and a length; ownership decisions (drops, moves,
materialization) are made in the ownership stage and replayed by codegen. Generic bodies are
checked once with unresolved type parameters, and codegen emits one copy per recorded instance
with a type substitution installed.

The four defects are gaps in these existing mechanisms, not missing subsystems. Each was
reproduced against the committed compiler and its emitted C inspected.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/types/mod.rs` | `Ty::array` construction; `substitute`; `iterated_element` | Fold a fixed-array element into the extents in the constructor and in `substitute`, so no reachable construction path leaves a nested fixed-array type (the parser flattens written types, so `resolve_ty_with` cannot nest fixed arrays) |
| `compiler/src/sema/expr.rs` | No-context array literal inference | Emit the flat type through the folded constructor (requirement R1) |
| `compiler/src/codegen/guards.rs` | Expression-form stores and the guard comma path | Add the per-row expression store for a non-literal nested row (R2) |
| `compiler/src/codegen/function.rs` | Fixed-array store core, literal stores, array targets | Promote the store core to sibling visibility for the return path (R3) |
| `compiler/src/codegen/cleanup.rs` | Return emission | Delegate the multi-extent place return to the store core (R3) |
| `compiler/src/own/loops.rs` | Loop-condition lowering | Materialize owned call results in conditions (R4) |
| `compiler/src/ir/guards.rs` | Guard prediction | Treat a materialized condition as needing the guarded loop path (R4) |
| `compiler/src/codegen/control.rs` | Loop emission | No change expected; it already selects the guarded path from the predicate (R4) |
| `compiler/src/own/instance_check.rs` | Per-instance audit precedent | Extended to discharge the requires-duplicable conflicts (R5) |
| `compiler/src/own/{call_check,call}.rs` | Take-argument move recording and the argument move-out | Record the move-if-owning marker and empty a param-typed take place under an owning substitution (R5) |
| `compiler/src/own/{place,flow}.rs` | Read hook and flow state | Detect the later use of a moved-if-owning binding and carry the marker through joins (R5) |
| `compiler/src/ir.rs`, `compiler/src/own.rs`, `compiler/src/own/{lower,expr,statements,loops}.rs` | IR statements, binding registration, param-suppression sites | Deposit maybe-owning and move-if-owning obligations (R5) |
| `compiler/src/codegen/{function,cleanup,drop}.rs` | Drop and release emission | Already substitute types; no change expected beyond the new flag (R5) |

### Code Analysis

`Ty::array` builds `TyKind::Array { element, extents }` without folding. Context-typed literals
use a flat base element plus the full extent list (`sema/expr.rs:693-705`); the no-context path
takes the first element's checked type as the element (`sema/expr.rs:726-741`) and appends one
extent (`:756-760`), so a literal of literals becomes `array(array(i32,[2]),[2])`.
`c_type` prints only the base of a fixed array (`codegen/types.rs:52-58`) and `declarator` appends
the recorded extents (`:242-250`), so the nested inner extent disappears in C.
`iterated_element` (`types/mod.rs:341-352`) and struct instance recording
(`own/generics.rs:148`, which recurses the element) already assume a flat element. `type_key`
(`codegen/types.rs:290`) folds an array to its base element and ignores extents, so distinct
fixed-array type arguments share C symbol names (out of scope; DEF-50). Generic substitution is
the second construction path: `substitute` (`types/mod.rs:423-449`) rebuilds `TyKind::Array`
directly (`:429-436`), so `T[N]` instantiated with a fixed array is nested too and the fold must
cover it.

The expression-form store `array_assign_expr` (`codegen/guards.rs:342-420`) recurses only when the
row value is itself a `ValueKind::ArrayLit` (`:358-362`); any other row falls to the scalar store
(`:364-381`) and `store_text` renders a brace list or a conditional directly. The statement core
handles every row shape (`codegen/function.rs:1843-1916`, `:1130-1294`).

The multi-extent return arm for a place copies only the first extent in a hand-written loop
(`codegen/cleanup.rs:539-555`), while the literal branch already delegates to the store core
(`:532-537`). The core recurses extents, memcpys the bytes, zeroes an owned source, and gates on
`needs_drop` (`codegen/function.rs:970-1120`; the source move-out helper `emit_arm_source_moves`
is defined at `:1723-1752` and called at `:1093`/`:1117`).

Loop conditions are lowered at `own/loops.rs:48-49`, `:133-134`, and `:228-233`. An owned call
result reached through a member read is not materialized: `Member` lowering never calls
`materialize_guard_operand` (`own/lower.rs:602-638`), and `value_needs_guards`
(`ir/guards.rs:19-84`) reports false, so `codegen/control.rs` prints the call in the header
(`:390-396`) and no release is emitted. The guarded path already exists for conditions with a
runtime guard: it materializes into a temp, drops the previous value before re-storing it
(`codegen/guards.rs:538-593`), and releases the last value through the statement's pending-release
drain (`own.rs:965-989`).

Ownership decisions are made once, before substitution: `owns_drop` (`own.rs:720-726`) and
`duplicable` (`types/table.rs:377-409`) return false/true for a `Param`, so a param-typed binding
gets no drop and a param-typed read is copied rather than moved. The lowerer is substitution-blind
at eleven param-typed materialization/flow sites, skips the take-argument move and move-out, and
the loan-owner predicate ignores maybe-owning temporaries (the site table in `03-04`);
`instance_check` already demonstrates the per-instance re-check pattern after monomorphization
(`own/instance_check.rs`).

## Gaps Identified

### Gap 1: Nested no-context literal type and missing deferred row store (DEF-46)

**Current Behavior:** `let lit = [[1, 2], [3, 4]];` declares `i32 lit[2]` and emits rejected stores;
the deferred spelling emits assignments of an array pointer and a brace literal inside a guard
comma expression; a conditional whose branches are a flat literal and a nested one is rejected
with a nonsensical `cannot convert 'i32[2][2]' to 'i32[2][2]'`.
**Required Behavior:** R1 and R2 — the literal has the flat `i32[2][2]` type and the deferred path
stores each row through its expression form.
**Fix Required:** Fold fixed-array elements in the type constructor (keeping slice elements
nested) and add the expression-form row store.

### Gap 2: One-extent return copy (DEF-47)

**Current Behavior:** `return a` for a `String[2][2]` emits `__cl_ret[0] = a[0];` per extent; C
rejects the array assignment.
**Required Behavior:** R3 — every extent is copied and the source is emptied.
**Fix Required:** Delegate the place branch to the existing store core.

### Gap 3: Unreleased owned call in an unguarded loop condition (DEF-48)

**Current Behavior:** `while (grow().count > 0 && i < 3)` prints the call in the header; the value
is never released (ASan: 72 bytes leaked in 6 allocations).
**Required Behavior:** R4 — the condition is evaluated per iteration, the previous value is
released before a re-store, and the last value is released after the loop.
**Fix Required:** Materialize owned call results in the three loop-condition lowering points and
let the existing guard predicate select the guarded path.

### Gap 4: Substitution-blind generic ownership (DEF-49)

**Current Behavior:** a generic body with a `take` parameter of param-mentioning type emits no
release; assignments from param-typed places are copies; the lowerer is substitution-blind at
eleven materialization/flow sites and skips the take-argument move and move-out, and none is
re-applied per instance.
**Required Behavior:** R5 and R6 — owning instances release and move correctly (including take
argument places, with borrow parameters never dropped); non-owning instances are unchanged; a
reuse of a moved-if-owning value reports `E10020` at the body's use.
**Fix Required:** Deposit symbolic ownership obligations at the param-typed decision points and
discharge them per instance: maybe-owning drops and releases, move-if-owning stores and take
argument move-outs, requires-duplicable conflicts, the materializations the lowerer skipped, and
the loan-owner predicate (AR #17).

## Dependencies

### Internal Dependencies

- The flat extents convention and the sema literal checks (`03-01`).
- The fixed-array store core and the array target abstraction (`03-02`, `03-04`).
- The guard/materialize machinery and the pending-release bookkeeping (`03-03`).
- The monomorphization instance set and the per-instance emission loop (`03-04`).

### External Dependencies

None — the workspace stays std-only; no new crate, build script, or tool (AR #14).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Folding `Ty::array` changes a slice-of-fixed-array type | Medium | High | Fold only a fixed-array element (all extents present); keep slice elements nested; the ST-slice regression pins it (AR #9) |
| The expression row store duplicates the statement core | Medium | Medium | Implement one expression builder that mirrors the core's cases; impl tests pin each form's emitted shape |
| The DEF-49 pass misses a suppression site | Medium | High | The site table in `03-04` is exhaustive (14 sites, 12 rows); the site→case map in `07` covers each; the red phase proves each site fails before the fix |
| The DEF-49 move rewrite frees a value a later path uses | Low | High | The audit reports `E10020` at the body's use when the substituted type owns and the source is reused; the ST case pins the rejection |
| Per-instance bodies change emission determinism | Low | Medium | Instances are emitted in recorded order; the determinism suite and byte-identical re-emission impl test pin it |
