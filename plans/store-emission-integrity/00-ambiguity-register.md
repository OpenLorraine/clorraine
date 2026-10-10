# Ambiguity Register: Store-Emission Integrity

> **Status**: ✅ GATE PASSED — all 18 items resolved
> **Last Updated**: 2026-10-10 08:12

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| AR-1 | Scope | Which defects this plan fixes | DEF-113–116 (the four majors filed by the borrow-store-integrity preflight and Phase 3 reviews) / DEF-113 only / all 12 open majors | **User:** chose DEF-113–116 — the conditional-store use-after-free, the `str` element store, the slice typedef order, and the module-initializer array/slice emission. One batch plan; folder `store-emission-integrity`. | ✅ Resolved |
| AR-2 | Behavioral | DEF-114: which diagnostic rejects a write through an immutable view | New code E10042 + registry row + compile-fail golden / reuse E10221 / reuse E10023 | **User:** chose the new code — `spec/14-diagnostics.md` gains an `E10042` row and `spec/tests/compile-fail/` gains its golden; the plan carries this spec edit as the user's explicit ruling. | ✅ Resolved |
| AR-3 | Scope | DEF-115: whether the fix covers the adjacent ordering instances | Cover all three verified instances (slice-of-struct, slice-of-enum, struct-with-enum-field) / strict filed case only | **User:** cover all three — one emission-order pass fixes the class; the two adjacent instances stay in the plan's tests. | ✅ Resolved |
| AR-4 | Technical | DEF-113: how the conditional store stops being suppressed | Exclude the exact-target arm from the conditional walks before the lend test / a new duty record / arm-wise lowering / blanket removal of the suppression | **AI — delegated by --auto-design (eligible: compiler mechanism; conforms to BR-5 and the pinned concrete spellings).** `lend_moves_judged` excludes conditional leaves equal to the exact target path before the `any(lend)` test; the direct-place branch is unchanged. See notes. | ✅ Resolved (delegated) |
| AR-5 | Behavioral | DEF-113: the diagnostic per shape and substitution | Concrete owning → `E10034` on the target; generic → the existing per-instance duty; nullable out-slot shape unchanged | **AI — delegated by --auto-design (eligible: conformance to the pinned concrete rules).** Each shape reproduces the concrete spelling's existing report; the nullable out-slot conditional stays accepted (verified safe). See notes. | ✅ Resolved (delegated) |
| AR-6 | Technical | DEF-114: where the check lands and which write forms it covers | Ownership layer beside `E10023`/`E10024` (`write_place` + the global write branches) / sema layer / emitter-side cast | **AI — delegated by --auto-design (eligible: compiler mechanism).** A shared ownership-layer helper fires when the written place's final projection is an element whose base type is `str` or `static str`; it covers plain, compound, and update forms on locals, parameters, fields, and globals. See notes. | ✅ Resolved (delegated) |
| AR-7 | UX & presentation | DEF-114: the exact `E10042` registry text | Draft registry row (trigger / first line / help) | **AI — delegated by --auto-design (eligible: diagnostic wording within the user-approved code).** Trigger: "Write to an element of an immutable view (`str` or `static str`)"; first line: `cannot write through immutable view '{name}'`; help: "copy the bytes into an owned `String` or a `borrow`-derived `u8[]` view". See notes. | ✅ Resolved (delegated) |
| AR-8 | Technical | DEF-115: the corrected typedef emission order | Enums → struct forward typedefs → slice typedefs → struct definitions, with handles ahead of slices / forward-declare inside `emit_slice_typedefs` | **AI — delegated by --auto-design (eligible: emission mechanism).** Restructure `codegen.rs::run` + `codegen/program.rs` so every typedef precedes its uses; the dependency graph is acyclic (view-typed fields are banned by `E10205`; closures are emitted later). See notes. | ✅ Resolved (delegated) |
| AR-9 | Technical | DEF-116: how an array-typed global initializer is emitted | Element-wise stores mirroring the function-body element stores / a compound-literal form / a `memcpy` from a temporary | **AI — delegated by --auto-design (eligible: emission mechanism).** Array-typed initializers fill element-wise in declaration order; nested extents recurse; a non-literal array value is copied element-wise from its rendered base. See notes. | ✅ Resolved (delegated) |
| AR-10 | Technical | DEF-116: how a module-level intrinsic call is lowered | Mirror `lower_intrinsic_call`'s detection in `lower_global` / resolve to a library symbol / special-case `slice` only | **AI — delegated by --auto-design (eligible: compiler mechanism).** `lower_global` detects a compiler-intrinsic callee exactly as the function path does (never replacing a user or `ffi` declaration) and builds the intrinsic value; `emit_guards` already carries the slice bounds trap. See notes. | ✅ Resolved (delegated) |
| AR-11 | Edge cases | DEF-116: evaluation count of a side-effecting intrinsic operand | Materialize impure operands into init-function locals (operand-once) / render directly (double evaluation) / reject impure offsets | **AI — delegated by --auto-design (eligible: correctness mechanism).** Operands that are not pure values are materialized into `__cl_itmpN` locals in the init function before the construction, mirroring `materialize_guard_operand`'s rule. See notes. | ✅ Resolved (delegated) |
| AR-12 | Scope / process | The `spec/` modification set | The E10042 registry row + one `E10042` golden, in the DEF-114 phase only / no spec change / wider clarifications | **User ruling (AR-2) bounds it; AI records the boundary.** Only `spec/14-diagnostics.md`'s row and `spec/tests/compile-fail/E10042_write_immutable_view.clo` change; no other chapter, and no golden is ever adjusted to the implementation. | ✅ Resolved |
| AR-13 | Technical | Verify commands, test layout, and memory oracle | `make check` per phase, ASan host probes for the memory cases, `make verify` final; suites `store_emission_integrity_{spec,impl}_test.rs`; the golden runs in `./spec/check.sh` | **AI — delegated by --auto-design (eligible: testing strategy).** Commands detected from `AGENTS.md`/the `Makefile` (never invented); the in-container `RDC_ASAN=1` run stays a non-gating convenience; host `cc -fsanitize=address -no-pie` probes are the phase memory oracle. | ✅ Resolved (delegated) |
| AR-14 | Sequencing | Phase structure and phase-review dispatch | One phase per defect (4); reviews per the `AGENTS.md` change-surface matrix | **AI — delegated by --auto-design (eligible: implementation sequencing, process routing).** P1 DEF-113, P2 DEF-114, P3 DEF-115, P4 DEF-116; P1/P2 get correctness + memory-safety + semantics review; P3/P4 get correctness + memory-safety + emission-integrity review. See notes. | ✅ Resolved (delegated) |
| AR-15 | Scope | Adjacent shapes examined and left out | cstring writes; `const` array initializers; DEF-97/DEF-98; handle-element slices; the nullable out-slot conditional | **AI — delegated by --auto-design (eligible: verified no-action scope boundaries).** cstring/`const`-array/DEF-97-98 are not filed defects of this batch; the nullable out-slot conditional is verified safe and pinned as an acceptance test. See notes. | ✅ Resolved (delegated) |
| AR-16 | Process | Execution commit mode | `--auto-commit` (user instruction) / ask / none | **User instruction:** record `--auto-commit` for execution — commit and push each verified task via the git-commit skill (repo precedent: `defer-integrity`). | ✅ Resolved |
| AR-17 | Naming | Plan folder, suites, and internal names | `store-emission-integrity`; `store_emission_integrity_{spec,impl}_test.rs`; behavior-naming for tests | **AI — delegated by --auto-design (eligible: internal naming, zero external surface).** Follows the repo's `-integrity` batch naming and suite conventions. | ✅ Resolved (delegated) |
| AR-18 | Scope | DEF-116: whether a conditional array value at module level is covered | Include in DEF-116 (element-wise, condition evaluated once) / leave out and file a new defect | **User:** include — verified same emission site and same class (`let a: i32[2] = pick() ? x : y;` accepted, emits `a = (pick() ? x : y);`); the condition is materialized once and each element copies from the chosen arm. | ✅ Resolved |

