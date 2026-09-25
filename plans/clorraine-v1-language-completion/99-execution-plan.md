# Execution Plan: Clorraine v1 language completion

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-25 16:05
> **Progress**: 49/49 tasks (100%; all four phases complete and reviewed — generics, lambdas, volatile/MMIO, and closeout; `make verify` green)
> **CodeOps Artifact Schema**: 1

## Overview

Complete generics, lambdas (stages 1-4), and volatile access with the per-chip
MMIO wrappers, fixing the two silent miscompiles on the way. Four phases follow
the specification-first ordering: each feature phase writes its specification
tests and records the red phase, implements, then lands implementation tests,
spec edits, and full verification. Generics precedes lambdas because lambda
stage 4 depends on the callable bound; volatile/MMIO is independent.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Generics | 17 |
| 2 | Lambdas | 17 |
| 3 | Volatile and MMIO | 11 |
| 4 | Closeout | 4 |

**Total: 49 tasks across 4 phases** (no fabricated hour estimates; 1.2.10 and
1.2.11 were added during execution to resolve runtime ambiguities AR-40/AR-41)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
> The task checkboxes below are the single source of truth for progress. Mark a
> task `[~]` on implementation and promote it to `[x]` only on a passing verify;
> update the Progress header and Last Updated stamp after every task. Timestamps
> come from `date '+%Y-%m-%d %H:%M'`.

---

## Phase 1: Generics

> **Phase baseline tree**: `119365d28b44f24652e5384139ea26ca79d210f7` (the commit before phase 1; the phase-start snapshot script was unavailable, so the post-phase review diffs from this commit)
> **Expected modification set**: `compiler/src/types/table.rs`, `compiler/src/sema/{call,expr,mod}.rs`, `compiler/src/ir/mono.rs`, `compiler/src/own/generics.rs`, `compiler/src/codegen/{types,program,expr}.rs`, `compiler/tests/v1_language_spec_test.rs`, `compiler/tests/v1_language_impl_test.rs`, `compiler/tests/compile_fail_spec_test.rs`, `spec/tests/compile-fail/**`, `spec/14-diagnostics.md`, `plans/clorraine-v1-language-completion/99-execution-plan.md`
> **Scope mode**: strict

### Step 1.1: Encode the generics behaviour before it exists

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-1..ST-9, ST-27 · [03-01](03-01-generics.md) · AR-15..AR-18, AR-25, AR-29, AR-30
**Objective**: write the runnable generics spec tests, un-skip the generics
goldens, and record the red phase.

- [x] 1.1.1 [spec-author] Write the runnable generics cases from ST-1..ST-4 in a new spec suite — `compiler/tests/v1_language_spec_test.rs` ✅ (completed: 2026-09-25 09:02)
- [x] 1.1.2 [spec-author] Un-skip the generics compile-fail goldens: remove `E10303`/`E10304` from `EXCLUDED_CODES`, add `E10303`/`E10304`/`E10306` to `ASSERTED_CODES`, drop the `// stage:` annotation from `E10306`, and add the ST-27 golden — `compiler/tests/compile_fail_spec_test.rs`, `spec/tests/compile-fail/{E10306_generic_ephemeral_arg,E10305_generic_struct_recursion}.clo` ✅ (completed: 2026-09-25 09:03; E10306 was re-staged to Phase 2 later, see AR-42/task 2.2.10)
- [x] 1.1.3 Run the suite and record the red phase: the new generics cases and the un-skipped goldens fail; no existing case regresses ✅ (completed: 2026-09-25 09:04) — red: 4 goldens (E10303/E10304/E10306/E10305-struct) and all 4 runnable cases fail; E10305-struct currently overflows the compiler stack

**Deliverables**:
- The generics spec-test cases and the un-skipped goldens
- A recorded red phase

**Verify**: `make check` — the only failures are the intended generics ones.

### Step 1.2: Implement generics

**Reference**: [03-01](03-01-generics.md) §Implementation Details · AR-15..AR-18, AR-24, AR-29, AR-30
**Objective**: generic structs, callable bounds, callable-only values, and the
ephemeral-argument rule.

