# Task T-07: Reject a foreign lend stored into a whole out-slot

> **Type**: Task (multi-phase) · **CodeOps Artifact Schema**: 1
> **Progress**: 7/27 tasks (26%)
> **Last Updated**: 2026-10-22 01:18
> **Rows**: `DEF-541`
> **Baseline**: `a6dde7d` (plan creation); re-baseline at the first implementation commit
> **Reasoning**: high — critical memory safety, a frozen-spec clarification, and a deliberate reversal of the AR-20 acceptance
> **Ruling**: Preflight PF-006/PF-007 Option A (2026-10-22); see `00-preflight-report.md`

## Objective

Close `DEF-541`: a whole nullable out-slot store must not keep a value or view
of a **different** lend parameter. The callee's lend is call-scoped (BR-5);
the caller can reassign the backing after the call and leave the out-slot
dangling. The rule is one `E10034` at the store, for both a view source and an
owned source.

**Best outcome (chosen after preflight).** One consistent judgment for the
concrete and generic spellings:

- reject a non-`take` lend from a different root — unmarked read-only,
  `const`, or `borrow`, at any projection, conditional arm, capture, or
  view-returning call;
- accept the slot's own incoming value, static/module backing (a `static str`
  field; a static-backed fixed-array element is a recorded precision loss), a
  `take` source, and a local or fresh owned value;
- keep a local/fresh **view** on the existing `E10030` path;
- report once per store: the callee definition owns the report, and the
  caller-side cross-frame `E10030` is withheld for a callee that has this
  store (FFI/opaque callees keep it);
- narrow the frozen-spec exemption to the slot's own incoming value, and
  retarget the oracle before implementation;
- migrate the fixture programs — the 188 exact-helper occurrences plus every
  other whole-slot foreign-store fixture the inventory finds — without losing
  their original test intent.

The earlier "lightweight" estimate was wrong: the rule reaches at least 188
exact-helper occurrences across four test files, plus the wider whole-slot
store shapes in the same files and in at least two more test files (for
example `aggoutslot`, `outslotpartwide`, `closeout-st14`, and the capture
cases). The inventory in Phase 1 determines the final set.

## Ruling and specification change

The 2026-10-22 preflight ruling (PF-006/PF-007 Option A) authorizes:

- amend `spec/03-ownership.md` §6, §7 (the rule table), and §9 (OW-16),
  `spec/06-functions.md` §4, `spec/16-borrow-rules.md` §2,
  `spec/14-diagnostics.md` E10034, `spec/12-ffi-contracts.md` CN-16, and
  `spec/decisions.md` PF-001 to the narrow exemption;
- reverse the AR-20 acceptance: the caller frame outliving the callee is
  necessary but not sufficient, because the caller can reassign the backing
  after the call. AR-20 is recorded in
  `plans/conditional-projection-residue/00-ambiguity-register.md` row 20;
- retarget the affected specification-test pins (listed in Phase 1);
- reject the foreign **owned** move at a nullable instance, matching the
  concrete `borrow v: String` → `String?` spelling;
- accept the precision loss for the safe-but-unprovable parameter-backed and
  same-frame pass-through programs; they are rejected because the checker
  cannot prove the caller will not reassign the backing.

## Decision

| Option | Ruling |
|---|---|
| Reject a foreign non-`take` lend stored into a whole nullable out-slot (`E10034`), view and owned | **Chosen** |
| Preserve acceptance with per-callee summaries and caller-side loans | Rejected: materially larger; BR-5 already mandates rejection; the frozen soundness stance rejects what it cannot prove |
| Restrict to `borrow`/`const` sources | Rejected: leaves the `fill(p: str, ...)` repro accepted |
| Keep two reports for the nullable out-slot shape (caller `E10030` + callee `E10034`) | Rejected: violates the one-report contract; a one-bit callee effect is smaller |
| Add a new diagnostic | Rejected: `E10034` already reports the concrete foreign owned store; the spec amendment keeps one code |

## Source matrix

The store is a whole-binding assignment `slot = value` into a `borrow`
parameter (or a captured whole out-slot), with no projections.

