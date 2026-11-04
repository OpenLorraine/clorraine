# Chapter 16 - Borrow-Checker Rules

> **Status:** stable (v1.0)
> **Depends on:** Ch 03 (ownership), Ch 06 (functions and parameters), Ch 07 (structs), Ch 14 (diagnostics), Ch 17 (codegen) *(informational)*
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 3.1 (parameter modes), 4 (types), 6.4 (call-site markers)
> **Ledger:** O2, O8-O12, F2, F9, T6, T9, T12; [lineage.md](lineage.md) sections 4 (lifetime elision) and 6 (borrow checking)

This chapter defines how the compiler proves the borrow contract of
[Ch 03](03-ownership.md): many readers XOR one writer, loans never outlive
their owners, and views escape only through parameters. The `BR-*` identifiers
are the rule index that diagnostics and tests cite (ledger section 1). Message
text lives in [Ch 14](14-diagnostics.md); this chapter references codes only.

## 1. Model

The checker reasons over a small, explicit model.

| Concept | Definition |
|---|---|
| **Place** | A memory location: a local, a parameter, a struct field, an array element, or a temporary |
| **Loan** | A permission to access a place, created by a lend: a read-only lend parameter, a `borrow` parameter, a view derivation, or a reborrow |
| **Origin** | The region in which a loan is valid: from its creation to its last use on any path (a last-use region, RFC 2094) |
| **Live** | A loan is live at a point when one of its uses is reachable from that point and its creation precedes it |
| **Access** | A read, a write, a move, or another loan of a place |
| **Conflict** | Two accesses at one point that the kind of a live loan forbids |

A **shared loan** permits any number of reads; an **exclusive loan** permits no
other access. Loans come only from lends (section 2); there is no borrow
expression to write. Field borrows are allowed and tracked conservatively as
loans of the whole owner (O8). Moves, moved values, and partial moves are owned
by [Ch 03](03-ownership.md); the checker adds the loan constraints on top
(BR-4, section 5).

## 2. Borrowing through parameter modes

Clorraine has no borrow operator. There is no `&`, no `&mut`, and no
`borrow <place>` expression. A borrow is a **parameter mode** or a **view
derivation**:

| Declaration | Loan created | Access through it |
|---|---|---|
| `x: T` | shared loan of the argument's place | read-only |
| `const x: T` | shared loan, explicit spelling | read-only |
| `borrow x: T` | exclusive loan of the argument's place | read and write |
| `take x: T` | no loan; ownership moves into the callee | - |

View types (`T[]`, `str`, `cstring`, `raw T`) carry reads. Passing a view is a
shared loan of whatever storage the view names. Passing a view where the callee
must write is a reborrow (section 3).

**Implicit `String` -> `str` widening is a loan.** Turning a `String` into a
`str` implicitly — for example when passing it to a `str` parameter, returning
it from a `str`-returning helper, or binding a local `str` view — creates a
shared loan of that `String`. The loan ends at the last use of the derived view
or any alias of it. Mutating, moving, or reallocating the `String` while the view is
live is E10031 (mutation while a shared loan is live) or E10022 (move while a
loan is live).

**Helpers that return a view or `cstring` over owned storage are loans.** A
standard-library helper such as `bytes`, `toCString`, `slice`, or `bufferBytes`
returns a view or `cstring` into storage the owner still owns, so the call
creates a shared loan of that owner for the result's origin. The same E10031
and E10022 consequences apply while the result is live
([appendix-stdlib.md](appendix-stdlib.md) section 11).

**There are no local view expressions** (O9). A binding holds a value; a view
value exists only because it arrived from a view-typed parameter, a view-typed
return, or a standard-library helper such as `slice`, `bytes`, or
`bufferBytes`. Naming a local never creates a view.

Call-site markers (`const x`, `borrow x`, `take x`) are optional assertions
(V6). They do not create a loan by themselves; they must match the declaration,
and a mismatch is E10039 (see [Ch 06](06-functions.md)).

