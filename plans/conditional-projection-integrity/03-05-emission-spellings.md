# Phase 5: Emission Spellings

> **Document**: 03-05-emission-spellings.md
> **Parent**: [Index](00-index.md)

## Overview

Three emission spellings where an accepted program produces C the strict build rejects: a
projection of a conditional in return position mixes the arm spellings (DEF-183), a postfix update
shares one expression with a read (DEF-184, a spec/04 §3 sequence-point violation), and an owned
fixed-array projection of a call result takes the address of a non-lvalue (DEF-188). The invariant:
every accepted shape emits valid, single-evaluation C (Ch 17), byte-identical everywhere the phase
does not change the shape.

## Current Arc

- The return-position projection of a conditional emits a C conditional mixing `ClString` and
  `ClStr` (`type mismatch in conditional expression`); the design for the owned conditional store
  already exists in the arm machinery — the return spelling bypasses it (DEF-183).
- The postfix update renders inline (`format!("({operand_c})++")`, `codegen/expr.rs:420`), so
  `i++ + i` shares one sequence point (`-Werror=sequence-point`) (DEF-184).
- `return mk().xs;` on a plain array-field struct lowers to `memcpy(__cl_ret, &(mk().xs), …)` — an
  address of a non-lvalue (`lvalue required as unary '&' operand`) because the call root is not
  bound before the address is taken (DEF-188). A struct with an owned field happens to bind its
  root already, which is why the filed shape needs the plain array-field struct.

## Proposed Changes

### 1. Render the conditional arms in one spelling — `compiler/src/codegen/` and the lowering

Route the return-position projection of a conditional through the same arm materialization the
consuming store uses (Phase 1's per-arm shape) so both arms render as one C type: an owned arm
binds its temporary and the projection reads the bound value; a place arm reads its view. The
result: one `ClStr` (or the projected type) on both branches.

### 2. Pin the postfix update into a temporary — `compiler/src/codegen/expr.rs`

When a postfix update shares an expression with another read of the same place (the sequence-point
shape), lower the update into an explicit temporary statement (`__cl_tmp = i++;` then use
`__cl_tmp`), so the emitted C has one update and one read with a defined order. A standalone
postfix statement keeps its current spelling.

### 3. Bind the call root before the fixed-array address — `compiler/src/own/lower.rs`

An owned fixed-array projection of a call result binds the call root first (the same statement
temporary the owning value uses) and then takes the address of the bound local; the root's release
runs at the statement end. The already-binding struct-with-owned-field shape is unchanged.

## Tests

| Case | Shape | Expectation |
|------|-------|-------------|
| `condreturn` | DEF-183's `return (c ? b.v : a);` | strict build OK; one arm spelling; runs on both arms |
| `postfixseq` | DEF-184's `i++ + i` | strict build OK; the update pinned; the result value per spec/04 §5 |
| `fixedarrayaddr` | DEF-188's `return mk().xs;` | strict build OK; the root bound; ASan-clean |
| `emissioncontrol` | a direct-return conditional; a standalone `i++`; a struct-with-owned-field projection | byte-identical emission |

## Acceptance

- Every accepted shape builds strictly with `-Wall -Wextra -Werror` on `host` and emits for
  `hosted-amiga`/`freestanding`.
- The corpus differential classifies every delta as the unified arm spelling, the update temporary,
  or the bound call root; no other unit changes.
- The phase's spec suite is red at the baseline (the three strict-build failures) and green at the
  head.
