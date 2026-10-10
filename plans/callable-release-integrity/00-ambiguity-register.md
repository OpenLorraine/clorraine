# Ambiguity Register: Callable Release Integrity

> **Status**: ✅ GATE PASSED — all 15 items resolved (user decisions + auto-design provenance)
> **Last Updated**: 2026-10-10 19:53
> **Batch**: E of the 37-defect mandate (DEF-130 – DEF-166); user-confirmed split E → F → G
> **Auto-design**: active — root invocation ID `callable-release-integrity-2026-10-10`,
> policy version 1; reserved decisions (spec text, new user-visible behavior, acceptance
> criteria) remain user-owned and none is open.

| # | Category | Ambiguity / Gap | Options Presented | Resolution | Status |
|---|----------|-----------------|-------------------|------------|--------|
| 1 | Scope | Which defects batch E closes | The 13-row callable/release cluster / a different partition of the 37 defects | **User:** the E → F → G split; E = DEF-138 – DEF-141, DEF-155 – DEF-159, DEF-163 – DEF-166; D stays parked until no defects are left | ✅ Resolved |
| 2 | Behavioral | DEF-138 (critical): a source local named `__cl_owned0` collides with the ownership layer's generated temporary and double-frees | Identity-keyed generated bindings / collision-avoiding name generation | **User:** identity-keyed generated bindings — the ownership layer resolves its generated temporaries by binding identity, never through the source name table | ✅ Resolved |
| 3 | Technical | DEF-138 mechanism: how identity-keying is realized | Generated bindings never enter the source name map; generated references stay id-based; a colliding C-level spelling renames via the existing `__cl_tmp*` reservation mechanism generalized to `__cl_owned*` / reserve the prefix / reject user spellings | **AI — delegated by --auto-design (PD-1)** | ✅ Resolved |
| 4 | Technical | DEF-140: the `func` type key ignores the signature, so `Holder<func(i32): i32>` and `Holder<func(u32): u32>` share `ClHolder__fn` and the unit redefines the struct | Include the signature in the key / separate per-instance mangling pass | **AI — delegated by --auto-design (PD-2)** | ✅ Resolved |
| 5 | Technical | DEF-165: an inline capturing lambda emits duplicate `struct ClEnv_0` definitions | One env definition per unique (site, instance) / rename duplicates / per-use structs | **AI — delegated by --auto-design (PD-2)** | ✅ Resolved |
| 6 | Behavioral | DEF-155/156/157/166: callable effect modes are missing for func-typed fields, module func-typed globals, `Param`-typed callees, and lambda parameters (double-frees, leaks, a false `E10020` against a shadowing parameter) | Thread each callee's own signature into the call's modes for every callee kind, judging generic callees per instance; identity-based callee resolution / reject the shapes / leave per-kind gaps | **AI — delegated by --auto-design (PD-3)** | ✅ Resolved |
| 7 | Behavioral | DEF-139/158/159/163/164: owned releases are missing at five holes (func-field receiver roots, defer bodies, conditional view arms, fixed arrays of view returns, discarded call statements) | Extend the existing owner/release machinery to each hole / reject the accepted-and-leaking shapes | **AI — delegated by --auto-design (PD-4)** | ✅ Resolved |
| 8 | Behavioral | DEF-163 diagnostic choice: what a fixed array of view elements returned from a temporary reports | The same diagnostics as the equivalent single-view spelling (E10036 family; escape checks) / a new code | **AI — delegated by --auto-design (PD-4)**; reopen trigger: a new code proves necessary → escalate | ✅ Resolved |
| 9 | Behavioral | DEF-141: `len` of a null `String` segfaults through the implicit view widening | Reject with `E10010` before the widening / define a null handle as an empty view | **User:** reject with `E10010` — the possibly-null use follows the frozen narrowing rule; no silent zero | ✅ Resolved |
| 10 | Non-functional | Verify commands for the plan | `make check` + `make verify` + scoped suites | **User (standing):** confirmed in the prior batch's register; unchanged | ✅ Resolved |
| 11 | Scope | Modification set: does any fix need spec text or a new diagnostic code? | None planned / a bounded registry or chapter edit | **User (standing mandate):** fixes are conformance to the frozen rules; any needed spec edit or new code pauses to the user (reserved) | ✅ Resolved |
| 12 | Process | Phase review dispatch | AGENTS.md matrix by change surface | **Convention (project policy):** ownership/drop/emission → correctness + memory-safety; IR/codegen/symbol names → + emission-integrity; language rules (lends, captures) → + semantics; recorded per phase in the execution plan | ✅ Resolved |
| 13 | Process | Execution commit mode | auto-commit + auto-design / ask-commit / no-commit | **User:** `--auto-commit --auto-design`; repeat batches until no defects remain | ✅ Resolved |
| 14 | Behavioral | Reproductions: do the filed shapes still fail at HEAD? | Re-derive each reproduction in the red phase / trust the filed evidence | **Convention:** each row's reproduction is re-derived at HEAD in the red phase; the findings ledger carries the base evidence | ✅ Resolved |
| 15 | Technical | Complexity: does any fix add a layer, dependency, harness, or refactor? | None — fixes reuse the existing owner, mode, key, and cleanup machinery / a larger support surface | **AI — delegated by --auto-design:** None; no complexity escalation, no challenger packet required | ✅ Resolved |
| 16 | Technical (runtime) | The drafted func-key rule excluded parameter modes, but the type model defines mode-differing signatures as different types (a `borrow` parameter reaches the callee as a pointer), so two mode-different instances would share one C struct and collide | Include each parameter's mode in its key / exclude modes as drafted | **AI — delegated by --auto-design (runtime; PD-2 amended)** — modes included | ✅ Resolved |
| 17 | Technical (runtime) | DEF-166's drafted expectation (the filed double-free shape is accepted and releases once) contradicted the frozen rules: unmarked parameters are read-only lends (`spec/03-ownership.md` §2), the module twin of the same body is rejected `E10034`, and a block-bodied lambda reports it too. The double-free exists only because an expression-bodied lambda skips the return-move judgement, and a closure call never materializes its lend arguments | Accept-and-release (as drafted) / the frozen `E10034` parity for the body plus caller-side release of owned temporary arguments on the call path | **AI — delegated by --auto-design (runtime; AR-6 amended)** — the spec is the oracle: the filed shape reports exactly one `E10034` (its own instance), and the call path releases owned temporary arguments | ✅ Resolved |

