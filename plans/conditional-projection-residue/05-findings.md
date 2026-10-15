# Findings Ledger: Conditional, Projection, and Update Residue

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Active — each phase review appends its verdicts, fix rounds, and filed rows here.

## Phase 1 — Arm roots and per-instance duty

**Reviewed fix**: `2c46774` (the red-cases baseline `744674b`). **Lenses**: correctness (RV) and
memory-safety (SA), dispatched on the phase diff in parallel.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | 1 blocker, 1 major, 2 minors (RV) + confirmation (CC) | CR-1 independently reproduced the same blocker as SA-001 (base clean vs head accepted-and-freed; disabling only the hoisted carrier drop makes the emission clean); CR-2 found the check-side per-instance duty landed for the take/return spellings while the lowering still copied there, so the new `E10020` rejection disagrees with the emission and the generic take/return use-after-free remains (base == head); CR-3 found the ST-4/ST-5 spec coverage omits the `g=false`, `\|\|`, fresh-arm-only, and loop-condition spellings; CR-4 is bookkeeping. The reviewer re-ran `make check` at `a0bc1c0` (green), the spec suite at the fix base (6/8 red), the pins at the fix base (4 red on exactly the fix markers), and 28 probes. **Targeted confirmation of the fix round**: the take/return lowering, gates, and pin integrity PASS; the new `E10020` now agrees with the emitted move; CC-1 (blocker) — the new deferred element branch fires for fixed-array (row) values and emits illegal C, breaking `residual_defect_hardening` (green at base); CC-2 (minor) — the eager field branch missed its duplicability/view/unknown gates. **Correction confirmation**: CC-1/CC-2 and the declared SA-101/SA-102 cases CONFIRMED (the row shapes strict-build and run exit 7; the suites green; the three new tests fail pre-correction for the intended reasons; no new acceptance/diagnostic changes), but the same helper's nested-literal spelling surfaced CCX-1 (the SA-104 mechanism one level down — eager take/store/return routes clean at the phase base, in-diff) and CCX-2 (the guarded spelling, base-equal); the `w: [b.v]` array path was clean on this reviewer's route while the memory lens reproduced the array-literal field element separately (SA-105) |
| memory-safety | 1 critical (in-diff), 2 majors and 1 minor (pre-existing) + confirmation (SA-101..103) | The four pinned shapes are fixed and several related spellings improve (the probe matrix: the eight pinned programs clean; ~25 variants clean, including the take-parameter receiver, nested store arms, the `while`/`\|\|` guarded takes, the growable-array types, and the generic instantiations); the statement-re-execution spellings exposed the critical. **Targeted confirmation of the fix round**: the four original repros and the sixteen-program set are clean and the SA-002 family is fixed; but the round's per-arm root binding introduced SA-101 — a materialized literal arm whose release double-frees a copied place sibling field (eager take, eager return, eager field, deferred element routes; clean at the pre-round tree); SA-102 is the same root cause in pre-existing guarded/deferred/store spellings; SA-103 a minor duplicable variant. **Final confirmation of the correction**: the direct-field literal-source moves verify clean (all SA-101/SA-102 repros and the full `m*`/`x*`/`y*` batch, with only the base-equal items remaining), and the borrow/loop/fresh-field hunts are clean; but the same helper's nested-literal residual surfaced SA-104 (four in-diff double-free routes) and SA-105 (the base-equal array-literal field element). **Final re-probe: CONFIRMED CLEAN** — all eight nested/array probes and nine new leaf probes (three-level literals, fresh-call leaves, duplicable leaves, array-of-struct, fill, and the generic instance) exit 0 with no sanitizer output on the corrected head, while every base run shows the fixed defect's class |

### Findings and rulings

