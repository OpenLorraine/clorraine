# Preflight Report: conditional-projection-residue

> **Audit target**: `plans/conditional-projection-residue/` (implementation plan, flat layout)
> **Artifact tree hash**: `3ab42a9c1874046d419183d4597c2145b3478d91` (at scan start)
> **Head at scan start**: `2c4677476c0a32a6ff77eadf13685667663d91e5` (2026-10-13 23:57)
> **Scan model**: 6 parallel dimension clusters (13 dimensions + the compiler-and-language lens)
> **Scope mode**: strict · **Modification set (authorized)**: the plan docs + this report
> **Same-session bias**: ⚠️ SAME-SESSION REVIEW — the plan was authored in this session; the scan
> was fanned out to fresh-context auditors, and the two critical findings were re-verified
> firsthand against the pinned test sources before this report was written.
> **Verdict (pre-fix)**: ❌ BLOCKED — 2 critical, 12 major findings unresolved
> **Verdict (post-fix)**: ✅ PASSED WITH NOTES — all critical/major resolved (2 rows parked for
> rulings by the reserved-decision rule), all minors/observations fixed or recorded
>
> **Superseded (2026-10-15)**: the two rows parked by PF-001/PF-002 (`DEF-279`, `DEF-268`) were
> re-decided with fresh evidence after this scan — `DEF-268` is incidentally fixed by the batch-H
> per-arm conversion and `DEF-279` is implemented as the one-report rule (see the register's
> AR-25/AR-26). The scan-time texts below are the audit record; the six older parked rows are
> unchanged (AR-2).

## Cluster returns

| Cluster | Dimensions | Source |
|---|---|---|
| ① Document soundness | Ambiguities · Contradictions · Consistency | merged |
| ② Grounding | Implicit assumptions · Codebase alignment (10 sub-checks) | merged |
| ③ Delivery | Completeness · Dependencies · Ordering | merged |
| ④ Risk | Security · Edge cases · Feasibility | merged |
| ⑤ Fit | Testability · Scope creep | merged |
| ⑥ Compiler-and-language lens | Rule conformance · Diagnostics · ABI | merged |

## Findings and resolutions

