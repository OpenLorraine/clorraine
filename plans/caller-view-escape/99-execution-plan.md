# Task T-06: Reject an escaping caller-retained view store

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 12/12 tasks (100%)
> **Commits**: `972c9b7` (implementation and tests); this plan's own commit
> **Reasoning**: high — semantic borrow rule and a critical memory-safety defect

## Objective

Close `DEF-540` for a view stored into a **caller-retained container field or
element**. A nonduplicable container parameter is passed by reference; a view
of a different lend parameter stored into its element outlives the lend's
call scope, so the caller can reassign the backing and leave a dangling view.
The store reports one `E10033` at the field or element store.

**Smallest viable design:** use the existing concrete `check_escaping_view_store`
and the per-instance opaque-store audit. Reject only when the substituted
container root is caller-retained and is not a whole-value copy (a struct passed
by value), and the source includes a parameter root that is not the destination
root. A type-parameter value or a concrete view field under a generic root is
recorded for the instance audit; a fixed-array or slice root is not a copy even
though the type table calls it duplicable. No callee summaries, call-site loans,
or new interpreter machinery.

## Decisions and boundaries

- The frozen `spec/16-borrow-rules.md` BR-5 (call-scoped lend) and BR-8 (no
  escape into a field or global) win over the Batch V acceptance pins that
  accepted a nonduplicable caller-retained view store.
- A duplicable **struct** root is copied at the call; the store fills the
  callee's copy and stays accepted. A fixed array decays to a pointer and a
  slice copies only its pointer and length, so neither is a whole-value copy.
- A frame-local container, a `take` container, a static backing, a copy within
  the same carrier, and a whole `borrow T?` out-slot assignment (including a
  captured out-slot) keep their existing behavior. A non-nullable writable view
  parameter is a caller-retained slot, not an ownership slot, and rejects a
  parameter-backed view.
- A whole out-slot assignment from another lend parameter remains a separate
  critical, `DEF-541`; it conflicts with the earlier AR-20 acceptance. The
  conservative rejection is now planned in
  `../out-slot-view-escape/99-execution-plan.md`; it is not silently changed
  here.
- A nested lifted-lambda capture relays the original whole-out-slot kind; a
  captured non-nullable writable view slot is not an ownership slot.
- A view-returning named call whose return expression mentions no parameter is
  static or module-backed, not a statement temporary; the call-site source
  summary uses the same return-expression names as the carrier path.

## Tasks

- [x] T-06.1 Add the failing container-field/element store expectations
  (generic, plain, concrete, frame-local control, duplicable-copy control).
- [x] T-06.2 Reject the concrete nonduplicable caller-retained field/element
  store with one `E10033`.
- [x] T-06.3 Record parameter-backed source roots in the opaque-store duty and
  reject the same shape per instance.
- [x] T-06.4 Retarget the Batch V `Wrap2` acceptance pin and the runtime
  view-store implementation pins to safe shapes.
- [x] T-06.5 Run focused suites and `RDC_ASAN=1` controls.
- [x] T-06.6 Run `make check` and `make verify`; file `DEF-541` for the
  whole-out-slot residual.
- [x] T-06.7 Record a type-parameter value and a concrete view target under a
  generic root for the instance audit, without repeating the owning judgment.
- [x] T-06.8 Treat a fixed-array or slice root as non-copying; keep a
  duplicable struct root accepted.
- [x] T-06.9 Carry the whole out-slot kind through a lifted-lambda capture.
- [x] T-06.10 Retarget the generic/array/capture tests to the hardened rule and
  add the generic value, generic root, fixed-array, slice, call-result, and
  captured out-slot controls.
- [x] T-06.11 Relay `capture_whole_out_slot` through a nested lambda and pin the
  nested captured whole-out-slot control.
- [x] T-06.12 Use the return-expression summary for a named view-returning call;
  keep a static-return call with an unused scalar or lend argument accepted.

**Verify**: `make check` and `make verify` in the Docker SDK container.
