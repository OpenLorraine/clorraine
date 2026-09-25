# Execution Plan: Clorraine v1 language completion

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-25 09:02
> **Progress**: 1/47 tasks (2%)
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
| 1 | Generics | 16 |
| 2 | Lambdas | 16 |
| 3 | Volatile and MMIO | 11 |
| 4 | Closeout | 4 |

**Total: 47 tasks across 4 phases** (no fabricated hour estimates)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
> The task checkboxes below are the single source of truth for progress. Mark a
> task `[~]` on implementation and promote it to `[x]` only on a passing verify;
> update the Progress header and Last Updated stamp after every task. Timestamps
> come from `date '+%Y-%m-%d %H:%M'`.

---

## Phase 1: Generics

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Expected modification set**: `compiler/src/types/table.rs`, `compiler/src/sema/{call,expr,mod}.rs`, `compiler/src/ir/mono.rs`, `compiler/src/own/generics.rs`, `compiler/src/codegen/{types,program}.rs`, `compiler/tests/v1_language_spec_test.rs`, `compiler/tests/v1_language_impl_test.rs`, `compiler/tests/compile_fail_spec_test.rs`, `spec/tests/compile-fail/**`, `spec/14-diagnostics.md`, `plans/clorraine-v1-language-completion/99-execution-plan.md`
> **Scope mode**: strict

### Step 1.1: Encode the generics behaviour before it exists

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-1..ST-9, ST-27 · [03-01](03-01-generics.md) · AR-15..AR-18, AR-25, AR-29, AR-30
**Objective**: write the runnable generics spec tests, un-skip the generics
goldens, and record the red phase.

- [x] 1.1.1 [spec-author] Write the runnable generics cases from ST-1..ST-4 in a new spec suite — `compiler/tests/v1_language_spec_test.rs` ✅ (completed: 2026-09-25 09:02)
- [ ] 1.1.2 [spec-author] Un-skip the generics compile-fail goldens: remove `E10303`/`E10304` from `EXCLUDED_CODES`, add `E10303`/`E10304`/`E10306` to `ASSERTED_CODES`, drop the `// stage:` annotation from `E10306`, and add the ST-27 golden — `compiler/tests/compile_fail_spec_test.rs`, `spec/tests/compile-fail/{E10306_generic_ephemeral_arg,E10305_generic_struct_recursion}.clo`
- [ ] 1.1.3 Run the suite and record the red phase: the new generics cases and the un-skipped goldens fail; no existing case regresses

**Deliverables**:
- The generics spec-test cases and the un-skipped goldens
- A recorded red phase

**Verify**: `make check` — the only failures are the intended generics ones.

### Step 1.2: Implement generics

**Reference**: [03-01](03-01-generics.md) §Implementation Details · AR-15..AR-18, AR-24, AR-29, AR-30
**Objective**: generic structs, callable bounds, callable-only values, and the
ephemeral-argument rule.

- [ ] 1.2.1 Carry each type parameter's callable bound into the type stage (`TypeParamSig`) and update its callers — `compiler/src/types/table.rs`, `compiler/src/sema/call.rs`
- [ ] 1.2.2 Add `StructInstance` and `record_struct`/`struct_instances` beside the function instances — `compiler/src/ir/mono.rs`
- [ ] 1.2.3 Collect struct instances from resolved declaration and IR types, recursing into nested arguments — `compiler/src/own/generics.rs`
- [ ] 1.2.4 Include a struct's type arguments in `type_key` and `c_type` (`Cl<Name>__<typekey>`) — `compiler/src/codegen/types.rs`
- [ ] 1.2.5 Emit one concrete struct per recorded instance with substituted field types — `compiler/src/codegen/program.rs`
- [ ] 1.2.6 Enforce `E10303` when a bound argument is not a matching `func(...)` or closure — `compiler/src/sema/call.rs`
- [ ] 1.2.7 Enforce `E10304` when a callable-only value is stored or returned — `compiler/src/sema/{call,expr}.rs`
- [ ] 1.2.8 Enforce `E10306` when a resolved aggregate type contains a closure argument — `compiler/src/sema/{mod,call,expr}.rs`
- [ ] 1.2.9 Extend the `E10305` recursion cap to cover generic-struct instantiation — `compiler/src/sema/call.rs`

### Step 1.3: Implementation tests, registry, and verification

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-1..ST-9, ST-27 · [03-04](03-04-spec-goldens.md) · AR-25
**Objective**: green the generics phase and align the registry wording.

- [ ] 1.3.1 Add generics implementation and unit tests (instance dedupe, nested keys, all three diagnostics) — `compiler/tests/v1_language_impl_test.rs`, in-module tests
- [ ] 1.3.2 Update the `E103xx` registry wording to the implemented status — `spec/14-diagnostics.md`
- [ ] 1.3.3 Verify the generics suite is green — `make check`
- [ ] 1.3.4 Verify the generics phase end to end — `make verify`

