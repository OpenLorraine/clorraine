# Diagnostics and Marks: Conditional, Projection, and Update Residue II

> **Document**: 03-05-diagnostics-and-marks.md
> **Parent**: [Index](00-index.md)

## Overview

Three rows in the report rendering and judgment marks: the duplicate reports at a nested mixed
view return (plus its cross-site siblings), the compiler-generated name in the deferred-owner
message, and the stale move mark on a whole-array conditional iterable. The operative rules are
the batch-I one-report boundary (its AR-26, extended here as AR-7), the generated-owner phrase
already pinned by `checker_conformance_spec_test`, and the frozen move marks. No message text is
new; the reports and marks align with the emissions the earlier phases settled.

## Design

### 1. One report per span (`DEF-282`, AR-7)

`return (c ? a : (d ? mkBox() : mkBox2())).v;` reports two identical
`E10030: borrow of a compiler-generated owner does not live long enough` at one span
(probe-confirmed, 12:5): the per-backing reports are not deduped. The extended instances from the
batch-I phase-3 confirmation: `gfieldcall` adds an `E10033` at the store while the callee's view
return already reports `E10036`, and a mixed `Pair` array reports two `E10030` at one span
naming two owners.

Dedupe at one span: when the same judgment fires twice for one expression, one report carries
the rejection. For the cross-site pair (`E10036` at the callee's return, `E10033` at the
caller's store), the phase measures both sites: where the store report is a consequence of the
callee judgment already reported, withhold the consequence (the row's "misleading second-site"
wording); where both are independent judgments at different sites, keep both and pin the count.
The semantics lens reviews the boundary. The immutable pins — `callable_release_integrity`'s
`should_reject_a_mixed_conditional_view_return` (exactly one `E10030`) and the
`conditional_projection_residue` one-report cases — stay green.

### 2. The generated-owner phrase (`DEF-283`)

The deferred mixed view return prints `borrow of '__cl_owned1' does not live long enough`
(probe-confirmed): the deferred owner's display bypasses the generated-owner phrasing the
`checker_conformance` suite pins for the sibling shape.

Route the deferred owner's display through the existing generated-owner phrase
(`a compiler-generated owner`) exactly as the direct spelling does — the phrase helper is
`own/liveness.rs` `display` (:594); the owner's internal name is never user-visible. The pinned
phrase test keeps holding.

### 3. The conditional-iterable move mark (`DEF-307`, AR-9)

`for (borrow x in (c ? b1 : b2))` with owned elements reports
`E10020: 'b1' was moved and cannot be used` at a later use (probe-confirmed with `String[2]`;
the `u8[4]` twin is accepted), although the batch-I phase-5 correction made the emission read
the arm in place. The checker's move mark is the stale half.

Align `mark_read_conditional_move` with the in-place route: a whole-array conditional iterable
marks no arm as moved, so a later use stays accepted; the element/projection spellings keep
their marks where the emission moves. The pinned `fixediterowned` behavior and the re-executed
loop controls stay green.

## Integration Points

- `own/liveness.rs`: the generated-owner escape report and its display helper.
- `own/statements.rs`: `check_return` (:1253) — the local-provenance/one-report boundary.
- `own/place.rs`: the per-backing walk (`collect_view_owners` :1028, `view_is_mixed` :1207).
- `own/expr.rs`: `mark_read_conditional_move` (:353) and the element-store marks.
- The report count/span helpers the batch-I phases used.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| Two reports at one span | One report carries the rejection | AR-7 |
| A downstream consequence report while the callee judgment reports | Withheld; the callee report stands | AR-7 |
| Independent judgments at different sites | Both kept (measured); the count pinned | AR-7 |
| A deferred compiler-generated owner | The pinned phrase renders | — |
| A later use of a conditional-iterable arm | Accepted (the emission reads in place) | AR-9 |
| An element/projection spelling that still moves | Keeps its mark and its `E10020` | — |

## Testing Requirements

- Spec: the nested mixed return (exactly one report, the code and span asserted), the
  `gfieldcall` and mixed-`Pair`-array instances (the measured counts), the local-only and
  parameter controls (unchanged counts), the deferred-owner phrase case (asserts
  `a compiler-generated owner` and never the internal name), and the conditional-iterable later
  use (accepted with owned elements; the `u8` twin; run exit 0, ASan-clean). The
  `callable_release_integrity` pin and the `checker_conformance` phrase test run before and
  after.
- Impl: the report counts and spans, the displayed phrase, the mark shape, and the re-emission
  determinism pins.
