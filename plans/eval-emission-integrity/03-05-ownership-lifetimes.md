# Ownership Lifetimes and Releases: Evaluation and Emission Integrity

> **Document**: 03-05-ownership-lifetimes.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 5 closes the memory holes: a view bound from an owned projection temporary takes a
binding-scoped owner (DEF-95), a defer-preempted ordinary return releases its abandoned value
(DEF-96), a func-value call materializes its owned temporary arguments (DEF-125), and the capture
duties cover the block-bodied forms and conditional arms (DEF-126, DEF-127).

## Architecture

### Current Architecture

- The adopting carrier (`ValueKind::Materialize { adopting: true }`, `own/lower.rs:921-950`) is
  non-releasing by design; the view binding records only loans (`own/statements.rs:477-510,563-597`),
  and the temp-owner walk has no `StructLit` arm (`own/lower.rs:1130-1191`), so `let s: str = (Box { v: mk().a }).v;`
  leaks the backing per iteration.
- The supersede gate (`codegen.rs:143-151,683-695`) arms only inside a deferred copy
  (`:691-693`), so a deferred `break`/`continue` preempting a pending ordinary return leaves the
  abandoned `__cl_ret` unreleased (`cleanup.rs:687-690`).
- `lower_call`'s modes block runs only when `receiver.is_none() && !local_callee`
  (`own/call.rs:211`), so a func-value call passes an owned temporary inline and leaks it.
- A block-bodied capture return routes to the not-lend duty (`own/place.rs:446-485`), reporting
  `E10034` for a capture; the sema recorder is gated to expression bodies
  (`sema/expr.rs:304-335`).
- `place_root_name` has no `Conditional` arm (`sema/mod.rs:692-699`), so `c ? x : x` escapes both
  the rejection and the recorder and the emitted C double-frees.

### Proposed Changes

1. **Binding-scoped owner (DEF-95).** When a view binding's lowered chain reads an adopting
   `Materialize`, the backing binds through the existing owner machinery
   (`materialize_owned_argument(value, false)` tail `own/lower.rs:1689-1719`, `declare_local`,
   `move_binding_to_body_scope:1729-1742`), so the binding's scope cleanup releases it — the loop
   re-assignment makes a body-scope owner safe (O3, spec/03-ownership.md:96-97). Supporting: the
   temp-owner walk and `bind_ownerless_projection_root` descend `StructLit` fields
   (`:1130-1191`, `:1357-1409`).
2. **Preemption release (DEF-96).** The gate arms when an ordinary result-var store's deferred
   scopes contain a defer with an out-edge (`defer_region_has_out_edge`, `cleanup.rs:931-953`);
   the existing supersede release then drops the pending value. Ordinary returns with no reachable
   preemption keep byte-identical emission.
3. **Value-call argument modes (DEF-125).** For a func-typed binding callee, the call takes the
   modes from the binding's own `func` type (`types/mod.rs:80-91`) and runs the same
   `materialize_owned_argument` loop; the walk (`own/call_check.rs:55-70`) and
   `take_move_out_places` (`own/call.rs:386-420`) mirror for take/borrow effects. The
   module-callee identity gate stays.
4. **Block-bodied capture duties (DEF-126).** The not-lend duty is suppressed for capture
   bindings (`Binding.base == Some(EnvField)`); the capture-move recorder covers block returns and
   takes (`sema/stmt.rs:59`, `sema/call.rs:267,572`), so the ruled `E10401` reports per instance.
   The closure batch's block-body pin updates to the ruled code as its documented consequence.
5. **Conditional arms (DEF-127).** The rejection and the recorder recurse conditional arms
   (mirroring `conditional_move_sources`, `own/expr.rs:109-133`) with one report per site
   (dedupe by function/capture/span).

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `own/{lower,statements,place,call,call_check}.rs` | owners, modes, duties |
| `own/instance_check.rs` | the duty audits (reused) |
| `sema/{expr,stmt,call,mod}.rs` | the recorder scope and the conditional recursion |
| `codegen/{cleanup,control}.rs` | the preemption release |
| the RV-004 pin | updated deliberately (block take → `E10401`) |

## Code Examples

```clorraine
// DEF-95: the binding owns the backing; ASan-clean per iteration
while (cond()) {
    let s: str = (Box { v: mk().a }).v;
    use(s);
}

// DEF-96: the preempted ordinary value releases on the break edge
while (i < 5) {
    defer { if (i == 1) { break; } }
    if (i == 1) { return stringOf("ab"); }
    i = i + 1;
}

// DEF-125: the owned temporary argument binds and releases
func use(f: func(borrow String): String): i32 {
    return i32(len(f(stringOf("x"))));
}

// DEF-127: one E10401 per site
func outer<T>(x: T): T { return ([x] () => c ? x : x)(); }
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| View bound from an owned temporary | accepted; scope-owned release | AR-B2 |
| Preempted ordinary return | accepted; release on the edge | AR-B13 |
| Owned temporary argument to a value call | accepted; materialized | AR-B13 |
| Block-bodied capture return/take | one `E10401` per instance | AR-B12 |
| Conditional capture move | one `E10401` per site | AR-B13 |
| Duplicable instances, non-preempting returns | unchanged | AR-B12, AR-B13 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-32 – ST-41 ([07-testing-strategy.md](07-testing-strategy.md)), including
  the per-iteration ASan run, the preemption leak probe, the value-call leak probe, the four
  conditional rejections, and the duplicable/non-preempting controls.
- Implementation pins: the owner binding and release counts, the preemption-edge drop shape, the
  argument materialization shape, the duty/report counts, and byte-identity where pinned.