| Stored source | View slot (`str?`, `T[]?`, `raw?`, `cstring?`) | Owning slot (`String?`, `File?`) |
|---|---|---|
| Destination root, direct `slot = slot` | existing `E10020` self-assignment rule | existing `E10020` |
| Local copy read from the slot (`let t = slot; slot = t;`) | accept (no new backing) | accept; move-out/back obeys the existing rules |
| `static` / module global / `static str` field | accept | accept |
| Fixed-array element of a by-value aggregate with static contents | recorded precision loss: `E10034` (the parameter type does not prove the static backing) | n/a (the `str`-to-owning conversion reports `E10200`) |
| Different non-`take` lend (unmarked, `const`, `borrow`), any projection | `E10034` | `E10034` |
| Different out-slot | `E10034` | `E10034` |
| `take` owning source | the view of the taken value dies with the frame → existing `E10030` | accept (ownership transfer) |
| Taken view source | existing `E10030` (the view dies with the frame) | n/a (a taken view does not convert to an owning slot) |
| Local owner or fresh call result | existing `E10030` | existing move rules |
| Local copy of a foreign lend (`let s = p; slot = s`) | `E10034` (the copy keeps the foreign root) | n/a; moving out of the lend is already `E10034` |
| Conditional / array literal | one `E10034` if any component is a foreign lend; otherwise the first frame-backed component reports `E10030` | one `E10034` if any component is a foreign lend; otherwise the existing move rules |
| View-returning call derived from a foreign lend | `E10034`, judged before the view-return withholding | `E10034` (foreign sourced) |
| View-returning call derived from static / the slot's own value | accept | accept |
| Captured foreign lend | `E10034`, unless the capture-move rule already reports `E10401`; then the capture rule owns the one report | `E10034`, unless the capture rule reports |
| Captured frame-local view | `E10030` | existing move rules |
| Duplicable value / non-nullable slot | no view; unchanged | existing rules |

Notes:

- The report names the source root parameter (`p`, `v`, `h`) at the store
  span. One report per store: the foreign `E10034` takes precedence over
  `E10030` when a conditional has both kinds; the first foreign component in
  source order names the parameter.
- A `static str` field of a by-value aggregate is static storage; the check
  must not treat its root parameter as foreign.
- A fixed-array element of a by-value aggregate with static contents
  (`Holder { names: ["AA", "BB"] }`) is static-backed in the source but not
  provable from the callee's parameter type; the `aggstatic` control is
  retargeted to `E10034` as a recorded precision loss. A static-aggregate
  provenance analysis can restore the acceptance later.
- The non-nullable `str[2]` array slot is not a whole out-slot; it keeps the
  existing `E10033` + `E10030` contract and is not part of the one-report
  bit.
- A captured writable borrow stored into an out-slot overlaps the
  capture-move rule: the capture rule owns the report (`E10401`) and the
  out-slot rule withholds, so one report stands. A captured by-value copy is
  not a lend and keeps the capture rule; a captured foreign lend the capture
  rule does not reject reports `E10034`.
- The non-nullable writable view slot (`borrow d: str`) keeps the concrete
  `E10033`. The generic `copy<str>` spelling is a separate residual
  (PF-010), not fixed here.
- The generic owned move (`copy<String?>`) is rejected at the instance,
  matching the concrete `borrow v: String` → `String?` spelling.

## Tasks

### Phase 0 — Ruling record and frozen-spec clarification

> **Phase baseline tree**: 9c6bd1d12cb2df01817b91cb63ae1d34bd685eb7

- [x] 0.1.1 Record the ruling in the plan and the preflight report
  (PF-006/PF-007 Option A, 2026-10-22); cite AR-20's location.
  ✅ (completed: 2026-10-22 01:04)
- [x] 0.1.2 Amend the frozen spec: `03-ownership.md` §6, §7 (the rule
  table at line 239), and §9 (OW-16 at line 292), `06-functions.md` §4,
  `16-borrow-rules.md` §2, `14-diagnostics.md` E10034, `12-ffi-contracts.md`
  CN-16, `decisions.md` PF-001. The exemption is the slot's **own incoming
  value**; a foreign lend's value or view is `E10034`. Update the canonical
  help to "declare the parameter `take` or by value; a `borrow T?` slot may
  hold its own incoming value, a static view, or a newly owned value".
  ✅ (completed: 2026-10-22 01:04)
- [x] 0.1.3 Run `./spec/check.sh`; confirm the diagnostic registry and links;
  no new code and no new compile-fail golden (E10034 already exists).
  ✅ (completed: 2026-10-22 01:04)

### Phase 1 — Red cases and oracle retargeting

> **Phase baseline tree**: de2a650404aa9d2854a74e4567ad909bfb50d719

- [x] 1.1.1 Concrete spec cases in `borrow_store_integrity_spec_test.rs`:
  direct `fill(p: str, borrow slot: str?)`; field
  `store(h: H) { slot = h.name; }`; static field `h.tag` accepted;
  conditional foreign/foreign and foreign/static (one report); captured
  `slot = p`; captured same-slot copy
  `([borrow slot] () => { let t = slot; slot = t; })()` accepted;
  view-returning call `slot = id(p)`; cross-slot `a = b`; same-slot
  `slot = slot` (`E10020`) and copy-via-local accepted; a local owner's
  view or a fresh view `E10030`; a local copy of a foreign lend `E10034`;
  static/module accepted; an owning `take` source accepted; a taken view
  source `E10030`. ✅ (completed: 2026-10-22 01:16)
