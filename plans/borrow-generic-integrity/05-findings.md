# Findings Ledger: Borrow/Generic Integrity

Independent per-phase review record. One compact row per ruling batch; the full
evidence lives in the reviewers' reports and in the dispositions below.

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| SR-001, MS-001 | 1 | CRITICAL | `own/instance_check.rs` nullable exemption too broad | fixed |
| RV-001, SR-004 | 1 | MAJOR | `own/expr.rs` mixed-conditional duplicate `E10034` | fixed |
| MS-003 (re-review) | 1 | CRITICAL | read-only-source nullable slot exemption | fixed |
| SR-002, RV-002 | 1 | MAJOR (pre-existing) | generic whole-binding self-assign through a lend slot | deferred — filed DEF-100 |
| SR-003 | 1 | MINOR | out-slot self-assign on non-droppable slot types | accepted — root-based rule as confirmed; the projected clause depends on it |
| RV-003 | 1 | MINOR | out-slot shape impl test is a base-green guard | accepted — the loop-count test is the red pin |
| RV-004 | 1 | MINOR (pre-existing) | all-borrow conditional reports two diagnostics at one site | accepted — base-identical; folded into the AR-13 refinement reasoning |
| RV-005 | 1 | MINOR (pre-existing) | field store into a non-nullable borrow parameter | accepted — field stores keep their existing rules by design |
| MS-002 | 1 | MAJOR (pre-existing) | concrete `borrow T?` return double-frees | deferred — filed DEF-101 |
| PF-005 residual | 1 | MINOR (pre-existing) | runtime-indexed out-slot element self-assign | deferred — filed DEF-102 |
| EI-P2-1 | 2 | MINOR (newly reachable) | `type_key` `Q` marker vs identifier collision in nested positions | accepted — documented limitation, filed DEF-103 |
| Lifted-closure capture return | 2 | MAJOR (pre-existing) | emitted alias between a captured owner and its returned value | deferred — filed DEF-104 |
| SR-001 | 3 | MAJOR | `sema/ops.rs` null pairing matched any unknown operand | fixed |
| PRE-1 | 3 | MAJOR (pre-existing) | unguarded element store through a possibly-null view | deferred — filed DEF-105 |
| PRE-2 | 3 | MAJOR (pre-existing) | constant-index slice store missing its bounds trap | deferred — filed DEF-106 |
| PRE-001 | 3 | MAJOR (pre-existing) | `str`/`static str` pair equality emits invalid C | deferred — filed DEF-107 |
| PRE-002 | 3 | MINOR (pre-existing) | inferred-null local emits into an integer declaration | deferred — filed DEF-108 |
| Field-rooted narrowing conservatism | 3 | note | an element write through a narrowed field view keeps the read-back rejected | accepted — conservative, mirrors the pre-existing field-write behavior |
| SR-002 / RV-001 | 3 | MINOR (re-review) | the nullable-unknown proxy also matches a binding initialized from `null` | accepted — the predicate doc states the exact rule; tied to DEF-108 |
| RV-002 | 3 | MINOR (pre-existing) | untyped operand versus `str`/`cstring` views emits invalid C | accepted — folded into DEF-107 |

## Phase 1 dispositions (2026-10-09)

**SR-001 / MS-001 (CRITICAL) — fixed.** The nullable exemption in
`check_not_lend_requirements` was unconditional, so a generic `let`, `return`,
or `take` move of a nullable-substituted lend was accepted; the return shape
newly double-freed (the concrete `borrow T?` return has a pre-existing release
bug, filed DEF-101). Fix: the duty now records whether the move destination is
a writable parameter slot (`RequiresNotLend.stores_into_slot`), and the audit
exempts a nullable substitution only for those duties. Probes confirm the
generic `let`/`return`/`take` forms are rejected again with one `E10034` while
the nullable out-slot store stays accepted and ASan-clean. Pinned by two new
specification cases (owned local, returned lend).

**RV-001 / SR-004 (MAJOR) — fixed.** `lend_moves_judged` skipped the
target-side judgment only when every right-side move source was an already
judged lend move, so a conditional mixing a local arm with a lend arm produced
two `E10034` at one site (base produced one). Fix: the skip applies when any
source is judged. Pinned by the mixed-conditional clause of the both-lends
specification case.

**MS-003 (CRITICAL, scoped re-review) — fixed.** The first fix gated the
nullable exemption on the destination alone, so a read-only source
(`put<T>(borrow slot: T, v: T)` at `T = String?`) was still exempted and the
emitted store aliased the caller's value (ASan double-free); the concrete
spelling rejects it. Fix: the duty also records whether the source is a
writable lend (`RequiresNotLend.writable_source`), and the exemption requires
both conditions. Probes: read-only and `const` sources rejected with one
`E10034`; the both-borrow and `take`-source stores stay accepted and
ASan-clean. Pinned by ST-23. Per the phase-review budget cap this follow-on
fix was not re-reviewed a third time; it is covered by the new specification
case, the lattice probes, and `make check`.

**SR-002 / RV-002 (MAJOR, pre-existing) — deferred, DEF-100.** The exact
self-assign through a parameter-typed lend root (`copySelf<T>(borrow slot: T)`)
is unguarded at base and head; at `T = String` it double-frees. Filing it as a
defect keeps the phase's confirmed scope; expanding it would need the self
assigned rule extended to parameter-typed roots with a per-instance duty.

**SR-003 (MINOR) — accepted.** The confirmed rule rejects an exact self-assign
on an out-slot root regardless of the slot's element or pointee type; the
`T[]?` projected clause depends on that root-based shape. Narrowing it to
droppable slot types is recorded as a possible future refinement, not a phase
change.

