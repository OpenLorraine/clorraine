# Preflight Report: Partial-Move / Drop Coherence

> **Status**: ✅ PREFLIGHT PASSED — 20 findings across 3 iterations, all resolved
> **Iteration**: 3 (final)
> **Artifact**: Implementation plan at `plans/partial-move-drop-coherence/`
> **Git ref**: commit `757cae0e` · initial `99-execution-plan.md` blob `4cb4ab20`
> **Codebase Grounded**: 20+ source files examined; claims re-verified by compiling the
> reproduction programs with `target/debug/clc` and running under ASan
> **Last Updated**: 2026-09-26 00:45

> **SAME-SESSION REVIEW**: This artifact was created in the current session.
> Same-agent bias risk is elevated. Four independent auditors were dispatched
> (soundness, grounding, delivery, risk/fit) to counter it; their findings converged.

## Codebase Context Summary

**Tech Stack:** Rust (std-only) workspace; binary `clc` compiles Clorraine to C99.
**Architecture:** `own/` (ownership + IR lowering) → `ir` → `codegen/` (C emission); a frozen `spec/`.
**Key Files Examined:** `own/lower.rs`, `own/expr.rs`, `own/statements.rs`, `own/place.rs`,
`own/call.rs`, `own/test_support.rs`, `ir.rs`, `ir/place.rs`, `codegen/function.rs`,
`codegen/drop.rs`, `codegen/cleanup.rs`, `tests/codegen_run_spec_test.rs`,
`tests/stage_spec_test.rs`, `tests/determinism_spec_test.rs`, `spec/17-codegen.md`,
`spec/03-ownership.md`, `spec/07-structs.md`.

**Confirmed sound:** `PlaceBase::Temp`/`new_temp`/`materialize_value` exist and codegen handles a
`Move` to a temp; the spec citations are accurate (incl. `spec/17-codegen.md:192`); the central
observation (double free) is real; no phantom references.

## Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 1 | pending |
| MAJOR | 3 | pending |
| MINOR | 7 | pending |
| OBSERVATION | 1 | pending |

---

### PF-001: Phase 2 is unnecessary and its assignment reorder breaks the immutable spec oracle [🔴 CRITICAL]

**Dimension:** Feasibility / Codebase Alignment
**Location:** `03-02-assignment-ordering.md` §Implementation Details; `99-execution-plan.md` Phase 2
**Codebase Evidence:** `compiler/src/codegen/function.rs:494-501` (drop-then-store fast path),
`:620-623` (`value_reads_base`); `compiler/tests/codegen_run_spec_test.rs:488-515`
(`assignment_drops_the_old_value` expects `open/close/open/close`); the immutable-oracle rule.

**The Problem:** Once Phase 1 consumes the literal field into a temp
(`Statement::Move{target: temp, source: b.value}` before the `Statement::Assign`), the
assignment's RHS references the temp, not the target, so `value_reads_base` is **false** and the
existing fast path already frees once. Phase 2 is therefore unnecessary, its task 2.1.2 cannot
confirm red, and its unconditional "evaluate RHS into a temp then drop then store" reorders the
non-reading case to `open/open/close/close` — breaking the spec oracle and the AR-8 byte-identity
claim. Auditors A, B, and D reproduced this by compiling the programs.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Drop Phase 2 and AR-3's assignment clause; keep Phase 1 only | Simpler, preserves the spec oracle and non-reading emission | Loses the (unneeded) defense-in-depth |
| B | Keep Phase 2 but retain the `!value_reads_base` fast path and only fix the save branch | Keeps a safety net | The save branch is unreachable after Phase 1 → dead code |

**Recommendation:** Option A — Phase 1 alone fixes both DEF-11 forms; removing Phase 2 preserves
the immutable oracle and byte identity.

### PF-002: A consumed aggregate-literal field is not move-marked in the ownership flow [🟠 MAJOR]

**Dimension:** Completeness / Edge Cases / Codebase Alignment
**Location:** `03-01` §Implementation Details; `01` R8; `07` ST-6; AR-14
**Codebase Evidence:** `compiler/src/own/expr.rs:133-137` (literal eval only recurses),
`own/place.rs:170-229`/`232-244` (only `move_target`/`move_place` mutate move state),
`own/statements.rs:266-272` (the only caller for bindings).

**The Problem:** The plan's `consume_place` emits only IR; it does not update the ownership flow
state. So a second read of the same field is **not** `E10020`, and a deeper path is **not**
`E10026`: `p = Pair { first: p.first, second: p.first }` and `Box { value: o.inner.s }` compile
and mis-handle (the auditors reproduced both). AR-14's "second read is already E10020" is false.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Mark a consumed non-duplicable literal field via `move_target` in the ownership stage | Enforces E10020/E10026 and records the partial move; one rule | One extra task in Phase 1 |
| B | Leave the flow state unchanged and drop the E10020/E10026 claims | Smaller | Silent miscompile on legal-looking input |

