# Phase 5: Diagnostic Hygiene

> **Document**: 03-05-diagnostic-hygiene.md
> **Parent**: [Index](00-index.md)

## Overview

Four reporting defects: the double and triple state reports at one update site (DEF-153), the
`E10034`-instead-of-`E10401` capture move outside the escape sites (DEF-160), the doubled
report on a block-bodied composite capture return (DEF-161), and the `__cl_ownedN` name in an
`E10030` message (DEF-162). Acceptance does not move for any of the four: every affected shape
stays rejected (or accepted) exactly as today, with a single, correctly attributed report.

## Current Arc

- **DEF-153** — the statement update path evaluates the operand (`own/statements.rs:157-170`),
  then `check_update` (`own/expr.rs:907-965`) reads and writes the place; each read/write
  reaches the state reporters (`own/place.rs:20-76`), so one site emits `E10020` up to three
  times (a wholly moved local's field) or `E10021` twice (an uninitialized field/element). The
  value form reports through the same reporters.
- **DEF-160** — the capture-move recorder (`sema/expr.rs:293-345`) runs at the `return` and
  `take`-argument sites; `mark_moved_if_owning_at` (`own/place.rs:470-495`) skips the lend
  duty only for an escaping by-value capture (`escape && is_by_value_capture_base(...)`) and
  documents the gap: "A move into an owned local stays with the lend duty, which the
  capture-move recorder does not cover." A `let`/store move therefore reports `E10034` where
  Ch 06 L-3 assigns `E10401`.
- **DEF-161** — a block-bodied composite return (`Holder<T> { v: x }`) reaches the return
  recorder (`sema/stmt.rs:60`) **and** the field store's lend duty, reporting `E10401` and
  `E10034` together; the expression-bodied twin reports one `E10401`.
- **DEF-162** — the `E10030` subject renders through `display` (`own/liveness.rs:575`) which
  names a binding directly; a generated owner (`__cl_ownedN`, flagged by
  `own/flow.rs:74`) leaks its compiler spelling into the message.

## Proposed Changes

### 1. One state report per site (DEF-153) — `compiler/src/own/`

Add a per-site ledger to the checker (a set keyed by the binding, the projections, and the
reporting span) consulted by the state reporters: the first report at a site records and
emits, later reports for the same site are suppressed. The ledger resets per function; the
report text and the first report's span are unchanged.

### 2. Every capture-move site reports `E10401` (DEF-160/161) — `sema/` + `own/`

- Record the by-value capture move at the remaining move sites: the `let` initializer, the
  assignment value, and the composite/struct-literal field value (the existing
  `record_generic_capture_move` walk covers each expression shape).
- In `mark_moved_if_owning_at`, retire the lend duty for a by-value capture root at every
  recorded site (drop the `escape &&` condition now that the recorder covers them); the
  lending rules for every other root and the out-slot carve-outs are untouched.
- One site keeps one report: the return and field-store paths converge on the single `E10401`.

### 3. A display phrase for a generated owner (DEF-162) — `own/liveness.rs`

When the loan's owner binding carries the `generated` flag, the subject renders as a phrase
(for example "a compiler-generated owner") instead of the binding name; named owners and the
temporary case are unchanged, and the registry template `borrow of '{name}' does not live long
enough` is untouched.

## Tests

| Case | Shape | Expectation |
|------|-------|-------------|
| `statededupe` | a value update on a wholly moved local's field (DEF-153) | exactly one `E10020` at the site |
| `statededupeinit` | an uninitialized field/element update | exactly one `E10021` |
| `statededupeok` | a legal update | accepted; byte-identical emission |
| `capturelet` | `let y: T = x;` at an owning instance (DEF-160) | exactly one `E10401`; no `E10034` |
| `captureduplicable` | the same shape at a duplicable instance | accepted; byte-identical emission |
| `captureblock` | the block-bodied composite return (DEF-161) | exactly one `E10401` |
| `captureexpr` | the expression-bodied twin | exactly one `E10401` (the baseline paired `E10034` and `E10401`, like the block body — corrected during the phase) |
| `escapeowner` | the escaped generated owner (DEF-162) | the message renders the phrase; no `__cl_owned` |

## Acceptance

- One report per site per code, at the site's original span; the affected programs' acceptance
  is unchanged (errors stay errors, accepted stays accepted).
- The corpus differential shows no delta (diagnostic-only changes emit nothing new); the
  message pin for `escapeowner` is the only text-level difference.
