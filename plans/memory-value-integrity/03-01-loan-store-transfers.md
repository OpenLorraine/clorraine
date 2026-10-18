# Loan Transfer on Stored Copies: Memory and Value Integrity

> **Document**: 03-01-loan-store-transfers.md
> **Parent**: [Index](00-index.md)
> **Rows**: `DEF-428` (critical), `DEF-429` (critical, primary), `DEF-431` (critical, verification
> branch), `DEF-435` (major, verification branch)
> **Phase**: P1 · **Lenses**: correctness + semantics + memory-safety (AR-31)

## Overview

Four accepted programs copy view values into a binding, an element, or a projected destination and
then lose the loan the source carried, so a later out-slot store writes a dangling view and the run
reads freed storage. The **primary** fix is the composite-copy gate: skip the exact self-assignment
replacement, write the composite source's positions onto the destination path (element, projected,
or runtime), and keep the collector's prefix fallback in the move transfer. `DEF-431` and
`DEF-435` are verification branches after that gate: if it alone greens them they close as
incidental; if not, the prefix fallback and the runtime-element collector complete them. No new
rejection surface, no new store route, no new IR: every accepted copy stays accepted; the escape
reports the frozen `E10030` (AR-6–AR-9, AR-12).

## Frozen Rule

- Ch 16 §2 (`spec/16-borrow-rules.md:51-58`): binding a local `str` view loans the storage its
  source names; "the loan ends at the last use of the derived view or any alias of it."
- Ch 16 BR-1 (`:187`): a loan never outlives its owner (`E10030`).
- Ch 16 BR-4 (`:190`): a move of the owner requires that no loan of it is live.
- Ch 16 §6 (`:233-236`): a use reachable only through the owner's release reads freed storage and is
  rejected (`E10030`).
- The copy itself is legal: the loan follows the copied view; the escape is the later out-slot
  store.

## Root Cause

The batch-M `DEF-315`/`DEF-372`/`DEF-373` fixes built the element-loan record and the copy
collector. Four call sites remain incomplete:

1. **Exact self-assignment (`DEF-428`).** `bind_value` (`own/statements.rs:793`) prepares and
   applies the binding effect; the view effect calls `set_binding_loans` (`:881`, `:951`;
   `own/place.rs:1096`), which drops the destination's `element_loans` **before** the lowered
   store's copy transfer snapshots the source (`own/expr.rs:1296`). When source and destination
   are the same binding, the snapshot is empty and the copy records nothing. The head already has
   `is_exact_self_assign` (`own/expr.rs:1481-1493`); reuse it.
2. **Composite source into an element/projected/runtime destination (`DEF-429`, primary).** The
   copy collector is gated by `projections.is_empty()` (`own/expr.rs:1299-1306`); a projected
   destination (`grid[0]`, `out2[0]`), an element destination (`g[0]`), and the runtime case
   (`grid[k]`, which reaches `emit_indexed_store`) therefore record no positions, even though
   `record_copy_element_loans` (`own/place.rs:1236`) already takes a `base` path and
   `collect_copy_element_loans` (`:1274`) already unions conditional arms and array literals. The
   runtime branch of `replace_element_loans` (`:1119-1127`) only unions `state.loans`; it records
   no element position. The constant-store sibling `ys[0] = a.v; let s = ys[1]; fill(s)` is
   accepted and clean and must stay that way.
3. **Deeper projection after a shorter recording (`DEF-431`, verification branch).**
   `move_element_loans` (`own/place.rs:1192`) snapshots the source and keeps a path only when it is
   **below** the read (`path.strip_prefix(source)` at `:1212`); the collector's rule
   (`own/place.rs:1369-1398`) also keeps a path that **covers** the read
   (`read.starts_with(path)`), because a fill record covers the element read through it. If
   removing the `429` gate does not green the case, the move transfer needs that prefix fallback.
4. **Element-store nested fill (`DEF-435`, verification branch).** The **constant** destination
   `g[1] = [; a.v]` goes through the ordinary store path and is blocked by the same `429` gate; it
   never reaches `emit_indexed_store`. If the gate alone greens it, it is incidental (the
   `DEF-431` prefix-fallback path). The **runtime** destination `g[k] = [; a.v]` reaches
   `emit_indexed_store` (`own/expr.rs:2557-2629`), which lowers the whole-element value and emits
   `StorePointer` directly; it never calls `record_copy_element_loans`, so a view-like element
   value records nothing and a later constant read escapes.

## Fix Approach

1. **`DEF-428`:** detect the exact self-assignment (`is_exact_self_assign`,
   `own/expr.rs:1481-1493`: `place_target(value)` equals the target binding with the same
   projections) and leave the destination's element-loan record unchanged; the store is a no-op.
   This is the single chosen fix; the snapshot-before-effect alternative is **not** taken (it
   touches every binding effect for the one shape). The self-assignment stays accepted.
2. **`DEF-429` (primary):** drop the `projections.is_empty()` gate for a view-like destination and
   pass the destination's projections as `base` to `record_copy_element_loans`; for a move
   transfer, pass them to `move_element_loans` instead of `[]`. The union is written once (the
   helper already replaces before recording). For the runtime destination, record a wildcard
   `Element(None)` position so a constant read under the runtime-written path matches, and do not
   union every binding loan onto every read (the `ys[1]` sibling control is the oracle). Then run
   the `DEF-431`/`DEF-435` verification branch below.