## Resolution Notes

**AR-4 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler mechanism; conforms to the frozen BR-5 rule and the pinned concrete spellings; no reserved consequence
Objective: a conditional store through a non-nullable lend root must be judged like the direct store, with one report per site
Decision: in `lend_moves_judged` (compiler/src/own/expr.rs), exclude conditional-walk leaves whose place equals the exact target path before the `any(lend)` test; the direct-place branch keeps today's behavior; the function receives the target place to compare
Evidence: `st(borrow slot: String, pick: bool)` with `slot = pick ? slot : other` (other a local) is accepted and the emitted conditional store frees the caller's saved handle — ASan heap-use-after-free at `cl_string_len` (probe d113, exit 1); the generic `st<T>` with a `take` arm is identical (probe d113e, ASan UAF); the direct self-store `slot = slot` already reports E10034 'slot' through the place-move path (probe p8); `slot = pick ? a : b` with no lend arm already reports E10034 'slot' (probe p4); a direct store from a read-only lend reports E10034 on the source (probe p1); a conditional of two read-only lends reports once per arm (probe p3); the marking path already exempts "an arm that names the target path itself" (compiler/src/own/expr.rs:554-576) while `lend_moves_judged` still counts it — the suppression then silences the store rule exactly when the only lend source is the target. The `any(lend)` suppression was narrowed deliberately to keep one report per site for mixed conditionals (borrow-generic-integrity RV-001/SR-004) and its pinned case (`slot = c ? local : v`) stays reported; a self-arm plus borrow-arm composition reports once on the borrow arm (probe p9: one `E10034` on `v`), which the filter preserves because the non-self lend arm still suppresses
Rejected alternatives: (a) a new duty record — the existing `RequiresStorableSlot` duty already yields the per-instance judgment once the suppression stops firing; (b) accepting and lowering arm-wise — contradicts BR-5 (the callee may not store through a lend) and keeps the caller's handle freed on the non-slot arm; (c) blanket removal of the suppression — double-reports direct stores (probe p8) and regresses pinned behavior
Strongest counterargument: excluding the self arm could reopen a site the source side really reports — verified not to for conditional arms (d113/d113e are silent today), and direct-place values keep the suppression
Confidence: High — every affected shape is pinned by a probe
Hardening: in-context layers only (medium stakes: conformance fix with a probe per shape)
Policy version: 1
Root invocation ID: plans-store-emission-integrity-20261010
Reopen triggers: a conditional arm that the source side does report and the store rule now repeats; a mixed conditional whose non-self lend arm stops being reported by the source side; a spec change to BR-5 or the out-slot carve-out
```

**AR-5 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: conformance to the confirmed concrete rules (AR-4's mechanism); no new language surface
Objective: one mapping from shape to report for the DEF-113 family
Decision: concrete owning target → exactly one E10034 naming the target ('slot'), matching probes p2/p4/p8; a still-parameter-typed target → the existing `RequiresStorableSlot` duty per instance (owning substitution → E10034 'slot'; duplicable substitution → accepted, probe p4-class behavior; nullable substitution → accepted per the out-slot carve-out); a conditional whose only lend source is a read-only arm keeps the source-side report ('other', probe d113c); the nullable out-slot conditional (`g(borrow slot: String?)`) stays accepted — its emission nulls the save on the self arm (probe d113d, exit 4 = the program's length, ASan clean)
Evidence: probes d113, d113e, d113d, d113c, p2, p3, p4, p5, p8 (paths in 02-current-state.md)
Rejected alternatives: one diagnostic for every substitution (would diverge from a pinned concrete spelling)
Strongest counterargument: the generic non-nullable and nullable instances share one declared type parameter, so the audit must map them differently — it already does (instance_check.rs `check_storable_slot_requirements`)
Confidence: High
Hardening: in-context layers only
Policy version: 1
Root invocation ID: plans-store-emission-integrity-20261010
Reopen triggers: a concrete spelling that behaves differently from the mapped substitution; a change to the storable-slot audit
```

