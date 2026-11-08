# Execution Plan: v1.1 Language Surfaces

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Created**: 2026-11-08
> **Last Updated**: 2026-11-08
> **Progress**: 42/74 tasks (57%)
> **CodeOps Artifact Schema**: 1

## Overview

Deliver the three v1.1 surfaces in dependency order: the `Box<T>` type model
first (every later surface depends on it), then its allocator and recursive
drop glue, then ordinary `freedBy` deleters, then the C-hook contract and
trampoline emission, then the specification amendments and the full
acceptance matrix. Each phase follows the frozen specification-first order:
spec tests → recorded red evidence → implementation → green → impl tests →
focused verify. Each committed task leaves the tree buildable and the focused
tests green.

The plan incorporates every resolution in the
[Requirements Preflight](00-preflight-report-requirements.md) and the
[Plan Preflight](00-preflight-report.md): the type-only resolver
predeclaration, the `Box` shadowing rule, the `E10105` FFI check,
`emits_drop`, the helper prototypes, the module-scoped deleter identity in
`OwnershipOutput`, the ABI-identical hook signature set, the closure
materialization at its argument position, the two new compile-fail
specification tests, and the `E11215` runtime exclusion.

**🚨 Update this document after EACH completed task!**

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 0 | Baseline, ledger, and docs | 4 |
| 1 | Box type-model plumbing | 16 |
| 2 | Box allocation, API, and drop glue | 12 |
| 3 | X1 ordinary `freedBy` deleters | 9 |
| 4 | L5 hook contract and rejection | 5 |
| 5 | L5 acceptance, trampoline, and wrapper | 16 |
| 6 | Specification amendments and docs | 4 |
| 7 | Full acceptance, reviews, push | 8 |

**Total: 74 tasks across 8 phases.**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the single source of truth for progress. Mark
> `[~]` with a timestamp on start; promote `[x]` with a timestamp on verify
> pass; update the Progress header after every task. On a blocker, mark `[!]`
> and append `Blocked: <reason>`. Timestamps come from
> `date '+%Y-%m-%d %H:%M'`. Commit through the git-commit skill; keep docs
> commits separate from implementation commits; never amend or force-push.
> Update the PLAN-19 roadmap stage at every phase boundary.

---

## Phase 0: Baseline, Ledger, and Docs

> **Phase baseline tree**: `8a91bc5` plus the revised docs commit
> **Reasoning**: low — record the starting gate and preserve the verified
> requirements boundary.
> **Lenses**: compiler semantics · verification quality
> **Verify**: `make check` exit 0; `git status --short` empty before Phase 1.

### Step 0.1: Baseline evidence

**Reference**: [Current State](02-current-state.md) §7 · AR-1, AR-59
**Objective**: Freeze the starting compiler, the findings ledger, and the
docs baseline.

- [x] 0.1.1 Record the `make check` baseline (exit 0; fuzzer smoke 28/0) and the starting tree `a7dbdd5` ✅ (completed: 2026-11-08)
- [x] 0.1.2 Commit the requirements, designs, testing strategy, ambiguity register, and requirements preflight; record `8a91bc5` ✅ (completed: 2026-11-08)
- [x] 0.1.3 Create `05-findings.md` with the baseline, red-evidence, gate-log, and review sections ✅ (completed: 2026-11-08)
- [x] 0.1.4 Update the PLAN-19 roadmap row to Plan Preflighted and commit the revised plan documents (including this plan and `05-findings.md`); record the new baseline hash ✅ (completed: 2026-11-08; `3dac902`)

**Deliverables**:
- [ ] Baseline log under `build/v11/` (ignored)
- [ ] Findings ledger and revised plan committed; `git status --short` empty
- [ ] PLAN-19 at Plan Preflighted

**Verify**: `make check` exit 0; `git status --short` empty.

---

## Phase 1: Box Type-Model Plumbing

> **Phase baseline tree**: after Phase 0
> **Reasoning**: high — a new `TyKind` touches every ownership and emission
> path; a missed arm or silent `_` fallback is a soundness or invalid-C bug.
> **Lenses**: compiler semantics · memory safety · emission integrity
> **Verify**: `cargo test --test v11_language_surfaces_spec_test` and
> `cargo test --test v11_language_surfaces_impl_test`; `cargo test --workspace`;
> `make check`.

