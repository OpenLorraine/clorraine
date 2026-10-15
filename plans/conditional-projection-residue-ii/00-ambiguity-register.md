# Ambiguity Register: Conditional, Projection, and Update Residue II

> **Status**: ✅ GATE PASSED — the planning decisions are recorded (auto-design delegated); no
> batch row is parked; the six older rows stay parked for user rulings (AR-2); the runtime
> decisions below name their stop conditions
> **Last Updated**: 2026-10-15 (planning)
> **Batch**: J of the defect mandate (the batch-I follow-on rows: 27 handled — 1 critical, 5
> majors, 21 minors; none parked)
> **Auto-design**: active — policy version 1 (the standing user mandate: all defects first,
> batches continue without further go-aheads; criticals first, then majors). Reserved decisions
> (spec text, new user-visible behavior, acceptance criteria) are **not** touched: every fix
> below is conformance to the frozen specification. Rows whose fix needs a spec or registry
> ruling stay parked and are collected in AR-2.

## Why this batch

Batch I closed its declared cluster (`DEF-257` – `DEF-280` minus the two incidental) and its
review rounds filed 30 follow-on rows (`DEF-281` – `DEF-310`). Of those, 27 remain open: the
only open critical in the backlog (`DEF-303` — a `String[2]` conditional-root return
segfaults), five majors (`DEF-286` cross-frame out-slot UAF, `DEF-288` container escape with
frame-backed element loans, `DEF-294` loop-condition mixed receiver, `DEF-295` loop-condition
member-projection double-free, `DEF-310` dropped raw-deref store), and 21 minors. Three rows
were fixed in batch-I follow-ups (`DEF-289`, `DEF-296`, `DEF-301`) and `DEF-278` completed with
its phase; all were re-verified at the planning head (AR-3). Under the standing
"criticals first, then majors" mandate this batch is next: it carries the only open critical,
the most majors of any open batch, one shared machinery (minimizing the review surface and
reusing the batch-I test infrastructure), and every row already carries a reproduction, a
base/head observation, and a fix direction.

