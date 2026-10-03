# Ambiguity Register: Def Backlog Completion

> **Status**: ✅ GATE PASSED — all 13 items resolved
> **Last Updated**: 2026-10-03 11:06
> **Preflight**: 30 findings resolved (PF-001 – PF-030); resolution notes extended above.
> **Execution additions**: AR-13 (runtime), recorded during execution.

## Ambiguity Register

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | DEF-60's filed remedy ("defer the take-root binding through the guard machinery") does not fix its verified double free: re-testing at `d06d451` shows the runtime-index move-out renders `[0]` for `Element(None)` while the callee took slot `i` (also unguarded); the guard-deferred root binding only fixes a different, guarded member-root sibling leak | Fix the index-carrying move-out, correct the DEF-60 row, and file the member-root leak as a new backlog defect / also include the guard-deferred root binding in this plan | Fix the index-carrying move-out; correct the DEF-60 row; file the member-root leak as a new backlog defect | ✅ Resolved |
| 2 | Behavioral | DEF-56 + DEF-61: the generic spellings accept a lend returned/stored/passed as owned and double-free, while the concrete spellings report `E10034` | Reject at instantiation via a post-substitution `E10034` duty centralized in `mark_moved_if_owning` / implement move-on-lend-return semantics / infer `take` (spec change) | Reject via the post-substitution `E10034` duty centralized in `mark_moved_if_owning` | ✅ Resolved |
| 3 | Technical | DEF-57: how the specialized capturing closure's enclosing instance reaches the recorded callable-bound type argument so the callee spelling matches the call site | Tag the recorded closure arg with the enclosing key during instance collection and include it in `type_key`, relocating the key renderer to the stage-neutral `types` module / store the enclosing type arguments in `ClosureTy` / reject in sema (contradicts L-5; needs a spec ruling) | Tag the key at recording and include it in `type_key`; relocate the key renderer to `types` | ✅ Resolved |
| 4 | Technical | DEF-62: which layer releases the do-while condition temporary on an early `return` | Append the condition's pending releases to the body's `Return` terminators in `check_do_while` (checker-local; needs a small builder mutator) / emitter tracks the flag-first condition temp and drops it on every return / reorder lowering (evaluates the condition in the wrong flow state) | Checker-local release append; emitter tracking is the bounded fallback if the design probe shows the temp declaration placement cannot support it | ✅ Resolved |
| 5 | Technical | DEF-59: which layer moves the ownerless element read out of the statement-owned destination temp on the store and return paths | Store via the lower-layer `PreparedStore::Move` gated on `is_destination_temp`, return via the emitter's no-ladder branch mirroring `emit_arm_source_moves` / lower-layer move-out for both paths | Store via the lower-layer `PreparedStore::Move`; return via the emitter's no-ladder branch | ✅ Resolved |
| 6 | Scope | Whether the guarded member-root sibling leak (`flag && sink(mkv().a)`) is fixed here or filed, and how the stale pins are handled | File it as a new backlog defect; correct the DEF-60 row's remedy text; update the implementation pin that asserts the buggy `__cl_tmp0[0] = NULL` for `__cl_tmp0[i]` / include the fix | File the member-root leak as a new backlog defect; correct the DEF-60 row; update the stale implementation pin | ✅ Resolved |
| 7 | Technical (verification) | Verify command per task and at close-out | `make check` per task and phase; `make verify` plus host AddressSanitizer probes at close-out (new gitignored runner under `build/def-backlog-probes/`) / `make verify` every phase | Confirmed: `make check` per task and phase; `make verify` plus host ASan probes at close-out | ✅ Resolved |
| 8 | Naming | Plan folder and test-suite names | `plans/def-backlog-completion/` with `compiler/tests/def_backlog_completion_{spec,impl}_test.rs` / other names | Confirmed: `def-backlog-completion` and `def_backlog_completion_{spec,impl}_test.rs` | ✅ Resolved |
| 9 | Scope | Compatibility baseline for the seven fixes | No `spec/` change, no new diagnostic code (reuse `E10034`), no dependency or layout change; the DEF-56/61 rejection is the only user-visible acceptance change and matches the concrete spellings; own/codegen stage boundaries preserved | Confirmed as stated | ✅ Resolved |
| 10 | Behavioral | Phase ordering, coverage, and success criteria | Phase 1 generic-lend ownership (DEF-56, DEF-59, DEF-61, DEF-60); Phase 2 guard/loop release (DEF-62); Phase 3 closure C spelling (DEF-57, DEF-58); each with spec tests → implementation → impl tests; success = five ASan run probes clean, two rejection probes correct, suites green, pinned re-emission byte-identical | Confirmed as stated | ✅ Resolved |
| 11 | Process | Cycle and commit mode | Commit the plan set after preflight; execute with `--auto-commit` (commit and push each verified task/phase), as the cycle prompt directs | Commit the plan set after preflight; `--auto-commit` through close-out (user instruction, 2026-10-03 08:0x cycle prompt) | ✅ Resolved |
| 12 | Edge cases | Coverage obligations for the fixes | Runtime non-zero element indices (guarded and unguarded), guarded member-root take, store and return element reads, do-while early return with multiple materialized condition temps, nested flag-first loops, byte-identical re-emission | Confirmed as stated | ✅ Resolved |
| 13 | Technical (runtime) | The spec suite's ST-24 (generic `borrow T?` out-param exemption) is unwritable: a generic `T?` slot is `E10011`, and a concrete non-null `borrow T?` slot double-frees through an unrelated by-value lowering gap | Narrow ST-24 to a concrete out-param with a `null` start and file the non-null double-free as a new backlog defect / expand scope to fix the lowering / drop the exemption case | Accepted narrowing: the concrete null-slot case stays; the plan's ST-24 and exemption wording are corrected; the non-null double-free is filed as DEF-65 (runtime resolution, 2026-10-03) | ✅ Resolved |

