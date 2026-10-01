# Preflight Report: Residual Defect Hardening — Iterations 1–2

> **Status**: ✅ PASSED — all 19 findings resolved (1 critical, 8 major, 9 minor, 1 observation); fixes applied and verified by the iteration-2 re-scan
> **Iteration**: 2 (fix verification and bounded re-scan; iteration-1 scan recorded below)
> **Artifact**: implementation plan at `plans/residual-defect-hardening/` — 10 documents, untracked at scan start (repo commit `278f029`)
> **Codebase Grounded**: ~35 source files examined; 47 artifact references verified against the code
> **Auto-design**: active — root invocation ID `pf-rdh-20261001-1`; scope mode **strict** (no `--explore-scope`)
> **Last Updated**: 2026-10-01 14:43

## Audit target and scope

| Item | Value |
| ---- | ----- |
| Audit target | All 10 documents in `plans/residual-defect-hardening/` |
| Context documents (read, not audited) | `plans/call-value-integrity/99-execution-plan.md` (defect filings), `plans/00-roadmap.md:65-68`, `spec/14-diagnostics.md:51`, `AGENTS.md` |
| Product scope baseline | R1–R10 in `01-requirements.md`; approved complexity: none (AR) |
| Modification set | Authorized after the findings were accepted: the audit target's 10 documents plus `plans/00-roadmap.md` (the DEF-50 residual row and the DEF-46–49 stage advance) |
| Scope exploration | Not active (strict): optional additions are not reported |

**Artifact hashes at scan start** (untracked; `git hash-object`):

| File | Blob |
| ---- | ---- |
| 00-index.md | `a64d0251` |
| 00-ambiguity-register.md | `5cc99d54` |
| 01-requirements.md | `d80bb572` |
| 02-current-state.md | `b9b73e56` |
| 03-01-flat-array-literal-types.md | `6dabb8ff` |
| 03-02-multi-extent-return-copy.md | `20833532` |
| 03-03-loop-condition-owned-call-release.md | `6c543537` |
| 03-04-substitution-aware-ownership.md | `356c5bf1` |
| 07-testing-strategy.md | `062f1440` |
| 99-execution-plan.md | `753be21a` |

## Codebase Context Summary

**Repository:** the Clorraine compiler (`clc`), Rust workspace, std-only (`Cargo.toml`, `compiler/Cargo.toml`).
**Architecture:** lex → parse → resolve → sema → own (ownership checker, flow state, guards) → IR → codegen (one C99 TU per profile). Generic bodies are checked once with `TyKind::Param`; codegen emits one copy per recorded instance with a substitution installed (`codegen/program.rs`). `Monomorphization` (`ir/mono.rs`) records instances/structs and `CopyObligation`s; `own/instance_check.rs` re-runs fill/view predicates per instance with `substitute_instance`.
**Key files examined:** `compiler/src/types/{mod,table,tests}.rs`, `compiler/src/sema/{expr,regressions}.rs`, `compiler/src/codegen/{types,guards,function,cleanup,control,drop,program}.rs`, `compiler/src/own/{lower,loops,expr,statements,instance_check,generics,call,call_check,place,flow}.rs`, `compiler/src/ir/{guards,mono}.rs`, `compiler/tests/call_value_integrity_{spec,impl}_test.rs`, `spec/03/04/05/06/08/14`, `AGENTS.md`, `Makefile`, `docker/Dockerfile`.

**Reference verification:** 47 references mapped — 40 verified exactly, 4 verified with corrected line numbers, 3 claimed mechanisms shown not to exist as stated (fold coverage, `type_key` extent walk, "existing discharged obligation pattern").

**Empirical pre-fix observations** (committed `target/debug/clc`, commit `278f029`; scratch files only under `/tmp/opencode`):

| Repro | Observed |
| ----- | -------- |
| `let lit = [[1,2],[3,4]];` | `i32 lit[2]` + `lit[0] = { 1, 2 };` — cc rejects |
| `ok && [c ? rows[0] : rows[1], [5,6]][0][0] == 1` | `i32 __cl_tmp0[2]`, row ternary + brace list — cc rejects |
| `func copy(take a: String[2][2]): String[2][2] { return a; }` | `__cl_ret[0] = a[0];` — cc rejects |
| `while (grow().count > 0 && i < 3)` | call in the header; ASan/LSan: 33 bytes / 3 allocations leaked |
| `fillIt<i32[3]>(a)` on `func fillIt<T>(a: T): T[2]` | `error E10200: cannot convert 'i32[3][2]' to 'i32[3][2]'` |
| `struct Grid<T> { rows: T[2]; }` + `Grid<i32[3]>` | accepted; emits `i32 rows[2]` + pointer stores — cc rejects |
| `Grid<i32[3]>` + `Grid<i32[2]>` in one module | duplicate `struct ClGrid__i32` — cc: redefinition |
| `while (i < 3 && mkArr()[i] != null)`, `mkArr(): String[2]` | already emits `ClString __cl_tmp0[2]` + single materialize inside the guard |
| `observe<T>(borrow a: T)` with `String` | accepted; `cl_observe__String(ClString a)`, no release in the callee |

## Summary by Dimension

| # | Dimension | Findings | Highest severity |
| --- | --- | --- | --- |
| 1 | Ambiguities | 2 | 🟠 MAJOR |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 0 | — |
| 4 | Completeness Gaps | 1 | 🔴 CRITICAL |
| 5 | Dependency Issues | 2 | 🟠 MAJOR |
| 6 | Feasibility Concerns | 1 | 🟠 MAJOR |
| 7 | Testability | 2 | 🟡 MINOR |
| 8 | Security Blind Spots | 2 | 🟠 MAJOR |
| 9 | Edge Cases | 1 | 🟠 MAJOR |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 1 | 🟡 MINOR |
| 12 | Consistency | 4 | 🟡 MINOR |
| 13 | Codebase Alignment | 3 | 🟠 MAJOR |

Domain lens: **compiler and language** (selected per `references/domains/selection.md`); no financial, web, distributed, or data-migration lens applies. Every cluster applied counterexample thinking; several findings were reproduced by compiling probe programs.

