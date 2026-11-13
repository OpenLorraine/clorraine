# Design: Constant-Array Codegen Residuals (PLAN-36)

> **Document**: 03-design.md
> **Parent**: [Index](00-index.md)

## 1. Shared Template Data Helper

`compiler/src/codegen/program.rs` keeps the existing narrow predicate in one
place and splits data emission from the copy:

- `emit_constant_array_template_data(ty, value, volatile, local_taken) ->
  Option<String>` validates the shape and emits the block-scope `static const`
  declaration. It returns the generated name. The module path passes an empty
  `local_taken` set; the function path passes the current function's local and
  parameter C spellings.
- `emit_constant_array_template(target, ty, value, volatile) -> bool` is the
  module-level wrapper. It calls the helper and emits
  `memcpy(target, name, sizeof(target))`.

The helper is `pub(super)` so the function emitter can call it without
duplicating the element-type table, the 16-element/64-byte floor, the
complete-list check, or the collision-counter loop.

## 2. Function-Local Call Site

`compiler/src/codegen/function.rs` adds one branch to the top-level
`Statement::Assign` path for an `ArrayLit` value with fully known extents:

1. Compute `volatile = fill_target_volatile(target)` without rendering the
   place.
2. Call `emit_constant_array_template_data`. If it returns `None`, fall through to
   the existing `reads_target`/store path unchanged.
3. If it returns a name, render the place once with `self.place(target)`,
   build the `ArrayTarget`, and emit
   `memcpy(array_target.address, name, array_target.bytes)`.

The helper is called **before** the place is rendered. An ineligible shape
therefore takes the existing path without evaluating a run-time index or
other place-side expression twice.

The target can be a plain local, an array parameter, a direct struct field, or
a constant-indexed element. An element target with a run-time or unknown
index keeps the stores: a `memcpy` of a known-size
row into an element GCC can prove out of range makes `-Wstringop-overflow`
reject the strict build before the guard can trap, while the store form builds
clean and traps at the same point. `ArrayTarget` already supplies the correct address and byte count for
each: `&(t)` and `sizeof(t)` for an object, the adjusted parameter name and
`sizeof(t[0]) * extent` for a parameter, and `&(h.values)` for a field.

## 3. Safety Argument

| Concern | Why it is safe |
|---|---|
| Initialization position | The `static const` declaration has no run-time effect; the `memcpy` is emitted at the same statement position as the stores it replaces |
| Loop re-entry | The `memcpy` runs on every execution of the assignment; a full list covers every element, so no element keeps a prior value |
| Writable destination | The template is read-only; the destination remains the original writable object and keeps its address |
| Ownership/drops | The predicate admits only 2- or 4-byte scalar leaves, which never own storage; `needs_drop` is false and the skipped `emit_reassigned_target_drop` is a no-op for this shape |
| Guards | Every element must be a guard-free constant, so there is no evaluation order, division, bounds check, or short-circuit to preserve |
| Volatile | `fill_target_volatile` covers a volatile global and a volatile member; the helper refuses the template |
| Freestanding | The runtime `memcpy` is a byte loop, so the helper refuses the template |
| Symbol collision | The counter advances past any `is_taken_symbol` name, including a user `ffi __cl_ctmpl_0`, and the local path also passes the function's local C names (`local_taken`) so a user local named `__cl_ctmpl_N` cannot be shadowed |
| C validity | A block-scope `static const` array with constant initializers is valid C99; the host and m68k GCC and vbcc legs build the shape. A runtime-indexed element target keeps the stores to avoid GCC's out-of-range `memcpy` diagnostic |
| Double evaluation | The eligibility check runs before the place is rendered; the existing path renders the place exactly once |

The branch does not change the module-level path: `emit_global_init` still
calls the wrapper, and the helper's predicate and data text are the same.

## 4. Benchmark

`benchmarks/local-table/` adds a 256-entry function-local `i32` sine table,
scanned once, with a position-weighted `u32` checksum. The values match the
probe set. The checksum is order-sensitive and non-zero, so a skipped, zeroed,
or reordered table changes the output. The scored row is listed in
`benchmarks/README.md`.

The benchmark's base measurement uses the base compiler (stores) and the head
measurement uses the new template; the comparison is the guard for the new
shape.

## 5. Rejected Alternatives

| Alternative | Reason |
|---|---|
| Emit the template from the recursive `emit_array_assign_to` | It also handles row temporaries and nested fills; a template there would multiply declarations and change shapes outside the intended top-level assignment |
| Gate the local template on `Statement::Assign` being a declaration | The IR has no separate declaration statement; the assignment form is the declaration's initializer and a later `t = [...]` assignment is the same shape. Restricting to "first assignment" would need a new IR fact for no safety gain |
| Size-aware cap for the 2–5 KB band | Re-measurement shows the template is a large `.text` win for +0.4% instructions at 1024; a cap would trade that win away and could regress `.text` for a scored benchmark in the band |
| Store-packing rule for 1-byte tables | A new emission path with no measured need; GCC already packs adjacent byte stores, and the template loses 211 instructions at 256 bytes |
| Template for short lists | It would zero-fill the tail on every execution, changing an unmeasured semantic; the tail policy needs its own ruling |
| Module `const` arrays | `E10224` is a checker/language change; out of scope |
