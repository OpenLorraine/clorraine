# Instance Substitution and Element Surfaces: Residual Major Closeout II

> **Document**: 03-03-element-instance-substitution.md
> **Parent**: [Index](00-index.md)

## Overview

Four rows ride the instance surface batch K phase 2/4 opened: the element store's
overwritten-owner drop (a placeholder-typed destination), the substituted element in a concrete
conversion or member position (no per-instance judgment), the duplicable `borrow out` instance
(the by-value lowering drops the write-through), and the incidentally fixed capture pair as the
phase's control. The common mechanism is the per-instance view of a value that is
placeholder-typed at the declaration site: substitution resolves the concrete type, and the
existing duty/audit pattern (`own/instance_check.rs`) is the hook. The batch-K phase-5 memory
audit filed three of these; the phase-2 reviews filed the fourth.

## Design

### 1. The element store's overwritten-owner drop (`DEF-339`)

`for (borrow v in xs) { v.name = stringOf("zzzz"); }` inside `each<T>` leaks 11 B per store
(LeakSanitizer; the concrete twin is clean). The store's overwritten-owner drop decision runs
against the declared (placeholder) type through `codegen/drop.rs` (`drop_expr_text`,
`emits_drop`), where a type parameter emits no drop. Substitute the destination through the
instance bindings when the store lowers (or record the drop need as an instance duty), so the
overwritten `String` releases exactly once per store on every path (plain, loop, early exit).
The concrete twin's emission is the byte-identity control.

### 2. The substituted element in a conversion or member position (`DEF-345`)

The `e04…e12` spellings (a place/conditional/direct element return, an element `let`, an element
argument, loop element returns/lets) are accepted and fail the strict build because a substituted
element used in a concrete conversion or member position is never judged per instance. Extend
the existing duty pattern: record the conversion/member use at the declaration site (the
`OpaqueConversion`/`OpaqueMemberUse` precedent) and judge it at the instance — accepted where the
concrete sibling is accepted, the frozen `E10200`/`E10212` where it is not. The concrete twins
(`dn4`/`dn5`, `d05c`/`d06`) are the controls; `e01_scalar_member_min` is a pre-existing red
outside this row's scope (an unknown member on a scalar base).

### 3. The duplicable `borrow out` instance (`DEF-353`)

`wrap<T>(borrow out: T, x: T)` at `T = i32`: `([borrow x, borrow out] () => { out = x; })()`
builds and runs 249 (`out` stays 0) — the by-value lowering for a duplicable instance drops the
out-slot write. The fix restores the write-through store for a duplicable `borrow` instance
(the out-slot lowering decision), so the caller reads back the written value; the owning
instance and the concrete twins keep their behavior. No memory effect at the head (probe
`p5_capture_dup_instance` is ASan-clean), so the case is a run value, not a sanitizer case.

### 4. The control: the capture-move pair (`DEF-341`)

`b10_capturereturn` (`let s: String = ([v] () => v.name)();` inside `each<T>`) emitted invalid C
at filing; at the head both the generic and the crafted concrete twin report one `E10401`
(`cannot move captured 'v' in a lambda`) — the batch-K phase-2 C3 correction substituted the
capture-move requirements, so the invalid C is unreachable and the concrete parity holds. The
phase pins the pair (one `E10401` each, no TU) and does not implement anything; a close-out
probe that shows a regression turns the row red here (AR-8).

## Integration Points

- `compiler/src/own/instance_check.rs:78` (`check_capture_move_requirements`), the duty pattern
  (:402, :991, :1039, :1141 the sibling audits), `types::instance_bindings` (:94).
- `compiler/src/codegen/drop.rs:141` (`drop_expr_text`), `:237` (`emits_drop`).
- The out-slot lowering (the by-value/borrow decision in `own/lower.rs`/`codegen`), the
  `check_out_slot_receiver` route.
- `compiler/src/sema/call.rs`/`own/call_check.rs` for the conversion/member duty record.
- Controls: the concrete twins (`s2` concrete, `dn4`/`dn5` concrete siblings), the
  batch-K `p5_capture_dup_instance` sibling, the `checker_conformance` element cases.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A store whose destination stays placeholder-typed (an unkeyable instance) | Keep the declaration path; the existing rejection for an unbounded parameter | — |
| A conversion the concrete sibling rejects | The frozen `E10200` once per site | — |
| A duplicable instance whose out-slot write cannot be emitted | The recorded park: a `borrow` write-through the lowering cannot express becomes a recorded limit (AR-14) | AR-14 |
| The capture control regresses | The row turns red with the fresh probe and is fixed in this phase | AR-8 |

## Testing Requirements

- Spec: the element store leak (plain, loop and early-exit spellings; ASan/LSan-clean; the
  concrete twin as control), the conversion/member instances (the frozen rejection where the
  concrete twin rejects; accepted and building where it accepts), the duplicable out-slot
  (write-through read back), and the capture pair (one `E10401` each, no TU).
- Impl: the substituted drop emission for the element store, the duty records and report spans,
  the out-slot store shape, the rejection parity; a re-emission determinism pin.
