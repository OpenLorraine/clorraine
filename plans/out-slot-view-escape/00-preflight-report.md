# Preflight Report: out-slot-view-escape

> **Status**: FINAL — 10 iteration-1 findings and 12 follow-on root causes resolved; plan rewritten and re-scanned
> **Verdict**: ✅ PASSED WITH NOTES — two accepted precision losses recorded in the plan
> **Iteration**: 3 (initial scan plus two fix rounds)
> **Artifact**: Implementation plan (multi-phase task) at `plans/out-slot-view-escape/99-execution-plan.md`
> **Codebase Grounded**: ~20 source files, 6 test files, 7 specification/decision documents examined; 30+ references verified; 8 compiler probes run (2 under AddressSanitizer)
> **Last Updated**: 2026-10-22
> **Scope**: Strict. Audit target is the plan only. Context documents: the frozen spec chapters, the compiler ownership/codegen modules, the affected test suites, `plans/caller-view-escape`, `plans/deferred-evaluation-completion`, `plans/conditional-projection-residue/00-ambiguity-register.md`, and `plans/00-roadmap.md`.
> **Domain lens**: Compiler and language (ownership/lifetime closure, diagnostics, IR invariants, conformance tests).
> **Review basis**: Two independent audit dispatches (document soundness; codebase grounding and risk) plus one independent challenger against the critical/major batch. The challenger corrected two reviewer premises: a by-value owning aggregate's field view is still caller-backed and must not be kept on `E10030`, and the accepted `copy<String?>` pin is an owned move, not a view store.

## Codebase Context Summary

**Tech stack**: Rust workspace, std-only; C99 emitted translation unit; a frozen specification under `spec/`; Docker-pinned toolchain.

**Architecture**: The ownership checker (`compiler/src/own/`) has three relevant paths: the concrete expression/store walk (`expr.rs`), the flow-sensitive move/drop state (`statements.rs`, `place.rs`), and the per-instance audits that judge generic bodies after monomorphization (`instance_check.rs`, `ir/mono.rs`). A separate caller-side call check (`call_check.rs`) judges cross-frame view pass-through at the call site.

**Key observations**:

- `check_escaping_view_store` (`compiler/src/own/expr.rs:2414`) is the concrete destination walk. It withholds every destination report when `callee_elides_no_single_parameter(value)` is true (`expr.rs:2430`), because the callee's own view-return judgment is supposed to carry the rejection.
- `dynamic_parameter_roots` (`expr.rs:2357-2375`) returns every binding with `BindingKind::Param` and `mode != Mode::Take` (unmarked read-only, `const`, `borrow`, captures) plus every `OutSlot`.
- `spec/03-ownership.md:53` defines an unmarked `x: T` parameter as a read-only lend. The plan's DEF-541 repro `fill(p: str, borrow slot: str?)` is therefore a read-only-lend store, not a `borrow`-mode store.
- A whole-binding store (no projections) never records a `RequiresOpaqueStore`: `record_opaque_store` returns early on `projections.is_empty()` (`expr.rs:1865`). The whole-slot store is recorded as `RequiresNotLend` (`place.rs:600-660`, `ir/mono.rs:172`) and `RequiresStorableSlot` (`expr.rs:3047`, `ir/mono.rs:254`). Neither carries a source root or a destination root.
- The caller-side `check_cross_frame_pass_through` (`call_check.rs:950-1080`) independently reports one `E10030` at a call whose view argument dies with the caller's frame and whose out-slot lives further out.
- 188 program strings across four test files define `func fill(p: str, borrow slot: str?) { slot = p; }`: 103 + 12 in the two `conditional_projection_residue_*` suites, 69 + 4 in the two `eval_emission_integrity_*` suites.

**Reference verification**: 30+ mapped (all core functions, requirements, diagnostics, pins). Unverified: the exact two-report count after the new callee check (code analysis only, not patched); capture-id normalization for a captured same-slot copy.

## Summary by Dimension

