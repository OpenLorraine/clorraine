# Execution Plan: Zero-Defect Closeout

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-28
> **Progress**: 4/57 tasks (7%)
> **CodeOps Artifact Schema**: 1

## Overview

Close all 114 open DEF rows in `plans/00-roadmap.md` in six risk-ordered
batches after a Phase 0 revalidation. Each batch follows the specification-first
ordering: red ST cases, the fix, implementation/ASAN/strict-C pins, and a
four-lens phase review. The plan completes only when the roadmap has zero open
DEF rows and `make check`/`make verify` pass.

**🚨 Update this document after EACH completed task!**

---

## Phase 0: Revalidation and Baseline

> **Reasoning**: high — 114 rows must be reconfirmed at HEAD before any fix.
> **Baseline tree**: `54197ce`
> **Lenses**: correctness, semantics, memory, emission (probe routing only)

- [x] 0.1.1 [spec-author] Revalidate the Batch A and B rows (30: DEF-544…DEF-557, DEF-190…DEF-542) at HEAD `54197ce`; for each, run its recorded reproduction with the driver and record confirmed/stale/incidental plus the exact command and result in `05-findings.md`.
  ✅ 2026-10-28: 30 rows probed at `2aa138c`; 26 confirmed, 3 stale (DEF-414/554/557), 1 manual (DEF-550).
- [x] 0.1.2 [spec-author] Revalidate the Batch C and D rows (40: DEF-169…DEF-419, DEF-250…DEF-530) at HEAD; record the same evidence and flag any row whose fix needs a new ruling.
  ✅ 2026-10-28: 40 rows probed; 29 confirmed, 4 stale, 3 incidental, 3 needs-ruling (DEF-169/214/252; 214 resolved by AR-4), 1 manual (DEF-370).
- [x] 0.1.3 [spec-author] Revalidate the Batch E and F rows (44: DEF-194…DEF-500, DEF-210…DEF-321) at HEAD; record the same evidence and flag any row whose fix needs a new ruling.
  ✅ 2026-10-28: 44 rows probed; 20 confirmed, 20 incidental, 4 manual (DEF-324/334/355/438).
- [x] 0.1.4 [orchestrator] Close stale and incidentally fixed rows through the roadmap skill with HEAD evidence; confirm the remaining count and the batch mapping; if a row needs an unauthorized spec change or a new material surface, stop under AR-14.
  ✅ 2026-10-28: 30 rows closed with HEAD evidence; 84 open (5 critical + 7 major + 72 minor); DEF-169 and DEF-252 are reserved for a user ruling before Phase 3/4.

**Verify**: every open row has a HEAD evidence line; stale closes are recorded; the confirmed mapping is in `05-findings.md`.

---

## Phase 1: Critical/Major Backlog — DEF-544…DEF-557

> **Reasoning**: high — the five critical and nine major rows are the safety gate.
> **Lenses**: correctness, semantics, memory, emission

- [x] 1.1.1 [spec-author] Write ST-1…ST-5 in `def_zero_critical_spec_test.rs` for DEF-544…DEF-548 and confirm each is red on the Phase 0 baseline.
- [x] 1.1.2 [spec-author] Write ST-6…ST-10 for DEF-549…DEF-553 and confirm red.
- [x] 1.1.3 [spec-author] Write ST-11…ST-14 for DEF-554…DEF-557 and confirm red (DEF-557 red = the generic rejection while the concrete twin is accepted).
- [x] 1.2.1 [impl-author] Fix DEF-544…DEF-547: the plain-parameter conditional store, the multi-subrange take capture, the conditional lend return, and the maybe-target view instances. Keep every safe control accepted and ASAN-clean.
- [x] 1.2.2 [impl-author] Fix DEF-548…DEF-552: the runtime-index store path, the literal zero divisor, the view-destination loan/escape duty, the expression-form array fill, and the argument order hoist.
- [x] 1.2.3 [impl-author] Fix DEF-553…DEF-557: the initializer literal leak/order rows, the generic `T` view-return duty, and the bounded instance-bound static-return marker (AR-5) with its `spec/16` note.
- [x] 1.3.1 [impl-author] Write `def_zero_critical_impl_test.rs` pins, run the focused `RDC_ASAN=1` and strict-C legs, and dispatch the four-lens phase review; fix every critical/major finding through the one allowed re-review.
- [x] 1.3.2 [orchestrator] Run `make check` and `make verify`; commit and push through the git-commit skill in push mode; record the phase result in `05-findings.md`.

