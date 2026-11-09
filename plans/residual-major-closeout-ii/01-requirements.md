# Requirements: Residual Major Closeout II (Batch L)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Complete the 23 remaining un-parked 🟠 major defect rows named by AR-1, severity-first (the six
older ruling-parked rows and the minors stay out, AR-2): eight phases grouped by machinery, 21
red rows implemented with red → green cases and two rows (`DEF-341`, `DEF-350`) verified
incidentally fixed at the planning head and pinned as controls. Every row was re-probed against
the compiler built from head `8829be6` (AR-3). Each requirement below is conformance to the
frozen specification: the fix makes an accepted program build and run with the language's
semantics, stops an invalid-C acceptance, or makes a runtime ownership failure impossible. No
new language surface, no new diagnostic code, no specification or registry edit (AR-12). The
behavior choices the rows left open are resolved inside the frozen rules: `DEF-322` reports the
frozen `E10221` place rejection (the per-arm route is parked; AR-6, preflight PF-001), `DEF-347`
casts at the mutable raw destination (AR-7), `DEF-357` defers the extent fold to the C type size
(AR-13).

## Functional Requirements

### Must Have

- [x] **R1 — Nullable-view arms and initializer view stores** (`DEF-335`, `DEF-336`, `DEF-337`):
  a nullable-owned conditional at a view destination guards each arm's nullness at every
  widening position — the conditional arm (`conditional_arm`), the call argument, and the
  initializer store member projection — so a null selection carries the destination's zero view
  instead of `cl_string_view(NULL)`, and every position builds strictly and runs without a null
  dereference.
- [x] **R2 — The walk's projected-argument resolver** (`DEF-338`, `DEF-340`, `DEF-342`): the
  monomorphization walk records the instance of a call whose projected argument roots at a
  local-headed qualified receiver, a statement temporary, or a conditional/non-local place base,
  using one shared resolver that follows the same projections the emitter follows; the emitted C
  declares and calls the keyed instance.
- [x] **R3 — Instance substitution and element surfaces** (`DEF-339`, `DEF-345`, `DEF-353`,
  plus the `DEF-341` control): an element store through a borrow element drops the overwritten
  owner at the substituted destination; a substituted element used in a concrete conversion or
  member position is judged per instance (accepted where the concrete twin is accepted, frozen
  rejection where it is not); a duplicable `borrow out` instance keeps its write-through store;
  the capture pair stays rejected with the frozen `E10401` at both spellings.
- [x] **R4 — Ownership transfer judgments** (`DEF-315`, `DEF-329`): a view-like fixed-array copy
  transfers (or judges) its element loans exactly as the single-view twin does, so a later
  element store into the copy reports the shared-loan conflict instead of writing freed storage;
  an owned read of a raw pointee routes through the ordinary owned read's move/duplicate
  judgment, so a second owner is created only when the language allows one.
- [x] **R5 — Store emission: conditional bases and guarded view literals** (`DEF-322`,
  `DEF-323`): an element store through a conditional or projected base reports the frozen
  `E10221` place rejection exactly as the scalar member twin does (Ch 04 §4 / EX-14; the per-arm
  route is parked, AR-6/PF-001), so the store is never silently dropped and the RHS side effects
  never run; a guarded conditional of call-filled view-array literals materializes like its
  scalar/owned twins, so the `take` spelling passes the hidden destinations and the plain
  spelling evaluates only the taken arm.
- [x] **R6 — Call conversions and `ffi` typing** (`DEF-344`, `DEF-351`, plus the `DEF-350`
  control): an inferred call whose argument is null-shaped while a sibling argument fixes the
  parameter to a concrete non-nullable type reports the frozen null rule (the count rule settled
  per site); a substituted destination at a concrete `cstring`/`str`/`raw` conversion position —
  the return, the argument, the explicit-type-argument route, and the caller's own type parameter
  as an explicit argument — reports the frozen `E10200` once per site while the legal instances
  stay accepted; the `ffi` view/slice result keeps its fixed ABI-pointer typing with the
  spellings pinned as controls.
