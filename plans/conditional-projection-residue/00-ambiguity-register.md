# Ambiguity Register: Conditional, Projection, and Update Residue

> **Status**: ✅ GATE PASSED — all in-batch items resolved (auto-design delegated); no batch row is
> parked (the two preflight parks were re-decided with fresh evidence — AR-25/AR-26); the six
> older rows stay parked for user rulings (AR-2)
> **Last Updated**: 2026-10-15 06:28
> **Batch**: I of the defect mandate (the batch H residue: 23 handled rows — 21 implemented, two
> incidentally closed (`DEF-259` at the planning baseline, `DEF-268` by the batch-H emission work),
> no batch row parked)
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
| 1 | Scope | Which rows batch I closes | A) the 24 batch-H residue rows minus the ruling-blocked `DEF-263` (23 rows: 15 majors, 8 minors); B) the batch E residue per the recorded sequence (8 unblocked rows: 2 majors); C) the batch G checker residue (13 majors) | **A** — severity-first: A carried 15 of the 37 open majors against B's 2, one shared root and the freshest reproductions; the batch handles 23 rows: 21 implemented and two incidentally closed (`DEF-259` at the planning baseline, `DEF-268` by the batch-H emission work — AR-23/AR-25), none parked — the preflight's two parks were re-decided with fresh evidence on 2026-10-15 (AR-25/AR-26). B and C stay sequenced after. Delegated under the standing "all defects, criticals first" mandate (batch H's own register item 1 named E→F→G before this residue existed) | ✅ Resolved |
| 2 | Scope | Rows whose fix needs a reserved ruling | A) include and pause at the ruling; B) park them, out of this batch | **B** — the parked list is exactly the six older rows: `DEF-167` (a call of a non-callable callee needs a diagnostic code; `DEF-214` already records the code choice as ruling-gated), `DEF-169` (an escaping key scheme is an ABI decision with its own trigger), `DEF-171` (staged `embed`), `DEF-190` (a nameless nullable source diagnostic), `DEF-191` (the `unsafe` exemption), `DEF-263` (whether a folded fixed-array `.len` receiver evaluates). The two rows the preflight parked (`DEF-268`, `DEF-279`) were re-decided with fresh evidence on 2026-10-15 (AR-25/AR-26) and are executed rows again, no longer parked. The six stay open in the backlog; their rulings are collected for the user | ✅ Resolved |
| 3 | Behavior | `DEF-257`'s per-instance duty for a generic arm projection | A) judge the projection per instance through the checker's duty machinery (the concrete twin moves); B) reject the generic spelling | **A** — the concrete-phase fix deliberately gated the path off for a type that mentions a parameter; the frozen move rules (Ch 03 §3/§5) judge the projection per instance, and the concrete twin already moves, so the generic instance must too | ✅ Resolved |
| 4 | Behavior | `DEF-260`'s guarded `take` of a conditional projection | A) route the guarded position through the eager per-arm carrier (materialize on the evaluated path, one shared carrier); B) reject the spelling | **A** — the guarded call is accepted by the frozen rules; the release must run exactly once, and the shared-carrier design already exists for the eager path | ✅ Resolved |
| 5 | Behavior | `DEF-265`/`DEF-276`'s view stored where the backing dies | A) judge the stored value's provenance against the frame it escapes (reject like the return path; `E10033` for a field or global, `E10030` for an out-slot); B) accept and adopt the backing | **A** — the frozen rule (BR-8, Ch 03 §7) rejects a view stored into storage that outlives the backing; the return path already reports the twin judgment. The destination kind decides the code and which owners survive (AR-20) | ✅ Resolved |
| 6 | Behavior | `DEF-280`'s over-strict element-store rule | A) scope the element-store loan rule to the backing's scope — a local array accepts a backing whose scope covers its own (a loop iteration or a nested block reports `E10030`), recording the binding path's loan; B) keep the rejection | **A** — the reviewer judged the rejection no lifetime-safety regression but inconsistent with the borrow rules: a local array dies with the frame, so its element store does not escape; the loan record makes the scope rule enforceable (AR-21) | ✅ Resolved |
| 7 | Behavior | `DEF-268`'s disagreeing conditional return | A) a conditional return whose arms do not agree on the single view-like candidate reports `E10036` (and the mixed spelling stops emitting invalid C); B) accept | **Superseded (re-decided 2026-10-15 — AR-25; option B holds)** — no rejection is added: the row's repro is fixed at HEAD, where the emitted C converts the owned arm on its own path (`return (c ? x : cl_string_view(h.v));`), builds under `cc -std=c99 -Wall -Wextra -Werror` and under `-fsanitize=address`, and runs exit 0 leak-free. The batch-H per-arm conditional-view conversion (pinned accepted by `conditional_projection_integrity_spec_test.rs:2313-2333`) covers both arm orders, so the suggested `E10036` is unnecessary; ST-10 becomes the reversed-order green control | ✅ Resolved (superseded — AR-25) |
| 8 | Behavior | `DEF-279`'s duplicate escape report | A) suppress the generated-owner `E10030` (keeping the local `E10035`); B) keep both; C) withhold the local-owner `E10035` at the site where the generated-owner escape reports | **C (re-decided 2026-10-15 — AR-26)** — the one-report rule keeps the pinned generated-owner report: at a return whose mixed conditional has, on the other arm, a fresh owning expression whose lowered view reads a compiler-generated root or a statement temporary, the local-owner `E10035` is withheld and the liveness rule's `E10030` carries the rejection, so the immutable oracle `callable_release_integrity_spec_test::should_reject_a_mixed_conditional_view_return` (:1772-1802, exactly one `E10030`) keeps holding. Shapes where the local backing is the only escape keep exactly one `E10035`; a non-dying sibling (for example a raw-pointer view arm) keeps its `E10035`, so no acceptance hole opens; the parameter variant keeps `E10036` + `E10030` | ✅ Resolved (re-decided — AR-26) |
| 9 | Design | `DEF-273`/`DEF-277`/`DEF-278`'s pin extension | A) extend the existing `pin_update_value` triggers (the assignment target's location and index parts, the update-containing right-hand side, the dot receiver, a loop condition, an owning argument) inside the current lowering; B) a new pinning pass | **A** — the pin machinery, the per-position emission, and the guard/loop exclusions already exist; the rows are trigger and ownership gaps in it. The loop-condition pin is a per-iteration `Materialize` fill (a plain statement pin is what the `in_loop_condition` guard excludes), and an update-containing right-hand side pins at its own source position | ✅ Resolved |
| 10 | Design | `DEF-274`'s literal field/element order | A) pin an update-containing literal field or element at its source position; B) leave the unspecified C order | **A** — the language evaluates struct fields and array elements in source order (spec/04 §3); the emitted C must not leave it unspecified | ✅ Resolved |
| 11 | Design | `DEF-272`/`DEF-275`'s fixed-array conditional arm and conditional-root projection | A) route each arm through the fixed-array destination-store shape (the per-arm destination the discard path already uses) and bind the conditional root before the address; B) reject | **A** — the shapes are accepted by the frozen checker and Ch 17 requires valid C; the destination-store shape exists for the direct spelling | ✅ Resolved |
| 12 | Design | `DEF-262`'s loop-condition conditional literal | A) reuse the direct-literal receiver fix's per-arm binding for the loop-condition walk; B) leave | **A** — the same second-owner shape the phase-2 fix closed for a direct literal receiver; the loop condition must not give the field its own temporary while the literal temporary also owns it | ✅ Resolved |
| 13 | Verification | The command set | — | `make check` per phase; `make verify` at close-out; the focused suites (`cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test`); host ASan (`RDC_ASAN=1` and host probes) on every ownership phase; the emission auditor's base/head differential on every codegen/emission phase | ✅ Resolved |
| 14 | Naming | Plan and suite names | — | Plan `conditional-projection-residue`; suites `compiler/tests/conditional_projection_residue_spec_test.rs` and `conditional_projection_residue_impl_test.rs`; the `AGENTS.md` test map updates at close-out | ✅ Resolved |
| 15 | Lenses | Per-phase reviewer dispatch | — | P1/P4 correctness + memory-safety; P2 correctness + semantics + memory-safety; P3 correctness + semantics + memory-safety + emission-integrity (its fix changes acceptance and implements in `codegen`); P5 correctness + emission-integrity + memory-safety; P6 correctness + emission-integrity + semantics; close-out none new (per the `AGENTS.md` dispatch matrix). The read-only auditors stay read-only | ✅ Resolved |
| 16 | Specialist gap | Missing specialist types for this surface | — | None — `correctness-reviewer`, `semantics-reviewer`, the read-only `memory-safety-auditor`, and the read-only `emission-integrity-auditor` cover the batch; recorded in the index's Specialist Agents section | ✅ Resolved |
| 17 | Complexity | Escalation triggers | — | None — every fix extends existing machinery (the arm binders, the owner walk, the provenance judgment, the discard/literal binders, the fixed-array destination shape, the update pin); no new architecture, dependency, or cross-cutting refactor | ✅ Resolved |
| 18 | Deviation (runtime) | A phase's named fix surface proves narrower than the row (the batch-H precedent: `DEF-178` needed checker-side marking beyond `own/lower.rs`) | A) stop and re-plan; B) extend the phase's task files to the needed read-only-adjacent modules with the evidence, and record the deviation here | **B** — the batch-H register's items 15/16/17 set the precedent: a conformance fix may touch the checker module that owns the judgment, and the deviation is recorded and reviewed; the phase review then covers the extra files | ✅ Resolved (runtime, delegated) |
| 19 | Behavior | `DEF-261`'s live red spelling | A) recurse through the view-returning call's argument and collect **every** arm's owners, exactly as the direct walk does; B) keep the call path's short-circuit and judge per argument | **A** — the local-arm spelling already reports at the head (`E10031`) and becomes the control; the parameter/parameter and two-arm spellings are the accepted use-after-free, and the direct walk already collects every arm | ✅ Resolved (delegated) |
| 20 | Behavior | `DEF-265`/`DEF-276`'s destination kinds and the out-slot code | A) judge per destination kind — a global or field accepts module-level static owners only (parameter-backed stores stay rejected, `E10033`), an out-slot accepts static and parameter-backed owners (the caller frame outlives the callee) and otherwise reports `E10030`; B) one rule for every destination | **A** — the frozen rule judges the storage, not the value's shape: a parameter does not outlive a global, while the caller's frame outlives the callee's slot; an out-slot is a parameter, so `E10033`'s field/global trigger does not fit. One report per site | ✅ Resolved (delegated) |
| 21 | Behavior | `DEF-280`'s element-store acceptance scope and loan | A) judge the store against the destination-expected type (a handle→view widening counts), record the stored view's owner loan exactly as the binding path, and accept only backings whose scope covers the array's (a loop iteration or a nested block reports `E10030`); B) accept any local-backed store without a loan | **A** — acceptance without the loan leaves re-executed loop stores and nested-scope backings as accepted use-after-frees (probed); the binding path's loan makes the scope rule enforceable | ✅ Resolved (delegated) |
| 22 | Behavior | `DEF-270`'s discard alias model: move or read | A) a place-valued literal field in a discarded arm is a **read**, matching the direct `ok ? t : u;` behavior (pinned as a control); the fresh fields materialize for release; B) move the place field into the literal | **A** — the head's move model rejects a later use of the field (`E10020`), contradicting the pinned control; the discard never moves a place field | ✅ Resolved (delegated) |
| 23 | Verification | `DEF-259`'s reproduction is green at the planning baseline | A) reclassify as incidentally closed (batch H closed the probed spellings), keep the program as a green control, and close the roadmap row with the incidental note; B) keep it red and force a shape change | **A** — the defect contract is the closed failure, not a forced diff; the probe evidence is recorded and a still-leaking spelling becomes a new row with base == head evidence (the batch convention). ST-3 becomes the control | ✅ Resolved (delegated) |
| 24 | Verification | P6's red-phase basis | A) re-base the red on the missing site/emission pins for the literal shapes and make ST-26's `g` order-observing; B) keep a claimed host-only red for all cases | **A, corrected** — the pinned container (GCC 12.2 / clang 14) rejects ST-23/ST-25's sequence shapes under the suite's own strict flags, so their strict-build reds reproduce; ST-24's literal shapes build and are ordered left-to-right, so its red is the missing site/emission pins, and ST-26 is its own red | ✅ Resolved (delegated) |
| 25 | Verification | `DEF-268`'s disagreeing conditional return at HEAD | A) keep the `E10036` rejection and re-pin the batch-H suite; B) close the row as incidentally fixed | **B** — the row's reproduction is fixed at HEAD: the emitted C converts the owned arm on its own path (`return (c ? x : cl_string_view(h.v));`), builds under `cc -std=c99 -Wall -Wextra -Werror` and under `-fsanitize=address`, and runs exit 0 leak-free; the batch-H per-arm conversion covers both arm orders, so the suggested rejection is unnecessary. ST-10 becomes the reversed-order green control | ✅ Resolved (re-decided 2026-10-15) |
| 26 | Behavior | `DEF-279`'s duplicate escape report | A) suppress the generated-owner report; B) keep both; C) withhold the local-owner report where the generated-owner escape fires | **C** — one report per site: at a return whose mixed conditional's other arm is a fresh owning expression whose lowered view reads a compiler-generated root or a statement temporary, the local-owner `E10035` is withheld and the escape `E10030` carries the rejection, so the immutable pin (exactly one `E10030`) holds; local-only/local-plus-parameter/raw-sibling shapes keep exactly one `E10035`; the parameter variant keeps `E10036` + `E10030` | ✅ Resolved (re-decided 2026-10-15) |