**Recommendation:** Option A — without it the "general invariant" is unsound.

### PF-003: Gap 3 (`take`-field) is misdiagnosed; ST-7/ST-10 cannot go red [🟠 MAJOR]

**Dimension:** Implicit Assumptions / Codebase Alignment
**Location:** `02-current-state.md` Gap 3; `03-01` §Integration Points, Example 2; `07` ST-7/ST-10; `99` 1.1.2
**Codebase Evidence:** `compiler/src/parse/expr.rs:375-393` (`b.value` is `ExprKind::Qualified`),
`own/lower.rs:30-33`, `own/call.rs:317` (matches `ValueKind::Place`); the auditors reproduced
`consume(b.value)` emitting `b.value = NULL;`. The failing shape is the `Member` form `(b).value`.

**The Problem:** `take_move_out_places` already zeroes a dotted field argument; ST-7/ST-10 are green
today. Also `consume_place` (`&mut self`, returns `Value`) cannot be called from
`take_move_out_places` (`&self`, returns `Vec<Place>`) as specified. The real remaining hole is a
postfix `Member` projection `(b).value`, which does double-free.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Retarget ST-7/ST-10 to `(b).value` and consume during argument lowering | Fixes a real hole; honors the "general invariant" | Adds a task and touches call lowering |
| B | Remove Gap 3, R7, ST-7/ST-10 and the `call.rs` change | Keeps strict scope tight | Leaves the `Member` double-free unfixed (tracked separately) |

**Recommendation:** Option B for strict scope — the common form already works; file the `Member`
shape as a new defect rather than expanding this plan.

### PF-004: ST-5 / AR-13 self-assignment is misdiagnosed [🟠 MAJOR]

**Dimension:** Consistency / Codebase Alignment
**Location:** `99` 2.1.2; `07` ST-5; `03-02` Error Handling; AR-13; `01` R7
**Codebase Evidence:** `compiler/src/own/statements.rs:179-189` (`s = s` is `Statement::Move`),
`codegen/function.rs:324-331` (Move drops then zeroes). Emitted `s = s` is
`if (s) FreeString(s); s = NULL; s = s; s = NULL;` and is ASan-clean.

**The Problem:** `s = s` does not flow through `emit_assign`; it goes through `Statement::Move` and
already frees once at the assignment (destroying the value). It is not red and is untouched by the
design. "Frees once" does not distinguish this from preserving the value to scope exit.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Drop R7/ST-5 and the AR-13 claim | Honest; matches current behavior | Loses a (non-bug) case |
| B | Keep ST-5 but assert the value survives to scope exit | Stronger semantic pin | Self-assignment destroys the value today → would be red, needing a behavior change |

**Recommendation:** Option A — self-assignment is not a DEF-11 case and already frees once.

### PF-005: ASan in the spec suite makes the immutable oracle environment-dependent [🟡 MINOR]

**Dimension:** Testability
**Location:** `07` §ASan Oracle; `99` 1.1.1
**Codebase Evidence:** `AGENTS.md` test-naming map separates spec vs. impl tests;
`codegen_run_spec_test.rs` is a spec oracle; LSan is on by default with ASan.

**The Problem:** Putting ASan into a spec suite changes its meaning and ties an immutable oracle to
sanitizer/OS versions; "exit 0 and no AddressSanitizer output" also asserts leak-freedom.
**Recommendation:** Move the ASan compile-and-run oracle to an impl suite (e.g.
`v1_language_impl_test.rs` or a new `partial_move_impl_test.rs`), set `ASAN_OPTIONS` explicitly,
and keep the spec-suite assertions sanitizer-independent.

### PF-006: AR-8 / ST-8 byte-identity is not pinned [🟡 MINOR]

**Dimension:** Testability / Consistency
**Location:** AR-8; `01` acceptance 4; `07` ST-8; `02` risk table
**Codebase Evidence:** `determinism_spec_test.rs:104-107` compares two runs of the same source;
`corpus_spec_test.rs` checks runtime output; no committed `.c` goldens.
**The Problem:** No suite compares against a stored baseline, so "byte-identical" is unenforced.
**Recommendation:** With Phase 2 dropped, restate the guarantee as "the existing fast path is
preserved for a non-reading RHS" and rely on determinism/corpus, or add a real golden.