```clorraine
struct Entity {
    kind: u8;
    x: i32;
    y: i32;
}

func nudge(borrow e: Entity): void {
    e.x = e.x + 1;              // writable lend of the caller's record
}

func demo(): i32 {
    let e = Entity { kind: 0, x: 1, y: 2 };
    nudge(e);                   // an exclusive loan for this call only
    return e.x;                 // the loan is over; the owner is usable again
}
```

A lend of a duplicable value binds a copy (V9, V5). Writes through a `borrow`
parameter of a duplicable type therefore affect that copy only, and W10001
marks such a writable lend as needless; it is a warning, not an error (D9).
Read-only lends of duplicables copy silently. The warning keeps generic code
that spells `borrow T` valid for `T = u8`. A view lend is not a
value lend: the view itself copies, but the storage it points into is shared,
so a `borrow` view parameter and an element view (`for (borrow e in xs)`) write
through to the owner.

An out-parameter is an **in-out ownership slot**: the caller must initialize
the slot before the call (passing an uninitialized slot is E10021), the callee
owns the incoming value on entry and may release it, and the write leaves the
slot owning the new handle. Writes through the out-parameter drop the
callee-owned previous value (the O3 carve-out in
[Ch 03](03-ownership.md)). E10034 does not apply to releasing or replacing the
slot's own incoming value, but it still applies to storing a different lend
parameter's value or view, which would outlive that call-scoped lend
([Ch 06](06-functions.md) section 4 states the full rule).

## 3. Reborrowing (O12)

A **reborrow** is a loan derived from another loan. It is how a view parameter,
or a view derived from a borrow parameter, is passed on without moving
ownership. The O12 rules are:

* A **shared reborrow** is a read-only view derived from a loan. It adds one
  more reader, and it is valid while the underlying loan is live.
* An **exclusive reborrow** is a writable view derived from a writable loan.
  It is limited to the expression that creates it, so the outer loan is usable
  again immediately after.

This is the rule that makes `borrow bufferBytes(buf)` valid when `buf` is
itself a `borrow` parameter: the call creates an exclusive reborrow, and the
reborrow ends when the call returns.

```clorraine
import { File, readFile } from std.io;
import { Buffer, bufferBytes } from std.mem;

func loadFrom(borrow f: File, borrow buf: Buffer): void {
    readFile(f, borrow bufferBytes(buf));
    // bufferBytes(buf) derives a writable view from a writable borrow:
    // an exclusive reborrow, limited to this call expression (O12).
}
```

A writable view derived from a read-only loan is E10041. A reborrow that
overlaps another live access to the same place is E10032. A view parameter
passed to another call is a reborrow limited to that call (EL-5). Markers at
the call site remain optional assertions (V6, E10039).

## 4. Lifetime elision

The language has no lifetime annotation syntax (D2), so elision decides every
view lifetime. The rules, restated from [lineage.md](lineage.md) section 4:

| Rule | Statement |
|---|---|
| EL-1 | Every view-like parameter (view, handle, `raw`, `cstring`) carries a separate elided lifetime |
| EL-2 | A function that returns a view type must derive it from **exactly one** view-like lend parameter |
| EL-3 | Zero lending candidates, or two or more, with a view return is an error; annotations do not exist to break the tie |
| EL-4 | `static str` returns do not participate: static storage needs no input |
| EL-5 | A view parameter passed on to another call is a reborrow (O12), limited to the call expression |
| EL-6 | Nullable views add `null` as a value; the lifetime rule is unchanged |

A `take` parameter is **not** an elision candidate: it creates no loan
(section 2), so it cannot be the source of a returned view. A view return whose
only view-like lend candidate is a `take` parameter is E10036.

| Situation | Diagnostic |
|---|---|
| Zero or two-or-more lend candidates for a view return | E10036 |
| A view derived from a local owner | E10035 |
| A loan that leaves its origin (field, global, or local return) | E10033 |