| Finding | Severity | Area | Ruling |
|---------|----------|------|--------|
| SA-001 | 🔴 critical (in-diff) | `codegen/guards.rs` (`emit_conditional_drop_hoists`) | **Fixed and confirmed** (the correctness lens independently reproduced it as CR-1, including the isolated-cause check: disabling only the hoisted carrier drop makes the emission clean): the deferred-field route made an *adopting* consume carrier reachable inside a conditionally evaluated operand; the hoist destructures the `Materialize` without consulting `adopting` and emits a statement-form drop of the previous carrier value before the guarded construct, so the second execution of the statement frees storage the receiver's release already freed (`Array<T>`-typed fields; repros `e2b_loop_guarded_array_field`, `e2b_skip2`, `e3_arm_array_field_loop`, `e9_struct_field_container`; base leaked where head reads freed). The fix skips the hoist for an adopting carrier, matching the sibling sites in the same emitter |
| SA-002 | 🟠 major (pre-existing) | `own/lower.rs` (the take dispatch; per-level move-outs in `conditional_move_expr`) | **To file**: a nested conditional (or a type mentioning a parameter) inside a guarded `take` double-frees (`consume((c ? (d ? a : mkBox()) : b).v)`, `consume((c ? b : (d ? a : mkBox())).v)`, the both-nested spelling, and the generic `consumeGeneric<T>` spelling); base and head double-free alike; the eager spelling is clean |
| SA-003 | 🟠 major (pre-existing; the fresh arm's leak turns into a double free at head) | `own/lower.rs` (the aggregate-literal field path) | **Decision pending the correctness verdict**: `H { call: addOne, s: (c ? a : mkBox()).v }.call(5)` guarded and eager — the field copies the chosen member without emptying the source, so the arm root's release frees the same storage |
| SA-004 | 🟡 minor (pre-existing) | unconsumable positions | **To file**: a bare conditional of projections leaks the fresh root's residual in the remaining unconsumable positions (struct-literal field, array-literal element, eager `take` argument, `return`); the statement-store, indexed-store, and literal-arm spellings are clean at head |
| CR-2 | 🟠 major (in-diff rejection; pre-existing unsoundness) | `own/lower.rs` take dispatch, `own/statements.rs` return gate | **Fix round**: the phase-1 per-instance duty marks the move in the checker for the take/return spellings, but the lowering still copies there — the new shape `hold<T>((c ? a : b).v); let u: T = a.v;` is now rejected (`E10020`) while its accepted twins still read freed storage (base == head). Land the per-arm move-out lowering for the `take` and `return` positions (the AR-3 direction) so the rejection and the emission agree |
| CR-3 | 🟡 minor | `compiler/tests/conditional_projection_residue_spec_test.rs` | **Fix round**: extend the ST-4/ST-5 cases with the `g=false`, `\|\|`, fresh-arm-only, and loop-condition spellings (coverage only; the probes are green) |
| CR-4 | 🟡 minor | `99-execution-plan.md` | **Fixed at close**: the phase bookkeeping (`1.3.2`, the deliverables) closes with this review round's evidence |
| CC-1 | 🔴 blocker (in-diff regression) | `own/lower.rs` (the new deferred element branch :361-374) | **Fixed and confirmed** (the row shapes strict-build and run exit 7; the hardening suites green): the branch lacked the fixed-array exclusion its take sibling has, so it fires for row values and emits illegal C (`(__cl_tmp0 = (c ? rows[0] : rows[1]))`), breaking `residual_defect_hardening` (green at base). Mirror the `is_fixed_array` exclusion across every carrying branch added this round |
| CC-2 | 🟡 minor (in-diff) | `own/lower.rs` (the eager field branch :329-337) | **Fixed and confirmed** (the inline ternary restored): the branch missed its duplicability/view/unknown gates (a duplicable indexed field emitted a dead `if {} else {}`); add the gates the array sibling has |
| SA-101 | 🔴 critical (in-diff regression) | `own/lower.rs` (`hoist_ownerless_arm` / `materialize_value`) | **Fixed and confirmed** (the literal-source moves; the direct routes clean): the round's per-arm root binding materializes a literal arm into an owning temp and registers its release, but a sibling field that reads a place was kept as a copy, so the temp's release and the place's scope cleanup free the same storage. Four routes reach it (eager take, eager return, eager field, deferred array element; clean at the pre-round tree; repros `m6`, `m3`, `m8`, `m11` under `build/memory-audit/`). The place-derived owning fields must move (source zeroed) as part of the construction, or the temp's release must not cover aliased storage; add one spec case per route with a moved place sibling field |
| SA-102 | 🟠 major (pre-existing, same root cause) | the guarded/deferred spellings of SA-101 | **Fixed and confirmed** (the same mechanism covers the guarded/deferred/store spellings): the guarded take, guarded literal field, and member-store spellings of the same shape double-free at both the pre-round and head trees (`m2_lit_place_field_guarded_take`, `m10_guarded_field_place`, `m12_member_store_place`); fix with SA-101's mechanism if it covers them, else file |
| SA-103 | 🟡 minor (pre-existing) | take/return of a duplicable projection over a residual-owning fresh root | **Filed as `DEF-281`** (Backlog): `holdNum(c ? a.n : mkBox().n)` and its return twin leak 11 B per fresh `mkBox()` at base and head (the literal-field twin is clean at head); the carrying gates skip the duplicable instance, so the fresh root's residual never releases. The fix binds the fresh root for its residual release while the selected value stays a copy |
| SA-104 | 🔴 critical (in-diff residual of the SA-101 fix) | `own/lower.rs` (`consume_literal_sources` :3290-3301; `value_place` :1917-1961) | **Fixed and confirmed** (the nested recursion; the final re-probe clean): the helper maps direct fields only; a nested `StructLit` field's place-derived owning value stays a copy (`consume_place_into` bails at :1611-1613 for a missing `StructLit` arm), so the materialized literal and the source's cleanup free the same storage. Four in-diff routes (`n2` return, `n3` eager field, `n5` eager take, `n6` deferred element; base leaked, head double-frees) plus three base-equal (`n1`, `n4`, `n7`). Recurse into `StructLit` fields and `ArrayLit` elements so the move reaches the place leaves. The correctness lens independently reproduced it as CCX-1 (the same mechanism one level down; the eager take/store/return routes are clean at the phase base — in-diff) and its base-equal guarded spelling as CCX-2 |
| SA-105 | 🔴 critical (pre-existing, base-equal) | the array-literal field element of the same shape | **Fixed and confirmed** (same fix point): `OutA { v: …, w: [b.w, stringOf("q")] }.v` double-frees at base and head via `{ b.w, … }` copied unmoved; the same recursion decides it |

### Pre-existing findings (fixed in the round or filed, not folded into the phase's verdict)

| Row | Severity | Shape | Outcome |
|-----|----------|-------|---------|
| SA-002 | 🟠 major | the nested/generic guarded `take` double-frees | Fixed in the phase-1 fix round (the per-level move-outs; the nested spellings included) |
| SA-003 | 🟠 major | the conditional projection in an aggregate-literal field | Fixed in the phase-1 fix round (the eager and deferred literal-field routes) |
| SA-004 | 🟡 minor | the bare-conditional leaks in unconsumable positions | Fixed in the phase-1 fix round (the eager take/element/return and the deferred spellings) |
| DEF-281 | 🟡 minor | a duplicable projection's residual leak in the take/return positions | Filed (Backlog; base == head) |

### Coverage notes

- The phase's loop cases re-enter the *function*; no pinned case re-executes the guarded *statement*
  with the same carrier temp, and the pinned field types (`String`) have expression-form drops, so
  the hoist path was never reached — the gap that hid SA-001. The fix round adds a re-execution
  case with a statement-form-drop field (`Array<String>`).
- Container `RDC_ASAN=1` focused suites: `conditional_projection_integrity` 61/61,
  `runtime_value_integrity` 30/30, `eval_emission_integrity` 45/45; the 74 embedded programs of the
  two slow suites extracted and run individually on host AddressSanitizer: 51 runnable, 0 memory
  failures, 18 compile-fail cases — identical for the pre-fix compiler.
- Trap/panic paths were not probed: the process aborts before any cleanup, and in the changed
  shapes every release precedes the same-expression trap.
- The correction round's probe batch surfaced two synthetic fixed-array field shapes
  (`Holder { row: c ? a.row : mkBox().row }` guarded; `Holder { row: (c ? a : b).row }`) whose
  emitted C fails the strict build at both the pre-round and head trees — candidate spellings for
  the phase-5 fixed-array scope; pin them there if they belong to its rows, else file.
- Phase-4 spec authoring added two more phase-5 candidates to the same note: the guarded fixed-array
  field expression (`g && len(Holder { names: [mk().v] }.names[0]) > 0` fails `-Werror=missing-field-initializers`
  / `missing-braces` at head and base) and the conditional carrier literal's taken-arm leak (now filed as
  `DEF-292`).
