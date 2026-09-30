# Preflight Report: Call Value Integrity

> **Status**: ✅ PREFLIGHT PASSED — all 26 findings resolved (accepted, fixed, and verified in iteration 2)
> **Iteration**: 2 (re-scan after fixes)
> **Artifact**: Full implementation plan at `plans/call-value-integrity/` (10 docs), untracked at HEAD `15cf339`
> **Codebase Grounded**: ~30 source files examined; ~60 document references mapped (all verified except the behavioral claims below); probe evidence on the committed `clc` + host clang 18/ASan
> **Last Updated**: 2026-09-30 13:30
> **Scope**: strict (no `--explore-scope`); normal mode (no `--auto-design`); lenses: compiler-and-language

## Codebase Context Summary

**Tech Stack:** Rust std-only workspace (`cargo`), emits one C99 translation unit; C runtime shims; frozen spec under `spec/` as the test oracle.
**Architecture:** staged compiler (`lex` → `parse` → `load` → `resolve` → `sema` → `own` → `ir` → `codegen`); the emitter renders a typed control-flow IR; ownership decisions (moves, drops, releases, guards) are made in `own/` and consumed by `codegen/`.
**Key Files Examined:** `compiler/src/own/{statements,expr,lower,call,loops,place}.rs`, `compiler/src/own.rs`, `compiler/src/codegen/{function,expr,types,cleanup,guards,drop,calls,control,program}.rs`, `compiler/src/codegen.rs`, `compiler/src/{ir.rs,ir/place.rs,types/{mod,table}.rs,sema/expr.rs}`, `compiler/tests/*`, `spec/{17-codegen,05-statements,04-expressions,03-ownership}.md`.
**Reference Verification:** all cited files and symbols exist at the cited lines (spot-checked ~35); behavioral claims needing probes were tested directly.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 5 | 🟠 MAJOR |
| 2 | Implicit Assumptions | 5 | 🔴 CRITICAL |
| 3 | Logical Contradictions | 3 | 🔴 CRITICAL |
| 4 | Completeness Gaps | 6 | 🔴 CRITICAL |
| 5 | Dependency Issues | 1 | 🟠 MAJOR |
| 6 | Feasibility Concerns | 6 | 🟠 MAJOR |
| 7 | Testability | 4 | 🟠 MAJOR |
| 8 | Security Blind Spots | 4 | 🔴 CRITICAL |
| 9 | Edge Cases | 5 | 🟠 MAJOR |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 5 | 🟡 MINOR |
| 13 | Codebase Alignment | 4 | 🟠 MAJOR |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 2 | 2 pending |
| MAJOR | 12 | 12 pending |
| MINOR | 9 | 9 pending |
| OBSERVATION | 3 | 3 pending |

---

## CRITICAL Findings

### PF-001: DEF-37 `goto` contradicts the frozen codegen contract and the "no spec changes" claim 🔴 CRITICAL

**Dimension:** Logical Contradictions / Codebase Alignment
**Location:** `00-ambiguity-register.md` AR #8; `03-04-guards-and-cleanup.md` §Proposed Changes 2; `00-index.md` ("changes no specification text"); `01-requirements.md` Feature Overview
**Codebase Evidence:** `spec/17-codegen.md:24` allows "`goto` for cleanup ladders only (section 4)" and forbids "`goto` for any other purpose"; `compiler/tests/codegen_run_spec_test.rs:582-601` (the cited "structured-loop guarantee") only asserts a run exit code for a `while (i < n / 2)` program — it never inspects the emitted C and never exercises a `do`/`while`; `compiler/src/codegen/control.rs:362-375` is the change site.
**The Problem:** The chosen DEF-37 mechanism emits a loop-control `goto`, which the frozen codegen contract forbids, while the plan asserts no spec change and structured-loop guarantees. The user approved AR #8 without this conflict being surfaced. Executing the plan as written either violates the oracle or forces a mid-phase spec ruling.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Keep the `goto`; record an explicit spec-deviation/spec-change user ruling and correct both claims | Smallest code change; preserves AR #8 | Spec change is out of the plan's stated scope; the false claim must be retracted |
| B | Goto-free flag-first restructure: `{ i32 __cl_first = 0; for (;;) { if (__cl_first) { <guards>; if (!(cond)) break; } __cl_first = 1; <body> } }` | Honors the frozen contract; standard do/while translation; handles `continue` and embedded guards uniformly | Emission change in `control.rs`; must preserve `break`/cleanup-ladder edges and determinism |
| C | Another goto-free form (guarded-condition statement block reusing the existing `while (1)` guard pattern) | Reuses existing guard host | More emitter surface than B |