### Resolution Notes

**AR-1:** Re-sequencing evidence: open majors — batch-H residue 15 (the preflight parked the major
`DEF-268` briefly; re-decided 2026-10-15 — AR-25), batch G residue 13, batch E residue 2 unblocked
(of 3; `DEF-171` is ruling-gated), batch F residue 5. The batch-H residue's rows share the
conditional/projection machinery and the reproductions the batch-H reviews kept; the E residue
stays sequenced (its unblocked rows are 2 majors + 6 minors) and the F/G residues follow.

**AR-2:** Parked rows and the decision each needs:
`DEF-167` — the diagnostic code for calling a non-callable callee (shared with `DEF-214`);
`DEF-169` — whether to add an escaping scheme for generated type keys (an ABI decision);
`DEF-171` — whether a staged `embed` outside a constant context stays rejected;
`DEF-190` — the diagnostic for a nameless nullable source;
`DEF-191` — whether the `unsafe` exemption for the null-use rule matches the conversion rule;
`DEF-263` — whether a folded fixed-array `.len` receiver evaluates its operand.
The two preflight parks (`DEF-268`, `DEF-279`) were re-decided with fresh evidence on 2026-10-15
(AR-25, AR-26) and are no longer parked; only the six rows above remain.

**AR-3:** `DEF-257`'s per-instance duty rides the checker's existing projection duty (the
`projection_move_arm_sources` mark from batch H's phase 1); the generic spelling is judged like the
concrete one instead of being skipped.

**AR-9:** The pin triggers to add: the assignment target's location and index parts (before the
right-hand side, spec/04 §3), an update-containing right-hand side at its own source position (so
the update completes before the store), the dot receiver as argument position zero, a loop
condition (a per-iteration `Materialize` fill, because a plain statement pin is what the
`in_loop_condition` guard excludes), and an owning (non-duplicable) argument whose copy carries
ownership (a scope owner or move carrier at its own position).

