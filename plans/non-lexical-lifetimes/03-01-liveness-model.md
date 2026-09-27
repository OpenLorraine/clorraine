# Liveness Model: Non-Lexical Lifetimes

> **Document**: 03-01-liveness-model.md
> **Parent**: [Index](00-index.md)

## Overview

This document defines the v2 loan model: what a live loan is, how regions are
computed, and how the four affected rules are decided. It is the single owner of
the semantic decisions; `03-02` owns how the compiler implements them.

The model is RFC 2094 applied to Clorraine's surface: loans come only from
lends (view derivations, lend parameters, and standard-library view helpers),
there is no borrow expression, and there are no lifetime annotations
(`spec/16-borrow-rules.md:34-73`). Non-lexical lifetimes therefore change one
thing only: **when a loan is live**.

## Concepts

| Concept | Definition |
| ------- | ---------- |
| **Position** | A point in one function's emitted CFG: a block and an ordinal inside that block's event sequence |
| **Event** | One recorded fact at a position: a loan creation, a loan use, an owner access, a loan request, or an owner release |
| **Loan** | One lend of an owner, identified by a `LoanId`; its kind is `Shared` or `Exclusive` (`compiler/src/borrow.rs:20-25`), and it carries the owner binding, a span, its creation position, and its use positions |
| **Region** | The set of positions at which the loan is live |
| **Use** | Any access through a view value that carries the loan (AR-6) |
| **Owner release** | A position where an owner's storage stops existing: a scope exit, a return cleanup, or a `break`/`continue` cleanup |

### Ordered events (AR-7)

Events are appended in the order the walk evaluates them, so inside one
statement a read that is evaluated before a store is also recorded before it.
Two recorded events never share a position: within a block the ordinal
increases by one per recorded event. A deferred block's events are recorded on
each edge where the block runs, in the order the block executes them (see
Deferred blocks).

### The position graph

The liveness pass builds a directed graph over positions:

* `(b, i) → (b, i + 1)` for consecutive events in one block;
* the last event of `b` reaches the first event of every successor block of
  `b`, skipping blocks that recorded no events (a visited set makes the skip
  terminate on cycles of empty blocks).

Successor blocks come from the block's terminator: `Goto`, both arms of
`Branch`, every `Switch` arm and its `default` (never the `join` block, which
other terminators reach), and none for `Return` or `Unreachable`.
Reachability is reflexive and transitive: a position reaches itself.

### Region definition (AR-3)

A loan is **live at a position `p`** iff both hold:

1. the loan's creation position reaches `p`; and
2. `p` reaches one of the loan's use positions.

A loan with no uses has an empty region (it is never live), which is the
central precision gain over v1: a view that is bound and never used no longer
blocks anything.

## Loan identity and aliases (AR-5)

Every view value carries a loan id:

* Binding a view value whose storage comes from a local or a parameter
  (`Provenance::Local`/`Param`) **creates** a loan when the value carries no
  loan yet — for example `bytes(s)`, `viewOf(s)`, or an implicit
  `String` → `str` widening of an owner.
* Binding a view whose value already carries a loan **shares** that loan — a
  copy (`let w = a;`) or a derivation (`slice(a, ...)`, `a.field`, `a[i]`).
* Passing, returning, or reading through any alias extends the shared loan's
  region, so all aliases have one region, the RFC 2094 behavior for a copied
  reference.

The owner of a loan is always the storage owner the provenance chain reaches,
never an intermediate view binding, so release checks compare against the real
owner (AR-8).

The loans a view binding carries are flow-sensitive: the carried set lives in
the flow state, so when two paths give the binding different owners, the set
after the join holds both loans and a use extends both regions. A binding that
is rebound to a value that carries no loan (a literal, a global, or a fresh
unowned value) carries none.

## Uses and accesses (AR-6)

| Event | Recorded when | Consumed by |
| ----- | ------------- | ----------- |
| `Created` | a view binding creates a loan | region start |
| `Used` | a view value is read, written through, derived from, passed as an argument, or returned | region extent |
| `Access { access: Write }` | a binding that owns storage is written | `E10031`, `E10022` |
| `Access { access: Move }` | a binding that owns storage is moved (whole or partial) | `E10022` |
| `Request` | a persistent loan is created, or a `borrow` argument lends an owner | `E10032` |
| `Release` | scope exit, return cleanup, or `break`/`continue` cleanup | `E10030` |

Lending an owner to a call is a **new call-scoped loan**, not a use of a
persistent loan. Two `borrow` arguments for one owner in the same call are
still a duplicate-lend error and stay an inline check (03-02).

### Deferred blocks

