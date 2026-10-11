# Current State: Checker Conformance

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)
> **Basis**: every claim below was probed or read at the batch baseline `ddc42f0` (the batch F
> close-out head); the probe programs are cited in the rows and their emitted C was inspected.

## Overview

The batch's defects live in three stages — the semantic checker (`compiler/src/sema/`), the
ownership/lowering stage (`compiler/src/own/`), and the C emitter (`compiler/src/codegen/`) —
plus the embedded standard library. This document maps each defect to its verified present
behavior.

## Verified present behavior (probes at `ddc42f0`)

| Defect | Probe | Observed at head | Verdict |
|--------|-------|------------------|---------|
| DEF-136 | `let g: i32 = d() / 1;` (module initializer) | guard renders `cl_probe_d136i_d()` and the store renders it again — **two calls** | confirmed (initializer path) |
| DEF-136 | `func demo(s: i32[]): i32 { return v(s)[1]; }` (body) | `__cl_tmp0 = v(s);` once; guard and access read the temp | body already single — the phase pins the parity |
| DEF-136 | `let e: i32 = v(sl)[1];` with `v: (i32[]) -> i32[]` (module initializer) | guard and access call `v` twice | confirmed (initializer path) |
| DEF-142 | `len(slice(mkS(), 0, 2))` with a `String[2]` source | emits `ClSlice_String` with **no typedef**; strict build: `'ClSlice_String' undeclared` | confirmed |
| DEF-144 | `RawC(s)` with `s: str` and `func RawC(s: cstring)` | accepted; strict build: `incompatible type for argument 1 of 'RawC'` | confirmed |
| DEF-144 | `C(v)` with `v: u8[]` and `func C(s: cstring)` | accepted; strict build: `incompatible type for argument 1 of 'C'` | confirmed |
| DEF-144 | `RawC(stringOf("ab"))` (a `String` handle) | accepted; the call site typechecks (only the extern's definition is missing) | the accepted shapes to preserve |
| DEF-145 | `func each<T>(xs: T) { for (v in xs) { … } }` | accepted at declaration; instances emit invalid C (struct-element array binds a raw pointer) | confirmed by the row's review reproductions |
| DEF-146 | `let y = x.foo;` with `x: T` | accepted; the instance emits `x.foo` | confirmed by the row's reproduction |
| DEF-147 | `x.addOne(5)` with `func addOne(x: T)` | accepted; emits `addOne(x, 5)`; strict build: `too many arguments` | confirmed |
| DEF-148 | `s.foo(5)` (`foo` not a member), raw twin `p.foo(5)` | accepted; emits an implicit `foo(...)`; strict build: `implicit declaration of function 'foo'` | confirmed |
| DEF-149 | `let x = null; let s: str = "ab"; x = s;` | accepted; strict build: `incompatible types when assigning to type 'void *' from type 'ClStr'` | confirmed |
| DEF-150 | `func cond<T>(b: T, flag: bool): usize { return len(flag ? b.s : b.s); }` at a `B { s: i32[7] }` instance | the instance emits `return 0;` where 7 is expected | confirmed |
| DEF-151 | `if (x != null) { return x; }` with `x` inferred from `null`, in an `i32` function | accepted; strict build: `returning 'void *' from a function with return type 'i32'` | confirmed |
| DEF-153 | a value update on a moved field / an uninitialized element | 3× `E10020` / 2× `E10021` at one site | confirmed by the row's reviews |
| DEF-154 | `toCString(view)` | `E10200` (the signature takes `String` only) | confirmed by the row; the appendix documents `String` or `str` |
| DEF-160 | `let y: T = x;` with `x` a listed by-value capture | `E10034` (the lend duty) where L-3 assigns `E10401` | confirmed by the row's review |
| DEF-161 | `([x] () => { return Holder<T> { v: x }; })()` | `E10034` **and** `E10401` at one site | confirmed by the row's review |
| DEF-162 | an escaped view whose owner is generated | `borrow of '__cl_owned0' does not live long enough` | confirmed by the row's review |
| DEF-213 | `f(1, 2)` with `func f(a: i32)` | accepted; emits `f(1, 2)`; strict build: `too many arguments to function 'cl_probe_ar_f'; expected 1, have 2` | confirmed (plan-created row) |
| DEF-215 | `func makeSize<T>(): func(): usize { return () => sizeof(T[]); }` | the specialization emits `sizeof(ClSlice_i32)` with no typedef | confirmed by the batch F emission audit |
| DEF-216 | `sizeOf<Wrap<i32>>()` | the typedef names an undeclared `ClWrap__i32` | confirmed by the batch F emission audit |
| the null-shaped iterable | `let x = null; for (v in x) { }` | accepted; strict build: `comparison of unsigned expression in '< 0' is always false` | confirmed (the DEF-145 sub-case) |

## Code map

### Semantic checker (`compiler/src/sema/`)

| Area | Location | Present behavior |
|------|----------|------------------|
| Null literal typing | `expr.rs:51` | `null` types as `TyKind::Unknown` with `nullable: true` — the null shape |
| Implicit conversions | `types/mod.rs:595` | `StaticStr`→`str`, `String`→`str`, scalar widening, slice view identity; an unknown on either side passes (the escape hatch that lets the null shape through) |
| Member typing | `expr.rs:538-560` (`member_type`) | struct and enum members report `E10212`; a raw base resolves fields or unknown; every other base falls through to `Ty::unknown()` silently — the DEF-146 gap |
| Dot-call resolution | `call.rs:389-485` (`check_dot_call`), `487-517` (`dot_call`) | a field wins; a free-function candidate splits the receiver off and checks the remaining arguments (`check_args_impl`), but an arity mismatch is deliberately left alone (`call.rs:536-538`) and the `None` arm reports `E10212` only for a bare type-parameter field |
| Argument checking | `call.rs:575-605` (`check_args_impl`) | equal lengths check each argument against its parameter; unequal lengths check the argument values only — no arity diagnostic (DEF-147/DEF-213) |
| CString arguments | `call.rs:777-790` (`is_cstring_literal_argument`) | a `static str` literal at a `cstring` parameter is accepted; nothing rejects a view argument (DEF-144) |
| FFI crossing model | `ffi.rs:79-104` (`is_uncontracted*`) | ownership crossing applies to handles, structs, and fixed arrays; a slice stays acceptable as a view |
| Iterable declaration check | `own/loops.rs:470-500` (`check_for_in`) | accepts array, slice, `Param`, and `Unknown`; an opaque `Param`/`Unknown` keeps the accepted path "judged at the instance", but no instance audit exists (DEF-145) |
| Capture-move recording | `expr.rs:293-345` (`record_generic_capture_move`), call sites `expr.rs:214`, `stmt.rs:60`, `call.rs:268`, `call.rs:585` | records by-value capture moves at the return and `take`-argument sites only (DEF-160/161) |
| Sema duty output | `mod.rs:71` (`capture_moves`) | the per-instance audit inputs; there is no iterable duty list yet |

### Ownership stage (`compiler/src/own/`)

| Area | Location | Present behavior |
|------|----------|------------------|
| State reporters | `place.rs:20-76` (`read_place`), `place.rs:40-68` (write side) | report on every call; the update path reaches them more than once per site (DEF-153) |
| Update checking | `expr.rs:907-965` (`check_update`), `statements.rs:157-170` (`check_expr_statement`) | the statement arm evaluates the operand, then `check_update` re-reads and writes — the triple report |
| Capture lend duty | `place.rs:470-495` (`mark_moved_if_owning_at`) | skips the lend duty for a by-value capture **that escapes** (`escape && is_by_value_capture_base(...)`) and documents the non-escape gap |
| Escape skip predicate | `place.rs:1214` (`is_by_value_capture_base`), `flow.rs:74` (`generated`) | the machinery the fixes extend |
| Structural `len` fold | `lower.rs:1273`, `1759`, `1846`, `codegen/expr.rs:44` | folds through `base.ty.is_fixed_array()` and the substituted member chains (`resolved_value_ty`, `codegen/expr.rs:280`), but a conditional base stays unresolved → the emitter's unknown path yields `0` |
| Substituted projection | `codegen/expr.rs:280-310` (`resolved_value_ty`) | descends `Member` and `Index` value chains; no `Conditional` arm (DEF-150) |
| E10030 subject | `liveness.rs:325-340`, `575` (`display`) | names the owner binding, including a generated `__cl_ownedN` (DEF-162) |
| Instance audit | `own/instance_check.rs:40-56` (the four checks) | the per-instance pattern the DEF-145 fix mirrors |

### Emitter and standard library

| Area | Location | Present behavior |
|------|----------|------------------|
| Initializer materialization | `codegen/program.rs:1428` (the doc's operand list), `1872` | an impure index, divisor, or cast operand stores once into an init temporary; the dividend and the index base are missing (DEF-136) |
| Slice typedef collection | `codegen/program.rs:212-314` (`slice_element_types`, the batch F body walk) | collects locals, temporaries, signatures, instance substitutions, and written `Intrinsic.type_arg`s; a value's own slice result type, lifted specializations, and generic struct-instance elements are missing (DEF-142/215/216) |
| `len` intrinsic emission | `codegen/expr.rs:38-60`, `calls.rs:280-290` | a runtime `len` reads the view's `.len`; a resolved fixed array folds |
| View argument crossing | `codegen/calls.rs:130-160`, `225-275` | an uncontracted-parameter view crosses as its pointer member; a `cstring` parameter special-cases literals only |
| `toCString` | `stdlib/std.text.clo:45`, `ffi stdtext` (`:14-19`) | declares `String` and returns the `cl_string_ptr` pointer (DEF-154) |
| Shim view helpers | `runtime/shim_host.c:256` (`cl_string_ptr`), `:294-317` (the concat parts) | the pointer helpers the stdlib body uses |

## What the phases add

| Phase | New machinery |
|-------|---------------|
| 1 | The initializer guard-operand materialization gains the dividend and the index base; the typedef collector gains a value's slice-typed result, the lifted specializations, and the instance-element registration |
| 2 | A recorded `iterables` duty list plus `check_iterable_requirements` in the instance audit; `member_type`'s bare-parameter arm; `resolved_value_ty`'s conditional arm |
| 3 | The `cstring` view rejection; the argument-count check (both call forms, AR-2-gated); the unresolved-member report; the widened `toCString` |
| 4 | A shared null-shape predicate and its `E10012` sites (store, return, argument, iterable) |
| 5 | The per-site state-report ledger; the extended capture recorder and the retired non-escape lend duty; the generated-owner display phrase |