**AR-13:** No command is invented: `make check` and `make verify` come from the project
`AGENTS.md`; the focused suites follow the batch-H naming; host ASan is the established memory
probe.

**AR-15:** P5's fixed-array work is emission-shaped (the hidden out-parameter ABI), so the
emission-integrity auditor is included; P2/P6 also change judged rules, so the semantics reviewer
joins those phases; P3 changes acceptance and implements in `codegen`, so the semantics reviewer
and the emission-integrity auditor join it as well.

**AR-19:** The direct owner walk already collects every arm's owners; the call path is the gap.
The local-arm spelling reports at the head (`E10031` at the mutation) and becomes a control; the
parameter/parameter and two-arm spellings carry the red (accepted and use-after-free).

**AR-20:** The destination kind decides which owners survive: a global or field outlives any
parameter, so only module-level static owners are accepted there (`E10033`); an out-slot lives in
the caller's frame, which outlives the callee, so static and parameter-backed owners are accepted
and a fresh- or local-derived view reports `E10030`. One report per site.

**AR-21:** The element store is judged against the destination-expected type (a handle→view
widening counts), records the stored view's owner loan exactly as the binding path does, and
accepts only a backing whose scope covers the array's; a loop iteration or a nested block reports
`E10030`.

**AR-22:** A discarded conditional's place-valued literal fields are reads (the direct
`ok ? t : u;` behavior, pinned as a control); the fresh fields materialize for release. The
discard never moves a place field.