### Step 1.1: Type tests and red evidence

**Reference**: [Requirements](01-requirements.md) R1, R6 · [Box Design](03-01-box.md) §1, §8 · AR-6, AR-20, AR-65, AR-66
**Objective**: Pin the type-name, arity, construction, view, overlay,
ephemeral, freestanding, and FFI-boundary rules before the type exists.

- [x] 1.1.1 [spec-author] Write the Box type-model and FFI spec cases: BX-10..BX-17; `Box` arity; a user `Box` declaration shadows the built-in; `Box {}` E10203; `Box(x)` E10201; `raw i32(b)` outside `std.mem` E10201; `Box<static str>` accepted; `Box<Array<str>>` E10227; `Box<i32>` in `ffi` and inside a callback type E10105; `Wrap<i32>` in `ffi` E10105 — `compiler/tests/v11_language_surfaces_spec_test.rs`
- [x] 1.1.2 Record the red run: every `Box` spelling is E10219 on `a7dbdd5`; a user `struct Box<T>` is accepted; the FFI invalid-C cases are accepted and fail the strict C build. Record the first-attempt red results (E10203/E10201/E10227/E10105) when they appear after 1.2.5-1.2.9

### Step 1.2: Type model

**Reference**: [Box Design](03-01-box.md) §1-§2, §6, §9 · AR-6, AR-7, AR-68
**Objective**: Add `TyKind::Box` with a complete, compiler-checked type model,
including the silent `_` fallbacks and the FFI boundary.

- [x] 1.2.1 Add `TyKind::Box(Box<Ty>)`, `Ty::boxed`, display, equality, substitute, `contains_param`, `is_param`, `contains_closure`, and the `mentions_type_param` arm — `compiler/src/types/mod.rs`, `compiler/src/own/lower.rs`
- [x] 1.2.2 Add the type key, `RESERVED_COMPONENTS` entry, pointer size/alignment, `duplicable`, `size_of`, and `abi_size_of` arms — `compiler/src/types/{keys,layout,table}.rs`
- [x] 1.2.3 Add `is_view`/`is_lend_candidate`/`is_handle`/`is_nullable_capable`/`is_handle_or_aggregate`/`embeds_by_value`/overlay recursion — `compiler/src/borrow.rs`, `compiler/src/sema/items.rs`
- [x] 1.2.4 Add the total `abi::ffi_type` mapping (`void *`) and the `ffi::is_uncontracted_param` classification — `compiler/src/abi.rs`, `compiler/src/ffi.rs`
- [x] 1.2.5 Add `Box` to the resolver's type-only predeclarations and keep the ordinary shadowing order (a user declaration named `Box` wins) — `compiler/src/resolve/mod.rs`, `compiler/src/types/table.rs`
- [x] 1.2.6 Add the `E10300` arity check in `validate_type_at` and the `match_param`/`mentions_unbound` recursion for `boxWith` inference — `compiler/src/sema/{items,call}.rs`
- [x] 1.2.7 Add nullable/overlay/closure/view walks (`E10227`), construction rejection (`E10203`/`E10201`), and the raw-cast gate (`E10201`) — `compiler/src/sema/{expr,items,mod}.rs`
- [x] 1.2.8 Add `mentions_box` to the freestanding profile check (`E11200`) — `compiler/src/sema/mod.rs`
- [x] 1.2.9 Add the `E10105` FFI check: pass `&sema.table` and each block's module to `check_items`, resolve with `resolve_ty_folded`, and recurse through arrays, raw pointers, and function-pointer types — `compiler/src/ffi.rs`, `compiler/src/own.rs`
- [x] 1.2.10 Add the `own/generics.rs` `is_concrete`/`record_struct_type` arms and the `Box` arms in `own/place.rs`, `own/expr.rs`, `own/statements.rs`, `own/instance_check.rs` — `compiler/src/own/`
- [x] 1.2.11 Add the `E10227`/`E10105` registry entries and the two compile-fail files; run `./spec/check.sh` — `spec/14-diagnostics.md`, `spec/tests/compile-fail/`

