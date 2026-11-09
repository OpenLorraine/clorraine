# Requirements: Slice and Temporary Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Close seven filed compiler defects without changing the language: the multi-extent slice view type
model, the aggregate-literal consume fill/order gaps, non-direct-call owned roots and receivers,
and the invalid call-indexed element-store spelling. Every requirement is a correctness obligation
proven by a filed reproduction in `plans/00-roadmap.md` and the plan records of
`plans/def-backlog-closeout/`.

The fixes are compiler-internal. No new language surface, no new diagnostic code, and no
specification change is in scope; the DEF-70 behavior is already written in
`spec/08-arrays-strings.md:28`.

## Functional Requirements

### Must Have

- **R1 (DEF-70) — Row-typed slice views.** A slice whose source has more than one extent has
  `T[M]` rows as elements, per the frozen spec. A row read or move through the view, a nested
  access, and a row-typed parameter all use whole rows. Row elements inherit the existing element
  rules: the v1 partial-move limit keeps rejecting a concrete row move through a view (`E10026`),
  while the maybe-owning/generic store forms the defect filings use move the row out and empty the
  source slot. A generic row move through a borrow-backed view is a pre-existing escape recorded as
  a follow-up defect outside this plan. An explicit `slice<T>` whose `T` disagrees with the source's
  row reports the existing `E10200` (preflight PF-001).
- **R2 (DEF-72) — Impure index in a literal field.** An aggregate-literal field that reads a call
  result at a run-time index in the filed generic form (`Box<T> { v: make()[pick()] }`) evaluates
  the index call exactly once and binds the selected slot; for an owning element the slot moves and
  the call result's remaining storage releases. The behavior holds for duplicable and owning
  instantiations (preflight PF-009).
- **R3 (DEF-73) — Source-order field evaluation with an element projection.** An aggregate literal
  evaluates its fields strictly left to right (`spec/04-expressions.md` §3); a field that reads an
  element projection (`Box<T> { w: f(), v: make()[i] }` in the filed generic form), for duplicable
  and owning element types, runs `make()` only at that field's turn (preflight PF-009).
- **R4 (DEF-74) — Member-rooted consume releases the call result's siblings.** A field that
  consumes a member of a fresh call result (`Box { v: mk().a }`) empties the selected field and
  releases the remaining fields at statement end, while preserving R3's order.
- **R5 (DEF-71) — Non-direct-call take/consume roots release their siblings.** A take/consume whose
  root is a call with its own `take` argument (`sink(mkfrom(mka()).a)`) and a root that is a
  conditional of calls (`flag && sink((c ? mk1() : mk2()).a)`) each release the call result's
  remaining fields on every path.
- **R6 (DEF-89) — Owned member receivers are bound.** An owned call result read through a plain
  member (`return i32(spawn().count);`) is bound and released like a dot-`len` receiver; `len(...)`
  behavior is unchanged.
- **R7 (DEF-75) — Call-indexed element store emits valid C.** `out[target()] = mkvArray()[i]` and
  `out[target()] = Pair { ... }` build under the strict host C compiler with no pointer-arrow
  error and store the correct value, releasing the replaced element once.

### Should Have

- **R8 — Emission stability.** Programs that do not use the changed shapes emit byte-identical C.
  Multi-extent slice users are retyped (keys and typedefs change); programs whose behavior depends
  on the view element are wrong today, the rest keep their behavior. All emitted units stay
  strict-clean (`-Wall -Wextra -Werror`, the C99 subset without compound literals).

### Won't Have (Out of Scope)

- The other 17 open defects (defer semantics, borrow/generic keys and rules, lambdas, diagnostics,
  emission preamble): owned by the follow-up plans of the sequenced close-out (AR-1).
- Any new diagnostic code or specification edit (AR-3, AR-9).
- Expression-wide materialization of every owned call result (AR-5).
- Generated-C optimization work (gated on the backlog closing, PLAN-14).

## Technical Requirements

### Performance

No performance targets. The fixes must not add statement traffic beyond the existing deferred-fill
patterns; optimization remains gated on the backlog closing.

### Compatibility

- All three profiles (`host`, `hosted-amiga`, `freestanding`) must build and run the new programs
  where the existing suites cover them; `make verify` is the acceptance leg.
- The emitted C subset rules of `spec/17-codegen.md` hold (no compound literals, no empty structs,
  strict-clean units).
- Existing specification tests are the oracle and must stay untouched and green.

### Security

No user-input path changes; the compiler's own inputs are developer-authored programs. No secrets,
network, or persistence surfaces are touched. No security-specific tests are required beyond the
strict-build and sanitizer coverage.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Defect batch | slice+temporaries / defer family / all 24 | Slice + expression temporaries first | Contains the only critical defect; continues the expression-temporaries workstream | AR-1 |
| DEF-70 disposition | Row-typed views / typed rejection | Row-typed views | The frozen spec already mandates row elements; rejection would amend the oracle | AR-3 |
| Consume preparation | Deferred fill at the consuming point / eager operand store | Deferred | Eager store leaves mixed-literal order broken (R3) | AR-4 |
| Non-call roots and receivers | Extend existing binds / general owned-read walk | Extend existing binds | Smaller, reuses proven shapes | AR-5 |
| `slice<T>` disagreement | Existing `E10200` / new code | Existing `E10200` | Reported directly at the intrinsic call site; no new code, no spec change | AR-9 |
| DEF-75 spelling | Field-append only / both suffix sites | Both suffix sites | Keeps the handle-field and array-target pins intact while fixing the same precedence hazard | AR-8, AR-10 |

> **Traceability:** Every scope decision references the Ambiguity Register entry (AR #) that
> resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] R1 – R7 each carry specification tests that are red on the pre-fix tree and green after the
       fix, run under host AddressSanitizer where memory safety is at stake.
2. [x] No new leak, double free, use-after-free, wrong slot, order reversal, or invalid C in any
       phase's probe matrix.
3. [x] `make check` green after every phase; `make verify` green at plan completion.
4. [x] Base-vs-head emission sweep: no unintended byte diffs; every diff traces to a requirement.
5. [x] Phase reviews complete with no open critical/major finding; the affected implementation
       shape pins are migrated (R8).
6. [x] `AGENTS.md` test-naming map and the roadmap are updated.
