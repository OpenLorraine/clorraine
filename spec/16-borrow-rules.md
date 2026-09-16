# Chapter 16 - Borrow-Checker Rules

> **Status:** draft
> **Depends on:** Ch 03 (ownership), Ch 14 (diagnostics)
> **Implementation phases:** lexical (v1), non-lexical (v2), Polonius-style (v3)

## 1. Model

The checker reasons over a small, explicit model:

| Concept | Definition |
|---|---|
| **Place** | A memory location: local, parameter, field, array element, temporary |
| **Loan** | A borrow expression: `borrow` (shared) or `borrow mut` (exclusive) of a place |
| **Origin** | The region a loan belongs to; v1 origins are lexical scopes |
| **Live** | A loan is live from its creation until its last use in the origin (v1: until the end of the origin's scope) |
| **Access** | Read, write, move, or another borrow of a place |
| **Conflict** | Two accesses inconsistent with a live loan's kind |

The v1 checker is deliberately conservative: it uses lexical origins, the
simplest sound model, and rejects programs the later non-lexical checker will
accept. Source-level contracts (Ch 03) do not change between phases.

## 2. Rules

| Rule | Statement | Diagnostic |
|---|---|---|
| BR-1 | A loan's origin must be within its owner's origin | E10030 |
| BR-2 | A live shared loan permits only reads and reborrows at the borrowed place | E10031 |
| BR-3 | A live exclusive loan permits no other access to the borrowed place | E10032 |
| BR-4 | A move of the owner requires that no loan of it is live | E10022 |
| BR-5 | A borrow parameter is live for the call only; it may not be released or stored | E10034 |
| BR-6 | A returned borrow must originate from a parameter | E10035 |
| BR-7 | Aggregates may not carry borrow-typed fields (v1) | E10036 |
| BR-8 | Globals may not be borrowed (v1) | E10038 |
| BR-9 | A use of a place after its owner's release point is rejected | E10201 |
| BR-10 | Reborrows: `borrow mut` may reborrow from `borrow mut` only; both weaken to `borrow` | E10200 |

## 3. Checker pipeline

The checker runs on the typed IR (Ch 17 §2), not on the AST:

1. **Lower** each function to a control-flow graph of basic blocks with
   `assign`, `move`, `borrow`, `call`, `drop`, and `return` statements.
2. **Collect loans**: every borrow statement creates a loan with
   `(place, kind, origin)`.
3. **Liveness**: for every program point, which loans are live (backward
   dataflow; v1 uses scope boundaries, v2 uses last-use points).
4. **Constraints**: origin containment (`origin(loan) within origin(owner)`)
   and call constraints for borrow parameters.
5. **Conflicts**: at every access, check consistency with live loans using
   BR-2/BR-3/BR-4.
6. **Escape analysis**: reject borrows that leave their origin (returns,
   assignments to fields or globals, storage in aggregates).
7. **Drop legality**: every reachable drop of an owned place precedes any
   loan use; releasing an owner with live loans is rejected.

## 4. Accepted / rejected examples (v1 lexical model)

| Example | Verdict |
|---|---|
| Borrow a local, use it, let its scope end before the owner's scope ends | accepted |
| Borrow a local in an `if` branch, owner declared in the enclosing scope | accepted |
| `borrow mut` after a shared borrow's scope has ended | accepted |
| `borrow mut` while a shared borrow's scope is still open, even if unused later | rejected (v1 conservative; accepted in v2) |
| Move an owner while a borrow's scope is still open | rejected (E10022) |
| Return a borrow of a parameter-derived buffer | accepted |
| Return a borrow of a local | rejected (E10035) |

The "unused later" case is the well-known lexical-vs-non-lexical gap; Phase 3
relaxes it by computing actual last uses.

## 5. Relationship to Rust's borrow checker

The rule set is the same shape as Rust's loans/origins/liveness/conflicts
model, minus the features Clorraine does not have in v1 (closures, traits,
generics, HRTBs). The planned v2 non-lexical algorithm follows the
alias-based formulation documented by the Polonius project; the v1 checker is
the lexical approximation, which is small enough to implement and test first.

## 6. Verification

* **Unit tests** on hand-built CFGs: one accept and one reject case per rule.
* **Golden compile-fail tests**: one source file per diagnostic code; the
  expected output is the code plus the first message line (Ch 14 §7).
* **Positive corpus**: idiomatic hosted programs must keep compiling as the
  checker gains precision; any new rejection on the corpus requires a
  documented decision.
* **Soundness stance**: when the checker cannot prove safety, it rejects.
  Precision is added deliberately, never by default.

## 7. Out of scope for v1

| Feature | Reason |
|---|---|
| Non-lexical lifetimes | v2 |
| Struct lifetime parameters | v2, needed before borrow fields |
| Interior mutability (`RefCell`-class) | Not in the core language; concurrency story comes later |
| Send/Sync-style task safety | Deferred with LorraineOS tasking |
| Callback lifetime guarantees (`stores`) | Ch 12 deferral |
