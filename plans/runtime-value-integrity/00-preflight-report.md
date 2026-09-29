# Preflight Report: Runtime Value Integrity

> **Status**: ✅ PASSED — all 12 findings resolved (iteration 2 verified; `PF-006` residual reopened and fixed in iteration 2)
> **Iteration**: 2 (re-scan after fixes)
> **Artifact**: implementation plan at `plans/runtime-value-integrity/` (9 documents)
> **Audit target**: the whole plan directory `plans/runtime-value-integrity/` (00-index, 00-ambiguity-register, 01, 02, 03-01, 03-02, 03-03, 07, 99)
> **Context documents**: `plans/array-value-integrity/` (defect origin), `plans/00-roadmap.md`, `AGENTS.md`, `spec/`
> **Content hashes (iteration 2)**: `00-index.md` bd7d4412907a3f7c…, `00-ambiguity-register.md` 0734bd112a96a2fe…, `01-requirements.md` 3b7695cbcd3199ce…, `03-01` 13064b7e1a61829f…, `03-02` dd7dcf2c116390dc…, `03-03` 3202b6e0daa0b699…, `07` e41368d7a1fa87dd…, `99` f742fa0eb09d59d2… (iteration 1 hashes remain recorded below; `02-current-state.md` is at bc15352e…)
> **Last Updated**: 2026-09-29 15:44

⚠️ **Same-agent bias notice**: the plan was not created in this session, but it was authored by the same model family. External standards checked here are the in-repo frozen specification; every citation is quoted from `spec/` rather than recalled. The DEF-23 ownership/lifetime ruling was made by the user through the PF-002 gate packet.

### Iteration 2 — Verification

| Finding | Verification | State |
|---------|--------------|-------|
| PF-001 | R2 states the statement-scoped view rule; 03-01 §1/1a and Example 4; AR-18; 02 Gap 1; ST-24 (same-statement use accepted) and ST-25 (`E10030` on the later use) | ✅ Resolved |
| PF-002 | `00-index.md` records the approved complexity (gate packet + user ruling 2026-09-29); AR-18 restates the surface; report holds every packet field | ✅ Resolved (approved) |
| PF-003 | 03-01 §1/New Types use a dedicated statement-position release, drained after each statement and before terminators; task 1.2.1 and 02 files updated | ✅ Resolved |
| PF-004 | Release set pinned to destination materializations; transfer temps explicitly excluded; unit test in 07 and task 1.3.1 | ✅ Resolved |
| PF-005 | R5/03-02 narrowed to the filed spellings and the AR-9 baseline; closure/lambda form filed as `DEF-30`; Won't Have and AR-9 note | ✅ Resolved |
| PF-006 | Return emitter added to the fix (ST-8 preserved), AR-16 narrowed, `DEF-31` filed. **Iteration-2 residual:** the argument clause still over-claimed (borrow-param place-arm conditionals emit `memcpy(&__cl_owned0, &(cond ? a : b))`, verified invalid). Fixed: place-arm conditional arguments stay forwarding (place-projection predicate extension), ST-29 added, and the call-arm form filed as `DEF-32`. | ✅ Resolved (reopened and fixed in iteration 2) |
| PF-007 | ST-9 moved to the passes-pre-fix group; ST-26 added for the owned argument | ✅ Resolved |
| PF-008 | 03-02 mechanism, R5, task 2.2.2, and ST-17 note now state the local func-typed callee lookup | ✅ Resolved |
| PF-009 | Citations corrected in 02/03-01/03-03/AR-5; acceptance criterion 1 narrowed; slice bullet rewritten | ✅ Resolved |
| PF-010 | ST-27 (read position) and ST-28 (conditional iterable) added; generic iterable recorded in R1, AR-6, 03-01 §2, Won't Have, and the error table | ✅ Resolved |
| PF-011 | 03-03 specifies a length-based writer per shim, embedded-NUL behavior, and the freestanding truncation | ✅ Resolved |
| PF-012 | ST-17 removed from the ASan set (ST-3, ST-4, ST-7, ST-24, ST-26, ST-29 remain); ST-17 text corrected | ✅ Resolved |

**New defects filed during this review:** `DEF-30` (closure/lifted-lambda fixed-array ABI), `DEF-31` (indexed fixed-array element store), `DEF-32` (conditional argument with a call arm). None is a finding against this plan: each is a documented, out-of-scope accepted-but-invalid form with a backlog row.

