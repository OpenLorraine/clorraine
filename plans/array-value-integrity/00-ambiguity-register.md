# Ambiguity Register: Array Value Integrity

> **Status**: ✅ GATE PASSED — all 12 items resolved
> **Last Updated**: 2026-09-29 07:53

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | Which defects this plan implements | array-value cluster (DEF-15 + new array-assignment defect + DEF-21 + DEF-19) / all six remaining defects / element-ownership cluster first / criticals only | The array-value cluster: DEF-15, the new fixed-array assignment defect, DEF-21, DEF-19; DEF-18 and DEF-20 follow in a second plan | ✅ Resolved |
| 2 | Scope | New defect found during planning: fixed-array value assignment (`let b: i32[2] = a;`, `let r: i32[3] = m[0];`, `let c = s.v;`, whole-array move) is accepted by `clc` and emits invalid C | file a new DEF and include the fix / file only / fix without filing | File as a new roadmap defect and include it in this plan; it is the prerequisite of the DEF-15 copy form | ✅ Resolved |
| 3 | Naming & terminology | Identifier and title of the new defect | `DEF-22` · "Fixed-array value assignment emits invalid C" / other title | `DEF-22` with that title (next sequential id) | ✅ Resolved |
| 4 | Technical | How fixed-array copy/move reaches C, where C arrays are not assignable | whole-array byte copy through the C runtime memcpy, source zeroed on move / generated element-wise loops / wrap arrays in C structs | Byte copy through the C runtime `memcpy`; a move additionally zeroes the source; no new layer, dependency, or harness (Complexity Escalation Gate not triggered) | ✅ Resolved |
| 5 | Behavioral | What a projection over an array-returning call supports: `mk()[i]` | materialize the call result once, then support duplicable reads and view lends while element moves stay E10026 / also support element moves out of the temporary / reject all projections | Materialize the call result; duplicable reads and view lends work; element moves stay E10026 (`spec/03` section 4) | ✅ Resolved |
| 6 | Behavioral | `len` of a fixed multi-extent array `T[N][M]` | outermost element count N (rows) / total scalar count N×M | Outermost element count N; the element loop uses it as its bound, matching slicing and the slice path | ✅ Resolved |
| 7 | Edge cases | Self-referencing fixed-array assignment (`a = a;`, or an RHS that reads the target) | extend the existing save path to arrays (save the old target by byte copy, store the new value, release the saved old) / reject self-referencing array assignment | Extend the save path to arrays, following `spec/03` O3 and the existing droppable-value handling | ✅ Resolved |
| 8 | Integration points | Verification commands for every task and the close-out | `make check` per task, `make verify` at close (project standard) / other | User accepted: `make check` per task, `make verify` at close | ✅ Resolved |
| 9 | Naming & terminology | Plan folder, component docs, and test suite names | `array-value-integrity`, `03-01-multi-extent-element-loops.md`, `03-02-array-value-projections.md`, `array_value_integrity_spec_test.rs` / `_impl_test.rs` | User accepted this naming | ✅ Resolved |
| 10 | Scope | DEF-18 (move through a borrow element view) and DEF-20 (spurious E10010) while this plan runs | include them / leave them filed for a second plan | Leave both filed; a second plan follows this one | ✅ Resolved |
| 11 | Scope | `spec/` and the diagnostics registry | change them to fit the implementation / keep the frozen oracle | Keep the frozen specification and the registry unchanged; every expectation comes from the existing spec | ✅ Resolved |
| 12 | Security & compliance | Security surface of this change | compiler-internal codegen only / new input or data path | Compiler-internal; no user input, no secrets, no persistent data, no network surface | ✅ Resolved |

### Closure Note

