# Current State: Element Loop Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

- The ownership stage lowers expressions to the typed control-flow IR
  (`compiler/src/own/lower.rs`), decides ownership and loans, and binds owned
  temporaries that the scope cleanup releases.
- `materialize_owned_argument` (`compiler/src/own/lower.rs:659`) gives an ownerless
  owned value a scope local (`__cl_ownedN`), because "a call result the callee only
  borrows stays owned by the caller; an unnamed one is bound to a scope local so
  the cleanup drops it". It skips values that already have an owner or a home: a
  `ValueKind::Place`, a view, a duplicable value, a `take` argument, closures,
  unknown types, and type-parameter mentions.
- Call arguments pass through it at `compiler/src/own/call.rs:175`; template
  placeholders pass through it at `compiler/src/own/lower.rs:211`.
- Element reads lower to `ValueKind::Index` (`compiler/src/own/lower.rs:38-51`);
  member reads over a local lower to `ValueKind::Place` through the qualified-name
  path (`compiler/src/own/lower.rs:30-33`), which is why `p.name` argument
  materialization already skips.
- Loops are built by the ownership stage (`compiler/src/own/loops.rs`). A structured
  loop records a `BranchShape::Loop { kind, body, exit }` on its header branch
  (`compiler/src/ir/control.rs:26-34`). Both `for` forms create their `step_block`
  up front (`own/loops.rs:178`, `:421`) but do not record it in the shape.
- The emitter (`compiler/src/codegen/control.rs`) prints a `for` by searching for
  the step block: `for_step_block` (`:407`) takes `closure_from(function, body)`
  and returns the first block whose terminator is `Goto(head)`. The search exists
  because the shape does not carry the step.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/lower.rs` | Expression lowering and temporary materialization | Add `projects_from_place`; skip materialization for such values (03-01) |
| `compiler/src/own/call.rs` | Call-argument materialization call site | None (the guard changes in `lower.rs`) |
| `compiler/src/ir/control.rs` | `BranchShape` definition | Add `step: Option<BlockId>` to `Loop` (03-02) |
| `compiler/src/own/loops.rs` | Loop construction | Record `step_block` at all three `Loop` construction sites (03-02) |
| `compiler/src/codegen/control.rs` | Structured-loop emitter | Consume the stored step; delete `for_step_block` and its closure search (03-02) |
| `compiler/tests/element_loop_integrity_spec_test.rs` | New specification suite | Create (07) |
| `compiler/tests/element_loop_integrity_impl_test.rs` | New implementation suite | Create (07) |

### Code Analysis

**DEF-16 materialization.** In the emitted C for `return vlen(arr[0]);` the
ownership stage binds `__cl_owned0 = arr[0]` and the scope cleanup frees it, while
the array cleanup also frees `arr[0]` — two owners, one buffer. The guard at
`lower.rs:664` tests `matches!(value.kind, ValueKind::Place(_))` only; an
`Index` value is not a `Place`. A runtime index follows the same path (the guard
does not distinguish index kinds), and `ps[0].name` / `b.xs[0]` are `Member` or
`Index` values whose base is not a `Place` value either. An IR probe confirms the
difference: `p.name` lowers to `ValueKind::Place { base: Local("p"), Field(name) }`;
`arr[0]` lowers to `ValueKind::Index { base: Place(arr), index }`.

**DEF-17 step selection.** `for_step_block(function, body, head)` computes
`closure_from(body)` and returns the first block in function order that ends
`Goto(head)`. For a nested loop, the loop's pre-header block — the block holding
its index initialization and jumping to its head — is reachable from the body:
body → step → head → exit → the enclosing loop's remaining body → enclosing step →
enclosing head → enclosing body → this loop's pre-header. The pre-header has a
lower block id than the real step, so it wins. Its assignments print as the C `for`
step (`__cl_tmp0 = 0u`), the real step block is never marked emitted, and when it
is finally printed as a labelled block, its `Goto(head)` prints the loop a second
time (empty). The result is an infinite loop.

## Gaps Identified

### Gap 1: Owned sub-object reads materialize as a second owner

**Current Behavior:** `materialize_owned_argument` binds a scope temporary for any
non-`take` argument that is not a `ValueKind::Place`, including `Index` values and
their projections. The temporary owns a shallow handle copy; the container also
owns the element; both drops run. Reproduced under AddressSanitizer as a
double free for a constant index, a runtime index, a borrow-param base, a struct
field of an element, a borrow-struct field, an unmarked owned parameter, a
`borrow` owned parameter, and a template placeholder.

**Required Behavior:** Such arguments are lends or views (R1); only genuinely
ownerless temporaries materialize (R2).

**Fix Required:** A read predicate that recognizes a place and any field/element
projection of one, used in `materialize_owned_argument` (`03-01`).

### Gap 2: Nested `for` step mis-selection

**Current Behavior:** `for_step_block` returns the loop's pre-header for any `for`
nested inside another loop; the real step is dead, so the loop never terminates.

**Required Behavior:** The loop's own step block is printed in the C header on
every nesting (`03-02`).

**Fix Required:** Record the step block in `BranchShape::Loop` at construction and
use it in the emitter; delete the search.

### Gap 3: Array-returning call projection emits invalid C (filed separately)

**Current Behavior:** `mk()[0]` where `mk(): String[2]` emits
`cl_tests_cos_read12_mk()[0]`, which the host C compiler rejects (the array return
is a hidden out-parameter, `compiler/src/codegen/function.rs:74`).

**Required Behavior:** Out of this plan's scope; filed as `DEF-21` (AR #4).

## Dependencies

### Internal Dependencies

- `value_place` (`own/lower.rs:604`) already walks `Place`/`Member` chains; the new
  predicate is its read-only sibling.
- `closure_from` (`codegen/cleanup.rs:424`) is used by the emitter elsewhere and is
  not changed; only its use in `for_step_block` disappears.
- The `BranchShape` match sites (`codegen/control.rs:98`, `:211`, `:322`) must
  compile after the field is added.

### External Dependencies

- None.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| A too-broad guard stops materializing genuine temporaries (leaks or double frees elsewhere) | Low | High | Predicate keys on place projections only; ST-12 and the impl shape test pin the ownerless-temporary path |
| The `BranchShape` field causes missed match arms | Low | Low | The compiler enforces exhaustive matches; all loops are covered by `make check` |
| Runtime-index element reads rely on guard emission after the fix | Low | High | The statement's guard pass emits guards for the value: `emit_return` (`codegen/cleanup.rs:264`) on the return path and `emit_assign` (`codegen/function.rs:577`) on the assignment path; ST-2 and the trap case pin the runtime guard |
| A nested-loop test hangs the suite before the fix | Certain (red phase) | Medium | Bounded run in both suites (AR #10) |