| # | Dimension | Findings | Highest severity |
|---|---|---|---|
| 1 | Ambiguities | 2 (PF-001, PF-008) | 🔴 |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 2 (PF-005, PF-006) | 🟠 |
| 4 | Completeness Gaps | 1 (PF-002) | 🔴 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 1 (PF-004) | 🟠 |
| 7 | Testability | 0 | — |
| 8 | Security Blind Spots | 0 (the UAF is reported under PF-002) | — |
| 9 | Edge Cases | 1 (PF-009) | 🟡 |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 1 (PF-007) | 🟠 |
| 12 | Consistency | 0 | — |
| 13 | Codebase Alignment | 2 (PF-003, PF-010) | 🟠 |

## Summary by Severity

| Severity | Count | Status |
|---|---|---|
| 🔴 CRITICAL | 2 | 2 resolved in the rewrite |
| 🟠 MAJOR | 5 | 5 resolved in the rewrite |
| 🟡 MINOR | 3 | 3 resolved in the rewrite |
| 🔵 OBSERVATION | 0 | — |

## Decision log

| Finding | User ruling | Date |
|---|---|---|
| PF-001 | Resolved — User accepted recommendation: Option A (source-kind/storage matrix) | 2026-10-22 |
| PF-002 | Resolved — User accepted recommendation: Option A (judge before the view-return withholding) | 2026-10-22 |
| PF-003 | Resolved — User accepted recommendation: Option A (dedicated whole-slot duty + generic cases) | 2026-10-22 |
| PF-004 | Resolved — User accepted recommendation: Option A (188-program inventory and migration) | 2026-10-22 |
| PF-005 | Resolved — User accepted recommendation: Option A (callee owns the report with a one-bit flag) | 2026-10-22 |
| PF-006 | Resolved — User accepted recommendation: Option A (spec amendment + ruling) | 2026-10-22 |
| PF-007 | Resolved — User accepted recommendation: Option A (record the ruling, enumerate pins, red-first) | 2026-10-22 |
| PF-008 | Resolved — User accepted recommendation: Option A (reword the self-assignment row) | 2026-10-22 |
| PF-009 | Resolved — User accepted recommendation: Option A (classify cross-slot and captured copies) | 2026-10-22 |
| PF-010 | Resolved — User accepted recommendation: Option A (correct the claim, file a residual) | 2026-10-22 |

---

## PF-001: The source-kind predicate is undefined and the wrong choice either leaves DEF-541 open or breaks existing pins 🔴 CRITICAL

**Dimension**: 1 — Ambiguities (also 2, 13)
**Location**: `plans/out-slot-view-escape/99-execution-plan.md:11-22` (Objective, Smallest viable design), `:47-55` (Boundary), `:62-67` (T-07.2, T-07.3)

**Codebase Evidence**:
- `spec/03-ownership.md:53` — unmarked `x: T` is a read-only lend, so the DEF-541 repro `fill(p: str, borrow slot: str?)` is a read-only-lend store.
- `compiler/src/own/expr.rs:2357-2375` — `dynamic_parameter_roots` includes every non-`take` parameter (unmarked, `const`, `borrow`, captures) and every out-slot.
- `compiler/tests/borrow_store_integrity_impl_test.rs:684-711` and `borrow_store_integrity_spec_test.rs:1881-1935` — `pmixed`/`mixedcarrier`: a by-value `H` parameter's owned `name` field stored into a `str?` out-slot currently reports one `E10030` at the call; the static-`tag` control (`mixedcarrierok`) is accepted.
- `compiler/tests/conditional_projection_residue_spec_test.rs:6417-6460`, `eval_emission_integrity_spec_test.rs:5078-5111` — the parameter-backed chain, the same-frame call, and the captured `slot = p` controls are accepted today.
- 188 `fill(p: str, borrow slot: str?)` fixture definitions across the four test files.

**The Problem**: The plan never states which source bindings are "foreign lends". Three readings are possible, and each behaves differently:

1. `borrow`/`const` only — leaves the plan's own DEF-541 repro (`p: str`) accepted.
2. Every non-`take` parameter, by root identity — catches the repro, but also reclassifies `pmixed` (E10030 → E10034), flips the static-field control `mixedcarrierok` (the root is `Param(h)`, even though `h.tag` is `static str`), and rejects all 188 fixtures.
3. A storage-aware predicate — rejects a view of a read-only/`const`/`borrow` lend at every projection, accepts a `static` position even inside a parameter, and keeps local/fresh frame-backed sources on `E10030`.

