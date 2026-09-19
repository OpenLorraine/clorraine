# Phase 4 Review: Types, Constants, Structs, Enums

> **Document**: 12-phase4-review.md
> **Parent**: [Index](00-index.md)
> **Date**: 2026-09-19
> **Scope mode**: strict

## What was reviewed

The complete Phase 4 diff from the phase baseline tree
(`ec101bd121c45b8a1c1a1bdd4ceb0690f6d05001`) through task 4.3.3: the new
type model (`types/`), the semantic checker (`sema/`), the constant folder
(`consts/`), the driver wiring, the built-in `String` handle, the Phase 4 spec
tests, and the plan-doc correction. Reviewer: an independent phase-review
subagent carrying the correctness lens, running `make check` and exercising the
built binary on adversarial inputs. No scope drift was found.

## Findings and rulings

| ID | Severity | Finding | Ruling (user, 2026-09-19) |
|---|---|---|---|
| RV-200 | 🟠 MAJOR | Constant folding wrapped in a wide accumulator, ignoring operand width: a `u8` enum value `255 + 1` was a false `E10202`, and `i16(-32768) / i16(-1)` missed `E10223` | Fix: fold at the constant-context width |
| RV-201 | 🟠 MAJOR | Array extents read only integer literals, so a `const` or arithmetic extent became an omitted extent and reported a false `E10215`/`E10224` | Fix: fold constant names and arithmetic for extents |
| RV-202 | 🟠 MAJOR | A nested array literal expected the base element type, so `u8[3][2]` with nested lists reported false `E10200` | Fix: expected element type honors the remaining extents |
| RV-203 | 🟠 MAJOR | Dot-calls were unchecked: a value-headed dotted callee never reached dot-call checking, and the member path passed the receiver among the arguments | Fix: route value-headed dotted callees through dot-call checking and drop the receiver |
| RV-204 | 🟠 MAJOR | An integer literal could take a `bool` context (`let b: bool = 1;` was accepted) | Fix: literal adaptation is limited to integer scalar contexts |
| RV-205 | 🟠 MAJOR | The overlay by-value ban applied only to struct literals, so an overlay `let`, global, or parameter was accepted | Fix: reject overlay values everywhere except behind `raw` |
| RV-206 | 🟡 MINOR | `assert` checked its condition twice, emitting duplicate diagnostics | Fix: check the condition once |
| RV-207 | 🟡 MINOR | A compound assignment result was never converted back to the target, so `x += y` with a wider `y` was accepted | Fix: convert the result to the target type |
| RV-208 | 🟡 MINOR | The folder treated an unclassified name as constant `0`, missing `E10224` and producing false extent errors | Fix: an unclassified name is not constant; constant names, imported `const`s, and enum members still fold |

## Deferred latent notes

Strict scope excludes optional work; these are recorded, not fixed:

- `int_fits` treats `usize`/`isize` as 32-bit, while the `host` profile is
  pointer-sized. No golden or corpus case triggers it, and a proper fix needs
  the target model; it will be addressed with codegen.
- The lexer discards a literal's radix, so the hex "full unsigned width of the
  target" allowance cannot apply to a signed target. Fixing it changes the AST.
- `size_of`/`offset_of` use a packed layout; field alignment matters only when C
  is emitted.
- Type and function keys are program-global rather than per-module; documented
  as an MVP limitation and unaffected by the corpus.

## Fix evidence

- `make check` green: spec check OK (27 files, 101 compile-fail tests), 150
  unit tests (15 new regression tests), 2 compile-fail driver tests, 14 stage
  driver tests; all 47 asserted goldens pass.
- Every finding's trigger was re-run through the built binary and now behaves
  as the specification requires.

## Re-review of the fix diff

_(recorded after the single scoped re-review)_