- [x] 1.1.2 Generic spec cases in `borrow_generic_integrity_spec_test.rs`:
  `copy<str?>` → `E10034`; `copy<String?>` → `E10034` (owned move);
  the concrete `fillString(take v: String, borrow slot: String?)` accepted;
  an owning `take` store at a nullable instance accepted; a taken view
  source `E10030`; a non-nullable owning `T=String` take store keeps the
  existing `RequiresStorableSlot` rejection and is not an acceptance
  control; a non-nullable duplicable slot accepted; the existing
  `both-generic` `E10034` unchanged. ✅ (completed: 2026-10-22 01:16)
- [ ] 1.1.3 Retarget the explicit pins: `mixedcarrier` and `aggoutslot`
  `E10030` → `E10034`; `aggstatic` accepted → `E10034` (the recorded
  static-array precision loss). For `outslotpartwide` and `closeout-st14`,
  classify by intent: if the test asserts the foreign store, retarget to
  `E10034`; if it uses the store only to exercise widening or write-through,
  rewrite the helper source to `static` or the slot's own value and keep the
  original assertion. The
  `CROSS_FRAME_PARAM_SOURCE`/`SAME_SOURCE`, `OWNER_PASS_*`, and
  `captureoutslotviewok`/`local` accepted controls → `E10034`; keep the
  static captured control accepted; in the capture `E10401` cases decide
  that the capture rule owns the report and the out-slot rule withholds (no
  added `E10034`); split
  `should_accept_and_run_when_a_slot_instantiation_is_nullable` so the `take`
  and null-slice sub-cases stay accepted and the `copy<String?>` sub-case
  becomes `E10034`.
- [ ] 1.1.4 Inventory every whole-binding store into a `borrow` slot (or a
  captured whole out-slot) whose source is a non-`take` lend, across all
  `compiler/tests/*.rs` and `compiler/src/own/tests.rs`; start from the 188
  exact-helper occurrences and add
  the known wider shapes (`eval_emission_integrity_spec_test.rs`
  `aggoutslot`/`aggstatic` plus the `store(h: Holder)`, `store(s: str)`,
  `store(h: Box)`, and `store(h: str[2])` shapes — 21/13/4/3 in the spec
  file and 5/2/1/1 in the impl file at the plan baseline;
  `conditional_projection_residue_spec_test.rs` `outslotpartwide`;
  `def_backlog_closeout_spec_test.rs` `closeout-st14`; the
  `checker_conformance_*` capture cases; and their `impl` twins).
  Classify each as unsafe rejection, safe-but-unprovable rejection,
  static-array precision loss, or incidental setup; record the final count
  and file list in the phase notes under `$CODEOPS_TMPDIR`.
- [ ] 1.1.5 Migrate `conditional_projection_residue_spec_test.rs`
  (103 occurrences): retarget the foreign pass-through acceptances to
  `E10034`; rewrite incidental setup to a static or `take` source; keep the
  static and own-value controls accepted.
- [ ] 1.1.6 Migrate `conditional_projection_residue_impl_test.rs`
  (69 occurrences): the same classification; update the emission and
  byte-identity pins for the accepted programs.
- [ ] 1.1.7 Migrate the whole-slot store fixtures in
  `eval_emission_integrity_spec_test.rs` and
  `eval_emission_integrity_impl_test.rs`: the 12/4 `fill` helpers, the
  21/5 `store(h: Holder)`, 13/2 `store(s: str)`, 4/1 `store(h: Box)`, and
  3/1 `store(h: str[2])` shapes; retarget the foreign stores, keep the
  static and own-value controls, and keep the captured static control.
- [ ] 1.1.8 Migrate `borrow_store_integrity_*`,
  `borrow_generic_integrity_*`, `def_backlog_closeout_*`, and
  `checker_conformance_*`: retarget `mixedcarrier`, `nullable-generic`,
  `closeout-st14`, the capture out-slot cases, and any remaining whole-slot
  fixture from the inventory; keep the `take`, static, and own-value
  controls.
- [ ] 1.1.9 Confirm red: the new cases fail with the expected missing
  `E10034` or wrong code; the migrated acceptance controls fail until the
  implementation lands; no implementation changes yet.

