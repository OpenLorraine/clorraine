# Requirements: Template Placeholder Residuals

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Issue #27 records five template-placeholder residuals that reproduce on
`b6f12d8` after the #26 delivery. Each is either an untyped null value that
reaches the formatter, a conditional result that drops its nullable flag, a
zero-state `String` that the runtime treats as a pointer, or an ownership
cleanup/order path the ordered lowering does not cover.

The feature closes all five with the existing frozen rules. It adds no
language surface and no runtime entry point.

## Functional Requirements

### Must Have

- [x] **R1 — Null literal placeholder.** A placeholder whose value is
  null-shaped (`null`, or a conditional result with no concrete type) reports
  E10500 (`type 'null' is not formattable`) at the placeholder and writes no
  translation unit. A null literal inside a conditional with a view arm stays
  accepted as the empty view when the placeholder has no expected type; a
  written or resolved destination (including a generic instantiation) keeps the
  existing E10012 rule. A null arm with a handle sibling makes the conditional
  nullable and the placeholder reports E10010 at the null arm.
- [x] **R2 — Mixed-nullability conditional.** A conditional with two same-kind
  arms keeps the nullable flag when either non-null-shaped arm is nullable. An
  unnarrowed nullable arm in a placeholder reports E10010 at that arm; a
  narrowed arm is accepted. The same rule applies in a plain store
  (`let s: String = flag ? a : b;`).
- [x] **R3 — Zero-state String handle.** The all-zero `String` handle is the
  empty String. A placeholder over a zero-state field (`H {}`) and a direct
  `println(h.name)`, `len(h.name)`, and `clone(h.name)` print/return empty
  without a crash on host; the hosted-amiga shim mirrors the same null-safe
  helpers and builds.
- [x] **R4 — Discarded template.** A discarded owned template result is
  released at the end of its statement, including in a `for` step; a loop with
  a discarded step template completes with no leak. A discarded owning call in
  a step is released by the same drain.
- [x] **R5 — Owning aggregate literal order.** An owning aggregate field before
  a later template placeholder evaluates its own fields at its source position,
  before the later copy. `P { inner: Inner { a: mkString() }, b: $"{mark(2)}" }`
  yields `seq=12`, and loop re-entry does not double-free the adopted aggregate.
- [x] **R6 — No language surface or ABI change.** No `spec/` edit; the String
  helper signatures, the emitted C ABI, and the template part contract are
  unchanged. The zero-state read is the empty-state reading the issue requests
  (AR-3).
- [x] **R7 — Gate and budget.** `make check`, `make asan`, `make ubsan`,
  `make verify`, and `make bench-shape` are green; every scored benchmark
  output is equal and no scored metric regresses by more than 1% against
  `b6f12d8`.

### Should Have

- [x] **R8 — Diagnostic consistency.** The null placeholder uses the frozen
  E10500 message/help; the mixed conditional and the null arm at a handle
  sibling use the frozen E10010 message/help and name the nullable arm's root,
  call, or `value`.
- [x] **R9 — Emission clarity.** The adopted aggregate is a `cl_zero` plus a
  fieldwise store, or a plain temporary store, in the emitted C; no invalid
  aggregate assignment and no `cl_text_unknown` for a supported program.

### Won't Have (Out of Scope)

- `text(null)` and any other explicit-converter formattability gap (AR-10).
- #24 (`text` arity) and the other deferred issues (AR-14).
- A new diagnostic code, IR value kind, runtime helper, or formatting
  framework.
- Any edit to `spec/`, `stdlib/`, or the emitted C ABI.
- A general null-flow analysis of zero-state struct fields.

## Technical Requirements

### Performance

- The three String accessor checks are the only added runtime work. The
  `strings` benchmark and every other scored output are measured with
  `make bench-compare`.
- If a scored metric exceeds 1%, first inline one NULL test in the generated
  `cl_string_view` while keeping all three accessors safe, then re-measure. If
  it is still above 1%, stop and seek a ruling; removing an accessor check is
  not allowed because `len` and `clone` call the accessors directly (AR-22).

### Compatibility

- Preserving conditional nullability tightens only unsound programs: an
  unnarrowed non-null-shaped nullable arm at a non-nullable destination is now
  E10010; a null arm at a handle sibling makes the result nullable and its use
  E10010 (an annotated non-null handle keeps E10012); every narrowed
  concrete or generic case stays accepted.
- The zero-state runtime change makes a previously crashing valid program
  work; it does not change a valid program's output.
- `host`, `hosted-amiga`, `hosted-amiga-13`, and `freestanding` keep their
  behavior. Freestanding rejects templates at sema (E11200); the String
  helpers are host/Amiga only.

### Security

- No new input, parser, FFI, or `unsafe` surface. The runtime checks are
  internal and null-safe; no user data or secrets are involved (AR-20).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
|----------|--------------------|--------|-----------|--------|
| Null placeholder | E10500 / E10012 / type as `str` | E10500 | The frozen placeholder formattability rule; a handle null arm settles to `T?` and reports E10010 | AR-1, AR-23 |
| Mixed conditional | preserve nullable / template-only / reject all | preserve nullable from non-null-shaped arms | TY-6; valid narrowed programs stay legal; view null stays empty | AR-2 |
| Zero state | null-safe accessors / reject | null-safe | O7 and STR-7, and the issue's empty-state fix direction | AR-3 |
| Discarded template | bind + drain / emitter free / leave | bind + drain | Covers templates and owning calls | AR-4 |
| Aggregate order | adopted fieldwise store / whole literal / leave | adopted fieldwise store | Reuses the existing store core with a no-drop caller | AR-5 |
| `text(null)` | include / follow-up | follow-up | Different conversion surface | AR-10 |
| Spec edit | yes / no | no | The rules already require the behavior | AR-13 |

> **Traceability:** every scope decision cites its owning Ambiguity Register
> entry; the register holds the full delegated provenance.

## Acceptance Criteria

1. [x] All requirements R1–R9 are verified by the ST cases in
   [07-testing-strategy.md](07-testing-strategy.md).
2. [x] The five #27 reproductions behave as specified under the host build and
   under AddressSanitizer.
3. [x] `make check`, `make asan`, `make ubsan`, `make verify`,
   `make bench-shape`, and `make bench` + `make bench-compare` are green.
4. [x] All scored benchmark outputs are equal; no scored metric regresses by
   more than 1%.
5. [x] No critical or major review finding remains open.
6. [x] The findings ledger, the roadmap row, and issue #27 reflect the
   delivered evidence.
