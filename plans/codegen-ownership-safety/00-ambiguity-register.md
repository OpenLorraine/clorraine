# Ambiguity Register: Codegen Ownership Safety (DEF-13, DEF-14)

> **Status**: ✅ GATE PASSED — all 16 items resolved
> **Last Updated**: 2026-09-28 08:31
> **CodeOps Artifact Schema**: 1

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | Which defects this plan fixes | DEF-13 + DEF-14 only / also the defer-temporary leak / also FM-008 | DEF-13 + DEF-14 only; the leak and FM-008 stay filed outside this plan | ✅ Resolved |
| 2 | Technical | DEF-13 fix design: how a `for (borrow e in xs)` element aliases the array element | element alias via a per-iteration `raw` temp / patch the copy-back cache protocol / C pointer alias local | Element alias via a per-iteration `raw` temp: reads and writes go through the deref, O3 replacement applies to the element, and the copy-in/write-back machinery is deleted | ✅ Resolved |
| 3 | Technical | DEF-14 fix design: how shadowed locals get distinct identity | unique IR spelling per binding / binding-id-keyed IR locals | Unique IR spelling per binding (source names stay for diagnostics); codegen's name-keyed maps work unchanged | ✅ Resolved |
| 4 | Naming | Plan folder name and ceremony | `codegen-ownership-safety` / `view-alias-and-shadowing` / two separate plans | `plans/codegen-ownership-safety/`, full 00–07 + 99 document set | ✅ Resolved |
| 5 | Verification | Task gates and final gate | `make check` per task + `make verify` final / check-only | `make check` per task, `make verify` (all hosted-amiga legs) before completion | ✅ Resolved |
| 6 | Behavioral | Semantics of assigning an owned value through a borrow element (`e = v` where the element owns memory) | Option A: drop the element's old value, then store the new one / Option B: overwrite without dropping | Spec-derived: `spec/16-borrow-rules.md` §2 (element views write through) + `spec/03-ownership.md` O3 (assignment drops the live old value first); Option A | ✅ Resolved |
| 7 | Behavioral | Which loop exits and how observe a write through the element (fallthrough, `break`, `continue`, `return`, deferred cleanup) | Option A: the element is the single source of truth, all exits see the write directly / Option B: cache plus write-back on every exit | Option A: the binding aliases the element for the whole iteration; no write-back step exists | ✅ Resolved |
| 8 | Behavioral | Behavior of non-`borrow` element loops (`for (x in xs)`) | Option A: unchanged per-iteration copy / Option B: alias as well | Option A: unchanged; only `borrow` elements alias | ✅ Resolved |
| 9 | Edge cases | Which shadowing forms the fix must cover | all MOD-16 forms (local over local, local over parameter, nested blocks, sibling scopes, defer-machinery locals, const locals) / only the reproduced local-over-local case | All MOD-16 forms: local over local (nested and sibling scopes), local over parameter, const locals, and defer-machinery locals; extended in preflight to include generic instantiations and callable-bound lambda bodies | ✅ Resolved |
| 10 | Scope | `for (row in m)` over a multi-extent array (`T[2][3]`) binds the base element, not the row — an observed defect with a different root cause (`compiler/src/own/loops.rs:404`, `sema/stmt.rs:200`) | exclude from this plan and file a roadmap DEF row / exclude only / include in this plan | Exclude from this plan and file a roadmap DEF row (`DEF-15`) | ✅ Resolved |
| 11 | Naming & terminology | Internal names the plan will create: the deref place variant and the unique IR spelling scheme | `PlaceBase::DerefTemp { temp, ty }` plus `name` kept for the first binding and `name#<binding-index>` for collisions / other | `PlaceBase::DerefTemp { temp, ty }`; first binding keeps the source spelling, collisions use `name#<binding-index>` | ✅ Resolved |
| 12 | Naming & terminology (testing) | Where the new specification and implementation tests live | new per-plan suites `compiler/tests/codegen_ownership_spec_test.rs` and `codegen_ownership_impl_test.rs` / extend `codegen_run_spec_test.rs` and `partial_move_impl_test.rs` | New per-plan suites `compiler/tests/codegen_ownership_spec_test.rs` and `codegen_ownership_impl_test.rs` | ✅ Resolved |
| 13 | Integration points | The element-alias fix changes emitted C for every borrow-element loop on every profile and toolchain | single option: covered by the approved verification decision (AR #5) | All profiles are covered by `make check` + `make verify`; no per-profile exception | ✅ Resolved |
| 14 | Non-functional | IR names and emitted C must stay deterministic across speculative loop passes, generic instances, and lambda lifting | single option: deterministic name allocation from the binding table, covered by determinism tests | Design requirement; no user choice needed | ✅ Resolved |
| 15 | Security & compliance | Threat surface | none: compiler-internal correctness, no user input, no secrets, no network or storage | Not applicable | ✅ Resolved |
| 16 | UX & presentation | User-visible diagnostics | single option: diagnostics keep source spelling via the existing binding names; no message changes | No user-visible change | ✅ Resolved |

### Resolution Notes

**AR-1:** The defer-temporary leak remains a filed minor (missing drop in the registration-lowered defer block) and FM-008 (the implementation-test oracle does not run ASan) stays accepted; neither is in this plan.

**AR-2:** Element alias via a per-iteration `raw` temp. The challenger's blind recommendation was adopted after verification: `TyKind::Raw` renders as a typed C pointer (`compiler/src/codegen/types.rs:26`), compiler temporaries are already declared and initialized (`compiler/src/codegen/function.rs:248`), and `Binding.base` already provides an explicit place-base override (`compiler/src/own/flow.rs:62`). The fix deletes `ForInWrite`, `set_for_in_write`, `store_for_in_element`, and the `break`/`continue` write-back hooks.

**AR-3:** `Binding` gains a unique IR spelling (recommended `name` for the first binding, `name#<binding-index>` on collision, assigned from the binding table so speculative passes stay deterministic); `binding_place` and the `Function.locals` builders use it, and `binding_names` keeps source names for diagnostics. Verified defect: the spec's own MOD-16 example emits C that reads an uninitialized renamed local and fails `-Werror`.

**AR-5:** `make check` runs `./spec/check.sh` plus `cargo test --workspace` in the container; `make verify` adds every hosted-amiga cross-toolchain leg. Host-only ASan probes remain manual evidence (the container lacks the ASan runtime).

**AR-6/7:** Spec anchors: element views write through to the owner (`spec/16-borrow-rules.md` §2, LOOP-2 in `spec/08-arrays-strings.md` §5), and assignment drops the live old value first (`spec/03-ownership.md` O3).

**AR-9:** The fix covers every shadowing form the specification names in MOD-15/MOD-16: a local shadowing a local in a nested or sibling scope, a local shadowing a parameter (the specification's own example), `const` locals, and locals declared by the defer machinery. Extended during preflight review to name generic instantiations and callable-bound lambda bodies, which make the same name-collision machinery observable in generated instances.

**AR-10:** Multi-extent element loops bind the base element instead of the sub-array row. This has a different root cause from DEF-13, so it stays out of the implementation and is filed as `DEF-15` on the roadmap for a dedicated plan.

**AR-11:** New IR place variant `PlaceBase::DerefTemp { temp, ty }`. Unique IR spelling: the first binding keeps its source name, colliding bindings use `name#<binding-index>`, assigned from the binding table for determinism. The table is not truncated when a scope closes (`compiler/src/own.rs:356`), so every binding declared so far in the function counts and sibling scopes get distinct spellings. The C emitter's `sanitize_identifier` plus `unique_local` still dedups the final C names.

**AR-12:** New suites `compiler/tests/codegen_ownership_spec_test.rs` (specification cases) and `compiler/tests/codegen_ownership_impl_test.rs` (edges and internals), matching the recent per-feature test-file convention; `AGENTS.md` gains both entries.

### Execution closure (2026-09-28)

All 16 decisions held through execution. AR-2/AR-7 landed as designed; AR-3/AR-9
gained the initializer-resolution rule confirmed by ST-16 (a shadowing
initializer resolves to the outer binding) and the Phase 2 review; AR-5's
verification ran in full (`make check` per task, host ASan probes at close-out,
`make verify: all legs passed`). Separate defects found while executing were
filed outside this plan: `DEF-16` – `DEF-20`.
