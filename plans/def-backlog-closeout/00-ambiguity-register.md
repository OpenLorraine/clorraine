# Ambiguity Register: def-backlog-closeout

> **Status**: ✅ GATE PASSED — 18 planning items + AR-19/AR-20 (runtime) resolved
> **Last Updated**: 2026-10-04 10:56

This register is the Zero-Ambiguity Gate audit trail for the `def-backlog-closeout`
plan. All items were resolved in the planning interview on 2026-10-03.

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| AR-1 | Scope | Which filed defects does the plan cover, and under what name? | All seven in one mechanism-grouped plan named `def-backlog-closeout` / DEF-65 separate / fewer than seven | All seven (DEF-63 – DEF-69) in one plan; folder `plans/def-backlog-closeout/` | ✅ Resolved |
| AR-2 | Scope | DEF-66 was filed before the later base-settlement fix; it may no longer reproduce | Reproduce first / fix regardless / defer out | Reproduce first: task 0 probes the let-store, take, and return forms under host ASan; if clean, add regression pins and close the row as already-fixed, else fix as scoped | ✅ Resolved |
| AR-3 | Scope / Technical | How wide is the DEF-65 pointer-lowering fix? The current code deliberately passes duplicable struct borrows by value (W10001) | Nullable out-slots only / all `borrow` parameters per `spec/17-codegen.md:65` / include duplicable structs | Nullable out-slots only — key on `mode == Borrow && ty.nullable` (`String?`, `File?`, `str?`, `T[]?`); leave the deliberate duplicable-struct by-value path unchanged | ✅ Resolved |
| AR-4 | Behavioral | DEF-69 direction: an unlisted enclosing read must not emit undeclared C | Reject, require listing (needs E10405) / implicit capture / defer | Reject, require the name in the capture list; user approved the frozen-spec change adding **E10405** | ✅ Resolved |
| AR-5 | Behavioral | Nested capture-of-capture: may an inner lambda read an outer lambda's capture without listing it? | List at each level / outer captures flow in | List at each level; the check applies to any binding from an enclosing function or lambda | ✅ Resolved |
| AR-6 | Technical | DEF-67 release strategy for an ownerless call result consumed as a borrow inside a larger expression | Bind intrinsic args and dot-`len` receivers through the existing owned-binding path / expression-wide materialization / filed `len` shape only | Bind intrinsic arguments and dot-`len` receivers; guarded operands keep the deferred path | ✅ Resolved |
| AR-7 | Technical | DEF-68: how the do-while condition release reaches a `defer`-registered return | Collect defer blocks in `append_condition_releases` via a marker scan / record loop membership in `check_defer` / defer out | Collect defer blocks: scan the body's reachable defer markers and union each deferred block's return successors; `Builder::append_return_releases` unchanged | ✅ Resolved |
| AR-8 | Technical | DEF-63 fix shape: where the call-rooted projection gets bound under a guard | Bind the call root through the existing deferred `Materialize` temp when guarded and let `take_move_out_places` derive the place / separate guard-root design | Accepted recommendation: bind the call root through the deferred `Materialize` temp; the guard emitter zeroes the moved slot | ✅ Resolved |
| AR-9 | Technical | DEF-64 emission: `Statement::Move` cannot carry the discarded base fill | Prepare the consume path like stores and emit the base fill as its own statement or a value-carrying store / pre-materialize the root before the literal | Accepted recommendation: prepare the consume path like stores (settle + index chain + root bind) and emit the fill explicitly | ✅ Resolved |
| AR-10 | Technical | DEF-65 call-site mechanics for a pointer-lowered `borrow str?` when a `String`/`str` view conversion is needed | Materialize the view conversion into a temporary before taking its address / restrict the pointer fix to place arguments | Accepted recommendation: materialize the conversion into a temporary before taking its address | ✅ Resolved |
| AR-11 | Technical | Phase grouping and the per-phase shape | Mechanism groups plus close-out / one phase per defect / two large phases | Accepted recommendation: P1 take/move-out (DEF-63+64+66), P2 expression temporaries (DEF-67), P3 borrow out-slot (DEF-65), P4 defer condition release (DEF-68), P5 capture diagnostic (DEF-69), P6 close-out | ✅ Resolved |
| AR-12 | Naming | Plan folder, test suite, and module names | `def-backlog-closeout` names / continuation of the `def_backlog_completion_*` names | Accepted recommendation: `def-backlog-closeout`, `def_backlog_closeout_{spec,impl}_test.rs`, module prefix `spec.closeout.stNN` | ✅ Resolved |
| AR-13 | Integration | The full frozen-spec modification set for E10405 | Registry row + one compile-fail golden + `ASSERTED_CODES` / also restate the rule in `spec/06-functions.md` | Accepted recommendation: registry row + golden + `ASSERTED_CODES`; no `spec/06` chapter edit (L-1/L-2 already carry the rule) | ✅ Resolved |
| AR-14 | UX / Naming | E10405 message text and suggestion | Approved text proposed by the planner | `lambda reads '{name}' without listing it` with suggestion `add '{name}' to the capture list` | ✅ Resolved |
| AR-15 | Non-functional | Which smoother-implementation practices the plan adopts | Adopt all / subset / keep prior workflow | Adopt all: deterministic host-ASan probes at task time, blast-radius and test-integrity steps in every reviewer packet, mechanism-grouped phases, a close-out probe runner | ✅ Resolved |
| AR-16 | Security | Threat surface for these fixes | — | No external input; the fixes are release-count/memory-safety work, guarded by the deterministic host-ASan probes and the repo's auditor dispatch matrix | ✅ Resolved |
| AR-17 | Stakeholder | Competing priorities | — | None; the defects are all filed backlog items with one owner | ✅ Resolved |
| AR-18 | Data & State | Data-model changes | — | None; release/ownership counts per shape are specified in the component docs, no persistent data | ✅ Resolved |
| AR-19 (runtime) | Plan / Technical | Phase 3's plan documents require a nullable duplicable-struct out-slot (`Point?`): 03-03 Proposed Change 1 and its error table, PF-005's resolution, ST-15, and task 3.3.1. The frozen specification forbids nullable structs: `spec/02-types.md:118-124` (TY-4, E10011). Probes on the current compiler: a `Point?` parameter and a `Point?` local report E10011; `Point? == null` reports E10222. A spec test for `Point?` can never pass after any in-scope DEF-65 fix | A (recommended) correct the plan to the frozen-spec boundary: drop the `Point?` sub-case, keep the valid nullable kinds (`String?`, `File?`, `str?`, `T[]?` — arrays are views, per E10036); B amend TY-4 and implement nullable structs (out of scope: R10 allows only the E10405 spec change; large complexity escalation); C keep the ST-15c test red (violates the spec-first green-phase mandate) | ✅ Resolved — correct the plan to the frozen-spec boundary: drop the `Point?` sub-case (TY-4/E10011 forbids nullable structs); keep the valid nullable kinds (`String?`, `File?`, `str?`, `T[]?`); no spec change (R10 holds) | ✅ Resolved |
| AR-20 (runtime) | Implementation / Scope | ST-14 (`borrow slot: str?`, a `str?` place) cannot pass with the planned changes alone. The caller's nullable-view local lowers to the emitted view struct (`ClStr`), but the null literal always renders `NULL`/`0` and `== null` renders a direct comparison, so the host cc rejects `slot = NULL;` and `(slot == NULL)` (`incompatible types when assigning to type 'ClStr' from type 'void *'`; `invalid operands to binary ==`). Handles, `raw`, and `cstring` lower to C pointers and are unaffected; `str`, `static str`, and slice views (`T[]`, the AR-3 `T[]?` kind) are struct-typed. The necessary correction is type-aware null rendering (`empty_state`) plus a `.ptr` null test for struct-typed views in `compiler/src/codegen/expr.rs`, with a small shared helper in `compiler/src/codegen/types.rs`; that adds `codegen/expr.rs` to the phase's expected modification set | A (recommended) extend Phase 3 with this necessary correction (small, local, reuses `empty_state`); B weaken ST-14 to a declaration-shape check (contradicts the approved ST case); C file a new defect and leave the ST-14 case red (violates the green-phase mandate) | ✅ Resolved — extend Phase 3 with the nullable-view null handling (`codegen/expr.rs` added to the expected modification set; helper in `codegen/types.rs`) | ✅ Resolved |

