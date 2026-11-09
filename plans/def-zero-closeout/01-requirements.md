# Requirements: Zero-Defect Closeout

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Close every open defect row in `plans/00-roadmap.md` so the repository has no
open DEF row: 114 rows (5 critical, 9 major, 100 minor). The plan fixes them in
six risk-ordered batches after a revalidation phase. Every row receives a
specification regression that is red before its fix and green after it, and
every memory or emission row receives the ASAN or strict-C pin its class needs.

## Functional Requirements

### Must Have

- [x] R1 — Batch A (Critical/major backlog): all 14 rows below are fixed and their ST cases pass: DEF-544, DEF-545, DEF-546, DEF-547, DEF-548, DEF-549, DEF-550, DEF-551, DEF-552, DEF-553, DEF-554, DEF-555, DEF-556, DEF-557.
- [x] R2 — Batch B (Null and type rules): all 16 rows below are fixed and their ST cases pass: DEF-190, DEF-191, DEF-192, DEF-193, DEF-195, DEF-246, DEF-247, DEF-248, DEF-249, DEF-383, DEF-414, DEF-417, DEF-420, DEF-467, DEF-483, DEF-542.
- [x] R3 — Batch C (Calls and resolution): all 22 rows below are fixed and their ST cases pass: DEF-169, DEF-173, DEF-174, DEF-198, DEF-203, DEF-205, DEF-212, DEF-214, DEF-236, DEF-239, DEF-240, DEF-241, DEF-244, DEF-254, DEF-349, DEF-382, DEF-387, DEF-388, DEF-389, DEF-404, DEF-405, DEF-419.
- [x] R4 — Batch D (Borrow and loans): all 18 rows below are fixed and their ST cases pass: DEF-250, DEF-252, DEF-311, DEF-312, DEF-354, DEF-370, DEF-422, DEF-434, DEF-444, DEF-463, DEF-474, DEF-484, DEF-490, DEF-491, DEF-492, DEF-494, DEF-524, DEF-530.
- [x] R5 — Batch E (Emission and ABI): all 37 rows below are fixed and their ST cases pass: DEF-194, DEF-199, DEF-200, DEF-201, DEF-202, DEF-206, DEF-208, DEF-217, DEF-218, DEF-221, DEF-222, DEF-223, DEF-224, DEF-225, DEF-226, DEF-227, DEF-228, DEF-234, DEF-253, DEF-263, DEF-313, DEF-320, DEF-324, DEF-325, DEF-326, DEF-327, DEF-328, DEF-330, DEF-334, DEF-352, DEF-355, DEF-358, DEF-392, DEF-393, DEF-438, DEF-482, DEF-500.
- [x] R6 — Batch F (Initializers and carriers): all 7 rows below are fixed and their ST cases pass: DEF-210, DEF-314, DEF-316, DEF-317, DEF-318, DEF-319, DEF-321.

- [x] R7 — Phase 0 revalidates every row at HEAD; a stale or incidentally fixed
  row closes with a HEAD command and its accepted result recorded, never by
  assumption.
- [x] R8 — The authorized specification changes are exactly: `E10309` (a call of
  a non-callable value), `len(cstring)` support with scalar `len` rejected by
  `E10200`, the `E10010` call-spelling subject, and the bounded
  instance-bound static-return marker for `DEF-557`. No other spec or golden
  change is authorized.
- [x] R9 — No feature work, no generalized analysis framework, and no change to
  the pre-existing non-DEF list in `def-critical-major-closeout/05-findings.md`.

### Non-Functional

- [x] N1 — Specification-first order per batch: ST cases red, then the fix,
  then implementation/ASAN/strict pins.
- [x] N2 — Deterministic diagnostics and byte-identical re-emission where the
  owning suite pins it.
- [x] N3 — `make check` and `make verify` pass at every phase close; the focused
  `RDC_ASAN=1` suites are clean for memory rows; strict `-Werror` C builds for
  emission rows.
- [x] N4 — Security: no new input path; the compiler remains the only parser of
  untrusted `.clo` sources; no secrets or network access are introduced.
- [x] N5 — Documentation: the roadmap, the plan ledger, and the AGENTS test map
  stay in sync; no plan/DEF identifier appears in source comments.

## Acceptance Criteria

1. `plans/00-roadmap.md` has 546/546 DEF rows Done and zero open DEF rows.
2. All 114 ST cases are red before their fix and green after it (stale closes
   are green HEAD pins).
3. `make check` (74+ targets, the compile-fail goldens) and `make verify` (all
   legs, including FS-UAE) pass.
4. The final four-lens review is clean or every finding is recorded with a
   ruling; no critical/major finding is open.
5. The plan is 100% and the close summary is in `05-findings.md`.

## Out of Scope

Feature work; the pre-existing non-DEF report-only list; a general
cross-function summary subsystem beyond the DEF-557 marker.