| # | Category | Ambiguity / Gap | Options Presented | Resolution | Status |
|---|----------|-----------------|-------------------|------------|--------|
| 1 | Scope | Which rows batch J closes | A) the 27 open batch-I follow-on rows (1 critical, 5 majors, 21 minors); B) the batch E residue (2 unblocked majors); C) the batch F initializer / batch G checker residues | **A** — pre-decided by the dispatch and recorded, not re-litigated: the only open critical, the most majors, one machinery (the conditional/projection/update surface batch I just reviewed), and fresh evidence with fix directions for every row. B and C stay sequenced after | ✅ Resolved (pre-decided) |
| 2 | Scope | Rows whose fix needs a reserved ruling | A) include and pause at the ruling; B) park them, out of this batch | **B** — the parked list is exactly the six older rows: `DEF-167` (a call of a non-callable callee needs a diagnostic code), `DEF-169` (an escaping type-key scheme is an ABI decision), `DEF-171` (staged `embed`), `DEF-190` (a nameless nullable source diagnostic), `DEF-191` (the `unsafe` exemption), `DEF-263` (whether a folded fixed-array `.len` receiver evaluates). None of the 27 needs a spec or registry edit: every fix reuses the frozen rules and the frozen codes (`E10020`, `E10030`, `E10031`, `E10033`, `E10035`), and the two design risks below (`DEF-286`, `DEF-294`) are implementation decisions, not spec changes | ✅ Resolved |
| 3 | Verification | The current status of the follow-up rows | probe the actual compiler at the batch-I head vs the roadmap notes | **The probe table below (AR-3)** — `DEF-278`/`289`/`296`/`301` verified fixed (their roadmap "Done" statuses hold); `DEF-306`/`308` verified partial (the landed part green, the residual red); the other in-scope reds confirmed (including the critical and all five majors) | ✅ Resolved |
| 4 | Design | Phase grouping and sequencing | A) five phases by machinery, executed 1 → 2 → 3 → 4 → 5; B) one row per phase; C) one phase for all ownership rows and one for all emission rows | **A** — the rows group cleanly by the machinery they touch (owners/loans/stores; guarded/discarded/carrier roots; fixed-array/literal emissions; loop/update/store lowering; diagnostics/marks); a phase per row would multiply the review overhead, and option C would mix the root binders with the emission shapes that depend on them. The dependency order is P1 → P2 → P3 → P4 → P5 (AR-4) | ✅ Resolved (delegated) |
| 5 | Design | Per-row handling mode | red → green spec case for every row, or a control/incidental where a read-only probe shows the row already fixed | **Red → green for all 27**, with the already-landed partial fixes as green controls: `DEF-306`'s store/return/iterable distribution and `DEF-308`'s plain loop-condition sibling are verified green and pinned as controls; `DEF-294`'s red is host-toolchain-only and is re-measured in the phase (AR-10); no row closes incidentally at planning time (AR-3) | ✅ Resolved |
| 6 | Design | `DEF-286`'s cross-frame rule shape | A) a call-site rule: when a view argument's backing is a local of the caller's frame and the callee stores it into a further-out out-slot, the judgment propagates the argument's backing frame and rejects; B) a recorded limit | **A, with a stop condition** — the frozen BR-8 rule (Ch 03 §7) rejects a view stored where it outlives its backing, and the call-site rule is the bounded conformance shape; if the rule cannot be expressed without over-rejecting the accepted parameter-backed twins, the phase stops and the row is parked with the evidence (a recorded limit is itself a user-visible acceptance decision, so it is not auto-approved) (AR-6) | ✅ Resolved (delegated, stop-gated) |
| 7 | Behavior | `DEF-282`'s one-report boundary | A) one report per span: where two reports fire at the same span, emit one, and withhold a downstream consequence report where the callee-side judgment already reports (the `gfieldcall` E10033 at the store when the callee's return already reports `E10036`); B) keep both | **A** — the batch-I one-report rule (AR-26) is the precedent: one report carries the rejection. The phase pins the exact counts per span; the semantics lens reviews the boundary (different-site judgments are measured and kept where both are true — the decision is evidence-led, AR-7) | ✅ Resolved (delegated) |
| 8 | Design | `DEF-283`'s deferred-owner display | route the deferred owner's display through the existing generated-owner phrase | **Resolved** — the phrase `a compiler-generated owner` is already pinned by `checker_conformance_spec_test` for the sibling shape; the deferred spelling reuses it (no new message text) (AR-8) | ✅ Resolved |
| 9 | Design | `DEF-307`'s mark alignment | A) align `mark_read_conditional_move` with the in-place emission (the whole-array conditional iterable marks no arm as moved); B) keep materializing a copy | **A** — the batch-I phase-5 correction made the emission read the arm in place; the checker's move mark is the stale half, so the mark drops and the later use stays accepted. The red-run proves the marker is still live (AR-3, AR-9) | ✅ Resolved |
| 10 | Design | `DEF-294`'s toolchain scope | A) keep the row with a host-toolchain red and pin both builds; B) close it as a control on the pinned container's evidence | **A, re-measured** — the pinned container (GCC 12.2) accepts the emitted comma expression, while the host GCC 15 rejects it with `-Werror=sequence-point`; the fix direction (pin the receiver position, the DEF-277 pattern) makes the emission explicit. The phase re-measures both toolchains, pins the emission, and if the fix turns out to remove only the host warning while the container already accepts the standard-conforming spelling, the row closes as a control with that evidence (the batch-I re-decision precedent, AR-5/AR-10) | ✅ Resolved (delegated) |
| 11 | Verification | The command set | — | `make check` per phase; `make verify` at close-out; the focused suites (`cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test`); host ASan (`RDC_ASAN=1` and host probes) on every ownership phase; the emission auditor's base/head differential on P3/P4; strict builds on the pinned container and (for `DEF-294`) the host toolchain | ✅ Resolved |
| 12 | Naming | Plan and suite names | new suite pair vs appending to batch I's | **Append to the existing pair**: `compiler/tests/conditional_projection_residue_spec_test.rs` and `conditional_projection_residue_impl_test.rs` — the machinery is the same and the batch-I suite is the regression oracle; the new cases append with additions only. The `AGENTS.md` test map updates only if its wording needs the batch mention at close-out | ✅ Resolved |
| 13 | Lenses | Per-phase reviewer dispatch | — | P1/P2: correctness + memory-safety; P3: correctness + emission-integrity + memory-safety; P4: correctness + emission-integrity + semantics + memory-safety (the loop-condition root binding is an ownership fix); P5: correctness + semantics. The read-only auditors stay read-only; the project briefs carry correctness/semantics | ✅ Resolved |
| 14 | Complexity | Escalation triggers | — | None — every fix extends existing machinery (the owner/provenance walk, the store judgment, the root binders, the discard/literal materializers, the fixed-array destination and arm shapes, the update pin, the loop fill, the report renderers); no new architecture, dependency, or cross-cutting refactor | ✅ Resolved |
| 15 | Deviation (runtime) | A phase's named fix surface proves narrower or wider than the row | A) stop and re-plan; B) extend the phase's task files to the needed adjacent modules with the evidence, and record the deviation here | **B** — the batch-I precedent: a conformance fix may touch the checker or codegen module that owns the judgment; the deviation is recorded and the phase review covers the extra files | ✅ Resolved (runtime, delegated) |
| 16 | Specialist gap | Missing specialist types for this surface | — | None — `correctness-reviewer`, `semantics-reviewer`, the read-only `memory-safety-auditor`, and the read-only `emission-integrity-auditor` cover the batch; recorded in the index's Specialist Agents section | ✅ Resolved |
| 17 | Test placement | The new cases' suite | — | The same two suites as batch I (AR-12): `compiler/tests/conditional_projection_residue_spec_test.rs` (ST cases, red then green) and `_impl_test.rs` (emission/report pins); additions only, the committed batch-I cases untouched | ✅ Resolved |