**AR-23:** `DEF-259`'s probed spellings are green at the planning baseline (batch H incidentally
closed the literal-arm root); the defect contract is the closed failure, so the row closes with
the probe evidence and the program stays a green control. ST-3 carries the control.

**AR-24:** On the pinned container toolchain (GCC 12.2 and clang 14) the sequence shapes are
rejected under the suite's own strict flags (`-Wall -Wextra -Werror` selects
`-Werror=sequence-point` / `-Werror,-Wunsequenced`), so ST-23/ST-25's strict-build reds reproduce
as the roadmap rows state; a host GCC 15 outside the container happens to accept them, so the red
phase records the container result. The literal shapes (ST-24) build and are ordered
left-to-right by the container compilers, so their red is the missing site/emission pins, and
ST-26's order-observing case is its own red.

**AR-25:** `DEF-268` was re-decided 2026-10-15: the row's reproduction
(`func pick(x: str, h: Box, c: bool): str { return c ? x : h.v; }`) is fixed at HEAD. The emitted C
converts the owned arm on its own path (`return (c ? x : cl_string_view(h.v));`), builds under
`cc -std=c99 -Wall -Wextra -Werror` and under `-fsanitize=address`, and runs exit 0 leak-free. The
batch-H per-arm conditional-view conversion (pinned accepted by
`conditional_projection_integrity_spec_test.rs`'s
`should_build_a_view_typed_conditional_over_an_owned_member`, covering both arm orders) makes the
suggested `E10036` rejection unnecessary, so the row closes as incidentally fixed by the batch-H
emission work. ST-10 becomes the reversed-order green control (accepted, strict build, ASan-clean,
exit 0) so the conversion cannot regress.

**AR-26:** `DEF-279` was re-decided 2026-10-15 as one report per site: at a return whose mixed
conditional has, on the other arm, a fresh owning expression whose lowered view reads a
compiler-generated root or a statement temporary, the local-owner `E10035` is withheld and the
liveness rule's `E10030` carries the rejection — so the immutable pin
(`callable_release_integrity_spec_test::should_reject_a_mixed_conditional_view_return`, exactly one
`E10030`) keeps holding. Shapes where the local backing is the only escape — `return (c ? b.v : a)`
with a local `b` and a parameter `a`, `return [k]`, `bytes(h.name)` — keep exactly one `E10035`. A
non-dying sibling (for example a raw-pointer view arm) keeps its `E10035`, so no acceptance hole
opens. Observation: the parameter variant `return (c ? a : mkBox()).v` with `a: Box` a parameter
reports `E10036` + `E10030` — two different judgments; it is left unchanged.