## Summary by Severity

| Severity | Count | Status |
| --- | --- | --- |
| 🔴 CRITICAL | 1 | 1 unresolved (delegated fix selected; requires a reserved modification-set ruling) |
| 🟠 MAJOR | 8 | 8 unresolved (5 delegated fixes selected, 3 require reserved rulings) |
| 🟡 MINOR | 9 | delegated resolutions selected; pending artifact edits |
| 🔵 OBSERVATION | 1 | delegated resolution selected |

---

## Findings

### PF-001: Take-argument consumption is missing from the DEF-49 duty set [🔴 CRITICAL]

**Dimension:** Completeness Gaps (also Dependency Issues, Codebase Alignment)
**Location:** `03-04-substitution-aware-ownership.md` §Proposed Changes 1–2 and §The site table; `99-execution-plan.md` Phase 4 expected modification set and tasks 4.2.2/4.2.3
**Codebase Evidence:** `compiler/src/own/call_check.rs:281-284` — a `take` argument records a move only `if !self.sema.table.duplicable(arg_ty)`; `compiler/src/own/call.rs:346-368` — `take_move_out_places` skips `mode != Take || duplicable(&arg.ty)` at :352; `duplicable(Param)` is the wildcard `true` (`types/table.rs:377-409`); `declare_param` gates only `mode == Take && owns_drop` (`own.rs:630`).
**The Problem:** Rule 1 registers a `take a: T` parameter as maybe-owning, so under a `String` instance the callee drops what it received. But call-argument passing neither records the move-if-owning marker nor empties the caller's place for a param-typed argument, and neither site is in the four duty kinds or the site table. For `func caller<T>(take a: T) { sink<T>(a); }` with `T = String`, `sink` frees its copy and `caller`'s maybe-owning drop frees the same handle — a double free. AR-17 itself lists "take arguments are not emptied" as one of the unsafe sites, yet the table maps take arguments only to the materialization helper (`own/lower.rs:1090/:1104/:1117`), not to the checker/codegen argument sites.

