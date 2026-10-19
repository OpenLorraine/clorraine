# Current State

All eight rows are accepted at the planning baseline `d19a467`. The two audit rows marked
"byte-identical" emit the same C at the last batch's base and head; the generic conditionals
double-free under ASan; the take alias either double-frees or SEGVs.

| Row | Repro (filed) | Observed | Site |
|-----|---------------|----------|------|
| `DEF-449` | `let h2: Holder = c ? h1 : h3;` where `h1.names` holds `b.v` loans; later `b.v = ...` then read `h2.names[0][0]` | accepted; heap-use-after-free; the unconditional-overwrite and mixed place/literal variants reproduce | `own/statements.rs:393-403` (the let copy handles a constant place only); `own/expr.rs:1749-1805` (no loan record for a bare place read at a non-view position) |
| `DEF-450` | `func store(h: Box, borrow slot: str?) { slot = h.v; }` with a local `b` dying before the slot owner reads | accepted; heap-use-after-free | `own/call_check.rs:512-521` (cross-frame rule inspects view-like arguments only); `own/place.rs:865-880` (callee treats a non-`take` aggregate as caller-backed) |
| `DEF-451` | `let s: str = pick([mk().v; "EE"], 0); fill(s, slot)` and the direct argument form | accepted; heap-use-after-free | `own/place.rs:1498`, `:1735-1740` (`view_loans`/`collects_copy_positions` cannot see the lowered owners behind a call argument); `:1784-1810` (`lowered_root_bindings` has no `Call` arm) |
| `DEF-452` | `let ys: str[2] = [mk().v; "EE"]; len(ys[0])` | accepted; the generated owner lives to body scope (`own/lower.rs:5595`) while Ch 03 D5 drops a temporary at statement end | `own/lower.rs:5595`; `spec/03-ownership.md:169` (OW-13), `spec/17-codegen.md:80` |
| `DEF-453` | `let g: str = "AA"; let xs: str[2] = ["AA","BB"]; xs[0] = a.v; g = pick(xs, 0);` then `g[0]` | accepted; base C builds strictly and reads freed storage | `own/place.rs:948` (`view_loan_destination_owners` uses `expr_view_roots`, which has no `Call` arm); `own/expr.rs:1510-1590` (`check_escaping_view_store` does not consult the value loans) |
| `DEF-459` | `v = (g && eat(take (*ps[idx])) == 1) ? x : y` where the callee sets the global `idx` | accepted; the move-out and alias test re-read `ps[idx]` after the call; double-free or SEGV | phase-3 take alias capture (`codegen/function.rs:2736`); `own/expr.rs` take materialization |
| `DEF-468` | `func f<T>(c: bool, h: T): String? { return c ? h : null; }` at `T = String` | accepted; ASan double-free; the concrete twin reports `E10034` | `own/call_check.rs:883` / `own/expr.rs:178` (`conditional_move_sources` skips a type-parameter arm) |
| `DEF-469` | `let r: String? = c ? h.name : null;` with `h: T`, `T = H { name: String }` | accepted; ASan double-free; the concrete twin reports `E10034` | the same conditional-move source walk; the projection type is opaque at the definition, so no per-instance duty is recorded |

## Test placement

The batch reuses the suites that already own the neighboring shapes:

| Row | Suite |
|-----|-------|
| `DEF-449` | `compiler/tests/borrow_store_integrity_spec_test.rs` |
| `DEF-450`, `DEF-451`, `DEF-453` | `compiler/tests/eval_emission_integrity_spec_test.rs` |
| `DEF-452` | `compiler/tests/eval_emission_integrity_spec_test.rs` (control) |
| `DEF-459` | `compiler/tests/conditional_projection_residue_spec_test.rs` |
| `DEF-468`, `DEF-469` | `compiler/tests/borrow_generic_integrity_spec_test.rs` |

The same suites' `_impl_test.rs` pairs receive the emission pins.