The plan's wording mixes 1 ("lend parameter", BR-5) and 2 ("parameter root other than the destination root") and never mentions the static-field or projection cases. The `pmixed` and `mixedcarrierok` pins are concrete evidence that the wrong reading changes behavior beyond the defect.

**Options**:

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | State and implement a source-kind + storage matrix: reject a view-like or view-converted store whose storage derives from a non-`take` lend at any projection; accept a static/module-backed position (including a `static str` field); treat another out-slot as foreign; keep local/fresh/temp on `E10030`; conditional rejects once if any arm is foreign; captured slot uses the same rule | Closes the repro; preserves static acceptance; pins the matrix | The 188 fixtures still flip (PF-004); needs the storage-position check |
| B | Restrict the rule to `borrow`/`const` sources | Smallest diff | Leaves `fill(p: str, ...)` accepted — the defect stays open |
| C | Reject every non-`take` parameter root | Simple | Flips `mixedcarrierok` (safe static field), reclassifies `pmixed`, and adds no precision |

**Recommendation**: Option A — it is the only reading that closes the repro without over-rejecting static storage. The plan must write the matrix explicitly, including the `static str` field carve-out, and add a `h.name` (E10034) plus `h.tag` (accepted) control.

**User Decision**: Resolved — User accepted recommendation: Option A (2026-10-22)

---

## PF-002: A view-returning call is withheld from the destination judgment, leaving a demonstrated heap-use-after-free 🔴 CRITICAL

**Dimension**: 4 — Completeness Gaps (also 8, 9, 13)
**Location**: `plans/out-slot-view-escape/99-execution-plan.md:16-22` (Smallest viable design), `:62-64` (T-07.2), `:59-61` (T-07.1 cases)

**Codebase Evidence**:
- `compiler/src/own/expr.rs:2426-2434` — `check_escaping_view_store` returns early when `self.callee_elides_no_single_parameter(value)` is true. The comment says the callee's own view-return judgment carries the rejection.
- `compiler/src/own/expr.rs:2539-2585` — the withholding is applied to every named call that elides exactly one view-like lend parameter, including a view-returning call such as `id(p)`.
- Probe (`/tmp/opencode/viewcall.clo`): `func id(p: str): str { return p; }`; `func fill(p: str, borrow slot: str?) { slot = id(p); }`; caller reassigns `b.v` after the call. The compiler accepts the program; the ASan run reports `heap-use-after-free` on `out[0]`.

**The Problem**: The plan's central design is "reuse the existing `check_escaping_view_store` provenance walk". That walk deliberately withholds the destination report for a view-returning call, because the callee's return judgment is supposed to cover it. The callee's return judgment only checks that the view derives from exactly one lend parameter; it knows nothing about the out-slot destination. So `slot = id(p)` is never judged at the out-slot and the exact DEF-541 class remains open. The plan's T-07.1 cases do not include a view-returning call.

**Options**:

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Run the whole-out-slot foreign-lend judgment before the `callee_elides_no_single_parameter` withholding; keep the withholding for the E10030/E10033 destination consequences only; add a spec case and an ASan run | Closes the hole with the same provenance data; keeps one report for the existing destination cases | Requires separating the new judgment from the existing withholding |
| B | Keep the withholding and declare view-returning calls out of scope | Smallest diff | The demonstrated UAF remains; the plan cannot claim to close DEF-541 |
| C | Add a second call-expression provenance resolution for the out-slot destination | Explicit | Duplicates the walk; larger than A |

**Recommendation**: Option A — the new E10034 is a destination judgment, not the callee's return judgment; the withholding was never meant to cover it. Add `slot = id(p)` to T-07.1 and run it under `RDC_ASAN=1`.

**User Decision**: Resolved — User accepted recommendation: Option A (2026-10-22)

---

## PF-003: T-07.3 targets an audit that never sees a whole-slot store; the duty it names has no source or destination identity 🟠 MAJOR

**Dimension**: 13 — Codebase Alignment (also 5, 6, 7)
**Location**: `plans/out-slot-view-escape/99-execution-plan.md:65-67` (T-07.3)