**Deliverables**:
- Generic structs and the three diagnostics working
- Generics spec tests green

**Verify**: `make verify`.

---

## Phase 2: Lambdas

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Expected modification set**: `compiler/src/types/mod.rs`, `compiler/src/sema/{expr,call}.rs`, `compiler/src/ir.rs`/`compiler/src/ir/**`, `compiler/src/own/generics.rs`, `compiler/src/codegen/{expr,calls,types,program}.rs`, `compiler/tests/v1_language_{spec,impl}_test.rs`, `compiler/tests/compile_fail_spec_test.rs`, `spec/tests/compile-fail/**`, `spec/tests/staged/README.md`, `spec/tests/compile-fail/README.md`, `spec/check.sh`, `spec/06-functions.md`, `spec/grammar.ebnf.md`, `plans/clorraine-v1-language-completion/99-execution-plan.md`
> **Scope mode**: strict

### Step 2.1: Encode the lambda behaviour before it exists

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-10..ST-19 · [03-02](03-02-lambdas.md) · AR-9..AR-14, AR-21, AR-22
**Objective**: write the runnable lambda spec tests, un-skip the lambda goldens,
retire `E90002`, and record the red phase.

- [ ] 2.1.1 [spec-author] Write the runnable lambda cases from ST-10..ST-14 — `compiler/tests/v1_language_spec_test.rs`
- [ ] 2.1.2 [spec-author] Un-skip the lambda goldens: add `E10400`-`E10404` to `ASSERTED_CODES`, remove `E10400` from `EXCLUDED_CODES`, drop the `// stage:` annotations; retire `E90002` (remove its golden; add it to the excluded lists; update the staged and compile-fail READMEs) — `compiler/tests/compile_fail_spec_test.rs`, `spec/tests/compile-fail/E1040*.clo`, `spec/check.sh`, `spec/tests/{staged,compile-fail}/README.md`, delete `E90002_lambda_capture_staged.clo`
- [ ] 2.1.3 Run the suite and record the red phase: the lambda cases and un-skipped goldens fail; no existing case regresses

**Deliverables**:
- The lambda spec-test cases and the un-skipped goldens
- A recorded red phase

**Verify**: `make check` — the only failures are the intended lambda ones.

### Step 2.2: Implement lambdas

**Reference**: [03-02](03-02-lambdas.md) §Implementation Details · AR-9..AR-14, AR-23, AR-28, AR-31
**Objective**: lift, type, and emit lambdas, with the capture and ephemerality
rules.

- [ ] 2.2.1 Add the `ClosureTy` representation and the `TyKind::Closure` variant — `compiler/src/types/mod.rs`
- [ ] 2.2.2 Type and scope-check a lambda, binding captures, and classify each capture (copy / read-only borrow / `[borrow]`) — `compiler/src/sema/expr.rs`
- [ ] 2.2.3 Enforce `E10403`, `E10401`, and `E10402` — `compiler/src/sema/expr.rs`
- [ ] 2.2.4 Enforce `E10400` at a `func(...)` conversion and `E10404` at a storage, return, or capture — `compiler/src/sema/{expr,call}.rs`
- [ ] 2.2.5 Add the IR closure value and closure-call forms and thread them through lowering and the ownership pass — `compiler/src/ir/**`, `compiler/src/own/**`
- [ ] 2.2.6 Lift each lambda to a `static` function and emit its environment struct — `compiler/src/codegen/{expr,program}.rs`
- [ ] 2.2.7 Emit the capture-less function-pointer value and the capturing two-word value — `compiler/src/codegen/expr.rs`
- [ ] 2.2.8 Emit the closure call and the monomorphized call form inside a generic body — `compiler/src/codegen/{calls,expr}.rs`
- [ ] 2.2.9 Key closure instances in monomorphization (`closure_<N>`) — `compiler/src/own/generics.rs`, `compiler/src/codegen/types.rs`

### Step 2.3: Implementation tests, spec prose, and verification

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-10..ST-19 · [03-04](03-04-spec-goldens.md) · AR-11, AR-32
**Objective**: green the lambda phase and align the spec prose.

- [ ] 2.3.1 Add lambda implementation and unit tests (lift/env naming, two-word call, callable-bound instantiation) — `compiler/tests/v1_language_impl_test.rs`, in-module tests
- [ ] 2.3.2 Update the spec prose: Ch 06 §6.1 and L-7 and the diagnostics table, and the grammar §8 note — `spec/06-functions.md`, `spec/grammar.ebnf.md`
- [ ] 2.3.3 Verify the lambda suite is green — `make check`
- [ ] 2.3.4 Verify the lambda phase end to end — `make verify`

**Deliverables**:
- Lambdas stages 1-4 working
- Spec prose aligned