**Recommendation:** Option B — stays inside the frozen spec, reuses the loop emitter's existing guard handling, and removes the F2 edge class; AR #8 must be reopened and the "no spec changes"/structured-loop citations corrected in iteration 2.
**Challenger:** converged (B); noted the existing `__cl_b` control gotos weaken the "new violation" claim but the false documentation claim stands.
**Related:** PF-002.
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-002: DEF-37 goto mechanism misses `continue` edges and embedded-only guards 🟠 MAJOR

**Dimension:** Edge Cases / Completeness Gaps
**Location:** `03-04-guards-and-cleanup.md` §Proposed Changes 2; task 4.2.2
**Codebase Evidence:** `compiler/src/own/loops.rs:84-91,125` sends the fall-through and every user `continue` to the same condition block; `compiler/src/codegen/control.rs:184-187` prints `continue;` for any edge to that target, and `:372-374` prints the guard statements after the body — a C `continue` skips them; `compiler/src/ir/guards.rs:19-38` (`value_needs_guards` recurses into short-circuit rights) vs `compiler/src/codegen/guards.rs:26-33` (`emit_guards` emits no statement for an embedded-only guard — e.g. `10 / i > 0`); probe `/tmp/opencode/preflight-cvi/do_div.c`.
**The Problem:** As worded ("the body's fall-through ... instead of `continue`"), an explicit `continue` in a guarded `do` body still skips the guards (DEF-37 survives for that shape), and a trigger keyed on IR `value_needs_guards` would print a label with no following statement (invalid C) for embedded-only guards.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | If the goto is kept: define the trigger on emitted statement guards, route every `continue` edge to the guarded block, add a `continue` ST | Fixes the defect fully | More edge surface; still carries PF-001 |
| B | PF-001 Option B restructure | Makes this class moot | None beyond PF-001 |

**Recommendation:** Option B (via PF-001); if A is ruled instead, A is mandatory and must pin the `continue` and embedded-only guard cases.
**Challenger:** converged.
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-003: `return` is a missing adopting site — ASan-confirmed use-after-free 🔴 CRITICAL

**Dimension:** Security Blind Spots / Completeness Gaps
**Location:** `03-01-scalar-conditional-ownership.md` §Proposed Changes 4-5; AR #2 Resolution Notes ("every adopting site (a binding, a by-value parameter)"); R1 (no return position)
**Codebase Evidence:** probe `/tmp/opencode/preflight-cvi/ret_cond_false.clo` (reproduced): `return ok ? sink(s) : t;` with `ok=false` emits `__cl_ret = (ok ? ... : t);` and the cleanup ladder frees `t` before the caller reads the returned copy → AddressSanitizer heap-use-after-free in `cl_string_len`; `compiler/src/codegen/cleanup.rs:255-327` move-zeroes only `ValueKind::Place` and runs `emit_temp_releases` before the ladder; `compiler/src/own/statements.rs:842-851` copies pending releases onto the return edge; `compiler/src/own/lower.rs:99-118` creates the transfer `Materialize` on any `defer_move_outs` context including a return. With plan change 4 registering the transfer temp, the `ok=true` direction also releases the value just copied to `__cl_ret` (new UAF).
**The Problem:** The plan's adoption inventory omits `return`; the pre-existing place-arm return UAF is not in scope, and the new registration turns the working direction into a UAF. Both are memory-unsafe accepted programs in the feature's core area.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Treat return as an adopting site: branch-store the conditional into the result (mirror `emit_fixed_array_return_arm`), zero the temp after the copy, zero place arms as today; add STs | Fixes both directions; reuses the array-return precedent | Extends scope beyond the filed shapes (same root cause) |
| B | Register transfer temps only where an adoption path exists; file the return shape as a separate defect | Strict scope; avoids the new UAF | Leaves the pre-existing place-arm return UAF unfixed |
| C | Leave both | — | Ships memory-unsafe C the plan touches |

**Recommendation:** Option A — accepted-program memory safety in the exact conditional family; the systemic sub-option "register only when the consumer will not adopt (release on discard)" also removes the F4/F5 regression risk.
**Challenger:** converged (A; confidence 0.9; strongest counter = strict scope favors B, in which case B must be mandatory).
**Related:** PF-004, PF-005.
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-004: Scalar `take`-argument adoption has no mechanism — double-free 🟠 MAJOR