Execution honored all 12 items; no runtime ambiguity required a new entry. The
phase reviews and execution filed adjacent pre-existing defects instead
(`DEF-25` – `DEF-29`). Each authorized adjacent fix carries its ruling in the
phase review blocks of `99-execution-plan.md`: the save-branch reachability
correction (`DEF-25`), the row-unit slice fix (Phase 2 review, RV-201/RV-202),
the literal `panic` form (`DEF-28`), and the slice-guard, constant-index guard,
and `__cl_ret` fixes.

### Resolution Notes

**AR-1:** The plan covers the array/string value cluster. DEF-18 and DEF-20 stay in the roadmap for a second plan; DEF-15 and DEF-21 are the remaining critical defects, and DEF-19 is the `len(String)` value bug.

**AR-2:** Live probes on 2026-09-28 with `target/debug/clc` (host profile): `let b: i32[2] = a;`, `let r: i32[3] = m[0];`, `let c: i32[2] = s.v;`, and a whole-array move all compile and then fail the C compiler with "assignment to expression with array type". `spec/03` section 3 requires copy for duplicable aggregates and move for move-only aggregates, so every one of these is a spec violation. The DEF-15 non-`borrow` element-loop form copies each row (`spec/08` LOOP-1), so it cannot work before fixed-array assignment works.

**AR-3:** `DEF-22` is the next sequential defect id after `DEF-21`; the title names the observed failure (accepted fixed-array value assignment emits C that cannot compile).

**AR-4:** The generated C can copy a fixed array with the C runtime `memcpy`: host and hosted-amiga reach `memcpy` through the `<string.h>` their shims include, and the freestanding shim defines it for its `-nostdlib` link (`spec/17` section 12). `drop.rs` already drops fixed-array elements first-to-last. The change reuses both; it adds no runtime helper, layer, dependency, framework, harness, or infrastructure, so the Complexity Escalation Gate does not trigger. (Preflight correction: an earlier draft named a `cl_memcpy` shim helper, which does not exist on freestanding and is not authorized for the move lowering by `spec/17` section 12; the decision itself — a whole-array byte copy — is unchanged.)

**AR-5:** `mk()[i]` currently reaches `ValueKind::Index` with a hidden out-parameter call as its base and renders `cl_mk()[i]`, which no C compiler accepts. The existing `ValueKind::Materialize` machinery defers side-effecting operands into a compiler temporary but deliberately excludes fixed arrays ("a fixed array cannot be assigned to a temporary in C" — `own/lower.rs`), so fixing the assignment path is what lets the projection materialize. Moving an element out of a temporary array is an element move and stays `E10026` in v1 (`spec/03` section 4), consistent with the local-array case (`let s: String = a[0];` reports `E10026` today).

**AR-6:** The user chose the outermost element count. `spec/04` section 9 says `len(xs)` gives the element or byte count; `spec/08` AR-3 makes the elements of `T[N][M]` rows of type `T[M]`, and the slice path (`T[][M]`) already returns the outer count. The fixed multi-extent path currently multiplies every extent (`i32[2][3]` yields 6); the plan changes it to the outermost extent and pins `len(m) == 2` in the test oracle.

**AR-7:** The user chose to extend the existing save path. `emit_assign` already saves the old target when the right-hand side reads it; for a fixed array the save itself is an array assignment, so the save and the store both use the new byte-copy path. This follows `spec/03` O3 (drop the old value, then store the new one) and keeps `a = a;` well-defined.

**AR-8:** `make check` runs `./spec/check.sh` and `cargo test --workspace` in the pinned Docker container; `make verify` adds every hosted-amiga cross-toolchain leg.

**AR-9:** Matches the repository convention of per-plan suites derived from the same failure (see `element_loop_integrity_spec_test.rs` / `_impl_test.rs`).

**AR-10:** Both defects are checker-semantics fixes, not array-value lowering; a focused second plan keeps each plan's gate and review coherent.

**AR-11:** The specification is the immutable oracle; no golden, chapter, grammar, or diagnostic-registry change is authorized by this plan.

**AR-12:** The change alters emitted C for compiler-accepted programs only; it adds no input parsing, data storage, or network path.