**Verify**: the phase ST cases are green; focused ASAN/strict-C legs are clean; `make check` and `make verify` pass; the phase review is recorded. **Phase 1 result (2026-10-29)**: all 12 live rows fixed; the three-lens review findings were fixed through the review-fix batches; `make check` green and `make verify: all legs passed`; report-only pre-existing shapes are recorded in `05-findings.md`.

---

## Phase 2: Null and Type Rules — DEF-190…DEF-542

> **Reasoning**: high — null and narrowing rules touch every stage; the `len` and `E10010` rulings are authorized.
> **Lenses**: correctness, semantics, memory

- [x] 2.1.1 [spec-author] Write ST-15…ST-20 in `def_zero_null_spec_test.rs` for DEF-190…DEF-195 and confirm red.
- [x] 2.1.2 [spec-author] Write ST-21…ST-24 for DEF-246…DEF-249 and confirm red.
- [x] 2.1.3 [spec-author] Write ST-25…ST-31 for DEF-383, DEF-414, DEF-417, DEF-420, DEF-467, DEF-483, DEF-542 and confirm red.
- [x] 2.2.1 [impl-author] Fix DEF-190…DEF-195: the nameless nullable call subject (AR-6), the `unsafe` null-rule conformance (AR-7), the qualified nullable read, the short-circuit/loop narrowing (AR-8), and the `len` surface (AR-3; `cstring` runtime length, scalar `E10200`). Update the authorized `spec/04`/appendix notes.
- [x] 2.2.2 [impl-author] Fix DEF-246…DEF-249 and DEF-383: the aggregate, member, arithmetic, condition, and inferred-call null positions.
- [x] 2.2.3 [impl-author] Fix DEF-414, DEF-417, DEF-420, DEF-467, DEF-483: the nullable-gated report, TY-4 coverage, the negated float literal, the raw out-slot self-assign, and scalar `len`.
- [x] 2.2.4 [impl-author] Fix DEF-542: the generic non-nullable view out-slot store, matching the concrete twin.
- [x] 2.3.1 [impl-author] Write the `def_zero_null_impl_test.rs` pins, run the focused ASAN/strict legs, and dispatch the phase review; fix critical/major findings.
- [x] 2.3.2 [orchestrator] Run `make check` and `make verify`; commit and push; record the phase result.

**Verify**: the phase ST cases are green; focused ASAN/strict-C legs are clean; `make check` and `make verify` pass; the phase review is recorded. **Phase 2 result (2026-10-29)**: all 15 open rows closed; 43 spec cases and two impl pins; the three review lenses and re-audits were resolved; `make check` green (77 targets, 110 goldens) and `make verify: all legs passed`. Report-only precision trade-offs are recorded in `05-findings.md`.

---

## Phase 3: Calls and Resolution — DEF-169…DEF-419

> **Reasoning**: medium — bounded resolution/diagnostic rules; `E10309` is the only registry addition.
> **Lenses**: correctness, semantics, emission