3. **`DEF-431` (only if the gate does not green it):** in `move_element_loans`, keep a path when it
   is below **or covers** the source read, mirroring `collect_place_copy_loans`; the destination's
   record under `base` is still replaced first. If the gate alone greens the case, close the row
   as incidental with the emitted and red-green evidence instead of adding this fallback.
4. **`DEF-435` (constant half via `DEF-431`; runtime half if needed):** in `emit_indexed_store`,
   when the element type is view-like and the value is a copy source, call
   `record_copy_element_loans` with the element's destination path (`place_target(target)` gives
   the container binding and projections) before/with the `StorePointer`. Reuse
   `collects_copy_positions`; do not duplicate the collector. The constant half closes as
   incidental if the `429` gate greens it.

**Phase-1 verification branch:** after the `429` gate lands and ST-2 is green, re-run ST-3 and
ST-4 on the same tree. If they pass, record the incidental closure in the phase record (the row
still moves to Done with the gate's evidence). If either fails, apply the specific fallback above
and keep the row's red case. This branch is a decision point, not a new task.

## Specification-First Cases (ST-1–ST-4)

All go to `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (AR-30). Each red
case must fail without the fix; each direct control stays green.

| ST | Row | Case | Expected |
|----|-----|------|----------|
| ST-1 | `DEF-428` | `s = s` and `ys = ys` over `xs[0] = a.v`, then `fill`; the in-scope read-back; the direct `fill(xs[0])` control | One `E10030` at each fill after the self-assignment; no TU; the in-scope read-back runs; the direct control keeps its report |
| ST-2 | `DEF-429` | `grid[0] = c ? xs : zs`; `out2[0] = c ? xs[0] : zs[0]`; `g[0] = [xs[0], xs[1]]`; the runtime-destination forms `grid[k] = c ? xs : zs` with `fill(grid[1][0])` and `let s = grid[1][0]`; the direct conditional fill; the constant sibling control `ys[0] = a.v; let s = ys[1]; fill(s)` | One `E10030` at each affected fill, including the direct conditional fill (red at head, control after the fix); no TU; the copies stay accepted; the sibling control is accepted and clean (no over-report) |
| ST-3 | `DEF-431` | `rows[0] = xs; let s = rows[0][0]; fill(s)`; the direct `fill(rows[0][0])` control; the deeper read the prefix fallback must cover | One `E10030` at the copy's fill; no TU; the direct control keeps its report. If the `429` gate alone greens this, the row closes incidental with the red-green evidence |
| ST-4 | `DEF-435` | The constant `g[1] = [; a.v]` copy and its direct read; the runtime `g[k] = [; a.v]` copy and its later constant read; the literal-form fill | One `E10030` at the constant copy's fill (incidental if the `429` gate greens it) and one at the runtime copy's fill; the direct reads keep their reports; no TU; the store itself is accepted |

The impl pins (`_impl_test.rs`) assert the loan record paths and the report code/count/span per
shape, the accepted copy emissions, and a re-emission determinism pin. Only byte-identity pins may
be re-pinned (AR-30).

## Verification

- `make check` per task.
- `make -C docker exec CMD="cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"`.
- `make -C docker exec CMD="RDC_ASAN=1 cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"`.
- The host ASan/LSan probes for the four red shapes (before the fix) and the accepted in-scope
  controls (after). The reds reject after the fix, so the sanitizer leg re-runs the controls; the
  runtime-destination `p429_runtime_dest`/`q429runtime_copy` probes are the ASan reds, and
  `p430_sibling`/`q430sibling` is the sibling acceptance control.
- The batch-M loan matrix and the `E10031` write-conflict controls (`borrow_store_integrity`)
  stay green (batch-M AR-45).
- The `DEF-431`/`DEF-435` verification branch: re-run ST-3/ST-4 after the `429` gate and record
  whether each closed incidental or needed its fallback.

## Risks and Lenses

- **Memory-safety** owns the loan/origin matrix and the ASan leg: a fix that records the loan but
  drops the report, or records it on the wrong path, is caught by the fill-site matrix.
- **Semantics** owns the Ch 16 reading: the copy is legal; only the escape reports. A fix that
  rejects the copy itself is wrong.
- **Correctness** owns the blast radius: every call site of `set_binding_loans`,
  `move_element_loans`, `record_copy_element_loans`, and `emit_indexed_store`; every accepted
  in-scope shape stays accepted.
- **Risk:** the self-assignment no-op must not resurrect stale loans after an overwrite; the
  batch-M per-position drop and the `E10031` write-conflict rule stay. The `E10031` controls are
  the oracle.
- **Risk:** the runtime union is conservative; a naive whole-binding merge would false-reject the
  accepted constant sibling `ys[0] = a.v; let s = ys[1]; fill(s)`. The wildcard `Element(None)`
  position must match only reads under the runtime-written path; the sibling control and the
  `k = 0` expected outcome are the oracle.
- **Risk:** the element-store collector must not record a loan for a non-view element
  (`g[1] = mk()`); guard it with the view-like destination/type, not the store shape alone.