**Codebase Evidence**:
- `compiler/src/own/expr.rs:1865` — `record_opaque_store` returns `false` when `projections.is_empty()`. A whole-binding store into `slot` records no `RequiresOpaqueStore`.
- `compiler/src/ir/mono.rs:401` — `whole_out_slot` exists only on `RequiresOpaqueStore`; because of the early return, the `whole_out_slot` path at `instance_check.rs:499` is unreachable for a whole-slot store.
- `compiler/src/own/place.rs:600-660` — the whole-slot store is recorded as `RequiresNotLend` (source move) with fields `function`, `parameter`, `ty`, `projections`, `writable_source`, `stores_into_slot`, `span`.
- `compiler/src/ir/mono.rs:254-263`; `compiler/src/own/expr.rs:3047` — `RequiresStorableSlot` carries only the destination parameter name and type.
- `compiler/src/own/instance_check.rs:211-287, 289-327` — the two audits that actually judge the whole-slot store; neither knows the source root or the destination root.
- Probe: `func copy<T>(borrow v: T, borrow slot: T) { slot = v; }` with `copy<str?>` is accepted today and has no test.

**The Problem**: `parameter_backed_sources` and `source_arms` are fields of `RequiresOpaqueStore`. A whole-slot store never creates that requirement, so T-07.3 cannot be implemented as written. The requirement that does record the generic whole-slot store is `RequiresNotLend`/`RequiresStorableSlot`, which has no destination identity and no source list. The generic `copy<str?>`/`copy<String?>` path therefore remains accepted unless a new duty (or an extended one) is introduced. T-07.1 also has no generic case, so the per-instance deliverable has no spec test.

**Options**:

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Add a dedicated whole-out-slot duty carrying the source root/mode, destination root, declared types, and span; judge it per instance; remove or use the dead `whole_out_slot` field; add `copy<str?>` and `copy<String?>` cases | Correct, explicit, testable | New small IR requirement and audit |
| B | Extend `RequiresNotLend` with the destination identity and judge there | Reuses an existing duty | Conflates a source-move duty with a destination judgment; risks disturbing the existing E10034 move rules |
| C | Remove the `projections.is_empty()` early return and reuse the opaque-store fields | Reuses existing fields | Changes every whole-slot store's duty; may duplicate the store/move audits |

**Recommendation**: Option A — it is the smallest shape that keeps the two judgments separate and gives the generic path a test. T-07.3 should be rewritten to name the new/extended duty, not `parameter_backed_sources`/`source_arms`.

**User Decision**: Resolved — User accepted recommendation: Option A (2026-10-22)

---

## PF-004: The 188-program fixture blast radius is not analyzed; T-07.4 understates the migration 🟠 MAJOR

**Dimension**: 6 — Feasibility Concerns (also 13)
**Location**: `plans/out-slot-view-escape/99-execution-plan.md:68-69` (T-07.4)

**Codebase Evidence**:
- 188 definitions of `func fill(p: str, borrow slot: str?) { slot = p; }`: 103 in `conditional_projection_residue_spec_test.rs`, 69 in `conditional_projection_residue_impl_test.rs`, 12 in `eval_emission_integrity_spec_test.rs`, 4 in `eval_emission_integrity_impl_test.rs`.
- The callee body is unconditionally foreign for every one of them; the caller's argument cannot make `slot = p` legal once the callee-side rule exists.
- The affected pins include acceptance controls (`CROSS_FRAME_PARAM_SOURCE`, `CROSS_FRAME_SAME_SOURCE`, `OWNER_PASS_*`, `captureoutslotviewok`, `captureoutslotviewlocal`), rejection controls (`CROSS_FRAME_LOCAL_SOURCE` expecting one `E10030` at the call), and unrelated feature fixtures that use the helper only to place a view in an out-slot.

**The Problem**: T-07.4 says only "retarget the cross-frame and out-slot acceptance pins". The actual surface is 188 program strings, including rejection controls whose expected code/count changes and unrelated tests whose fixture becomes illegal. The plan's "lightweight" framing and 5-task progress do not account for this migration. A naive sed to `E10034` would also break the tests' purpose (some need a static or `take` source to stay accepted).