**Bounded fresh scan:** no new critical/major issues; the iteration-2 residual was a direct consequence of PF-006's fix and is recorded above. The artifact is internally consistent (18 AR items, 29 ST cases, tasks and documents cross-reference correctly).

**Verdict:** ✅ PASSED — all 12 findings resolved; 0 critical/major open; no minor/observation left unaccepted.

---

### Codebase Context Summary (iteration 1)

**Repository:** Clorraine compiler — std-only Rust workspace emitting C99, plus three C runtime shims.
**Tech Stack:** Rust (compiler), C99 (emitted TU and shims), `.clo` sources and embedded stdlib.
**Architecture:** pipeline `lex → parse → load → resolve → sema → own (ownership/loans/lowering to typed control-flow IR) → codegen (C)`. Runtime guards (bounds/division/float) are emitted by `codegen/guards.rs`; scope cleanup is collected by `codegen/cleanup.rs`.
**Reconnaissance:** ~25 source files read directly, plus emitted-C probes run with `target/debug/clc` and `cc`; 37 code/spec anchors referenced by the plan were checked, 33 verified, 4 stale (PF-009).

**Key observations:**
- `ValueKind::Materialize` has exactly one constructor, `materialize_guard_operand` (`compiler/src/own/lower.rs:523-553`), which defers the assignment into the guarded expression. Two other temp producers exist: `materialize_value` (`own/lower.rs:560-570`, take-call results) and `consume_place` (`:580-599`, aggregate-literal element moves); both emit immediately and their values are adopted by a destination.
- `Statement::Drop { place, scope, order }` is a *scope* cleanup action: `Emitter::collect_cleanup` (`codegen/cleanup.rs:39-52`) collects it, and `emit_block_body`/`emit_scope_inline` (`:122-199`, `:231-244`) render it at the scope's end or ladder rung; `codegen/function.rs:423-427` ignores Drop statements inline.
- `materialize_owned_argument` (`own/lower.rs:689-720`) already binds an owning anonymous call result to a function-scope `__cl_ownedN` local because "a view derived from the value can outlive the block that produced it" — the nearest existing answer to the DEF-23 lifetime problem.
- Probes: `let v: str = mk()[0];` is accepted and emits `__cl_tmp0` with no drop; `return ok ? a : b;` and `rows[0] = ok ? a : b;` are accepted and emit invalid C; a capture-less lambda with `i32[2]` return emits `static i32 cl_lambda_0(i32 x)` while its func value is `i32 (*)(i32)` and the call passes a destination.

### Summary by Dimension

| # | Dimension | Findings | Highest severity |
|---|-----------|----------|------------------|
| 1 | Ambiguities | 1 | 🟠 |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 1 | 🔴 |
| 4 | Completeness Gaps | 3 | 🟠 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 2 | 🟠 |
| 7 | Testability | 3 | 🟡 |
| 8 | Security Blind Spots | 0 (compiler-internal; AR-13 holds) | — |
| 9 | Edge Cases | 1 | 🔵 |
| 10 | Scope Creep Indicators | 1 | 🟠 |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 1 | 🟡 |
| 13 | Codebase Alignment | 3 | 🟠 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| 🔴 CRITICAL | 1 | PF-001 decided (Option A); fix pending |
| 🟠 MAJOR | 5 | PF-002 approved via gate packet; PF-003 — PF-006 decided (Option A); fixes pending |
| 🟡 MINOR | 4 | PF-007 — PF-010 decided (Option A); fixes pending |
| 🔵 OBSERVATION | 2 | PF-011, PF-012 decided; fixes pending |