### Step 1.3: Impl tests and verify

**Reference**: [Testing Strategy](07-testing-strategy.md) BX-10..BX-17 · AR-6, AR-66, AR-68
**Objective**: Pin the type keys, display, layout, FFI rejection, and generic
instance recording.

- [x] 1.3.1 [impl-author] Write the type-model impl pins: `Box__<key>`, `Box<T>` display, `sizeof(Box<i32>)`, a type-only `sizeof(StructWithBox)`/`offsetof`/extent (no Box value is constructed), `Box<Wrap<i32>>` instance emission, and the absent `ClWrap` prototype — `compiler/tests/v11_language_surfaces_impl_test.rs`
- [x] 1.3.2 Run the focused suites and `cargo test --workspace`; fix the plumbing
- [x] 1.3.3 Run `make check`; run the correctness + emission-integrity review for the type-model phase and record findings in `05-findings.md`; commit the type-model task ✅ (completed: 2026-11-08)

**Deliverables**:
- [ ] `TyKind::Box` accepted by every stage
- [ ] E10227/E10105 registered and tested
- [ ] All Phase 1 cases green; `make check` exit 0

**Verify**: focused suites, `cargo test --workspace`, `make check`.

---

## Phase 2: Box Allocation, API, and Drop Glue

> **Phase baseline tree**: after Phase 1
> **Reasoning**: high — the recursive drop is the memory-safety core; a wrong
> arm leaks, double-frees, or emits invalid C.
> **Lenses**: memory safety · emission integrity · compiler semantics
> **Verify**: `cargo test --test v11_language_surfaces_spec_test` under
> `RDC_ASAN=1`; `make check`; `make asan`.

### Step 2.1: Behavior tests and red evidence

**Reference**: R1.3, R2, R3, R4, R5 · [Testing Strategy](07-testing-strategy.md) BX-1..BX-23 · AR-8, AR-9, AR-11, AR-12, AR-22, AR-24
**Objective**: Pin construction, replacement, access, nullability, moves,
recursive drop, reassignment, loops, defer, mutual recursion, hosted-amiga,
and m68k size.

- [x] 2.1.1 [spec-author] Write the Box behavior cases and the ASan memory helper cases: BX-1..BX-9, BX-18..BX-23, including `b.member`/`b[i]`/`b()`, the conditional/return exactly-once case, `boxSet`, reassignment, loop, defer, null, move, `boxWith` lambda, and view escape — `compiler/tests/v11_language_surfaces_spec_test.rs`
- [x] 2.1.2 Add a hosted-amiga Box build/run case and an m68k `sizeof(Box<i32>)`/struct-layout check to the Box suite
- [x] 2.1.3 Record the red run: the API and type are absent; capture the outputs

### Step 2.2: Allocation, API, and drop glue

**Reference**: [Box Design](03-01-box.md) §3-§5 · AR-8, AR-9, AR-14, AR-22, AR-24, AR-24b, AR-24c, AR-63
**Objective**: Provide the allocator, the stdlib API, and the recursive drop
glue without adding a new dependency.

- [x] 2.2.1 Add `cl_box_alloc` (zeroed, trapping) and `cl_box_free` (null-safe) to the host and hosted-amiga shims — `compiler/runtime/shim_host.c`, `compiler/runtime/shim_hosted_amiga.c`
- [x] 2.2.2 Add the `boxAlloc` intrinsic declaration to `std.mem`, `COMPILER_INTRINSICS`, the free/qualified sema branches, the `own/call.rs` lowering, the `codegen/calls.rs` emission arm, and the `cl_box_alloc__<key>` helper definition — `stdlib/std.mem.clo`, `compiler/src/stdlib.rs`, `compiler/src/sema/call.rs`, `compiler/src/own/call.rs`, `compiler/src/codegen/{calls,program}.rs`
- [x] 2.2.3 Add `boxNew<T>`, `boxSet<T>`, `boxWith<T, F>`, and `boxPtr<T>` to `std.mem`; add `boxNew`/`boxSet`/`boxWith`/`Box` to the export table; add `Box` to `COMPILER_KNOWN_EXPORTS` — `stdlib/std.mem.clo`, `compiler/src/stdlib.rs`
- [x] 2.2.4 Collect Box element types transitively, emit the `static inline` prototypes and `cl_box_drop__<key>` definitions, add the drop arm and expression form, and replace the Phase 1 drop placeholder; keep `is_pointer_param` false for Box — `compiler/src/codegen/{program,drop,function,types}.rs`
- [ ] 2.2.5 Audit substituted raw casts: reject a `raw T(x)` whose operand resolves to a Box outside embedded `std.mem` in each function instance; add explicit and inferred generic cases — `compiler/src/own/instance_check.rs`, `compiler/tests/v11_language_surfaces_spec_test.rs`

