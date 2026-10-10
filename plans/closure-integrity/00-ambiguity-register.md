# Ambiguity Register: Closure Integrity

> **Status**: ✅ GATE PASSED — all 15 items resolved (5 user-decided; 3 conformance to the frozen
> spec; 7 auto-design-delegated)
> **Last Updated**: 2026-10-10 10:23

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| AR-1 | Scope | Which defects, and under which rule authority | Six defects as proposed / five (DEF-110 to batch B) / eight (merge DEF-95+96) | **User:** six defects as proposed — DEF-92, DEF-93, DEF-94, DEF-104, DEF-109, DEF-110; fixes conform to the frozen capture/ownership rules where those rules already decide the behavior (DEF-92, DEF-94, DEF-104); no spec change beyond the two new diagnostic rows (AR-2, AR-3). Excluded: batch B (DEF-95, DEF-96, DEF-99, DEF-107, DEF-111, DEF-117, DEF-118, DEF-119, DEF-120, DEF-121, DEF-122, DEF-124, and the remaining minors) and PLAN-05/13/14. | ✅ Resolved |
| AR-2 | Behavioral | DEF-93: a module-level name in a capture list (`[base] () => base`) is accepted and emits a broken environment-less call; the spec is silent | Reject with a new E10406 / support by threading the binding into the environment | **User:** reject with a new `E10406` row — draft: `| E10406 | Capture list names a module-level binding | module-level '{name}' needs no capture | use the name directly |` — plus its golden and driver registration. | ✅ Resolved |
| AR-3 | Behavioral | DEF-110: calling an unbounded type parameter (`func g<T>(x: T) { return x(5); }`) is accepted and emits invalid C; the grammar allows callable bounds on type parameters (`<T: func(...)>`) | Reject at the declaration with a new E10307 / defer to batch B | **User:** reject with a new `E10307` row — draft: `| E10307 | Call of a type-parameter value without a callable bound | type parameter '{param}' is not callable | declare a callable bound: `T: func(...)` |` — plus its golden and driver registration. | ✅ Resolved |
| AR-4 | UX & presentation | DEF-92: the relayed capture (`([n] () => ([m] () => m)())()`) needs a report; the frozen E10405 rule supports rejection | Reuse E10405 on the outer lambda / new E10407 | **User:** reuse `E10405` — report the existing code, message, and help on the outer lambda, whose body reads `m` through the nested capture. | ✅ Resolved |
| AR-5 | Behavioral | DEF-104: `func outer<T>(x: T): T { return ([x] () => x)(); }` at a non-duplicable `T` double-frees | Reject the return per E10401 / environment-owns transfer | **User:** reject per `E10401` ("cannot move captured '{name}' in a lambda"); duplicable instances keep copying. **Phase 2 note (2026-10-10):** the concrete `String` spelling already reported `E10401` at the baseline, so the per-instance judgment harmonizes the generic instance; the two pre-existing tests that pinned the owning-instance acceptance (`residual_defect_completion_{spec,impl}_test.rs`) were corrected to the ruled behavior as a necessary consequence. | ✅ Resolved |
| AR-6 | Behavioral | DEF-94: a field write through a listed copy capture reports E10024 instead of the L-4 rule | E10402 / keep E10024 | **Conformance:** the frozen L-4 rule ("writing a captured binding needs `[borrow x]`") fixes the report — `E10402` fires and `E10024` must not; the spec is the authority (covered by the AR-1 framing). | ✅ Resolved |
| AR-7 | Behavioral | DEF-109: a captured func-typed binding called inside the lambda leaks its owned result | release-once via the capture lookup / leave as is | **Conformance:** the ownership model requires release-once; the callable capture must resolve to the capture (not the same-named module lender) so the result is released exactly once (AR-1 framing). | ✅ Resolved |
| AR-8 | Edge cases | The E10406 rule's scope: which module-level spellings it covers | let/const/imports / const only | **AI — delegated by --auto-design (eligible: scope of a user-confirmed rule; consistent application).** The rule covers every module-level name a capture list can name — `let`, `const`, an imported name, a function, a type, or any other module declaration — because all are visible without capture. **Amended in Phase 1 (2026-10-10):** the review reproduced the same environment-less emission for function- and type-name entries (the user-approved trigger text already reads "a module-level binding"); the predicate now covers every module declaration. | ✅ Resolved (delegated) |
| AR-9 | Technical | DEF-92: where and how the relay is detected | extend the E10405 pass to names read by nested captures, reported on the outer lambda / a separate nested-capture pass | **AI — delegated by --auto-design (eligible: compiler mechanism within AR-4).** The existing E10405 checking pass gains the relay case: a name read anywhere inside the outer lambda's body — including through a nested lambda's capture list — is an enclosing read the outer must list. | ✅ Resolved (delegated) |
| AR-10 | Technical | DEF-109: how the capture kind survives to the call | mask the capture kind and resolve through the environment lookup / rename the capture | **AI — delegated by --auto-design (eligible: compiler mechanism within AR-7).** The call resolves through the capture, never a same-named module lender. **Corrected in Phase 2 (2026-10-10):** the review-grade probe showed the leak comes from the may-lend gate (`lowered_lend_source`) resolving the callee name to the same-named module lender whose return is an elided view, which suppressed the owned result's binding; the fix returns `None` for a callee that names a local binding, so the value call binds and releases its result. Masking the IR spelling was tried and rejected — it leaked into the C spelling because codegen resolves captures by source name. | ✅ Resolved (delegated) |
| AR-11 | Technical | DEF-104: where the rejection fires | extend the existing move-of-capture rule to the return/consume position / a new return-specific check | **AI — delegated by --auto-design (eligible: compiler mechanism within AR-5).** The existing L-3 (E10401) rule judges a lambda body that returns or consumes a by-value non-duplicable capture; duplicable captures keep copying. | ✅ Resolved (delegated) |
| AR-12 | Process | Each new diagnostic's landing set | registry row + compile-fail golden + driver `ASSERTED_CODES` entry + test coverage | **AI — delegated by --auto-design (eligible: established process; the E10042/AR-19 precedent).** Both new codes land with their registry row, a compile-fail golden, and the compile-fail driver's asserted-code registration in the same phase; `./spec/check.sh` stays green. | ✅ Resolved (delegated) |
| AR-13 | Technical | Verify commands and the memory oracle | detected project commands | **AI — delegated by --auto-design (eligible: testing strategy; commands detected from `AGENTS.md`/the `Makefile`, never invented).** Per phase: `make check` and the plan's suites in the pinned container; host ASan probes (`clc` emit, `cc -fsanitize=address -no-pie`, leak detection) are the memory oracle for the DEF-104/DEF-109 family; `make verify` at close-out. | ✅ Resolved (delegated) |
| AR-14 | Naming | Plan folder, suites, and module names | follow the batch convention | **AI — delegated by --auto-design (eligible: internal naming, zero external surface).** Folder `plans/closure-integrity/`; suites `compiler/tests/closure_integrity_spec_test.rs` and `_impl_test.rs`; test sources use `spec.ci.*` modules; no new language syntax names. | ✅ Resolved (delegated) |
| AR-15 | Sequencing | Phase structure and review dispatch | one phase per defect (6) / one phase per component layer (3) | **AI — delegated by --auto-design (eligible: implementation sequencing).** Three phases by layer: P1 capture listing and validity (DEF-92, 93, 94), P2 captured-owner integrity (DEF-104, 109), P3 callable bounds (DEF-110); each phase runs the three-step specification-first cycle and dispatches its reviews per the change surface (P1: correctness + semantics; P2: correctness + memory-safety; P3: correctness + semantics). | ✅ Resolved (delegated) |

