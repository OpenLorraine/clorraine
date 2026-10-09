# Out-Slot Return: Borrow-Store Integrity

> **Document**: 03-02-out-slot-return.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns DEF-101: a direct whole-slot return
(`func cid(borrow v: String?): String? { return v; }`) must transfer the slot's value to the
caller and empty the caller's slot. The user chose the move-out semantics (AR-3), matching the
already-accepted `let local = v` store and the local-return path. The generic instance stays
`E10034`-rejected by the existing borrow-generic-integrity audit (a documented conservatism;
[01-requirements.md](01-requirements.md) Won't Have).

## Architecture

### Current Architecture

`Emitter::emit_return` (`codegen/cleanup.rs:373`) has three value paths:

1. **Fixed-array** — a by-value copy into the hidden destination (`emit_fixed_array_return`).
2. **Result variable / block-local** — the value is stored into `__cl_ret` (or a `__cl_return`
   block-local when releases must run before the return), then `emit_arm_source_moves`
   (`codegen/function.rs:1828`) empties the source place: `emit_move_zero_place` →
   `emit_move_zero`. This is what emits `__cl_ret = s; s = NULL;` for a returned local.
3. **Direct** (`None` result variable and no releases) — emits `return {lowered};` and runs the
   deferred cleanup; **no source emptying**. The out-slot-only function lands here (probe p101f:
   even with unrelated locals, an out-slot parameter forces no cleanup ladder, because the callee
   owns nothing it must release).

The emitted instance (`ClString cl_probe_p101a_cid(ClString *v) { … return (*v); }`) aliases the
caller's slot; both the slot and the returned handle release the same allocation.

### Proposed Changes

In the direct branch of `emit_return`, when the returned value is an owning value whose source is
**caller storage**, emit the block-local form with the existing zeroing step:

```c
{
    ClString __cl_return = (*v);
    (*v) = NULL;
    return __cl_return;
}
```

- **Caller-storage detection:** the lowered value is a place (`ValueKind::Place`) whose base is
  `PlaceBase::Param(name)` with `name` in the emitter's `pointer_params` set
  (`codegen/function.rs:89`, built from `is_pointer_param` — the same information that renders the
  place as a dereference). A by-value parameter is not pointer-tracked and keeps the current
  direct emission; a local/temporary source cannot reach this branch (it would force a ladder).
- **Gate:** `needs_drop` for both the return type and the substituted source type, exactly as the
  result-variable branch gates `emit_arm_source_moves`. Views (`String[]?`, `str`, `cstring`,
  `raw?`) and duplicable payloads need no emptiness and keep their current emission.
- **Ordering:** store the value into the block-local, empty the source, run
  `emit_deferred_return_cleanup`, then `return __cl_return;` — the same order the result-variable
  branch uses (value before cleanup, cleanup before control leaves).
- **Reuse:** `emit_arm_source_moves` / `emit_move_zero_place` produce the zeroing spelling the
  store path already emits (`(*v) = NULL;`); no new zeroing helper.

**Guards (unchanged, pinned):**

- A projected slot return (`return xs[0]`, `return xs[i]`) stays `E10026` (the v1 element-move
  limit; probes p101d/p101e).
- A read-only lend return (`func gid(borrow v: String): String { return v; }`) stays `E10034`.
- The generic instance (`gid<T>(borrow v: T): T` at `T = String?`) stays `E10034` per the existing
  audit; relaxing it is out of scope (AR-3).

## Implementation Details

### New Functions/Methods

- One private helper on the emitter, e.g. `return_source_needs_emptying(value, ret_ty) -> bool`,
  that decides whether the direct branch switches to the block-local form: needs-drop on the
  return and source types and a `PlaceBase::Param` root in `pointer_params`.
- `emit_return`'s direct branch then routes through the existing block-local machinery
  (`unique_local("__cl_return")`, declarator, `emit_arm_source_moves`).

### Integration Points

- `emit_arm_source_moves` and `emit_move_zero_place` are unchanged.
- No IR change: the checker already records the return (and the existing nullable/generic rules
  stand); this is an emission correction.

## Code Examples

### Example 1: DEF-101 (fixed)

```clorraine
func cid(borrow v: String?): String? {
    return v;
}

func main(): i32 {
    let s: String? = stringOf("abc");
    let r: String? = cid(borrow s);
    if (s != null) {
        return 90;              // the caller's slot is empty after the call
    }
    if (r != null) {
        return i32(len(r));     // the caller owns the transferred value (3)
    }
    return -1;
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Direct slot return of an owning handle | Move out: value stored, source emptied, ASan-clean | AR-3 |
| Projected slot return | `E10026`, unchanged | AR-3, AR-8 |
| Read-only lend return | `E10034`, unchanged | AR-3 |
| Generic nullable instance | `E10034`, unchanged (documented conservatism) | AR-3 |

> **Traceability:** every error-handling strategy and design choice back-references the register
> entry that resolved it.

## Testing Requirements

- Specification cases ST-8 – ST-12 ([07-testing-strategy.md](07-testing-strategy.md)), including
  the ASan-clean run and the caller's null-slot observation.
- Implementation pins: the emitted direct-return shape (block-local, `(*v) = NULL;`), the
  unchanged generic rejection, and byte-identical re-emission.