### PF-007: `consume_place` omits the `mentions_type_param` guard [🟡 MINOR]

**Dimension:** Edge Cases / Codebase Alignment
**Location:** `03-01` §New/Changed Functions; AR-1; AR-11
**Codebase Evidence:** `own/lower.rs:501,554,587` (sibling materializers bail on `mentions_type_param`);
`types/table.rs:376-385` (`duplicable(Param)` is `true`); generic bodies lower once with `Param`.
**The Problem:** In a generic body a `Box<T>` literal move is emitted un-zeroed, so the "general
invariant" does not hold there.
**Recommendation:** Add the same guard and state the generic-body boundary (or leave it to the
paused generic plan).

### PF-008: `emit_store` refactor is under-specified [🟡 MINOR]

**Dimension:** Feasibility
**Location:** `03-01` §Integration Points; `99` 1.2.2; `02` Gap 3
**Codebase Evidence:** `own/statements.rs:179-189` (whole-place move is one `Statement::Move`).
**The Problem:** "Express `emit_store` through the same rule" could change whole-place moves to
Move-to-temp + Assign, altering emitted C.
**Recommendation:** Specify that `emit_store` keeps its direct `Statement::Move` and shares only
the `duplicable` test (or remove it from scope, per PF-003).

### PF-009: ST-10 is placed in a unit-test module, not a spec suite [🟡 MINOR]

**Dimension:** Testability / Consistency
**Location:** `07` §Test placement; `99` 1.1.1
**Codebase Evidence:** `AGENTS.md` test-naming map (`*_spec_test.rs` = specification tests;
in-module `mod tests` = unit tests).
**Recommendation:** Move the emitted-C/IR case to a `compiler/tests/*_spec_test.rs` suite, or drop
it with PF-003.

### PF-010: Per-step Verify commands are not runnable on the host [🟡 MINOR]

**Dimension:** Feasibility / Consistency
**Location:** `99` Verify lines
**Codebase Evidence:** host has no Rust toolchain; `AGENTS.md` routes `cargo test` through the
container.
**Recommendation:** Wrap per-step verifies as `make -C docker exec CMD="cargo test ..."` or rely on
`make check` for intermediate gates (AR-5).

### PF-011: Guarded/short-circuit owned places are unaddressed [🟡 MINOR]

**Dimension:** Edge Cases
**Location:** `03-01` §Integration Points
**Codebase Evidence:** `own/lower.rs:149-157` lowers both `Conditional` branches eagerly; the guard
deferral precedent is `own/lower.rs:504-507`.
**The Problem:** `cond ? b.value : other` would either stay unfixed or zero the source
unconditionally.
**Recommendation:** Scope guarded owned places out explicitly (and document it), or defer the move
to the guarded path.

### PF-012: Stale line range and overstated "single owner" [🔵 OBSERVATION]

**Dimension:** Consistency
**Location:** `02-current-state.md:13`; `03-01:28-29`; `99` criterion 4
**Codebase Evidence:** `emit_move_zero` is at `codegen/function.rs:553-561`, not `551-561`; the
return path zeroes via `codegen/cleanup.rs:264-273`, not the helper.
**Recommendation:** Correct the line range and restate "one owner in the ownership stage".

---

## Verdict (pending decisions)

**❌ PREFLIGHT BLOCKED — 4 critical/major unresolved (PF-001, PF-002, PF-003, PF-004).**
A pass requires resolving them; PF-005..PF-012 should be accepted or fixed.

---

## Resolution Decisions (Iteration 1)

> Recorded 2026-09-26. Selecting a resolution is not fix authorization; the artifact is
> unchanged until the user instructs "apply fixes".

| Finding | User Decision |
|---------|---------------|
| PF-001 | Resolved — User chose: **drop Phase 2** and AR-3's assignment clause |
| PF-002 | Resolved — User chose: **add ownership move-marking** for consumed literal fields |
| PF-003 | Resolved — User chose: **remove Gap 3**, R7, ST-7, ST-10, and the `call.rs` change |
| PF-004 | Resolved — User chose: **drop R7/ST-5** and the AR-13 claim |
| PF-005 | Resolved — User chose: move ASan to an impl suite; set `ASAN_OPTIONS` |
| PF-006 | Resolved — User chose: restate the byte-identity guarantee |
| PF-007 | Resolved — User chose: add the `mentions_type_param` guard / state the boundary |
| PF-008 | Resolved — User chose: `emit_store` out of scope (per PF-003) |
| PF-009 | Resolved — User chose: drop ST-10 (per PF-003) |
| PF-010 | Resolved — User chose: wrap per-step verifies in the container |
| PF-011 | Resolved — User chose: scope guarded places out explicitly |
| PF-012 | Resolved — User chose: correct the line range and "single owner" wording |