## Resolution Notes

**AR-1 (user):** scope confirmed against the roadmap rows DEF-92/93/94/104/109/110 and the batch
framing (closure/callable integrity). Exclusions are explicit: every other open defect stays in the
backlog for the agreed later batches (batch B: evaluation/emission majors and the ruled DEF-119,
DEF-124 items; batch D: the DX slice; batch C: the fuzzing harness).

**AR-2 / AR-3 (user):** the two new registry rows are drafted in the rows above; their exact
placements follow the existing sections (`spec/14-diagnostics.md` §4 for E104xx-family additions
after E10405; the template section for E103xx after E10306). Both are spec edits the user has
explicitly authorized, equivalent in scope to the E10042 ruling.

**AR-4 (user):** the report span is a mechanism detail (delegated under AR-9); the code, message,
and help are the existing E10405 ones.

**AR-8 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: the scope of the user-confirmed E10406 rule; every module-level binding is visible without capture, so the same reason applies to let, const, and import bindings; no new surface
Objective: the reject rule catches every capture-list entry that names a module-level binding
Decision: E10406 fires for module-level let, const, and import bindings alike
Evidence: the frozen visibility rules expose module-level bindings inside function bodies without capture (spec/06-functions.md); the current emission drops the environment when such a capture is listed (DEF-93 row); the same broken emission serves all three spellings
Rejected alternatives: const-only coverage — leaves let and import spellings emitting the same broken call
Strongest counterargument: an import binding might have subtleties (aliasing) worth its own message — none found; the rule's help text ("use the name directly") fits all three
Confidence: High
Hardening: not required (rule-scope application)
Policy version: 1
Root invocation ID: plans-closure-integrity-20261010
Reopen triggers: a module-level spelling whose removal from a capture list does not fix the program
```

**AR-9 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler mechanism inside the user-confirmed E10405 reuse (AR-4); no new surface
Objective: a relayed capture is reported once, on the outer lambda
Decision: the existing capture-listing check also treats a name read by a nested lambda's capture list as a read by the enclosing lambda's body; the report reuses E10405 and points at the entry inside the enclosing lambda (the text whose fix is to add the name to the enclosing list)
Evidence: the current check walks direct body reads only (DEF-92 row: the relay is accepted and the call drops the environment); the frozen E10405 rule requires listing enclosing reads
Rejected alternatives: a separate nested-capture pass — duplicates the listing walk for the same rule
Strongest counterargument: reporting on the outer lambda may surprise when the inner list is the visible text — but the outer is the lambda that must add the name, and its help text says so
Confidence: High
Hardening: in-context layers only (conformance check extension)
Policy version: 1
Root invocation ID: plans-closure-integrity-20261010
Reopen triggers: a relay shape the outer-lambda report cannot name (for example a deeper nesting whose outer list is already correct)
```