**Options**:

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Add an inventory/classification task: unsafe → retarget to rejection; safe-but-unprovable → decision; unrelated fixture → rewrite to a legal static/`take` source; re-run the ASan controls | Makes the plan executable; preserves test intent | Larger task list; the "lightweight" label is no longer accurate |
| B | Narrow the rule to avoid the fixtures | No migration | Reopens DEF-541 |
| C | Implement the judgment at the call site to avoid changing the callee fixture | Fewer callee edits | Does not catch the same-frame reassignment UAF and needs a call-side relation; still changes callers |

**Recommendation**: Option A — the rule is callee-side by design; the fixture migration is unavoidable. Add it to T-07.4 and update the plan's effort/progress framing.

**User Decision**: Resolved — User accepted recommendation: Option A (2026-10-22)

---

## PF-005: The caller-side cross-frame check and the new callee-side check can report the same store twice 🟠 MAJOR

**Dimension**: 3 — Logical Contradictions (also 7, 13)
**Location**: `plans/out-slot-view-escape/99-execution-plan.md:16-22` (one `E10034` at the store), `:59-61` (T-07.1 one-report expectation)

**Codebase Evidence**:
- `compiler/src/own/call_check.rs:950-1080` — `check_cross_frame_pass_through` reports one `E10030` at a call whose view argument dies with the caller's frame and whose out-slot lives further out.
- `compiler/src/own/expr.rs:2414-2511` — the new callee-side judgment would report `E10034` at the same `slot = ...` store.
- `compiler/tests/conditional_projection_residue_spec_test.rs:6417-6470` — `CROSS_FRAME_LOCAL_SOURCE` expects exactly one `E10030` at the call; the parameter-backed chain and same-frame twins expect exit 0.
- `compiler/tests/borrow_store_integrity_spec_test.rs:1881-1905` — `mixedcarrier` expects exactly one `E10030`; under a non-`take`-parameter rule the callee body adds `E10034`.

**The Problem**: The two checks are independent: the callee body check runs at the definition and fires even with zero calls; the caller-side check runs per call and fires on the frame relation. Nothing suppresses one for the same store. The plan promises "one `E10034` at the store" but does not say how the existing `E10030` is withheld or how exact-count pins are retargeted. Two unsafe calls would produce one callee report plus two caller reports, so even "one per program" is not stable.

**Options**:

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | The callee owns the report; add a one-bit per-callee "whole-out-slot foreign store rejected" flag so the caller-side check withholds its `E10030` for that callee | Keeps one report; the flag is an effect bit, not a full summary | Small cross-function signal; the plan's "no summaries" statement needs a precise exception |
| B | The caller owns the report; the callee body stays accepted | No cross-function signal | Misses the direct same-frame case and the zero-call definition; does not close DEF-541 as stated |
| C | Accept two reports and retarget every count pin | Honest; no new signal | Violates the one-report convention and the plan's stated deliverable |

**Recommendation**: Option A — a one-bit callee effect is the smallest signal that preserves the one-report contract. If that is judged a cross-function summary, choose C explicitly and retarget the counts; do not leave the duplicate implicit.

**User Decision**: Resolved — User accepted recommendation: Option A (2026-10-22)

---

## PF-006: The new `E10034` at an out-slot write contradicts the frozen spec's exemption text; no spec-amendment task exists 🟠 MAJOR

**Dimension**: 3 — Logical Contradictions (also 12, 13)
**Location**: `plans/out-slot-view-escape/99-execution-plan.md:31-38` (Decision), `:59-71` (Tasks)

**Codebase Evidence**:
- `spec/03-ownership.md:216` — "so E10034 does not apply to the slot".
- `spec/16-borrow-rules.md:107` — "E10034 does not apply to out-parameter writes".
- `spec/14-diagnostics.md:62` — the E10034 help text: "declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt".
- `spec/12-ffi-contracts.md:147` — CN-16 exempts "the value in a `borrow T?` out-parameter slot".
- `spec/decisions.md:330` — PF-001: "`borrow T?` out-parameters are in-out ownership slots; E10034 does not apply".
- `spec/06-functions.md:141-144` — the narrower rule: the callee owns the incoming value and may release it, so E10034 does not apply to *that value*.
- Probe: `func fill(borrow v: String, borrow slot: String?) { slot = v; }` already reports `E10034` at HEAD, so the broad sentences are already inconsistent with the implementation.

