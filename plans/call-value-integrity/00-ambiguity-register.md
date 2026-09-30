# Ambiguity Register: Call Value Integrity

> **Status**: ✅ GATE PASSED — all 19 items resolved
> **Last Updated**: 2026-09-30 07:56

Systematic review across all 12 categories for the `call-value-integrity` plan (defects
DEF-30 – DEF-45). Rows 1–6 are pre-resolved by the scope interview; rows 7–19 were surfaced
during the current-state reconnaissance and are presented to the user in two batches.

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | Plan shape for the 16 defects | One full plan (all 16) / two plans by family / mini-plan per defect | One full plan, all 16, phased by cluster | ✅ Resolved |
| 2 | Technical | DEF-34 + DEF-45 mechanism | Branch store + transfer-aware release / branch store only / defer both | Branch store for scalar owned conditionals; transfer temps join the release set and adoption moves out (zeroes) the temp | ✅ Resolved |
| 3 | Behavioral | Move-only element arms (`ok ? a[0] : a[1]`) | Accept as part of DEF-34 / keep the spec E10026 rejection | Keep the spec E10026 rejection (`spec/03-ownership.md:126-128`); add a regression pin | ✅ Resolved |
| 4 | Technical | DEF-44 dropped `borrow` mode in func types | Implement borrow / reject with a diagnostic / defer | Implement borrow (grammar sanctions `function_type_param = [param_mode] value_type`) | ✅ Resolved |
| 5 | Naming | Regression-test placement | New `call_value_integrity_*` suites / extend `runtime_value_integrity_*` | New suites; the closed plan's suites stay frozen | ✅ Resolved |
| 6 | Scope | Baseline, name, phase order | Confirm / adjust | Confirmed: `call-value-integrity`; phases 1 scalar ownership, 2 fixed-array values, 3 func pointers, 4 guards/cleanup, 5 close-out; in/out of scope as presented | ✅ Resolved |
| 7 | Technical | DEF-38 struct-literal array field emission | (a) Zero-init struct then per-field store; array fields through the fixed-array assign machinery / (b) materialize the value into a temp then memcpy | User accepted recommendation: zero-init struct then per-field store; array fields through the fixed-array assign machinery | ✅ Resolved |
| 8 | Technical | DEF-37 do-while condition evaluation | (a) Emit an explicit `goto` to the condition block instead of `continue` when the fall-through carries guards / (b) restructure the condition block | Revised after preflight PF-001: the goto conflicts with `spec/17-codegen.md:24` (goto for cleanup ladders only), so the user re-ruled for the goto-free flag-first restructure (`00-preflight-report.md`) | ✅ Resolved |
| 9 | Technical | DEF-39 growable-array drop in embedded guards | (a) Add the `drop_expr_text` form for growable `Array<T>` / (b) force the statement spelling when no drop expression exists | Revised after preflight PF-010: the statement path is an inline loop, not a runtime call, so the expression form covers only element types that need no drop; drop-needing element types keep the skip (recorded in Won't Have) | ✅ Resolved |
| 10 | Technical | DEF-36 array-parameter zeroing | (a) Element walk for by-reference array parameters; `cl_zero` unchanged for true locals / (b) other | User accepted recommendation: element-walk zeroing for by-reference array parameters; `cl_zero` unchanged for true locals | ✅ Resolved |
| 11 | Technical | DEF-32 / DEF-33 duplicable fixed-array materialization | (a) Materialize a duplicable fixed-array value whenever the C position has no place, removing the duplicable skip at `own/lower.rs:953` where a place is required / (b) special-case each position | User accepted recommendation: materialize a duplicable fixed array whenever the C position needs a place (gate the `own/lower.rs:953` skip on that condition) | ✅ Resolved |
| 12 | Technical | DEF-42 widening source for func-value callees | (a) Read parameter types/modes from the callee expression's `TyKind::Func`; declared-function and FFI paths unchanged / (b) infer differently | User accepted recommendation: read parameter types/modes from the callee's `TyKind::Func`; declared-function and FFI paths unchanged | ✅ Resolved |
| 13 | Scope | DEF-31 constant-index sibling (`rows[0] = a`) and DEF-35 call-argument path | Include (within the filed defect titles) / exclude as separate findings | User accepted recommendation: include the DEF-31 constant-index sibling and the DEF-35 call-argument path | ✅ Resolved |
| 14 | Technical | DEF-44 `FuncTy` parameter-mode representation | (a) A `FuncParam { ty, mode }` element / (b) a parallel mode vector | User accepted recommendation: carry the mode as a `FuncParam { ty, mode }` element in `FuncTy` | ✅ Resolved |
| 15 | Technical | Closed-plan impl/unit pins describing the old transfer behavior (`runtime_value_integrity_impl_test.rs:171`, `own/tests.rs:879`) | (a) Update the impl/unit pins in place as part of the fix; spec suites untouched / (b) leave them and suppress | User accepted recommendation: update the old impl/unit pins in place; the frozen spec suites stay untouched | ✅ Resolved |
| 16 | Verification | AddressSanitizer probe set for this plan | (a) ASan-probe the memory-safety defects (DEF-34, 36, 37, 39, 45) in close-out only / (b) probe every defect | User accepted recommendation: ASan probes at close-out for DEF-34, 36, 37, 39, and 45 | ✅ Resolved |
| 17 | Verification | Verify command for every task and the close-out | (a) `make check` per task in the pinned container, `make verify` at close-out / (b) other | User accepted recommendation: `make check` per task in the pinned container; `make verify` at close-out | ✅ Resolved |
| 18 | Scope | Roadmap and defect linkage | (a) The plan owns DEF-30 – DEF-45; roadmap rows move to Executing at start and Done at close, linked to `plans/call-value-integrity/` / (b) other | User accepted recommendation: the plan owns DEF-30 – DEF-45; roadmap rows Executing → Done linked to `plans/call-value-integrity/` | ✅ Resolved |
| 19 | Behavioral | DEF-34 call arms (`ok ? mk() : b`) and by-value argument adoption | (a) Include call arms and by-value ownership transfer in the branch-store design / (b) whole-place arms only | User accepted recommendation: include scalar call arms and by-value argument adoption in the branch-store design | ✅ Resolved |

## Resolution Notes

**AR-2:** The approved mechanism: owned (non-array) conditionals use the branch store (the chosen
arm is moved into the destination per path, with the fixed-array twin's old-target/save and
self-naming rules); a deferred take-call transfer temporary joins the pending release set, and
every adopting site (a binding, an assignment, a `take` argument through a non-releasing argument
temporary, a `return`, or a struct-literal field) moves the value out by zeroing the temporary, so
a discarded path releases exactly once and an adopted path releases nothing.
**AR-3:** Grounded in `spec/03-ownership.md:126-128` ("Nested paths and element moves are E10026 in
v1"); the revision of the earlier answer was recorded after this evidence was presented.
**AR-6:** Confirmed scope baseline: fixes for the 16 filed defects and their named variants; no
spec changes; no new language surfaces beyond the grammar-sanctioned `borrow` func-type mode.

**Preflight revisions (2026-09-30):** PF-001/PF-002 revised AR #8 to the goto-free flag-first
do-while; PF-010 revised AR #9 to the limited growable expression form. PF-003 – PF-007 extended
the adopting-site inventory (return, `take` argument, struct-literal field) and the branch-store
scope (owned aggregates, old-target and self-naming rules); PF-008/PF-023 extended the `FuncTy`
mode work to `FuncSig::ty`, the full `TyKind::Func` consumer list, exact-signature equality, and
the call-site callee type; PF-009, PF-011 – PF-014, and PF-015 – PF-026 amended the designs, tests,
ASan verification, and tracking as recorded in `00-preflight-report.md`.