- [x] 3.1.1 [spec-author] Write ST-32…ST-39 in `def_zero_calls_spec_test.rs` for DEF-169, DEF-173, DEF-174, DEF-198, DEF-203, DEF-205, DEF-212, DEF-214 and confirm red.
- [x] 3.1.2 [spec-author] Write ST-40…ST-47 for DEF-236, DEF-239, DEF-240, DEF-241, DEF-244, DEF-254, DEF-349, DEF-382 and confirm red.
- [x] 3.1.3 [spec-author] Write ST-48…ST-53 for DEF-387, DEF-388, DEF-389, DEF-404, DEF-405, DEF-419 and confirm red.
- [x] 3.2.1 [impl-author] Fix DEF-169, DEF-173, DEF-174, DEF-198, DEF-203, DEF-205: the func-type key collision, the cross-module mode lookup, the closure/initializer markers, the initializer global receiver, and the intrinsic gate.
- [x] 3.2.2 [impl-author] Fix DEF-212, DEF-214, DEF-236, DEF-239, DEF-240, DEF-241: the unused-index local, `E10309` (AR-4) with its `spec/14` registry row and compile-fail golden, the func-value arity, the Array dot-call, the qualified arity, and the raw receiver arrow.
- [x] 3.2.3 [impl-author] Fix DEF-244, DEF-254, DEF-349, DEF-382: the ffi parameter validation, `--max-errors 0`, the host-clean cast help, and the literal-context argument route.
- [x] 3.2.4 [impl-author] Fix DEF-387, DEF-388, DEF-389, DEF-404, DEF-405, DEF-419: the free-function member shadow, the null-base member, the module-path method error, the cstring literal argument, the explicit-generic literal, and the import-renamed type names.
- [ ] 3.3.1 [impl-author] Write the `def_zero_calls_impl_test.rs` pins, run the focused legs, and dispatch the phase review; fix critical/major findings.
- [ ] 3.3.2 [orchestrator] Run `make check` and `make verify`; commit and push; record the phase result.

**Verify**: the phase ST cases are green; focused ASAN/strict-C legs are clean; `make check` and `make verify` pass; the phase review is recorded.

---

## Phase 4: Borrow and Loans — DEF-250…DEF-530

> **Reasoning**: high — ownership, loans, and capture subjects; ASAN is the primary oracle.
> **Lenses**: correctness, semantics, memory

- [ ] 4.1.1 [spec-author] Write ST-54…ST-62 in `def_zero_loans_spec_test.rs` for DEF-250, DEF-252, DEF-311, DEF-312, DEF-354, DEF-370, DEF-422, DEF-434, DEF-444 and confirm red.
- [ ] 4.1.2 [spec-author] Write ST-63…ST-71 for DEF-463, DEF-474, DEF-484, DEF-490, DEF-491, DEF-492, DEF-494, DEF-524, DEF-530 and confirm red.
- [ ] 4.2.1 [impl-author] Fix DEF-250, DEF-252, DEF-311, DEF-312, DEF-354, DEF-370, DEF-422: capture subjects, const moves, residual roots, element-loan owners, duplicate capture reports, move-only arrays, and the raw-prefix store.
- [ ] 4.2.2 [impl-author] Fix DEF-434, DEF-444, DEF-463, DEF-474, DEF-484: the overwritten literal provenance, the loop-head pointee move, the raw-store unused warning, the generic view conditional, and the loop-alias static carrier.
- [ ] 4.2.3 [impl-author] Fix DEF-490, DEF-491, DEF-492, DEF-494, DEF-524, DEF-530: the inline static aggregate, the frame-backed arm, the zero-arg factory, the taken view double report, the shared later-use report, and the two carrier returns.
- [ ] 4.3.1 [impl-author] Write the `def_zero_loans_impl_test.rs` pins, run the focused ASAN legs, and dispatch the phase review; fix critical/major findings.
- [ ] 4.3.2 [orchestrator] Run `make check` and `make verify`; commit and push; record the phase result.

**Verify**: the phase ST cases are green; focused ASAN/strict-C legs are clean; `make check` and `make verify` pass; the phase review is recorded.

---

## Phase 5: Emission and ABI — DEF-194…DEF-500

> **Reasoning**: high — accepted programs must emit valid, ordered strict C.
> **Lenses**: correctness, semantics, emission, memory