Everything a `defer` block does happens on every exit edge where that block
runs. The exit-edge check re-runs the block against a discarded builder clone,
so the walk places every event the block records at the real block the edge
leaves, in the order the block's statements record them, instead of at a clone
position a later real block could reuse. The owner releases on that edge are
recorded after the block's events, so an access inside the defer precedes the
release on the same edge. The rule covers every event kind, not only uses: an
owner access, a loan creation, or a release inside a defer is placed the same
way. For a straight-line defer this is exact; for one containing control flow
it is conservative (the event may happen on only some paths).

## Rules decided by regions

The conditions below preserve the observable rule set of v1 and only narrow
when a rejection fires (R6, AR-16):

| Rule | Event checked | Condition for the diagnostic |
| ---- | ------------- | ---------------------------- |
| `E10022` move while borrowed (`spec/14-diagnostics.md:53`) | `Access { access: Move }` for owner `o` | any loan of `o` is live at the event |
| `E10031` mutation while shared-borrowed (`spec/14-diagnostics.md:59`) | `Access { access: Write }` for owner `o` | any **shared** loan of `o` is live at the event |
| `E10032` conflicting loans (`spec/14-diagnostics.md:60`) | `Request` for owner `o`, kind `k` | any loan of `o` that is live at the event has `kind == Exclusive`, or `k == Exclusive` |
| `E10030` loan outlives owner (`spec/14-diagnostics.md:58`) | every use of a loan of `o` | the use has no path from the loan's creation that avoids a release of `o`; reported once per loan, at the loan's creation span |

`E10031` keeps its v1 condition exactly: only a **shared** live loan blocks a
write, so this plan adds no rejection v1 did not have. The same holds for
`E10032` (the existing `Loans::conflict` excludes nothing that was allowed:
`compiler/src/borrow.rs:93-98`).

`E10030` unifies origin containment and drop legality: a use reachable only
through an owner-release point is a use after the owner's storage died on every
path to it, which is both "outlives its owner" and "releasing an owner while a
loan of it is live" (`spec/16-borrow-rules.md:225-235`). Requiring the
release-free path (rather than any release before a use) keeps a loop-local
owner accepted: the next iteration's use is reached from a fresh creation
before the release, so it stays valid, and every program the lexical checker
accepted stays accepted.

## Worked behaviors

| Program shape | v1 | v2 |
| ------------- | -- | -- |
| `let v = bytes(s); observe(v); sink(s);` | `E10022` | accepted — `sink` is after the last use |
| `let v = bytes(s); sink(s); observe(v);` | `E10022` | `E10022` — a use is reachable from the move |
| `let v = bytes(s); observe(v); s = stringOf("y");` | `E10031` | accepted |
| `let v = bytes(s); use(v); reset(s);` (`borrow` argument) | `E10032` | accepted — no use after the request |
| `let v = bytes(s); if (c) { observe(v); } sink(s);` | `E10022` | accepted — after the `if`, no use is reachable |
| `while (...) { observe(v); s = stringOf("y"); }` | `E10031` | `E10031` — the next iteration's use is reachable through the back edge |
| `let held: str; { let s = ...; held = s; observe(held); }` | `E10030` | accepted — every use precedes the release |
| `let held: str; { let s = ...; held = s; } observe(held);` | `E10030` | `E10030` |

## What does not change

* Loan granularity: whole-owner loans; field borrows remain conservative (O8,
  AR-1).
* Escape rules: `E10033`, `E10035`, `E10036`, `E10038` and elision are
  untouched (`compiler/src/own/place.rs:232-244,544-574`,
  `compiler/src/own/statements.rs:592-626`).
* Move, initialization, and partial-move rules: `E10020`, `E10021`, `E10025`,
  `E10026` are untouched.
* Call-scoped lends and reborrows (O12), including the duplicate-lend check.
* Diagnostic text, help, and spans (AR-22); only the programs they fire on
  narrow.
* Runtime behavior of accepted programs; no emitted-C change.

## Error Handling

| Error case | Handling | AR Ref |
| ---------- | -------- | ------ |
| A conflict is found at a post-pass event | report with the v1 message/help at the event span, in event order | AR-9 |
| A loan is released with a later use | report `E10030` once per loan at its creation span | AR-8 |
| A use site is missed | forbidden by the hook enumeration in 03-02 and the ST coverage in 07 | AR-6 |
| An event position falls outside the finished function | impossible by construction: the log is saved/truncated around speculative passes and lambda lifting (03-02) | AR-4 |

## Testing Requirements

The specification test cases live in `07-testing-strategy.md` (ST-1 – ST-16);
the implementation tests exercise the region rules directly and build and run
the accepted programs.
