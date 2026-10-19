# Current State

All fifteen rows are accepted at the planning baseline `efc68f9`, except where a sibling spelling
already rejects. Each filed reproduction is rebuilt under the host AddressSanitizer; the observed
result is the sanitizer's report, or a clean run when the filed shape is an escape that the current
walk does not reach.

| Row | Repro (filed) | Observed at `efc68f9` | Site |
|-----|---------------|-----------------------|------|
| `DEF-485` | `const h2: Holder = c ? h1 : h3;` then `b.v = ...` and a read | accepted; ASan heap-use-after-free | `own/statements.rs:824-859` (`check_const` copies positions only for a view-like target; the carrier branch exists in `check_let` only) |
| `DEF-486` | `g = c ? h1 : h3;` where an arm carries `b.v` | accepted; ASan heap-use-after-free | `own/expr.rs:1544-1579` (global carrier branch inspects a place only, not a conditional's arms) |
| `DEF-487` | `return pick(xs, 0)` after `xs[0] = a.v;` | accepted; ASan heap-use-after-free | `own/statements.rs:1597-1599` (`view_frame_loan_owners` does not resolve a call source); `own/place.rs:838-857` (`carrier_return_owners` early-returns for a view return) |
| `DEF-488` | `store(make(a.v), borrow slot)` | accepted; ASan heap-use-after-free when the slot is read | `own/call_check.rs:621-704` (`frame_owned_view_owner` walks a single-lend view call only, not a carrier-returning call) |
| `DEF-489` | `let h = c ? Holder { names: ["AA","BB"] } : make(a.v); store(h, slot);` | accepted; ASan heap-use-after-free | `own/place.rs:1414-1561` (copy collector handles literals and conditional arms; a call arm contributes no positions) |
| `DEF-493` | `let h: Holder = makeHolder(b.v); b.v = ...; read h` | accepted; ASan heap-use-after-free | `own/place.rs:1560` (`collect_place_copy_loans` reads a call result as a place with no recorded positions) |
| `DEF-495` | closure, func-typed local, generic, and `unsafe`/conditional wrapper calls | accepted; ASan heap-use-after-free | `own/place.rs:2227-2274` (`call_view_source` resolves a named declaration only); `:2068-2112` (`call_loans` the same) |
| `DEF-496` | `b.store(borrow slot)` and `mk().store(borrow slot)` | accepted; ASan heap-use-after-free | `own/call_check.rs:482-554` (the cross-frame loop inspects written arguments only, not the receiver) |
| `DEF-497` | `struct H { name: String; tag: static str; }` with a caller-backed `tag`, local carrier passed out | accepted; ASan heap-use-after-free | `own/call_check.rs:633-637` (`recorded_position_frame_owners` returns early on the recorded static position and never judges the owned field) |
| `DEF-498` | `let h: Holder = c ? h : h;` naming the outer binding | accepted; ASan heap-use-after-free | `own/statements.rs:368-371` (the copy source is snapshotted for a plain place, but the conditional arm collector runs after the declaration) |
| `DEF-499` | `for (borrow e in xs) { e = a.v; } return e;` | accepted; ASan heap-use-after-free | `own/library` alias return path (`view_frame_loan_owners`/`carrier_return_owners` names no alias container) |
| `DEF-501` | `sink(take (c ? h.name : null))` in `f<T>` at `T = H { name: String }` | accepted; ASan double-free | the take source walk (`own/expr.rs` `Mode::Take`) keeps the opaque arm type, so no per-instance duty |
| `DEF-502` | `sink(take (c ? xs.name : null))` at `T = raw H` | accepted; ASan double-free | `own/instance_check.rs` field step does not strip `TyKind::Raw` before resolving the member |
| `DEF-503` | `S { name: c ? h.name : null }` and `s.name = c ? h.name : null` in `f<T>` | accepted; ASan double-free | the struct-literal field and projected-assignment paths record no per-instance duty |
| `DEF-504` | `return c ? (d ? h1 : h2).name : null` at `T = H { name: String }` | accepted; ASan double-free | the projection move-source walk does not descend into a conditional arm |

## Test placement

The batch reuses the suites that already own the neighboring shapes:

| Rows | Suite |
|------|-------|
| `DEF-485`, `486`, `493`, `497`, `498`, `499` | `compiler/tests/borrow_store_integrity_spec_test.rs` |
| `DEF-487`, `488`, `489`, `495`, `496` | `compiler/tests/eval_emission_integrity_spec_test.rs` |
| `DEF-501`..`DEF-504` | `compiler/tests/borrow_generic_integrity_spec_test.rs` |

The same suites' `_impl_test.rs` pairs receive the emission pins.