### Step 2.3: Impl tests, review, and verify

**Reference**: [Testing Strategy](07-testing-strategy.md) BX-1..BX-23 · AR-22
**Objective**: Pin the helper, prototype, and drop-site shapes and prove the
memory paths.

- [x] 2.3.1 [impl-author] Write the Box impl pins: helper names, prototypes before definitions, drop-site call, reassignment release, plain `void *` parameter, no `uint32_t`, no heap environment — `compiler/tests/v11_language_surfaces_impl_test.rs`
- [x] 2.3.2 Run the focused Box spec/impl suite under `RDC_ASAN=1`; run `make check`
- [x] 2.3.3 Run the correctness + memory-safety phase review for the Box drop paths; record findings in `05-findings.md`
- [x] 2.3.4 Run `make asan`; commit the Box task

**Deliverables**:
- [ ] Box allocates, moves, replaces, drops recursively, and is ASan-clean
- [ ] Mutual recursion, reassignment, hosted-amiga, and m68k layout are covered
- [ ] The phase review is recorded

**Verify**: `RDC_ASAN=1` focused suite, `make check`, `make asan`.

---

## Phase 3: X1 Ordinary `freedBy` Deleters

> **Phase baseline tree**: after Phase 2
> **Reasoning**: high — the deleter identity drives release; a wrong symbol or
> a second declaration is a double free or leak.
> **Lenses**: memory safety · emission integrity · compiler semantics
> **Verify**: `cargo test --test v11_language_surfaces_spec_test` under
> `RDC_ASAN=1`; `make check`.

### Step 3.1: Behavior tests and red evidence

**Reference**: R7-R9 · [Testing Strategy](07-testing-strategy.md) X1-1..X1-16 · AR-29..AR-36, AR-67
**Objective**: Pin acceptance, rejection, module-scoped resolution, aliases,
cross-module duplicates, owned-field deletion, forwarding deleters, and
move-only semantics.

- [x] 3.1.1 [spec-author] Write the X1 cases: X1-1..X1-16, including the `ffi` regression, unresolved name, E10104, forwarding `ffi` release, exported, cross-module simple via import, aliased parameter, foreign same-named type, cross-module duplicate, all wrong signatures, intrinsic, owned-field deleter, and move-only — `compiler/tests/v11_language_surfaces_spec_test.rs`
- [x] 3.1.2 Record the red run: a valid ordinary deleter is E10102 and two cross-module handles are accepted

### Step 3.2: Resolution, identity, and emission

**Reference**: [FreedBy Design](03-02-freedby.md) §2-§4 · AR-31, AR-33, AR-67
**Objective**: Resolve the deleter once, store its identity where the emitter
can read it, and use it in both the validator and the emitter.

- [x] 3.2.1 Add handle origins and report cross-module duplicates with `E11101`; add `module` and a resolved `(module, simple name, kind)` deleter to `HandleInfo`, and add a `deleter_identities` map to `OwnershipOutput` — `compiler/src/types/table.rs`, `compiler/src/sema/mod.rs`, `compiler/src/own/{program.rs,..}`
- [x] 3.2.2 Implement module-scoped deleter resolution (own module, imports, a counted unique program-wide candidate) and the resolved-`Ty` signature comparison; keep the `ffi` path — `compiler/src/ffi.rs`, `compiler/src/own/program.rs`
- [x] 3.2.3 Use the stored identity in `deleter_symbol`; remove the dotted-name `function_symbol` call and the caller-scope fallback; keep `String` → `FreeString` — `compiler/src/codegen/{drop.rs,..}`, `compiler/src/own.rs`