**Verdict after fixes are applied:** expected `✅ PREFLIGHT PASSED` (all critical/major resolved;
minors accepted). The report's pass tier is not claimed until the artifact is edited.

---

## Iteration 2 (re-scan after fixes applied)

> **Status**: RE-SCAN — 8 new findings (0 critical, 4 major, 2 minor, 2 observation), all resolved
> **Carried forward**: PF-001, PF-002, PF-003, PF-004 — verified resolved

Two independent auditors verified the iteration-1 fixes. Verification confirmed the revised
design is feasible (temp mechanism, fast path, `mentions_type_param` guard) and PF-001/003/004/
005/008/009/010/011/012 are applied. New root causes were found:

| ID | Sev | Problem | Resolution |
|----|-----|---------|------------|
| PF-013 | 🟠 | ST-6 used `Pair<i32>`, whose field is duplicable, so `E10020` could never fire | ST-6 changed to `Pair<String>` with an owned field |
| PF-014 | 🟠 | The ownership mark and the lowering used different predicates (the mark lacked the `mentions_type_param` guard), and the lowering targeted `StructLit`/`ArrayLit` while the mark targeted `StructLit` only | One shared predicate ("non-duplicable and does not mention a type parameter") in both stages; the lowering restricted to the `StructLit` arm; arrays stay out of scope |
| PF-015 | 🟠 | `02-current-state.md` still said "The fix is therefore lowering-only", contradicting the added ownership mark | Reworded to "an ownership mark plus the lowering change; the assignment emitter is untouched" |
| PF-016 | 🟠 | `03-01` listed the mark branches without the type-parameter guard | Added the guard to the mark and stated the predicate once |
| PF-017 | 🟡 | Step 1.1's Verify omitted the diagnostics after ST-6/ST-7 moved suites | Regressions consolidated into `stage_spec_test.rs`; Verify updated |
| PF-018 | 🟡 | `ASAN_OPTIONS` was unspecified; ST-3's free-once was asserted in the run suite where it is unobservable | Pinned `ASAN_OPTIONS=detect_leaks=0`; ST-3 moved to the ASan suite |
| PF-019 | 🔵 | ST-6/ST-7 were labelled specification tests but placed in an in-module unit test | Moved to `stage_spec_test.rs` |
| PF-020 | 🔵 | The ASan oracle was placed in `v1_language_impl_test.rs`, an unrelated suite | Moved to a new `compiler/tests/partial_move_impl_test.rs` |

Also fixed: the `materialize_value` line cite (`lower.rs:524-534`) and the AR-3/AR-4
Resolution Notes, which had described the superseded designs.

**Iteration-2 verdict:** ❌ not clean — 4 major; fixes applied. Iteration 3 required (bounded to
these fixes).

---

## Iteration 3 (bounded re-scan of the iteration-2 fixes)

> **Status**: ✅ clean — 4 documentation-consistency findings, all fixed and directly verified

| ID | Sev | Problem | Resolution |
|----|-----|---------|------------|
| PF-021 | 🟠 | Task 1.2.2 said "field/element lowering", implying array elements are in scope | Reworded to "route the `StructLit` field lowering" |
| PF-022 | 🟡 | The AR-4 decision cell still named the superseded `v1_language_impl_test.rs` | Updated to `compiler/tests/partial_move_impl_test.rs` |
| PF-023 | 🟡 | `02-current-state.md` described the mark without the type-parameter guard | Added the guard to both rows |
| PF-024 | 🔵 | Step 1.1's reference read `ST-1..ST-9` while task 1.1.1 also covers ST-10 | Corrected to `ST-1..ST-10` |

Direct verification (grep) confirms no live document references the deleted
`03-02-assignment-ordering.md`, the superseded ASan suite, the "field/element"
scope, "lowering-only", or `ST-1..ST-9`; the only remaining occurrences are in this
historical report.

---

## Final Verdict

**✅ PREFLIGHT PASSED — all 20 findings resolved (4 critical/major in iteration 1;
4 major in iteration 2; 4 in iteration 3; the remainder minor/observation).**

PF-001..PF-020 were resolved and applied; PF-021..PF-024 were applied and directly
verified. The revised plan (2 phases, 10 tasks) is ready for execution.

**Roadmap:** advance the DEF-11 plan row to `Plan Preflighted` (🔬); it was recorded
as `Executing` prematurely — flagged for correction per the roadmap never-regress rule.
Since execution follows immediately, the row may instead proceed to `Executing`.

