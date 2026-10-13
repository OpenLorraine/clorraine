# Requirements: Conditional, Projection, and Update Residue

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Close the 23 defect rows batch H filed and did not close. Each row is a spelling or position the
batch-H fixes did not reach, with a reproduction and base == head evidence in the roadmap. Every
requirement below is conformance to the frozen specification: the fix makes an accepted program
build and run with the language's semantics, or makes a rejected program report the judgment its
twin spelling already reports.

## Functional Requirements

### Must Have

- [ ] **R1 — Arm roots and per-instance duty** (`DEF-257`, `DEF-258`, `DEF-259`, `DEF-260`,
  `DEF-264`): a projection of a mixed conditional moves the selected member per *instance*
  (generics included); a conditional-of-projections binds the fresh arm's root so its sibling
  fields release; a struct-literal arm's root binds in the same positions; a guarded `take` of a
  conditional projection releases exactly once via the eager carrier; a struct-literal field whose
  value projects an ownerless root binds that root in guarded positions too.
- [ ] **R2 — View derivation and owners** (`DEF-261`, `DEF-266`, `DEF-267`, `DEF-268`, `DEF-279`):
  a view-returning call records its argument derivation (a call-wrapped conditional included); a
  view-element array in an argument or field position binds or retains each temporary element's
  backing; the `str[N]` return spelling reports exactly its single-view twin's judgment; a
  conditional return whose arms do not agree on the single view-like candidate reports `E10036`;
  a return that reports the local escape does not also report the generated-owner escape at the
  same site.
- [ ] **R3 — Escaping stores** (`DEF-265`, `DEF-269`, `DEF-276`, `DEF-280`): a view stored where
  it outlives its backing (a global, a field, a `borrow T?` out-slot) is rejected like the return
  path; the view-array store to a global struct field and the lambda capture of a `str[N]` build
  strictly; a local view-array element store (a backing that dies with the frame) is accepted.
- [ ] **R4 — Guarded and discarded literal positions** (`DEF-262`, `DEF-270`, `DEF-271`): a
  conditional literal receiver in a loop condition does not give one field two owners; a discarded
  struct-literal arm materializes its fresh fields (the place-aliasing ones stay inline); a
  discarded conditional's fresh projection arm binds its receiver root.
- [ ] **R5 — Fixed-array conditional arms and projections** (`DEF-272`, `DEF-275`): a
  fixed-array conditional arm routes through the destination-store shape (the hidden out-parameter
  ABI); a fixed-array projection of a conditional root and of a `for` iterable binds the root
  before the address.
- [ ] **R6 — Update order completion** (`DEF-273`, `DEF-274`, `DEF-277`, `DEF-278`): an assignment
  target's location and index parts are pinned before the right-hand side's update; an
  update-containing literal field or element is pinned at its source position; the dot receiver and
  a loop-condition operand are pinned; an update in a later argument does not reorder an earlier
  owning argument.

### Won't Have (Out of Scope)

- `DEF-167` — a call of a non-callable callee: the diagnostic code is ruling-gated (shared with
  `DEF-214`).
- `DEF-169` — the generated type-key collision: an escaping scheme is an ABI decision with its own
  trigger.
- `DEF-171` — a staged `embed` outside a constant context: awaits the staged-`embed` decision.
- `DEF-190` — the diagnostic for a nameless nullable source: ruling needed.
- `DEF-191` — the `unsafe` exemption's consistency with the conversion rule: ruling needed.
- `DEF-263` — whether a folded fixed-array `.len` receiver evaluates its operand: spec ruling
  needed.
- Any new diagnostic code, registry row, or specification text (reserved; every fix reuses the
  frozen registry).
- The batch E/F/G residues (sequenced after this batch; AR-1).

## Technical Requirements

### Compatibility

- The host, hosted-amiga, and freestanding profiles stay conformant; every accepted program keeps
  building under `-std=c99 -Wall -Wextra -Werror`.
- Byte identity wherever a phase does not change the shape (the emission differential classifies
  every delta).

### Security

- Not applicable beyond the compiler's own input validation (unchanged).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Batch scope | H residue (23) / E residue (8 unblocked) / G residue (13 majors) | H residue | Most majors (15), one machinery, fresh reproductions | AR-1 |
| Ruling-gated rows | include and pause / park | park | Reserved decisions are not touched | AR-2 |
| Escaping view stores | reject (`E10033`) / accept and adopt | reject | The frozen BR-8 rule rejects storage that outlives the backing | AR-5 |
| Local element stores | accept / keep rejecting | accept | A local array dies with the frame; the rejection was over-strict | AR-6 |
| Update pins | trigger extensions / a new pass | extensions | The pin machinery already exists; the rows are trigger gaps | AR-9 |
| Fixed-array arms | destination shape / reject | destination shape | Ch 17 requires valid C for accepted shapes | AR-11 |

> **Traceability:** Every scope decision references the Ambiguity Register entry that resolved it.
> See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] Every row in scope is closed with a reproduction that flips (red before, green after) and
   a suite pin that fails without the fix.
2. [ ] Every ownership fix is ASan-clean on the host (plain and `RDC_ASAN=1`), with loop and
   early-exit variants where the shape is re-executed.
3. [ ] The strict build passes for every accepted shape in scope; the corpus differential shows
   only intended deltas.
4. [ ] No behavior change outside the in-scope rows; the batch-H suites stay green.
5. [ ] `make check` green on every phase; `make verify` green at close-out.
6. [ ] The roadmap's 23 rows close with their evidence; the parked rows stay open with their
   rulings collected.