**AR-10 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler mechanism inside the user-confirmed release-once behavior (AR-7); no new surface
Objective: a called func-typed capture resolves to the capture, and its owned result releases once
Decision: the capture records a masked kind and resolution carries the capture through the environment lookup, replacing the direct IR-spelling assignment the current code performs
Evidence: DEF-109 row — `declare_captures` sets the IR spelling directly and the may-lend gate resolves the source name against module lenders, leaking the owned result (ASan, 10 bytes); parameter, field, and lambda-parameter cases were already fixed by the same masking pattern
Rejected alternatives: renaming the capture — hides the resolution bug and changes user-visible spellings in diagnostics
Strongest counterargument: the masking pattern now covers four capture positions; a fifth position could repeat the bug — the plan pins the callable-capture case explicitly
Confidence: High
Hardening: in-context layers only (established masking pattern)
Policy version: 1
Root invocation ID: plans-closure-integrity-20261010
Reopen triggers: a capture position where the masked kind reaches an owner decision it should not
```

**AR-11 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler mechanism inside the user-confirmed E10401 rejection (AR-5); no new surface
Objective: returning a by-value non-duplicable capture is rejected once with E10401
Decision: the existing move-of-capture rule (L-3) judges the return position of a lambda body; the report is E10401 and duplicable captures stay copies
Evidence: DEF-104 row — the current emission returns `__env->x` by value while the environment aliases the caller's owning value (double free, base and head identical); L-2 makes a non-duplicable capture a read-only borrow and L-3 rejects moving it
Rejected alternatives: environment-owns transfer — a language-surface change the user declined (AR-5)
Strongest counterargument: the author's intent (return the captured value) has a simple spelling (`return x;`) — the rejection is one edit away, and the alternative semantics would keep two owners of one allocation on other paths
Confidence: High
Hardening: in-context layers only (frozen rule application)
Policy version: 1
Root invocation ID: plans-closure-integrity-20261010
Reopen triggers: a return-through-capture shape the move rule cannot see (for example through a projection) that still double-frees
```

**AR-12 / AR-13 / AR-14 / AR-15 (delegated):** recorded in the table with their eligibility and
evidence; each follows an established repository mechanism (the E10042 golden precedent, the
detected verify commands used by every prior plan, the `*_integrity` batch naming, and the
per-layer phase structure), so no further note is needed.

**Scan completeness (12 categories):** Feature gaps — covered by the per-defect requirements (01)
and components (03-01 – 03-03). Behavioral — AR-2 – AR-7. Scope — AR-1, AR-8. Technical unknowns —
AR-9 – AR-11, AR-13. Edge cases — AR-8 and the per-defect edge lists in 07 (duplicable vs
non-duplicable captures, relay depth, module `let`/`const`/imports, return vs consume through a
capture). Integration points — the capture checker (`sema/expr.rs`, `sema/mod.rs`), the ownership
lowering and capture recording (`compiler/src/own/`), the emitter's closure paths, the diagnostics
registry (`spec/14-diagnostics.md`) and the compile-fail driver; owned by 03-01 – 03-03. Data &
state — no new data model; the existing capture/environment machinery only. Security & compliance —
memory safety is the security surface; ASan probes (AR-13) cover the DEF-104 double-free and the
DEF-109 leak. Non-functional — determinism (byte-identical re-emission) follows the suite
convention. UX & presentation — E10405 reuse (AR-4) and the two new rows (AR-2, AR-3) are the only
user-facing text. Stakeholder conflicts — none. Naming — AR-14.