Severity: 🔴 blocks execution · 🟠 significant rework/incorrectness · 🟡 friction · 🔵 observation.
Every resolution below is applied to the plan docs (auto-design delegated technical fixes; the
two parked rows are ruling-gated per the register's reserved-decision rule).

| # | Sev | Finding (one line) | Evidence | Resolution applied |
|---|-----|--------------------|----------|--------------------|
| PF-001 | 🔴 | `DEF-279`'s dedupe (suppress `E10030` when `E10035` reports) breaks the immutable oracle `callable_release_integrity_spec_test::should_reject_a_mixed_conditional_view_return` — that shape **has** a local arm and the pin asserts exactly one `E10030` | `compiler/tests/callable_release_integrity_spec_test.rs:1772-1802` (`let a: Box = …; return (c ? a : mkBox()).v;`; `assert_rejected_once(… "E10030" …)`); probe: both reports at one span | **Park `DEF-279`** (ruling-gated; re-pinning an immutable spec test needs a user ruling). Remove its design §5, task, and ST-11 from P2; add to AR-2's parked list with the oracle citation |
| PF-002 | 🔴 | `DEF-268`'s `E10036` rejects `return (c ? b.v : a)` — a shape the batch-H immutable spec suite pins **accepted, ASan-clean, exit 0** | `compiler/tests/conditional_projection_integrity_spec_test.rs:2313-2333`; impl pin `conditional_projection_integrity_impl_test.rs:936-970` | **Park `DEF-268`** (ruling-gated). Record the predicate ambiguity (the named `return h.v` pin rejects `E10036` while the conditional twin is accepted — the suite itself pins both) in AR-2; remove its design §4, task, and ST-10 from P2 |
| PF-003 | 🟠 | `DEF-261`'s premise is stale: the local spelling reports `E10031` since `96fe14b`; the live accepted UAF is the parameter/parameter (and two-arm) spelling, and the call path short-circuits the every-arm owner walk | `compiler/src/own/place.rs:636-652` (local-preserving rule); probes: local → `E10031`; `bytes((c ? b : d).v)` with `borrow` params → accepted + heap-use-after-free; `bytes(c ? b.v : d.v)` reports only the first arm | Rewrite P2 §1 and ST-7: the red target is the param/param and two-arm spellings; the fix recurses through the view-returning call's argument and collects **every** arm's owners (the direct walk at `place.rs:826-866` already does); the local spelling becomes a control |
| PF-004 | 🟠 | P3 §1's global/field wording accepts "a parameter-backed value" as an owner that outlives a global — unsound; the head correctly rejects it (`E10033`) today | `03-03` §1; `spec/16-borrow-rules.md:194` (BR-8); `spec/03-ownership.md:238`; head probe: parameter-backed global/field stores rejected | Rewrite P3 §1 per destination kind: **global/field → module-level static owners only** (parameter-backed stores stay rejected); **out-slot → static and parameter-backed** (the caller frame outlives the callee). Add parameter-backed global/field rejection controls |
| PF-005 | 🟠 | The out-slot store reports `E10033`, whose registry trigger is "Loan stored in a field or global"; an out-slot is a parameter, and the view outliving its owner's scope is `E10030` | `spec/14-diagnostics.md:58,61`; `spec/16-borrow-rules.md:194`; probes: fresh/local-backed out-slot stores accepted + UAF | P3 reports **`E10030`** on the out-slot path (both `borrow slot: str` and `borrow slot: str?` spellings), one report per site; `E10033` stays the field/global code |
| PF-006 | 🟠 | P3 §3's local element-store acceptance is under-specified: no loan is recorded, so loop re-execution and nested-scope backings become accepted UAFs; handle→view widening spellings are ungated by `is_loan_value` | `compiler/src/own/expr.rs:858-895` (gate, no owner record); probes: `while (i<3) { xs[1] = mk().v; }` accepted + heap-use-after-free; `xs[1] = b.v` into a borrow param and `names = b.v` to a global accepted + UAF | Rewrite P3 §3: judge the store against the **destination-expected type** (the widened view counts), record the stored view's owner loan exactly as the binding path, and scope acceptance to backings whose scope covers the array's (a loop iteration or a nested block reports `E10030`). Add the loop, nested-scope, and widening ST twins |
| PF-007 | 🟠 | P4's discard design leaves the move model unstated: the head rejects a later use of the place field (`E10020`) while ST-17 requires `t`/`u` readable | probes: `ok ? Pair { a: stringOf("x"), b: t } : …` accepted, fresh field leaks, later `t` use → `E10020`×2; the direct `ok ? t : u;` leaves both readable | State the alias model in P4 §2: a discarded conditional's place-valued literal fields are **reads** (matching the direct `ok ? t : u;` behavior, pinned as a control); the fresh fields materialize for release. Align ST-17 |
| PF-008 | 🟠 | P6's design lacks the right-hand-side pin: `i = i++` and `i += i++` stay sequence-point violations with only target-part pins | `03-06` §1; AR-9; emitted C `i = (i)++;` at head | Add "an update-containing assignment right-hand side pins at its source position" to P6 §1 and AR-9; ST-23 pins the three spellings' emission |
| PF-009 | 🟠 | `DEF-259`'s reproduction is green at the plan baseline (incidentally fixed by batch H) — ST-3 cannot flip, so AC-1 and the P1 red phase cannot be met for it | cluster probes (`armliteral` and the guarded variant clean, both arms); `02-current-state.md`'s DEF-259 entry describes pre-fix behavior | Reclassify `DEF-259` as **incidentally closed at the planning baseline** with the probe evidence (ST-3 stays as a green control); update the index, requirements, and P1 texts; the close-out closes the roadmap row with the incidental note |
| PF-010 | 🟠 | ST-14 contradicts the P3 design: `g.names = [mk().v]` is pinned "Accepted; strict build; run exit 0" while the same shape is an escaping fresh-element store | `07` ST-14 vs `03-03` §1; probes: clc accepts, strict build fails (`g.names = { … }`, also for the static-element spelling) | Rewrite ST-14: the fresh-element store is a rejection (out-slot/global codes per PF-005); the accepted emission case uses a static- or parameter-backed element; split the capture half into its own case |
| PF-011 | 🟠 | ST concretization and twins: eight ST rows state no determinate expectation (no code/count for rejections, no exit value for runs); missing twins — loop/nested-scope element stores, widening stores, the view-returning callee, the move-only fixed-array root, the `?:`/`\|\|` literal positions, the two-arm owner walk; ST-24/ST-26 are green pre-fix at run time on the pinned container (its compilers order initializer lists left-to-right; the host GCC 15 outside the container also accepts the sequence shapes, so only the container counts) | `07` ST rows; cluster ④/⑤/⑥ probes; iteration-2 re-probe on the pinned container (V-2) | Revise `07`: rejections gain code + count + site; runs gain their encoded exit value; add the twins; move the literal-order reds to impl emission pins and make ST-26's `g` order-observing; keep ST-23/ST-25's strict-build reds (the pinned container rejects them) and record the container result |
| PF-012 | 🟠 | P6's task names only `own/lower.rs` while every trigger site is elsewhere; the loop-condition pin must be a deferred `Materialize` fill (a plain statement pin is what `in_loop_condition` excludes) | `compiler/src/own/call.rs:232-247`, `own/expr.rs:1388-1392,1479-1502`, `own/loops.rs:48-53`, `control.rs:429-440`; `lower.rs:1234-1253` (the pin refusal) | Amend P6 §3 and task 6.2.1: the file list adds `own/call.rs`, `own/expr.rs`, `own/loops.rs`, and the loop lowering; the loop-condition pin is stated as a per-iteration `Materialize` fill |
| PF-013 | 🟠 | P5's site map and mechanism: the `for`-in iterable lowers in `own/loops.rs` (missing from the task); the fixed-array conditional arm is an emitter fallback today and the conditional-root projection needs a per-arm **address** form, not a value copy (illegal for a move-only root) | `own/loops.rs:479,561,650`; `codegen/guards.rs:125-148` ("no expression form; the fallback keeps the renderer total"); `codegen/function.rs:563-591,1674-1755`; probe: move-only root → `lvalue required` | Amend P5 §2 and task 5.2.1: add `own/loops.rs`, `codegen/guards.rs`, and `codegen/function.rs`; specify the per-arm address/statement forms (`(c ? &b1.xs : &b2.xs)` and the per-arm destination store); add the move-only-root ST |
| PF-014 | 🟡 | P2's task file list omits `compiler/src/own/lower.rs` (its own integration point for the view-element lowering) | `03-02` Integration Points vs `99` task 2.2.1 | Add `own/lower.rs` to task 2.2.1 |
| PF-015 | 🟡 | `DEF-262`'s array-literal element variant ownership is ambiguous: `03-01` §5 claims the same walk covers it while `03-04` §1 says it still reproduces | `03-01` §5; `03-04` §1; ST-16 | Scope `03-01` §5 to the literal-field walk; add the array-literal element fix sentence to `03-04` §1 |
| PF-016 | 🟡 | Review-lens mismatch: P3 implements in `codegen/*` but no emission-integrity lens is dispatched, and the index/AR-15 omit the semantics lens P3's acceptance changes need | `99` Phase 3 header; `00-index.md` Specialist Agents; AR-15; `AGENTS.md` dispatch matrix | Align: P3 = correctness + semantics + memory-safety + emission-integrity (in `99`, the index, and AR-15) |
| PF-017 | 🟡 | The "byte-identical emission" controls name no harness that can observe base-vs-head identity | `07` ST-6/ST-22 and the coverage goals | Restate the controls as impl-suite re-emission determinism pins; base-vs-head stays the emission-differential integration row |
| PF-018 | 🟡 | The E2E row pairs `clorraine run` with `RDC_ASAN=1`, but the driver has no sanitizer flag; `RDC_ASAN` only switches the Rust suite helpers | `07` End-to-End Tests; `compiler/src/clorraine/cli.rs:71-77` | Split the row: the driver run asserts behavior/exit; the suite helper under `RDC_ASAN=1` asserts memory; name the ASan leg in each ownership phase's Verify |
| PF-019 | 🟡 | "One report per site" is a repo convention, not frozen; the plan's blanket conformance claim needed a citation | `spec/14-diagnostics.md:18-20` (dependent-error suppression) vs AR-8 | With `DEF-279` parked, the claim is removed; the parked entry records the convention's citation |
| PF-020 | 🟡 | Line-number drift in `02`/`03-0X` after phase 1 | cluster ② mapping (`lower.rs` 1013→1019, 1063→1069, 1103→1109, 1114→1120, 1228→1234, 1273→1279, 2264→2331, 2352→2419, 2711→2778, 2953→3077, 3099→3223, 3378→3506; `call_check.rs` 676→680; `statements.rs` 1141→1163; `expr.rs` 463/780/835→564/885/940) | Refresh the cited line numbers in `02-current-state.md` and the `03-0X` docs |
| PF-021 | 🔵 | The index attributes the execution mandate to AR-1 (wrong entry) | `00-index.md` | Cite the standing mandate, not AR-1 |
| PF-022 | 🔵 | The dependency graph's arrows are undefined | `99` Dependencies | Label the arrows and keep the linear phase order as the executable sequence |
| PF-023 | 🔵 | The Won't Have list misses `DEF-211`, `DEF-214`, `DEF-233` (other batches' ruling-gated rows) | `01-requirements.md` | Add the three names for completeness |
| PF-024 | 🔵 | "Frozen single-view parity" and "the hidden out-parameter ABI" are pinned by the batch-H suites and the compiler contract, not by `spec/` text | `spec/16-borrow-rules.md:150,193`; `spec/17-codegen.md:59-84`; batch-H pins | Cite the pins/contract as the operative oracle (behavior unchanged) |

