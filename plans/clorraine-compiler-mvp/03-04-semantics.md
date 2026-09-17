# Semantics: Clorraine Compiler MVP

> **Document**: 03-04-semantics.md
> **Parent**: [Index](00-index.md)

## Overview

This layer turns the AST into a fully resolved and typed program: file
loading and module identity, name resolution, the type system and constant
evaluation, struct/enum checking, generic instantiation checks, and module
initialization order. It is implemented by `load.rs`, `resolve.rs`,
`types.rs`, `consts.rs`, and `sema.rs`. Rules are owned by Ch 02, Ch 04,
Ch 06, Ch 07, Ch 09, and Ch 10; this document owns the architecture.

## Loading and Module Identity (`load.rs`)

- The CLI file list is the program; files are parsed independently and grouped
  by their `module A.B;` declaration (MOD-1, MOD-2, AR-25). A missing or
  misplaced declaration is E11105; files declaring the same module merge
  (MOD-7) and duplicate names are E11101 (FN-2 for functions).
- Imports (`import { X, Y as Z } from A.B;`) bind names in module scope,
  order-independently, with renaming and duplicate-binding detection
  (MOD-9..MOD-12, E11102/E11103, W10006). Qualified access `A.B.x` is always
  available without an import (MOD-13); circular imports are allowed (MOD-14).
- Standard-library modules come from the embedded table (`stdlib.rs`, AR-19,
  AR-25); a module the selected profile does not provide is E11200 with the
  profile name (PR-2, PR-3).
- Module-level declarations are order-independent within a module (MOD-17).

## Name Resolution (`resolve.rs`)

Ch 10 §5 owns the rules (MOD-15..MOD-18): lexical scopes per block, parameters
in the body scope, declaration-before-use inside a block, innermost scope wins,
locals may shadow outer locals and module names, and a use before declaration
in its block is E10219. Resolution produces a symbol id per name use; every
later phase works on resolved ids, never on strings.

Dot-call `x.f(y)` resolves per Ch 04 §8 (F8): a field wins; otherwise a free
function `f(x, y)`; resolution is static with no receiver semantics. There is
no ambiguity case: fields win and duplicate declarations are already E11101 /
E11103 (the former E10225 is retired).

Built-in intrinsic names are reserved: only the compiler's `intrinsic func`
declarations bind them (MOD-18, LEX-9, GF23, AR-46); an ordinary declaration
using a reserved name is E20006.

## Type System (`types.rs`)

`Ty` covers the Ch 02 surface: scalars (`i8`-`u32`, `usize`/`isize`,
`f32`/`f64`), `bool`, `void`, `raw T`, handles, structs, enums, views
(`T[]`, `str`, `cstring`, `static str`), fixed arrays, function types, generic
parameters, and `T?` nullability. Duplicability is derived from fields (V8,
TY-?: handles and any aggregate containing one are move-only).

Typing rules cite the specification, not this document:

| Area | Owner |
|---|---|
| Implicit widenings, mixed-sign/narrowing errors | `02-types.md` TY-11..TY-13, T1a (PF-005) |
| Operator typing, usual arithmetic unification, literals adapt or E10202 | Ch 04 EX-*, Ch 02 §9 |
| Cast legality `T(expr)` and enum/handle casts | TY-14, EX-8, B8 |
| `bool` is not an integer; conditions must be `bool` | TY-1..TY-3, EX-7, E10218 |
| Defined arithmetic semantics | EX-4..EX-6, Ch 17 §10 |
| Assignment result is `void`; targets must be places | EX-2, EX-14, E10217, E10221 |
| Indexing rules | EX-10, IX-1..IX-3, E10209/E10216/E11211 |
| Function type mismatch | FN-4, E10210 |
| Nullability | Ch 03 §6, TY-4..TY-7, E10010-E10012 |
| Duplicability | Ch 02, V8 |

Structs (`sema.rs`): field order and C layout (STR-2), non-empty (E10206),
no unions (E10204), view-field ban except `static str` (T4, TY-10, E10205),
overlay types and their by-value ban (STR-15..STR-18, E10207/E10208),
named-field literals with missing/unknown/duplicate detection (STR-5..STR-8,
E10203), zero value `Point {}` (STR-7), no update syntax (STR-9).

Enums (`sema.rs`): integer backing type required (EN-1, E10211), sequential
values from 0 unless explicit constants (EN-3, T10a), scoped members reached
with `.` (EN-2, E10212), comparison only `==`/`!=` (EN-6, E10222), explicit
casts only (EN-4).

Function values (`sema.rs`): exact-signature `func(T): R` (FN-4); a bare
identifier names a function; capture-less lambdas would match but lambdas are
staged (AR-14) and report E90000/E90002.

## Constant Evaluation (`consts.rs`)

Constant contexts are fixed by Ch 04 §12 (`04-expressions.md:360-363`): array
extents, enum member values, `case` labels, module-level `const` initializers.
Folding uses runtime semantics (EX-16): wrapping arithmetic, masked shifts,
division by zero or `MIN / -1` is E10223, a non-constant expression is E10224,
a literal that does not fit is E10202. Extents greater than zero (AR-5,
E10226); constant indexing out of range is E10216; omitted inner extents are
E10220; a fill literal without a fixed extent is E10215 (AL-1).

`sizeof`, `offsetof`, and `len` fold to `usize` when their operand is known at
compile time; `sizeof`/`offsetof` use the emitted C layout (Ch 17 §7, P1).
`embed` is out of scope (E90000, AR-10).

## Generics (`sema.rs`, completed by ownership/codegen)

Per AR-13: type parameters on functions and structs, explicit type arguments,
arguments-only inference (G2), and monomorphization with hash-deduped symbols
(G3/G-6). Diagnostics: wrong arity E10300, cannot infer E10301, exported
generic E10302, recursion cap E10305. Callable bounds and their diagnostics
(E10303/E10304) and ephemeral arguments (E10306) are out of scope (AR-10) and
the corresponding golden tests are skipped.

Generic bodies are checked once against their type parameters, recording copy
obligations; each instantiation discharges the obligations and names the
instantiation site in errors (V9, `06-functions.md:287-291`).

## Module Initialization (`sema.rs`)

Module-level `let`/`const` initializers are checked for non-nullability (M7a,
E10200) and duplicable-or-const globals (O10/MOD-25, E11106). The init
dependency graph uses imports plus modules named by initializer expressions,
transitively; references inside called function bodies create no edge (MOD-21,
MOD-22). Initialization is a topological order with a byte-wise lexicographic
tie-break on the qualified module name (PF-009); a cycle involving a
runtime-initialized global is E11100 (same module or across modules). The
emitted `cl_init_<module>` functions and entry stub are owned by
[03-06-codegen.md](03-06-codegen.md).

## Testing Requirements

- Spec tests: the in-scope golden compile-fail files grouped by phase —
  name/module cases (E10219, E11101-E11105, W10006) in Phase 3; type,
  expression, struct, enum, const cases (E10200-E10212, E10215-E10226, E10500,
  E10501, E10503) in Phase 4.
- Stage unit ST cases from [07-testing-strategy.md](07-testing-strategy.md)
  for conversions, operator typing, const folding, struct/enum checks, and
  init ordering (impl tests may go deeper).
- Impl tests: `types.rs` unit tests for the conversion matrix; `consts.rs`
  unit tests for fold/overflow/fault cases; `resolve.rs` scope tests.