### Resolution Notes

**AR-1 (user):** the triage was presented and confirmed with the row lists and severities
(1 critical, 11 major, 1 minor in E). The mandate: close all 37 defects (E → F → G), then
resume the queue; the loop repeats until the roadmap shows no open defect row.

**AR-2 (user):** the direction was chosen from the two grounded options; the mechanism is
delegated (PD-1).

**AR-9 (user):** the rejection direction was chosen over the define-empty-view option; the
mechanism (which path lets the null through the widening use) is delegated.

### Delegated Decision Provenance

#### PD-1 — DEF-138 identity-keyed generated bindings

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanism (binding identity and generated-name handling); stays
  inside the user-selected direction; changes no accepted program's semantics outside the
  collision shape
Objective: eliminate the generated-name collision class (critical double-free) structurally
Decision: generated owners are declared so they never enter the source name map
  (`declare_local` for generated bindings keeps them out of `scopes[].names`; source `lookup`
  can never return one); every ownership-layer reference stays binding-id/place-based; a
  generated C spelling that collides with a source local renames through the existing
  generated-name reservation mechanism (`__cl_tmp0` → `__cl_tmp0_1` precedent, generalized to
  `__cl_owned*`)
Evidence: generation at own/lower.rs:1839, :1881; source name map at own.rs:799 (declare_local),
  :920 (lookup); scope transfer at own/lower.rs:1908; the Phase-2 RV-003 rename precedent
  (plans/eval-emission-integrity/05-findings.md:99); ASan evidence at .../05-findings.md:112