**Dimension:** Completeness Gaps / Security Blind Spots
**Location:** `01-requirements.md` R1 (owned argument / take position); AR #2, #19; `03-01` §Proposed Changes 5 ("the pending release is discharged at the call")
**Codebase Evidence:** `compiler/src/own/lower.rs:950-962` returns the value unchanged for `take`; `compiler/src/own/call.rs:333-347` (`take_move_out_places`) collects only `ValueKind::Place`; `compiler/src/codegen/calls.rs:129-156` has no post-call discharge; probes `/tmp/opencode/defprobe/def34take.clo` and `/tmp/opencode/preflight-cvi/take_arg_cond.clo`: `eat(ok ? a : b)` with `eat(take s: String)` is an ASan double-free. No ST covers a scalar take argument (ST-4 is `String[2]` and passes pre-fix through C array adjustment).
**The Problem:** "Discharged at the call" has no implementing code; the plan can go green on all STs while R1's take position stays broken (and the new registration would add a second double-free direction).
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Non-releasing argument temp: branch-store the conditional into a fresh temp, zero the chosen arm (element-walk for arrays), pass it; the callee owns it | Smallest change; mirrors `take_move_out_places`/`MoveOut`; no leak/double-free | New argument-temp shape to implement and pin |
| B | Register the temp and emit a post-call `MoveOut`/zero discharge | Reuses the release set | More moving parts; ordering with guards |