**The Problem**: The plan follows the narrow Chapter 06 rule but never reconciles the broad sentences in Chapters 03, 16, 14, 12, and the decision record. Implementing the new report makes the compiler contradict the frozen text, and the E10034 help text would print "the out-param slot is exempt" on the very error that rejects the out-param write. AGENTS.md requires an explicit user ruling for a specification change; the plan has no spec task.

**Options**:

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Add a spec-amendment task with a user ruling: amend 03:216, 06:144, 16:107, 14:62, 12:147, and decisions.md PF-001 to say the slot's own incoming value is exempt; a foreign lend/view source is still E10034; update the help text | Removes the contradiction; aligns the oracle with the intended rule | Touches the frozen spec; requires the ruling |
| B | Use a diagnostic the current spec permits for out-slot writes (for example `E10030` at the caller) | No spec change | Conflicts with the concrete `ownlend` E10034 and the plan's decision; changes the diagnostic contract |
| C | Leave the spec and record an accepted divergence | No spec edit | Violates the frozen-oracle rule; reviewers must reject |

**Recommendation**: Option A — the detailed Chapter 06 rule already supports the plan; the other chapters need the same narrow wording. Add the spec task before implementation and record the ruling.

**User Decision**: Resolved — User accepted recommendation: Option A (2026-10-22)

---

## PF-007: The plan reverses specification-test acceptance pins without a recorded ruling, and retargets them after implementation 🟠 MAJOR

**Dimension**: 11 — Ordering & Sequencing (also 4, 7)
**Location**: `plans/out-slot-view-escape/99-execution-plan.md:40-43` (acceptance pins retargeted), `:68-69` (T-07.4)

**Codebase Evidence**:
- `AGENTS.md` — `compiler/tests/*_spec_test.rs` are an immutable oracle; a specification change needs an explicit user ruling.
- `plans/00-roadmap.md:555` — "the AR-20 decision needs an explicit ruling first because many `xf*` tests depend on it".
- AR-20 lives in `plans/conditional-projection-residue/00-ambiguity-register.md` row 20 (and its resolution note at lines 100-103): an out-slot accepts parameter-backed owners because "the caller frame outlives the callee".
- `compiler/tests/borrow_generic_integrity_spec_test.rs:500-560` — `copy<String?>` accepted and ASan-clean; this is an owned nullable move, not a view store. Flipping it requires removing the nullable `writable_source && stores_into_slot` exemption in `check_not_lend_requirements` (`instance_check.rs:258-261`), a separate rule change.
- `compiler/tests/conditional_projection_residue_spec_test.rs:6417-6470` — the parameter-backed chain and same-frame call are accepted and safe as written; flipping them is a precision regression, not a demonstrated safety fix.
- `compiler/tests/eval_emission_integrity_spec_test.rs:5078-5111` — captured `slot = p` acceptance controls.
- `compiler/tests/borrow_store_integrity_spec_test.rs:1881-1905` — `mixedcarrier` expects one `E10030`.

**The Problem**: The plan's Decision table records a choice but not the authority the roadmap requires. It also lumps two different rule changes under "acceptance pins": the view-store rule (the demonstrated UAF) and the owned nullable move (`copy<String?>`, safe today). Retargeting the safe `:6417` and captured controls is over-rejection that must be decided, not assumed. T-07.4 performs the retarget after T-07.2/T-07.3, so the suite would be fitted to the implementation rather than red-first.

**Options**:

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Record the explicit user ruling (AR-20 + the spec-test oracle change), enumerate every affected pin in the plan, and retarget them red before implementation; split the view-store rule from the owned-nullable-move decision | Spec-first; auditable; keeps the two semantics distinct | Larger plan; the owned-move decision may stay accepted and out of T-07 |
| B | Treat the Decision table as the ruling and keep T-07.4 last | No extra work | No authority record; violates AGENTS.md and the roadmap's precondition |
| C | Keep the acceptance pins and adopt the summary/loan alternative | No oracle change | The plan rejects this as materially larger complexity |

