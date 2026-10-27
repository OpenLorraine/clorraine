# Batch A — Critical/Major Backlog

> **Document**: 03-A-critical-backlog.md
> **Parent**: [Index](00-index.md)
> **Rows**: 14

## Scope

The 14 backlog critical/major rows: the conditional-store and take integrity cases, the view-return rows, and the initializer/evaluation-order rows.

## Per-Row Specification

### DEF-544 — A plain (borrow) parameter moved into an owned conditional store double-frees (critical)

- **Observed:** 🔴 critical, pre-existing, surfaced by the Phase 2 review (SR-M-006) of the raw-store batch: an owned conditional store whose selected arm is a plain (borrow) parameter (`d = c ? p : q`) moves and empties the parameter, while the parameter's own epilogue still releases it; AddressSanitizer reports a double free.
- **Fix:** a plain parameter arm is a borrow, so the arm copies instead of moving (or the store rejects the move).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-545 — A take capture clears only the last of several consumed subranges (critical)

- **Observed:** 🔴 critical, pre-existing, surfaced by the Phase 2 re-review (P2): the capture keeps one `(offset,length)` pair, so two takes of one saved destination (`pair = (g && eat(take pair.first) > 0 && eat(take pair.sibling) > 0) ? x : y`) overwrite the range; only the last subrange is cleared and the save release frees the earlier consumed handles again (double-free).
- **Fix:** clear each consumed subrange at its own move-out, or accumulate multiple ranges per capture.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-546 — A concrete lend returned through a maybe-owning conditional is accepted and double-frees (critical)

- **Observed:** 🔴 critical, pre-existing, surfaced by the Phase 3 review (RV-004): a concrete borrow returned through a `T`-typed conditional is accepted at `T = String`; AddressSanitizer double-frees. The destination-aware duty currently covers the let/const/whole-assignment forms only.
- **Fix:** thread the destination into `judge_return_moves` and add the return owning/view cases.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-547 — Literal-element, conditional-take, and read maybe-target stores false-reject valid view instances (major)

- **Observed:** 🟠 major, pre-existing, surfaced by the Phase 3 review (RV-004): the destination-aware judgment is not wired into the literal field/element visitor, `check_argument_place`, or `mark_read_conditional_move`, so a `T = str` view instance reports E10034 while the concrete twin accepts.
- **Fix:** thread the destination through those paths.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-548 — A runtime-index StorePointer bypasses the struct-store path and emits invalid C (major)

- **Observed:** 🟠 major, pre-existing, surfaced by the Phase 3 emission review (EIA-2): a run-time-index store is lowered to a pointer store that renders its RHS with `store_text`, so a call-filled fixed-array field emits the call without its hidden destination.
- **Fix:** route the StorePointer literal through the struct/array store form.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-549 — A literal zero divisor is emitted and fails strict C (major)

- **Observed:** 🟠 major, pre-existing, surfaced by the Phase 3 emission review (EIA-4): `unsafe { (*h.p).n / 0; }` is accepted and emitted as a literal division, which the strict C build rejects.
- **Fix:** a front-end constant-division diagnostic.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-550 — A view-destination maybe-target store records no loan or escape duty (critical)

- **Observed:** 🔴 critical, pre-existing, surfaced by the Phase 3 review (RV-005): the store copies at a view destination but records no view loan, so a source mutation, returned view, or caller-slot store is accepted while the concrete twin reports E10031/E10035/E10033; AddressSanitizer reports heap-use-after-free.
- **Fix:** record the substituted view-destination loan and apply the escape rules.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-551 — Expression-form and nested-row array fills evaluate the fill once per element/row (major)

- **Observed:** 🟠 major, pre-existing, surfaced by the Phase 3 re-review (RR-05): the statement-form array fill now evaluates once into a local, but a fill in a guarded/expression position and a nested-row fill still render the evaluation per element or row.
- **Fix:** share the once-evaluated local with the expression and nested-row fill paths and add their cases.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-552 — A later order-mismatched argument is hoisted before an earlier argument's effects (major)

- **Observed:** 🟠 major, pre-existing evaluation order, surfaced by the Phase 3 re-review: `useTwo(a(), Outer { ... })` runs the hoisted literal before `a()` because the holding predicate does not pin earlier arguments when a later one needs a statement store.
- **Fix:** extend the pin-updates trigger to every later statement-store literal and add the cross-argument trace case.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-553 — A module-initializer store-needing literal leaks its owning fields (major)

- **Observed:** 🟠 major, pre-existing, surfaced by the Phase 3 memory re-review (P1): `materialize_init_literal` fills an init local but never registers it in `init_owners`, so the literal's owning fields leak.
- **Fix:** register the init local owner and add an LSan initializer case.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-554 — An owned String widened into a view slot is not released (major)

- **Observed:** 🟠 major, pre-existing, surfaced by the Phase 3 memory re-review (P2): `s = cl_string_view(stringOf("ef"))` drops the owned String without a release.
- **Fix:** materialize the owned source and release it after the view store.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-555 — A module-initializer nested owning call field evaluates before earlier source fields (major)

- **Observed:** 🟠 major, pre-existing source-order gap, surfaced by the Phase 3 memory re-review (P3): a nested owning call field in a module initializer binds during the walk before earlier source fields (`spec/04` §3).
- **Fix:** apply the source-order store rule to the initializer walk and add the trace case.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-556 — A generic T view return skips the lend-candidate count and lets a caller hold a dangling view (critical)

- **Observed:** 🔴 critical, pre-existing (base-identical; accepted at `02fa132` and the final head), found by the Phase 6 semantics review (PRE-1): `func f<T>(s: str, t: str): T { return t; }` called as `f<str>(a, b)` is accepted and builds; the emitted C frees `b` at the inner block end before `r[0]` reads it (ASAN heap-use-after-free, exit 63), while the concrete twin `func f(s: str, t: str): str` reports `E10036`. `elision_postlude` (`own/place.rs`) early-returns because a `T` return is not an elided view and records no instance candidate duty. Frozen EL-2/EL-3/BR-7.
- **Fix:** record a per-instance view-return candidate duty for a type-parameter return and apply the one-candidate rule at the instance.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-557 — A generic caller-returned static view is rejected through a borrow parameter (major)

- **Observed:** 🟠 major, pre-existing rejection with a plan-introduced concrete/generic divergence (the concrete twin became accepted at the DEF-506 head), found by the Phase 6 semantics review (SR-002) and reserved under AR-12/R-5: `func viewTag<T>(borrow h: T): str { return h.tag; }` over `struct H { tag: static str; }` reports one E10036 at the declaration while the concrete twin exits 0; `call_returns_static_view` (`own/place.rs`) scans with declaration-time types, so the caller cannot prove the static field through the parameter. The static-return proof needs an instance-bound marker (a cross-function summary surface, a new ruling) or the recorded conservative limitation.
- **Fix:** carry the static-return instance proof across the call.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.