**Recommendation:** Option A — the callee's parameter cleanup owns the value; add an ST for the scalar take argument and an impl pin; keep the take-array element zeroing intact.
**Challenger:** converged (A; noted the array twin's callee writes `arr[0] = NULL` back, so the fix must zero per path).
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-005: Struct-literal field conditionals are not move-marked or adopted — double-free/UAF 🟠 MAJOR

**Dimension:** Security Blind Spots / Completeness Gaps
**Location:** `03-01` R1 position list; `03-02` (array fields only)
**Codebase Evidence:** probe `/tmp/opencode/preflight-cvi/field_cond.clo`: `Box { name: ok ? a : b, tag: 1 }` with owned `String` fields is accepted, reading an arm is not rejected, and ASan reports a double-free; `compiler/src/own/expr.rs:48-58` (`consume_owned_place`) never consults `conditional_move_sources`, so arms are not marked moved; `compiler/src/codegen/expr.rs:681-708` renders fields inline; with plan change 4, `name: ok ? sink(s) : t` becomes a UAF (the temp is released after the field copied it).
**The Problem:** The field position is an owning destination the plan neither fixes nor scopes; the new registration regresses the take-call field variant.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Treat the field as an owning destination: mark the conditional arms moved and emit a per-field branch store (reuse the 03-02 zero-init + field-store machinery); add ST/impl pins | Fixes the shape; natural host already planned | Extends the branch-store matrix; scope growth |
| B | Register transfer temps only where an adoption path exists (release-on-discard) and file the field shape as a separate defect | Strict scope; no new UAF | Field double-free remains |

**Recommendation:** A; if strict scope wins, B is mandatory (never leave the regression).
**Challenger:** converged (A, confidence 0.7; strongest counter = scope creep).
**Related:** PF-003 (systemic registration option).
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-006: Scalar branch store omits the fixed-array twin's old-target and self-naming rules 🟠 MAJOR

**Dimension:** Completeness Gaps / Edge Cases
**Location:** `03-01` §Proposed Changes 1 (example), §Changed Functions; R1 assignment position; no ST for assignment
**Codebase Evidence:** probe `/tmp/opencode/preflight-cvi/self_cond.clo`: `c = ok ? c : b;` is accepted and ASan double-frees today; a naive branch store would emit `c = c; c = NULL;` and destroy the value. The fixed-array twin saves the old target, restores it for a self-naming arm with the save emptied, and releases the save once (`compiler/src/codegen/function.rs:825-867`, `:899-909`); scalar `emit_assign` has a `value_reads_base` save path (`:709-748`).
**The Problem:** R1 requires the branch store for assignment, but the design only shows the fresh-`let` shape; no old-target release/save and no self-naming-arm rule, and no ST pins them.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Mirror the fixed-array save/restore semantics in the scalar emitter (including nested conditional arms) and add assignment/self-naming STs | Complete and proven template | More emitter surface |
| B | Restrict the rule to empty destinations | Small | Contradicts R1; leaves a double-free |

**Recommendation:** Option A.
**Challenger:** converged (A, confidence 0.9).
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-007: Owned aggregate conditional in scope wording only — ASan-confirmed double-free 🟠 MAJOR

**Dimension:** Ambiguities / Codebase Alignment
**Location:** R1 ("a move-only value selected by `?:`") vs `03-01` ("scalar owned (move-only) value"); ST-1–ST-8 use `String` only
**Codebase Evidence:** probe `/tmp/opencode/cvi-preflight/agg.clo`: `let c: Box = ok ? a : b;` (Box owns a String) emits `c = (ok ? a : b);` and ASan double-frees; `prepare_binding_effect` already classifies it as `ConditionalMove` (`compiler/src/own/statements.rs:381-411`); the emission path is generic (`compiler/src/codegen/function.rs:709-748`).
**The Problem:** "Scalar" is undefined; R1 promises every move-only value while the design/tests cover handles only. An implementer gating on handles leaves the same root cause broken; gating on `emits_drop` fixes it but needs struct move/zero (available via `emit_move_zero`).
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Cover every non-array `emits_drop` destination; add a `Box` ST + impl pin | Same root cause, nearly free once PF-006 exists | Widens the branch-store matrix |
| B | Explicitly exclude aggregates in Won't Have and file a separate defect; narrow R1's wording | Strict scope | Leaves an accepted ASan double-free |

**Recommendation:** A.
**Challenger:** converged (A, confidence 0.9); B requires changing R1's Must-Have wording.
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-008: `FuncTy` mode change impact is under-scoped; ST-24 blocked at `FuncSig::ty()` 🟠 MAJOR

**Dimension:** Implicit Assumptions / Impact Blindness
**Location:** `03-03` §5, task 3.2.4 file list; `02-current-state.md` risk table ("declared/FFI paths unchanged")
**Codebase Evidence:** `compiler/src/types/table.rs:47-55` (`FuncSig::ty()`) builds `FuncTy.ty` from `self.params` and drops `modes`, so a declared function (`nudge`) cannot carry `borrow` into ST-24; `FuncTy` derives `PartialEq` (`types/mod.rs:79-86`) so modes will automatically participate in equality/type keys, but the plan states no equality rule; consumers outside the listed files: `types/mod.rs:356,421-431,561-582`, `sema/expr.rs:278-291` (E10210), `sema/call.rs:229,313-318,492-497`, `own/lower.rs:295,1017-1019`, `own/generics.rs:150-169`, `abi.rs:78-86`; no negative mismatch ST.
**The Problem:** The plan's "internal change" claim misses the declared-function path that ST-24 needs and the mode-equality/diagnostic semantics; an implementer could leave modes out of equality (silent ABI mismatch) or fail ST-24 for an unmodeled reason.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Extend task 3.2.4 with `FuncSig::ty()` mode zipping, the full consumer list, the equality rule (exact signature incl. mode → E10210 otherwise), mode-aware display, and a rejection ST | Complete; compiler-enforced | Larger explicit change set |
| B | Parallel `modes: Vec<Mode>` beside `FuncTy.params` | Smaller blast radius | Reopens AR #14; desync risk |

**Recommendation:** A (AR #14 already fixed the representation; only the impact list is open). Add `let g: func(Entity): void = nudge;` where `nudge` is `borrow` → E10210.
**Challenger:** converged (A, confidence 0.8).
**Related:** PF-023.
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-009: DEF-31 runtime-index store cannot reach the named array-assign machinery as specified 🟠 MAJOR

**Dimension:** Feasibility Concerns / Completeness Gaps
**Location:** `03-02` §Proposed Changes 1, Changed Functions (`emit_assign`, `projected_type`); task 2.2.1
**Codebase Evidence:** runtime-index stores go `compiler/src/own/expr.rs:644-704` → `Statement::StorePointer` → `compiler/src/codegen/function.rs:390-399` (`*(&(rows[i])) = value;`); `Projection::Element(Option<usize>)` cannot carry a runtime index value (`compiler/src/ir/place.rs:126-137`; `codegen/expr.rs:130-137` renders `[0]`); `emit_assign` already routes fixed arrays (`:712-714`), and `emit_array_assign_to` takes a rendered lvalue string (`:1049`); the index bounds guard lives in the pointer path (`:394`).
**The Problem:** ST-9/11/12 (the primary filed shapes) route through `StorePointer`, which the plan's named `Place`-based machinery cannot host; the emission route (pointer-target array assign) and guard placement are unspecified. The constant-index sibling can use a `Place`.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Add a rendered-target/pointer variant of the fixed-array assign/conditional/literal emission (one shared core) and route `StorePointer` for array element types | Minimal IR surface; guards already emitted | Refactor to parameterize `target_c` |
| B | Extend the IR projection to carry the index value | IR-clean | New IR variant; ordering contract (`Ord`) impact |
| C | Lower the element address into a `DerefTemp` place | Reuses the Place path | New lowering; for-in-specific machinery |

**Recommendation:** A (refactor the shared core first), plus the already-planned `projected_type` fix.
**Challenger:** converged (A, confidence 0.85; strongest counter = duplicated emission logic unless refactored).
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-010: DEF-39 growable drop expression is infeasible as recorded 🟠 MAJOR

**Dimension:** Feasibility Concerns / Security Blind Spots
**Location:** AR #9; `03-04` §Proposed Changes 3 ("mirroring the statement path's runtime call"); task 4.2.3
**Codebase Evidence:** `compiler/src/codegen/drop.rs:112-129` — the statement path is an inline loop with block-local declarations, not a runtime call; `drop_expr_text` returns `None` for `Array` (`:155-156`); `guards.rs:316-327` skips the drop when no form exists; C99 has no expression-form loop and `spec/17-codegen.md` forbids statement expressions; ST-29 is `Array<i32>` (duplicable elements).
**The Problem:** The AR-approved mechanism does not exist; a general form needs a generated per-element-type drop helper (new support surface, Complexity Escalation Gate) or a duplicable-elements-only form that would silently leak `Array<String>` elements.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Limited form only when the element type needs no drop (`(tmp.data ? (FreeBuffer(tmp.data), tmp.data = NULL, tmp.count = 0) : 0)`), state the limit, pin it | Fixes DEF-39 as filed (ST-29); no new surface | `Array<String>` embedded guards keep a leak (pre-existing, unfiled) — must be stated |
| B | Generated per-element-type drop helper (or shim helper) | Fully fixes the class | New support surface → explicit complexity approval required |
| C | Hoist the previous-value drop as a statement where a statement host exists (loop headers already do), limited form elsewhere | No new surface; better coverage | Second behavior fork; not all guard contexts have a statement host |

**Recommendation:** A for strict scope, with the limitation stated in `03-04`/Won't Have and an `Array<i32>` ST; option B only with an explicit Complexity Escalation Gate approval.
**Challenger:** converged (A, confidence 0.75; strongest counter = codifies an `Array<String>` leak in a memory-integrity plan).
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-011: Struct-literal value positions have no destination; mixed-form rule contradicts evaluation order 🟠 MAJOR

**Dimension:** Logical Contradictions / Feasibility Concerns
**Location:** `03-02` §Proposed Changes 4 ("mixed literal may combine the two forms as long as evaluation order is preserved"); AR #7; R8
**Codebase Evidence:** probe `/tmp/opencode/preflight-cvi/box_arg.clo`: `eat(Box { arr: ok ? a : b, tag: 1 })` emits `(ClBox){ (ok ? a : b), 1 }`, which clang rejects; `struct_literal` is a `&self -> String` renderer (`compiler/src/codegen/expr.rs:681-708`); `spec/04-expressions.md` requires struct fields to evaluate in source order, while C initializers evaluate before later stores, so a mixed form reorders side-effecting fields.
**The Problem:** AR #7's zero-init + per-field store is a statement sequence needing a destination; call-argument/return/nested-literal positions have none, and the mixed-form sentence gives an implementer two contradictory instructions.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Materialize any struct literal containing a place-less array field into a temporary at lowering (extend the AR #11 place-requiring gate), then zero-init + store every field in source order; delete the mixed-form sentence; add ST variants | Covers all positions; preserves order | Changes emitted shapes for some passing programs; adds a temp |
| B | Restrict to assign/`let` targets and scope-out value positions | Smaller | Leaves `eat(Box {...})` invalid C (accepted program) |

**Recommendation:** A (+ the one-rule wording: when any field lacks an initializer form, store every field in source order).
**Challenger:** converged (A, confidence 0.8).
**Related:** PF-005 (field conditionals).
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-012: DEF-30 closure/lifted-lambda array-return ABI plumbing is incomplete 🟠 MAJOR

**Dimension:** Implicit Assumptions / Codebase Alignment
**Location:** `03-03` §Proposed Changes 1-2, Changed Functions; task 3.2.1 file list
**Codebase Evidence:** `func_pointer_declarator`/`func_pointer_signature` have no environment prefix and cannot nest a func return (`compiler/src/codegen/types.rs:66-96`); the closure `fn` field and lifted signature are built inline with `c_type` (`program.rs:267-283`, `function.rs:198-220`); `emit_lifted_function` never sets `result_var = __cl_ret` for a fixed-array return (`function.rs:144,169-173`); `ClosureCall` sites pass no destination (`expr.rs:539-562`); probes: `static i32 cl_lambda_0(i32 x)` assigns an undeclared `__cl_ret`; a capturing array-taking lambda fails `-Werror`. ST-18/19 cannot pass without the destination plumbing.
**The Problem:** The plan names "declarator reuse" but not the shared signature builder semantics (env prefix + destination), `result_var`, or the closure call site, so a partial fix leaves DEF-30/43 halves open.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | One signature builder (optional env prefix, params, ret) returning ret/params/dest, shared by `program.rs` and `lifted_signature`; destination-aware closure calls; `result_var` | Complete, single source of truth | Refactor of two builders |
| B | Dedicated closure builder beside `func_pointer_signature` | Smaller immediate diff | Two builders can drift |
| C | Narrow: capture-less lifted signature + closure field only | Smallest | Capturing/direct-call shapes remain broken (probed) |

**Recommendation:** A; state the builder contract and dest plumbing explicitly; add an ST for a capturing array-returning lambda through a generic callable bound (`func(T): T`) and a direct-invocation case.
**Challenger:** converged (A, confidence 0.85).
**Related:** ST-19 wording (PF-019).
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-013: DEF-36 covers only the move source; the target side is miscompiled, and one change row would alter the parameter ABI 🟠 MAJOR

**Dimension:** Edge Cases / Architecture Mismatch
**Location:** `03-04` §Proposed Changes 1; Changed Functions row "`is_pointer_param` usage ... cover array parameters in the source spelling"; R13
**Codebase Evidence:** probe `/tmp/opencode/preflight-cvi/arr_param_assign.clo`: `func g(take a: String[2], take b: String[2]) { a = b; }` emits `memcpy(&(a), &(b), sizeof(a)); cl_zero(&b);` — for C-adjusted `ClString *a` this copies pointer bytes with the wrong size and zeroes the parameter slot; `is_pointer_param` is `Borrow && Struct && !duplicable` (`codegen.rs:527-529`) and feeds the parameter declaration (`:416-420`) and `pointer_params` rendering (`expr.rs:157-164`).
**The Problem:** The plan names only the move source; the identical target-side adjustment remains miscompiled. Implementing the row literally would change the array-parameter ABI and break the zero/drop walk.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Make copy/zero place-aware on both sides (detect by-reference array params from the place base; copy through the pointer with element extents; zero by element walk); keep `is_pointer_param` unchanged; add ST | One helper covers both; no ABI change | Slightly wider than the filed source-side title |
| B | Source-only scope; file the target side as a separate defect | Strict scope | Leaves an accepted miscompile |

**Recommendation:** A; reword the `is_pointer_param` row to "detect the by-reference array parameter from the move source/target place base".
**Challenger:** converged (A, confidence 0.85).
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-014: Phase-1 red phase is not achievable as specified; DEF-45 has no discriminating check 🟠 MAJOR

**Dimension:** Testability
**Location:** `01-requirements.md` acceptance criterion 1; `07-testing-strategy.md` ST-1–ST-8 and checklist; `99-execution-plan.md` tasks 1.1.2/5.1.2; AR #16
**Codebase Evidence:** probes on the committed compiler: ST-1-shaped code double-frees (red); ST-2/ST-3/ST-4/ST-7/ST-8 pass pre-fix (ST-7/8 even under ASan+LSan because the leaked temp is still reachable at exit, so LSan stays quiet); the filed failing mixed shape (`ok ? mk() : b` with the place arm chosen) has no ST; scalar take/assignment/return shapes have no ST; `07` checklist says "verified to FAIL before implementation" while task 1.1.2 hedges "record any that pass".
**The Problem:** Only ST-1 can be red; acceptance criterion 1 ("fails before each implementation phase") is unachievable as written, and DEF-45's planned verification cannot distinguish before/after. The ST set misses the failing mixed-arm shape it claims to cover.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Reword AC1 to "at least one case per defect fails pre-fix; pre-passing cases are recorded regression pins"; add the failing mixed/return/take/assignment shapes; specify a host ASan command (`clang -fsanitize=address`, `detect_leaks=1`, function-scoped/loop probe) and a leak-shape probe; add impl-shape pins for the transfer temp | Discriminating red phase; honest close-out | Larger ST set |
| B | Keep AC1 as written | — | Red phase cannot happen; executors either fail the gate or fake it |

**Recommendation:** A.
**Challenger:** converged (A, confidence 0.9).
**Related:** PF-018.
**User Decision:** Resolved — User accepted the recommended option (2026-09-30).

## MAJOR / MINOR Findings (continued index)

### PF-015: `03-01` code example does not compile 🟡 MINOR
**Dimension:** Consistency — `03-01` §Code Examples: `func take_it(...): i32` assigned to `let e: String = take_it(...)`; `ok ? mk() : "fg"` uses a `static str` where a `String` destination is required; the line-114 comment ("`a` stays owned-but-unused") contradicts the documented maybe-moved join. **Recommendation:** fix the expressions (`let e: i32`, owned arm) and reword the comment. **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-016: transfer-temp lifecycle is misdescribed 🟡 MINOR
**Dimension:** Consistency / Implicit Assumptions — AR #2 and `03-01` §4-5 say the temp is "initialized to the null handle at the statement's start" and "the pending release is discharged at the call"; in fact all temps are zero-initialized once at function entry (`compiler/src/codegen/function.rs:249-262`), releases zero the place after freeing (`drop.rs:32-38`), and no call-time discharge exists (`own.rs:963-987`). **Recommendation:** restate the real invariant ("zero at entry; every release/adoption zeros it, so re-release is a no-op; adoption is an explicit move-out/zero"). **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-017: verify cadence conflicts across documents 🟡 MINOR
**Dimension:** Consistency — AR #17 resolved "`make check` per task"; R17 and the execution plan say per phase and no task carries a verify note. **Recommendation:** add one binding rule ("every task verifies with `make check` before its checkbox flips") and align R17/AR #17. **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-018: ASan verification environment is unspecified 🟡 MINOR
**Dimension:** Testability — `99-execution-plan.md` 5.1.2 says "host AddressSanitizer probes with leak detection on" without compiler/command/env; prior plans recorded that the pinned container's clang 14 has no ASan runtime (host-side probes were used; one prior plan ran `detect_leaks=0`). **Recommendation:** specify the host command (`clang -std=c99 -O0 -g -fsanitize=address`, `ASAN_OPTIONS=detect_leaks=1`), the probe programs, and the bounded run. **Related:** PF-014. **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-019: ST-19 is not expressible as worded 🟡 MINOR
**Dimension:** Testability — "a struct field `run: func(i32): i32[2]` holding a lifted lambda" is rejected `E10400` unless routed through a generic callable bound (`F: func(i32): i32[2]`), which is the only path that exercises `ClClosure_*.fn`; probe `/tmp/opencode/preflight-cvi` (st19). **Recommendation:** restate ST-19 with the generic-bound shape (keep a capture-less field case). **Related:** PF-012. **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-020: helper-reuse claim is false 🟡 MINOR
**Dimension:** Testability — `07` §Fixtures says `run_host`/`assert_exit`/`assert_rejected_with` are reused; they are private per-test-binary functions, so the new suites must copy them. **Recommendation:** reword to "same helper pattern, defined per suite" (do not extract a shared module without approval). **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-021: ST-4 mixes a run assertion with a compile-time move-state claim 🟡 MINOR
**Dimension:** Testability — ST-4 expects "both arm bindings are maybe-moved; clean exit" in one run case; the move-state claim is unobservable in a run and already pinned by the closed plan's `rvi30`. **Recommendation:** drop the move-state clause (or add a scalar E10020 rejection twin). **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-022: `take` mode in func types remains representable with no semantics 🟡 MINOR
**Dimension:** Scope / Edge Cases — after `FuncParam { ty, mode }`, `func(take String)` is representable but has no ownership handling (probe `deftake`: accepted and ASan double-free). **Recommendation:** state the limitation in Won't Have (or handle take move-outs if the delta is small); do not leave it implicit. **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-023: func-value call-site mode lookup lacks the callee type 🟡 MINOR
**Dimension:** Feasibility — `call_arguments(callee: &str, args)`/`callee_modes` resolve by name and gate `&` on `user_call`; a func-value call through a local, capture, or struct field has no declared name to look up, so a `borrow` parameter's `&` cannot be chosen (ST-25 is a field call). **Recommendation:** pass the resolved callee `TyKind::Func` (receiver/local value) into the argument-mode resolution; add an impl pin. **Related:** PF-008. **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-024: dangling `quality-checklist.md` link 🔵 OBSERVATION
**Dimension:** Consistency — `99-execution-plan.md:33` links a file that does not exist in the plan dir (the sibling plan references it as plain text). **Recommendation:** drop the link and use the sibling plan's wording. **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-025: the "Executing at start" roadmap transition is unowned 🔵 OBSERVATION
**Dimension:** Consistency — AR #18 says roadmap rows move Executing → Done, but no task owns the Executing transition (rows currently read "Plan Created"). **Recommendation:** add a one-line phase-1 deliverable for the transition. **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

### PF-026: risk-table AR reference is wrong 🔵 OBSERVATION
**Dimension:** Consistency — `02-current-state.md` risks cite "(AR #12)" for the Func-type mode risk; AR #14/#4 resolved that. **Recommendation:** cite AR #4/#14. **User Decision:** Resolved — User accepted the recommended option (2026-09-30).

---

## Dimension notes (clean)

- **Scope Creep Indicators (10):** no unapproved expansion found; the only new-surface candidates are consequences of resolving PF-010 (generated helper) and are explicitly gated.
- **Ordering & Sequencing (11):** phase independence holds for the ST sets; the only cross-phase coupling is shared functions, which the sequential order respects.
- **Redundancy:** no proposed fix duplicates an existing helper.

## Recommended overall path

All 12 MAJOR/CRITICAL recommendations converge (challenger: converged). PF-001, PF-003, PF-004, PF-005, PF-006, PF-007, PF-008, PF-009, PF-012, PF-013, PF-014 required design-text revisions (PF-001 reopened AR #8); PF-010 required an AR #9 correction; PF-002 followed PF-001's outcome.

## Iteration 2 — verification of the applied fixes

All fixes were applied to the plan documents on 2026-09-30. Verification performed:

| Check | Result |
|-------|--------|
| AR #8 revised to the goto-free flag-first do-while; AR #9 revised to the limited expression form; a "Preflight revisions" note records both | ✅ |
| `03-04` goto-free restructure covers fall-through, `continue`, and embedded-only guards; no loop-control `goto` remains in the design; no residual "structured-loop guarantee" claim | ✅ (stale mentions fixed in `02-current-state.md:184`, `03-04:12`, `99-execution-plan.md` task 4.3.1) |
| Adopting-site inventory now covers binding, assignment, `take` argument (non-releasing temp), `return`, and struct-literal field; temp lifecycle wording matches `function.rs:249-262` and `drop.rs:32-38` | ✅ |
| Branch-store destination predicate is `needs_drop` + not-fixed-array (aggregates covered); old-target/save, self-naming, and nested-arm rules ported from the fixed-array twin | ✅ |
| `FuncTy` mode work extended to `FuncSig::ty`, the consumer list, exact-signature equality, mode display, and the call-site callee type; ST-40 pins the mismatch | ✅ |
| DEF-31 rendered-target route and DEF-36 both-sides copying specified; `is_pointer_param` row reworded to keep the ABI | ✅ |
| Closure/lifted-lambda shared signature builder, `result_var`, and destination-aware `ClosureCall` specified | ✅ |
| ST set extended to ST-1 – ST-41 (41 unique rows, no duplicates); red-phase rule and regression pins documented; ST-7/8 LSan reachability noted; host ASan command and probe programs specified | ✅ |
| Requirements R1/R2/R15, AC1, AC3, R17, scope decisions, and Won't Have align with the designs | ✅ |
| Execution-plan tasks, ST ranges, per-task `make check` rule, ASan command, roadmap-transition deliverable, and the `quality-checklist` link corrected | ✅ |
| `02-current-state` risk AR reference corrected; no dangling internal links; all cross-doc ST references resolve | ✅ |

No new findings were introduced by the fixes. **Final verdict: ✅ PREFLIGHT PASSED — all 26 findings resolved.** Roadmap rows advanced to Plan Preflighted.
