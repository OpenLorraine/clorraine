# Ownership and Borrows: Clorraine Compiler MVP

> **Document**: 03-05-ownership.md
> **Parent**: [Index](00-index.md)

## Overview

This layer implements Ch 03 (ownership), Ch 16 (borrow rules), the FFI
contract checks of Ch 12, and the generics obligations of Ch 06. It consumes
the typed AST and produces the **typed control-flow IR** that codegen
translates; the IR is the contract between the checker and the emitter, so the
two can never disagree about ownership (Ch 17 §2, `16-borrow-rules.md:213`).
The v1 checker is the conservative lexical approximation of `lineage.md:99-104`:
a loan lives to the end of its origin's scope, never accepting a program NLL
would reject.

## Typed Control-Flow IR (`ir.rs`)

The IR statements are fixed by Ch 17 §2 and Ch 16 §6: `assign`, `move`,
`borrow`, `call`, `drop`, `return`, `branch`, `switch`, `defer` (plus the
`call`/temporary bookkeeping). Every owned place carries an explicit `drop` on
every path that leaves its scope; `defer` entries join the ladder in the scope
that registered them. Places are locals, parameters, struct fields, array
elements, and temporaries.

Lowering (`sema.rs` → `ir.rs`) builds per-function CFGs from the checked AST.
The borrow checker then follows the Ch 16 §6 pipeline: collect loans
`(place, kind, origin)` → liveness → constraints (origin containment, call
constraints) → conflicts → escape analysis → drop legality. The IR is also the
unit-test surface: an impl test may snapshot the IR of a small program without
involving the C emitter.

## Definite Assignment (`own.rs`)

- A local must be assigned on every path before a read (F9, STMT-5); otherwise
  E10021. `let x: T;` without an initializer is legal (Ch 05 §? `05-statements.md:44-54`).
- Dataflow is a forward may-analysis over the CFG with conservative joins.
- `borrow T?` out-parameter slots follow Ch 06 §4: the caller must initialize
  the slot (E10021 if not); the callee owns the incoming value and may release
  it; a write through the slot drops the callee-owned previous value first
  (PF-001, the O3 carve-out).
- Reading `null`-narrowed values: nullability tracking is a second dataflow
  fact — a nullable place is narrowed only by an `== null` / `!= null` test on
  the dominating path (E10010; Ch 03 §6). Widening `T` → `T?` is implicit;
  `null` where a non-nullable type is required is E10012.

## Moves and Initialization (`own.rs`)

- A move happens when a move-only value is bound, passed to `take`, returned,
  assigned, or when a top-level field leaves an owned local or `take` parameter
  (`03-ownership.md:80`). Views and duplicables copy.
- Move state is flow-sensitive: after a move the source is invalid (E10020) and
  its planned drop becomes a no-op; at a join, a field moved on any path counts
  as moved (O6, STR-14, conservative).
- Partial moves are limited to top-level fields of owned locals and `take`
  parameters (O4/STR-11): nested paths and element moves are E10026; using a
  partially moved value as a whole is E10025; remaining fields stay usable and
  field reassignment re-initializes (O5/STR-13).
- Assignment to a live owner drops the old value first (O3); re-initializing a
  moved binding is allowed.

## Loans (`borrow.rs`)

Loans come from lends only — there is no borrow expression, no local view
expression (O9; `16-borrow-rules.md:36,66`). Loan sources: view-like
parameters, view-typed returns, field lends (tracked conservatively as
whole-struct loans, O8), and the stdlib view helpers (`slice`, `bytes`,
`bufferBytes`, `toCString`), whose results create a shared loan of the owner
for the result's origin (`appendix-stdlib.md:176-182`).

