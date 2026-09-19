# Phase 5 Review: Ownership, Borrows, FFI Contracts, Generics

> **Document**: 13-phase5-review.md
> **Parent**: [Index](00-index.md)
> **Date**: 2026-09-19
> **Scope mode**: strict

## What was reviewed

The complete Phase 5 diff from the phase baseline tree
(`3e4b7a20a54c32114a3da594633de2e0063b0bff`) through task 5.3.2: the typed
control-flow IR (`ir.rs`), the ownership passes (`own.rs`, `own/`), the borrow
checker (`borrow.rs`), the FFI and unsafe checks (`ffi.rs`), the sema/driver
wiring that exposes checked expression types and runs the ownership stage after
a clean resolve and type pass, the AR-65 golden corrections, and the spec
drivers. Reviewers: an independent correctness phase-reviewer and an
independent memory-safety semantics auditor, both running `make check` and
exercising the built binary on adversarial inputs. No scope drift was found.

## Merged findings

Two reviewers' findings are merged onto one `RV-` id space; where both found
the same root cause the SA duplicate is noted.

| ID | Severity | Finding | Trigger |
|---|---|---|---|
| RV-300 | 🔴 CRITICAL | Loop bodies are not re-analyzed to a fixpoint, so a second-iteration use-after-move is accepted (SA-302: a `for` body that may run zero times is not merged either) | `let s = stringOf("x"); while (i < 3) { consume(s); i = i + 1; }` |
| RV-301 | 🔴 CRITICAL | Loans are recorded only when the owner is a local; a view derived from a `take`/`borrow` parameter can then be moved or mutated (SA-300) | `func f(take s: String) { let v: str = s; consume(s); }` |
| RV-302 | 🟠 MAJOR | A terminated branch is merged into the join, so the spec's own early-return pattern false-reports (`E10010`, and likewise `E10020`/`E10021`); the corpus file-copier uses this pattern throughout | `let f: File? = ...; if (f == null) { return; } use(f);` → `E10010` |
| RV-303 | 🟠 MAJOR | Reassigning a moved top-level field still reports `E10025`, contradicting the rule that a field write re-initializes the struct | `let l = p.left; p.left = stringOf("c");` |
| RV-304 | 🟠 MAJOR | `break`/`continue` are no-ops: a use after a loop that always breaks is a false `E10020`, and no cleanup is emitted on those leaving paths | `while (true) { if (c) { break; } consume(s); } consume(s);` |
| RV-305 | 🟠 MAJOR | `switch` has no per-clause or default-aware merge, so definite assignment misses and no `Switch` terminator is lowered (SA-301) | `let x: i32; switch (n) { case 1: x = 1; } return x;` |
| RV-306 | 🟠 MAJOR | A writable or out-parameter lend does not clear a caller place's narrowing | `if (f != null) { Reset(f); Close(f); }` where `Reset(borrow f: File?)` may null the slot |
| RV-307 | 🟠 MAJOR | `E10038` is reported for every global, including a `const static str`; the corpus file-copier's `IN_PATH`/`OUT_PATH` are rejected | `const NAME: static str = "x"; func demo() { sink(NAME); }` |
| RV-308 | 🟠 MAJOR | The lowered IR is never finalized or consumed and the monomorphization set is discarded; `own.rs`/`borrow.rs`/`ffi.rs` have no impl tests, so the drop planning and dataflow cannot be observed | `Builder::finish()` is never called; `own::check`'s return is dropped |
| RV-309 | 🟡 MINOR | A use-after-move argument reports `E10020` twice | `consume(s);` after a move |
| RV-310 | 🟡 MINOR | `expr_types` is keyed by the AST node address; a lookup miss silently degrades to `unknown`, which disables checks | latent |
| RV-311 | 🟡 MINOR | Exclusive lend arguments are never recorded as loans, so aliasing is accepted (`E10032`) (SA-313) | `two(s, s)` where `two(borrow a: String, borrow b: String)` |
| RV-312 | 🟠 MAJOR | Projected and call-derived view provenance is missing: a nullable field is not narrowed (SA-307), the stdlib view helpers do not run reborrow/conflict checks (SA-308), and a loan origin is not derived from a call or a projected place (SA-309/SA-314) | `use(h.f)` where `h.f: File?`; `bufferBytes(b)` on a read-only `b` |
| RV-313 | 🟠 MAJOR | Qualified member access through `raw` is not checked, and compound assignment on a `raw` pointer is not checked | `o.inner.x` where `inner: raw Inner`; `p += 4` with `p: raw u32` |
| RV-314 | 🟠 MAJOR | Lambda bodies are skipped, so an unsafe operation inside one is accepted; lambdas are staged, so the correct answer is the staged-feature diagnostic | `(func(): void { let x = *p; })` |
| RV-315 | 🟠 MAJOR | Partial initialization is not tracked per field (writing one field marks the whole binding assigned) and `defer` effects leak into the enclosing flow (SA-304/SA-303) | `let p: Pair; p.a = 1; return p.b;` |
| RV-316 | 🟠 MAJOR | Returning a `borrow` parameter is not checked (SA-306) | `func leak(borrow f: File): File { return f; }` |
| RV-317 | 🟡 MINOR | A narrowed nullable handle passed to an uncontracted parameter is accepted (SA-315) | `ff_narrow.clo` |

## Rulings

User ruling (2026-09-19): **fix all findings** — the 2 critical, all 12 major,
and the 4 minor. Each accepted fix is implemented with a regression test and
verified by `make check`, then the single permitted scoped re-review runs on the
fix diff. RV-302 and RV-307 are required for the validation corpus to compile.

## Fix evidence

All 18 findings were addressed with 47 new implementation tests; `make check`
is green (212 unit tests, 2 compile-fail driver tests, 21 stage driver tests,
81/81 asserted goldens). Highlights: loop bodies now reach a fixpoint
(RV-300), parameter-derived loans are recorded (RV-301), terminated branches no
longer poison the join (RV-302), field writes re-initialize (RV-303),
`break`/`continue` carry cleanup edges (RV-304), `switch` merges per clause and
lowers a terminator (RV-305), const globals may be lent (RV-307), and `own` now
returns the finished IR and the monomorphization set (RV-308).

Two notes recorded during verification:

- **RV-304 repro.** The reviewed loop now reports the in-loop use-after-move as
  well as the reachable outer use, because a move on any path counts as moved.
  The earlier outer-only false positive is gone; the program remains rejected
  for the correct reason.
- **RV-318 (found while verifying the fix batch, spec-required).** Raw indexing
  `p[i]` was typed as `raw T`; Ch 04 §9 makes it yield the element (a place of
  type `T`) and require `unsafe` (`E10602`). The type is corrected to the
  element. A related conservative behavior remains: a write through a raw
  pointee of a read-only parameter reports `E10024`. The specification is
  silent on that case, the v1 checker is allowed to reject programs a later
  checker accepts, and no corpus program uses raw/unsafe, so it is left as is
  and noted for a later ruling.

## Re-review of the fix diff

_(recorded after the single scoped re-review)_