```clorraine
func head(buf: u8[], n: usize): u8[] {
    return slice(buf, 0, n);      // EL-2: exactly one view-like lend, buf
}

func pick(a: u8[], b: u8[]): u8[] {
    return a;                     // E10036: two candidates, no way to choose
}

func greeting(): static str {
    return "hello";               // EL-4: static storage, no candidate needed
}
```

## 5. Rules

The `BR-*` rules are the checker's obligations. The codes below are the
ownership and borrow codes of [Ch 14](14-diagnostics.md) section 3.

| Rule | Statement | Diagnostic |
|---|---|---|
| BR-1 | A loan's origin must lie within its owner's origin; a loan never outlives its owner | E10030 |
| BR-2 | A live shared loan permits only reads and shared reborrows at the borrowed place | E10031 |
| BR-3 | A live exclusive loan permits no other access to the borrowed place | E10032 |
| BR-4 | A move of the owner requires that no loan of it is live | E10022 |
| BR-5 | A lend parameter is call-scoped; the callee may not release or store it, except the `borrow T?` slot's own incoming value, which the callee owns on entry and may release; a different lend's value or view is E10034 | E10034 |
| BR-6 | A returned view must derive from a parameter, not from a local | E10035 |
| BR-7 | A view return must derive from exactly one view-like lend parameter | E10036 |
| BR-8 | A loan must not escape its origin: no storage in a field or a global | E10033 |
| BR-9 | A global may not be borrowed, and a global that would need an exit drop is rejected | E10038 / E11106 |
| BR-10 | A reborrow is a loan derived from a loan: a read-only view derived from a loan is a shared reborrow, and a writable view derived from a writable loan is an exclusive reborrow limited to the expression (O12). A writable view derived from a read-only loan is an error | E10041 |

Related access rules live in [Ch 03](03-ownership.md) and
[Ch 07](07-structs.md) and are enforced by the same pass:

| Situation | Diagnostic |
|---|---|
| A read, move, or loan of a moved value | E10020 |
| A read before the binding is definitely assigned | E10021 |
| A whole-value use of a partially moved value | E10025 |
| A partial move outside a top-level field of an owned local or `take` parameter | E10026 |
| A call-site marker that contradicts the declaration | E10039 |
| An element loop over non-duplicable elements without `borrow` | E10040 |
| Mutation, move, or reallocation of a `String` while a derived `str` view is live | E10031 / E10022 |
| A writable lend of a duplicable value (writes affect only the copy) | W10001 (warning) |

## 6. Checker pipeline

The checker runs on the typed control-flow IR of [Ch 17](17-codegen.md)
section 2, not on the syntax tree. The IR is built from `assign`, `move`,
`borrow`, `call`, `drop`, `return`, `branch`, and `switch` statements.

1. **Lower to CFG.** Each function becomes basic blocks of IR statements.
2. **Collect loans.** Every lend parameter, view derivation, reborrow, and
   call-site marker yields a loan record `(place, kind, origin)`.
3. **Liveness.** Backward dataflow computes, for every loan, the positions from
   which one of its uses is reachable. A loan is live at a position when its
   creation precedes the position and a use is still reachable. Definite
   assignment is tracked alongside, so an uninitialized place is never lent
   (E10021).
4. **Constraints.** Origin containment (`origin(loan)` within
   `origin(owner)`) and the call constraints for lend parameters.
5. **Conflicts.** At every access, check consistency with the live loans using
   BR-2, BR-3, and BR-4; report E10031, E10032, or E10022.
6. **Escape analysis.** Reject loans that leave their origin: a returned local
   view (E10035), storage in a field or global (E10033), a borrow of a global
   (E10038).
7. **Drop legality.** A use of a loan must be reachable from the loan's
   creation without passing the owner's release; a use that every path reaches
   only through the release reads freed storage and is rejected (E10030). This
   is the static check behind the cleanup ladder of [Ch 17](17-codegen.md)
   section 4.