| Rule | Check | Diagnostic |
|---|---|---|
| BR-1 / OW-7 | a loan never outlives its owner's scope | E10030 |
| BR-2 / OW-8 | mutation of the owner while a shared loan is live (reads stay legal) | E10031 |
| BR-3 / OW-9 | an exclusive loan admits no other loan or access | E10032 |
| BR-4 / OW-4 | no move of an owner while any loan is live | E10022 |
| BR-5 / OW-16 | a lend parameter may not be released or stored, except a `borrow T?` out-slot | E10034 |
| BR-6/BR-7 / OW-17 | a returned view derives from a parameter, not a local, and from exactly one view-like lend parameter | E10035, E10036 |
| BR-8 / OW-10 | a loan never escapes to a field or global | E10033 |
| BR-9 / OW-19 | a global may not be borrowed | E10038 |
| BR-10 | writable view derived from a read-only loan | E10041 |

Elision is EL-1..EL-6 (Ch 06 §3.1, PF-013): every view-like parameter carries
its own elided lifetime; a view return needs exactly one view-like **lend**
candidate (`take` is not a candidate); `static str` returns need none; a view
parameter passed on is a reborrow limited to the call expression. Reborrowing
(O12, `16-borrow-rules.md:109-121`): a read-only view derived from a loan is a
shared reborrow; a writable view derived from a writable loan is an exclusive
reborrow limited to the creating expression — this is what makes
`borrow bufferBytes(buf)` legal when `buf` is itself a borrow parameter.

Field lends are whole-struct loans (O8): the checker is conservative, never
per-field-precise, and never accepts a program Rust would reject.

## Drop Planning (`own.rs` → IR)

Drops and `defer` registrations are materialized in the IR for the emitter
(Ch 17 §2, §4): locals/defers in reverse registration order (O2, CL-3),
temporaries at statement end in reverse creation order (D5/CL-3), struct
fields in declaration order, array elements first-to-last, parameters after
body locals. Every drop is null-safe and exactly once (O1, P5, CL-4); a moved
value has been zeroed, so its drop is a no-op. `panic` and the runtime faults
skip the ladder (E4, Ch 17 §10); `defer` runs on `return`/`break`/`continue`
but not on `panic` (F5/STMT-11).

## FFI Contracts (`ffi.rs`, checked from the IR)

Ch 12 owns the semantics; the checks are:

- a call to an `ffi` function is checked against the declared modes (CN-9):
  `take` moves (E10020 after use), `const`/`borrow` lend for the call;
- an `ffi` declaration with at least one unmarked handle/aggregate parameter is
  **uncontracted**: the call must be inside `unsafe` (E10100) and its
  uncontracted parameters accept only scalars, `raw`, `cstring`, views, and
  `null` handles (E10101); unmarked view parameters are read-only lends and
  contracted;
- a `handle T freedBy F` deleter must be an `ffi` function taking `take T` and
  returning `void` (CN-6, E10102); exactly one deleter per handle (CN-7);
  native structs never declare `freedBy` (CN-8, E10104);
- `export` on an `ffi` block, or FFI declarations leaking into application
  code, is E10103 (M9/CN-13); application code never writes `ffi` blocks (X6);
- a handle in a global is E11106 (OW-14/BR-9).

## Generics and Copy Obligations

Generic bodies are checked once (Ch 06 §8). A writable lend parameter
instantiated with a duplicable type binds a copy (V9a); writes affect the copy
and the compiler warns W10001 (not an error, D9). Read-only lends copy
silently. Copy
obligations are recorded in the generic body and discharged at each
instantiation; the error names the instantiation site, not the declaration.
Monomorphization (dedupe, naming, recursion cap E10305) is finalized in
[03-06-codegen.md](03-06-codegen.md); the checker supplies the instantiation
set and the per-instance obligations.

## Testing Requirements

- Spec tests: the in-scope golden compile-fail files for E100xx and E101xx
  (Phase 5), plus the ownership ST cases in
  [07-testing-strategy.md](07-testing-strategy.md) — move/DA/borrow/elision
  scenarios with concrete inputs and expected codes.
- Impl tests: IR snapshots for representative programs; dataflow unit tests
  for join behavior and out-parameter slots; loan conflict unit tests for
  reborrow O12 and field-loan conservatism.
- The red phase must show the golden E100xx/E101xx files failing before the
  ownership layer exists.
