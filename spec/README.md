# The Clorraine Language Specification

> **Status:** working draft (v0.4 — Phase C: chapters rewritten against the ledger and the grammar)
> **Scope:** hosted AmigaOS profile first; freestanding (LorraineOS) implemented provisionally (m68k, no libc)
> **Decision ledger:** [decisions.md](decisions.md) — accepted decisions; the ledger wins on any conflict
> **Grammar freeze:** [grammar.ebnf.md](grammar.ebnf.md) — the master EBNF; every delta from Blend65 is in [grammar-deltas.md](grammar-deltas.md); the Rust semantics pin is [lineage.md](lineage.md)

Clorraine is the systems language of [LorraineOS](../README.md): a safe, C-like
language with ownership and borrow checking that compiles to plain C. It is
developed in the open under
[OpenLorraine](https://github.com/OpenLorraine).

## How to read this specification

* Chapters are numbered and referenced as `Ch NN`.
* Every chapter states a **Status**: `planned`, `draft`, `reviewed`, `stable`.
* Rules carry IDs (for example `OW-3`) and diagnostics carry five-digit
  `Exxxxx`/`Wxxxxx` codes registered in [Ch 14](14-diagnostics.md).
* The [decision ledger](decisions.md) is authoritative: Phase A decisions,
  Phase B grammar freezes (section 19), and the Phase C clarifications
  (section 20) all live there.

### Reading order

Read `00` first for the axioms, then the numbered chapters, then the
appendices (A, B) alongside the chapters they support. The `Depends on` lines
in the chapter headers are informational, not a topological order: the
dependency graph is cyclic on purpose, so it orders understanding, not work.

### Phase 2 stages

The compiler is built in this order; each stage lands with its tests.

1. Front end: lexer and parser.
2. Name resolution.
3. Types and constant evaluation.
4. Ownership, definite assignment, and borrow checking.
5. C codegen with the cleanup ladder.
6. Runtime shim and the standard-library slice.
7. The compile-fail and run suites.

## Derivation from Blend65

Clorraine reuses the structure and process of the Blend65 specification
(drafted by the same author) and replaces everything that existed only because
of the 6502 target:

| Blend65 | Clorraine |
|---|---|
| Syntax, modules, diagnostics registry, decision records | Kept |
| Static Frame Allocation, zero page, 16-bit addresses, 6502 intrinsics | Replaced by an ownership model, 32-bit m68k types, Amiga/OS profiles |
| Proto-borrow/provenance model | Replaced by typed owned/shared/exclusive modes with a real borrow checker |
| 6502 codegen tables | Replaced by a C99 codegen contract |

The full accounting is [grammar-deltas.md](grammar-deltas.md): removed
constructs, restrictions lifted by the C backend, carried constructs, and the
guarantees lost with their substitutes.

## Chapter index

| Ch | Title | Status |
|---|---|---|
| — | [Decision ledger (Phase A)](decisions.md) | accepted |
| — | [Semantic lineage (Rust pin)](lineage.md) | draft |
| — | [Master EBNF grammar (freeze)](grammar.ebnf.md) | draft |
| — | [Grammar deltas from Blend65](grammar-deltas.md) | draft |
| 00 | [Introduction and axioms](00-introduction.md) | draft (Phase C) |
| 01 | [Lexical structure](01-lexical.md) | draft (Phase C) |
| 02 | [Type system](02-types.md) | draft (Phase C) |
| 03 | [Ownership, moves, and destructors](03-ownership.md) | draft (Phase C) |
| 04 | [Expressions and operators](04-expressions.md) | draft (Phase C) |
| 05 | [Statements and control flow](05-statements.md) | draft (Phase C) |
| 06 | [Functions and parameters](06-functions.md) | draft (Phase C) |
| 07 | [Structs](07-structs.md) | draft (Phase C) |
| 08 | [Arrays, slices, and strings](08-arrays-strings.md) | draft (Phase C) |
| 09 | [Enums and tagged data](09-enums-unions.md) | draft (Phase C) |
| 10 | [Modules](10-modules.md) | draft (Phase C) |
| 11 | [Memory and resources](11-memory.md) | draft (Phase C) |
| 12 | [FFI and ownership contracts](12-ffi-contracts.md) | draft (Phase C) |
| 13 | [Data inclusion](13-data-inclusion.md) | draft (Phase C) |
| 14 | [Diagnostics registry](14-diagnostics.md) | draft (Phase C) |
| 15 | [Platform profiles](15-profiles.md) | draft (Phase C) |
| 16 | [Borrow-checker rules](16-borrow-rules.md) | draft (Phase C) |
| 17 | [C codegen contract](17-codegen.md) | draft (Phase C) |
| A | [Amiga appendix](appendix-amiga.md) | draft (Phase C) |
| B | [Standard library (v1)](appendix-stdlib.md) | draft (Phase C) |

## Validation

```sh
./check.sh        # headers, status lines, links, diagnostics, compile-fail coverage
```

The specification is a non-code artifact; the validation command above is its
test. Besides headers, status lines, and relative links, it cross-validates
every diagnostic code used in a chapter or test against the registry (N7) and
enforces compile-fail coverage for every registered compile-time code. The
Phase E preflight gate runs the `preflight` workflow before the first compiler
milestone.