**Challenger disclosure (recommendation hardening):** one independent `design-challenger` received the CRITICAL/MAJOR batch (statements + surviving options, without the auditor's picks) plus this recon. Verdicts: PF-002/PF-003/PF-004/PF-006 converged; PF-001 converged on the full fix (Medium confidence); PF-005 diverged on remedy — the challenger preferred narrowing R5 to the AR-9 baseline and filing the lambda/closure ABI as a new defect, which this report adopts with the divergence recorded.

---

### PF-001: Statement-end release turns accepted views into dangling pointers 🔴 CRITICAL

**Dimension:** 3 — Logical Contradictions
**Location:** `01-requirements.md` R2; `03-01-materialization-and-temps.md` rule 1; `00-ambiguity-register.md` AR-5
**Codebase Evidence:** `compiler/src/own/lower.rs:523-553` (the only `Materialize` constructor), `:689-720` (`materialize_owned_argument` moves its binding to the function scope precisely because a derived view can outlive the block), `compiler/src/own/place.rs:268-343` (view provenance/loans do not name compiler temporaries), `compiler/src/own/liveness.rs` (E10030); probe: `let v: str = mk()[0];` accepted, emitted `cl_..._mk(__cl_tmp0); v = cl_string_view(__cl_tmp0[0]);`, no drop.
**The Problem:** R2/03-01 release every owning materialized temporary at statement end, but an accepted program can derive a view into that temporary (`let v: str = mk()[0];`) and use the view in later statements. The release frees the storage the view points into (use-after-free / BR-1 violation). The prior array-value-integrity plan explicitly recorded "views bound from such a temporary are included in that follow-up [DEF-23]"; this plan never mentions views, and its Risks table only covers the empty-temp case. Views cannot escape a function (returned call-rooted views are E10036; view-typed fields are E10205), but a local view binding used in a later statement is enough to dangle.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Extend view/loan provenance to statement-scoped temporaries: a use of a view derived from a temporary after its statement violates the existing loan rule (E10030). | Spec-complete (D5/O2 drop + BR-1); reuses the existing loan/liveness surface and diagnostic; covers all derived views. | Adds an ownership-lifetime surface (PF-002 gate); rejects currently accepted programs; needs new ST cases. |
| B | Do not release owning temporaries at statement end; bind them at function scope like `materialize_owned_argument` does, and record the D5/O2 timing deviation as a limitation. | Smallest change; no new surface; views stay valid; leak fixed; matches the codebase's own justified pattern. | Deviates from the frozen spec's statement-end rule; R2 must be rewritten; timing tests absent. |

Option C (release only temporaries with no derivable view) was dropped as not soundly decidable: slices built by `slice(...)` carry `Provenance::None`, view-array element stores are accepted, and the predicate cannot be made reliable.

**Recommendation:** Option A, contingent on the PF-002 gate approval; if the user declines the added surface, Option B with a corrected R2 and a recorded limitation is the fallback.
**User Decision:** Resolved — User chose Option A (extend view/loan provenance to statement-scoped temporaries; a post-statement use of a derived view falls under the existing loan rule). Artifact fix pending.

---

### PF-002: Complexity Escalation Gate not honored for the DEF-23 ownership surface 🟠 MAJOR

**Dimension:** 10 — Scope Creep Indicators
**Location:** `00-index.md` "Minimum-Sufficient Baseline" (`Excluded machinery: None — no larger support surface was considered`, `Approved complexity: None`); AR-5
**Codebase Evidence:** prior finding PF-005 in `plans/array-value-integrity/00-preflight-report.md:107-115` and its challenger verdict ("Option A — statement-scoped temp release plus view promotion/rejection — is spec-complete but adds a new ownership surface requiring Complexity Escalation Gate approval"); `plans/00-roadmap.md` DEF-23 row ("views bound from them are unhandled"); probe evidence in PF-001.
**The Problem:** The correct DEF-23 fix (PF-001 Option A) adds a distinguishable ownership-lifetime surface: statement-scoped temporary provenance plus a view-lifetime rule in the existing loan/liveness stage. The plan asserts no larger surface was considered and has no approval. Under the shared Complexity Escalation Gate, an unapproved escalation is at least MAJOR and blocks a pass, and generic finding acceptance does not approve named machinery.

**Required visible gate packet (must be recorded with the user ruling):**

| Field | Value |
|-------|-------|
| Original goal | Release every owning compiler temporary at statement end (DEF-23; `spec/17-codegen.md:80`, D5/O2) so accepted programs stop leaking. |
| Named extra system/support code | Statement-scoped temporary ownership in view provenance/loans (`compiler/src/own/place.rs:268-343`, `own/statements.rs:351-381`) and the existing E10030 liveness rule; plus the statement-position release path of PF-003. |
| Why it may be needed | Without it the release frees storage a live derived view points into (PF-001); BR-1 ("a loan never outlives its owner", `spec/16`/`spec/04`) otherwise has no owner to anchor the view to. |
| Codebase evidence | Probe `let v: str = mk()[0];` accepted with no drop; `materialize_owned_argument`'s documented view rationale (`own/lower.rs:681-688`). |
| Smallest solution that still works | PF-001 Option B — function-scope binding of owning temporaries; fixes the leak without the new surface but keeps D5/O2 timing as a recorded deviation. |
| Extra cost | New provenance/release surface, changed acceptance (rejects currently accepted view bindings), new diagnostics behavior and tests. |
| Independent verdict | Challenger recommended Option A at Medium confidence; converged with the auditor. |
| User decision | **Pending — explicit approval of the named machinery or selection of the smaller solution is required.** |

**Options:** A — approve the named surface via this packet (resolves PF-001 toward Option A). B — reduce scope so no surface is added (PF-001 Option B; correct the index and AR-5 claims). C — remove DEF-23 from this plan and keep the leak/deferred status.
**Recommendation:** A with the recorded packet; B is the smaller authorized path if the user prefers no new ownership surface.
**User Decision:** Resolved — User explicitly approved the named machinery in the gate packet (statement-position release path plus statement-scoped view/temp loan provenance) on 2026-09-29; PF-001 proceeds as Option A. Approval evidence is the packet table above plus this ruling.

---

### PF-003: `Statement::Drop` cannot release at statement end 🟠 MAJOR

**Dimension:** 13 — Codebase Alignment
**Location:** `03-01-materialization-and-temps.md` rule 1 and Integration Points ("the new `Drop` statements reuse them"); `99-execution-plan.md` task 1.2.1
**Codebase Evidence:** `compiler/src/ir.rs` `Statement::Drop { place, scope, order }`; `compiler/src/codegen/cleanup.rs:39-52` (collects every Drop into per-scope actions), `:122-199` (emits scope cleanup only at ScopeEnd/ScopeExit), `:231-244` (`emit_actions`); `compiler/src/codegen/function.rs:423-427` (Drop is a no-op inline); `compiler/src/own/statements.rs:54-96` (the block walk stops at a terminator, so a post-statement drain misses `return`/`break`/`continue`).
**The Problem:** The plan's core DEF-23 mechanism ("At the end of the statement, the walk emits one `Statement::Drop` for each pending temporary") does not produce statement-end code: every `Drop` statement is collected as a scope cleanup and rendered at the enclosing scope's end or ladder rung. A drain placed after `check_stmt` in `check_block` also never runs for a terminated statement. So R2's timing cannot be met as written, and no ST case observes the difference (the ASan probes only check that memory is eventually released).

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Add a statement-position release path (a dedicated inline IR statement or an emitter-side temp-release arm) and drain pending temps before each terminator. | Meets R2 and `spec/17-codegen.md:80`; keeps binding drops untouched. | Small new IR/emitter surface; must be named in the PF-002 packet. |
| B | Accept scope-scoped release, rewrite R2, and record the deviation from D5/O2 as a limitation. | No new machinery. | Contradicts the plan's stated oracle and R2; changes the PF-001 hazard window. |

**Recommendation:** Option A (challenger and auditor converged).
**User Decision:** Resolved — User accepted the recommendation (Option A). Artifact fix pending.

---

### PF-004: Release-target predicate is undefined and can double-free adopted values 🟠 MAJOR

**Dimension:** 1 — Ambiguities
**Location:** `01-requirements.md` R2 ("A materialized temporary that owns a value…"); `03-01-materialization-and-temps.md` rule 1 ("each materialized temporary whose type needs a drop"); `99-execution-plan.md` task 1.2.1
**Codebase Evidence:** `own/lower.rs:560-570` (`materialize_value`) and `:580-599` (`consume_place`) create `ValueKind::Temp` temps; probes: `let r: String = sink(take s);` emits `__cl_tmp0 = sink(s); s = NULL; r = __cl_tmp0;`; `b = Box { value: b.value };` emits `__cl_tmp0 = b.value; b.value = NULL; b = (Box){ __cl_tmp0 };` — both destinations now own the value and the temp is intentionally left aliased.
**The Problem:** The plan never states which temporaries join the release list. A natural reading ("materialized temporary" = any `materialize_value`/`consume_place` temp) releases values already adopted by `r`/`b` and double-frees them. A narrow reading (only `ValueKind::Materialize`) is correct but is not what the plan says; the new iterable and conditional-read materializations must also be registered.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Pin the release set to the destination materializations (the `Materialize` guard constructor plus the new iterable and conditional-read temps) and explicitly exclude the transfer temps, with a unit test pinning the registered sites. | Minimal; matches the code; existing DEF-11/take tests keep guarding the excluded paths. | Requires exact wording and a guard test. |
| B | Zero every transfer temp after its value is adopted so releasing all temps is safe. | Uniform rule. | Changes existing emitted shapes; adoption happens inside composite initializers; higher regression risk. |

**Recommendation:** Option A, with a unit test that the registered constructors are exactly the `Materialize` (destination) sites (challenger converged).
**User Decision:** Resolved — User accepted the recommendation (Option A). Artifact fix pending.

---

### PF-005: R5/03-02 promise closure and lifted-lambda ABI coverage the fix cannot deliver 🟠 MAJOR

**Dimension:** 13 — Codebase Alignment
**Location:** `01-requirements.md` R5; `03-02-callable-fields.md` §Proposed Changes 2 ("This covers … closure structs (`codegen/program.rs:267-281`), and lifted lambdas (`codegen/function.rs:198-220`), because they share the helpers")
**Codebase Evidence:** `codegen/program.rs:267-283` and `codegen/function.rs:198-220` build the return and parameters inline with `c_type`, not through `func_pointer_type`/`func_pointer_declarator` (`codegen/types.rs:58-73`); probe: `let g: func(i32): i32[2] = (x: i32) => { return [x, x + 1]; };` is accepted and emits `static i32 cl_lambda_0(i32 x)`, `i32 (*g)(i32)`, and `g(3, __cl_call_1)` — invalid C.
**The Problem:** R5 and 03-02 claim the shared declarator fix covers closure structs and lifted lambdas. It does not; those two sites need separate changes (and the closure-call destination), and no ST case (ST-16/ST-17 cover a struct field and a local func value only) would catch the gap. AR-9's approved option scopes the fix to "fields, locals, parameters, discarded calls".

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Narrow R5/03-02 to the AR-9 baseline (fields, locals, parameters, discarded calls through those) and file the closure/lifted-lambda fixed-array return as a new defect (accepted today, invalid C). | Matches the authorized baseline; keeps the plan honest; follows the prior plans' file-a-follow-up pattern. | Leaves an accepted-broken form; the overview's "every accepted program compiles" wording must be qualified. |
| B | Extend the fix to `emit_closure_declarations` and `lifted_signature`, plus closure-call destinations and new ST cases. | Fixes the accepted-broken form; keeps one ABI story. | Beyond the filed DEF-27 and AR-9's wording; larger implementation; new scope needing authorization. |

**Recommendation:** Option A (challenger's pick; the auditor's initial lean was B, and the divergence is resolved toward the authorized baseline). The user may authorize B explicitly.
**User Decision:** Resolved — User chose Option A (narrow R5 to the AR-9 scope; file the closure/lifted-lambda fixed-array return as a new defect). Artifact fix pending.

---

### PF-006: DEF-25 claims all value positions, but the return and indexed-element paths do not route through the shared store 🟠 MAJOR

**Dimension:** 13 — Codebase Alignment
**Location:** `01-requirements.md` R3; AR-16 (`00-ambiguity-register.md:58`); `03-01` rule 3 ("The shared fixed-array value assignment path … and its callers: initializer, assignment, return destination, argument materialization, and a read position")
**Codebase Evidence:** `codegen/cleanup.rs:308-361` — `emit_fixed_array_return` falls back to `__cl_ret = (ok ? a : b);` (invalid C) and never calls `emit_fixed_array_assign`; `codegen/function.rs:390-399` — `Statement::StorePointer` emits `rows[0] = (ok ? a : b);` (invalid C; plain `rows[0] = a;` is also invalid today); no argument materialization exists for a fixed-array conditional (ST-9 passes pre-fix via C array decay).
**The Problem:** ST-8 (`return ok ? a : b;`) is an immutable plan case, but the plan's implementation recipe (making the shared fixed-array assignment path branch-aware) does not touch the return emitter, so ST-8 would stay red. AR-16's "every position … through the shared value-assignment path" is factually wrong for the return, indexed-element, and argument positions; the indexed-element store and nested-literal row stores are broader pre-existing defects (accepted, invalid C) that the plan silently inherits.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Extend the fix to `emit_fixed_array_return` (branch-store into `__cl_ret`, staying within the destination type), narrow AR-16 to the positions that actually route through the shared path, and file the indexed-element/nested-literal row store as an adjacent defect. | Makes ST-8 green; keeps claims true; follows the file-a-follow-up pattern. | One more emitter site; a new defect row. |
| B | Narrow R3/AR-16 to the routed positions and drop ST-8. | Smallest claims. | Drops a mandatory-case promise and leaves a spec-accepted form broken. |

**Recommendation:** Option A (challenger and auditor converged).
**User Decision:** Resolved — User accepted the recommendation (Option A). Artifact fix pending.

---

### PF-007: ST-9's red-phase prediction is wrong (the case passes pre-fix) 🟡 MINOR

**Dimension:** 7 — Testability
**Location:** `07-testing-strategy.md:80` (red-phase table lists ST-9 with the "invalid C" group); ST-9 case at `:47`
**Codebase Evidence:** probe `eat(ok ? a : b)` with `eat(take arr: i32[2])` compiles (`cl_..._eat((ok ? a : b));`, arrays decay) and runs to `4`.
**The Problem:** The executor is told a deviation from the recorded red phase is an oracle/test defect; ST-9 passing pre-fix will look like a deviation. Also, because the duplicable argument compiles pre-fix, ST-9 does not exercise the new branch-aware store for the owned-arm argument case the requirement claims.

**Options:** A — move ST-9 to the "passes pre-fix" group as a behavior guard and add an owned `String[2]` conditional-argument case to exercise the branch store. B — move ST-9 and change nothing else.
**Recommendation:** Option A.
**User Decision:** Resolved — User accepted the recommendation (Option A). Artifact fix pending.

---

### PF-008: ST-17's discarded local func-value call needs a local-callee return-type lookup 🟡 MINOR

**Dimension:** 6 — Feasibility Concerns
**Location:** `03-02-callable-fields.md` §Proposed Changes 2 ("`emit_call` resolves the callee's return type from the callee value's recorded type when it is not a named function"); task 2.2.2
**Codebase Evidence:** `own/call.rs:137-185` — a local func value lowers with `receiver: None` and `callee` = the local name; `codegen/function.rs:580-586` — `callee_return_type` resolves named functions only; `Statement::Call` carries no callee value. Probe `g(3);` with `g: func(i32): i32[2]` needs the local's declared type to append the destination.
**The Problem:** The stated mechanism ("the callee value's recorded type") has no callee value for a plain local `g(3)`; the discard path (ST-17) cannot append a destination without resolving the local's type by name.

**Options:** A — extend the plan's mechanism: resolve a func-typed local callee from the function's locals by name (the emitter already maps locals) and use its return type for the destination/discard decision; add it to task 2.2.2. B — carry the callee type on the IR call (larger IR change).
**Recommendation:** Option A.
**User Decision:** Resolved — User accepted the recommendation (Option A). Artifact fix pending.

---

### PF-009: Stale and imprecise citations and claims 🟡 MINOR

**Dimension:** 12 — Consistency
**Location / Evidence:**

| Claim | Actual |
|-------|--------|
| `02-current-state.md:32`, `03-01:21-23`: `FunctionChecker::temps: Vec<Ty>` at `own.rs:888-892` | Field is `own.rs:160-161`; `:888-892` is `new_temp` |
| `03-03:29-31`: short-circuit lowering at `own/lower.rs:149-157` | That range is `?:` only; `&&`/`||` are at `own/lower.rs:114-133` |
| `02-current-state.md:76-78`: the emitter treats the temp as inert at `codegen/guards.rs:319` | `:319` is the `_ => {}` catch-all; the eager behavior is created by `materialize_value` at `own/lower.rs:102-106` |
| AR-5: reverse creation order per `spec/decisions.md:28` (D5, O2) | D5 states statement-end drop; reverse order is `spec/17-codegen.md:80`, `spec/17-codegen.md:212-213`, `spec/decisions.md:369` (PF-040) |
| `03-03:22`: `String` renders a `void *` at `codegen/types.rs:87` | `:87` returns the typedef name `ClString`; the `void *` typedef is `codegen/program.rs:98` |
| `01`: acceptance criterion 1 — "Every requirement R1–R8 traces to at least one passing specification test case" in the new suite | R8 (guard behavior unchanged) is guarded by existing suites, not by a new ST case |
| `01` Won't Have: "A slice-typed or view-typed call-rooted iterable whose storage outlives the callee" | The wording suggests the safe case is excluded; the materialization exclusions are fixed-array/value forms while slice call results stay inline |

**Options:** A — correct the citations/claims in one documentation pass. B — leave as minor.
**Recommendation:** Option A.
**User Decision:** Resolved — User accepted the recommendation (Option A). Artifact fix pending.

---

### PF-010: Untested positions and unhandled generic iterables 🟡 MINOR

**Dimension:** 7 — Testability
**Location:** `03-01` rule 3 read-position materialization; AR-16; R1/R3
**Codebase Evidence:** `own/lower.rs:534-539` — `materialize_guard_operand` refuses a type mentioning a type parameter because it cannot be declared as a C temp; `own/loops.rs:396-407` (the new iterable temp).
**The Problem:** No ST case exercises a conditional value in a read position (index base/element source) or a conditional iterable, although AR-16 and 03-01 describe that machinery. A call-rooted iterable inside a generic body whose element/array type mentions a type parameter hits the same C-spelling limit the guard materializer documents, and the plan's iterable rule does not mention it (skip, diagnose, or record a limitation).

**Options:** A — add ST cases (conditional index base / conditional iterable) and state the generic-iterable behavior explicitly (follow the `mentions_type_param` precedent or record a limitation). B — narrow the rules to the non-generic tested positions.
**Recommendation:** Option A.
**User Decision:** Resolved — User accepted the recommendation (Option A). Artifact fix pending.

---

### PF-011: Panic printing semantics are unspecified at the edges 🔵 OBSERVATION

**Dimension:** 9 — Edge Cases
**Location:** `03-03-runtime-boundary.md` §Proposed Changes 1
**Codebase Evidence:** `shim_host.c:47-52` (`fprintf(stderr, "panic: %s\n")`), `shim_hosted_amiga.c:79-84` (`printf`, libnix), `shim_freestanding.c:218-222` (`cl_report` with a 256-byte buffer at `:194-207`); the plan says `%.*s` "print exactly `len` bytes".
**The Problem:** `%.*s` stops at an embedded NUL, so it does not print exactly `len` bytes for arbitrary `str`/`String` bytes; the freestanding shim truncates at its 256-byte buffer; libnix `%.*s` support is not verifiable in-repo.
**Recommendation:** State the exact behavior (write `min(len, buffer)` bytes through a length-based writer, or explicitly accept truncation/NUL-stopping), and verify the m68k legs.
**User Decision:** Resolved — User accepted the recommendation. Artifact fix pending.

---

### PF-012: The ST-17 AddressSanitizer probe is vacuous 🔵 OBSERVATION

**Dimension:** 7 — Testability
**Location:** `07-testing-strategy.md:116`, `99-execution-plan.md` task 4.1.2
**The Problem:** ST-17's discarded func value returns `i32[2]`, which owns no heap elements, so a leak probe on it proves nothing.
**Recommendation:** Use an owning element type (e.g. `String[2]`) or drop ST-17 from the ASan list.
**User Decision:** Resolved — User accepted the recommendation. Artifact fix pending.

---

## Adversarial checklist (same-agent bias)

- "What assumption made during creation might be unconsciously confirmed now?" — that "materialized temporary" unambiguously meant the guard `Materialize` temp (PF-004), and that `Statement::Drop` releases where it is emitted (PF-003). Both were checked against emitted C and the cleanup pass.
- "What external standard or convention might this violate?" — the in-repo frozen specification was read directly for D5/O2, §9 loops, panic, `String`/`str`, and the diagnostics registry; the plan's citations that overreach are listed in PF-009.
- "What would a domain expert who disagrees flag as wrong?" — a Rust/ownership expert would flag the statement-end drop of a borrowed temporary without a lifetime rule (PF-001/PF-002); a C codegen expert would flag the four invalid-C accept paths (PF-005, PF-006 probes).
