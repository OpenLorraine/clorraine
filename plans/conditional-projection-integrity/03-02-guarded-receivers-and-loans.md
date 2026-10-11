# Phase 2: Guarded Receivers and Loans

> **Document**: 03-02-guarded-receivers-and-loans.md
> **Parent**: [Index](00-index.md)

## Overview

Three gaps in the guarded binder and the loan recording: a conditional place arm registers no
shared loan (DEF-180), a guarded struct-literal receiver root is not routed through the guarded
binder (DEF-181), and a dot-`len` receiver of a conditional view projection misses the
conditional gate (DEF-182). The invariant: a guard's taken path releases the receiver's storage
exactly once, and a live conditional view loan behaves exactly like its named spelling.

## Current Arc

- `bind_guarded_projection_root` (`own/lower.rs:1931`) descends `Member`/`Index` to the root and
  handles call results and conditionals; **a struct-literal root is returned unchanged**, so
  `true && H { call, s }.call(5)` leaks the receiver's `s` (DEF-181).
- The conditional value lowering registers no shared loan for a **place arm**; the named spelling
  (`let s: str = b.v;`) records one and reports `E10031` on mutation, while
  `let s: str = (c ? b : mk()).v;` is accepted (DEF-180).
- `bind_intrinsic_argument` (`own/lower.rs:2004`) gates conditional arguments, but the dot-`len`
  receiver path reaches neither that gate nor the guarded binder; the two-field `Box` probe leaks
  the root's second field (DEF-182).

## Proposed Changes

### 1. Route struct-literal receiver roots — `compiler/src/own/lower.rs`

Extend `bind_guarded_projection_root` (and its caller in `own/call.rs` for receiver extraction) to
wrap a **struct-literal** root in the same deferred `Materialize` shape a call result gets: the
guard's fill binds the literal once on the taken path, the call reads its receiver, and the
statement's registered release frees the fields the call does not move.

### 2. Record the conditional place arm's shared loan — `compiler/src/own/`

When a conditional view projection's value is bound and an arm roots in a place, register the same
shared loan the named spelling registers (the loan set that `E10031` consults), so a later mutation
of the place while the view is live reports `E10031` once at the mutation site. The fresh-call arm
adds no loan (the batch E machinery already owns its root).

### 3. Extend the conditional gate to the dot-`len` receiver — `compiler/src/own/lower.rs`

Route the dot-`len` receiver through the same conditional gate as the intrinsic argument: a
conditional receiver with a place-borrowing arm is hoisted per arm (the argument keeps its
evaluation order), and a conditional receiver of call results binds through the guarded binder, so
the root's unselected fields release exactly once. The `len(cond.v)` argument spelling stays
unchanged.

## Tests

| Case | Shape | Expectation |
|------|-------|-------------|
| `guardliteral` | DEF-181's `&&` shape and the `\|\|`/`?:` taken paths | ASan-clean; one release |
| `placearmloan` | DEF-180's conditional view + mutation, and the named control | one `E10031` at the mutation; the control unchanged |
| `dotlenreceiver` | DEF-182's two-field conditional `.v.len()` | ASan-clean; the `len(cond.v)` spelling byte-identical |
| `guardplace` | accepted controls: an unguarded struct-literal receiver, a conditional of places, a slice view receiver | unchanged behavior |

## Acceptance

- Every leak shape is ASan-clean; the loan rejection reports exactly once with the frozen message.
- Accepted controls keep their emitted C byte-identical.
- The phase's spec suite is red at the baseline and green at the head; the semantics lens confirms
  the loan reading against Ch 03 §7 and Ch 16 §3.
