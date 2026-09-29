# Requirements: Borrow-View Moves and Shadowed Narrowing

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The ownership checker accepts two programs the frozen specification forbids or
mishandles. Moving an element out of a `for (borrow e in xs)` view is accepted
even though the view never owns the element and element moves are limited in v1;
and a declaration that shadows a narrowed nullable binding reports a spurious
`E10010` because the shadowing binding is declared before the initializer's null
check runs. This plan makes the checker reject the first with the specification's
element-move diagnostic and accept the second.

The specification is the only oracle: `spec/03` sections 3 and 4 (element moves
are `E10026`; partial moves are limited), `spec/08` section 5 (the view never
owns its element), `spec/14` (the `E10010` and `E10026` rows), and `spec/16`
(borrow rules). No specification, grammar, or registry change is authorized
(AR-10).

## Functional Requirements

### Must Have

- [x] **R1 — Moving through a borrow element view is rejected (`DEF-18`).**
  An ownership transfer whose source is a `for (borrow e in xs)` element view —
  a `take` argument, an owned-local binding, a field path, an aggregate-literal
  field, or a return — reports `E10026` with the registry's message and help.
  (`spec/03` §3–4, `spec/08` §5; AR-3, AR-4, AR-7, AR-9)
- [x] **R2 — A narrowed shadowing initializer is accepted (`DEF-20`).**
  `if (b != null) { let b: Buffer = b; ... }` compiles: the initializer's name
  resolves to the outer binding, whose non-null narrowing holds. (`spec/02`,
  `spec/16`; AR-5)
- [x] **R3 — The `const` shadowing form is accepted too.** The same program with
  `const b: Buffer = b;` compiles for the same reason. (`spec/16`; AR-6)

### Should Have

- [x] **R4 — Guard behavior is unchanged.** Reads, lends, and writes through a
  borrow element view keep working; the copy-form loop binding keeps owning its
  copy; a move out of a `borrow` parameter keeps reporting `E10034`; a nullable
  use without a narrowing check keeps reporting `E10010`. (AR-13)

### Won't Have (Out of Scope)

- Supporting ownership transfers through a borrow view (the v1 element-move
  limit stands; AR-3).
- Any change to `spec/`, the grammar, or the diagnostics registry (AR-10).
- The other filed defects (`DEF-23` – `DEF-29`) and the backlog programs
  `PLAN-05`/`PLAN-13` (AR-12).

## Technical Requirements

### Performance

No runtime target: both changes are checker decisions and emit no code.

### Compatibility

All three profiles and both C toolchains keep working; only programs the
specification already rejects (R1) or the checker already misjudges (R2/R3) are
affected. `make check` and `make verify` cover the corpus and legs.

### Security

Not applicable: compiler-internal acceptance rules with no user input, secrets,
persistent data, or network surface (AR-11).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Defects in this plan | DEF-18 + DEF-20 / DEF-18 only / another cluster | Both | The second plan announced by the completed array-value plan's AR-10; both are ownership-checker conformance gaps | AR-1 |
| Rejection diagnostic | E10026 / E10034 / new code | `E10026` | `spec/03` §4: element moves are `E10026`; the registry row exists | AR-3, AR-7 |
| Rejection site | the shared `move_place` / each move site | Shared `move_place` | Every move form funnels there; the borrowed-parameter rule already lives there | AR-9 |
| Shadowing resolution | initializer resolves to the outer binding / other | Outer binding, before the declaration | Matches the file's own pre-declaration resolution for store preparation | AR-5 |
| `const` variant | include / separate | Include | Same root cause and declaration order | AR-6 |
| Oracle | change `spec/` / keep frozen | Keep frozen | The specification is the immutable oracle | AR-10 |
| Verification | `make check` + `make verify` / other | `make check` per task, `make verify` at close | Project standard from `AGENTS.md` | AR-8 |

> **Traceability:** Every scope decision must reference the Ambiguity Register
> entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] Every requirement R1–R4 traces to at least one passing specification
   test case in `borrow_view_and_shadowing_spec_test.rs` (R1: ST-1 – ST-5;
   R2: ST-9, ST-14; R3: ST-10; R4: ST-6 – ST-8, ST-11 – ST-13).
2. [x] `make check` passes with no warnings; `make verify` reports all legs
   passed.
3. [x] `DEF-18` and `DEF-20` are marked Done in `plans/00-roadmap.md` with a
   link to this plan.
4. [x] Documentation updated: `AGENTS.md` test map, roadmap rows.
