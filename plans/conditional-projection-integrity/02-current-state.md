# Current State: Conditional and Projection Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)
> **Basis**: every claim below was probed or read at the batch baseline `5452bce` (the
> def-backlog-closeout close-out head); the probe programs live in the batch's scoping scratch
> directory and their emitted C was inspected. `DEF-256` was found by this survey and filed with
> the plan.

## Overview

The batch's defects live in the ownership/lowering stage (`compiler/src/own/`) and its emission
(`compiler/src/codegen/`), plus the borrow-model helpers (`compiler/src/borrow.rs`). This document
maps each defect to its verified present behavior and its code site.

## Verified present behavior (probes at `5452bce`, 2026-10-12)

| Defect | Probe (host profile) | Observed at head | Verdict |
|--------|----------------------|------------------|---------|
| DEF-176 | `let s: String = (c ? a : mkBox()).v;` with `c = true` | AddressSanitizer `attempting double-free` | confirmed |
| DEF-177 | `let vs: str[1] = [mk().v];` | leak of 11 bytes; the loan spelling (`[h.v]` then moving `h`) reports `E10025` where the single-view twin reports `E10022` | confirmed |
| DEF-178 | `if (c && i32(len((c ? b : mk()).v)) > 0)` with `c = true` | AddressSanitizer `attempting double-free` | confirmed |
| DEF-179 | `let s: String = d ? (c ? b : mk()).v : stringOf("zz");` with `c = d = true` | AddressSanitizer `attempting double-free` | confirmed |
| DEF-180 | `let s: str = (c ? b : mk()).v; b.v = stringOf("zz");` | accepted; the named spelling (`let s: str = b.v;` then the same write) reports `E10031` | confirmed |
| DEF-181 | `true && H { call: addOne, s: stringOf("x") }.call(5) > 0` | leak of 10 bytes | confirmed |
| DEF-182 | `let n: usize = (c ? mk(1) : mk(2)).v.len();` with a two-field `Box` | leak of 11 bytes (the root's other field); the `len(cond.v)` spelling is clean | confirmed |
| DEF-183 | `func use(c: bool, b: Box, a: str): str { return (c ? b.v : a); }` | strict build fails: `type mismatch in conditional expression` | confirmed |
| DEF-184 | `let j: i32 = i++ + i;` | strict build fails: `-Werror=sequence-point` | confirmed |
| DEF-185 | `run<String>(make)` discarding `([make] () => mkG<T>(make()))()` | leak of 11 bytes | confirmed |
| DEF-186 | `c ? make1() : make2();` in a three-iteration loop | leak of 33 bytes (three results) | confirmed |
| DEF-187 | `func pick(x: str, h: Box): str { return h.v; }` called with a temporary `Box` through `mid()` | AddressSanitizer `heap-use-after-free`; the zero-candidate twin (`func f(h: Box): str { return h.v; }`) already reports `E10036` | confirmed |
| DEF-188 | `func f(): u8[4] { return mk().xs; }` with a plain `struct Box { xs: u8[4]; }` | strict build fails: `lvalue required as unary '&' operand` | confirmed |
| DEF-256 | `let s: String = stringOf("ab"); s.len();` and `mk().v.len();` as statements | strict build fails: `implicit declaration of function 'len'`; the value position (`let n: usize = s.len();`) folds correctly | confirmed (filed with this plan) |

### Triage notes

- **DEF-175** (filed by the batch E Phase 2 re-review) is **fixed**: the batch E fix round recorded
  that the arm processor descends Member/Index to the root (RV-003), and both filed spellings
  (`lenOf(pick ? mk().name : b)` and the `len(...)` intrinsic) are clean on both taken paths at head.
  The row is closed in this batch's planning commit; the consuming spellings are `DEF-176`.
- `DEF-182`'s leak needs the root to own a second field (`Box { v: String; w: String; }`); the
  single-field spelling releases the whole root trivially. The phase's spec case uses the two-field
  shape.
- `DEF-256`'s statement spelling fails the strict build **and** the link (no `len` symbol); the filed
  row cross-references `DEF-224` (the value-form `.len`), which fixes a different side of the same
  surface.

## Code map

### Ownership and lowering (`compiler/src/own/`)

| Area | Location | Present behavior |
|------|----------|------------------|
| Guarded projection binder | `lower.rs:1931` (`bind_guarded_projection_root`) | Descends `Member`/`Index` to the root and wraps call results and conditionals in the deferred `Materialize` shape; a chain that already roots in storage, or a root that owns nothing, is returned unchanged. **Struct literals are not routed** (DEF-181) |
| Intrinsic argument binder | `lower.rs:2004` (`bind_intrinsic_argument`) | Binds ownerless owned intrinsic arguments; a conditional argument with a place-borrowing arm routes through `hoist_ownerless_conditional_arms`/`own_ownerless_conditional_arms` when `conditional_borrows_place && conditional_needs_arm_fill && !speculative_lowering && materialize_owned_temporaries`. **The dot-`len` receiver path does not reach this gate** (DEF-182) |
| Conditional arm helpers | `lower.rs:1810-2000` (`materialize_owned_temporaries`, `conditional_borrows_place`, `conditional_needs_arm_fill`, `conditional_has_ownerless_projection_arm`, `hoist_view_conditional_arms`, `materialize_guard_operand`, `hoist_ownerless_conditional_arms`, `own_ownerless_conditional_arms`) | The batch E fix round's machinery; a consuming projection copies the select instead of moving the chosen arm's member (DEF-176), a guarded intrinsic argument materializes into a temporary aliasing the place arm (DEF-178), and a nested conditional's store copies (DEF-179) |
| Discard lowering | `lower.rs:940` (`lower_discarded`) | The fallback for an expression statement whose value is not a bare call or the `assert`/`panic` intrinsics (DEF-186's site) |
| Statement path | `statements.rs:179` (`emit_expr_statement`) | The closure-callee branch binds only when `owns_drop` (DEF-185); the named-call branch already uses `owns_drop \|\| maybe_owns`; the `Call` arm lowers through `lower_call`, which **misses the intrinsic fold** (DEF-256) |
| Provenance suppression | `statements.rs:1050-1058` | Sets `suppress_elision` for a local- or static-derived return; the parameter derivation is not verified (DEF-187) |
| Elision pre/postlude | `place.rs:1189` (`elision_prelude`), `place.rs:1206` (`elision_postlude`) | Counts view-like lend candidates (`is_lend_candidate`); reports `E10036` at the declaration when the count is not one; `suppress_elision` overrides. **The actual derivation is not compared against the candidate** (DEF-187); the `str[N]` binding records no loan (DEF-177) |

### Borrow model (`compiler/src/borrow.rs`)

| Helper | Location | Behavior |
|--------|----------|----------|
| `is_elided_view` | `borrow.rs:69` | `str`/`cstring` and slices/repeating arrays of them participate in elision |
| `is_view_like` | `borrow.rs:83` | A view, or a fixed array whose element is a view |
| `is_lend_candidate` | `borrow.rs:98` | `str`, `cstring`, `static str`, arrays, `raw`, handles, and `Array`; a user struct is **not** a candidate (the DEF-187 judgment) |

### Emission (`compiler/src/codegen/`)

| Area | Location | Behavior |
|------|----------|----------|
| Conditional rendering | `expr.rs:86`, `expr.rs:307`, `expr.rs:451`; `calls.rs:190/210`; `cleanup.rs:407/428/754`; `function.rs:501/985/1006/1090` | The conditional spellings; a projection of a conditional in return position mixes the arm spellings (DEF-183) |
| Postfix update | `expr.rs:420` (`UnaryOp::PostIncrement => format!("({operand_c})++")`) | Renders the update inline, sharing the expression with the read (DEF-184) |
| Intrinsic rendering | `calls.rs` (`intrinsic()`), `cleanup.rs:373` (`emit_return`) | The `len` fold lives in the value path; the statement path emits a bare call (DEF-256) |
| Fixed-array address | the value-into-place paths in `lower.rs` (`is_fixed_array()` sites) | The projection of a call result takes `&(call().xs)` without binding the root (DEF-188) |

## Differential expectations

Every accepted shape the batch does not target must keep its emitted C byte-identical. The intended
deltas are: new statement temporaries and per-arm move-outs (the ownership phases), the conditional
return's unified spelling, the postfix temporary, the discarded-intrinsic call spelling, and the
fixed-array root binding. The emission-integrity auditor classifies every corpus delta per phase.