- [x] 1.2.1 Carry each type parameter's callable bound into the type stage (`TypeParamSig`) and update its callers — `compiler/src/types/table.rs`, `compiler/src/sema/call.rs` ✅ (completed: 2026-09-25 09:10)
- [x] 1.2.2 Add `StructInstance` and `record_struct`/`struct_instances` beside the function instances — `compiler/src/ir/mono.rs` ✅ (completed: 2026-09-25 09:11)
- [x] 1.2.3 Collect struct instances from resolved declaration and IR types, recursing into nested arguments — `compiler/src/own/generics.rs` ✅ (completed: 2026-09-25 09:14)
- [x] 1.2.4 Include a struct's type arguments in `type_key` and `c_type` (`Cl<Name>__<typekey>`) — `compiler/src/codegen/types.rs` ✅ (completed: 2026-09-25 09:24; landed with 1.2.5, plus `StructInstance.args` carries `Ty` and `struct_literal` casts to the concrete type)
- [x] 1.2.5 Emit one concrete struct per recorded instance with substituted field types — `compiler/src/codegen/program.rs` ✅ (completed: 2026-09-25 09:24; landed with 1.2.4; ST-1/ST-2 green, Array regressions cleared)
- [x] 1.2.6 Enforce `E10303` when a bound argument is not a matching `func(...)` or closure — `compiler/src/sema/call.rs` ✅ (completed: 2026-09-25 10:11; also fixed `types::substitute` to descend into function types so a bound's return/params substitute)
- [x] 1.2.7 Enforce `E10304` when a callable-only value is stored or returned — `compiler/src/sema/{call,expr}.rs` ✅ (completed: 2026-09-25 10:11; implemented in `sema/{mod,stmt}.rs`)

- [x] 1.2.9 Extend the `E10305` recursion cap to cover generic-struct instantiation — `compiler/src/{sema/items,call}.rs`, `compiler/src/types/table.rs` ✅ (completed: 2026-09-25 10:13; uses a value-embedding cycle check plus a depth guard on `size_of`)
- [x] 1.2.10 Store `Instance.args` as `Ty` and key every function instance from `type_key`, removing `type_from_key` — `compiler/src/ir/mono.rs`, `compiler/src/own/generics.rs`, `compiler/src/codegen.rs`, `compiler/src/codegen/{program,function}.rs`, `compiler/src/ir/tests.rs` (AR-40) ✅ (completed: 2026-09-25 10:09; ST-4 green)
- [x] 1.2.11 Allow `export` on a generic struct, export `Pair` in the ST-3 fixture, and update the spec rules `STR-21`/`G-5` and the `E10302` wording to functions only — `compiler/src/sema/items.rs`, `compiler/tests/v1_language_spec_test.rs`, `spec/07-structs.md`, `spec/06-functions.md`, `spec/14-diagnostics.md`, `spec/grammar.ebnf.md`, `spec/decisions.md`, `spec/17-codegen.md` (AR-41) ✅ (completed: 2026-09-25 10:09; ST-3 green, spec check OK)

### Step 1.3: Implementation tests, registry, and verification

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-1..ST-9, ST-27 · [03-04](03-04-spec-goldens.md) · AR-25
**Objective**: green the generics phase and align the registry wording.

- [x] 1.3.1 Add generics implementation and unit tests (instance dedupe, nested keys, all three diagnostics) — `compiler/tests/v1_language_impl_test.rs`, in-module tests ✅ (completed: 2026-09-25 10:32; 5 integration tests plus unit tests for `type_key`/`c_type` and struct-instance dedupe)
- [x] 1.3.2 Update the `E103xx` registry wording to the implemented status — `spec/14-diagnostics.md` ✅ (completed: 2026-09-25 10:32; the E10303/E10304/E10305 rows already state the implemented triggers and messages, and `spec/check.sh` cross-validates them; E10306 stays defined and is asserted in Phase 2 per AR-42, so no wording edit was needed)
- [x] 1.3.3 Verify the generics suite is green — `make check` (E10306 is re-staged to Phase 2 by AR-42) ✅ (completed: 2026-09-25 10:32; spec check OK; 17 targets pass)
- [x] 1.3.4 Verify the generics phase end to end — `make verify` ✅ (completed: 2026-09-25 10:34; all legs passed)

**Deliverables**:
- Generic structs and the three diagnostics working
- Generics spec tests green

**Verify**: `make verify`.

### Phase 1 review (strict profile)

Reviewed inline on 2026-09-25 (independent reviewer agents are manual-only in
this environment and the shared quality-profile file is absent, so the review
ran on the phase diff `119365d..HEAD`). Findings:

| # | Severity | Finding | Resolution |
|---|----------|---------|------------|
| F1 | 🟠 Major | A struct embedding an instantiated generic struct emitted invalid C: the embedded definition was written after its use, and a generic field's instantiation was not always collected (`Box<T>{ p: Pair<T> }` as `Box<i32>`). `clc` exited 0; `cc` failed `unknown type name 'ClPair__i32'`. Fixed under the user's ruling. | Expanded each recorded instance's fields in `collect_struct_instances`; emit struct definitions in by-value dependency order behind a forward typedef (`StructUnit`/`order_struct_units`). Verified by a runnable spec case (g5), an ordering impl test, and re-run `make check`/`make verify`. |
| F2 | 🟡 Minor | `E10304` is enforced for `let` and `return` only; storing a callable-only value in a field or global is not diagnosed (spec G-4). | Report-only; no trigger or spec test covers it in this plan. Left as a known gap. |
| F3 | 🟡 Minor | The E10305 value-cycle check missed recursion through a generic parameter field, e.g. `struct S { b: Box<S> }` with `Box<T>{ inner: T }`; `size_of`'s depth guard prevented a compiler crash but the C was invalid. | Fixed with F1: `embeds_by_value` now substitutes a generic struct's arguments into its fields before descending; `struct S { b: Box<S> }` reports E10305. Covered by an impl test. |

---

## Phase 2: Lambdas

> **Phase baseline tree**: `e14fb1ee1bd5b2e2dc93575dba85bf58f0ff3c37` (Phase 2 start; the review diffs from this commit)
> **Expected modification set**: `compiler/src/types/mod.rs`, `compiler/src/sema/{expr,call}.rs`, `compiler/src/ir.rs`/`compiler/src/ir/**`, `compiler/src/own/generics.rs`, `compiler/src/codegen/{expr,calls,types,program}.rs`, `compiler/tests/v1_language_{spec,impl}_test.rs`, `compiler/tests/compile_fail_spec_test.rs`, `spec/tests/compile-fail/**`, `spec/tests/staged/README.md`, `spec/tests/compile-fail/README.md`, `spec/check.sh`, `spec/06-functions.md`, `spec/grammar.ebnf.md`, `plans/clorraine-v1-language-completion/99-execution-plan.md`
> **Scope mode**: strict

### Step 2.1: Encode the lambda behaviour before it exists

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-10..ST-19 · [03-02](03-02-lambdas.md) · AR-9..AR-14, AR-21, AR-22
**Objective**: write the runnable lambda spec tests, un-skip the lambda goldens,
retire `E90002`, and record the red phase.

- [x] 2.1.1 [spec-author] Write the runnable lambda cases from ST-10..ST-14 — `compiler/tests/v1_language_spec_test.rs` ✅ (completed: 2026-09-25 14:00; l10..l14: capture-less pointer, capture-less parameter, copy capture, read-only borrow of a non-duplicable, and `[borrow]` write-through)
- [x] 2.1.2 [spec-author] Un-skip the lambda goldens: add `E10400`-`E10404` to `ASSERTED_CODES`, remove `E10400` from `EXCLUDED_CODES`, drop the `// stage:` annotations; retire `E90002` (remove its golden; add it to the excluded lists; update the staged and compile-fail READMEs) — `compiler/tests/compile_fail_spec_test.rs`, `spec/tests/compile-fail/E1040*.clo`, `spec/check.sh`, `spec/tests/{staged,compile-fail}/README.md`, delete `E90002_lambda_capture_staged.clo` ✅ (completed: 2026-09-25 14:00; `E10306` also joined `ASSERTED_CODES`; `E90002` moved to the Ch 14 retired table, which the existing retired-code rule already excludes, so no inert `check.sh` entry was added)
- [x] 2.1.3 Run the suite and record the red phase: the lambda cases and un-skipped goldens fail; no existing case regresses ✅ (completed: 2026-09-25 14:00) — red: l10..l14 fail (the front end still reports `E90000`/`E90002`), and `compile_fail_goldens` fails 6 goldens (E10306, E10400..E10404); the other 15 targets pass, so no existing case regresses

**Deliverables**:
- The lambda spec-test cases and the un-skipped goldens
- A recorded red phase

**Verify**: `make check` — the only failures are the intended lambda ones.

### Step 2.2: Implement lambdas

**Reference**: [03-02](03-02-lambdas.md) §Implementation Details · AR-9..AR-14, AR-23, AR-28, AR-31
**Objective**: lift, type, and emit lambdas, with the capture and ephemerality
rules.

- [x] 2.2.1 Add the `ClosureTy` representation and the `TyKind::Closure` variant — `compiler/src/types/mod.rs` ✅ (completed: 2026-09-25 14:03; `ClosureTy { id, params, returns }`, `TyKind::Closure`, `is_closure`, non-duplicable, and `c_type`/`type_key`/`ffi_type`/`display` coverage; the red set is unchanged, so no regression)
- [x] 2.2.2 Type and scope-check a lambda, binding captures, and classify each capture (copy / read-only borrow / `[borrow]`) — `compiler/src/sema/expr.rs` ✅ (completed: 2026-09-25 15:05; `check_lambda` types the body in a child scope, records each `ClosureSite`, and classifies captures by duplicability; `Func` for capture-less, `Closure` for capturing)
- [x] 2.2.3 Enforce `E10403`, `E10401`, and `E10402` — `compiler/src/sema/expr.rs` ✅ (completed: 2026-09-25 15:05; a `take` capture is E10403; moving a borrowed capture through a `take` argument or a return/expression body is E10401; writing a copy/read-only capture is E10402. `FuncSig` gained parameter modes so a call can tell a `take` argument)
- [x] 2.2.4 Enforce `E10400` at a `func(...)` conversion and `E10404` at a storage, return, or capture — `compiler/src/sema/{expr,call}.rs` ✅ (completed: 2026-09-25 15:05; E10400 in `check_conversion`; E10404 at `let`/`const`/return/array-element storage; a callable-bound `F` now accepts a matching closure signature)
- [x] 2.2.5 Add the IR closure value and closure-call forms and thread them through lowering and the ownership pass — `compiler/src/ir/**`, `compiler/src/own/**` ✅ (completed: 2026-09-25 15:20; `ValueKind::Closure`/`ClosureCall`, `PlaceBase::EnvField`, a `Function::lambda_site` marker, and lowering of `ExprKind::Lambda` with a saved/restored nested checker)
- [x] 2.2.6 Lift each lambda to a `static` function and emit its environment struct — `compiler/src/codegen/{expr,program}.rs` ✅ (completed: 2026-09-25 15:20; `cl_lambda_<N>` with a leading `struct ClEnv_<N>* __env`, the env struct emitted before its prototypes and body)
- [x] 2.2.7 Emit the capture-less function-pointer value and the capturing two-word value — `compiler/src/codegen/expr.rs` ✅ (completed: 2026-09-25 15:20; a capture-less value is the lifted function name; a capturing value is `((ClClosure_<N>){ cl_lambda_<N>, &env })`; a copy capture stores the value, a borrow capture stores the address)
- [x] 2.2.8 Emit the closure call and the monomorphized call form inside a generic body — `compiler/src/codegen/{calls,expr}.rs` ✅ (completed: 2026-09-25 15:20; a direct closure call is `cl_lambda_<N>(&env, args...)`; inside an instance the callable parameter emits `((f).fn((f).env, args...))`)
- [x] 2.2.9 Key closure instances in monomorphization (`closure_<N>`) — `compiler/src/own/generics.rs`, `compiler/src/codegen/types.rs` ✅ (completed: 2026-09-25 15:20; `Instance::dedupe_key` renders a closure argument site-aware as `closure_<N>`, matching `type_key`, so two lambdas with the same signature stay distinct)
- [x] 2.2.10 Enforce `E10306` when a resolved aggregate type contains an ephemeral closure argument, and un-stage its golden — `compiler/src/sema/{mod,call,expr}.rs`, `compiler/tests/compile_fail_spec_test.rs`, `spec/tests/compile-fail/E10306_generic_ephemeral_arg.clo` (moved from Phase 1 by AR-42) ✅ (completed: 2026-09-25 15:05; a closure bound to an unbounded generic parameter, or used as a generic struct's type argument, is E10306; the golden is asserted and green)

### Step 2.3: Implementation tests, spec prose, and verification

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-10..ST-19 · [03-04](03-04-spec-goldens.md) · AR-11, AR-32
**Objective**: green the lambda phase and align the spec prose.

- [x] 2.3.1 Add lambda implementation and unit tests (lift/env naming, two-word call, callable-bound instantiation) — `compiler/tests/v1_language_impl_test.rs`, in-module tests ✅ (completed: 2026-09-25 15:23; four impl tests: environment lifting and the two-word call, a borrow capture holding a pointer and writing through, distinct sites getting distinct instances, and a capture-less lambda needing no environment)
- [x] 2.3.2 Update the spec prose: Ch 06 §6.1 and L-7 and the diagnostics table, and the grammar §8 note — `spec/06-functions.md`, `spec/grammar.ebnf.md` ✅ (completed: 2026-09-25 15:23; §6.1 now states lambda semantics are implemented, the capturing example is an inline call, L-7 and the `E90002` table row are removed, and the grammar §8 note and freeze checklist drop the staged-lambda wording)
- [x] 2.3.3 Verify the lambda suite is green — `make check` ✅ (completed: 2026-09-25 15:23; `make check` exit 0 — spec check OK, all 17 Rust targets green)
- [x] 2.3.4 Verify the lambda phase end to end — `make verify` ✅ (completed: 2026-09-25 15:23; `make verify` exit 0 — "verify: all legs passed")

**Deliverables**:
- Lambdas stages 1-4 working
- Spec prose aligned

**Verify**: `make verify`.

### Phase 2 review (strict profile)

Reviewed inline on 2026-09-25 (independent reviewer agents are manual-only in
this environment and the shared quality-profile file is absent, so the review
ran on the Phase 2 diff `e14fb1e..HEAD`). Findings:

| # | Severity | Finding | Resolution |
|---|----------|---------|------------|
| R2-1 | 🟠 Major | A block-bodied lambda that used `return expr;` was rejected: `check_lambda` typed a block body as `void`, so binding it to `func(i32): i32` reported `E10210`. The grammar (§8) allows `expression \| block` as a lambda body, so this rejected valid in-scope syntax. | Fixed as a necessary correction: the semantic stage now infers a block body's return type from its first value `return` (bare `return;` in a block lambda is not the void-return error). Covered by the impl test `a_block_body_lambda_infers_its_return_type`; `make check` and `make verify` re-run green. |
| R2-2 | 🟡 Minor | A lambda defined inside a generic body is out of scope, so it is lifted once rather than per instantiation. | Report-only; no ST case or golden covers it, and the design excludes it. |
| R2-3 | 🟡 Minor | Several `return`s in a block-bodied lambda with conflicting types are not diagnosed; the first non-void type wins. | Report-only; type unification is not specified and no test covers it. |

---

## Phase 3: Volatile and MMIO

> **Phase baseline tree**: `9b69289d773c1e9907dfee2858d1c3030cbbbaa9` (Phase 3 start; the review diffs from this commit)
> **Expected modification set**: `compiler/src/sema/expr.rs`, `compiler/src/codegen/{calls,program}.rs`, `compiler/src/codegen.rs`, `compiler/src/consts/mod.rs`, `compiler/src/own/expr.rs`, `compiler/src/stdlib.rs`, `stdlib/std.amiga.custom.clo`, `compiler/tests/v1_language_{spec,impl}_test.rs`, `compiler/tests/{corpus_spec_test,compile_fail_spec_test}.rs`, `spec/tests/corpus/volatile-mmio/**`, `spec/tests/compile-fail/E11200_custom_module.clo`, `docker/scripts/corpus-amiga.sh`, `plans/clorraine-v1-language-completion/99-execution-plan.md`
> **Scope mode**: strict
>
> The modification set was expanded during execution: the volatile global flag
> lives on `codegen::GlobalInfo` rather than `ir/global.rs`/`sema/items.rs`, and
> two necessary corrections were needed to support the `const raw` register base
> (`compiler/src/consts/mod.rs` folds a constant `raw` cast, and
> `compiler/src/own/expr.rs` treats a field write through a `raw` global as a
> pointee write rather than an immutable-binding write).

### Step 3.1: Encode the volatile behaviour before it exists

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-20..ST-26 · [03-03](03-03-volatile-mmio.md) · AR-19, AR-20, AR-27, AR-28
**Objective**: write the runnable volatile spec tests, add the MMIO corpus, and
record the red phase.

- [x] 3.1.1 [spec-author] Write the runnable volatile cases from ST-20..ST-24 — `compiler/tests/v1_language_spec_test.rs` ✅ (completed: 2026-09-25 15:45; v20 volatile byte round-trip, v21 16/32-bit round-trip, v23 read-between-writes, v24 global qualifier; ST-22 stays on the existing E10604 golden)
- [x] 3.1.2 [spec-author] Add the MMIO corpus and the host import-rejection golden — `spec/tests/corpus/volatile-mmio/{program.clo,expected-output.txt,README.md}`, `spec/tests/compile-fail/E11200_custom_module.clo`, `compiler/tests/{corpus_spec_test,compile_fail_spec_test}.rs` ✅ (completed: 2026-09-25 15:45; corpus program calls `setColor`/`setDma`, README documents the surface, golden imports `std.amiga.custom` under host, and a corpus host-rejection test was added)
- [x] 3.1.3 Run the suite and record the red phase: the volatile cases fail (the intrinsic lowers to `0`) and the MMIO import is rejected; no existing case regresses ✅ (completed: 2026-09-25 15:45) — red: v20/v21/v23 fail (the intrinsic lowers to `0`, so the read result differs and `p` is unused), v24 fails (no `volatile` global qualifier), `compile_fail_goldens` fails E11200_custom_module, `corpus_spec_test` fails the volatile-mmio host rejection, and the lib `every_corpus_import_is_exported` fails (module not registered); `./spec/check.sh` is green and the other targets pass, so no existing case regresses

**Deliverables**:
- The volatile spec-test cases and the MMIO corpus
- A recorded red phase

**Verify**: `make check` — the only failures are the intended volatile ones.

### Step 3.2: Implement volatile and the MMIO module

**Reference**: [03-03](03-03-volatile-mmio.md) §Implementation Details · AR-19, AR-20, AR-28
**Objective**: type and lower the six intrinsics, emit the volatile global
qualifier, and add the wrapper module.

- [x] 3.2.1 Type the six volatile intrinsics and their argument conversions — `compiler/src/sema/expr.rs` ✅ (completed: 2026-09-25 15:50; `readVolatileN` returns `uN` and `writeVolatileN` returns `void`, with the address checked against `raw uN` and the value against `uN` through the ordinary conversion diagnostic; implementation lives in `compiler/src/sema/expr.rs`, not `ir/global.rs`/`sema/items.rs`)
- [x] 3.2.2 Lower the six intrinsics to a C `volatile`-qualified access — `compiler/src/codegen/calls.rs` ✅ (completed: 2026-09-25 15:50; `(*(volatile uN *)(addr))` for a read and `(*(volatile uN *)(addr) = value)` for a write, native width, no byte swap)
- [x] 3.2.3 Carry the `volatile` global flag to codegen and emit the C qualifier — `compiler/src/ir/global.rs`, `compiler/src/sema/items.rs`, `compiler/src/codegen/program.rs` ✅ (completed: 2026-09-25 15:50; the flag is carried on `codegen::GlobalInfo` from the AST `GlobalLetDecl.volatile` and emitted by `emit_globals`; `ir/global.rs`/`sema/items.rs` were not needed)
- [x] 3.2.4 Author the `std.amiga.custom` module and register it — `stdlib/std.amiga.custom.clo`, `compiler/src/stdlib.rs` ✅ (completed: 2026-09-25 15:50; overlay `CustomRegs` with `raw` base `CUSTOM_BASE` and exported `setDma`/`setColor`, registered in `SOURCES` and `MODULES`; a constant `raw` cast now folds in `compiler/src/consts/mod.rs`, and a field write through a `raw` global is a pointee write in `compiler/src/own/expr.rs`, both necessary for the `const raw` base)
- [x] 3.2.5 Wire the MMIO corpus into the hosted-amiga leg: add `volatile-mmio` to the corpus script's running-program set and add its corpus test — `docker/scripts/corpus-amiga.sh`, `compiler/tests/corpus_spec_test.rs` ✅ (completed: 2026-09-25 15:50; `PROGRAMS` now runs `volatile-mmio` and it matches its recorded output under `vamos`, and the host-rejection corpus test landed with 3.1.2)

### Step 3.3: Implementation tests and verification

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-20..ST-26 · AR-19, AR-20
**Objective**: green the volatile phase.

- [x] 3.3.1 Add volatile implementation and unit tests (global qualifier, module registration, lowering forms) — `compiler/tests/v1_language_impl_test.rs`, in-module tests ✅ (completed: 2026-09-25 15:54; impl tests for the C global qualifier, the two volatile access forms, and the `const raw` field write; a `consts` unit test for the folded `raw` cast; a `stdlib` unit test for the `std.amiga.custom` registration)
- [x] 3.3.2 Verify the volatile suite is green — `make check` ✅ (completed: 2026-09-25 15:55; `make check` exit 0 — spec check OK, all 17 Rust targets green)
- [x] 3.3.3 Verify the volatile phase end to end, including the hosted-amiga MMIO corpus — `make verify` ✅ (completed: 2026-09-25 15:58; `make verify` exit 0 — "verify: all legs passed", including the `volatile-mmio` corpus on hosted-amiga)

**Deliverables**:
- The six intrinsics, the volatile global qualifier, and the MMIO module working
- Volatile spec tests and the MMIO corpus green

**Verify**: `make verify`.

### Phase 3 review (strict profile)

Reviewed inline on 2026-09-25 (independent reviewer agents are manual-only in
this environment, so the review ran on the Phase 3 diff
`9b69289d..HEAD`). Findings:

| # | Severity | Finding | Resolution |
|---|----------|---------|------------|
| R3-1 | 🟠 Major | A field write through a module-level `raw` pointer outside `unsafe` was not gated: `check_place_raw` walked local bindings only, so `BASE.dmacon = 1` (a `const raw`) reached codegen with no `E10600`. The fix that makes a `const raw` writable is what exposed this, so it shipped with 3.2 and was caught by the review. | Fixed as a necessary correction: `check_place_raw` now walks a global root too, so both a local and a module-level raw pointer report the unsafe gate. Covered by the impl test `a_field_write_through_a_raw_global_needs_unsafe`; `make check` and `make verify` re-run green. |
| R3-2 | 🟡 Minor | An overlay field access outside `unsafe` reports `E10600` (raw dereference), while the appendix's table lists "volatile intrinsic or overlay access" under `E10604`. Pre-existing; only the intrinsics have an `E10604` golden. | Report-only; no ST case or golden covers the overlay spelling. |
| R3-3 | 🟡 Minor | `setColor`/`setDma` write real custom-chip registers; the `vamos` corpus run proves the access completes against plain memory, not a visible hardware effect. | Report-only; the FS-UAE leg is not part of this corpus program and the risk is limited to the write. |
| R3-4 | 🟡 Minor | `volatile_width` falls back to `u8` for an unknown name; the fallback is unreachable because the caller passes only the six names. | Report-only; keeps the helper total without a panicking arm. |
| R3-5 | 🟡 Minor | AR-28 names `cop1lc` among the wrappers, but the module declares the `cop1lc` field with no exported setter (matching the module example in the design). | Report-only; the field documents the register map and the corpus covers `dmacon` and `color`. |

---

## Phase 4: Closeout

> **Phase baseline tree**: `f470a6bb2c680f3214fb012b454935c94dbeb03e` (Phase 4 start; the review diffs from this commit)
> **Expected modification set**: `compiler/tests/determinism_spec_test.rs`, `AGENTS.md`, `plans/00-roadmap.md`, `plans/clorraine-v1-language-completion/99-execution-plan.md`
> **Scope mode**: strict

### Step 4.1: Sweep, document, and final verification

**Reference**: [01-requirements.md](01-requirements.md) acceptance criteria · AR-21, AR-32
**Objective**: remove stale references, confirm determinism, and close the plan.

- [x] 4.1.1 Sweep for stale `E90002` and `// stage:` references and dead code across the compiler, spec, and tests ✅ (completed: 2026-09-25 16:04; no `E90002` reference remains in the compiler or any `.clo`, no golden carries a `// stage:` annotation, and the build is warning-free; the remaining `E90002` mentions are the retired registry row, check.sh/README commentary, and plan history)
- [x] 4.1.2 Confirm byte-identical rebuilds for a sample exercising generics, lambdas, and volatile — `compiler/tests/determinism_spec_test.rs` ✅ (completed: 2026-09-25 16:04; a mixed sample compiles twice and asserts both byte-identity and that the emitted C contains a generic struct, a lifted lambda, and a volatile access)
- [x] 4.1.3 Update `AGENTS.md` (test map, any staged-surface note) and confirm `plans/00-roadmap.md` reflects completion — `AGENTS.md`, `plans/00-roadmap.md` ✅ (completed: 2026-09-25 16:04; AGENTS.md now lists the v1 language spec/impl tests, the `std.amiga.custom` module, and the extended compiler scope; the roadmap marks PLAN-04 Done and records the new generics callable-bound defect DEF-05)
- [x] 4.1.4 Full acceptance run — `make verify` ✅ (completed: 2026-09-25 16:05; `make verify` exit 0 — "verify: all legs passed")

**Deliverables**:
- A clean tree with no stale staged references
- `make verify` green

**Verify**: `make verify`.

### Phase 4 review (strict profile)

Reviewed inline on 2026-09-25 on the Phase 4 diff `f470a6bb..HEAD` (closeout
only: a determinism case, `AGENTS.md`, and the roadmap). Findings:

| # | Severity | Finding | Resolution |
|---|----------|---------|------------|
| R4-1 | 🟡 Minor | The first determinism sample tripped a pre-existing generics gap: a callable-bound generic rejects a concrete function or closure when `T` is inferred from a generic-struct field (`apply(p.first, addOne)` is E10303). It is a false rejection with an explicit-type-argument workaround, and no authorized ST case covers it. | Report-only and recorded as roadmap defect DEF-05; the sample was changed to infer `T` from a literal and keeps the generic struct, the lambda, and the volatile access in the same program. |

---

## Dependencies

```
Phase 1 (Generics)
    ↓  (lambda stage 4 needs the callable bound)
Phase 2 (Lambdas)
    ↓
Phase 3 (Volatile and MMIO)   ← independent of Phase 2; runs after for a linear review order
    ↓
Phase 4 (Closeout)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ `make check` and `make verify` pass
3. ✅ No warnings/errors; `./spec/check.sh` green
4. ✅ No dead code and no stale `E90002`/`// stage:` references
5. ✅ Security hardened — `unsafe` confined to the stdlib wrappers; no new crates, shell, or network
6. ✅ Documentation updated (`AGENTS.md`, spec chapters, roadmap)
7. ✅ Code reviewed (repository quality profile)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