### Resolution Notes

**AR-1:** The OUT boundary: PLAN-05 (developer experience), PLAN-13 (Amiga library
surface), PLAN-14 (generated-C optimization), capture inference, any diagnostic other
than E10405, and unrelated refactors.

**AR-2:** The RV-201 base-settlement change landed after DEF-66 was filed; code reading
suggests the generic store/take paths may already settle and move out. The plan treats
task 0 as a reproduction gate rather than assuming either state. A clean result closes
the roadmap row as already-fixed with regression coverage, not as a no-op.

**AR-4:** Adding E10405 to `spec/14-diagnostics.md` and its compile-fail golden is a
frozen-spec change approved explicitly for this plan. No other spec chapter changes.

**AR-6:** The intrinsic-argument arm currently lowers its argument with plain
`lower_value`; the chosen strategy routes it through the existing
`materialize_owned_argument` path, matching how named calls already bind an owned
argument the callee borrows.

**AR-7:** The deferred block is registered at its `defer` statement, before the
do-while condition exists, so the condition's release list never reaches that return.
The chosen strategy extends the existing successor walk in
`append_condition_releases` to include registered defer blocks; the release-append
mechanism itself is unchanged.

**AR-8:** The guarded take path currently skips the root bind and then finds no place
for a `Member{Call}` root, so the deferred move-out branch is never entered and the
call result leaks. Binding the root through the existing `Materialize` temp makes the
existing place derivation and guard-time zeroing apply unchanged.

**AR-9:** `consume_place` derives a place and emits a bare `Statement::Move`, which
cannot carry the discarded `Materialize` fill. Preparing the path like the store path
reuses the existing settle/index-chain functions; the fill needs its own statement or
the value-carrying store arm.

**AR-10:** A pointer-lowered `borrow str?` receiving a `String` argument needs a view
conversion whose address is valid for the call; materializing the conversion into a
temporary keeps the argument an lvalue without `&(rvalue)`.

**AR-11–AR-13:** Accepted as recommended; see the rows above for the exact content.

**AR-19 (runtime):** The Phase 3 plan required a nullable duplicable-struct out-slot (`Point?`),
but the frozen specification forbids nullable structs (TY-4/E10011). The user ruled to correct the
plan to the spec boundary: the `Point?` sub-case and its test are dropped, the valid nullable
kinds (handles, `raw`, `cstring`, and views, including `T[]?` slices) keep the DEF-65 pointer
treatment, and no specification change is made. A struct borrow can never be nullable in valid
code, so the deliberate by-value deviation stays as-is.

**AR-20 (runtime):** The nullable-view null handling was added to Phase 3 as a necessary
correction: a view is an emitted pointer-and-length struct, so the null literal now renders its
zero struct (`empty_state`) and a null test reads the view's `ptr` member instead of comparing the
struct with a pointer. The change is local to `compiler/src/codegen/expr.rs` with a shared
predicate in `compiler/src/codegen/types.rs`; it also enables the `T[]?` out-slot kind AR-3
includes. Scope mode stays `strict`; no optional addition was accepted.