### Step 3.3: Impl tests, review, and verify

**Reference**: [Testing Strategy](07-testing-strategy.md) X1-1..X1-16 · AR-31
**Objective**: Pin the symbols and prove the owned-field case.

- [x] 3.3.1 [impl-author] Write the X1 impl pins: `cl_<module>_<name>` for a cross-module ordinary deleter, the C name for `ffi`, no `cl_A_B_A_B_*` — `compiler/tests/v11_language_surfaces_impl_test.rs`
- [x] 3.3.2 Run the focused X1 suite under `RDC_ASAN=1`; run `make check`
- [ ] 3.3.3 Run the correctness + memory-safety phase review for the X1 release path; record findings
- [ ] 3.3.4 Commit the X1 task

**Deliverables**:
- [ ] Ordinary deleters work with the correct diagnostics
- [ ] One deleter per type, cross-module included
- [ ] Owned-field deletion is ASan-clean
- [ ] The phase review is recorded

**Verify**: `RDC_ASAN=1` focused suite, `make check`.

---

## Phase 4: L5 Hook Contract and Rejection

> **Phase baseline tree**: after Phase 3
> **Reasoning**: high — a mis-accepted capturing closure emits invalid C and a
> dangling environment pointer.
> **Lenses**: compiler semantics · FFI ABI · memory safety
> **Verify**: `cargo test --test v11_language_surfaces_spec_test`; `make check`.

### Step 4.1: Behavior tests and red evidence

**Reference**: R10, R12 · [Testing Strategy](07-testing-strategy.md) HK-1, HK-3..HK-5, HK-12..HK-16 · AR-37, AR-62, AR-65
**Objective**: Pin the `borrow` marker, the ABI-identical signature set, the
concrete-`func` rejection, the generic-bound carve-out, and the generic-body
restriction.

- [x] 4.1.1 [spec-author] Write the hook rejection/acceptance cases: capture-less accept; capturing at `borrow` still `E10400` in this phase; unmarked/`const`/`take` reject; user func reject; stored APIs reject; view/borrow/fixed-array signature reject; mismatch; non-callable; generic bound accept; generic body reject; short-circuit; `[borrow x]` capture — `compiler/tests/v11_language_surfaces_spec_test.rs` ✅ (completed: 2026-11-08 08:38)
- [x] 4.1.2 Record the red run: a capturing lambda at an unmarked ffi callback is accepted and emits `ClClosure_N`; a view callback is accepted ✅ (completed: 2026-11-08 08:38; 9 intended failures, recorded in `05-findings.md` §3.4)

### Step 4.2: Sema and record

**Reference**: [Hook Design](03-03-hook-trampolines.md) §2 · AR-39, AR-62, AR-65
**Objective**: Add the function-type branch and the ABI-identical check; keep
all capturing `func` arguments rejected until the emission path exists.

- [x] 4.2.1 Thread the call-node address from `check_expr` into `check_ffi_arguments`; convert `ffi_parameter_types` and both ffi call paths to `(Ty, Mode)` — `compiler/src/sema/{expr,call}.rs` ✅ (completed: 2026-11-08 08:38)
- [x] 4.2.2 Add the `Func` argument branch: signature match, `E10400` for every capturing closure (including `borrow` until Phase 5), `E10210` for the ABI set, `E10200` for non-callables — `compiler/src/sema/call.rs` ✅ (completed: 2026-11-08 08:38)
- [~] 4.2.3 Run the focused suite and `make check`; commit the hook-rejection task (started: 2026-11-08 08:38)

**Deliverables**:
- [ ] The base invalid-C capture is now `E10400`
- [ ] The ABI-identical accepted set is rejected/accepted correctly
- [ ] No capturing `borrow` hook is accepted before emission exists

**Verify**: focused hook sema suite, `make check`.

---

## Phase 5: L5 Acceptance, Trampoline, and Wrapper

