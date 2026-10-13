# Ambiguity Register: Conditional, Projection, and Update Residue

> **Status**: ✅ GATE PASSED — all items resolved (auto-design delegated; no user-owned item is open)
> **Last Updated**: 2026-10-13 20:40
> **Batch**: I of the defect mandate (the batch H residue: the conditional/projection/update rows
> batch H filed and did not close)
> **Auto-design**: active — policy version 1 (the standing user mandate: all defects first, batches
> continue without further go-aheads; criticals first, then majors). Reserved decisions (spec text,
> new user-visible behavior, acceptance criteria) are **not** touched: every fix below is
> conformance to the frozen specification. Rows whose fix needs a spec or registry ruling stay
> parked and are collected in AR-2.

## Why this batch

Batch H closed its declared cluster (`DEF-176` – `DEF-188`, `DEF-256`) and its reviews filed 24
new rows (`DEF-257` – `DEF-280`) with reproductions and base == head evidence. Those rows carry 15
of the 37 open majors and all live in the machinery batch H just exercised: conditional arm roots,
view provenance and owners, escaped views and their stores, guarded/discarded literal positions,
fixed-array arms, and the update-order pin. The batch E residue ( `DEF-167` – `DEF-195`) that batch
H's register sequenced next now has only two unblocked majors (five of its rows await rulings) and
the batch G checker residue follows. Under the standing "criticals first, then majors" mandate the
batch-H residue is re-sequenced first: the most majors, one shared machinery, and fresh
reproductions.

| # | Category | Ambiguity / Gap | Options Presented | Resolution | Status |
|---|----------|-----------------|-------------------|------------|--------|
| 1 | Scope | Which rows batch I closes | A) the 24 batch-H residue rows minus the ruling-blocked `DEF-263` (23 rows: 15 majors, 8 minors); B) the batch E residue per the recorded sequence (8 unblocked rows: 2 majors); C) the batch G checker residue (13 majors) | **A** — severity-first: A carries 15 of the 37 open majors against B's 2, one shared root and the freshest reproductions; B and C stay sequenced after. Delegated under the standing "all defects, criticals first" mandate (batch H's own register item 1 named E→F→G before this residue existed) | ✅ Resolved |
| 2 | Scope | Rows whose fix needs a reserved ruling | A) include and pause at the ruling; B) park them, out of this batch | **B** — `DEF-167` (a call of a non-callable callee needs a diagnostic code; `DEF-214` already records the code choice as ruling-gated), `DEF-169` (an escaping key scheme is an ABI decision with its own trigger), `DEF-171` (staged `embed`), `DEF-190` (a nameless nullable source diagnostic), `DEF-191` (the `unsafe` exemption), `DEF-263` (whether a folded fixed-array `.len` receiver evaluates). All six stay open in the backlog; their rulings are collected for the user | ✅ Resolved |
| 3 | Behavior | `DEF-257`'s per-instance duty for a generic arm projection | A) judge the projection per instance through the checker's duty machinery (the concrete twin moves); B) reject the generic spelling | **A** — the concrete-phase fix deliberately gated the path off for a type that mentions a parameter; the frozen move rules (Ch 03 §3/§5) judge the projection per instance, and the concrete twin already moves, so the generic instance must too | ✅ Resolved |
| 4 | Behavior | `DEF-260`'s guarded `take` of a conditional projection | A) route the guarded position through the eager per-arm carrier (materialize on the evaluated path, one shared carrier); B) reject the spelling | **A** — the guarded call is accepted by the frozen rules; the release must run exactly once, and the shared-carrier design already exists for the eager path | ✅ Resolved |
| 5 | Behavior | `DEF-265`/`DEF-276`'s view stored where the backing dies | A) judge the stored value's provenance against the frame it escapes (reject like the return path, `E10033`); B) accept and adopt the backing | **A** — the frozen rule (BR-8, Ch 03 §7) rejects a view stored into storage that outlives the backing; the return path already reports the twin judgment | ✅ Resolved |
| 6 | Behavior | `DEF-280`'s over-strict element-store rule | A) scope the element-store loan rule to storage that outlives the frame (a field or a global), so a local array accepts; B) keep the rejection | **A** — the reviewer judged the rejection no lifetime-safety regression but inconsistent with the borrow rules: a local array dies with the frame, so its element store does not escape | ✅ Resolved |
| 7 | Behavior | `DEF-268`'s disagreeing conditional return | A) a conditional return whose arms do not agree on the single view-like candidate reports `E10036` (and the mixed spelling stops emitting invalid C); B) accept | **A** — BR-7/EL-2 require the derivation to name the single view-like lend parameter; the named non-candidate spelling already reports `E10036` | ✅ Resolved |
| 8 | Behavior | `DEF-279`'s duplicate escape report | A) suppress the generated-owner `E10030` when the same return already reports the local escape `E10035` (one report per escape site); B) keep both | **A** — the repo's diagnostics convention is one report per site; both diagnostics are true but the local escape subsumes the generated-owner one at the same span | ✅ Resolved |
| 9 | Design | `DEF-273`/`DEF-277`/`DEF-278`'s pin extension | A) extend the existing `pin_update_value` triggers (the assignment target's location and index parts, the dot receiver, a loop condition, an owning argument) inside the current lowering; B) a new pinning pass | **A** — the pin machinery, the per-position emission, and the guard/loop exclusions already exist; the rows are trigger and ownership gaps in it | ✅ Resolved |
| 10 | Design | `DEF-274`'s literal field/element order | A) pin an update-containing literal field or element at its source position; B) leave the unspecified C order | **A** — the language evaluates struct fields and array elements in source order (spec/04 §3); the emitted C must not leave it unspecified | ✅ Resolved |
| 11 | Design | `DEF-272`/`DEF-275`'s fixed-array conditional arm and conditional-root projection | A) route each arm through the fixed-array destination-store shape (the per-arm destination the discard path already uses) and bind the conditional root before the address; B) reject | **A** — the shapes are accepted by the frozen checker and Ch 17 requires valid C; the destination-store shape exists for the direct spelling | ✅ Resolved |
| 12 | Design | `DEF-262`'s loop-condition conditional literal | A) reuse the direct-literal receiver fix's per-arm binding for the loop-condition walk; B) leave | **A** — the same second-owner shape the phase-2 fix closed for a direct literal receiver; the loop condition must not give the field its own temporary while the literal temporary also owns it | ✅ Resolved |
| 13 | Verification | The command set | — | `make check` per phase; `make verify` at close-out; the focused suites (`cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test`); host ASan (`RDC_ASAN=1` and host probes) on every ownership phase; the emission auditor's base/head differential on every codegen/emission phase | ✅ Resolved |
| 14 | Naming | Plan and suite names | — | Plan `conditional-projection-residue`; suites `compiler/tests/conditional_projection_residue_spec_test.rs` and `conditional_projection_residue_impl_test.rs`; the `AGENTS.md` test map updates at close-out | ✅ Resolved |
| 15 | Lenses | Per-phase reviewer dispatch | — | P1/P3/P4 correctness + memory-safety; P2 correctness + semantics + memory-safety; P5 correctness + emission-integrity + memory-safety; P6 correctness + emission-integrity + semantics; close-out none new (per the `AGENTS.md` dispatch matrix). The read-only auditors stay read-only | ✅ Resolved |
| 16 | Specialist gap | Missing specialist types for this surface | — | None — `correctness-reviewer`, `semantics-reviewer`, the read-only `memory-safety-auditor`, and the read-only `emission-integrity-auditor` cover the batch; recorded in the index's Specialist Agents section | ✅ Resolved |
| 17 | Complexity | Escalation triggers | — | None — every fix extends existing machinery (the arm binders, the owner walk, the provenance judgment, the discard/literal binders, the fixed-array destination shape, the update pin); no new architecture, dependency, or cross-cutting refactor | ✅ Resolved |
| 18 | Deviation (runtime) | A phase's named fix surface proves narrower than the row (the batch-H precedent: `DEF-178` needed checker-side marking beyond `own/lower.rs`) | A) stop and re-plan; B) extend the phase's task files to the needed read-only-adjacent modules with the evidence, and record the deviation here | **B** — the batch-H register's items 15/16/17 set the precedent: a conformance fix may touch the checker module that owns the judgment, and the deviation is recorded and reviewed; the phase review then covers the extra files | ✅ Resolved (runtime, delegated) |

