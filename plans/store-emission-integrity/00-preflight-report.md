# Preflight Report: store-emission-integrity

> **Status**: ✅ PREFLIGHT PASSED WITH NOTES — 1 major + 2 minor resolved; 1 observation recorded
> **Iteration**: 1 (first scan)
> **Artifact**: Implementation plan at `plans/store-emission-integrity/` (10 documents)
> **Artifact revision reviewed**: content hash of the 10 documents at scan start (SHA-256): `73ac850d1ae58152dc5ad82c2030b93835d75bc55a858e337bb6726c0af010c5` — the on-disk revision now additionally carries the applied resolved-finding edits below
> **Git baseline**: `b47de60` (working tree clean at scan start; all 10 documents committed)
> **Codebase Grounded**: 30+ source files examined; every design-line reference verified; 4 live probes run with the prebuilt `target/debug/clc` (functionally identical to `b47de60`: the only source change between the `clc` build commit `39f0e5a` and HEAD is documentation)
> **Auto-design**: active — eligible technical decisions delegated and recorded; one user-reserved scope decision surfaced (PF-001 ruling)
> **Scope mode**: strict (no `--explore-scope`)
> **Last Updated**: 2026-10-10 08:26
> **Same-session review**: not the case — the plan was authored in a previous session (commits `e58c2ac`, `b47de60`)

## Codebase Context Summary

**Tech Stack:** Rust workspace (std-only, no crates) building two binaries: `clc` (the
compiler, `compiler/src/`) and `clorraine` (the driver). Emits one C99 translation unit per
program for the `host`, `hosted-amiga`, and `freestanding` profiles; C shims in
`compiler/runtime/`. Frozen specification in `spec/` (grammar, chapters, diagnostics registry,
golden suites); embedded stdlib in `stdlib/`. Tests: `compiler/tests/*_spec_test.rs` (immutable
oracles) and `*_impl_test.rs`, run via `make check` inside the pinned SDK container.

**Architecture (relevant slice):** three subsystems carry the defects. (1) The ownership layer
(`compiler/src/own/`): `own/expr.rs` holds the lend-store judgment (`check_lend_target_store`
:698, `lend_moves_judged` :785, the conditional-walk source helpers :74/:109), and `own/place.rs`
holds the write-mutability checks (`write_place` :99, `is_readonly_param` :193). (2) The duty
audit (`own/instance_check.rs::check_storable_slot_requirements` :183) resolves per-instance
judgments for still-parameter-typed slots. (3) The emitter (`compiler/src/codegen.rs::run`
:348-361, `compiler/src/codegen/program.rs`): typedef and aggregate phases,
`emit_global_init` :620, `lower_global` :664, the init functions :604; the intrinsic renderer
(`codegen/calls.rs::intrinsic`) and slice bounds guard (`codegen/guards.rs` :98-104) already
exist. `spec/14-diagnostics.md` registers E10010–E10041 live; E10042 is unused and free.

**Key Observations:**
- All design-document line references verified accurate, including the subtle ones
  (`own/lower.rs:698` `materialize_guard_operand`, `:1764` `is_pure_value`;
  `own/call.rs:263` `lower_intrinsic_call` with the user/ffi shadow guard; `stdlib.rs:79`
  the intrinsic set includes `slice`).
- `slice`/`rawSlice`/`rawStr` are `intrinsic func` declarations in `stdlib/std.mem.clo:38-41`,
  so calls reach `lower_global` as ordinary `ExprKind::Call` (not `IntrinsicCall`) — the
  AR-10 design ("mirror `lower_intrinsic_call`'s detection in `lower_global`") is the correct
  and only viable mechanism for the reported defect.
- The DEF-115 reorder is acyclic and safe: slice typedefs reference element types through
  pointers (`pointer_to_array_declarator`, `codegen/types.rs:241`), which only need a prior
  declaration; view-typed struct fields are banned (E10205); closure environments (the only
  slice-typedef consumers after struct definitions) are emitted after the slice phase in both
  the current and proposed order.
- The DEF-113 target-aware suppression exclusion is exactly consistent with the marking path's
  existing self-arm exemption (`own/expr.rs:554-576`), and the emitted `ClStr.ptr` is
  `const u8 *` (`codegen/program.rs:103`), confirming the DEF-114 class.