## Verified Current Status (AR-3 probes, 2026-10-15)

Probe method: `make -C docker exec CMD="cargo build --workspace"`, then
`target/debug/clc --profile host -o out.c prog.clo` and
`cc -std=c99 -O0 -g -Wall -Wextra -Werror` (pinned container GCC 12.2; the host GCC 15.2
recorded where it differs), with `-fsanitize=address` runs for the memory rows. Probe sources
under `build/probe-batch-j/` (gitignored scratch, removed at planning close).

| Row | Probe | Verified current behavior |
|-----|-------|---------------------------|
| `DEF-278` (out of scope — completed) | `f(g(), i++)` owning argument | ✅ Fixed: strict build, run exit 0, ASan-clean |
| `DEF-289` (out of scope — closed) | `return Holder { names: [p] };` parameter-backed literal | ✅ Fixed: strict build, run exit 0, ASan-clean |
| `DEF-296` (out of scope — closed) | discarded literal field's place projection | ✅ Fixed: strict build, run exit 0, ASan-clean |
| `DEF-301` (out of scope — closed) | `(c ? mk() : b2).xs` ownerless arm | ✅ Fixed: strict build, run exit 0, ASan-clean |
| `DEF-306` (partial) | `(c ? (d ? b1 : b2).ys : b3.ys)[i]` element-read position | ❌ Red: strict build fails `lvalue required as unary '&' operand` (container and host) |
| `DEF-306` (landed part) | the same conditional's store and `for`-iterable positions | ✅ Green control: strict build, run exit 0, ASan-clean |
| `DEF-307` | `for (borrow x in (c ? b1 : b2))` with `String[2]` arrays, later use of `b1` | ❌ Red: `E10020: 'b1' was moved and cannot be used` (the `u8[4]` twin is accepted — the mark is per owned element) |
| `DEF-308` (partial) | `g && ok(f(i++, i))` and `c ? f(i++, i) : 0` | ❌ Red: strict build fails `-Werror=sequence-point` (container and host) |
| `DEF-308` (landed part) | `while (f(i++, i) < 30) { … }` plain loop condition | ✅ Green control: strict build, run exit 0 (left-to-right values) |
| `DEF-294` (major) | `while (i < 1 && (c ? mkH() : a).call(5) > 0)` | Toolchain split: container strict build + run exit 0; host GCC 15 strict build fails `-Werror=sequence-point`; emission byte-identical to the phase-4 tree |
| `DEF-295` (major) | `while (i < k && i32(len(mkH().s)) > 0)` | ❌ Red: strict build, run aborts 134 (glibc double free); ASan `attempting double-free` |
| `DEF-303` (critical) | `String[2]` conditional-root return `(c ? b1 : b2).xs` by value | ❌ Red: strict build, run segfault 139 (ownerless per-arm `__cl_ret = __cl_tmp` assignment) |
| `DEF-310` (major) | `unsafe { *q = 9; }` | ❌ Red: strict build, run exit 254 — the store emits nothing |
| `DEF-281` (minor) | `holdNum(c ? a.n : mkBox().n)` fresh arm | ❌ Red: strict build, run exit 0, ASan 11 B leaked in 1 allocation |
| `DEF-282` (minor) | `return (c ? a : (d ? mkBox() : mkBox2())).v;` | ❌ Red: two identical `E10030` reports at one span (12:5) |
| `DEF-283` (minor) | deferred `return (c ? l : mkBox()).v;` | ❌ Red: `borrow of '__cl_owned1' does not live long enough` (no generated-owner phrase) |
| `DEF-291` (minor) | `slot = c ? null : null;` into a `borrow str?` out-slot | ❌ Red: false `E10030: borrow of a statement temporary` at the store |

