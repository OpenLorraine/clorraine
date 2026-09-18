# Phase 0 Review: Validation Corpus

> **Document**: 08-phase0-review.md
> **Parent**: [Index](00-index.md)
> **Date**: 2026-09-18
> **Scope mode**: strict

## What was reviewed

The complete Phase 0 diff from baseline tree
`a798591ac32201831a1e4c1c13708e567a98ccfe` through task 0.2.1: the four corpus
programs, their expected outputs, their coverage records, and the
execution-plan updates. Reviewer: an independent phase-review subagent carrying
the correctness, maintainability, and standards lenses. The diff stayed inside
the expected modification set; no scope drift.

## Findings and rulings

| ID | Severity | Finding | Ruling (user, 2026-09-18) |
|---|---|---|---|
| RV-001 | 🔴 CRITICAL | `string-parser` asked for `hex=0x00003A` through `padLeft(toHex(u32(sum)), 8, '0')`, but the frozen `toHex` already returns the `0x` prefix, so the expression yields `00000x3A` and the golden could never pass | Fix the corpus: print `hex=0x3A` via `toHex(u32(sum))`; 03-01 and `expected-output.txt` amended (AR-54). No specification change |
| RV-002 | 🟠 MAJOR | `removeHandler(token)` passes a duplicable struct to an unmarked read-only lend, which the original W10001 rule made a warning, contradicting the zero-diagnostic corpus contract | Amend the specification: W10001 applies to writable lends of duplicable values only; read-only lends copy silently (AR-55) |
| RV-003 | 🟡 MINOR | `file-copier` reported the intended payload length rather than an accumulated counter, and its README omitted the unused-API note the other records carry | Report-only; the README now records the unused APIs and the count semantics |

Correctness and coverage were otherwise confirmed: every expected output ends
in exactly one newline, the programs contain no plan identifiers and no staged
constructs, and the W10002/W10006 audits are truthful.

## Fix batch applied

| Area | Files |
|---|---|
| Corpus | `spec/tests/corpus/string-parser/{program.clor,expected-output.txt,README.md}`, `spec/tests/corpus/interrupt-counter/README.md`, `spec/tests/corpus/file-copier/README.md` |
| Specification (AR-55) | `spec/03-ownership.md`, `spec/06-functions.md`, `spec/14-diagnostics.md`, `spec/16-borrow-rules.md`, `spec/decisions.md` (D9), `spec/lineage.md` (D9 row), `spec/tests/compile-fail/W10001_needless_lend.clor` |
| Plan | `03-01-corpus.md`, `03-05-ownership.md`, `03-06-codegen.md`, `00-ambiguity-register.md`, `00-index.md`, this record |

The AR-55 amendment changes only the W10001 trigger wording; the diagnostic
code and its first line are unchanged, so the golden suite contract holds.

## Verification and re-review

- `./spec/check.sh` stays green after the amendment; the corpus contract
  cross-check was re-run against the amended files.
- One scoped re-review of the fix diff confirmed RV-001 and RV-002 resolved
  with no corpus or golden defect. It raised three follow-ups, all addressed:
  - RV-004 (MINOR): the `const`-marker warning in Ch 03 had no registry code
    after the narrowing; the text now records that V4 is tolerated without a
    dedicated code in v1, deferred to marker-strictness work.
  - RV-005 (MINOR): AR-55 over-claimed the amended Ch 16 sections; corrected
    to §2/§5/§7.
  - RV-006 (INFO): AR-54 and the string-parser README over-stated stdlib test
    coverage for the dropped helpers; corrected.