**AR-6 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler mechanism; enforces the frozen mutability table (Ch 02 §6: `str` immutable); no reserved consequence
Objective: every write form through an immutable view is rejected once, with the new E10042
Decision: a shared ownership-layer helper called from the local write path (`write_place` — plain, compound, and update forms all route through it) and from the global branches of `check_assign`/`check_update`; it fires when the written place's last projection is an element (Element or ElementTemp) and the type at the projection prefix is `str` or `static str`; the report names the binding (or `binding.field` for a field view)
Evidence: accepted and cc-failing at baseline — `borrow s: str` store (probe d114), local store (d114c), `static str` local (d114d), compound (p2a), update (p2b), `static str` field (p2c), global view store (p2f); accepted and cc-clean controls — whole-view rebind (p2d), writable slice element store (p2e); `const`-mode parameter writes already report E10024 (d114b); cstring writes report E10024 through read-only modes (d114f) and are out of scope
Rejected alternatives: (a) sema-layer placement — splits the write-mutability taxonomy (E10023/E10024 live in own/place.rs) and duplicates reachability logic; (b) emitter-side cast — defeats const-ness and is undefined behavior for literal storage
Strongest counterargument: rejecting previously accepted programs is a behavior change — but the accepted programs emit C that no conforming C compiler compiles, and the spec already declares the view immutable
Confidence: High
Hardening: in-context layers only
Reopen triggers: a spec rule that admits writes through `str`; a write form the helper misses (pinned by the ST cases)
```

**AR-7 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: diagnostic wording pinned by the user-approved code (AR-2); registry style follows the existing rows
Objective: one registry row whose trigger, first line, and help state the rule
Decision: `| E10042 | Write to an element of an immutable view (`str` or `static str`) | cannot write through immutable view '{name}' | copy the bytes into an owned `String` or a `borrow`-derived `u8[]` view |`
Evidence: registry style (spec/14-diagnostics.md §3 rows E10023/E10024/E10041); the writable alternative in the help is verified accepted and running (probe p2e)
Rejected alternatives: a help that suggests `borrow` alone (the parameter may already be `borrow`; the fix is the view type, not the mode)
Confidence: High
Hardening: not required (wording only)
Policy version: 1
Root invocation ID: plans-store-emission-integrity-20261010
Reopen triggers: a wording review at preflight or in the semantics review
```