- One verified gap: fixed-array-returning calls as module initializer values emit invalid C
  (hidden-destination callee called without its destination). This form is not covered by any
  plan test case; the design text nevertheless names "a call result" among handled values.

**Reference Verification:** 41 design-line references mapped — 41 verified, 0 unverifiable.
4 live probes run: two call-result-array repros (new defect class, PF-001 evidence — the
challenger extended this to six positions), `len` global const-folding (no defect), and the
typedef-order/emission-order reading against the emitter source.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 0 | — |
| 2 | Implicit Assumptions | 1 (PF-001) | 🟠 MAJOR |
| 3 | Logical Contradictions | 0 (subsumed in PF-001) | — |
| 4 | Completeness Gaps | 1 (PF-003) | 🟡 MINOR |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 0 | — |
| 7 | Testability | 0 (subsumed in PF-003) | — |
| 8 | Security Blind Spots | 0 (the batch is itself the memory-safety fix) | — |
| 9 | Edge Cases | 1 (PF-001) | 🟠 MAJOR |
| 10 | Scope Creep Indicators | 0 (the plan is a bounded defect fix; no unapproved machinery) | — |
| 11 | Ordering & Sequencing | 0 (specification-first ordering verified in all four phases) | — |
| 12 | Consistency | 1 (PF-002) | 🟡 MINOR |
| 13 | Codebase Alignment | 1 (PF-001) | 🟠 MAJOR |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| 🔴 CRITICAL | 0 | — |
| 🟠 MAJOR | 1 | ✅ resolved (PF-001, user ruling C) |
| 🟡 MINOR | 2 | ✅ resolved (delegated: PF-002, PF-003) |
| 🔵 OBSERVATION | 1 | recorded, no action required |

### Domain lenses applied

`references/domains/selection.md`: **compiler and language** (grammar, checker, IR, emitter)
— applied throughout via the semantics/emission conformance checks. No financial, web,
distributed, or data-migration lens applies (no persisted formats; the emitted C is a build
artifact, not a durable serialized format). The batch's security surface is memory safety,
covered by the planned host ASan oracle (AR-13).

---

## Findings

### PF-001: Fixed-array call-result initializers are claimed by the design but not covered — accepted source emits invalid C [🟠 MAJOR]

> **Status**: ✅ Resolved — user ruling: Option C.

