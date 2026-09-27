# Spec Amendment: Non-Lexical Lifetimes

> **Document**: 03-03-spec-amendment.md
> **Parent**: [Index](00-index.md)

## Overview

This document owns the exact frozen-specification edits the plan is authorized
to make (AR-2). The chapters currently describe v1 as the implemented model and
name NLL as the future target; after this plan the implementation is v2, so the
chapters must describe last-use regions. The wording below is applied verbatim
and reviewed before the commit that lands it (AR-2).

No other golden changes: every other compile-fail fixture keeps a view used
after its conflict, so it still rejects under v2 (AR-11).

## Chapter edits

### `spec/16-borrow-rules.md`

| Location | Current | Replacement |
| -------- | ------- | ----------- |
| §1 table, `Origin` (`:22`) | "The region in which a loan is valid; v1 origins are lexical scopes, while the NLL target uses last-use regions" | "The region in which a loan is valid: from its creation to its last use on any path (a last-use region, RFC 2094)" |
| §1 table, `Live` (`:23`) | "A loan is live from its creation until its last use inside its origin; v1 approximates this as "until the end of the origin's scope"" | "A loan is live at a point when one of its uses is reachable from that point and its creation precedes it" |
| §2, implicit widening (`:54-55`) | "The loan is live until the end of the derived view's origin." | "The loan ends at the last use of the derived view or any alias of it." |
| §6 step 3, `Liveness` (`:221-224`) | "Backward dataflow computes the loans live at every point. v1 approximates liveness as "from creation to the end of the lexical origin"; the NLL target computes last use instead." | "Backward dataflow computes, for every loan, the positions from which one of its uses is reachable. A loan is live at a position when its creation precedes the position and a use is still reachable. Definite assignment is tracked alongside, so an uninitialized place is never lent (E10021)." |
| §6 closing paragraph (`:237-241`) | "**v1 is a conservative lexical approximation.** It uses lexical origins, the simplest sound model, and rejects programs the later checker will accept. The approximation never accepts a program that NLL (RFC 2094) would reject, and it never changes the source-level contract: a program that compiles under v1 keeps compiling when the checker gains precision (lineage section 6)." | "**v2 computes last-use regions.** A loan is live until its last use on any path, so the checker accepts the "unused later" cases the lexical approximation rejected. Loan granularity remains whole-owner (O8), and a `defer` containing control flow is analyzed conservatively, so precision is bounded by those rules rather than by a lexical origin. The source-level contract does not change: every program the earlier lexical checker accepted stays accepted (lineage section 6)." |
| §7 table, "unused later" row (`:253`) | "An exclusive loan while a shared loan is live, even if unused later \| rejected in v1 (E10032); accepted by v2 NLL" | "An exclusive loan while a shared loan is live, even if unused later \| accepted — the shared loan's last use ends its region" |
| §7 `E10022` example (`:256-270`) | the `sink(s)` call is after the view's last use (`len(view)`) and is commented `E10022` | rewrite the example so the move precedes a later `observe(view)` (still `E10022`), and add the accepted variant where the move follows the last use |
| §8 phase table (`:306-310`) | v1 lexical, v2 "Non-lexical lifetimes: last-use regions and full NLL (RFC 2094)", v3 Polonius | mark v2 as the implemented model (a third column "Status": v1 *superseded*, v2 *implemented*, v3 *future*) |
| §10 table (`:331-333`) | the "Non-lexical lifetimes \| v2, the target model" row | remove the row; NLL is no longer out of scope |

### `spec/lineage.md`

| Location | Current | Replacement |
| -------- | ------- | ----------- |
| §6 (`:97-104`) | "The target model is NLL (O11). **v1 implements a conservative lexical approximation**: a borrow lives to the end of its scope and the checker rejects some programs Rust would accept. …" | "The checker computes last-use regions (O11): a borrow lives until its last use on any path, at the language's whole-owner loan granularity. It accepts the programs the earlier lexical approximation rejected, and the source-level contract did not change, so no syntax is frozen around the old weakness." |
| §10 known seams (`:169-175`) | the seam list includes "the conservative pre-NLL approximation (section 6)" | drop the pre-NLL seam; the copy-on-lend and elision seams stay |

### `spec/decisions.md`

| Location | Current | Replacement |
| -------- | ------- | ----------- |
| row `O11` (`:89`) | "…borrows escape only when parameter-derived; v1 is lexical, non-lexical checking comes later" | "…borrows escape only when parameter-derived; the checker computes non-lexical last-use regions (RFC 2094)" |

### `spec/03-ownership.md`

| Location | Current | Replacement |
| -------- | ------- | ----------- |
| `:229` | "lexical model are specified in [Ch 16](16-borrow-rules.md)" | "borrow model are specified in [Ch 16](16-borrow-rules.md)" |

`spec/appendix-stdlib.md:66` ("no reallocation while a view is live") stays: it
is a statement about the runtime rule, not about the lexical model.

## Golden rewrite (AR-2, AR-11)

`spec/tests/compile-fail/E10031_view_widening_mutation.clo` is the one fixture
whose program NLL accepts. It keeps its code and module by adding a use after
the mutation:

```clorraine
// expect-error E10031
// trigger: mutating a String while a widened str view of it is still used
module tests.compile_fail.E10031;

import { stringOf } from std.text;

func observe(v: str): void {}

func demo(): void {
    let s: String = stringOf("hello");
    let v: str = s;            // String widens to str: a shared loan of s
    s = stringOf("world");     // E10031: the loan is live; observe(v) still uses it
    observe(v);
}

func main(): i32 {
    return 0;
}
```

The accepted form of the same program (the view's last use before the
mutation) is covered by ST-2 in `07-testing-strategy.md`, not by a golden.

## Review requirement

The amended chapter wording is shown in the commit that lands it (AR-2). A
mismatch between the amended text and the code is a Phase 2 failure, not a
golden edit: the spec is the oracle (AR-11).

## Testing Requirements

The amendment is verified by `make check`: `spec/check.sh` re-reads the
chapters and runs the goldens, and the rewritten golden must still report
`E10031` (ST-5 and ST-15 in `07-testing-strategy.md` pin the behavior).
