# The Clorraine Language Specification

> **Status:** working draft (v0.3 — Phase B anchors: the grammar freeze)
> **Scope:** hosted AmigaOS profile first; freestanding (LorraineOS) designed-for, implemented later
> **Decision ledger:** [decisions.md](decisions.md) — accepted decisions; where a chapter disagrees, the ledger wins until the chapter is rewritten
> **Grammar freeze:** [grammar.ebnf.md](grammar.ebnf.md) — the master EBNF; every delta from Blend65 is in [grammar-deltas.md](grammar-deltas.md); the Rust semantics pin is [lineage.md](lineage.md)

Clorraine is the systems language of [LorraineOS](../README.md): a safe, C-like
language with ownership and borrow checking that compiles to plain C. It is
developed in the open under
[OpenLorraine](https://github.com/OpenLorraine).

## How to read this specification

* Chapters are numbered and referenced as `Ch NN`.
* Every chapter states a **Status**: `planned`, `draft`, `reviewed`, `stable`.
* Rules carry IDs (for example `OW-3`) and diagnostics carry `E1xxxx`/`W1xxxx`
  codes from [Ch 14](14-diagnostics.md).
* Ownership keywords were decided in Phase A and are listed in the
  [decision ledger](decisions.md). Chapters written before that lock may still
  show the old placeholders (`own`, `borrow mut`, `move`); those chapters are
  rewritten against the ledger and the grammar in Phase C.

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
| 00 | [Introduction and axioms](00-introduction.md) | draft |
| 01 | [Lexical structure](01-lexical.md) | planned |
| 02 | [Type system](02-types.md) | planned |
| 03 | [Ownership, moves, and destructors](03-ownership.md) | draft |
| 04 | [Expressions and operators](04-expressions.md) | planned |
| 05 | [Statements and control flow](05-statements.md) | planned |
| 06 | [Functions and parameters](06-functions.md) | planned |
| 07 | [Structs](07-structs.md) | planned |
| 08 | [Arrays, slices, and strings](08-arrays-strings.md) | planned |
| 09 | [Enums and tagged unions](09-enums-unions.md) | planned |
| 10 | [Modules](10-modules.md) | planned |
| 11 | [Memory and resources](11-memory.md) | planned |
| 12 | [FFI and ownership contracts](12-ffi-contracts.md) | draft |
| 13 | [Data inclusion](13-data-inclusion.md) | planned |
| 14 | [Diagnostics registry](14-diagnostics.md) | draft |
| 15 | [Platform profiles](15-profiles.md) | planned |
| 16 | [Borrow-checker rules](16-borrow-rules.md) | draft |
| 17 | [C codegen contract](17-codegen.md) | draft |
| A | [Amiga appendix](appendix-amiga.md) | planned |

## Validation

```sh
./check.sh        # headers, status lines, relative links
```

The specification is a non-code artifact; the validation command above is its
test. Phase 1 review happens through the `preflight` workflow before the
first compiler milestone.