**AR-8 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: emission mechanism; restores the emitted C to the strict-subset contract; no language surface
Objective: every emitted typedef precedes its uses
Decision: `codegen.rs::run` emits, in order — preamble + handle typedefs (existing `emit_typedefs`, minus slices); enum typedefs and member macros; struct forward typedefs; slice typedefs and zero helpers; struct definitions; closure declarations; prototypes; globals; init functions; bodies
Evidence: cc failures at baseline — `ClSlice_P` before `ClP` (probe d115), `ClSlice_E` before `ClE` (d115b), `ClE` inside a struct definition (d115d); slices reference element typedefs through pointers (`pointer_to_array_declarator`), which only need a prior declaration; view-typed fields are banned (E10205), so no struct depends on a slice typedef except closure environments, which are emitted after; a closure capturing a slice builds today (probe p3a) and must stay green
Rejected alternatives: (a) forward typedefs nested inside `emit_slice_typedefs` — repeats typedefs (a C99 constraint violation and a warning/error risk across the compile matrix); (b) inline anonymous element structs inside the slice typedef — changes every slice spelling and the emission pin
Strongest counterargument: moving enums earlier changes emission for every unit that declares an enum — intended, deterministic, and covered by the corpus/emission audits
Confidence: High
Hardening: in-context layers only
Reopen triggers: a unit where a slice typedef must precede a struct forward typedef (none exist: view fields are banned); a compile-matrix change
```

**AR-9 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: emission mechanism; mirrors the function-body element-store semantics exactly; no new syntax
Objective: a module-level fixed-array initializer compiles
Decision: an array-typed global initializer fills element-wise in declaration order (`target[i] = …`), recursing one extent per level for nested arrays; `ArrayLit` elements render in order and the fill renders per remaining element exactly as the function-body element stores already do; a non-literal array value whose base is a place copies element-wise from its rendered base; call-result array values stay out of this batch (see the preflight correction note below)
Evidence: baseline accepted and cc-failing — flat literal (probe d116), nested extents (p4a), prefix+fill (p4b), a place copy (p4d), and a slice-of-array (p4e), all with `expected expression before '{'`; the function-body reference shape emits per-element stores (`a[0] = 1; a[1] = 2; a[2] = 7;` for `[1, 2; 7]`, probe fill1); the call-result form is not pinned by any probe and emits a destination-less call (preflight finding PF-001, verified live)
Rejected alternatives: (a) a compound-literal assignment — forbidden by the C99-subset contract (Ch 17 §1) and an existing tracked violation class; (b) a `memcpy` from a temporary — needs a temp registry the init emitter does not have
Strongest counterargument: per-element stores re-evaluate a side-effecting fill per element — matching the existing function-body semantics is the only consistent behavior; the ST cases pin constants and mirror the function path
Confidence: High
Hardening: in-context layers only
Reopen triggers: an initializer form the element walk cannot express; a change to the function-body array-literal semantics
```