Rows not probed at planning time (`DEF-285`, `286`, `287`, `288`, `290`, `292`, `293`, `297`,
`298`, `299`, `300`, `302`, `304`, `305`, `309`) carry the roadmap's recorded reproduction and
base/head evidence; the phase red-runs re-verify each before implementation.

## Resolution Notes

**AR-1:** The dispatch fixed the scope as the 27 open batch-I follow-on rows; the register
records it without re-litigating. Severity mix: 1 critical, 5 majors, 21 minors. The closed
follow-ups (`DEF-289`, `296`, `301`) and the completed `DEF-278` are out of scope and appear
only as verified status (AR-3) and regression controls.

**AR-2:** Parked rows and the decision each needs:
`DEF-167` — the diagnostic code for calling a non-callable callee (shared with `DEF-214`);
`DEF-169` — whether to add an escaping scheme for generated type keys (an ABI decision);
`DEF-171` — whether a staged `embed` outside a constant context stays rejected;
`DEF-190` — the diagnostic for a nameless nullable source;
`DEF-191` — whether the `unsafe` exemption for the null-use rule matches the conversion rule;
`DEF-263` — whether a folded fixed-array `.len` receiver evaluates its operand.
None of the 27 in-scope rows needs a spec or registry ruling: their fixes reuse the frozen
rules (TR/ownership BR-8, the single-view parity, the Ch 04 §3 evaluation order, the Ch 17
destination ABI) and the frozen diagnostic codes. A row that turns out to need a ruling is
parked mid-phase with the evidence (the stop conditions in AR-6 and AR-10).

**AR-3:** Probe evidence is the table above; the probe sources and logs live under
`build/probe-batch-j/` during planning (removed at planning close: they are scratch, not
artifacts). The verified statuses overturn no roadmap row: `DEF-306`/`308` keep their
"partially fixed" statuses with the residual confirmed, and the remaining rows keep "Backlog"
until their phases close them.