> **Phase baseline tree**: after Phase 4
> **Reasoning**: max — a new C callback ABI path with stack lifetimes,
> reentrancy, and evaluation-order constraints.
> **Lenses**: memory safety · emission integrity · FFI ABI · compiler semantics
> **Verify**: host fixture under `RDC_ASAN=1`; `make check`; m68k cross-build/
> `vamos`; `make verify` at the end.

### Step 5.1: Host behavior tests and red evidence

**Reference**: R11-R12 · [Testing Strategy](07-testing-strategy.md) HK-2, HK-6..HK-11 · AR-37a, AR-41..AR-47, AR-53
**Objective**: Pin the synchronous call, two callbacks, nesting, reentrancy,
repeat invocation, evaluation order, and the after-return fault.

- [ ] 5.1.1 Add a shared `compiler/tests/support/mod.rs` with the scratch-dir, compiler-run, extra-C-source host build, and `vamos` helpers; include it from both v1.1 suites
- [ ] 5.1.2 [spec-author] Write the host fixture and cases: `cl_fixture_apply`, `apply2`, `repeat`, `reenter`, `defer`; the effectful-sibling order case; the `[borrow x]` write; the short-circuit case; the after-return `E11215` case — `compiler/tests/fixtures/hook_fixture_host.c`, `compiler/tests/v11_language_surfaces_spec_test.rs`
- [ ] 5.1.3 Record the red evidence: the base `ClClosure_N` C mismatch; the order bug via the generic-bound proxy (`hook(bump(n), [n] ...)`); the missing wrapper/trampoline/env symbols

### Step 5.2: Lowering and acceptance

**Reference**: [Hook Design](03-03-hook-trampolines.md) §3 · AR-37a, AR-40, AR-44
**Objective**: Materialize the closure at its argument position, build the
wrapper call, and enable the `borrow` acceptance only now.

- [ ] 5.2.1 Add the hook argument set to `lower_call`; materialize the closure into a statement temporary at its source position; replace the callback arg with the trampoline path; append the closure temporaries — `compiler/src/own/{call,lower}.rs`
- [ ] 5.2.2 Build the `HookWrapper` record, rename the synthetic call, keep the original modes for `take` moves, and leave the extra closure args beyond `modes` — `compiler/src/own/call.rs`, `compiler/src/own.rs`
- [ ] 5.2.3 Enable the `borrow` hook acceptance and the `HookCall` record in `check_ffi_arguments`; keep the ABI-identical check — `compiler/src/sema/call.rs`, `compiler/src/sema/mod.rs`

### Step 5.3: Emission

**Reference**: [Hook Design](03-03-hook-trampolines.md) §4 · AR-41..AR-47
**Objective**: Emit the environment stack, trampolines, wrappers, and the
ABI-aware wrapper call.

- [ ] 5.3.1 Emit the `ClHookEnvNode` linked stack and `push`/`pop`/`find` once when a wrapper exists — `compiler/src/codegen/program.rs`
- [ ] 5.3.2 Emit one trampoline per hook argument with the exact declared signature and the `E11215` null-env check — `compiler/src/codegen/program.rs`
- [ ] 5.3.3 Emit one wrapper per call site with ABI-typed parameters, reverse pops, and a void/return form — `compiler/src/codegen/program.rs`
- [ ] 5.3.4 Add the named ABI declarator helper and the wrapper-record lookup in call rendering (original ffi ABI + `(<temp>).env` extras) — `compiler/src/abi.rs`, `compiler/src/codegen/calls.rs`
- [ ] 5.3.5 Register `E11215` in the runtime-fault block and add the exclusion to `spec/check.sh`; add the Rust spec case (the `run` suite cannot link the fixture) — `spec/14-diagnostics.md`, `spec/check.sh`, `compiler/tests/v11_language_surfaces_spec_test.rs`

### Step 5.4: m68k, impl tests, review, and verify

**Reference**: [Hook Design](03-03-hook-trampolines.md) §7 · AR-53
**Objective**: Prove the ABI on m68k, pin the emitted shapes, and review the
language-rule surface.