### Phase 2 — Concrete whole-out-slot judgment

> **Phase baseline tree**: e7d366b73ec3e55ea6a0c504186d0ee514cc62d7

- [x] 2.1.1 Add the source-matrix helper and the whole-out-slot foreign-lend
  judgment in `check_escaping_view_store`, before the
  `callee_elides_no_single_parameter` withholding; handle projections,
  conditionals, captures, and view-returning calls; accept a `static str`
  position; keep the existing `E10030` for local/fresh views.
  ✅ (completed: 2026-10-22 01:18)
- [x] 2.1.2 Update the canonical `E10034` help at the seven sites (or through
  a shared helper) and the diagnostic site; keep the message
  `cannot release or store borrowed parameter '{name}'`.
  ✅ (completed: 2026-10-22 01:18)
- [~] 2.1.3 Run the focused `borrow_store_integrity_*`,
  `conditional_projection_residue_*`, and `eval_emission_integrity_*` suites,
  the `compiler/src/own/tests.rs` unit tests, and the `RDC_ASAN=1` ownership
  probes. ⏳ (verification pending: 2026-10-22 01:18; the existing fixture
  migration is AR-1)

### Phase 3 — Generic whole-slot duty and the owned move

- [ ] 3.1.1 Add a dedicated whole-out-slot store duty carrying the source
  root, mode, and type, the destination root and type, and the span; record
  it for a whole-binding store into a `borrow` slot whose declared type is a
  type parameter (or unknown) and whose instance may be nullable; keep the
  existing `RequiresNotLend` for non-slot destinations.
- [ ] 3.1.2 Judge the duty per instance: a foreign non-`take` source at a
  nullable destination → one `E10034` (first offending instance); the
  destination's own root, `take`, static, duplicable, and non-nullable
  substitutions → accept/unchanged.
- [ ] 3.1.3 Route the whole-slot store through the new duty so
  `check_not_lend_requirements` no longer records it; keep the existing
  nullable `writable_source && stores_into_slot` exemption for non-whole slot
  stores. Retarget `copy<String?>` to `E10034`; keep the concrete `take` and
  null-slice sub-cases accepted.
- [ ] 3.1.4 Run the generic suites, the `compiler/src/own/tests.rs` unit
  tests, and `RDC_ASAN=1`.

### Phase 4 — One report per program

- [ ] 4.1.1 Add the per-callee one-bit effect "the definition contains a
  whole-slot store whose destination may be nullable and whose source is a
  non-`take` lend"; compute it in a diagnostic-free pre-pass before the
  caller-side check. A post-pass is rejected because it cannot retract an
  already-emitted diagnostic. The bit is shape-based, not instance-verdict
  based: the callee-side judgment owns the report at the rejecting instances
  and accepts at the accepting ones.
- [ ] 4.1.2 Withhold the caller-side `check_cross_frame_pass_through`
  `E10030` for a callee with the bit; keep it for FFI/opaque callees with no
  body. The non-nullable `str[2]` array slot keeps its existing `E10033` +
  `E10030` contract and is not part of the bit. Add one-report tests for the
  nullable cross-frame local and fresh shapes.
- [ ] 4.1.3 Reconcile the capture-move overlap: when a captured writable
  borrow is stored into an out-slot, the capture rule owns the report
  (`E10401`) and the out-slot rule withholds; pin the by-value and
  writable-borrow capture cases so no second `E10034` appears.
- [ ] 4.1.4 Run the cross-frame and captured suites; confirm exactly one
  `E10034` for the nullable shapes, no `E10030` duplicate, and the
  non-nullable array contract unchanged.

### Phase 5 — Verification, review, and close-out

- [ ] 5.1.1 Run the focused suites, `make check`, and `make verify` in the
  Docker SDK container.
- [ ] 5.1.2 Phase review: correctness reviewer plus the memory-safety
  auditor (ownership change); the emission-integrity auditor if the
  migration changes emitted C.
- [ ] 5.1.3 Re-emit determinism pins for the changed accepted units; confirm
  no byte-identity regression beyond the retargeted fixtures.
- [ ] 5.1.4 Close `DEF-541` in the `plans/00-roadmap.md` row `DEF-541`
  (Done) and update the residual row in
  `plans/deferred-evaluation-completion/05-findings.md` (line 61); file the
  generic non-nullable `copy<str>` residual (PF-010) as a new `DEF-` row in
  `plans/00-roadmap.md` with the probe evidence, using the next free id.

**Verify**: `make check` and `make verify` in the Docker SDK container; the
focused suites and `RDC_ASAN=1` controls green; one `E10034` per foreign
store and no duplicate `E10030`.