- [x] **R7 — Member typing on concrete paths** (`DEF-346`, `DEF-347`, `DEF-348`): an unknown
  member on a concrete base (a fixed array's `.ptr`/`.len`, an unknown view member) reports the
  frozen `E10212` instead of emitting invalid C; a view `.ptr` read at a mutable raw destination
  emits the sanctioned member cast (the AR-7 fallback is the conversion duty); a `raw E` member
  read routes through the enum rule so the concrete and instance paths agree.
- [x] **R8 — Module/global identity and the const folder** (`DEF-356`, `DEF-357`, `DEF-359`):
  an aliased import of a func-typed global and a qualified value-head path (the cross-module
  struct-global field) resolve the global's own identity and emit its symbol; the
  self-qualified and plain spellings stay accepted controls; a `sizeof` used in a type-level
  extent defers to the C type size so the host layout (padding, view size) is honored; the
  checker types a bare func-global call through the binding the scope resolves, so the argument
  conversion reports instead of the emission failing.

### Won't Have (Out of Scope)

- `DEF-167`, `DEF-169`, `DEF-171`, `DEF-190`, `DEF-191`, `DEF-263` — the six older rows parked
  for user rulings (AR-2): a non-callable-callee diagnostic code, an escaping type-key scheme,
  staged `embed`, a nameless nullable source diagnostic, the `unsafe` exemption, and the folded
  fixed-array `.len` receiver evaluation.
- The minors (the `DEF-311`–`DEF-359` subset not named by AR-1, including `DEF-352`'s raw-pointer
  `.ptr`/`.len` arrow shape and the `len`-surface folds): they form the batch after this one
  (AR-2); `DEF-350` and `DEF-341` are carried as controls only.
- Any new diagnostic code, registry row, specification chapter, or golden (AR-12); a row that
  turns out to need one is parked mid-phase with the evidence (AR-14).
- New language surface: no conversion is added to the frozen table (AR-7), no store route beyond
  the frozen place rule is added (AR-6), and no layout engine beyond the C `sizeof` deferral
  (AR-13).

## Technical Requirements

### Compatibility

- The host, hosted-amiga, and freestanding profiles stay conformant; every accepted program
  keeps building under `-std=c99 -Wall -Wextra -Werror` on the pinned container.
- Byte identity wherever a phase does not change the shape (the emission differential classifies
  every delta); the pinned origin suites and the batch-I/J/K oracles stay green.

### Security

- Not applicable beyond the compiler's own input validation (unchanged).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Batch scope | 23 remaining majors / parked rows / minors | the 23 majors | Severity-first; fresh evidence; the origin machineries group cleanly | AR-1, AR-2 |
| Phase grouping | eight machinery phases / one per row / origin-batch phases | eight machinery phases | Coherent review surfaces; dependency order P1 → P8 | AR-4 |
| `DEF-322` | per-arm route / frozen rejection | frozen rejection | The frozen place rule's closed list (Ch 04 §4 / EX-14) and the scalar member twin's `E10221`; the route would add an acceptance the list does not define (preflight PF-001) | AR-6 |
| `DEF-347` | destination cast / conversion duty | destination cast, stop-gated | The sanctioned member-conversion shape; no new conversion | AR-7 |
| `DEF-357` | defer the extent fold / layout engine | defer to the C type size | The value position already defers; smaller fix | AR-13 |
| `DEF-341`/`DEF-350` | red cases / controls | pinned controls | The probe shows the filed failure fixed; the residuals are other rows | AR-3, AR-8 |
| Test placement | origin suites / new pair | origin suites, additions only | The machinery is the origin batches'; a new pair only if a case has no natural home | AR-9 |

> **Traceability:** Every scope decision references the Ambiguity Register entry that resolved
> it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] Every one of the 21 red rows is closed with a reproduction that flips (red before, green
   after) and a suite pin that fails without the fix; `DEF-341` and `DEF-350` stay fixed as
   pinned controls (AR-3, AR-5).
2. [x] Every ownership/handle fix is ASan-clean on the host (plain and `RDC_ASAN=1`), with loop
   and early-exit variants where the shape is re-executed; every store-dropping fix reads the
   stored value back.
3. [x] The strict build passes for every accepted shape in scope on the pinned container (the
   `ffi` externs compile-only with their recorded stubs where applicable); the freestanding and
   hosted-amiga legs stay green for their rows.
4. [x] No behavior change outside the in-scope rows; the origin suites and the batch-H/I/J/K
   oracles stay green.
5. [x] `make check` green on every phase; `make verify` green at close-out.
6. [x] The roadmap's 23 rows are accounted for: 21 Done with per-row evidence and 2 recorded as
   incidentally fixed with their control pins; the six older parked rows stay open (AR-2); the
   minors stay untouched.
7. [x] No spec, registry, or golden changes; the new cases append to the origin suites only
   (AR-9); the `AGENTS.md` test map updates at close-out only if its wording needs the batch
   mention.
