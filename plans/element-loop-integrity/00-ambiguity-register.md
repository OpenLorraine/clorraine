# Ambiguity Register: Element Loop Integrity (DEF-16, DEF-17)

> **Status**: ✅ GATE PASSED — all 15 items resolved
> **Last Updated**: 2026-09-28 09:10
> **CodeOps Artifact Schema**: 1

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | Plan structure: one plan for both defects or two | one plan with two components / two separate plans | One plan for both, following the DEF-13 + DEF-14 precedent | ✅ Resolved |
| 2 | Scope | Which defects this plan fixes | DEF-16 + DEF-17 only / also DEF-18 / also DEF-18 + DEF-19 | DEF-16 + DEF-17 only; DEF-18 – DEF-20 stay filed outside this plan | ✅ Resolved |
| 3 | Scope | DEF-17 as filed says "nested element loops hang"; the grounded repro shows any `for` (element or C-style) nested in any loop hangs (same root cause) | cover every nested `for` form / restrict to element-loop nesting | User accepted recommendation: cover every nested `for` form; one root cause, breadth stated in the plan and roadmap | ✅ Resolved |
| 4 | Scope | A projection over an array-returning call (`mk()[0]`) emits invalid C | leave as-is and file a new defect (DEF-21) / include in this plan | User accepted recommendation: leave out of this plan; file DEF-21 in the roadmap | ✅ Resolved |
| 5 | Naming & terminology | Plan folder and test-suite names | `element-loop-integrity` with new `element_loop_integrity_spec_test.rs` / `_impl_test.rs` / extend `codegen_ownership_*` / `owned-element-reads` | `plans/element-loop-integrity/` with the two new per-plan suites | ✅ Resolved |
| 6 | Technical | DEF-16 fix approach | narrow materialization guard / also lower constant-index reads to places | Narrow materialization guard: only ownerless temporaries materialize; sub-object reads of container places stay lends/views | ✅ Resolved |
| 7 | Technical | DEF-17 fix approach | store the step block in the IR shape / fix the `for_step_block` search | Store the step block in the IR shape at construction; delete the emitter's search | ✅ Resolved |
| 8 | Technical | How the IR shape carries the step | `step: Option<BlockId>` on `BranchShape::Loop` (`None` for `while`/`do-while`) / a dedicated `For` variant | User accepted recommendation: `step: Option<BlockId>` on `BranchShape::Loop` | ✅ Resolved |
| 9 | Behavioral | An owned element read passed to an unmarked/`const` owned parameter (`byval(arr[0])`) | lend, no diagnostic (spec/03-ownership.md §2: caller keeps) / reject with E10026 | User accepted recommendation: lend, no diagnostic | ✅ Resolved |
| 10 | Non-functional | Hang-oracle test policy: a nested-loop regression test would hang `cargo test` without a bound | bounded run in both suites (reuse the 30s helper) / impl suite only | User accepted recommendation: bounded run in both suites | ✅ Resolved |
| 11 | Verification | Task gates and the final gate | `make check` per task + `make verify` before completion + host ASan probes at close-out / check-only | `make check` per task, `make verify` (all hosted-amiga legs) before completion, ASan double-free probes at close-out | ✅ Resolved |
| 12 | Naming & terminology | New internal names: the ownership predicate and the shape field | `projects_from_place` + `step` / other | User accepted recommendation: `projects_from_place` + `step` | ✅ Resolved |
| 13 | Edge cases | Which repro forms the spec suite must cover | full matrix (constant and runtime index, field-of-element, borrow-param base, global base, every non-take mode) / minimal repros only | User accepted recommendation: full matrix (every legal form; module-level owned arrays are rejected, so a global base cannot appear) | ✅ Resolved |
| 14 | Security & compliance | Threat surface | none: compiler-internal correctness (memory safety is the subject), no user input, secrets, network, or storage | Not applicable | ✅ Resolved |
| 15 | UX & presentation | User-visible diagnostics | single option: no new messages; `E10026` and all existing codes stay unchanged | No user-visible change | ✅ Resolved |

### Resolution Notes

**AR-1 / AR-2:** Confirmed in the Phase 1.1 interview (2026-09-28 08:5x): one plan,
DEF-16 + DEF-17 only. The narrowed scope keeps DEF-18 (move through a borrow element),
DEF-19 (`len` on an owned `String`), and DEF-20 (narrowed-shadow `E10010`) out.

**AR-4:** Detected 2026-09-28 (Phase 1.2): a function may return a fixed array (hidden
out-parameter, `compiler/src/codegen/function.rs:74`), but `mk()[0]` emits
`cl_tests_cos_read12_mk()[0]`, which does not compile. Pre-existing and separate from both
defects; the narrow guard does not change it.

**AR-6:** Grounded at `compiler/src/own/lower.rs:659` `materialize_owned_argument` and the
`materialize_owned_argument` caller in `own/call.rs:175`. Member reads over locals already
lower to `ValueKind::Place` (verified by an IR probe), so only element reads
(`ValueKind::Index`) become a second owner; the guard is extended to recognize
sub-object reads of container places. The existing owned-temporary path (a call result
passed to a borrowing callee, e.g. `bytes(stringOf("hi"))`) must keep materializing; the
guard keys on the value being a projection of a place, not on the type.

**AR-7:** Grounded at `compiler/src/codegen/control.rs:407` `for_step_block`: the
closure search finds the loop's own pre-header (its init block jumps to the head) because
the enclosing loop's back edge re-enters it; the pre-header is emitted as the C `for` step
and the true step block becomes dead code. Reproduced: element-in-element, element-in-while,
and C-style-in-while all hang; two sequential element loops are correct.

**AR-9:** Spec-derived: `spec/03-ownership.md` §2 lists unmarked `x: T` and `const x: T` as
read-only lends with "caller keeps", so no ownership transfer or drop happens at the
argument; no diagnostic applies.

**AR-12:** Proposed names: `projects_from_place` (the ownership predicate: a `Place`, a
`Member` of one, or an `Index` of one at any index) and `step` (the new
`BranchShape::Loop` field).