**Verify**: `make verify`.

---

## Phase 3: Volatile and MMIO

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Expected modification set**: `compiler/src/sema/expr.rs`, `compiler/src/codegen/{calls,program}.rs`, `compiler/src/{ir/global,sema/items}.rs`, `compiler/src/stdlib.rs`, `stdlib/std.amiga.custom.clo`, `compiler/tests/v1_language_{spec,impl}_test.rs`, `compiler/tests/corpus_spec_test.rs`, `spec/tests/corpus/volatile-mmio/**`, `docker/scripts/corpus-amiga.sh`, `plans/clorraine-v1-language-completion/99-execution-plan.md`
> **Scope mode**: strict

### Step 3.1: Encode the volatile behaviour before it exists

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-20..ST-26 · [03-03](03-03-volatile-mmio.md) · AR-19, AR-20, AR-27, AR-28
**Objective**: write the runnable volatile spec tests, add the MMIO corpus, and
record the red phase.

- [ ] 3.1.1 [spec-author] Write the runnable volatile cases from ST-20..ST-24 — `compiler/tests/v1_language_spec_test.rs`
- [ ] 3.1.2 [spec-author] Add the MMIO corpus and the host import-rejection golden — `spec/tests/corpus/volatile-mmio/{program.clo,expected-output.txt,README.md}`, `spec/tests/compile-fail/E11200_custom_module.clo`, `compiler/tests/{corpus_spec_test,compile_fail_spec_test}.rs`
- [ ] 3.1.3 Run the suite and record the red phase: the volatile cases fail (the intrinsic lowers to `0`) and the MMIO import is rejected; no existing case regresses

**Deliverables**:
- The volatile spec-test cases and the MMIO corpus
- A recorded red phase

**Verify**: `make check` — the only failures are the intended volatile ones.

### Step 3.2: Implement volatile and the MMIO module

**Reference**: [03-03](03-03-volatile-mmio.md) §Implementation Details · AR-19, AR-20, AR-28
**Objective**: type and lower the six intrinsics, emit the volatile global
qualifier, and add the wrapper module.

- [ ] 3.2.1 Type the six volatile intrinsics and their argument conversions — `compiler/src/sema/expr.rs`
- [ ] 3.2.2 Lower the six intrinsics to a C `volatile`-qualified access — `compiler/src/codegen/calls.rs`
- [ ] 3.2.3 Carry the `volatile` global flag to codegen and emit the C qualifier — `compiler/src/ir/global.rs`, `compiler/src/sema/items.rs`, `compiler/src/codegen/program.rs`
- [ ] 3.2.4 Author the `std.amiga.custom` module and register it — `stdlib/std.amiga.custom.clo`, `compiler/src/stdlib.rs`
- [ ] 3.2.5 Wire the MMIO corpus into the hosted-amiga leg: add `volatile-mmio` to the corpus script's running-program set and add its corpus test — `docker/scripts/corpus-amiga.sh`, `compiler/tests/corpus_spec_test.rs`

### Step 3.3: Implementation tests and verification

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-20..ST-26 · AR-19, AR-20
**Objective**: green the volatile phase.

- [ ] 3.3.1 Add volatile implementation and unit tests (global qualifier, module registration, lowering forms) — `compiler/tests/v1_language_impl_test.rs`, in-module tests
- [ ] 3.3.2 Verify the volatile suite is green — `make check`
- [ ] 3.3.3 Verify the volatile phase end to end, including the hosted-amiga MMIO corpus — `make verify`

**Deliverables**:
- The six intrinsics, the volatile global qualifier, and the MMIO module working
- Volatile spec tests and the MMIO corpus green

**Verify**: `make verify`.

---

## Phase 4: Closeout

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Expected modification set**: `compiler/tests/determinism_spec_test.rs`, `AGENTS.md`, `plans/00-roadmap.md`, `plans/clorraine-v1-language-completion/99-execution-plan.md`
> **Scope mode**: strict

### Step 4.1: Sweep, document, and final verification

**Reference**: [01-requirements.md](01-requirements.md) acceptance criteria · AR-21, AR-32
**Objective**: remove stale references, confirm determinism, and close the plan.

- [ ] 4.1.1 Sweep for stale `E90002` and `// stage:` references and dead code across the compiler, spec, and tests
- [ ] 4.1.2 Confirm byte-identical rebuilds for a sample exercising generics, lambdas, and volatile — `compiler/tests/determinism_spec_test.rs`
- [ ] 4.1.3 Update `AGENTS.md` (test map, any staged-surface note) and confirm `plans/00-roadmap.md` reflects completion — `AGENTS.md`, `plans/00-roadmap.md`
- [ ] 4.1.4 Full acceptance run — `make verify`

**Deliverables**:
- A clean tree with no stale staged references
- `make verify` green

**Verify**: `make verify`.

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