- [ ] 5.4.1 Add the m68k fixture (`LONG`/`UBYTE` from `<exec/types.h>`) and the `m68k-amigaos-gcc`/`vamos` gate; skip with a note when the toolchain is absent — `compiler/tests/fixtures/hook_fixture_m68k.c`, `compiler/tests/v11_language_surfaces_impl_test.rs`
- [ ] 5.4.2 [impl-author] Write the hook impl pins: a capture-less lambda is a plain pointer with no env/trampoline/push; a capturing hook has the env stack, one trampoline, one wrapper, no heap allocation; the closure temp order — `compiler/tests/v11_language_surfaces_impl_test.rs`
- [ ] 5.4.3 Run the focused hook suite under `RDC_ASAN=1`; run `make check`
- [ ] 5.4.4 Run the correctness + memory-safety + emission-integrity + semantics phase review for the hook surface (with the `spec/17-codegen` ABI lens); record findings
- [ ] 5.4.5 Commit the hook task

**Deliverables**:
- [ ] Capturing lambdas cross the synchronous C hook with a stack environment
- [ ] Nesting, reentrancy, two callbacks, and the after-return fault work
- [ ] The m68k ABI is proven
- [ ] The phase review is recorded

**Verify**: `RDC_ASAN=1` focused suite, `make check`, m68k cross-build/`vamos`.

---

## Phase 6: Specification Amendments and Docs

> **Phase baseline tree**: after Phase 5
> **Reasoning**: medium — the amendments must match the approved goal and the
> implemented behavior exactly.
> **Lenses**: compiler semantics · consistency
> **Verify**: `./spec/check.sh`, `make check`.

### Step 6.1: Amendments and docs

**Reference**: R13 · [Requirements](01-requirements.md) §R13 · AR-2, AR-5, AR-55
**Objective**: Record the approved amendments and the new diagnostics.

- [ ] 6.1.1 Add the v1.1 decision section to `spec/decisions.md` (X1, L5, P7, Box, profiles, diagnostics) and update the affected chapters: Ch 01 (type-name predeclaration and shadowing), Ch 02 (type universe/nullability/casts), Ch 03 (ownership/drop), Ch 06 §6.1, Ch 07 §5/§10, Ch 10 (module-scoped deleter resolution, if applicable), Ch 11 (heap allocation), Ch 12 §2/§4/§7, Ch 14, Ch 15 (profiles), Ch 16 §2/§4, Ch 17, appendix-stdlib §12 and a new Box/std.mem section, lineage — `spec/`
- [ ] 6.1.2 Update the `E10102`/`E10400` help text and keep their first lines stable; confirm the Phase 1 compile-fail tests and the Phase 5 `E11215` exclusion pass — `spec/14-diagnostics.md`, `spec/tests/compile-fail/`, `spec/check.sh`
- [ ] 6.1.3 Update `docs/known-limitations.md` (no nullable allocation failure, no move-out, no Box FFI, no freestanding Box, no heap env, synchronous-only, non-generic hooks, ABI set), the appendix implementation table, the roadmap stage/row, and AGENTS if a new suite/target needs a line — `docs/`, `plans/00-roadmap.md`, `AGENTS.md`
- [ ] 6.1.4 Run `./spec/check.sh` and `make check`; commit the specification-amendment docs separately from the implementation commits

**Deliverables**:
- [ ] Every approved amendment is in the ledger and the affected chapter
- [ ] The new diagnostics are registered and tested
- [ ] The limitations are documented

**Verify**: `./spec/check.sh`, `make check`.

---

## Phase 7: Full Acceptance, Reviews, and Push

> **Phase baseline tree**: after Phase 6
> **Reasoning**: high — the final gate is the release decision.
> **Lenses**: all four reviewer roles · verification quality
> **Verify**: every gate below; `verify: all legs passed`.

### Step 7.1: Acceptance

**Reference**: R13.4 · AR-58, AR-59
**Objective**: Run every acceptance leg, review the change surface, and push
only the green tree.

