# Requirements: Conditional, Projection, and Update Residue

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Handle the 23 defect rows batch H filed and did not close: 20 implemented closes, one incidentally
closed at the planning baseline (`DEF-259`, kept as a green control), and two parked for rulings
(`DEF-268`, `DEF-279`). Each implemented row is a spelling or position the batch-H fixes did not
reach, with a reproduction and base == head evidence in the roadmap. Every requirement below is
conformance to the frozen specification: the fix makes an accepted program build and run with the
language's semantics, or makes a rejected program report the judgment its twin spelling already
reports.

## Functional Requirements

### Must Have

- [ ] **R1 — Arm roots and per-instance duty** (`DEF-257`, `DEF-258`, `DEF-260`, `DEF-264`;
  `DEF-259` incidental): a projection of a mixed conditional moves the selected member per
  *instance* (generics included); a conditional-of-projections binds the fresh arm's root so its
  sibling fields release; a guarded `take` of a conditional projection releases exactly once via
  the eager carrier; a struct-literal field whose value projects an ownerless root binds that root
  in guarded positions too. (`DEF-259`'s struct-literal arm root is incidentally closed at the
  planning baseline and stays a green control — AR-23.)
- [ ] **R2 — View derivation and owners** (`DEF-261`, `DEF-266`, `DEF-267`; `DEF-268`/`DEF-279`
  parked): a view-returning call records its argument derivation — a call-wrapped conditional's
  *every* arm and a chained view-returning callee included; a view-element array in an argument or
  field position binds or retains each temporary element's backing; the `str[N]` return spelling
  reports exactly its single-view twin's judgment. (`DEF-268`'s disagreeing conditional return and
  `DEF-279`'s duplicate escape report stay at current behavior pending rulings — AR-2.)
- [ ] **R3 — Escaping stores** (`DEF-265`, `DEF-269`, `DEF-276`, `DEF-280`): a view stored where
  it outlives its backing is rejected per destination kind — a global or field accepts module-level
  static owners only (`E10033`; a parameter-backed store stays rejected), a `borrow T?` out-slot
  accepts static and parameter-backed owners and otherwise reports `E10030`; the view-array store
  to a global struct field and the lambda capture of a `str[N]` build strictly; a local view-array
  element store is accepted only when the backing's scope covers the array's (a loop-iteration or
  nested-scope backing reports `E10030`), with the binding path's owner loan recorded.
- [ ] **R4 — Guarded and discarded literal positions** (`DEF-262`, `DEF-270`, `DEF-271`): a
  conditional literal receiver in a loop condition does not give one field two owners (the
  struct-literal and array-literal forms); a discarded struct-literal arm materializes its fresh
  fields while its place-valued fields stay reads (the alias model); a discarded conditional's
  fresh projection arm binds its receiver root.
- [ ] **R5 — Fixed-array conditional arms and projections** (`DEF-272`, `DEF-275`): a
  fixed-array conditional arm routes through the per-arm destination store (the compiler contract's
  hidden out-parameter ABI, `spec/17-codegen.md:59-84`); a fixed-array projection of a conditional
  root binds each arm's root and renders the per-arm address form; a `for`-iterable projection
  routes through the projection preparation.
- [ ] **R6 — Update order completion** (`DEF-273`, `DEF-274`, `DEF-277`, `DEF-278`): an assignment
  target's location and index parts are pinned before the right-hand side's update, and an
  update-containing right-hand side pins at its own source position; an update-containing literal
  field or element is pinned at its source position; the dot receiver and a loop-condition operand
  are pinned (the loop condition through a per-iteration `Materialize` fill); an update in a later
  argument does not reorder an earlier owning argument.

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
- `DEF-268` — whether the disagreeing conditional return reports `E10036`: the batch-H spec suite
  pins the conditional twin accepted, so re-pinning it is a specification decision (AR-2).
- `DEF-279` — whether the local escape report replaces the generated-owner report at one site: the
  oracle pins exactly one `E10030` for a shape with a local arm (AR-2).
- `DEF-211`, `DEF-214`, `DEF-233` — other residues' ruling-gated rows: the fixed-array `sizeof`
  measurement, the diagnostic code for a call of a non-callable callee (shared with `DEF-167`), and
  the value-form enum member rule.
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
| Batch scope | H residue (23 handled) / E residue (8 unblocked) / G residue (13 majors) | H residue | Most majors (15; 14 implemented after the preflight parked one), one machinery, fresh reproductions | AR-1 |
| Ruling-gated rows | include and pause / park | park | Reserved decisions are not touched; the two preflight parks conflict with immutable suite pins | AR-2, AR-7, AR-8 |
| Escaping view stores | reject per destination (`E10033` fields/globals, `E10030` out-slots) / accept and adopt | reject | The frozen BR-8 rule rejects storage that outlives the backing; the destination kind decides which owners survive | AR-5, AR-20 |
| Local element stores | accept (scoped to the backing's scope, with the loan recorded) / keep rejecting | accept | A local array dies with the frame; the rejection was over-strict, and the loan keeps loop/nested backings rejected | AR-6, AR-21 |
| Update pins | trigger extensions / a new pass | extensions | The pin machinery already exists; the rows are trigger gaps | AR-9 |
| Fixed-array arms | destination shape / reject | destination shape | Ch 17 requires valid C for accepted shapes | AR-11 |

> **Traceability:** Every scope decision references the Ambiguity Register entry that resolved it.
> See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] Every implemented row is closed with a reproduction that flips (red before, green after)
   and a suite pin that fails without the fix; `DEF-259` closes incidentally with its probe
   evidence (a green control), and the two parked rows stay at current behavior.
2. [ ] Every ownership fix is ASan-clean on the host (plain and `RDC_ASAN=1`), with loop and
   early-exit variants where the shape is re-executed.
3. [ ] The strict build passes for every accepted shape in scope; the corpus differential shows
   only intended deltas.
4. [ ] No behavior change outside the in-scope rows; the batch-H suites stay green.
5. [ ] `make check` green on every phase; `make verify` green at close-out.
6. [ ] The roadmap's 23 handled rows are accounted for: 20 close as Done with their evidence,
   `DEF-259` closes with the incidental note, and `DEF-268`/`DEF-279` return to Backlog parked;
   the eight parked rows stay open with their rulings collected (AR-2).