Rejected alternatives: (a) reserve `__cl_owned*` spellings before naming — smaller but keeps
  name-keyed resolution and the class recurs; (b) reject user locals carrying the prefix —
  user-visible restriction without need
Strongest counterargument: identity-keying touches the binding-map contract and could mask
  other name-lookup bugs instead of fixing them
Confidence: High — the failure is exactly a name-vs-identity confusion; id-based resolution is
  structural; the red pin and the memory auditor re-derive the double-free
Hardening: the phase's memory-safety auditor reproduces the double-free on the base and probes
  collision variants on the head; the C-level rename half reuses a precedent already hardened
  in Phase 2
Policy version: 1
Root invocation ID: callable-release-integrity-2026-10-10
Reopen triggers: any accepted program still double-frees under ASan at an `__cl_owned*`-shaped
  collision; any generated-name lookup remains reachable from source resolution
```

#### PD-2 — DEF-140 and DEF-165 instance/env identity

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanism (symbol keys and generated struct names); no behavior or
  scope change
Objective: one C identity per distinct instance; no redefinition, no duplicate definition
Decision: the func type key carries the parameter and return type keys (`fn` → a spelled
  signature key; modes excluded because the C representation is mode-agnostic); env struct
  definitions dedupe to one per unique (site, instance key) across the emission paths
Evidence: types/keys.rs:66 (`TyKind::Func(_) => "fn"`); env names at codegen/function.rs:275,
  codegen/program.rs:429-430, codegen/expr.rs:845; findings .../05-findings.md:114, :149, :340
Rejected alternatives: a separate mangling pass (new machinery for a naming decision); renaming
  only the second definition (papering over the duplicate)
Strongest counterargument: 140's key change alters emitted symbol names, so a missed key site
  could desynchronize the ownership stage and the emitter — mitigated by the key's single
  definition point and the emission auditor's corpus diff
Confidence: High for 165 (a dedupe); Med-High for 140 (symbol-name change with one key owner)
Hardening: the emission-integrity auditor diffs base vs head over the corpus and re-runs the
  strict builds; the two-instance ST case pins both struct definitions
Policy version: 1
Root invocation ID: callable-release-integrity-2026-10-10
Reopen triggers: any accepted unit with a duplicate C definition; any ownership/emitter key
  disagreement visible in the corpus diff
```

**Runtime amendment (AR-16).** The drafted rule excluded parameter modes from the func key; the
implementation evidence (`types/mod.rs:82-91`: "two signatures that differ only in a parameter
mode are different types") shows a `borrow` parameter changes the C representation, so modes are
part of each parameter's key. Rejected alternative: excluding modes as drafted (would share one
struct between two genuinely different C types). Confidence: High — the type model states the
identity and the mode pair builds and runs (`should_build_two_mode_differing_func_instances_of_one_generic_struct`).
Hardening: the emission-integrity auditor's corpus diff covers the key change.

#### PD-3 — DEF-155/156/157/166 callable effect modes for every callee kind

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanism (callee resolution and mode propagation), conforming to
  frozen move/lend rules; no new diagnostics
Objective: every callable kind applies its own signature's modes exactly as its module twin
Decision: `LoweredCall.modes` is derived from the callee the call actually reaches: a func-typed
  field from the field's signature; a module-level func-typed global from the global's
  signature; a `Param`-typed callee from the substituted instance signature; a lambda parameter
  from the lambda's instantiated signature. Callee resolution is identity-based: a binding
  (local, parameter, capture) never falls back to a same-named module function's modes
Evidence: own/call.rs:158-182 (binding lookup), :219-235 (mode branches), own/call_check.rs:243;
  findings .../05-findings.md:284 (F-2/F3/F7), :331 (F-3), :332 (F-4/RV-004), :341 (P1)
Rejected alternatives: (a) apply the same-named module function's modes to a field call — the
  current comment already rejects this wrong-signature shortcut; (b) reject func-typed
  fields/globals/params — breaks accepted, definitely-working programs