**v2 computes last-use regions.** A loan is live until its last use on any
path, so the checker accepts the "unused later" cases the lexical
approximation rejected. Loan granularity remains whole-owner (O8), and a
`defer` containing control flow is analyzed conservatively, so precision is
bounded by those rules rather than by a lexical origin. The source-level
contract does not change: every program the earlier lexical checker accepted
stays accepted (lineage section 6).

## 7. Accepted and rejected examples

| Example | Verdict |
|---|---|
| Borrow a local, use it, and let the loan end at the call | accepted |
| Return a view derived from exactly one lend parameter | accepted |
| Two view-like lends with a view return | rejected (E10036) |
| Move an owner while a loan of it is live | rejected (E10022) |
| Return a view of a local owner | rejected (E10035) |
| Reborrow through a `borrow` parameter while calling a function | accepted (O12) |
| An exclusive loan while a shared loan is live, even if unused later | accepted — the shared loan's last use ends its region |
| Writable lend of a duplicable value | warning (W10001) |

```clorraine
import { String, stringOf, bytes } from std.text;

func observe(view: u8[]): void {
}

func sink(take s: String): void {
}

func rejected(): void {
    let s = stringOf("hello");
    let view = bytes(s);            // a shared loan of s
    sink(s);                        // E10022: observe(view) still uses the loan
    observe(view);
}

func accepted(): void {
    let s = stringOf("hello");
    let view = bytes(s);            // a shared loan of s
    observe(view);                  // the loan's last use
    sink(s);                        // accepted: the loan ended at observe(view)
}
```

```clorraine
import { String, stringOf, bytes } from std.text;

func bad(): u8[] {
    let s = stringOf("hello");      // a local owner
    return bytes(s);                // E10035: the view would outlive its owner
}
```

```clorraine
struct Point { x: i32; y: i32; }

func bump(borrow p: Point): void {
    p.x = p.x + 1;
}

func demo(): i32 {
    let p = Point { x: 1, y: 2 };
    bump(p);                        // W10001: the write lands on the copy
    return p.x;                     // still 1
}
```

The "unused later" row is the well-known lexical-versus-non-lexical gap. v1
kept the loan live to the end of the origin and rejected the program; v2 ends
it at its last use (section 8).

## 8. Relationship to Rust's checker

The rule set has the same shape as Rust's loans/origins/liveness/conflicts
model, minus the features Clorraine does not have: no reference counting, no
interior mutability, no threads, and no `Send`/`Sync` (D6, D7, C1). The plan is
staged:

| Phase | Model | Status |
|---|---|---|
| v1 | Lexical origins; the conservative approximation in this chapter | superseded |
| v2 | Non-lexical lifetimes: last-use regions and full NLL (RFC 2094) | implemented |
| v3 | A Polonius-style alias-based formulation if the NLL implementation needs it | future |

Each phase is internal. The source contract of [Ch 03](03-ownership.md) does
not change, so no syntax is frozen around the v1 weakness.

## 9. Verification

* **Unit tests on hand-built CFGs**: one accept and one reject case per `BR-*`
  rule, run directly on IR so the tests do not depend on parsing.
* **Golden compile-fail tests**: one source file per diagnostic code; the
  expected output is the code plus the first message line
  ([Ch 14](14-diagnostics.md) section 15). The `BR-*` identifiers index those
  tests.
* **Positive corpus**: idiomatic hosted programs must keep compiling as the
  checker gains precision. A new rejection on the corpus needs a documented
  decision, recorded as a ledger amendment.
* **Soundness stance**: when the checker cannot prove a loan safe, it rejects.
  Precision is added deliberately, never by default.

## 10. Out of scope for v1

| Feature | Reason |
|---|---|
| Struct lifetime parameters and view fields | `static str` fields only; other view fields are E10205 ([Ch 07](07-structs.md)) |
| Interior mutability | Not in the core language (D6) |
| `Send`/`Sync`-style task safety | Single-threaded language; OS tasking crosses through FFI (C1, D7) |
| Callback lifetime guarantees | `stores` and `transfers` contracts are deferred (E90003, X5) |