- [ ] 7.1.1 Run `make check` on the final tree
- [ ] 7.1.2 Run `make asan`
- [ ] 7.1.3 Run `make ubsan`
- [ ] 7.1.4 Run `make corpus-amiga`, `make differential`, and `make bench-shape`
- [ ] 7.1.5 Run `make verify` and confirm `verify: all legs passed`
- [ ] 7.1.6 Run the final cross-phase reviews: correctness + memory-safety for the Box/X1 drop paths; emission-integrity for the Box type/codegen and hook emission; semantics for the X1/L5/Box language rules; record every finding in `05-findings.md`
- [ ] 7.1.7 Fix every critical/major review finding, re-run the affected gates and the full acceptance, and record the re-gate and the authorized ruling
- [ ] 7.1.8 Push the verified tree with the git-commit skill in push mode; confirm `git status -sb` is clean and `git rev-list --count origin/main..HEAD` is 0

**Deliverables**:
- [ ] `verify: all legs passed`
- [ ] All review findings resolved
- [ ] `origin/main` up to date, clean tree

**Verify**: the seven gates above.

---

## Coverage Matrix

| Requirement | Tasks |
|---|---|
| R1 Box type/API | 1.1.1-1.3.3, 2.2.2-2.3.2 |
| R2 allocation/profiles | 2.2.1, 2.2.3, 2.1.2, 1.2.8 |
| R3 ownership/moves/nullability | 1.2.3, 2.1.1, 2.2.4, 2.3.1 |
| R4 recursive drop | 2.1.1, 2.2.4, 2.3.1-2.3.3 |
| R5 access/views/generics | 1.1.1, 1.2.6, 1.2.7, 2.1.1 |
| R6 FFI/diagnostics | 1.1.1, 1.2.9, 1.2.11 |
| R7 X1 acceptance | 3.1.1, 3.2.1-3.2.2 |
| R8 X1 rejection | 3.1.1, 3.2.2 |
| R9 X1 emission/ownership | 3.2.1-3.2.3, 3.3.1-3.3.3 |
| R10 L5 contract/acceptance | 4.1.1-4.2.3, 5.2.3 |
| R11 trampoline/environment | 5.1.1-5.4.5 |
| R12 hook errors/fault | 4.1.1, 5.3.5 |
| R13 spec/tests/gates/docs | 1.2.11, 5.3.5, 6.1.1-6.1.4, 7.1.1-7.1.7 |

## Dependencies

```text
0.1.1-0.1.4 baseline ──> 1.1.1/1.1.2 type tests ──> 1.2.1-1.2.11 type model ──> 1.3.1-1.3.3 verify/review
1.3.3 ──> 2.1.1-2.1.3 Box tests ──> 2.2.1-2.2.4 allocator/API/drop ──> 2.3.1-2.3.4 verify/review
2.3.4 ──> 3.1.1/3.1.2 X1 tests ──> 3.2.1-3.2.3 resolution/identity/emission ──> 3.3.1-3.3.4 verify/review
3.3.4 ──> 4.1.1/4.1.2 hook tests ──> 4.2.1-4.2.3 rejection
4.2.3 ──> 5.1.1-5.1.3 host tests ──> 5.2.1-5.2.3 lowering/acceptance ──> 5.3.1-5.3.5 emission
5.3.5 ──> 5.4.1-5.4.5 m68k/impl/review ──> 6.1.1-6.1.4 spec/docs
6.1.4 ──> 7.1.1-7.1.5 acceptance ──> 7.1.6-7.1.7 reviews/re-gate ──> 7.1.8 push
```

## Success Criteria

**Feature is complete when:**

1. ✅ `Box<T>` allocates, moves, replaces, drops recursively, and is ASan-clean
2. ✅ Ordinary `freedBy` deleters work with the correct diagnostics; one per type
3. ✅ Capturing lambdas cross synchronous C hooks through stack trampolines; no heap environment
4. ✅ The approved spec amendments are recorded in the ledger and chapters
5. ✅ `make check`, `make asan`, `make ubsan`, `make corpus-amiga`, `make differential`, `make bench-shape`, and `make verify` are green
6. ✅ No existing golden changed; the two new compile-fail tests exist, `E11215` is registered and excluded from the compile-fail coverage, and its Rust spec case exists
7. ✅ Every review finding is resolved
8. ✅ The working tree is clean and `origin/main` is up to date