**Dimension:** Implicit Assumptions / Edge Cases / Codebase Alignment
**Location:** `plans/store-emission-integrity/03-04-module-initializers.md` §Proposed Changes
item 1 ("A non-literal array value (a place, **a call result**) is copied element-wise from its
rendered base"); `00-ambiguity-register.md` AR-9 decision ("a non-literal array value copies
element-wise from its rendered base") and its Evidence claim "every affected shape is pinned by
a probe".

**Codebase Evidence:**
- `compiler/src/codegen/program.rs:772-784` — `lower_global`'s call branch builds a
  destination-less `ValueKind::Call`; `emit_global_init` :653 pushes one assignment
  (`{target} = {rendered};`).
- The function-body path handles the same shape through a different layer
  (`own/lower.rs:1009-1015` `value_needs_place`; `codegen/expr.rs:503-517`
  `call_with_destination`); the module-init path deliberately runs no checker
  (`03-04` §Integration Points).
- **Reproduced live** with `target/debug/clc` (host profile):
  - `func mk(): i32[2] { return [7, 8]; }` + `let a: i32[2] = mk();` (accepted) emits
    `cl_…_a = cl_…_mk();` — the callee's prototype is
    `static void cl_…_mk(i32 __cl_ret[2]);`, so this is a too-few-arguments void assignment to
    an array lvalue; `cc -std=c99 -Wall -Wextra -Werror` rejects it.
  - `let g: i32 = mk()[1];` (accepted) emits `cl_…_g = cl_…_mk()[1];` — same class.
- Independent challenger extended the class to six positions (whole value, element projection,
  member projection, user-fn argument, intrinsic argument, conditional arm), all accepted and
  all strict-build-failing; all function-body controls emit the hidden-destination shape
  correctly.
- No plan test case (ST-23 – ST-29) exercises any call-result form; AR-18 explicitly authorized
  only the conditional-value form for addition; R4 and the key-decisions table name
  literal/fill/place/conditional only.

**The Problem:** The design's array-value sentence instructs an implementation that cannot
work — indexing the lowered call (`mk()[i]`) is invalid C — and names a form no probe verified
and no ST case pins. Because it concerns the emitter function this plan rewrites, an
implementer could follow the text literally or leave the form half-handled. The plan's own
"every affected shape is pinned by a probe" claim is false for call results. The repo
convention for this exact family is filing separate defects (DEF-21, DEF-24, DEF-32 fixed
separately through the ownership materialization layer).

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Exclude call-result forms; correct the wording; file the defect separately | Smallest, honest; keeps the four-defect batch boundary | Probe evidence would be written but not kept durably in the plan |
| B | Expand Phase 4 to materialize fixed-array call results (hidden destination, six positions) | Closes the class in this batch | Effectively a new feature phase: needs a materialization layer, an ownership rule for non-duplicable elements, six new ST positions, semantics review; contradicts the one-phase-per-defect structure (AR-14) and the user's ratified batch scope |
| C | Hybrid: correct the two overreaching sentences to the ratified form list, record the call-result probes as durable boundary evidence, and file the class as a new backlog defect | Repairs the contradiction now; preserves evidence; matches repo precedent; pre-execution timing (0/26 tasks) makes it free | The filed class waits for a future batch |

**Recommendation:** Option C — it is the only option that keeps the plan internally consistent
and truthful without either silently expanding user-owned scope or discarding the evidence.
Option B's machinery is genuinely useful but must not enter this batch without a user ruling
(it would be a fifth phase, not a Phase 4 edit). The challenger independently recommended C.

**Independent challenger:** challenged pre-presentation with the finding statement + surviving
options + the recon summary, blind to the parent's lean. Verdict: C — with the stronger
grounding that `03-04:47-49` and AR-9 are the *only* places claiming call-result support (R4,
the index's key decisions, the execution plan 4.2.1, the roadmap row, and the ST cases all
exclude it) — so this is a consistency repair of an overreaching sentence, not a scope
reduction. `Challenger: converged` (it rejected B on evidence; the parent's initial lean A was
superseded by C on the evidence-preservation argument).

**User Decision:** ✅ **Resolved — User chose Option C** (2026-10-10): (a) `03-04` item 1 and
the AR-9 decision + evidence are corrected to the ratified form list (flat, nested, fill,
place-copy, conditional, intrinsic view) with the call-result gap explicitly out of scope;
(b) the six-position probe evidence is recorded in `02-current-state.md`; (c) the class is filed
as the new backlog defect **DEF-117** on `plans/00-roadmap.md`.

**Filing (done):** DEF-117 on `plans/00-roadmap.md` under Backlog / Known defects — "A
fixed-array-returning call in a module-level initializer (or initializer projection/argument)
is accepted and emits a destination-less call, so the strict build fails", 🟠 major,
pre-existing; evidence: six positions reproduced at `b47de60`; pointers `own/lower.rs:1009`,
`codegen/expr.rs:508`; suggested fix: a module-init materialization/hidden-destination pass.

---

### PF-002: `global_intrinsic_call` carries an unused `module` parameter; design comment contradicts its signature [🟡 MINOR] — ✅ Resolved (delegated)

**Dimension:** Consistency
**Location:** `plans/store-emission-integrity/03-04-module-initializers.md` §New
Functions/Methods: `fn global_intrinsic_call(&self, module: &Module, callee: &Expr/*name*/) -> Option<String>`

**Codebase Evidence:** The mirror target `own/call.rs:263-297` uses only the callee name
(`callee_name`, :269) and the program's callee map (:274); a global-context lookup needs
`self.program.callee(name)` (the same simple-name map `lower_intrinsic_call` uses) and a name
check against `stdlib::is_compiler_intrinsic` (`stdlib.rs:82`). Nothing in the helper's stated
work consumes the module, and `lower_global` already holds `module` (:664) for its own
resolution needs.

**The Problem:** An unused parameter is dead surface (and a Rust `unused_variables` warning
unless underscored); the inline `/*name*/` comment suggests the author was uncertain of the
callee parameter's type, which risks two implementers choosing different signatures for the
same helper.

**Options:** (A) Drop `module` from the helper signature; detection uses the callee name +
`self.program.callee` + `is_compiler_intrinsic`. (B) Keep `module` and consume it (qualified
name resolution). B models complexity the global path does not need — the existing rule is
simple-name based.

**Recommendation:** Option A — smallest correct signature; matches the mirrored rule exactly.

**Delegated resolution (auto-design, eligible: internal helper signature, no language or
product surface):**

```text
Authority: AI — delegated by --auto-design
Eligibility: internal emission-helper signature; contradicts no user decision; no reserved consequence
Objective: the design's helper signature matches the work it performs and cannot mislead the implementer
Decision: drop the `module` parameter; `global_intrinsic_call(&self, callee: &Expr) -> Option<String>`
  detects the intrinsic by callee name, `self.program.callee(name)` (no user/ffi shadow), and
  `is_compiler_intrinsic`
Evidence: own/call.rs:263-297 (mirror), stdlib.rs:79-84 (intrinsic set), codegen/program.rs:1042
  (callee_name), ar-10's stdlib.rs claim satisfied by the name check
Rejected alternatives: keeping and consuming `module` (adds resolution surface the rule does not use)
Strongest counterargument: a future intrinsic might need qualified-name resolution in the global path;
  none exists today, and adding it later is a local change
Confidence: High
Hardening: in-context layers only (mechanical signature hygiene)
Policy version: 1
Root invocation ID: pf-store-emission-integrity-2026-10-10
Reopen triggers: a global intrinsic whose detection needs module-qualified resolution
```

**User Decision:** ✅ Resolved — AI delegated (recorded above; applied to
`03-04-module-initializers.md` and the AR-10 note).

---

### PF-003: Two R2/R4-required forms lack specification cases [🟡 MINOR] — ✅ Resolved (delegated)

**Dimension:** Completeness Gaps
**Location:** `plans/store-emission-integrity/07-testing-strategy.md`, Phase 2 table (ST-10 –
ST-15 forms) and the Phase 4 coverage of intrinsic forms

**Codebase Evidence:** R2 (`01-requirements.md:34-37`) promises coverage "for plain, compound,
and update forms over parameter, local, field, and module-level bases"; the ST table samples
parameter (ST-10, ST-13, ST-14), field (ST-12), and module-level (ST-15) but no local
(`let t: str = "abc"; t[0] = …;`) case, although the design's predicate covers it by
construction (`own/place.rs:99-121` routes every local element write through `write_place`, and
the design's helper checks the type at the element prefix — the binding type for a
prefix-less local write). R4 covers module-level intrinsic calls; const-folding already
resolves `len`/`sizeof` at module scope (verified live: `let n: usize = len(a);` emits
`cl_…_n = 3;`), so the untested remainder is the dot-call spelling routed through the Call
path — a one-case addition keeps the suite honest about the shared rule.

**The Problem:** Two requirement claims are not directly pinned by a specification case. Both
are cheap to close at authoring time and prevent a later "which forms were actually verified"
question.

**Options:** (A) Add one local-write case to the Phase 2 table (append; renumber only the
trailing rows) and one dot-len-at-module-level case to Phase 4, updating the two document
references that cite the ST ranges. (B) Leave the tables as-is and rely on the implementation
tests. B leaves requirement-level coverage implicit.

**Recommendation:** Option A — the two cases trace directly to R2 and R4 and cost two table
rows.

**Delegated resolution (auto-design, eligible: testing strategy within confirmed requirements;
no new behavior; no reserved consequence):**

```text
Authority: AI — delegated by --auto-design
Eligibility: testing strategy for already-confirmed requirements R2/R4; no language surface; no reserved consequence
Objective: every claimed write form has one specification case; the module-scope intrinsic coverage is explicit
Decision: append to the Phase 2 table a local `str` element-write rejection case (one E10042 naming the
  local; no translation unit) and to the Phase 4 table a module-level dot-call `len` view case (`let n: usize =
  backing.len();` accepted, strict build, runs), renumbering only the trailing ST rows and updating the two
  range references (03-02 §Testing Requirements)
Evidence: 01-requirements R2/R4; own/place.rs:99-121 (local write path); live probe: `len(a)` at module scope
  const-folds to `3` (no intrinsic call reaches lower_global), so the dot-call case is the one that exercises
  the Call-path rule
Rejected alternatives: relying on implementation tests only (leaves the requirement claim unpinned)
Strongest counterargument: test-table growth without new behavior — but both rows pin requirements that are
  otherwise asserted only in prose
Confidence: High
Hardening: in-context layers only (test-plan addition)
Policy version: 1
Root invocation ID: pf-store-emission-integrity-2026-10-10
Reopen triggers: a form the added cases mis-model against the frozen rule
```

**User Decision:** ✅ Resolved — AI delegated (recorded above; applied to
`07-testing-strategy.md` and the `03-02` reference).

---

### PF-004: Observations (no action required) [🔵 OBSERVATION]

1. `spec/` immutability: the E10042 registry row + golden are the user's standing ruling
   (AR-2/AR-12); the plan conforms. No additional spec edits were found necessary — BR-5
   (Ch 16 §191), the Ch 02 §6 mutability table, and the Ch 17 §1 forbidden-construct list
   (compound literals, designated initializers) all already say what the fixes enforce; the
   plan's rejected alternatives are correctly grounded in them (`spec/17-codegen.md:19-25`).
2. The `--auto-effort` flag was not passed; per the preflight recommendation the appropriate
   level is `high` (or `max` for `--thorough`). Advisory only — nothing gates on it.
3. Version compatibility: the emitted C stays in the C99 common subset for all three profiles;
   the typedef reorder is pure statement ordering, so the m68k/vbcc/FS-UAE legs are behavior
   neutral (emission-integrity audit planned, AR-14).
4. `compiler/src/codegen/program.rs` (1049 lines today) will grow past its current size; the
   repo's existing oversized-module pattern (`own/expr.rs` 1123, `own/lower.rs` 1792) accepts
   this, and the plan adds no new module, so no finding.

## Adversarial question checklist (same-agent bias safeguard)

- *What assumption might the author have unconsciously confirmed?* — That "non-literal array
  value" covers call results; the code shows the function-body path handles them in a different
  layer the module-init path does not run. Confirmed as PF-001 and reproduced live.
- *What external standard might this violate?* — C99: verified directly against
  `cc -std=c99 -Wall -Wextra -Werror` and the frozen Ch 17 contract; no other external standard
  is binding. BR-5 and the mutability table were cited from the spec text.
- *What would a dissenting domain expert flag?* — (a) The call-result class (PF-001);
  (b) test coverage of "obvious" variants (PF-003); (c) whether reordering typedefs could
  change any pinned emission — checked: no test pins cross-category typedef order, and every
  strict-build site exercises the new order through compilation.

## Verification of the plan's own gate state

- Ambiguity Register: GATE PASSED, 18/18 resolved (user rulings AR-1–3, 12, 16, 18;
  delegated AR-4–11, 13–15, 17). No open rows; no silent deferrals found.
- Specification-first ordering: verified in all four phases (spec tests before implementation;
  red-phase records specified; immutability-oracle rule stated in 07).
- Acceptance criteria: all seven trace to verifiable commands (`make check`, `make verify`,
  host ASan probes, `./spec/check.sh`, strict-build scans); no untestable criterion found.
- Complexity Escalation Gate: no new support surface proposed; the plan's "Excluded machinery:
  None considered" statement is accurate (all rejected alternatives were variants of existing
  paths).

## Post-fix consistency check (all resolved findings applied)

- `03-04` array-value sentence, AR-9 decision + evidence, and the `02-current-state.md` boundary
  note now agree on the covered form list and the excluded call-result boundary (PF-001 ruling C).
- `03-04` helper signature, AR-10 note, and the PF-002 delegated record agree (no `module`
  parameter).
- ST table additions (ST-16b, ST-30) and the `03-02`/execution-plan test-reference ranges agree
  (PF-003).
- DEF-117 filed on the roadmap; no acceptance criterion changed; no ST case removed; the
  four-defect batch scope is unchanged.

## Verdict

**✅ PREFLIGHT PASSED WITH NOTES — 1 major resolved (PF-001, user ruling C), 2 minor resolved
(PF-002/PF-003, delegated), 1 observation recorded.** Zero unresolved critical/major findings.
The four design documents were verified against the codebase reference-by-reference; the only
substantive gap was the call-result overclaim, now corrected, evidenced, and filed. Iteration 2
is not required: the applied edits are wording, one signature, two test rows, and one backlog
row — none changes behavior, ownership, dependency ordering, security, or a compatibility
contract.
