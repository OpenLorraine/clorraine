# Requirements: Array Value Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The compiler accepts four kinds of fixed-array programs and then either rejects
them wrongly, emits C that cannot compile, or computes the wrong value. This
plan makes every accepted fixed-array *value* program compile to valid C with
specification-correct behavior: iteration over multi-extent arrays and their
slices, copy and move of whole arrays, projection of array-returning calls, and
`len` of multi-extent arrays and owned `String`s.

The specification is the only oracle: `spec/08` AR-3 and section 5 (rows are
the elements of `T[N][M]`; LOOP-1/2/3), `spec/03` section 3 and V8/V9 (copies
and moves), `spec/02` section 9 (implicit widenings, `String` to `str`),
`spec/04` sections 9 and 11 (indexing, `len`), and `spec/14` (diagnostic
registry). No specification or registry change is authorized (AR-11).

## Functional Requirements

### Must Have

- [x] **R1 — Multi-extent element loops bind the row (DEF-15).** For a fixed
  `T[N][M]...` or a slice `T[][M]...` held in a place (local, parameter, or
  field), a non-`borrow` element loop binds each row by copy and a `borrow`
  element loop binds an exclusive row view; the iteration count is the
  outermost extent (fixed) or the runtime outer length (slice);
  non-duplicable rows without `borrow` report E10040. (`spec/08` AR-3,
  section 5; AR-1, AR-6)
- [x] **R2 — Fixed-array value assignment is valid C (DEF-22).** Copying or
  moving a fixed array in an initializer, a reassignment, a field, or a row
  selection emits a whole-array byte copy; a move additionally zeroes the
  source; a reassignment drops the old target value first; a call
  right-hand side is written into a compiler temporary first and then moved;
  a same-place right-hand side emits no copy. (`spec/03` section
  3, O3, V8/V9; AR-2, AR-4)
- [x] **R3 — Self-referencing array assignment stays well-defined.** A
  non-call right-hand side that reads the target follows the existing save
  path extended to arrays; a call right-hand side is evaluated into a compiler
  temporary before the move, so the callee never reads and writes the target
  storage at once (AR-7).
- [x] **R4 — Projections over array-returning calls materialize once
  (DEF-21).** `mk()[i]` and its row/slice forms evaluate the call once into a
  compiler temporary, then read or lend the element; moving an element out of
  an array in every move form (initializer, assignment and field-target
  assignment, aggregate-literal field, `take` argument, return) stays E10026
  in v1. (`spec/03` section 4; `spec/04` section 9; AR-5)
- [x] **R5 — Length is correct for arrays and `String` (DEF-19).** `len` of a
  fixed multi-extent array yields the outermost element count; `len` of an
  owned `String` yields its byte count through the implicit `String` to `str`
  widening; `len` of `str`, `static str`, slices, single-extent fixed arrays,
  and the growable `Array<T>` is unchanged. (`spec/02` section 9; `spec/04`
  sections 9 and 11; AR-5, AR-6)

### Should Have

- [x] **R6 — No regressions.** Single-extent element loops, slice element
  loops, growable arrays, `str` views, hidden out-parameter array returns,
  array drops, and every existing diagnostic keep their current behavior.

### Won't Have (Out of Scope)

- DEF-18 (a move through a `for (borrow e in xs)` element view) and DEF-20
  (the spurious E10010 under shadowing) — a second plan (AR-10).
- Moving elements out of arrays (`a[0]`, `mk()[0]`, a `take` argument) — v1
  limits these to E10026 (AR-5).
- Statement-temporary lifetime: a materialized array-returning call that owns
  elements is not released by this plan (the existing scalar `Materialize`
  temporaries have the same property); a follow-up defect tracks it (`DEF-23`),
  and the ASan probes run with leak detection disabled.
- Element loops over array-returning calls (`for (x in mk())`) — R1 covers
  place-rooted iterables only; a follow-up defect tracks the call form
  (`DEF-24`).
- Any change to `spec/`, the grammar, or the diagnostics registry (AR-11).
- New compiler layers, dependencies, test harnesses, or generalized
  abstractions (AR-4).

## Technical Requirements

### Performance

No runtime target. Array copy/move lowers to one `memcpy` per array value,
which is the minimal correct instruction sequence; code emission shape gains
at most one temporary per projected call result.

### Compatibility

All three profiles (`host`, `hosted-amiga`, `freestanding`) and both C
toolchains (`gcc`, `vbcc`) must keep working. All three profiles reach the C
runtime `memcpy`, and `spec/check.sh` plus `make check` / `make verify` cover
the existing corpus.

### Security

Not applicable: compiler-internal code generation with no user input, no
secrets, and no persistent data or network surface (AR-12).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Defects in this plan | array-value cluster / all remaining / element cluster / criticals only | DEF-15, DEF-22 (new), DEF-21, DEF-19 | The array-value defects share the assignment and materialization surface; the checker defects form a coherent second plan | AR-1 |
| New defect discovery | file and include / file only / fix unfiled | File DEF-22 and include it | The DEF-15 copy form cannot work before fixed-array assignment does | AR-2, AR-3 |
| Copy/move in C | byte copy / element-wise loops / struct wrapper | Whole-array byte copy through the C runtime `memcpy` | Smallest correct change; reuses `memcpy` and the existing drop paths (preflight PF-002) | AR-4 |
| Call-result projection | materialize + keep E10026 / support element moves / reject | Materialize the result, support reads and lends, keep element moves E10026 | Matches the existing `Materialize` machinery and `spec/03` section 4 | AR-5 |
| `len` of `T[N][M]` | outermost count N / total N×M | Outermost count N | Rows are the elements (`spec/08` AR-3); matches slicing and the slice path | AR-6 |
| Self-referencing assignment | extend save path / reject | Extend the save path to arrays | Follows O3 and the existing droppable-value handling | AR-7 |
| `spec/` and registry | change / keep frozen | Keep frozen | The specification is the immutable oracle | AR-11 |
| Verification | `make check` + `make verify` / other | `make check` per task, `make verify` at close | Project standard from `AGENTS.md` | AR-8 |

> **Traceability:** Every scope decision must reference the Ambiguity Register
> entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] Every requirement R1–R6 traces to at least one passing specification
   test case in `array_value_integrity_spec_test.rs`.
2. [x] `make check` passes with no warnings; `make verify` reports all legs
   passed.
3. [x] AddressSanitizer probes over the new copy/move/projection programs
   report no errors in the configured run (leak detection disabled); the
   statement-temporary lifetime limitation is recorded (`DEF-23`).
4. [x] `DEF-22` is filed in `plans/00-roadmap.md` and the rows for `DEF-15`,
   `DEF-19`, `DEF-21`, and `DEF-22` link to this plan.
5. [x] Documentation updated: `AGENTS.md` test map, roadmap rows, register
   closure note.
