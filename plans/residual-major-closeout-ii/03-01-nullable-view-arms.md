# Nullable-View Arms and Initializer View Stores: Residual Major Closeout II

> **Document**: 03-01-nullable-view-arms.md
> **Parent**: [Index](00-index.md)

## Overview

Three rows share one mechanism: a nullable-owned value (a `String?` handle) reaching a nullable
view destination widens through `cl_string_view(handle)` without a null test on three positions
the batch-K phase-1 fix left open — the conditional arm (`conditional_arm`), the call argument
(`argument_text`'s conditional route), and the initializer store's member projection. The
destinations accept the value (the frozen `T?` lowering allows a null handle at a nullable view),
so the fix is the per-arm/place null guard the store spelling already carries (`store_text`),
never a rejection (not all of these spellings can be rejected: `let s: str? = c ? p : null;` is
legal and must stay accepted).

## Design

### 1. The conditional arm (`DEF-335`)

The emitted `(c ? cl_string_view(maybe()) : cl_string_view(stringOf("xy")))` dereferences NULL
when the arm's handle is null (the k1d10 probe runs 139; `def335f`/`def335g` show the same for a
local/parameter place arm). `conditional_arm` (`codegen/expr.rs:610`) widens each arm through
`widen_text_view` (`function.rs:2150`); `store_text` (`function.rs:2068`) already guards the
place-rooted source with a null test and the destination's zero view.

Extend the per-arm widening with the same guard: an arm whose source can be null renders
`(handle ? cl_string_view(handle) : cl_zero_ClStr())` (or the equivalent per-type zero helper);
a non-nullable arm stays byte-identical. The guard applies wherever `conditional_arm` widens an
owned handle to a view result — the conditional value, the argument route (`calls.rs:150`), and
the initializer store (see below) — so one helper carries the contract and the siblings cannot
drift. The nested-conditional recursion keeps the guard at each leaf.

### 2. The call argument (`DEF-336`)

k1d15 emits `cl_..._size(0, cl_string_view(__cl_owned0))` and runs 139: the argument's
conditional arm lacks the guard. The `calls.rs:150` route calls `conditional_arm`, so the fix in
§1 covers it; the phase pins the argument spelling separately because the argument's destination
type arrives through the parameter's expected type.

### 3. The initializer store's member projection (`DEF-337`)

`let t: str? = mkBox(false).name;` at module level emits
`cl_k1_e6_..._t = cl_string_view(__cl_itmp1.name);` on a null `name` and runs 139 (the body
spelling is a green control — the batch-K phase-1 fix covered the function store). The
initializer store (~`codegen/program.rs:1052-1077`) wraps a `Handle("String")` source through
`cl_string_view(value)` with no place-rooted guard; the member projection is not a conditional,
so `conditional_arm` is not even reached.

Render the place-rooted guard for the initializer's owned-handle widening exactly as the function
store does (`store_text`'s shape): a source that can be null emits its null test and the
destination's zero view, and the member move-out/release sequence keeps its position. The body
twin and the non-null member run as controls.

## Integration Points

- `compiler/src/codegen/expr.rs:610` (`conditional_arm`, the nested recursion at :634).
- `compiler/src/codegen/function.rs:2150` (`widen_text_view`), `:2068` (`store_text` — the guard
  contract to mirror).
- `compiler/src/codegen/calls.rs:150` (the argument's conditional route).
- `compiler/src/codegen/program.rs` the initializer store arms (~:1052-1077), the member
  projection release (~:1080-1105).
- Controls to keep byte-identical: the non-null conditional (k1d9 runs 2, `k1m1` runs 0),
  the body call (k1d17 runs 0), the body member (k1j1 runs 0).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A null arm at a non-nullable view destination | The existing possibly-null conversion judgment | — |
| An all-null conditional | The existing empty-value renderer | — |
| A non-null handle arm | No guard emitted; byte-identical control | — |
| A guard that cannot be expressed for a fresh call arm without double evaluation | Bind the call's result to one temporary and test it (the `store_text` shape); a reviewer design note if the temporary changes the release shape | AR-15 |

## Testing Requirements

- Spec: the call arm on both selections (null selection returns the null branch's value, never
  crashes; ASan-clean), the argument spelling, the initializer member projection (module level),
  and the body/non-null controls; every accepted program folds a read-back.
- Impl: the emitted arm spelling (the null test + the zero view), the argument spelling, the
  initializer store's guard; the non-null controls byte-identical; a re-emission determinism pin.