## Reserved decisions for the user (parked rows)

| Row | Needed ruling | Why parked |
|-----|---------------|------------|
| `DEF-268` | Whether the disagreeing conditional return reports `E10036` (and the batch-H spec pin `should_build_a_view_typed_conditional_over_an_owned_member` is re-pinned), or stays accepted | The plan's rejection contradicts an immutable spec-suite pin; amending it is a specification decision |
| `DEF-279` | Whether one escape report (the local `E10035`) replaces the generated-owner `E10030` at the same site (re-pinning `should_reject_a_mixed_conditional_view_return`), or both reports stay | Same: the pin asserts exactly one `E10030` for a shape that has a local arm |

The `DEF-268` and `DEF-279` rows were re-decided with fresh evidence on 2026-10-15 (AR-25/AR-26)
and are no longer parked: `DEF-268` is incidentally fixed by the batch-H per-arm conversion and
`DEF-279` is implemented as the one-report rule.

Still parked from the register (unchanged): `DEF-167`, `DEF-169`, `DEF-171`, `DEF-190`,
`DEF-191`, `DEF-263`.

## Iteration-2 verification

Scoped re-review of the fixed plan against PF-001 … PF-024 returned **ITERATION 2: FAILED — 2
items** (V-1, V-2) plus four minor inconsistencies, all corrected in a targeted fix round (the
review budget's final confirmation):

| Item | Finding | Correction |
|------|---------|------------|
| V-1 | The phase-1 deliverable listed ST-6 (the controls) as red | The deliverable now names ST-1/2/4/5 red and ST-3/ST-6 green controls |
| V-2 | AR-24 claimed the host compilers accept the sequence shapes under the suite's strict flags | Corrected throughout (the register row + note, `02`, `03-06`, `07`, `99`): the pinned container (GCC 12.2 / clang 14) rejects them; only a host GCC 15 outside the container accepts them, so the red phase records the container result |
| V-3 | ST-28's sketch did not type-check | Re-sketched as a view local through `peek(x: str): str` |
| V-4 | ST-30 did not name the backing kind | Names the local backing and adds the parameter-backed out-slot control |
| V-5 | The index's `Implements:` listed the incidental/parked rows; the register timestamp was future-stamped | `Implements` now lists the 20 implemented rows plus the closure/park notes; the timestamp is the write time |
| V-6 | `03-02` §1's "collects only the first arm" was imprecise | Reworded: the local-backed arm is the only one the current provenance rule preserves |

**Final verdict: ✅ PASSED WITH NOTES** — all critical/major findings resolved (two rows parked
for rulings by the reserved-decision rule at scan time; both re-decided with fresh evidence on
2026-10-15 — AR-25/AR-26), all minors fixed.

## Notes

- **New accepted-UAF spellings found while probing** (the handle→view widening stores and the
  loop/nested-scope element stores, PF-006): they are treated as variants of the in-scope
  `DEF-265`/`DEF-276`/`DEF-280` class and folded into the P3 design and tests. If the P3 review
  judges any of them a separate defect, the executor files a roadmap row with the base==head
  evidence (the batch convention).
- **Roadmap state**: the `DEF-268` and `DEF-279` rows were returned to `Backlog` (⬜) with their
  ruling notes when this report was applied and were re-scoped back to `Executing` (🔄) on
  2026-10-15 with the decision notes (AR-25/AR-26); the close-out task verifies the six older
  parked rows stay open and closes `DEF-259` and `DEF-268` as incidentally fixed with the probe
  evidence.
- The verification pass for this report is a scoped iteration-2 re-review of the fixed plan
  against the findings above.