**Options:** (A) add both sites to the duty set, record the move-if-owning marker in `check_argument_place`, emit the move-out for param-typed places in `take_move_out_places` with the same non-owning codegen branch as rule 2, and add an ST case for a place argument. (B) Do not register take parameters as maybe-owning until the caller empties (forfeits R5's release). (C) Add a separate call-argument obligation kind.
**Recommendation:** Option A. It reuses the existing `Statement::Move`/move-out machinery and is required for R5's "under every instantiation" guarantee.

**Authority:** AI — delegated by `--auto-design` (fix selection) **+ reserved ruling needed** (adding `own/call_check.rs` and `own/call.rs` to Phase 4's expected modification set is a modification-set broadening).
**Eligibility:** technical mechanism inside R5; the file-set change is reserved under scope-expansion-control.
**Objective:** no double free and no leak for every take-argument shape under owning instances.
**Decision:** Option A, with Phase 4's set extended by `compiler/src/own/call_check.rs` and `compiler/src/own/call.rs`, plus a rule-3 marker at the same site.
**Evidence:** cited file:line above; challenger independently reproduced the mechanism.
**Rejected alternatives:** B (breaks R5), C (duplicates rule 2).
**Strongest counterargument:** it is the largest scope increase; without the rule-3 marker a read after `sink<T>(a)` silently passes.
**Confidence:** High — **Hardening:** challenger converged (Option A). **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** the probe program stops leaking/double-freeing; Phase 4's set is amended.
**User Decision:** Resolved — User approved the Phase 4 modification-set extension (`compiler/src/own/call_check.rs`, `compiler/src/own/call.rs`); delegated Option A stands.

### PF-002: The fold in `Ty::array` does not make nested fixed arrays impossible [🟠 MAJOR]

**Dimension:** Codebase Alignment (also Feasibility, Consistency)
**Location:** `03-01-flat-array-literal-types.md` §`Ty::array` folding and §Integration Points; `02-current-state.md:23`; `00-ambiguity-register.md` AR #3 (line 33); `01-requirements.md` Compatibility
**Codebase Evidence:** `compiler/src/types/mod.rs:423-436` — `substitute` builds `TyKind::Array` directly (no fold); `compiler/src/types/table.rs:566-575` — `resolve_ty_with` likewise; `compiler/src/codegen/types.rs:52-58`, `:244-249` print only the base element plus recorded extents.
**The Problem:** The plan says folding `Ty::array` means "a nested fixed-array type cannot exist", but every instantiation of `T[N]` with an array type argument (`substitute`) still constructs the nested shape directly. Reproduced: `fillIt<i32[3]>(a)` reports the nonsensical `E10200 cannot convert 'i32[3][2]' to 'i32[3][2]'`; `Grid<i32[3]>` is accepted but emits `i32 rows[2]` and invalid pointer stores. After Phase 1's fold, the initializer becomes flat while the substituted field stays nested, so the currently accepted `Grid` shape risks becoming a new rejection — contradicting the plan's compatibility requirement. The parser flattens written multi-extent types (`parse/types.rs:36-64`), so `resolve_ty_with` is not reachable with an array element today; `substitute` is the real bypass.

**Options:** (A) route `substitute` through the folding constructor (a shared canonical helper; keep slice elements/slice extents nested), and add ST/unit cases for array type arguments; leave `resolve_ty_with` as a documented defensive note. (B) Fold at type-equality/C-spelling consumers instead (wider semantic change). (C) Constructor-only fold plus an honest restatement and a filed residual.
**Recommendation:** Option A (limited to `substitute`). It is one line at the other entry point, stays inside Phase 1's file set (`types/mod.rs`), and makes R1's canonicalization claim true.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** internal compiler representation inside confirmed scope; Phase 1's file set already contains `types/mod.rs` and the test suites.
**Decision:** Option A.
**Evidence:** `types/mod.rs:429-435`; repros above; AST parser flattening verified.
**Rejected alternatives:** B (changes comparison semantics broadly), C (leaves invalid C reachable and a false invariant).
**Strongest counterargument:** folding in `substitute` changes `Ty::display`/diagnostic text for substituted nested types; the ST cases will also surface the `type_key` collision (PF-011).
**Confidence:** High — **Hardening:** challenger converged (Option A). **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** the `Grid<i32[3]>` and `fillIt<i32[3]>` probes compile to valid C.
**User Decision:** Delegated resolution recorded (see Authority).

### PF-003: The maybe-owning rule drops the `take` mode gate [🟠 MAJOR]

**Dimension:** Ambiguities (also Implicit Assumptions, Security)
**Location:** `03-04-substitution-aware-ownership.md` §Proposed Changes 1 and §Checker rules (line 109); `99-execution-plan.md` task 4.2.1
**Codebase Evidence:** `compiler/src/own.rs:630` — `let needs_drop = param.mode == Mode::Take && self.owns_drop(&ty);`; `compiler/src/own.rs:720-726` — `owns_drop` excludes views/unknown/param/void but has no mode input; `is_view(Param)` is false (`borrow.rs:54-60`).
**The Problem:** Rule 1 excludes only view/unknown/void types and says `declare_param` uses `maybe_owns`. A `borrow a: T` parameter's declared type is a `Param`, not a view, so a literal implementation registers it maybe-owning and emits a callee-side drop; under a `String` instance that frees storage the caller still owns (double free). The Error Handling table never mentions `Mode::Borrow` or out-slots. Verified: `observe<T>(borrow a: T)` with `String` is accepted today and the callee releases nothing.

**Options:** (A) state that `declare_param` keeps the `mode == Mode::Take` gate around `owns_drop || maybe_owns` (borrow/out-slot never maybe-owning) and add an ST case (`borrow a: T` with String: caller frees once, callee nothing). (B) Make `maybe_owns` mode-aware structurally (does not fit `declare_local`/`register_pending_release`).
**Recommendation:** Option A plus a small shared predicate helper so the gate is spelled once.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** technical correctness inside R5/R6; file set unchanged.
**Decision:** Option A; also extend the 03-04 Error Handling table with the borrow/out-slot row.
**Evidence:** `own.rs:630`, `own.rs:435-443`; accepted borrow probe.
**Rejected alternatives:** B (mode does not apply to locals/temps).
**Strongest counterargument:** prose-only guards can be lost; the shared helper mitigates.
**Confidence:** High — **Hardening:** challenger converged (Option A). **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** the borrow ST case emits no release in the callee.
**User Decision:** Delegated resolution recorded.

### PF-004: Param-typed owned calls in loop conditions never reach the Phase 4 materializer [🟠 MAJOR]

**Dimension:** Dependency Issues (also Ordering, Completeness)
**Location:** `03-03-loop-condition-owned-call-release.md:51-58` and `:86`; `03-04-substitution-aware-ownership.md:66-70`; `99-execution-plan.md` task 4.2.5
**Codebase Evidence:** `compiler/src/own.rs:720-726` (`owns_drop` false for `is_param`); `types/table.rs:377-409` (`duplicable(Param)` true, including `Wrap<T>`); `own/lower.rs:619-621` (the in-helper skip Phase 4 removes).
**The Problem:** Phase 3's new condition walk pre-filters calls on `owns_drop(call.ty)`, which is false for every param-mentioning type, so Phase 4's removal of the skip *inside* `materialize_guard_operand` is never reached for loop conditions. A generic loop condition with an owned call result (e.g. `while (mk().count > 0 && i < 3)` with `mk(): Wrap<T>`, instantiated `String`) still prints the call inline and leaks (independently reproduced: 30 bytes / 3 allocations). 03-03 delegates the case to "the DEF-49 pass", but no phase changes the walk's caller-side predicate.

**Options:** (A) gate the Phase 3 walk on `owns_drop(ty) || maybe_owns(ty)` (a shared predicate), with the generic-loop ST/ASan case placed in Phase 4's red/green set. (B) Keep the pre-filter and add a second param-typed materialization walk in Phase 4 (duplicates the traversal).
**Recommendation:** Option A. Do not materialize every call unconditionally: that would change the non-owning spelling AR #10 pins.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** technical integration inside R4/R5; `own/loops.rs` is in both Phase 3's and Phase 4's sets.
**Decision:** Option A: carry the predicate change and its ST case in Phase 4, and restate 03-03:56-58 accordingly.
**Evidence:** cited above; reproduced leak.
**Rejected alternatives:** B (duplication).
**Strongest counterargument:** creates an explicit Phase 3→Phase 4 dependency, contradicting the "Phase 4 independent" graph (see PF-018).
**Confidence:** High — **Hardening:** challenger converged (Option A, with the sequencing note). **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** the generic-loop ASan probe is clean.
**User Decision:** Delegated resolution recorded.

### PF-005: The condition walk can double-materialize an already-materialized call [🟠 MAJOR]

**Dimension:** Feasibility Concerns (also Edge Cases)
**Location:** `03-03-loop-condition-owned-call-release.md:49-58`
**Codebase Evidence:** `compiler/src/codegen/expr.rs:107` (a `Materialize` renders as its temp name); `compiler/src/codegen/guards.rs:584` (`_ => "__cl_tmpN = <inner>"` fallback); `own/lower.rs:602-638` (guarded operands are already materialized today).
**The Problem:** The walk explicitly descends through nested `Materialize` values and replaces every owned call. For `while (i < 3 && mkArr()[i] != null)` (`mkArr(): String[2]`) the current compiler already emits `ClString __cl_tmp0[2]` stored once by the guard; wrapping the inner call again produces a `Materialize` whose inner is a `Materialize`, so the outer store falls to `__cl_tmp0 = __cl_tmp1` (invalid array assignment) with both temps pending release (double release). The shape is pinned by `call_value_integrity_impl_test.rs` (`:1564-1636`), which Phase 3's expected set does not include.

**Options:** (A) skip a `Call`/`ClosureCall` that is the direct stored value of a `Materialize` (detected structurally from the parent context), while still descending into a `Materialize` whose stored expression merely contains an owned call elsewhere; add a shape pin for the guarded fixed-array condition. (B) Materialize all owned calls earlier, before existing guard materialization (larger change). (C) Leave as written.
**Recommendation:** Option A. It preserves the current pinned shapes and still fixes DEF-48.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** technical mechanism in `own/loops.rs`; no file-set change if the existing pinned shapes stay byte-compatible.
**Decision:** Option A, with the exception specified structurally and an impl-shape pin added.
**Evidence:** the emitted guarded shape above; challenger independently confirmed the fallback path.
**Rejected alternatives:** B (larger), C (invalid C).
**Strongest counterargument:** the exception must be implemented from parent context, not by type, or an owned call stored by a non-call materialize is missed.
**Confidence:** High — **Hardening:** challenger converged (Option A). **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** the guarded-condition probe emits one temp and one pending release.
**User Decision:** Delegated resolution recorded.

### PF-006: `array_row_expr` conditional arms have no common C type [🟠 MAJOR]

**Dimension:** Edge Cases (also Feasibility, Security)
**Location:** `03-01-flat-array-literal-types.md:74-87` (row-value table)
**Codebase Evidence:** the proposed `Conditional` form is a C ternary of helper outputs; helper arms end as an assignment (int/handle), `memcpy` (`void *`), or a void call — confirmed by the statement core's shapes (`codegen/function.rs:1260-1292`) and the existing scalar fallback (`guards.rs:364-381`). Reproduced with the pinned flags: `cc -std=c99 -Wall -Wextra -Werror` rejects `(c ? (t[0] = 1) : memcpy(t, s, 4))` ("type mismatch in conditional expression").
**The Problem:** R2 names conditional rows, and a conditional with mixed arm kinds (literal vs place vs call) is legal Clorraine, but the two ternary operands must have a common C type. ST-1's conditional uses place/place arms, so the test set does not cover the failing combinations.

**Options:** (A) define a common-typed tail for every arm (e.g. normalize each arm with a trailing `, 0`) and apply it uniformly through nested conditionals; (B) use a branch-store form rather than a C ternary.
**Recommendation:** Option A; B is not viable in an expression context (`ok && [...][0][0]` needs an expression), and would reintroduce a materialization scheme.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** technical emission detail in `codegen/guards.rs` (Phase 1 set).
**Decision:** Option A, with the normalization specified in the helper table and an impl pin for mixed arms.
**Evidence:** cited; challenger reproduced the C constraint with the pinned flags.
**Rejected alternatives:** B (not viable in expression position).
**Strongest counterargument:** normalization must be applied through nested conditionals and keeps the move-zero tail inside the normalized parentheses.
**Confidence:** High — **Hardening:** challenger converged (Option A). **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** mixed-arm row probe builds under the pinned flags.
**User Decision:** Delegated resolution recorded.

### PF-007: Mark the `RequiresDuplicable` trigger hook explicitly [🟡 MINOR]

**Dimension:** Ambiguities
**Location:** `03-04-substitution-aware-ownership.md:55-64` and `:112`
**Codebase Evidence:** `own/statements.rs:213` evaluates the initializer before the store; `own/expr.rs:208-212` funnels every `Name` use through `read_place`; `own/place.rs:214-217` documents "a read always precedes a move in the same expression".
**The Problem:** The rule says a "read" after a move-if-owning store records the duty. The natural hook (`read_place`, reached before any store/consume) also covers a second consuming move, so the feared silent second-move corruption does not exist as stated — but the plan never names the hook, and an executor could implement a narrower notion of "read". The double-move case has no ST pin.
**Options:** (A) name `read_place`/the name-read funnel as the detection hook, note that it precedes consumption sites, and add a double-move ST case. (B) Widen the wording to "any later use".
**Recommendation:** Option A (the challenger's refinement): specify the hook and pin the case; the acceptance semantics stay as R5/R6 define them.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** documentation precision + one test case; files in Phase 4's set.
**Decision:** Option A.
**Evidence:** cited above.
**Rejected alternatives:** B (does not tell the executor where the check lives).
**Strongest counterargument:** if implemented narrowly, the hole is real; hence naming the hook.
**Confidence:** High on the mechanism, Medium on executor placement — **Hardening:** challenger diverged from the original MAJOR framing and downgraded it. **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** the double-move ST case reports `E10020` for an owning instance and accepts `i32`.
**User Decision:** Delegated resolution recorded.

### PF-008: The duty-to-`Monomorphization` path and the marker lifecycle are unspecified [🟠 MAJOR]

**Dimension:** Dependency Issues (also Completeness)
**Location:** `03-04-substitution-aware-ownership.md` §New records, §Checker rules; `99-execution-plan.md` Phase 4 expected set and task 4.2.3
**Codebase Evidence:** `compiler/src/own.rs:104-122` builds `FunctionChecker`s and only afterwards `generics::collect` creates the `Monomorphization`; `FunctionChecker` has no mono handle; `ir/mono.rs:85-151` `CopyObligation` is recorded by a post-IR pass (`own/generics.rs:423`) and has no discharge consumer; marker state must live in `PlaceState` (`own/flow.rs:93-106`, merged at `own.rs:834-865`) and clear on a whole write (`own/place.rs:96+`).
**The Problem:** The plan deposits flow-sensitive duties during the checking pass but never says how they reach the store created afterwards, and the natural hook files (`own/place.rs`, `own/flow.rs`) are outside Phase 4's expected modification set. The marker lifecycle (clear on whole-binding write, conservative join) is also unspecified, so a stale marker after a legal re-initialization could falsely reject an owning instance.

**Options:** (A) extend Phase 4's set with `own/place.rs` and `own/flow.rs`, specify the carrier (collect per-function records and merge in `own::check` before `instance_check`, or pass the mono handle in), and specify clear-on-whole-write plus conservative join. (B) Keep the file set and hold the marker in a side map in `own.rs` with manual merges (duplicates join semantics).
**Recommendation:** Option A.

**Authority:** AI — delegated by `--auto-design` (mechanism selection) **+ reserved ruling needed** (Phase 4 modification-set broadening).
**Eligibility:** technical mechanism; the file-set change is reserved.
**Decision:** Option A with the two files added to Phase 4's expected set and the lifecycle spelled out in 03-04.
**Evidence:** cited file:line above; challenger confirmed the construction order.
**Rejected alternatives:** B (drift risk against `PlaceState` joins).
**Strongest counterargument:** adding a field to `PlaceState` touches core flow state used by speculative passes.
**Confidence:** High — **Hardening:** challenger converged (Option A). **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** Phase 4's set amended; the marker merge/clear has a unit test.
**User Decision:** Resolved — User approved the Phase 4 modification-set extension (`compiler/src/own/place.rs`, `compiler/src/own/flow.rs`); delegated Option A stands.

### PF-009: Loan-owner detection stays substitution-blind while rule 1 releases param-typed temporaries [🟠 MAJOR]

**Dimension:** Security Blind Spots (also Codebase Alignment)
**Location:** `03-04-substitution-aware-ownership.md` rules 1 and 4; `02-current-state.md:131` ("site table is exhaustive")
**Codebase Evidence:** `compiler/src/own/lower.rs:854-868` — `collect_lowered_temp_owners` inserts a `Materialize` temp only `if self.owns_drop(&value.ty)` (:865); consumed by `view_owners` (`own/place.rs:378-391`); views of temporaries are the only `E10030` path (`own/statements.rs:832-841`).
**The Problem:** Rule 1/4 register param-typed temporaries for release, but the loan-owner predicate is still the un-substituted `owns_drop`, so a view derived from such a temporary carries no loan and loses its use-after-release protection — a potential use-after-free. Reachability was not established (the value must be view-typed at check time while its owner is param-typed, and provenance checks often reject first). The challenger also noted an acceptance wrinkle: making param-typed temps owners at check time would make outliving views `E10030` for *every* instance, including duplicable ones — a change to the accepted-program set that R6 currently forbids.

**Options:** (A) add the predicate to the site table, use `owns_drop || maybe_owns` there (inside Phase 4's set) and run a bounded reachability probe; if reachable, either rule an R6 carve-out (reserved) or design per-instance loan discharge. (B) Document why no view can outlive such a temporary and add a unit test on the predicate.
**Recommendation:** A-investigation first (the probe is cheap); escalate the acceptance ruling only if reachable.

**Authority:** AI — delegated by `--auto-design` (add site + probe) **+ reserved ruling needed if reachable** (acceptance-set change).
**Eligibility:** the predicate/site-table change is technical and in `own/lower.rs`; changing which programs are rejected is reserved.
**Decision:** Add the predicate site and a reachability probe to Phase 4; do not silently change acceptance semantics.
**Evidence:** cited file:line; no end-to-end counterexample yet.
**Rejected alternatives:** B alone (unproven without the probe).
**Strongest counterargument:** if unreachable, the probe is speculative work; the plan should still record why.
**Confidence:** Medium — **Hardening:** challenger converged on direction with the acceptance caveat. **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** probe proves/disproves reachability; any R6 carve-out ruling.
**User Decision:** Resolved — User chose Option A-investigation: add the predicate site and a bounded reachability probe to Phase 4; an acceptance (R6 carve-out) ruling is requested only if the probe proves reachable.

### PF-010: The row move-zero names the non-place-aware text helper [🟠 MAJOR]

**Dimension:** Security Blind Spots (also Edge Cases, Codebase Alignment)
**Location:** `03-01-flat-array-literal-types.md:80-86` (storage-row expression)
**Codebase Evidence:** `compiler/src/codegen/expr.rs:385-393` — `move_zero_text_expr` emits `(name = NULL)` or `(cl_zero(&name))` and cannot walk an adjusted array parameter; `compiler/src/codegen/expr.rs:367-378` — `move_zero_expr` is place-aware; `codegen/function.rs:1993-2018` — `emit_move_zero_place`/`emit_array_param_zero` walk the elements.
**The Problem:** For a row whose source is a C-adjusted array parameter (`take a: String[2][2]`, row `a`), `cl_zero(&a)` takes `sizeof` of the pointer, so the handles are not emptied; the destination owns copies while the callee's cleanup frees the same handles — a double free. The `move_zero_expr` helper already exists and is used by the guard emitter (`guards.rs:591`).

**Options:** (A) use `move_zero_expr(place)` for `ValueKind::Place` rows (or extend the text helper with the `is_array_param_place` walk), and pin the whole-place adjusted-parameter row shape. (B) Gate out rows whose source cannot be emptied place-aware (rejects valid shapes).
**Recommendation:** Option A.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** technical emission in `codegen/guards.rs` (Phase 1 set).
**Decision:** Option A.
**Evidence:** cited above; challenger confirmed the existing place-aware helper.
**Rejected alternatives:** B (turns a fix into a rejection).
**Strongest counterargument:** member/index rows over an adjusted parameter are not root-detectable by `is_array_param_place`; pin the whole-place shape explicitly.
**Confidence:** High — **Hardening:** challenger converged (Option A). **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** the adjusted-parameter row probe empties elements.
**User Decision:** Delegated resolution recorded.

### PF-011: `type_key` and the "symbol keys agree" claim [🟡 MINOR]

**Dimension:** Codebase Alignment
**Location:** `02-current-state.md:44-45`, `03-01-flat-array-literal-types.md:41`; R1's "symbol keys" phrase in `01-requirements.md`
**Codebase Evidence:** `compiler/src/codegen/types.rs:290` — `TyKind::Array { element, .. } => type_key(element)` ignores extents. Reproduced: a module with both `Grid<i32[3]>` and `Grid<i32[2]>` emits `struct ClGrid__i32` twice; `cc` reports a redefinition.
**The Problem:** The plan states `type_key` "already walk[s] extents as one flat list" and needs no change; it does not. Distinct fixed-array type arguments share C symbols. The underlying defect is adjacent to (not one of) DEF-46..49, but it is the next invalid-C source once PF-002's array-type-argument cases compile, and R1's wording asserts agreement.
**Options:** (A) fix `type_key` to include extents (adds `codegen/types.rs` to Phase 1 — reserved), with the two-instance test renamed accordingly. (B) Correct the claim, state that symbol keys cover only the base element today, and file the collision as a residual defect.
**Recommendation:** B for this plan, explicitly excluding the adjacent defect (it is not among DEF-46..49); the user may instead rule A in now.

**Authority:** reserved — scope/modification-set decision (A) vs in-scope claim correction (B).
**Eligibility:** B is a documentation correction inside the audit target; A broadens Phase 1's file set.
**Decision:** Pending user ruling. The reviewer's recommendation is B plus an explicit "Won't Have" entry.
**Evidence:** cited above; duplicate-symbol repro.
**Rejected alternatives:** silently leaving the claim uncorrected.
**Strongest counterargument:** leaving a known invalid-C path adjacent to the fix is poor sequencing; filing it as a defect keeps it visible.
**Confidence:** High on the facts — **Hardening:** challenger converged on "decide scope now". **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** a residual defect is filed or `codegen/types.rs` is added to Phase 1.
**User Decision:** Resolved — User chose Option B: correct the "symbol keys agree" claim, add a Won't-Have entry, and file the duplicate-symbol collision as a separate residual defect.

### PF-012: The DEF-49 completeness proof's count and per-site coverage do not match [🟡 MINOR]

**Dimension:** Testability
**Location:** `03-04:31` ("Nine actions"), `:72-85` (table: 10 rows / 11 locations), `:150`; `07-testing-strategy.md:12`, `:64-70`; `99-execution-plan.md:190`, `:216`
**Codebase Evidence:** `rg mentions_type_param compiler/src/own` finds 11 suppression locations (`lower.rs:619,705,980,1051,1090,1104,1117`; `expr.rs:50,85,108`; `loops.rs:503`). ST-10–ST-16 number seven. ST-14's example uses a slice (`xs: T[]`) and `mk(): i32`, so it cannot reach `lower.rs:980` (requires `is_fixed_array`) or `:619` (requires an impure param-typed operand).
**The Problem:** The plan promises "one specification case per suppression site" three times, but seven cases cannot cover eleven locations, no site→case map exists, and ST-14's example does not reach the sites it cites. The close-out deliverable "every site in the 03-04 table is covered by a passing case" is therefore unverifiable.
**Options:** (A) add a site→case mapping, add the missing cases (two-use operand, owned-argument binding, both conditional flow recorders, fixed-array read place), and rewrite ST-14 to an impure param-typed shape. (B) drop the per-site claim.
**Recommendation:** Option A; the per-site claim is the completeness proof AR #17 relies on.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** test design + docs inside the plan's file set; no acceptance-criteria change (the claim is strengthened to its own stated bar).
**Decision:** Option A; correct the "nine" count to the table's 11 locations.
**Evidence:** cited above; cluster audits agreed.
**Rejected alternatives:** B (removes the only check that the sites were fixed).
**Strongest counterargument:** more cases cost execution time; the count is small.
**Confidence:** High — **Hardening:** challenger not required (minor). **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** the site→case map covers every table row.
**User Decision:** Delegated resolution recorded.

### PF-013: Existing unit tests invalidated by the fold are not scheduled [🟡 MINOR]

**Dimension:** Codebase Alignment (test impact)
**Location:** `03-01-flat-array-literal-types.md` §Testing Requirements; `99-execution-plan.md` task 1.3.2
**Codebase Evidence:** `compiler/src/types/table.rs:913-923` builds a 40-deep nested type through `Ty::array` and asserts `duplicable == true`; under the fold each call collapses, so the element becomes `String` at depth 1 and the assertion fails. The nested-array assertions at `:899-907` become vacuous.
**The Problem:** `make check` fails in Phase 1 until these tests are rewritten, and no task names them; the plan's risk table does not mention the test impact.
**Options:** (A) rewrite them with an explicit `TyKind::Array` construction in task 1.3.2. (B) Drop the depth-cap coverage (weaker).
**Recommendation:** Option A. The Phase 1 set already authorizes `compiler/src/types/` unit tests.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** test maintenance inside the phase's set.
**Decision:** Option A.
**Evidence:** cited above; two clusters agreed.
**Rejected alternatives:** B.
**Confidence:** High — **Hardening:** challenger not required. **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** `make check` green in Phase 1.
**User Decision:** Delegated resolution recorded.

### PF-014: `RequiresDuplicable` diagnostic location and dedup wording [🟡 MINOR]

**Dimension:** Consistency
**Location:** `03-04:55-64`, `:94-101`; `07-testing-strategy.md:66`; R5 wording in `01-requirements.md`
**Codebase Evidence:** `CopyObligation` carries an `instance` key (`ir/mono.rs:86-95`); `Instance` carries only `function` and `args` (`:20-25`); `instance_check.rs` iterates instances without call-site spans (`:99-109`).
**The Problem:** The record has only the body read span, so the diagnostic cannot point at the instantiation call site; "once" is ambiguous when two owning instances conflict ("once per function and binding" vs "at the instantiation, once").
**Options:** (A) add the instantiation site to the record like `CopyObligation`. (B) Keep the read-site span and state explicitly: one `E10020` per function+binding at the body read.
**Recommendation:** Option B (smaller surface; `E10020` names the binding).

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** wording + diagnostic-shape decision inside R5/R8.
**Decision:** Option B; align R5 and ST-12 wording.
**Evidence:** cited above; cluster 1.
**Rejected alternatives:** A (larger).
**Confidence:** Medium — **Hardening:** challenger not required; **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** ST-12's assertion matches the chosen wording.
**User Decision:** Delegated resolution recorded.

### PF-015: Stale or wrong code citations [🟡 MINOR]

**Dimension:** Consistency
**Location:** `03-02:37-40` (`codegen/expr.rs:412-446` "call sites"), `02-current-state.md:55` and `03-02:26` (`function.rs:1072-1094` for `emit_arm_source_moves`), `02-current-state.md:44-45` (`own/generics.rs:148` "walks extents")
**Codebase Evidence:** `expr.rs:412-446` is `call_with_destination` (rendering); destinations are created/initialized at `function.rs:1018-1026`, `:1785-1792`, `:498-506`, `:1227-1259` and `guards.rs:141-147`, `:561`. `emit_arm_source_moves` is defined at `function.rs:1723-1752`, called at `:1093` and `:1117`. `own/generics.rs:148` recurses `element` and ignores extents.
**The Problem:** Safety arguments cite locations that do not support them, so reviewers cannot verify the premises.
**Options:** Single viable option — correct the three citations (no design change).
**Recommendation:** Fix the citations during the next plan edit.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** documentation accuracy.
**Decision:** Fix the citations.
**Evidence:** cited above.
**Rejected alternatives:** none viable.
**Confidence:** High — **Hardening:** challenger not required. **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** citations resolve to the described code.
**User Decision:** Delegated resolution recorded.

### PF-016: ST-10's "instantiated with `String[2]`" is ambiguous [🟡 MINOR]

**Dimension:** Consistency
**Location:** `07-testing-strategy.md:64`; `03-04:123-125` (example uses `keep<String>(xs)` with `xs: String[2]`)
**The Problem:** "instantiated with `String[2]`" reads as the type argument (`T = String[2]`, a nested array) while the design example binds `T = String` with a `String[2]` argument. The two readings test different shapes, and only one is intended.
**Options:** Single viable option — state the binding explicitly in ST-10.
**Recommendation:** Rewrite ST-10 as `T = String` with `xs: String[2]` (and, if the nested-array-argument shape is wanted, make it a separate case after PF-002).

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** test wording.
**Decision:** Rewrite ST-10; a nested-argument case is added only as part of PF-002.
**Evidence:** artifact text.
**Rejected alternatives:** none viable.
**Confidence:** High — **Hardening:** challenger not required. **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** ST-10 names the exact substitute.
**User Decision:** Delegated resolution recorded.

### PF-017: Red-phase and test-contract precision [🟡 MINOR]

**Dimension:** Testability
**Location:** `99-execution-plan.md:75` (task 1.1.2); `07-testing-strategy.md:43`, `:57`, `:90-91`; `03-01:86-87`
**Codebase Evidence:** the owned deferred-literal ST-4 fails pre-fix (reproduced); ST-2 fails pre-fix by invalid C too, so labeling it an "observation" understates it; ST-8/ST-9 assert an AddressSanitizer result, but `make check` runs in the container whose clang 14 has no ASan runtime (`07:122`); the fill-form extension (`03-01:86-87`) has no ST or impl pin.
**The Problem:** The red-phase task omits a failing case and misclassifies another; one ST row promises a verification its suite cannot produce; one changed path is untested.
**Options:** Single viable option — add ST-4 to the failing list, separate the ST-5-only regression pin, reword ST-8/9 to defer leak-freedom to the close-out probe (matching the prior plan's ST-29 precedent), and add one fill-form case.
**Recommendation:** Apply all four corrections.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** test-contract precision inside the plan's file set.
**Decision:** Apply all four corrections.
**Evidence:** cited above; cluster 5 verified the pre-fix classifications.
**Rejected alternatives:** leaving ST-8/9 as-is.
**Confidence:** High — **Hardening:** challenger not required. **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** task 1.1.2 lists every failing case; fill path pinned.
**User Decision:** Delegated resolution recorded.

### PF-018: The phase dependency graph contains false edges [🟡 MINOR]

**Dimension:** Ordering & Sequencing
**Location:** `99-execution-plan.md:249-263` (Dependencies)
**Codebase Evidence:** Phase 3's design says "No codegen change" (`03-03:45-47`) and its set is `own/loops.rs` + `ir/guards.rs`; no Phase 1/2 artifact is used. Phase 4 is not independent: PF-002's `substitute` canonicalization and PF-004's predicate interaction cross phases.
**The Problem:** The graph is the executor's ordering contract; "Phase 3 uses the fixed-array call destination from Phase 1/2" is unsupported, and "Phase 4 independent of 1–3" would let an executor start Phase 4 first and miss the integration.
**Options:** Single viable option — correct the graph: Phase 1 → 2 → 3 → 4 (or state Phase 4's real prerequisites), drop the fixed-array-destination edge, and keep the stated spec-first order per phase.
**Recommendation:** Correct the graph and record the Phase 3→Phase 4 predicate dependency from PF-004.

**Authority:** AI — delegated by `--auto-design`.
**Eligibility:** sequencing documentation.
**Decision:** Correct the graph.
**Evidence:** cited above.
**Rejected alternatives:** treating the graph as advisory.
**Confidence:** High — **Hardening:** challenger noted the same dependency. **Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**Reopen triggers:** the graph matches the designs.
**User Decision:** Delegated resolution recorded.

### PF-019: `value_needs_guards` contract and second consumer [🔵 OBSERVATION]

**Dimension:** Consistency
**Location:** `03-03:41-44`, `:66`
**Codebase Evidence:** `ir/guards.rs:1-18` documents the predicate as "division check, bounds check, float-cast check"; after the change a `Materialize` (store + drop) also reports true. `for_step_is_structured` (`own/loops.rs:586-598`) is a second consumer not listed in the integration points.
**The Problem:** The predicate's documented meaning narrows to "the loop header cannot carry this value"; the unlisted consumer also changes behavior (a step with a materialize drops to the flat form).
**Recommendation:** Update the doc contract and list `for_step_is_structured` in the integration points with a Phase 3 impl pin.

**Authority:** AI — delegated by `--auto-design`. **Eligibility:** documentation/test detail.
**Decision:** Apply the doc update and add the shape pin.
**Evidence:** cited above. **Confidence:** High.
**Policy version:** 1. **Root invocation ID:** `pf-rdh-20261001-1`.
**User Decision:** Delegated resolution recorded.

---

## Scope Expansion Register

Strict mode — no optional additions were collected. Out-of-scope items surfaced during the scan are intentionally not reported here.

## Hardening, independence, and bias notes

- **Fan-out fallback:** the project's `preflight-auditor` and `design-challenger` agents are manual-invocation-only in this session, so the clustered scan ran as five generic subagents with the complete quality-profile packets, and hardening as one generic challenger with the batch contract. Reviewer independence (fresh contexts) was preserved.
- **Challenger result:** converged on Option A for all ten critical/major findings, with refinements: PF-002's bypass is `substitute` (not `resolve_ty_with`); PF-004's predicate must not materialize every call (preserve AR #10); PF-005's exception must be structural; PF-007 downgraded to a specification hook; PF-009's acceptance caveat; PF-010's `move_zero_expr` exists.
- **Same-agent bias:** the artifact was authored earlier on 2026-10-01 in a different session (this review ran in a fresh session), but likely by the same model family. High-stakes findings were independently reproduced with the committed compiler; the challenger ran blind to the auditor's picks.
- **Standard-first checks:** repository standards (`AGENTS.md`, `spec/check.sh`) and the pinned build flags were cited directly; no external standard governs this change.

## Fix application log

**2026-10-01** — User instruction: "apply all fixes". Edits were applied to the audit target only,
plus the authorized DEF-50 row in `plans/00-roadmap.md`:

| Finding | Documents edited | Change |
| --- | --- | --- |
| PF-001 | 02, 03-04, 07, 99 | Take-argument duty sites recorded; Phase 4 set extended (`call_check.rs`, `call.rs`); ST-19 |
| PF-002 | 00-index, 00-AR, 01, 02, 03-01, 99 | Fold `substitute`; AR-3 premise corrected; substitution cases added |
| PF-003 | 03-04, 07, 99 | `Mode::Take` gate made explicit; ST-13 borrow case |
| PF-004 | 03-03, 03-04, 07, 99 | Shared `owns_drop || maybe_owns` predicate; ST-18 lands in Phase 4 |
| PF-005 | 03-03, 99 | Already-materialized exception; one-temp/one-release pins |
| PF-006 | 03-01, 99 | Common-typed conditional arm tail |
| PF-007 | 03-04, 99 | `read_place` hook named; double-move pin |
| PF-008 | 03-04, 99 | Record carrier + marker lifecycle; Phase 4 set extended (`place.rs`, `flow.rs`) |
| PF-009 | 01, 02, 03-04, 07, 99 | Loan-owner site + task 4.2.6 probe; conditional acceptance ruling |
| PF-010 | 03-01, 99 | Place-aware `move_zero_expr` for place rows |
| PF-011 | 01, 02, 03-01, roadmap | "Symbol keys" claim corrected; DEF-50 filed |
| PF-012 | 03-04, 07, 99 | 14 sites / 12 rows; site→case map; ST-20 – ST-22 |
| PF-013 | 03-01, 07, 99 | Existing test updates scheduled in task 1.3.2 |
| PF-014 | 03-04, 07, 99 | `E10020` once per function+binding at the body's use |
| PF-015 | 02, 03-02 | Citations corrected (`emit_arm_source_moves`, destination sites) |
| PF-016 | 07 | ST-10 binding stated explicitly |
| PF-017 | 07, 99 | ST-4/ST-2 red phase; ST-8/9 wording; ST-17 fill case |
| PF-018 | 99 | Dependency graph corrected |
| PF-019 | 03-03 | Predicate contract + `for_step_is_structured` consumer |
| (consistency) | 00-index | Stale "targeted per-instance pass" baseline aligned with AR #17 |

No source files were modified. Verification is the bounded iteration-2 re-scan.

## Iteration 2 — fix verification and bounded re-scan

**2026-10-01 14:43** — bounded re-scan of the unchanged audit target per the iterative-re-scan
protocol. Re-read every document; re-checked each Fix-log row against its documents; spot-checked
every source citation the fixes introduced or changed against the committed compiler; re-scanned
all 13 dimensions in the changed areas and their cross-document references.

**Citation checks (all confirmed):** `own/place.rs:18,96`; `own/flow.rs:94-96`;
`own/call_check.rs:270` (skip at `:281`); `own/call.rs:346` (skip at `:352`);
`own/lower.rs:602,701,854,974,1034,1075`; `own/loops.rs:502,586-598`; `codegen/expr.rs:367,385`;
`ir/mono.rs:86`; `ir/guards.rs:69`; `own.rs:104-122,630`.

**Residual findings — reopened under their original numbers, fixed in this iteration, re-read clean:**

| Finding | Location | Residual | Resolution |
| --- | --- | --- | --- |
| PF-004 (iteration 2) | 03-03, 03-04, 07, 99 | The Phase 3/4 predicate split was inconsistent: 03-03 and task 3.2.1 had the Phase 3 walk use `owns_drop \|\| maybe_owns`, while `maybe_owns` is added in Phase 4 (task 4.2.1) and Phase 3's modification set contains no file that could define it; the Phase 3 deliverable also claimed ST-18 passes | 03-03 now assigns the predicate widening to the substitution-aware phase; task 3.2.1 uses `owns_drop`; the ST-18 row moved to the Phase 4 table in 07; the Phase 3 deliverable drops ST-18 |
| PF-017 (iteration 2) | 99 | ST-17 was missing from tasks 1.1.1 and 1.2.3 although it is a Phase 1 case | Both tasks now name ST-17 |
| PF-019 (iteration 2) | 03-03 | `for_step_is_structured` was described in the change text but absent from Integration Points | Added with its line range, satisfying the recorded resolution |
| (consistency, iteration 2) | 00-index, 01, 02 | Superseded "targeted pass" wording remained in three places; 02 carried a duplicate stale table row | Reworded to the obligation-record design; duplicate row removed |

No new findings; the audit target is otherwise unchanged, and no 🔴/🟠 finding remains.

## Verdict

**✅ PREFLIGHT PASSED — all 19 findings resolved (iteration 2 verified).** The accepted fixes were
applied 2026-10-01, and the bounded iteration-2 re-scan verified the resolved findings, their
direct consequences, and the unchanged audit target; the four residual issues found in iteration 2
were fixed and re-read clean (table above). Reserved rulings stand: Phase 4's set is extended for
PF-001/PF-008; PF-009 proceeds as the bounded task 4.2.6 probe whose acceptance consequence needs
an explicit ruling if the probe is reachable; PF-011 is a claim correction plus the filed DEF-50
residual.

**Notes:** same-model review (the artifact was authored earlier on 2026-10-01; high-stakes findings
were independently reproduced against the committed compiler, and the iteration-1 challenger ran
blind to the auditors). The DEF-46–49 roadmap rows advanced to **Plan Preflighted**; the DEF-50
residual row was added during fix application.