> **Preflight correction (2026-10-10, PF-001):** the original decision sentence named "a call
> result" among the copied non-literal array values. Fixed-array call results are not covered:
> they lower without their hidden destination, so the batch keeps their current single-assignment
> path. The decision and evidence rows above carry the corrected boundary; the verified class is
> recorded in the preflight report for a separate decision.

**AR-10 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler mechanism; mirrors the established function-path detection; no reserved consequence
Objective: a module-level call to a compiler intrinsic lowers to the intrinsic value instead of a bare name
Decision: `lower_global`'s call branch checks whether the callee is a compiler intrinsic with the same rule as `own/call.rs::lower_intrinsic_call` (a user or `ffi` declaration of the name keeps its own symbol) and builds `ValueKind::Intrinsic { name, args, type_arg }`; detection resolves the callee by simple name against the program's callee map plus `is_compiler_intrinsic`, so the helper takes only the callee expression (no module parameter); `emit_guards` already emits the slice bounds trap for intrinsic values in the init function
Evidence: baseline emits the bare `slice(...)` in `cl_init_<module>` (probe d116, implicit-declaration errors); a user-function initializer resolves and mangles correctly (`cl_repros_g1_make()`, probe g1), so only the intrinsic path is missing; the renderer (`codegen/calls.rs::intrinsic`) and the guard arm (`codegen/guards.rs:98`) already exist
Rejected alternatives: (a) resolving to a library symbol — compiler intrinsics have no symbol by design; (b) special-casing `slice` in `lower_global` — diverges from the function path's shared rule and would leave `rawSlice`/`rawStr` inconsistent
Confidence: High
Hardening: in-context layers only
Policy version: 1
Root invocation ID: plans-store-emission-integrity-20261010
Reopen triggers: a compiler intrinsic whose global-context arguments need function-body-only lowering (pin per intrinsic when added)
```

**AR-11 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: correctness mechanism; preserves the operand-once rule the function path already enforces; no new surface
Objective: a side-effecting intrinsic operand runs exactly once in a module initializer
Decision: operands that `is_pure_value` does not accept are materialized into init-function locals (`__cl_itmpN`) before the construction; pure values (constants, places) render directly
Evidence: the function path materializes slice operands via `materialize_guard_operand` (own/lower.rs:698) because the rendered slice expression reads `low` twice; a module-level `slice(backing, bump(), 3)` is accepted today and fails the C build before this fix (probe p4e) — after the fix the double evaluation would be observable
Rejected alternatives: (a) rendering directly — silently double-calls an impure offset; (b) rejecting impure offsets at module level — a new rejection surface the language never stated
Confidence: High
Hardening: in-context layers only
Policy version: 1
Root invocation ID: plans-store-emission-integrity-20261010
Reopen triggers: an operand kind that materialization cannot represent in the init context; a renderer change that reuses an operand more times
```

