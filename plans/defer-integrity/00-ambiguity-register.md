# Ambiguity Register: defer-integrity

> **Status**: ✅ GATE PASSED — all 15 items resolved (AR-1–AR-14 at plan time; AR-15 added and resolved during Phase 1 execution)
> **Last Updated**: 2026-10-09 15:39

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| AR-1 | Scope | Which defect batch this plan closes | A: defer semantics (DEF-84, DEF-85, DEF-86, DEF-87, DEF-88 — five majors, one subsystem, all filed from the same review) · B: borrow/generic keys and rules (DEF-77, DEF-78, DEF-79, DEF-81, DEF-82, DEF-83) · C: lambdas, diagnostics, emission (DEF-76, DEF-80, DEF-90, DEF-92, DEF-93, DEF-94) · D: expression temporaries (DEF-95) | **A** — user instruction ("the next logical defect batch") plus evidence: the five defects are all 🟠 major, all filed from the same Phase 4 review, all live in one subsystem (deferred-block lowering and cleanup-ladder emission), and the previous plan's AR-1 names "defer semantics" as the first of the three remaining named batches. B and C mix majors and minors across subsystems; D is a single expression-temporaries item excluded by the previous plan's AR-5. | ✅ Resolved |
| AR-2 | Scope | Frozen-spec stance: does any fix need a `spec/` change? | A: implementation-only — the existing rules already define the required behavior (`spec/05-statements.md` §13 and §15, `spec/17-codegen.md` §4 CL-1–CL-7, `spec/decisions.md` F5/P2); the five defects are implementation deviations (undefined C label, self-loop, skipped cleanup, wrong nesting, drop-before-read) · B: amend a chapter where the rule is unclear | **A** — implement the written rules; the specification stays untouched. If implementation proves a rule is genuinely undefined, STOP and escalate (a spec change needs an explicit user ruling). | ✅ Resolved |
| AR-3 | Scope | Modification set for this plan | A: `compiler/src/own/*`, `compiler/src/codegen/*`, `compiler/tests/*` (new spec/impl suites; existing pins only with a recorded justification), `plans/defer-integrity/*` · B: also `spec/`, the driver, or other batches | **A** — compiler stages touched by the five defects plus the plan folder; no spec change (AR-2), no other batch (AR-1), no new dependency. | ✅ Resolved |
| AR-4 | Technical (design) | Core design: one shared deferred-copy emission context versus per-defect patches versus re-modeling at lowering | A: one shared context threaded through the emitter (per-copy printed-block set and label namespace; ambient `FlowContext`; bounded closure; running-cleanup frames with continuations), implemented as parameter-threaded copy/frame state · B: five minimal per-defect patches · C: re-lower the deferred body at each edge instead of copying it in the emitter | **A** — the independent challenger verified B is not a correct fix (D84 still pulls the loop tail, D85's skip-own-rung leaks the remaining drops, D86 per-site copies without a namespace collide on `__cl_bN` labels, D87's suppression skips required cleanup) and C violates the byte-identity obligation and pushes the hardest semantics into the checker. A reuses the existing `scope_actions`/rung model and satisfies CL-1–CL-7; the accepted instrumentation limit: the rung-hosted `break`/`continue` sibling is handled by a small guard, not a full root-cause claim. | ✅ Resolved (auto-design; see note) |
| AR-5 | Technical (design) | Deferred `return` continuation (DEF-85): how the return inside a defer completes its scope's remaining cleanup and continues the ladder | A: run the remaining actions of the running scope, then the outer rungs — implemented as a continuation label emitted after the returning copy when that copy contains a return · B: skip the owning scope's rung (`goto` the next rung) | **A** — the filed shape `[Defer, Drop(a)]` (`d85c`) proves B leaks the remaining drop; A matches the spec's cleanup order (CL-3) and CL-6's "return to the outer label". | ✅ Resolved (auto-design; see note) |
| AR-6 | Technical (behavior) | `continue`/`break` inside a deferred block (DEF-84): the emitted edge target and the copy's own region | A: thread the ambient `FlowContext` into the copy so out-edges classify as `Continue`/`Break`; bound the copy's closure to the deferred block's own block range so outer blocks are never pulled in; force inline cleanup for a scope whose defer bodies have `break`/`continue` out-edges so a rung-hosted copy never needs a `break`/`continue` it cannot spell · B: diagnose/reject the shapes · C: leave to per-edge re-lowering (AR-4 option C) | **A** — the filed do-while shape must build strict and run; B contradicts §13 (the checker accepts these programs); C is AR-4's rejected option. The rung-hosted sibling is the same classification defect with a different trigger; the inline-only guard is the smallest correct handling (verification probe; file as a follow-up if the guard proves insufficient). | ✅ Resolved (auto-design; see note) |
| AR-7 | Technical (design) | Fall-through emission and rung nesting (DEF-86, DEF-87) | A: emit one copy per cleanup site (per-copy printed-block set plus a per-copy label namespace) and re-check the deferred block's effects on every exit edge (the checker's defer entry survives later edges) · B: keep the single one-shot copy and re-anchor it to the first edge | **B is incorrect** — D86's fall-through and D87's outer site must each run the defer; A is the fix. The early-return sibling (`{ defer {...} if (c) { return 1; } }`) is the same missing-copy mechanism and is covered by the same fix. | ✅ Resolved (auto-design; see note) |
| AR-8 | Technical (design) | Defer-local cleanup versus the return-value read (DEF-88) | A: generic ordering fix — for a `Return`-terminated block, print the value store before the trailing inline cleanup (covers the defer-local shape and the same-root-cause loop-local shape) · B: defer-scoped fix (give the defer block's own scope a rung, or reorder inside deferred blocks only) | **A** — one root cause, one fix: the emitter prints trailing inline cleanup before the terminator's value store (verified: a return in a loop body reading a loop-local `String` segfaults with no defer present). B is more special-case code and leaves the memory-safety defect in place. Store-before-release pins stay green (`def_backlog_closeout_impl_test.rs`), and the void-return and rung paths are unchanged. | ✅ Resolved (auto-design; see note) |
| AR-9 | Technical (testing) | Specification-test set, red/green evidence, and the verification strategy | A: one focused ST case per defect plus the same-root-cause sibling cases and at least one combination case per phase, with the strict-build acceptance, exit values, and host-ASan oracles · B: fewer, broader cases | **A** — one ST case per defect (the filed repro shape where one exists), plus the early-return sibling (AR-7), the loop-local root cause (AR-8), the while/break variants and the rung guard probe (AR-6), and the remaining-actions case (AR-5); detail in `07-testing-strategy.md`. | ✅ Resolved |
| AR-10 | Technical (runtime) | Existing tests that pin the current defer emission: migrate or design around | A: design the fixes so every preidentified pin stays green, and treat any red pin as a surface-during-authoring event needing a recorded decision before migration · B: pre-authorize migrating the pin list | **A** — the mechanism report identified the pins (defer-runs-on-return, reverse-registration order, one-label-per-owning-scope for defer-free programs, store-before-release, no `__cl_b` block-label pins). No spec-test expectation change is planned; a spec test is never weakened to match the implementation — if one turns red, it is reported and decided, not edited silently. | ✅ Resolved |
| AR-11 | Technical | Phase decomposition and task ordering | A: four phases sequenced by dependency — (1) per-site deferred copies with namespaces (DEF-86, DEF-87 + the early-return sibling), (2) bounded closure and ambient context (DEF-84 + the rung guard), (3) deferred return (DEF-85 continuation + DEF-88 return-before-cleanup ordering + the loop-local root cause), (4) plan-wide hardening, final review, and close-out · B: the challenger's original risk order (ordering first, continuation last) · C: one phase | **A** — the specification-first per-phase green constraint forces the re-sequencing: the filed DEF-88 defer-local shape cannot go green until DEF-85's continuation lands (the self-loop `goto __cl_c1` survives the ordering fix and turns the SEGV into a hang), so D85 and D88 ship together; the first two phases build the copy/context machinery the return phase reuses. The challenger's core verdict (AR-4 Option A and its dependency analysis) is unchanged and its risk order is rejected only on per-phase green viability. | ✅ Resolved (auto-design; re-sequenced during authoring — see note) |
| AR-12 | Naming | Plan slug and test suite names | **`defer-integrity`**; `compiler/tests/defer_integrity_spec_test.rs` and `compiler/tests/defer_integrity_impl_test.rs` | **`defer-integrity`**, matching the sibling slugs (`element-loop-integrity`, `slice-temporary-integrity`) and the batch's subject. | ✅ Resolved |
| AR-13 | Technical (testing) | Verify commands and the container sanitizer policy | A: `make check` per phase, `make verify` for the final phase; host AddressSanitizer probes for memory shapes; the in-container full-suite `RDC_ASAN` run stays non-gating because of its documented 30 s startup flake · B: gate on the in-container `RDC_ASAN` suite | **A** — `make check` per phase and `make verify` at the end (both from `AGENTS.md`); host ASan is the memory oracle; a flaked in-container run is retried alone and never treated as a failure. | ✅ Resolved |
| AR-14 | Technical (complexity escalation) | Does the fix design add a material support surface (new layer, dependency, harness, framework, infrastructure, or cross-cutting refactor)? | A: no — the fixes are ordinary feature code inside the existing ownership/emission machinery (copy context, closure bound, frame continuation, emit ordering); no new dependency, harness, or external surface · B: a larger mechanism (new emission pass, generalized cleanup framework, or per-edge IR re-lowering) | **A** — the challenger searched for escalations and found none smaller than the accepted design; option C (the larger mechanism) was rejected on blast radius and byte-identity. No material support surface is added, so no complexity stop packet is open. | ✅ Resolved |
| AR-15 | Technical (runtime) | The ST-3 program `return count * 10 + f(true) + f(false);` reads the module counter before the calls under the frozen left-to-right evaluation order (`spec/04-expressions.md` §3), so the recorded expectation 21 was not derivable from the spec for that program (left-to-right yields 1); the emitted code was already correct | A: bind the call sum first (`let a: i32 = f(true) + f(false); return count * 10 + a;`) so the counter is observable and 21 follows from left-to-right evaluation plus the once-per-edge defer runs · B: keep the program and change the expectation to 1 | **A** — the expectation must derive from the spec; binding the sum keeps the intended behavior oracle (two defer runs, observable counter) and avoids pinning an evaluation-order artifact | ✅ Resolved (runtime, auto-design) |

## Resolution Notes

**AR-1:** The user's instruction — "make plan for the next logical defect batch" — names the target
class (the next filed batch) and delegates the identification. Evidence for the defer batch: the
five defects are the only fully cohesive group of the three named follow-up batches (all 🟠 major,
filed together in the def-backlog-closeout Phase 4 review as OBS-1/OBS-2/OBS-3/OBS-A/OBS-C/OBS-D/MSA-401/MSA-402/MSA-403/EI-401/EI-402), and 5 of the 6 remaining major defects in the backlog are
in this group. DEF-77–79/81–83 (batch B) and DEF-76/80/90/92–94 (batch C) stay owned by later
plans; DEF-95 stays in the expression-temporaries workstream.

**AR-2:** Rule anchors that already define the required behavior: `spec/05-statements.md` §13
("`defer { ... }` runs when the enclosing scope exits… code motion… joins the cleanup ladder, which
runs drops and defers in reverse registration order. They run on `return`, `break`, and
`continue`") and §15 ("Every exit edge from a scope — the normal end, `return`, `break`, and
`continue` — runs the cleanup ladder for that scope"); `spec/17-codegen.md` §4, rules CL-1–CL-7
(the normative ladder contract, including "A `defer` block is code motion; it is copied into the
ladder at its scope's label" and "Early exits jump to the label of the scope they leave"); the
strict-C99 unit contract of §1 and the P2/F5 decisions. The five defects are implementation
deviations from these rules, so the plan is implementation-only.

**AR-4 (delegated record):**

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanism inside the confirmed batch and modification set (AR-1, AR-3);
  compiler-internal emitter architecture; no product-behavior, scope, or acceptance-criteria change
Objective: fix all five defer defects with a single-owner, strict-C99, memory-safe design that keeps
  every unaffected program's emission byte-identical and reuses the existing scope-action/rung model
Decision: Option A — one shared deferred-copy emission context, implemented as the smallest
  parameter-threaded copy/frame state: (i) per-copy printed-block set + per-copy label namespace;
  (ii) ambient FlowContext; (iii) closure bounded to the deferred block's own block range;
  (iv) a running-cleanup frame with a continuation (remaining actions, then outer rungs)
Evidence: the mechanism report (verified file:line and reproduced probes: d84b strict-C failure,
  d84 loop swallowing, d86 exit 0, d87d exit 0, d85 self-loop, d85c skipped drop, d88 SEGV,
  d88c loop-local SEGV without defer); emitter call sites are singular (`control.rs:177`
  `emit_block_body`; `cleanup.rs:195-244, 615-636` the only cleanup entry points), so the context
  threads through few functions; CL-1–CL-7 stay satisfied
Rejected alternatives: B (five minimal patches) — the independent challenger proved each patch
  incomplete or incorrect (loop tail pulled into a copy; duplicate __cl_bN labels; skip-own-rung
  leak; suppressed required cleanup), with mutually dependent context pieces; C (re-model at
  lowering) — changes emission for working programs, duplicates IR per edge, and moves the hardest
  semantics into the checker
Strongest counterargument: A is a central emitter change and still does not close the full family
  (the rung-hosted break/continue sibling remains); the counter is that the sibling is a
  pre-existing, unfiled, differently triggered shape handled by a small guard and reported, while
  B's smaller diff is smaller only because it is incorrect
Confidence: High — every mechanism claim is reproduced at HEAD; the interlock analysis is
  line-grounded; the accepted instrumentation limit is stated in the design docs
Hardening: independent design-challenger verdict — converged (it independently selected Option A
  and grounded the rejection of B/C in reproduced probes); challenger budget used: 1 of 2
Policy version: 1
Root invocation ID: ad-defer-integrity-20261009
Reopen triggers: the emitter cannot thread the copy context deterministically across its label
  preparation and emission passes; per-site copies change emitted bytes for programs without
  defer-with-early-exit shapes; or a phase review finds the frame continuation cannot emit the
  remaining actions exactly once per runtime path
```

**AR-5 (delegated record):** Authority: AI — delegated by --auto-design · Eligibility:
implementation mechanism (continuation semantics) inside AR-4's design · Objective: a deferred
return returns its value once, after running its scope's remaining cleanup · Decision: Option A —
remaining actions of the running scope, then the outer rungs, via a continuation label emitted
after the returning copy when that copy contains a `Return` (the copy's returns target it) ·
Evidence: `d85c` (`[Defer, Drop(a)]` — the filed skip-own-rung reading leaks `a`; the drop must run);
`d85d` (nested-block defer where the owning rung holds only the drop works today and must keep
working); CL-3 and CL-6 · Rejected: B (skip the owning rung) leaks the remaining drops ·
Strongest counterargument: the continuation label adds a generated label beyond "one label per
scope" (CL-2) — answered by CL-5's equivalent-per-registration allowance and by emitting the label
only when a returning copy needs it (no unused-label warnings) · Confidence: Medium-High — the
continuation is hand-traced and pinned by three ST cases, not yet prototyped (the execution plan
front-loads this phase's red/green evidence) · Hardening: the challenger evaluated the same shapes
and confirmed the remaining-actions semantics as the only correct reading · Policy version 1 ·
Root invocation ID: `ad-defer-integrity-20261009` · Reopen triggers: a runtime path where the
remaining actions run twice or zero times (host ASan/probe evidence), or a shape where the
continuation label changes a defer-free program's emission.

**AR-6 (delegated record):** Authority: AI — delegated by --auto-design · Eligibility:
implementation mechanism (edge classification and copy bounding) in AR-4's design · Objective: a
`continue`/`break` inside a defer reaches the loop's edge and the copy never swallows outer
blocks · Decision: Option A — thread the ambient `FlowContext` into the copy's walk; bound the
copy's closure to the deferred block's own block range (recorded at lowering; nested deferred
bodies included); force inline cleanup for a scope whose defer bodies contain break/continue
out-edges (the rung guard) · Evidence: `d84b` (do-while: undefined `__cl_b2`), `d84`
(while: loop swallowed), `d84c` (for: mangled), the challenger's rung-sibling reproduction ·
Rejected: B (diagnose/reject — contradicts §13 and the accepted-program contract); C (per-edge
re-lowering — AR-4 rejected) · Strongest counterargument: the rung guard does not close the full
sibling family — accepted and stated; the fallback is to file the residual shape with evidence
(the guard's verification probe is part of Phase 3) · Confidence: High for the filed shapes
(reproduced), Medium for the guard (mechanism-grounded, not prototyped) · Hardening: challenger
converged; it proposed the inline-only guard explicitly · Policy version 1 · Root invocation ID:
`ad-defer-integrity-20261009` · Reopen triggers: the guard changes emission for programs without
break/continue-in-defer shapes, or a rung-hosted copy still cannot spell its out-edge.

**AR-7 (delegated record):** Authority: AI — delegated by --auto-design · Eligibility:
implementation mechanism (copy-site policy and checker entry lifetime) in AR-4's design ·
Objective: the deferred block runs on every exit edge of its scope, exactly once per runtime pass ·
Decision: Option A — one copy per cleanup site with a per-copy printed-block set and label
namespace (first copy of each block keeps its `__cl_b{id}` spelling for byte-identity; later
copies take fresh numbers), and the checker's defer entry survives later edges so each edge
re-checks the block's effects · Evidence: `d86` (trace: the second site's `emit_closure` emits
nothing because the global `emitted` set already holds the blocks), `d87d` (the nested copy is
required on the guard path; the outer copy must still exist), the early-return sibling
(`{ defer {...} if (c) { return 1; } }` — the return edge jumps to an empty rung) · Rejected: B
(one-shot re-anchored) is incorrect per the evidence · Strongest counterargument: per-site copies
grow emitted code for defer-in-loop bodies (a defer inside a loop body is copied per cleanup site,
and loops have one inline site) — the growth is bounded by the site count and matches code motion
semantics; the determinism of the copy numbering across the preparation/emission passes is the
sharp edge (reopen trigger) · Confidence: High (one-shot cause traced and reproduced) · Hardening:
challenger converged and flagged the two-pass numbering requirement · Policy version 1 · Root
invocation ID: `ad-defer-integrity-20261009` · Reopen triggers: non-deterministic copy numbering
(a re-emission diff), or a per-site copy changing labels/bytes for single-site defers.

**AR-8 (delegated record):** Authority: AI — delegated by --auto-design · Eligibility:
implementation mechanism (emission ordering) in AR-4's design; the loop-local shape is the same
root cause, not new scope · Objective: a return value is read before any cleanup that frees the
storage it reads · Decision: Option A — for a `Return`-terminated block, emit the value store (and
the terminator's releases) before the trailing inline cleanup that `emit_block_body` currently
prints during the walk; keep void returns and rung-hosted cleanup unchanged · Evidence: `d88` IR
(`ScopeExit{2}, Drop{s}, ScopeExit{1}, Return{value: len(s)}` — the drop prints before the store);
`d88b` contrast (the rung path already orders store-before-drop); `d88c` (the same SEGV without
defer for a loop-local read) · Rejected: B (defer-scoped rung/reorder) — more special-case code,
leaves a memory-safety defect, and contradicts the single-root-cause evidence · Strongest
counterargument: the ordering change touches every value-returning block with inline cleanup, so
it changes emitted bytes for currently working programs — mitigated by keeping the change gated to
`Return`-terminated blocks with trailing inline cleanup and by the emission-integrity auditor
classifying every base-vs-head diff · Confidence: High (mechanism reproduced end to end) ·
Hardening: challenger converged; the store-before-release pins are preserved and named ·
Policy version 1 · Root invocation ID: `ad-defer-integrity-20261009` · Reopen triggers: a
store-before-release pin turning red, or an emission diff on a program without a value return
whose expression reads inline-cleaned storage.

**AR-9 / AR-13:** The harnesses the plan will use already exist: `compiler/tests/*_spec_test.rs`
(host C strict build + run, host AddressSanitizer via `run_memory`/`assert_no_asan`), the
strict-build acceptance used by the slice plan's ST-21, and the container legs (`make check`,
`make verify`). Every ST case's expectation is derived from the spec anchors in AR-2 or from the
probe evidence recorded in `02-current-state.md`; the red phase must show each case failing at the
phase baseline for the recorded reason.

**AR-10:** Pin evidence (mechanism report §7): `codegen_run_spec_test.rs:211,239,273,621,835`
(cleanup once per exit, reverse order), `def_backlog_closeout_spec_test.rs:1601` (one release per
exit, ASan), `stage_spec_test.rs:1446` (defer-free ladder: exactly two `__cl_c` labels),
`def_backlog_closeout_impl_test.rs:861,1287,1315` (store-before-release), `non_lexical_lifetimes_`
`spec_test.rs:1547,1570,1603` (acceptance only, no emitted-shape pin for the continue/break copy),
`codegen_ownership_spec_test.rs:497` (defer shadowing). No test pins `__cl_b` block labels. The
design keeps every listed pin green; a red pin stops the task (surface-during-authoring) and is
decided with the fix in hand.

**AR-11:** Phase map (detail in `99-execution-plan.md`): Phase 1 — per-site deferred copies with
namespaces (DEF-86, DEF-87 + the early-return sibling); Phase 2 — bounded closure and ambient
context (DEF-84 + the rung guard probe); Phase 3 — deferred return: remaining-actions continuation
plus the return-before-cleanup ordering (DEF-85, DEF-88 + the loop-local root cause); Phase 4 —
plan-wide hardening, final review, and close-out. Each defect phase carries the specification-first
cycle with its own ST cases and red-phase evidence.

**AR-11 (authoring amendment):** the register first recorded the challenger's recommended risk
order (ordering → copies → context → continuation). While deriving the ST set it became clear that
the filed DEF-88 shape — the one whose expectation names a return inside a defer — cannot pass
until the deferred-return continuation exists: after the ordering fix alone the emitted C reads
`__cl_ret = len(s); drop(s); goto __cl_c1;` and the self-loop turns the SEGV into a hang. Moving
the ordering fix into the same phase as the continuation keeps every ST case green at its phase
end; the dependency analysis, mechanisms, and Option A verdict are unchanged. This amendment was
recorded before any plan document was written (the register is the only artifact on disk at this
point).

**AR-15 (runtime, delegated record):** Authority: AI — delegated by --auto-design · Root invocation
ID `ad-defer-integrity-20261009` · Eligibility: testing-strategy correction inside the confirmed
scope (the ST case's expectation must derive from the spec; no behavior or scope change) ·
Objective: an ST-3 oracle whose expectation follows from the frozen rules · Decision: bind
`let a: i32 = f(true) + f(false);` and return `count * 10 + a` · Evidence: the spec's
left-to-right evaluation order (`spec/04-expressions.md` §3) makes `count * 10` read the counter
before the calls (observed exit 1 with the emitted C evaluating the counter first); with the sum
bound first, both defer runs are observable and 21 follows · Rejected: changing the expectation to
1 pins an ordering artifact, not the defer behavior · Confidence: High — hand-traced from the spec
and confirmed by the phase's probe runs · Reopen triggers: none expected; the phase's red and green
runs re-verify the corrected program · Policy version 1.

**AR-14:** The challenger's missing-option search (outlined helpers — impossible in C99 without an
environment/ABI change; static per-position label chains — cannot split the inline normal/return
continuations without duplicating remaining action code; mechanical IR cloning at lowering — keeps
every hard part and adds IR growth; diagnose-and-reject — contradicts §13) found no smaller
mechanism and no material support surface. All changes live in the existing
`compiler/src/own/` and `compiler/src/codegen/` modules; no new dependency, crate, harness, or
external surface. No Complexity Escalation Gate packet is open.