**Recommendation**: Option A — record the ruling and separate the two rules. If the owned nullable transfer stays accepted, remove it from T-07's promises; if it is rejected, give it its own task and test. The safe-but-unprovable `:6417` shapes should be recorded as an accepted precision loss, not as an unsoundness.

**User Decision**: Resolved — User accepted recommendation: Option A (2026-10-22)

---

## PF-008: The `slot = slot` boundary row contradicts the pinned self-assignment rule 🟡 MINOR

**Dimension**: 1 — Ambiguities (also 3)
**Location**: `plans/out-slot-view-escape/99-execution-plan.md:52`

**Codebase Evidence**:
- Probe: `func self(borrow slot: str?) { slot = slot; }` reports `E10020`; `borrow slot: String?` also reports `E10020`.
- Pins: `borrow_generic_integrity_spec_test.rs:569-584` (out-slot self-assign), `borrow_store_integrity_spec_test.rs:432-448` (self-assign rules).

**The Problem**: The row says "`slot = slot` or a copy from the same slot | Accept". Direct self-assignment is rejected today, and the accepted shape is a copy via a local (`let t = slot; slot = t;`), which is not what the row spells. T-07.1's "same-slot control" could be written to expect acceptance and fail.

**Options**:

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Reword: "no new `E10034`; the existing `E10020` self-assignment report stands"; define the accepted copy-via-local shape; add a control | Matches the pinned rules; keeps the new rule scoped | None |
| B | Suppress `E10020` for view slots to make the row true | Simpler row | Changes a frozen move rule and its pins |

**Recommendation**: Option A.

**User Decision**: Resolved — User accepted recommendation: Option A (2026-10-22)

---

## PF-009: Cross-slot copies and captured same-slot copies are unclassified 🟡 MINOR

**Dimension**: 9 — Edge Cases (also 1)
**Location**: `plans/out-slot-view-escape/99-execution-plan.md:47-55` (Boundary)

**Codebase Evidence**:
- `compiler/src/own/expr.rs:2357-2375` — every out-slot is a `Param` root; a different out-slot is foreign by root identity.
- `compiler/src/own/expr.rs:2601-2603` — a captured whole out-slot keeps the ownership-slot kind; capture id normalization for a same-slot copy is unverified.