- [ ] 5.1.1 [spec-author] Write ST-72…ST-80 in `def_zero_emission_spec_test.rs` for DEF-194, DEF-199…DEF-202, DEF-206, DEF-208, DEF-217, DEF-218 and confirm red.
- [ ] 5.1.2 [spec-author] Write ST-81…ST-89 for DEF-221…DEF-228 and DEF-234 and confirm red.
- [ ] 5.1.3 [spec-author] Write ST-90…ST-98 for DEF-253, DEF-263, DEF-313, DEF-320, DEF-324…DEF-328 and confirm red.
- [ ] 5.1.4 [spec-author] Write ST-99…ST-108 for DEF-330, DEF-334, DEF-352, DEF-355, DEF-358, DEF-392, DEF-393, DEF-438, DEF-482, DEF-500 and confirm red.
- [ ] 5.2.1 [impl-author] Fix DEF-194, DEF-199…DEF-202, DEF-206, DEF-208, DEF-217, DEF-218: the strict-C guards, closure widening, qualified function values, zero divisors, func-typed array declarators, fixed-array field guards, mixed-arm ternaries, and func-pointer slices.
- [ ] 5.2.2 [impl-author] Fix DEF-221…DEF-228 and DEF-234: the literal base index, missing field expansion, sizeof instance spelling, cstring/String `.len`, mixed-arm `len`, opaque projection chains, lifted fixed-array captures, the raw unsafe gate, and the freestanding fold.
- [ ] 5.2.3 [impl-author] Fix DEF-253, DEF-263, DEF-313, DEF-320, DEF-324…DEF-328: discarded array literals, the folded `len` receiver (AR-9), the all-null argument, carrier read-back, guarded literals, fill evaluation, deferred fills, and the GCC sequence-point carrier.
- [ ] 5.2.4 [impl-author] Fix DEF-330, DEF-334, DEF-352, DEF-355, DEF-358: the raw-store unused warning, the rollback subject, raw `.ptr`/`.len`, the compound-initializer null, and the timeout harness pipe drain.
- [ ] 5.2.5 [impl-author] Fix DEF-392, DEF-393, DEF-438, DEF-482, DEF-500: the sizeof fold, offsetof, eager take hoist, mixed-fresh arm types, and the three strict-build residual classes.
- [ ] 5.3.1 [impl-author] Write the `def_zero_emission_impl_test.rs` pins, run the strict-C and ASAN legs, and dispatch the phase review; fix critical/major findings.
- [ ] 5.3.2 [orchestrator] Run `make check` and `make verify`; commit and push; record the phase result.

**Verify**: the phase ST cases are green; focused ASAN/strict-C legs are clean; `make check` and `make verify` pass; the phase review is recorded.

---

## Phase 6: Initializers and Carriers — DEF-210…DEF-321

> **Reasoning**: high — fresh-root leaks and initializer literals; ASAN is the primary oracle.
> **Lenses**: correctness, semantics, memory

- [ ] 6.1.1 [spec-author] Write ST-109…ST-115 in `def_zero_initializers_spec_test.rs` for DEF-210, DEF-314, DEF-316…DEF-319, DEF-321 and confirm red.
- [ ] 6.2.1 [impl-author] Fix DEF-210, DEF-314, DEF-316, DEF-317: the initializer take sibling leak, the module-view conditional leak, the duplicable guarded member root, and the owning-literal carrier residual.
- [ ] 6.2.2 [impl-author] Fix DEF-318, DEF-319, DEF-321: the initializer carrier residual, the direct literal carrier return, and the view-typed Index receiver.
- [ ] 6.3.1 [impl-author] Write the `def_zero_initializers_impl_test.rs` pins, run the focused ASAN legs, and dispatch the phase review; fix critical/major findings.
- [ ] 6.3.2 [orchestrator] Run `make check` and `make verify`; commit and push; record the phase result.

**Verify**: the phase ST cases are green; focused ASAN/strict-C legs are clean; `make check` and `make verify` pass; the phase review is recorded.

---

## Phase 7: Full Verification, Review, and Close-out

> **Reasoning**: high — program gate; zero open DEF rows and a clean final review.
> **Lenses**: all four

- [ ] 7.1.1 [orchestrator] Run the full `make check`, `make verify`, and every focused `RDC_ASAN=1` suite; confirm the determinism pins.
- [ ] 7.1.2 [orchestrator] Dispatch the final four-lens review; fix critical/major findings and perform the single fix-diff re-review; record minor rulings.
- [ ] 7.1.3 [orchestrator] Close every remaining row through the roadmap skill; confirm 546/546 Done and zero open; update the ledger and plan to 100%.
- [ ] 7.1.4 [orchestrator] Post-completion re-analysis, workspace cleanup, and the final summary.

**Verify**: zero open DEF rows; `make check` and `make verify` all legs pass; the final review is clean or recorded; the plan is 100%.

---

## Dependencies

```
Phase 0 → Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 6 → Phase 7
```
