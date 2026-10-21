# Task T-07: Reject a foreign lend stored into a whole out-slot

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 0/5 tasks (0%)
> **Reasoning**: high — critical memory safety and a deliberate reversal of an
> earlier out-slot acceptance pin
> **Commits**: none yet; this plan is the resume point

## Objective

Close `DEF-541`: a whole `borrow T?` out-slot assignment must not keep a view
of a **different** lend parameter. The callee's lend is call-scoped, so the
caller can reassign the backing after the call and leave the out-slot view
dangling.

**Smallest viable design:** reuse the existing `check_escaping_view_store`
provenance walk and the per-instance opaque-store audit. At a whole out-slot,
if the stored view derives from a parameter root other than the destination
root, report one `E10034` at the store naming that source parameter. Keep the
slot's own incoming value, static/module backing, and same-slot copy accepted;
keep a local or fresh backing on its existing `E10030` path. Add no
cross-function summaries, call-site loan graph, or new interpreter machinery.

## Decision

| Option | Ruling |
|---|---|
| Reject a foreign lend stored into a whole out-slot (`E10034`) | **Chosen** |
| Preserve acceptance with per-callee view-store summaries and caller-side loans | Rejected for this task |

The frozen `spec/16-borrow-rules.md` BR-5 makes a lend parameter call-scoped
and forbids the callee from storing it; the `borrow T?` exception covers the
slot's own incoming owned value, not a view borrowed from another parameter.
The independently reviewed summary-and-loan path needs order-independent
effect collection, instance-aware composition through forwarding and indirect
calls, and caller-side loan propagation; that is a materially larger surface
than the safety defect requires. The frozen soundness stance rejects when the
checker cannot prove safety.

The acceptance pins that deliberately allowed `slot = p` for a caller-backed
`str`/`T` lend are retargeted to the rejection. A non-nullable writable view
slot is already rejected by the `DEF-540` field/element rule; this task covers
the whole `borrow T?` ownership slot.

## Boundary

| Shape | Treatment |
|---|---|
| `slot = p` where `p` is another lend parameter | Reject with `E10034` at the store |
| A conditional with a foreign lend arm | Reject once with `E10034` |
| A captured or nested-captured whole out-slot | Same rule; the captured whole-out-slot kind already relays |
| `slot = slot` or a copy from the same slot | Accept; no new escaping backing |
| `slot = "static"` or a module global view | Accept; the backing outlives every frame |
| `slot = local` or a fresh call result | Keep the existing `E10030` lifetime judgment |
| `borrow slot: str` (non-nullable writable view slot) | Already `E10033` from `DEF-540`; not reclassified |

## Tasks

- [ ] T-07.1 Write the spec cases: direct, conditional, captured/nested, and
  same-slot/static/local/fresh controls; expect one `E10034` for a foreign lend
  and no translation unit.
- [ ] T-07.2 Implement the whole-out-slot foreign-lend check in the concrete
  `check_escaping_view_store` path; keep the existing lifetime report for a
  local or fresh source.
- [ ] T-07.3 Apply the same judgment in the per-instance opaque-store audit,
  using `parameter_backed_sources` and `source_arms` against the destination
  root.
- [ ] T-07.4 Retarget the cross-frame and out-slot acceptance pins; update the
  findings and roadmap.
- [ ] T-07.5 Run the focused suites, `RDC_ASAN=1` controls, `make check`, and
  `make verify`.

**Verify**: `make check` and `make verify` in the Docker SDK container.