### Resolution Notes

**AR-1:** Re-sequencing evidence: open majors — batch-H residue 15, batch G residue 13, batch E
residue 2 unblocked (of 3; `DEF-171` is ruling-gated), batch F residue 5. The batch-H residue's
rows share the conditional/projection machinery and the reproductions the batch-H reviews kept;
the E residue stays sequenced (its unblocked rows are 2 majors + 6 minors) and the F/G residues
follow.

**AR-2:** Parked rows and the decision each needs:
`DEF-167` — the diagnostic code for calling a non-callable callee (shared with `DEF-214`);
`DEF-169` — whether to add an escaping scheme for generated type keys (an ABI decision);
`DEF-171` — whether a staged `embed` outside a constant context stays rejected;
`DEF-190` — the diagnostic for a nameless nullable source;
`DEF-191` — whether the `unsafe` exemption for the null-use rule matches the conversion rule;
`DEF-263` — whether a folded fixed-array `.len` receiver evaluates its operand.

**AR-3:** `DEF-257`'s per-instance duty rides the checker's existing projection duty (the
`projection_move_arm_sources` mark from batch H's phase 1); the generic spelling is judged like the
concrete one instead of being skipped.

**AR-9:** The pin triggers to add: the assignment target's location and index parts (before the
right-hand side, spec/04 §3), the dot receiver as argument position zero, a loop condition (a
per-iteration pin), and an owning (non-duplicable) argument whose copy carries ownership (a scope
owner or move carrier at its own position).

**AR-13:** No command is invented: `make check` and `make verify` come from the project
`AGENTS.md`; the focused suites follow the batch-H naming; host ASan is the established memory
probe.

**AR-15:** P5's fixed-array work is emission-shaped (the hidden out-parameter ABI), so the
emission-integrity auditor is included; P2/P6 also change judged rules, so the semantics reviewer
joins those phases.