**AR-4:** The phase grouping and dependency order:
P1 view owners/loans/stores (the owner/provenance walk, the store judgment, the module-init
path) → P2 guarded/discarded/carrier roots (the root binders and the discard/literal
materializers extend P1's binding rules) → P3 fixed-array/literal emissions (the per-arm
destination and the literal field routes consume P1/P2's bound roots) → P4 loop conditions,
update order, and store lowering (the loop fill and the update pin; `DEF-295` reuses P1's root
binding at the loop condition) → P5 diagnostics and marks (the report renderers and the move
mark align with the emissions the earlier phases settled). Executed strictly 1 → 2 → 3 → 4 → 5;
`make check` gates every phase.

**AR-5:** Handling modes: every in-scope row is red → green. The two partial rows' landed parts
(`DEF-306`'s store/return/iterable distribution; `DEF-308`'s plain loop-condition sibling) are
verified green and pinned as controls so the fixes cannot regress; `DEF-294`'s red is
host-toolchain-only and the phase re-measures it (AR-10). No row is parked at planning time.

**AR-6:** `DEF-286`'s call-site rule: at a call whose argument is a view derived from the
callee's frame, and whose callee may store the view into an out-slot that lives in a frame
further out, the argument's backing frame relation is propagated so the store rejects like its
direct twin would. Stop condition: if the phase cannot implement the rule without rejecting the
accepted parameter-backed and same-frame twins, the phase stops, records the probes, and parks
`DEF-286` for a user ruling (a recorded limit changes the accepted surface, which is reserved).

**AR-7:** `DEF-282`'s one-report boundary extends batch-I AR-26: one report per span. Where two
reports fire at one span, one is withheld (the `E10030` pair at the nested mixed return). Where
a downstream consequence report fires at another span while the callee-side judgment already
reports (`gfieldcall`: `E10033` at the store, `E10036` at the callee's return), the phase
measures both sites and withholds the consequence report per the row's "misleading second-site"
wording; the semantics lens reviews the boundary and the pinned counts.

**AR-8:** `DEF-283` routes the deferred owner's display through the existing
`a compiler-generated owner` phrase, already pinned by `checker_conformance_spec_test`; no new
message text.

**AR-9:** `DEF-307`: the in-place conditional iterable (batch-I phase-5 correction) and the
checker's move mark disagree; the mark is dropped so a later use stays accepted, matching the
emission and the pinned `fixediterowned` behavior.

**AR-10:** `DEF-294`'s toolchain evidence: the pinned container (GCC 12.2) accepts the emitted
comma expression (strict build + run exit 0) while the host GCC 15 rejects it
(`-Werror=sequence-point`); the emission is byte-identical to the batch-I phase-4 tree where
the row was filed. The fix direction (pin the receiver position like `DEF-277`) makes the
emission explicit; the phase pins the emission and both builds, and decides the row's closure on
the measured evidence: **fixed** if the pinned emission changes; a **control** outcome requires
the phase to record why the remaining host warning does not represent undefined behavior (a
sequence-verified spelling), else the row stays open and the fix direction is revisited.

**AR-11:** `DEF-282`/`283`/`291` are rejection-shaped minors: their ST cases assert the exact
report counts, codes, and spans (or the absence of a report), not a run value.

**AR-12:** The new cases append to `compiler/tests/conditional_projection_residue_spec_test.rs`
and `compiler/tests/conditional_projection_residue_impl_test.rs` (additions only): the machinery
is the same, the batch-I suite is the regression oracle, and a new pair would duplicate the
harness.

**AR-13:** Lens justification: P1/P2 change ownership judgments and ownership bindings
(memory-safety); P3 changes fixed-array destination and literal emission shapes (emission);
P4 changes the loop fill, the update pin, the raw-deref lowering, and the loop-condition root
binding (emission + ownership + acceptance); P5 changes report counts and the pinned phrase
(semantics).

**AR-14:** Complexity: every fix rides an existing hook — `view_provenance`/`storage_atoms`
(`place.rs`), `prepare_store`/`check_return` (`statements.rs`), `check_escaping_view_store`
(`expr.rs`), the root binders and discard/literal materializers (`lower.rs`), the fixed-array
destination shapes (`function.rs`, `guards.rs`, `cleanup.rs`), `pin_update_value`/`contains_update`
(`lower.rs`), and the loop fill (`loops.rs` + `guards.rs`).

**AR-15:** The runtime deviation protocol follows batch I: a phase's fix may touch the module
that owns the judgment beyond the task's file list; the deviation is recorded in this register
and reviewed by the phase's lenses.

## Open Questions (collected for the user — none blocks execution)

- The six parked rows' rulings (AR-2) are still due; they stay out of this batch.
- `DEF-286`'s stop condition (AR-6) may return a parked row with fresh evidence.
- `DEF-294`'s final classification (AR-10) is recorded in the findings ledger either way.

**AR-16 (runtime):** The phase-1 red phase re-derives three filed reds: `DEF-285`'s mutation reports already fire at the head (the landable reds are the owner stranding and the accepted fresh-backed loop store), `DEF-287`'s direct `stringOf` spelling is already clean (the member-projection spelling is the red), and `DEF-286`'s non-nullable `borrow slot: str` chain is a dead write, not a use-after-free (the nullable `str?` spelling is the red). The cases pin the observed reds; the fix directions are unchanged.