Strongest counterargument: per-instance judging moves work into the instance pass and can miss
  concrete spellings; mitigated by pairing every generic ST case with its concrete control
Confidence: Med-High — the mechanism exists (DEF-125 threaded binding signatures); this extends
  it to the remaining callee kinds
Hardening: the memory auditor probes each callee kind for leaks/double-frees; the semantics
  reviewer confirms the mode reports match the module twins
Policy version: 1
Root invocation ID: callable-release-integrity-2026-10-10
Reopen triggers: any callee kind still leaking/double-freeing under ASan; any false report
  against a shadowing binding
```

**Runtime amendment (AR-17).** The DEF-166 expectation drafted here assumed a lambda's
parameter takes ownership, so the filed shape would be accepted and release once. The frozen
rules say otherwise: an unmarked parameter is a read-only lend (`spec/03-ownership.md` §2 — the
mode table), a module function storing the lend reports `E10034` (verified: the twin
`func make(p: String): Holder<String> { return Holder<String> { v: p }; }` is rejected), and the
block-bodied lambda spelling reports the same `E10034` — even per instance through the existing
lend-storage duty. The filed double-free is the expression-bodied spelling only, because an
expression body skips the return-move judgement its block-bodied sibling runs, and the call path
never materializes the lend arguments. Resolution: judge an expression body exactly like a
`return` (eval, then the move/lend judgement) and materialize owned temporary arguments on a
closure call. Rejected alternative: accept-and-release as drafted (would contradict the mode
table and diverge from the twin). Confidence: High — the spec text, the twin probe, and the
block-bodied probe agree.

#### PD-4 — DEF-139/158/159/163/164 release completeness

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanism (owner binding and cleanup emission), conforming to the
  frozen release model and view-lifetime rules; the one rejection (163) reuses the existing
  E10036/escape-check family
Objective: every owned value the program creates is released exactly once; no accepted program
  leaks its backing or double-releases
Decision: (139) a func-field call's receiver root binds like other ownerless call roots;
  (158) the defer body lowers with owned-temporary materialization enabled, owners scoped to
  the defer block's own cleanup; (159) a conditional view projection binds each owning arm's
  root at its evaluation site through the guarded materialize machinery, place-rooted arms stay
  borrows; (163) the view-return elision and escape checks cover fixed arrays of views, so the
  array spelling reports what the single-view spelling reports; (164) an owned call statement's
  unused result registers and releases like other statement temporaries
Evidence: findings .../05-findings.md:113 (PRE-2/139), :311 (F2–F7), :333 (158), :334 (159),
  :338 (163), :339 (164); own/lower.rs:1525-1545 (conditional arm), :1834 (materialization
  gate); own/statements.rs:846-847 (defer suppression), :895 (pending release copies);
  own/place.rs:1114-1128 (E10036 postlude); borrow.rs:66 (is_elided_view)
Rejected alternatives: rejecting the accepted-and-leaking shapes where the release model says
  release (breaks working programs); immediate arm-local release (the view outlives the arm)
Strongest counterargument: more owner kinds means more cleanup shapes; an incorrect owner can
  double-release — the emission diff and ASan must prove one release per creation
Confidence: Med-High — every hole reuses an existing owner/machinery precedent from Phase 5
Hardening: the memory auditor probes each shape under ASan; the emission auditor verifies the
  corpus diff contains only the intended release deltas; the semantics reviewer confirms the
  view-lifetime rules
Policy version: 1
Root invocation ID: callable-release-integrity-2026-10-10
Reopen triggers: any ASan leak/double-free at a new release shape; any release emitted on a
  path where the owner was never filled; any accepted program that still leaks its backing
```

### Complexity check

No complexity escalation: the batch adds no dependency, layer, harness, or framework. Every fix
extends machinery that already exists (scoped owners, mode propagation, type keys, cleanup
edges, the E10036 postlude). `Approved complexity: None`.