**The Problem**: The boundary covers only `slot = slot` and "a copy from the same slot". It does not say whether `slotA = slotB` (two caller out-slots) is foreign (it is: the caller can reassign B's backing) or whether a captured same-slot copy is accepted. Without a stated rule, the implementation will decide ad hoc and the tests will not pin it.

**Options**:

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Classify `slotA = slotB` as foreign (E10034) and a same-slot copy via a local as accepted; add both controls; pin the capture case | Complete boundary | Small extra test surface |
| B | Leave unclassified | Less text | Ambiguous behavior; unreviewed edge cases |

**Recommendation**: Option A.

**User Decision**: Resolved — User accepted recommendation: Option A (2026-10-22)

---

## PF-010: The claim that a non-nullable writable view slot is already rejected is false for the generic spelling 🟡 MINOR

**Dimension**: 13 — Codebase Alignment (also 2)
**Location**: `plans/out-slot-view-escape/99-execution-plan.md:55`

**Codebase Evidence**:
- Probe: `func copy<T>(borrow v: T, borrow slot: T) { slot = v; }` with `copy<str>(borrow v, borrow slot)` is accepted at HEAD. The emitted C declares `static void cl_copy__str(ClStr v, ClStr slot)` (the slot is passed by value) and assigns `slot = v`, so the caller's slot is unchanged — the write is silently lost.
- Concrete spelling probe: `func fill(p: str, borrow d: str) { d = p; }` reports `E10033`, so the boundary claim is true only for the concrete spelling.

**The Problem**: The row says "`borrow slot: str` ... already `E10033` from `DEF-540`; not reclassified". That is true for the concrete spelling but false for the generic instance, where the store is accepted and the write is lost. The plan leaves a real residual unmentioned while claiming the boundary is closed.

**Options**:

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Correct the boundary claim and file a separate residual for the generic non-nullable spelling (out of T-07's scope) | Keeps T-07 focused; records the gap | The residual remains until its own plan |
| B | Extend T-07 to close the generic non-nullable spelling | Closes the gap now | Expands a critical plan's scope and audit surface |

**Recommendation**: Option A — correct the claim and file the residual; T-07's scope is the nullable ownership slot.

**User Decision**: Resolved — User accepted recommendation: Option A (2026-10-22)

---

## Iteration 2 and 3 follow-on findings

The rewritten plan was re-scanned. Nine follow-on root causes (3 major, 6 minor) were found and fixed; a third pass reopened one major and added three minors, all fixed.

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PA-001 | 🟠 | The 188 count covered only the exact helper; wider whole-slot fixtures exist (`aggoutslot`, `aggstatic`, `outslotpartwide`, `closeout-st14`, the `Holder`/`str`/`Box`/`str[2]` shapes, and the capture cases) | Inventory by store shape across all test files and the own unit tests; explicit counts in 1.1.4/1.1.7; migration tasks 1.1.5-1.1.8 |
| PA-002 | 🟠 | The matrix still accepted direct `slot = slot` | Split direct self-assign (`E10020`) from copy-via-local (accept) |
| PA-003 | 🟠 | The static carve-out did not cover the static-backed `aggstatic` fixed-array element | Recorded static-array precision loss; `aggstatic` retargeted to `E10034` |
| PA-004 | 🟡 | The one-report claim included the non-nullable `str[2]` array shape | Excluded the array shape; it keeps `E10033` + `E10030` |
| PA-005 | 🟡 | The pre-pass/post-pass choice was non-viable | Diagnostic-free pre-pass mandated; post-pass rejected; shape-based bit |
| PA-006 | 🟡 | No captured same-slot copy control | Added to 1.1.1 |
| PA-007 | 🟡 | The spec amendment missed `03-ownership.md` §7 and OW-16 | Added §7 (line 239) and §9/OW-16 (line 292) |
| PA-008 | 🟡 | The batch-V residual and new-row references were dangling | Named `plans/00-roadmap.md` and `plans/deferred-evaluation-completion/05-findings.md:61` |
| PA-009 | 🟡 | The `take` acceptance was stated without an instance | Specified the concrete nullable `fillString` case and the taken-view `E10030` |
| NEW-1 | 🟡 | A local copy of a foreign lend was conflated with a frame-local view | Added the row; `captureoutslotviewlocal` → `E10034` |
| NEW-2 | 🟡 | Owning `take` and taken-view sources were conflated | Split the cases in the matrix and 1.1.1/1.1.2 |
| NEW-3 | 🟡 | OW-16 was labelled §7, not §9 | Corrected the section label |

All follow-on findings are resolved in the rewrite.

## Pass/fail determination

✅ **PREFLIGHT PASSED WITH NOTES** — all 10 iteration-1 findings and all 12 follow-on root causes are resolved in the rewritten plan. Two accepted precision losses are recorded in the plan's Ruling section:

1. A static-backed fixed-array element of a by-value aggregate (`aggstatic`) is rejected; a later static-aggregate analysis can restore the acceptance.
2. The safe-but-unprovable parameter-backed and same-frame pass-through programs are rejected, because the checker cannot prove the caller will not reassign the backing.

The roadmap row for `out-slot-view-escape` advances to `Plan Preflighted` (🔬).

## Next actions

1. **Execute** the 27-task plan via `exec-plan`, phases 0-5.
2. **No further preflight** is required; the report is final. Any subsequent plan edit must be compared with the plan's content hash/mtime and re-checked in the changed sections.
3. **Roadmap**: `Plan Preflighted` now; execution advances the row to Done after Phase 5.

## Notes

- The plan was rewritten after the rulings; this report records the iteration-2/3 follow-on findings and their resolution. No `spec/` or code file was modified.
- The plan, this report, and the `plans/00-roadmap.md` row are the only modified repository files. Probe files live under `/tmp/opencode` and were deleted at the end of the run.
- The report is the durable audit trail; there is no Ambiguity Register for this plan (the ruling is recorded here and in the plan's Ruling section).
- Same-agent bias: the plan was created in an earlier session (commit `a6dde7d`), not this one; independent audit dispatches and a challenger were used, but a human domain expert review is still advisable for the spec-oracle reversal.