### Resolution Notes

**AR-1:** The runtime-index move-out loses the index because `value_place` stores `Projection::Element(None)`
for a runtime index and codegen renders `Element(None)` as `[0]`. The fix carries the taken argument's
index through to the move-out, so the zeroed slot is the slot the callee moved. The guarded
member-root leak becomes a new backlog row (file-only), consistent with the repository's pattern of
filing verified residuals rather than expanding a defect's charter.

**AR-2:** The concrete spellings report `E10034` through `move_target`; the generic spellings take the
`maybe_owns` branch (`mark_moved_if_owning`) because the type is unknown pre-substitution. The fix
records the ownership duty at the single central site and audits it after substitution, mirroring the
existing duplicability-duty machinery. For duplicable instantiations nothing changes; for owning
instantiations the diagnostic matches the concrete behavior. No diagnostic code is added. Preflight
fixed the audit's details (PF-014, PF-021, PF-026, PF-027): the predicate is the binding kind plus
lend mode (the `borrow T?` out-param slot exempt), there is no param-mention early skip, element
projections count, one duty reports once, and a moved lend read again keeps its duplicability duty
(an owning instantiation reports `E10034` and `E10020`; PF-024).

**AR-3:** `ClosureTy.instance` already models "the enclosing instance this closure was specialized
for"; it is only set during emission. Stamping it when the instance argument is recorded makes the
callee spelling agree with the call site by construction. The key must enter `type_key`, or two
enclosing specializations collapse to one C symbol. Relocating the key renderer to the stage-neutral
`types` module keeps the `own` stage from importing `codegen`. Preflight corrected the relocation
scope (the slice helpers move too; PF-019), the stamping order (lifted specializations are recorded
before the body walk; PF-008), and the key roles (`type_key` names symbols, `dedupe_arg` feeds the
dedupe/diagnostic key; PF-030); the zero-argument separator rule is part of this decision (PF-013).

**AR-4:** The body's `Return` terminators are built before the condition lowers, so they copy a
release set that does not yet include the condition's materialized temp. Appending the condition's
pending releases to those terminators after the condition lowers (checker-local, with a small
builder mutator) keeps the IR authoritative and reuses the whole return pipeline. Emitter-side
tracking is the fallback only if the design probe finds the temp declaration cannot be referenced
from the body's return sites. Preflight fixed the mechanics (PF-020): checkpoint the pending-release
count, collect the return blocks reachable from the body block through the existing successor walk,
prepend newest-first with a dedupe check, record the release events, and add the mutator in
`compiler/src/ir.rs`. Function-scope temp hoisting makes the checker-local path feasible, so the
bounded emitter fallback stays unused.

**AR-5:** The store path reuses the existing `PreparedStore::Move` copy-and-zero arm and carries the
index in the `ElementTemp` place; the return path renders the lowered value through
`emit_arm_source_moves` in the no-ladder branch. Neither collapses a runtime index to `[0]`
(PF-028); the aggregate-literal consume form that still does is the filed DEF-64 limitation
(PF-009).

**AR-6:** The stale implementation pin asserted the buggy `__cl_tmp0[0] = NULL` next to
`__cl_tmp0[i]`; it is an implementation test, so updating it to the corrected shape is allowed.
Preflight found two further pinned programs — one in each suite — that instantiate a lend-returning
generic with `String`; under the explicit user approval recorded 2026-10-03 (PF-002) their `apply<T>`
programs migrate to `take x: T`, preserving their original lambda-specialization and determinism
assertions. The aggregate-literal consume found while re-verifying is filed as DEF-64 (PF-009); the
guarded member-root leak is DEF-63.

**AR-7:** `make check` runs `./spec/check.sh` plus the workspace tests in the pinned container;
`make verify` adds every hosted-amiga cross-toolchain leg. The close-out probe runner emits, builds
with AddressSanitizer, and runs the reproductions, mirroring the previous cycle's runner. Preflight
split it into five run probes and two rejection probes (PF-011) and added a `timeout` plus
`handle_segv=0` for the two shapes that hang under the default handler (PF-025).

**AR-9:** The frozen specification stays the oracle; the rejection reuses `E10034` and its existing
message. The DEF-56/61 rejection is intentional: programs that relied on a generic lend escaping as
owned were never safe and never matched the concrete rule.

**AR-11:** The user's cycle prompt explicitly directs `make-plan → preflight → exec-plan (--auto-commit
through close-out)`; the plan set is committed after preflight, as in the previous cycle.

**AR-12 (coverage):** Preflight extended the case set (PF-004, PF-010, PF-012, PF-018, PF-021,
PF-023, PF-024, PF-026, PF-027): the constant-index store/return split (ST-20), the stored-lend
form (ST-21), the concrete-typed lend (ST-22), the element projection (ST-23), the `borrow T?`
exemption (ST-24), multi-level indices (ST-25), the dual report (ST-26), and the closure-only
instance dedupe (ST-27), with pinned exit codes for the run cases and the documented red-phase
host-sanitizer procedure for the memory cases.
