# Phase 2 Review: Front End, Diagnostics, CLI, Harness Skeleton

> **Document**: 10-phase2-review.md
> **Parent**: [Index](00-index.md)
> **Date**: 2026-09-19
> **Scope mode**: strict

## What was reviewed

The complete Phase 2 diff from the phase baseline tree
(`5d0f1a57f20c8bf901d4ec701675ba12b4c4d673`) through task 2.3.3: the
workspace, crate, and root Makefile; the two spec-test drivers; the span,
diagnostics, lexer, AST, parser, and driver modules with their implementation
tests. Reviewer: an independent phase-review subagent carrying the
correctness, maintainability, and standards lenses, exercising the built
binary on every in-scope golden plus adversarial inputs.

## Findings and rulings

| ID | Severity | Finding | Ruling (user, 2026-09-19) |
|---|---|---|---|
| RV-001 | 🟠 MAJOR | `comptime func` consumed its `func` keyword twice, so the staged-comptime golden reported a spurious `E20000` | Fix: `parse_func` consumes the keyword once |
| RV-002 | 🟠 MAJOR | Nested type arguments (`Box<Box<i32>>`) failed because the lexer emits `>>` as one token and nothing split it | Fix: the type-argument close splits a `>>` token and carries the extra angle bracket |
| RV-003 | 🟠 MAJOR | In expression position the type-argument guess committed even without a following `(`/`{`, so `a < b > c` was not relational as the grammar requires | Fix: restore the cursor and let the binary chain parse `<` as relational; the unused generic-reference node is removed |
| RV-004 | 🟡 MINOR | Top-level misplaced `volatile` was consumed before reporting, pointing `E10208` at the next token | Fix: report at the `volatile` token, then recover by parsing what follows |
| RV-005 | 🟡 MINOR | Lexical errors cascaded: the `E20002`/`E20004`/`E20007` goldens each added two extra `E20000` | Fix: keep error tokens in the stream and consume them as expression atoms and skippable units; a swallowed statement terminator after an error is accepted silently |
| RV-006 | 🟡 MINOR | Dead code: an unused `comptime` parameter and an unreachable union-diagnostic branch | Fix: remove both |
| RV-007 | 🟡 MINOR | The driver's `output` field documented standard output although no output is written without `-o` | Fix: correct the documentation |

Follow-up found while verifying RV-004: `volatile let` at module level (the
valid form) was itself reporting `E10208`; the check now accepts it and only
reports a misplaced `volatile`.

## Fix evidence

- `make check` green: spec check OK (27 files, 101 compile-fail tests), 47
  unit tests, 2 compile-fail driver tests, 6 stage driver tests.
- New regression tests: `comptime` parsing, nested type arguments, the
  relational fallback, lexical-error non-cascade, and module `volatile let`
  versus misplaced `volatile`.
- Direct binary probes: the `E20002`/`E20004`/`E20007` goldens each print one
  diagnostic; `Box<Box<i32>>` and `a < b > c` compile cleanly; the staged
  `comptime` golden parses without syntax noise.

## Areas confirmed clean

- The compile-fail driver discovers the goldens, matches the registry
  templates with `{...}` wildcards, honors per-file `cli`/`profile`
  annotations, and reports 13 asserted, 14 excluded, and 78 pending files.
- The driver honors the CLI contract: required `--profile`, both flag
  spellings, `-o`, `--deny-warnings`, `--max-errors`, usage exit code 2, and
  the empty translation unit on success.
- No external crates, no build scripts, no `unsafe` in the crate, and no
  plan or decision identifiers in code or comments.