- The phase-4 correctness review added the discarded all-fresh fixed-array literal spelling
  (`c ? [stringOf("x")] : [stringOf("y")];` accepted with invalid C at both refs — PE-3, the `DEF-272` family)
  to the phase-5 candidates.
- SA-101/SA-102's correction (the literal-source moves and the fixed-array exclusions) landed
  uncommitted with the round; the final targeted confirmation is in progress.

## Phase 2 — View derivation and owners

**Reviewed implementation**: `14aea8a` (fix-tree base `c148fc5`), pins `f15cbd0`; the review-round
fixes on top (uncommitted). **Lenses**: correctness (P2R), semantics (P2S), memory-safety (M),
dispatched in parallel.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | 2 blockers (P2R-1 critical = the carrier escape below; P2R-2 major = the deferred contract), 1 minor (P2R-3 baseline hash) | The blast-radius table shows the `call_view_source` refactor identity-preserving and the gates verified; test-integrity green (the pinned reds red at `f8b04f4`); `make check` green |
| semantics | 2 blockers (P2S-2 = the carrier escape; P2S-1 = the conditional store's eager element), 2 minors (P2S-3 aggregate argument; P2S-4 the reborrow report code), 4 pre-existing records (PE-1..PE-4) | The five questions answered with spec citations; the one-report relationship defensible (`spec/14:18-20`) with the right control boundary and no acceptance hole; PE-2..PE-4 noted below |
| memory-safety | 2 criticals (M-1 = the carrier escape; M-2 = the inner-block store), 1 pre-existing minor, 1 coverage gap | 49 head / 33 base probes; the coverage gap: the cases read only the view's length, so a silent UAF hid — the fix-round cases read the backing's bytes |

### Fix round (working tree)

| Finding | Severity | Ruling |
|---------|----------|--------|
| M-1/P2R-1/P2S-2 | 🔴 | **Fixed**: a flow-sensitive carried-owner set (`own/flow.rs`, joins/pruning in `own.rs`, accessors in `own/place.rs`) with the `carried_view_owners` walk at a return — one `E10030` for a generated/temp backing, one `E10035` for a local-only view; the direct-literal and lambda spellings become rejections; in-frame uses stay accepted |
| M-2 | 🔴 | **Fixed**: `adopt_view_backings` moves each generated owner to the function body scope, so a destination in an enclosing block keeps its backing |
| P2R-2/P2S-1 | 🟠 | **Fixed**: the field adoption gated on `!deferred`; the argument adoption on `defer_move_outs == 0`; a guarded view-element array argument defers its fill |
| P2S-3 | 🟡 | **Fixed**: the aggregate argument adoption (through the carrier change and the body-scope owner) |
| P2S-4 | 🟡 | **Fixed**: `view_is_mixed` looks through a view-returning call's single source (one `E10030` like the direct spelling) |
| PE-1 | pre-existing 🟠 | **Fixed by the round** for the skipped path; the taken path is the C2 correction below |
| PE-3/PE-4 | pre-existing 🟡 | **Filed**: `DEF-282` (the two-message dedupe), `DEF-283` (the deferred spelling's phrase) |
| P-2/CF-3 (p13) | pre-existing 🟡 | Recorded for phase 4: the conditional carrier field's taken path leaks |
| PE-2 (p6) | pre-existing 🟡 | Recorded for phase 4/5: the guarded fixed-array field spelling fails the strict build |

### Confirmation of the fix round

| Lens | Result | Detail |
|------|--------|--------|
| semantics | PASS WITH MINORS — 0 blockers | The six confirmations hold (the probes, the suite, no over-rejection, the corpus sweeps); CF-1 (the guarded struct-literal argument still runs the skipped call), CF-2 (the deferred guarded array argument leaks on the taken path), CF-3 (p13, phase 4), CF-4 (coverage) |
| memory-safety | FINDINGS — 4 | F-1 (critical, in-scope: the view-like `Call` branch misses a `StructLit` descent, so an escaped adopted backing reads freed); P-1 (= CF-2); P-2, P-3 |

### Correction round (fixed)

| Item | Severity | Outcome |
|------|----------|---------|
| C1/F-1 | 🔴 | **Fixed**: the carrier walk descends the view call's lend-source argument (the lowered collectors stop at a struct literal, so the walk reaches the adopted owner the field holds); `n7`/`x1uaf` reject with one `E10030`; the control `n4` unchanged; a spec case added |
| C2/P-1/CF-2 | 🟠 | **Fixed**: `adopt_guarded_view_backings` materializes the deferred argument's view-element owners inside the guard; both paths run one call per evaluation, values correct, ASan-clean; taken-path assertions added |
| C3/CF-1 | 🟠 | **Fixed**: the guard deferral accepts view-carrying aggregate arguments and the guards emitter fills struct-literal fields field-by-field; skipped calls 0, taken 1; both-path assertions added |
| C4/CF-4 | 🟡 | **Fixed**: `view_is_mixed` descends array literals; the chained and array spellings report one `E10030` like their single-view twins; a spec case added |
| E-1 | 🟠 (in-diff, found by the final memory confirmation) | **Fixed**: `array_assign_expr` recurses through `struct_assign_expr` for a struct element, so a nested guarded aggregate stores field by field (the strict build passes and the stored view keeps its length); a spec case added; the `unused variable` warning cleaned |
| M-1 | 🟡 pre-existing | **Filed as `DEF-284`**: a guarded element wrapping a fresh call root leaks its residual and runs the skipped call (base-equal); its fix stays out of the round |

### Final confirmation

The memory lens confirmed the F-1/P-1/CF-1 corrections (no head UAF/DF/leak where the
base was clean; the taken/skipped call counts and values correct; the deeper F-1 chains reject) and
found E-1 in its hunt (fixed above). The phase's closing verification: the residue spec 36/36 plain
and under `RDC_ASAN=1`; the impl pins 22/22; the batch-H oracles, `callable_release_integrity`, and
`make check` (66 suites) green with no warnings; the 236-probe corpus differential shows every
delta as leak/UAF → rejection or clean, or eager → lazy.

## Phase 3 — Escaping stores

**Reviewed implementation**: the working tree over `acee2cb` (later extended by the review round's fixes and the follow-up below); pins written mid-round. **Lenses**: correctness (P3R), semantics (P3S), memory-safety (F/M/O), emission-integrity, dispatched in parallel.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | 1 blocker (P3R-1 the merged-provenance over-rejection), P3R-2 (take destinations), P3R-3 (extra reports), P3R-4 (dead branches), P3R-5 (dot-receiver asymmetry) | The blast-radius table; test integrity (eight of nine new spec cases and three of six new pins red at `acee2cb`, the rest proven by mutation trees); the corpus differential; `make check` green |
| semantics | 1 blocker (P3S-1 the take-owned caller slot), P3S-2 (the merged-provenance class), P3S-3 (the `for` alias), P3S-4 (literal parity), P3S-5 (view-mark moves), PE-1..PE-4 | The frozen-text citations (EL-4/OW-14 for module storage; BR-1 for the take rejection); the six confirmations |
| memory-safety | **CONFIRMED CLEAN — 0 in-diff**; pre-existing F1 (the cross-frame out-slot, = semantics PE-1), F2 (the runtime-index element store, = correctness PX-1), F3 (the `for`-alias UAF, fixed in the round); over-rejections O1/O2 | The exhaustive base/head acceptance diff over 74 probes; every newly accepted shape strict-built and ASan/LSan clean |
| emission-integrity | **CONFIRMED — 0** (7 intended deltas, no head-only strict failure, no invalid C, determinism holds, both cross-profiles build); its F1 = the over-rejection class | 843-program differential; 408 accepted strict builds; the declaration and dead-instance checks |

### Fix round

| Finding | Severity | Ruling |
|---------|----------|--------|
| P3R-1/P3S-2/O1/emission-F1 | 🟠 | **Fixed**: `view_provenance` returns per-component storage atoms (`Composite`, `storage_atoms`; a `null` arm skipped; a `static str` call → `Static`) and the store judgment accepts when every component fits the destination's set, reporting once naming the first failing component |
| P3S-1/P3R-2 | 🔴/🟡 | **Fixed**: a take-owned view at a real caller slot reports one `E10030` naming the parameter; the caller-slot classification is `OutSlot`/borrow-parameter/env field, so a take parameter is a frame destination |
| P3S-3/F3 | 🟠 | **Fixed**: the `for (borrow e in xs)` alias resolves its iterable container (`alias_root`, `resolve_alias_container`) for the judgment and the container loans; the alias UAF probes reject |
| P3S-4 | 🟡 | **Fixed**: the literal-field checks use the per-component destination treatment (parity with the assignment) |
| P3S-5/O2/P3R-3 | 🟡 | **Fixed**: view stores mark nothing as moved; the element loan is replaced on overwrite while the others keep; an already-rejected write keeps one report |
| P3R-4/P3R-5 | 🟡 | **Fixed**: the dead branches removed (and `is_loan_value` retired); the qualified-head dot receiver resolves the module binding |
| Extra | — | **Fixed**: the two-step carrier return records the carrier binding's loans (one `E10030`; the static/param twins accepted) |

### Confirmations

| Lens | Result | Detail |
|------|--------|--------|
| correctness | PASS WITH MINORS — 0 blockers | PC-1 (the parameter-backed carrier literals, fixed in the follow-up), PC-2..PC-5 (filed/extended); five confirmations pass; no base-clean program defective |
| semantics | PASS WITH MINORS — 0 blockers | SC-1 (= PC-1), SC-2 (filed as `DEF-288`), SC-3 (a sound consequence); the six confirmations pass |
| memory-safety | CONFIRMED CLEAN — 0 in-diff | The alias class closed; F1/F2 unchanged and filed |
| emission-integrity | FINDINGS — 1 | The re-accepted cross-frame spelling resolves to `DEF-286` (base == head, byte-identical C); no other emission work |

### Post-confirmation fix (PC-1/SC-1)

A `LiteralViews { Frame, Return, Escaping }` destination treatment: a non-`take` parameter-backed view field in a returned carrier literal is accepted
(the caller's frame outlives the return) while the loan stays for module destinations; the take and frame-local twins still reject. The two literal twins accept with
a caller read; the global twins keep one `E10033`; the take twin keeps one `E10030`. The spec suite is at 55 tests.

### Filed rows

`DEF-285` (the runtime-index element store), `DEF-286` (the cross-frame out-slot pass-through, extended with the fresh-argument spellings), `DEF-287` (the module-global
fresh-init leak), `DEF-288` (a container escape with frame-backed element loans), `DEF-289` (the direct-return parameter-backed literal — fixed by the follow-up), `DEF-290`
(a stale literal-carrier loan), `DEF-291` (an all-null composite); `DEF-282` extended with the PC-4/PC-5 diagnostic-count instances; the pre-existing emission notes map to
`DEF-171` (the `E10213`/embed case) and `DEF-189` (the freestanding stack alignment); the fixed-array-in-literal invalid C stays recorded for phase 4/5.

## Phase 4 — Guarded and discarded literal positions

**Reviewed implementation**: the working tree over `f853ef9` plus the review-driven correction rounds. **Lenses**: correctness (P4R/P4C) and memory-safety (F/MEM-1), dispatched in parallel on the phase diff.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | 1 blocker (P4R-1 the discarded aggregate's fresh leaves unmaterialized) + P4R-2 (the discarded fixed-array literal arm accepted into invalid C), PE-1..PE-4 pre-existing | The blast-radius table (the walk's wholesale rule, the flag lifecycle, the discard materialization, the projection-root binder); test integrity (the four cases red at `f853ef9`); the corpus differential (1763 embedded programs, 8 intended deltas; 138 repo files, 0 deltas); `make check` green |
| memory-safety | FINDINGS — 1 in-diff (F-1 = P4R-1) + a pre-existing emitter note | 49 probes plus variants: the fixed shapes clean, no double-free/UAF introduced, the do-not-fix set unchanged |

### Fix rounds

| Item | Severity | Outcome |
|------|----------|---------|
| P4R-1/F-1 | 🟠 | **Fixed**: `materialize_discarded_field` recurses into nested literal fields and array elements, materializing each fresh owning leaf into its own released statement temporary (the fixed-array leaf element-wise); spec cases `discardnested`/`discardfixed` |
| P4R-2 | 🟠 | **Fixed**: the discarded-conditional element read skip is gated on the new `discarded_literal_field` flag, so a discarded array-literal arm rejects like its direct twin; spec case `discardarray` |
| MEM-1 | 🔴 (base-equal; the phase rule widened its reach) | **Fixed**: the discarded literal field's conditional projection stays a read (redirected to the root binder with `discard_conditional_depth > 0`); closes `DEF-296`; spec cases `discardprojfield`/`discardprojmixed` |

### Confirmations

| Lens | Result | Detail |
|------|--------|--------|
| correctness | PASS — 0 blockers | The corrections verified across 87 probes and 1904 corpus/spec programs; the three observations (P4C-1..3) informational/pre-existing |
| memory-safety | CONFIRMED CLEAN | The corrections plus the MEM-1 fix (37-probe set + an 11-probe hunt: no leak/UAF/DF; the base-equal records unchanged, C byte-identical) |

### Filed rows

`DEF-294` (a loop-condition receiver with mixed arms emits sequence-point C), `DEF-295` (a loop-condition member projection double-frees), `DEF-296` (the discarded-field projection — fixed and closed with the phase), `DEF-297` (a bare discarded all-fresh literal statement leaks); the discarded all-fresh fixed-array literal spelling (PE-3) joined the phase-5 candidates note; PE-1/PE-2 recorded above with base evidence.