**AR-12:** The only `spec/` changes are the `E10042` row in `spec/14-diagnostics.md` and the new `spec/tests/compile-fail/E10042_write_immutable_view.clo`; both land in Phase 2 and are the user's AR-2 ruling. All other fixes conform the implementation to the frozen chapters; no golden is ever adjusted to match the implementation.

**AR-13 (delegated):** verify commands detected from `AGENTS.md` and the `Makefile` — `make check` (spec check + workspace tests in the pinned container) per phase; `make verify` (all hosted-amiga legs) once at close-out; host ASan probes (`clc` emit, `cc -std=c99 -g -fsanitize=address -no-pie`, run with leak detection) are the memory oracle for the P1 cases; the in-container `RDC_ASAN=1` suite run stays a non-gating convenience (timing-flaky, matching previous cycles). Suites: `compiler/tests/store_emission_integrity_spec_test.rs` and `store_emission_integrity_impl_test.rs` (repo naming convention).

**AR-14 (delegated):** P1 (DEF-113, the self-arm suppression — ownership semantics) → correctness + memory-safety + semantics reviewers; P2 (DEF-114, the E10042 language rule + spec edit) → correctness + memory-safety + semantics reviewers; P3 (DEF-115, emission order) and P4 (DEF-116, module-init emission) → correctness + memory-safety + the emission-integrity auditor, per the `AGENTS.md` change-surface matrix. Reviewer packets include the blast-radius and test-integrity steps.

**AR-15 (delegated):** verified no-action boundaries: a `cstring` element write through a read-only mode already reports E10024 (probe d114f) and borrow-mode cstring writes are a separate, unfiled mutability question (the repository files defects, not this batch); a `const` array initializer is currently rejected E10224 (probe p4c) — a rejection gap, not an accepted-program breakage; DEF-97/DEF-98 stay in the backlog; handle-element slices could not be exercised syntactically at baseline (probes d115c/d115e) and are covered by the ordering pass anyway; the nullable out-slot conditional is verified safe (probe d113d) and pinned as an acceptance test.

**AR-16:** recorded per the user's invocation: `--auto-commit` → the plan's Key Decisions and execution plan record commit-and-push per verified task; the plan-set commit follows the repository cycle convention.

**AR-17 (delegated):** plan folder `plans/store-emission-integrity/` (flat layout); suites `compiler/tests/store_emission_integrity_spec_test.rs` and `_impl_test.rs`; test names state behavior ("reports … once", "builds and runs …"); internal helpers follow existing naming (`check_*`, `emit_*`).

**AR-18 (user ruling, 2026-10-10):** probe p4f — `let a: i32[2] = pick() ? x : y;` at module level is accepted and emits `a = (cl_..._pick() ? cl_..._x : cl_..._y);`, which fails the strict build (an array ternary decays to a pointer). The user chose to cover it in DEF-116: the fill walk evaluates the condition once into an init local and copies each element from the chosen arm (03-04).

**Scan completeness (12 categories):** Feature gaps — covered by the per-defect requirements (01) and components (03-01 – 03-04). Behavioral — AR-4, AR-5, AR-6, AR-7. Scope — AR-1, AR-2, AR-3, AR-12, AR-15, AR-18. Technical — AR-4, AR-6, AR-8, AR-9, AR-10, AR-11, AR-13. Edge cases — AR-11 and the 03 documents (nested conditionals, duplicable/nullable/view substitutions, nested extents, fill literals, impure offsets, field/global view bases) pinned in 07. Integration points — the checker (`own/expr.rs`, `own/place.rs`), the duty audit (`own/instance_check.rs`), the emitter (`codegen.rs`, `codegen/program.rs`, `codegen/guards.rs`), the diagnostics registry (`spec/14-diagnostics.md`); owned by 03-01 – 03-04. Data & state — no new data model; the existing place/duty/temp machinery only. Security & compliance — memory safety is the security surface for a compiler; the ASan probes (AR-13) cover the DEF-113 family. Non-functional — determinism (byte-identical re-emission) follows the existing suite convention. UX & presentation — E10042 (AR-7) is the only new user-facing text; all other reports reuse existing codes. Stakeholder conflicts — none. Naming — AR-17.