**MS-002 (MAJOR, pre-existing) — deferred, DEF-101.** The concrete
`borrow String?` return emits a non-emptying return and double-frees; base and
head emission is byte-identical apart from `#line`. The generic form stays
rejected (SR-001 fix), so this residual is unreachable through the new rules.

**PF-005 residual — deferred, DEF-102.** The runtime-indexed element
self-assign (`xs[i] = xs[i]`) stays unguarded; the exact constant-projection
form is covered.

## Phase 2 dispositions (2026-10-09)

**EI-P2-1 (MINOR, newly reachable) — accepted, filed DEF-103.** The `Q` marker
collides with a user identifier ending in `Q` in nested instance positions: a
type named `StringQ` used as `Wrap<StringQ>` keys the same `ClWrap__StringQ` as
`Wrap<String?>`, and the strict build fails on the struct redefinition. The
collision is fail-closed (never a runnable wrong program), needs an unusual
type name, and the marker spelling is pinned by the confirmed AR-4 decision;
the correctness reviewer recorded it as the known limitation the `type_key` doc
and the nested-key implementation test already document. An escaping scheme
would change internal C symbol spellings — an ABI-affecting choice outside this
batch's confirmed scope. Filed so the backlog carries it.

**Pre-existing lifted-closure capture double free — deferred, DEF-104.**
An immediately-invoked closure returning its captured owner by value
(`func outer<T>(x: T): T { return ([x] () => x)(); }`) aliases the result and
the source, so both release one allocation (`free(): double free detected`,
exit 134; base and head identical, and it reproduces without a nullable type).
The emission auditor surfaced it during the sweep and routed it here as
pre-existing, outside this phase's symbol-key scope; filed with the
reproduction.

No other findings in Phase 2: the correctness review reported none, the
memory audit found no leak, double free, or release-count mistake on the
changed shapes, and the emission sweep over 1837 units found only the intended
`Q`-class diffs (14 diffs, all in the batch programs; 7 base-fail-to-head-ok
strict builds; zero head-only failures over 760 accepted host units; no
dangling symbol references).

## Phase 3 dispositions (2026-10-09)

**SR-001 (MAJOR, phase-introduced) — fixed.** The view/`null` pairing keyed on
"unknown shape", which also matches a local declared without a type; `let x;
x = 5; if (x == xs)` with a slice view was accepted and emitted a C comparison
of an `int` and a slice struct, which the strict build rejects (the base
reported `E10222`). Fix: the pairing's null side is the null literal's type — an
unknown shape that is *nullable*; an unknown without the flag keeps the operand
rules. Pinned by a new rejection case.

**PRE-1 (MAJOR, pre-existing) — deferred, DEF-105.** An unguarded element store
through a possibly-null view dereferences NULL (`xs[0] = 1` on a null view emits
`((*xs)).ptr[0] = 1u;` with no test; SEGV, base and head identical).

**PRE-2 (MAJOR, pre-existing) — deferred, DEF-106.** A constant-index element
store into a slice emits no bounds trap, while the read path and the
runtime-index store path do (`E11211`); an empty non-null view store is an
out-of-bounds write (ASan, base and head identical).

**PRE-001 (MAJOR, pre-existing) — deferred, DEF-107.** `str` and `static str`
pair equality is accepted and emits a C comparison of `ClStr` structs, which
the strict build rejects; `cstring` comparisons work (pointer equality).

**PRE-002 (MINOR, pre-existing) — deferred, DEF-108.** An inferred-null local
emits into an integer declaration (`let x = null;` → `i32 x = NULL;`), which
the strict build rejects; the phase's acceptance stops an unrelated diagnostic
from masking it in one spelling.

**Field-rooted narrowing conservatism (note) — accepted.** An element write
through a narrowed *field* view (`if (s.a != null) { s.a[0] = 1; use s.a }`)
keeps the read-back rejected because the field arm clears the field's narrowing
on any write whose first projection is that field — the pre-existing behavior
for handle fields, conservative in the safe direction, and outside the phase's
whole-binding narrowing case. Recorded, not filed.

The correctness reviewer reported no findings and the memory audit found no
in-scope defect (every newly accepted guarded store releases exactly once on
the normal, early-return, loop, break/continue, and both-branch paths).

### Phase 3 re-review (2026-10-09)

The single scoped re-review confirmed **SR-001 resolved**: the untyped-local
counterexample and its operand-order mirrors report `E10222` on the view side
exactly once and write no translation unit; the intended acceptances are
unchanged (both orders, nullable and non-nullable views) and `str`/`static
str`/`cstring`, view pairs, view-versus-non-view, and handle/raw emissions are
byte-identical to the phase head; the new assertion is proven red at the phase
head and green after; `make check` is green.

Two minor residuals were ruled accepted and recorded:

- **SR-002 / RV-001 (MINOR, re-review):** the nullable-unknown proxy also
  matches a local initialized from `null`, which pairs with a view and is
  accepted. The program is already unbuildable at that point (DEF-108: the
  declaration itself emits an invalid integer initialization), the handle path
  has the same pre-existing acceptance, and tightening the pairing to the
  literal expression would change `binary_result`'s signature for a construct
  whose real defect is the binding's type. The predicate's documentation now
  states the exact rule; the binding's type and emission are DEF-108's work.
- **RV-002 (MINOR, pre-existing):** an untyped operand compared with a
  `str`/`cstring` view is accepted by the operand rules and emits invalid C at
  base and head; folded into DEF-107's evidence.
