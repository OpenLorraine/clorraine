# Current State: Clorraine v1 language completion

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The compiler is a staged pipeline (`lex`, `parse`, `load`, `resolve`, `sema`,
`own`, `ir`, `codegen`) plus a `types` and `consts` layer. The frozen grammar is
parsed in full; the semantic stage reports staged-feature codes for the surfaces
the MVP did not implement, so the front end accepts all three target surfaces
today while the back end is incomplete.

Generic **functions** already work end to end: the type stage binds and infers
their type arguments, `own/generics.rs` records deduped instances, and codegen
emits one `static` copy per instance keyed `cl_<name>__<typekey>`. Generic
**structs that store a type parameter** do not: the C type and symbol drop the
arguments, so the field type is an unresolved `ClT`.

Lambdas parse into `ExprKind::Lambda { captures, params, body }`
(`compiler/src/ast/expr.rs:122`) and the capture syntax is frozen, but the
semantic stage reports `E90000` for a capture-less lambda and `E90002` for any
non-empty capture list (`compiler/src/sema/expr.rs:81-98`). Codegen has no
lambda case.

Volatile **overlay struct fields** already emit a C `volatile` member
(`compiler/src/codegen/program.rs:266`), and the unsafe gate for volatile
intrinsics exists (`compiler/src/own/expr.rs:130`, `compiler/src/ffi.rs:159`).
The six intrinsics themselves are neither typed nor lowered, and a `volatile let`
global loses its qualifier.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/ir/mono.rs` | `Instance`, `Monomorphization` (function instances) | Add struct instances and their dedupe key (AR-29) |
| `compiler/src/own/generics.rs` | Collects function instances and copy obligations | Collect struct instances from resolved types and literals (AR-29) |
| `compiler/src/types/mod.rs` | `Ty`, `TyKind::{Struct, Func, Param}`, `substitute`, `display` | Add a closure type kind and carry callable bounds; no change to `Struct` shape (AR-24, AR-23) |
| `compiler/src/types/table.rs` | `FuncSig { params, returns, type_params: Vec<String> }`, `TypeTable` | Carry the callable bound with each type parameter (AR-15) |
| `compiler/src/sema/expr.rs` | `check_expr`, `check_intrinsic`, `check_template` | Implement lambda checking; type the volatile intrinsics (AR-12-AR-19) |
| `compiler/src/sema/call.rs` | `check_generic_call`, `check_args` | Enforce `E10303`; infer a closure type argument (AR-15) |
| `compiler/src/sema/mod.rs` | `check`, item checks | Reject `E10306` at an aggregate instantiation; update `E10302` path (AR-16) |
| `compiler/src/sema/items.rs` | Item checks, struct literal binding | Thread the `volatile` global flag (AR-20) |
| `compiler/src/codegen/types.rs` | `c_type`, `type_key`, `declarator` | Include struct type arguments in the key and the emitted name (AR-30) |
| `compiler/src/codegen/program.rs` | `emit_struct`, `emit_globals`, prototypes | Emit one struct per instance; emit the volatile qualifier (AR-17, AR-20) |
| `compiler/src/codegen/calls.rs` | `intrinsic` | Add the six volatile cases (today `_ => "0"`) (AR-19) |
| `compiler/src/codegen/expr.rs` | Lambda expression lowering fallback | Lift and emit lambdas (AR-9-AR-14) |
| `compiler/src/ir/global.rs` | Global metadata | Carry the `volatile` flag to codegen (AR-20) |
| `compiler/src/stdlib.rs` | Embedded stdlib modules and per-profile gate | Register `std.amiga.custom` for hosted-amiga (AR-7) |
| `stdlib/std.amiga.custom.clo` | New MMIO wrapper module | Create it (AR-28) |
| `compiler/tests/compile_fail_spec_test.rs` | `ASSERTED_CODES`, `EXCLUDED_CODES` | Un-skip `E10303`/`E10304`/`E10400`; retire `E90002` (AR-21, AR-25) |
| `spec/tests/compile-fail/**` | Golden inputs with `// stage:` annotations | Remove the annotations, retire the `E90002` golden (AR-32) |

### Code Analysis

**Generic structs.** `c_type` maps every `TyKind::Struct(name, args)` to
`Cl{name}` and `type_key` returns `name` (`compiler/src/codegen/types.rs`), so
the type arguments never reach the symbol. `emit_struct` resolves each field
against the type-parameter names and writes `struct Cl{name}` once
(`compiler/src/codegen/program.rs:261-272`). A struct that stores `T` therefore
emits a `ClT` field — invalid C — while the process exits 0. The type stage
already models the instantiation correctly as `TyKind::Struct(name, args)`
(`compiler/src/types/mod.rs:53`), so the fix is in monomorphization and codegen.

**Callable bounds.** `TypeParam { bound: Option<CallableBound> }` is parsed
(`compiler/src/ast/mod.rs:149-165`) but `FuncSig` keeps only the parameter
**names** (`compiler/src/types/table.rs:26`), so the checker cannot evaluate a
bound and never emits `E10303`; `E10304` and `E10306` have no source at all.

**Lambdas.** The AST and parser are complete; only the semantic and codegen
halves are missing. Ch 17 §8 fixes the intended emission: capture-less is a
plain function pointer; capturing is a two-word callable (function pointer plus
a stack environment pointer) whose lifted function takes the environment.

**Volatile.** `check_intrinsic` falls through to `Ty::unknown()`
(`compiler/src/sema/expr.rs:599-601`), and `intrinsic` falls through to
`"0".to_owned()` (`compiler/src/codegen/calls.rs:204`). A probe emitted
`return 0;` for `unsafe { return readVolatile32(addr); }`. `emit_globals`
writes `static {ty} {symbol} = {init};` with no qualifier
(`compiler/src/codegen/program.rs:366-380`), and a probe emitted
`static u32 cl_probe_vol_ticks = 0;` for `volatile let ticks: u32 = 0;`.

## Gaps Identified

### Gap 1: Generic structs are silently miscompiled

**Current Behavior:** `struct Pair<T> { first: T; }` emits `struct ClPair { ClT
first; };` and exits 0.
**Required Behavior:** One concrete `ClPair__i32` (and so on) per storable
instantiation (R1).
**Fix Required:** Record struct instances in `Monomorphization`; include the
arguments in `type_key`/`c_type`; emit one struct per instance (AR-17, AR-29,
AR-30).

### Gap 2: Callable bounds, callable-only values, and ephemeral arguments are inert

**Current Behavior:** `E10303`, `E10304`, and `E10306` have goldens but no
source implementation; the goldens are excluded or skipped.
**Required Behavior:** All three are emitted (R2-R4).
**Fix Required:** Carry the bound into the type stage; check the inferred or
explicit argument; check storage/return of a callable-bound value; reject a
closure type argument at an aggregate (AR-15, AR-16).

### Gap 3: Lambdas are staged

**Current Behavior:** A capture-less lambda is `E90000`; a capture list is
`E90002` (`compiler/src/sema/expr.rs:81-98`).
**Required Behavior:** Stages 1-4 lower, with `E10400`-`E10404` for the
forbidden forms (R7-R10).
**Fix Required:** Lift, type, and emit lambdas; enforce the capture rules
(AR-9-AR-14).

### Gap 4: Volatile intrinsics emit `0`

**Current Behavior:** `readVolatile*`/`writeVolatile*` are untyped and lower to
`0` (`compiler/src/codegen/calls.rs:204`).
**Required Behavior:** A C `volatile`-qualified access, unsafe-gated (R11).
**Fix Required:** Type the calls and add the codegen cases (AR-19).

### Gap 5: `volatile let` globals lose the qualifier

**Current Behavior:** `emit_globals` ignores the AST `volatile` flag.
**Required Behavior:** A C `volatile` static (R12).
**Fix Required:** Thread the flag to codegen and emit it (AR-20).

### Gap 6: The MMIO wrapper module does not exist

**Current Behavior:** `stdlib/` has no `std.amiga.custom`.
**Required Behavior:** The minimal normative wrapper set exists and is
hosted-amiga only (R13).
**Fix Required:** Author the module and register it (AR-7, AR-28).

## Dependencies

### Internal Dependencies

- Function monomorphization and the instance set (`ir/mono.rs`,
  `own/generics.rs`) — reused for structs and closures.
- The type stage's `Struct(name, args)`, `Func`, `Param`, and `substitute`
  (`types/mod.rs`, `types/table.rs`).
- The codegen naming/contract table (`codegen/types.rs`) and struct/global
  emitters (`codegen/program.rs`).
- The stdlib module registry and the per-profile gate (`stdlib.rs`,
  `profile.rs:73`).
- The compile-fail suite and its excluded-code list
  (`compiler/tests/compile_fail_spec_test.rs`).

### External Dependencies

- `m68k-amigaos-gcc`, `vbcc`, `vamos`, and FS-UAE in the SDK container (via the
  hosted-amiga legs).
- No external crate, build script, or network access is added.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The closure calling convention is the largest new design and could ripple into IR and codegen | Medium | High | Keep the environment on the enclosing frame; forbid naming so the value is a temporary; land it in its own phase with spec tests first |
| Adding a `TyKind` variant touches exhaustive matches across the compiler | Medium | Medium | Add the variant early in the phase and let the compiler enumerate the match arms |
| Carrying callable bounds changes `FuncSig`'s shape and its callers | Medium | Medium | Do it in the generics phase; the type stage is the single owner (AR-15) |
| Struct-instance collection can miss a reachable instantiation | Medium | Medium | Collect from resolved types in signatures, fields, locals, and literals, and assert coverage with a nested-generic ST case (R16) |
| Volatile access is untestable on the Amiga custom chip under emulation | Medium | Low | Host test uses a real local address; the hosted-amiga leg builds the wrapper and runs only the safe path (AR-27) |
| A spec edit could drift from the frozen goldens | Low | High | Every edit is listed in `03-04-spec-goldens.md`; `./spec/check.sh` must stay green (AR-3, AR-32) |
