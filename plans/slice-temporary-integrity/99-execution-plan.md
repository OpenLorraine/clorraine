# Execution Plan: Slice and Temporary Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-08 16:35
> **Progress**: 18/34 tasks (53%)
> **CodeOps Artifact Schema**: 1

## Overview

Four phases close the seven defects of this batch: row-typed slice views (DEF-70), deferred consume
preparation (DEF-72/73/74), root and receiver binding (DEF-71/89), and the element-store drop
spelling (DEF-75) with the plan-wide hardening. Each phase follows the specification-first ordering
and ends with its review dispatch per `AGENTS.md`.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Row-typed slice views (DEF-70) | 11 |
| 2 | Deferred consume preparation (DEF-72/73/74) | 8 |
| 3 | Root and receiver binding (DEF-71/89) | 8 |
| 4 | Element-store drop spelling (DEF-75) and plan hardening | 7 |

**Total: 34 tasks across 4 phases** (scope is bounded by the task-size criteria in
`quality-checklist.md`)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for progress.
> Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated stamp
>    after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed first,
>    else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Row-typed slice views (DEF-70)

> **Phase baseline tree**: c1cbb077038b00df95c915da624f10fac7268174
> **Lenses**: compiler-and-language; `spec/17-codegen` ABI lens (type keys, typedefs, lowering)
> **Reasoning**: high — type-model, symbol-key, and emission changes with a cross-cutting audit

### Step 1.1: Specification Tests

**Reference**: [03-01 §Proposed Changes](03-01-slice-row-views.md) · [07 ST-1 – ST-5](07-testing-strategy.md) · [AR-3, AR-9](00-ambiguity-register.md)
**Objective**: Pin the row-typed behavior before any implementation change.

- [x] 1.1.1 [spec-author] Write specification tests ST-1 – ST-5 in `compiler/tests/slice_temporary_integrity_spec_test.rs` on the established harness (strict host build, host ASan, diagnostic asserts) ✅ (completed: 2026-10-08 09:26)
- [x] 1.1.2 Run the new suite and record the red phase under the host-ASan command named in 07 (the generic row move double-frees; nested access mis-types; the `E10200` mismatch is accepted; ST-1 is recorded as a justified pass) ✅ (completed: 2026-10-08 09:26)

**Deliverables**:
- [ ] ST-1 – ST-5 in the new spec suite
- [ ] Red-phase evidence

**Verify**: `make check` (the new suite fails as expected; the existing suites stay green)

### Step 1.2: Implementation

**Reference**: [03-01 §Implementation Details](03-01-slice-row-views.md) · [AR-3, AR-9](00-ambiguity-register.md)
**Objective**: Make the view's element the row end to end.

- [x] 1.2.1 Type the slice result from the source's view element and add the explicit type-argument agreement check (`E10200`) — `compiler/src/own/call.rs`, `compiler/src/types/mod.rs` ✅ (completed: 2026-10-08 09:36)
- [x] 1.2.2 Key and register row-typed views; skip parameter-typed rows — `compiler/src/types/keys.rs`, `compiler/src/codegen/types.rs` ✅ (completed: 2026-10-08 09:36)
- [x] 1.2.3 Emit the row-typed typedef and builder — `compiler/src/codegen/program.rs`, `compiler/src/codegen/calls.rs` ✅ (completed: 2026-10-08 09:36)
- [x] 1.2.4 Replace the recovery gate with element agreement — `compiler/src/own/lower.rs` ✅ (completed: 2026-10-08 09:36)
- [x] 1.2.5 Migrate the flat-name shape pins this step breaks (`compiler/tests/residual_defect_completion_impl_test.rs`, `compiler/tests/array_value_integrity_impl_test.rs`, `compiler/src/codegen/types.rs` unit tests) so `make check` can be green ✅ (completed: 2026-10-08 09:36)
- [x] 1.2.6 Run the new suite and record the green phase ✅ (completed: 2026-10-08 09:36)

**Deliverables**:
- [ ] Row-typed view type, key, typedef, builder, and access path
- [ ] ST-1 – ST-5 green

**Verify**: `make check`

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01 §Testing Requirements](03-01-slice-row-views.md)
**Objective**: Pin the emitted shapes and prove memory safety.

- [x] 1.3.1 Write implementation tests for the typedef key and pointer declarator, the `zero_view_helper` key, a nullable row view, an empty `return;` from a view-returning function, the builder and index spellings, and byte-identical re-emission — `compiler/tests/slice_temporary_integrity_impl_test.rs` ✅ (completed: 2026-10-08 09:38)
- [x] 1.3.2 Run the host-ASan probe matrix for the ST-1 – ST-5 shapes (row move, nested access, sub-slice, bounds trap) and the strict `-Werror` builds; record the results ✅ (completed: 2026-10-08 09:38)
- [x] 1.3.3 Dispatch the phase review — correctness plus the memory-safety and emission-integrity auditors, with the semantics reviewer on the view/element ownership boundary and the `spec/17-codegen` ABI lens; every packet carries the blast-radius (every key/typedef/builder/index call site) and test-integrity (assertions fail without the fix) steps; resolve in-scope findings; one bounded re-review of a fix diff ✅ (completed: 2026-10-08 13:23)

**Deliverables**:
- [x] Implementation suite and migrated pins
- [x] Clean probe matrix
- [x] Phase review with no open critical/major finding

**Verify**: `make check`

**Phase 1 review findings (2026-10-08 09:40)** — four reviewers dispatched in parallel on the
phase diff (baseline `c1cbb077`): correctness, memory-safety, emission-integrity, and semantics
(the last two per the `AGENTS.md` matrix for symbol-key/emission and view/element ownership
changes). Memory safety was clean (23 host-ASan probes over the changed shapes, release-count
table) and emission integrity found no `PE-*` finding (corpus emission byte-identical except
deliberate multi-extent users, each traced to R1; every accepted variant strict-built; no
declaration mismatch). Three 🟠 MAJOR findings, resolved by user ruling:

| ID | Severity | Finding | Status |
| -- | -------- | ------- | ------ |
| RV-001 | 🟠 MAJOR | An inferred row view used where a different slice type is expected was accepted and emitted uncompilable C (undeclared `ClSlice_arr3_i32`); the base emitted valid C. The type stage typed every `slice` result as unknown, so the context conversion never saw the row type. | ✅ resolved — user chose: type the `slice`/`rawSlice` result in the type stage (explicit type argument, else the source's iterated element) and align the two row representations in the conversion rules |
| SR-001 | 🟠 MAJOR | A `const`-sized row extent in a written view (`i32[][M]`) resolved with the literals-only resolver in codegen, so the callee typedef referenced an undeclared `ClSlice_slice_i32`; the base ran it. | ✅ resolved — user chose: fold constant/arithmetic extents with the existing `ConstFolder` in the declaration/type resolvers (type table, ownership, emitter), matching the type stage |
| SR-002 | 🟠 MAJOR | `slice<i32[M]>` / `slice<i32[2 + 1]>` were falsely rejected because the explicit type argument resolved literals-only. | ✅ resolved — the same folding resolver |
| SR-003 | 🟡 MINOR | The `E10200` message spelled the requested row view as `i32[3][]`, not a legal source spelling. | ✅ fixed — the view type is built in its written `T[][M]` form at the result sites |
| PE/PRE | — | Pre-existing defects reported separately by the reviewers (str-source slices, unannotated inference, compound-literal spelling, FFI/struct-extent resolution corners, generic borrow-view escape); base-identical evidence is in the review reports. | out of scope (not phase-introduced) |

**Fix evidence (2026-10-08 10:25)** — the finding fixes landed with spec-first regression coverage:

| Fix | Coverage | Result |
| --- | -------- | ------ |
| RV-001 (`slice`/`rawSlice` typed in the type stage; view-element conversion) | spec `should_reject_when_an_inferred_row_view_is_bound_to_a_scalar_view` (`E10200`); `implicit_conversion` accepts two views whose elements agree | the mismatched context rejects with `E10200` and writes no C (base accepted it); the agreeing written/inferred row views still compile and run |
| SR-001/SR-002/SR-003 (constant and arithmetic extents) | spec `should_run_when_a_const_sized_row_view_and_explicit_argument_agree` and `should_run_when_an_arithmetic_extent_names_the_source_row` (both exit 3) | the counterexamples compile strict-clean and run under host ASan; the unfixed head failed them (invalid C / false `E10200`); the diagnostic now spells `i32[][M]` |
| all | spec suite under host ASan; full `make check` | spec 8/8 ASan-clean (serial run; the parallel in-container run times out per the documented scheduling flake), `make check` green |

One bounded re-review is dispatched on the fix diff.

**Re-review outcome (2026-10-08 13:23)** — the single permitted re-review (correctness, semantics,
emission-integrity, memory-safety) confirmed the four fixes with independent red/green evidence
(the rejection case fails on the base, the const/arithmetic run cases fail on the pre-fix head)
and a clean memory pass (51 host-ASan probes, release-once throughout); the emission pass found
no fix-induced diff (corpus byte-identical; only the three new cases differ) and only
base-identical pre-existing build failures. It found one new 🟠 MAJOR in the fix itself:
the slice-to-slice conversion arm compared elements through the full conversion relation, so
element-widening pairs (`u8[]`→`u16[]`, `String[]`→`str[]`, `f32[]`→`f64[]`) were accepted and
emitted mismatched C view structs; the base and the pre-fix head both reject them. All three
code-reviewing agents prescribed the same minimal remedy. After the ruling request, the user
resumed execution and the recommended element-identity remedy was applied and verified: the
widening pairs report `E10200` again, the two representations of one row view still convert,
spec pins were added (`sti-st9`, `sti-st10`), the spec suite is 10/10 under host ASan, and
`make check` is green. Pre-existing defects reported by the round (const-extent struct fields,
an unregistered view-temporary typedef, view-struct ordering, freestanding `bool`) are
base-identical and remain out of scope. No further re-review remains under the protocol; the
targeted re-verify above is the final check.

---

## Phase 2: Deferred consume preparation (DEF-72/73/74)

> **Phase baseline tree**: c425d3e1da054f2126c395a4bb4eb24c4cf8e7bb
> **Lenses**: compiler-and-language; evaluation-order and statement-temporary contracts
> **Reasoning**: high — deferred-fill lifecycle across lowering and emission, order-sensitive

### Step 2.1: Specification Tests

**Reference**: [03-02 §Proposed Changes](03-02-deferred-consume-preparation.md) · [07 ST-6 – ST-8](07-testing-strategy.md) · [AR-4](00-ambiguity-register.md)
**Objective**: Pin the fill, the source order, and the member-rooted release before implementation.

- [x] 2.1.1 [spec-author] Write specification tests ST-6 – ST-8 in `compiler/tests/slice_temporary_integrity_spec_test.rs` ✅ (completed: 2026-10-08 13:32)
- [x] 2.1.2 Run the new suite and record the red phase under the host-ASan command named in 07 (the generic index call never runs; trace 21 for the duplicable and owning instantiations; the sibling leak) ✅ (completed: 2026-10-08 13:32)

**Deliverables**:
- [ ] ST-6 – ST-8
- [ ] Red-phase evidence

**Verify**: `make check` (the new suite fails as expected)

### Step 2.2: Implementation

**Reference**: [03-02 §Proposed Changes](03-02-deferred-consume-preparation.md) · [AR-4](00-ambiguity-register.md)
**Objective**: Carry the consume preparation and emit it at the consuming point.

- [x] 2.2.1 Defer the consume preparation: return the deferred value with its fill and move-out, and extend the fill emitter for a fixed-array (row) object — `compiler/src/own/lower.rs`, `compiler/src/codegen/guards.rs` ✅ (completed: 2026-10-08 16:17)
- [x] 2.2.2 Prepare member roots, empty the consume temporary at the indexed-store adoption, and select the store form for a literal whose field carries a deferred consume or a deferred base fill of an element read (duplicable or owning) — `compiler/src/own/lower.rs`, `compiler/src/codegen/function.rs` ✅ (completed: 2026-10-08 16:17)
- [x] 2.2.3 Run the new suite and record the green phase ✅ (completed: 2026-10-08 16:17)

**Deliverables**:
- [ ] Deferred consume preparation with source-order fills
- [ ] ST-6 – ST-8 green

**Verify**: `make check`

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02 §Testing Requirements](03-02-deferred-consume-preparation.md)
**Objective**: Pin the emitted fill/move-out shapes and the edge cases.

- [x] 2.3.1 Write implementation tests for the literal-field and indexed-assignment fill/move-out shapes, the consumer-side emptying at the indexed store, the store-form selection for a deferred consume and a deferred base fill, the pinned index on the move-out place, and byte-identical re-emission — `compiler/tests/slice_temporary_integrity_impl_test.rs` ✅ (completed: 2026-10-08 16:35)
- [x] 2.3.2 Run the host-ASan probes and edge cases (row element, nested index chain, guarded operand, indexed-assignment RHS, the loop-form indexed assignment, the duplicable-element order variant) ✅ (completed: 2026-10-08 16:35)
- [~] 2.3.3 Dispatch the phase review — correctness plus the memory-safety and emission-integrity auditors, and the semantics reviewer with the `spec/04-expressions.md` §3 evaluation-order lens; blast-radius (every `consume_place` call site, every fill-emitter arm) and test-integrity steps; resolve findings; one bounded re-review ⏳ (implemented: 2026-10-08 16:40)

**Phase 2 review findings (2026-10-08 16:40)** — four reviewers dispatched in parallel on the
phase diff (baseline `c425d3e1`): correctness, memory-safety, emission-integrity, and semantics
(the `spec/04-expressions.md` §3 evaluation-order lens). Corpus emission was byte-identical and
the memory pass found no leak/double-free apart from the order defect below; two phase-introduced
critical findings, resolved by user ruling (apply both fixes):

| ID | Severity | Finding | Status |
| -- | -------- | ------- | ------ |
| RV-001 + SA-001 + PE-001 + SR-002 | 🔴 CRITICAL | The new fixed-array fill arm delegated to `emit_fixed_array_assign_to`, which re-emits the inner guards the fill had already emitted, so a side-effecting source ran twice (`Box<String[2]>` field: head exit 202 vs base 102; also member rows, indexed-store rows, and array-literal rows). AddressSanitizer-invisible; wrong values. | ✅ fixed — the fill copies the already-evaluated object's bytes directly (`array_source_address`), with no second guard emission |
| SR-001 | 🔴 CRITICAL | The deferred carrier was not emptied by a projection/compound-literal consumer (`(Box { v: mk().a }).v` in a loop), so the next fill's reassigned drop freed what the destination still owned (`free()`: double free; the base leaked instead). | ✅ fixed — the carrier is marked non-releasing (an `adopting` flag on the deferred materialization) and skips the reassigned-temp drop, matching AR-4's "non-releasing temporary" |
| RV-003 + PE-003 | 🟡 MINOR | The migrated closeout pins passed on the eager shape, so they did not discriminate the deferred preparation. | ✅ fixed — both pins assert the deferred clear-before-call order, which the eager shape fails (verified against `55a9b99`) |
| RV-002 | 🟡 MINOR | `has_deferred_fill` does not recurse through `Binary`/`Cast`, so a wrapped element read (`make()[i] + 1`) keeps the reversed order; base-identical. | report-only (pre-existing) |
| PE-002 | 🟡 MINOR | A `?:`-arm literal with a deferred element read now uses the store form (source order improved); the packet's "guarded emission unchanged" wording covered member consumes only. | report-only (positive) |
| pre-existing | — | The guarded/conditional literal sibling leak, the `len(mk().a)` loop-condition double free, the Member-of-Index order gap, and the DEF-75 struct-element store are base-identical and out of scope. | reported separately |

**Fix evidence (2026-10-08 17:20)** — both fixes landed with regression coverage:

| Fix | Coverage | Result |
| --- | -------- | ------ |
| Fixed-array fill single evaluation | impl `should_call_the_source_once_when_a_fixed_array_element_is_consumed` and `…_fixed_array_member_is_consumed` (call-count pins) | the emitted C contains exactly one call; the counting sources answer 1 and 12; host ASan clean |
| Non-releasing carrier | spec `should_not_free_a_projected_literal_twice_when_the_loop_rebinds_it` (loop projection consumer) | exit 0, host ASan clean (the base leaks, the unfixed head double-frees) |
| Discriminating pins | the two closeout pins assert the clear-before-call order | on `55a9b99` the first `make(` precedes the clear, so the pins fail as required |
| all | full host-ASan matrix re-run (row element 4, nested 3, unguarded literal 2, indexed RHS 3, loop store 3, duplicable order 12, single-call and loop-projection regressions 1/0) | every shape exit-correct and ASan clean; the guarded literal keeps its base-identical pre-existing leak; `make check` green |

One bounded re-review is dispatched on the fix diff.

**Deliverables**:
- [ ] Implementation suite
- [ ] Clean probe matrix
- [ ] Phase review with no open critical/major finding

**Verify**: `make check`

---

## Phase 3: Root and receiver binding (DEF-71/89)

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: compiler-and-language; ownership/lifetime rules for owners and receivers
> **Reasoning**: high — release registration and conditional/receiver binding on ownership paths

### Step 3.1: Specification Tests

**Reference**: [03-03 §Proposed Changes](03-03-root-and-receiver-binding.md) · [07 ST-9 – ST-13](07-testing-strategy.md) · [AR-5](00-ambiguity-register.md)
**Objective**: Pin the missing releases and the adoption forms before implementation.

- [ ] 3.1.1 [spec-author] Write specification tests ST-9 – ST-13 in `compiler/tests/slice_temporary_integrity_spec_test.rs`
- [ ] 3.1.2 Run the new suite and record the red phase under the host-ASan command named in 07 (sibling-field leaks; the member-receiver buffer leak; the eager-temp loop double free)

**Deliverables**:
- [ ] ST-9 – ST-13
- [ ] Red-phase evidence

**Verify**: `make check` (the new suite fails as expected)

### Step 3.2: Implementation

**Reference**: [03-03 §Proposed Changes](03-03-root-and-receiver-binding.md) · [AR-5](00-ambiguity-register.md)
**Objective**: Extend the binds to the missing roots and receivers.

- [ ] 3.2.1 Bind the eager temporary through the destination-temp materializer and emit the emptying adoption for registered destination temporaries; bind a conditional root as one value (guarded by the place-arm predicate) — `compiler/src/own/lower.rs`, `compiler/src/own/statements.rs`
- [ ] 3.2.2 Bind owned member receivers at the read positions only (cast/intrinsic/argument/return reads), excluding consume preparation, guarded positions, and loop-condition lowering — `compiler/src/own/lower.rs`
- [ ] 3.2.3 Run the new suite and record the green phase

**Deliverables**:
- [ ] Destination-temp eager bind, conditional root bind, read-position member receiver bind
- [ ] ST-9 – ST-13 green

**Verify**: `make check`

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03 §Testing Requirements](03-03-root-and-receiver-binding.md)
**Objective**: Pin the new shapes and prove the single-owner invariant.

- [ ] 3.3.1 Write implementation tests for the destination-temp mark and release, the emptying adoption shape, the conditional branch-store shape, the read-position member-receiver binding and its excluded positions, the place-arm conditional staying inline, the loop-condition pin migration, and the existing `__cl_owned` absence pins — `compiler/tests/slice_temporary_integrity_impl_test.rs`
- [ ] 3.3.2 Run the host-ASan probes including the whole-result adoption, the loop form of the eager temporary (single release), the guarded literal, the loop-condition member receiver, and the `len(spawn())` regressions
- [ ] 3.3.3 Dispatch the phase review — correctness plus the memory-safety and emission-integrity auditors, and the semantics reviewer on the guarded receiver path's evaluation order; blast-radius (every root-bind and receiver-bind call site) and test-integrity steps; resolve findings; one bounded re-review

**Deliverables**:
- [ ] Implementation suite
- [ ] Clean probe matrix
- [ ] Phase review with no open critical/major finding

**Verify**: `make check`

---

## Phase 4: Element-store drop spelling (DEF-75) and plan hardening

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: compiler-and-language; emitted-C subset (`spec/17-codegen`)
> **Reasoning**: medium — one emission spelling plus the plan-wide acceptance sweep

### Step 4.1: Specification Test

**Reference**: [03-04 §Proposed Changes](03-04-element-store-drop-spelling.md) · [07 ST-12](07-testing-strategy.md) · [AR-8, AR-10](00-ambiguity-register.md)
**Objective**: Pin the strict-build acceptance before the fix.

- [ ] 4.1.1 [spec-author] Write specification test ST-12 in `compiler/tests/slice_temporary_integrity_spec_test.rs` (both filed programs, strict host build and run)
- [ ] 4.1.2 Run the test and record the red phase (pointer-arrow error under `-Werror`)

**Deliverables**:
- [ ] ST-12
- [ ] Red-phase evidence

**Verify**: `make check` (the new case fails as expected)

### Step 4.2: Implementation

**Reference**: [03-04 §Implementation Details](03-04-element-store-drop-spelling.md) · [AR-8, AR-10](00-ambiguity-register.md)
**Objective**: Parenthesize the drop-expression suffix bases only where grouping is needed.

- [ ] 4.2.1 Parenthesize a drop-expression suffix base only when it is not a primary/postfix expression (field and fixed-array index appends; statement and expression forms) — `compiler/src/codegen/drop.rs`
- [ ] 4.2.2 Run the new suite and record the green phase

**Deliverables**:
- [ ] Valid emitted drop targets; unchanged handle spelling
- [ ] ST-12 green

**Verify**: `make check`

### Step 4.3: Implementation Tests & Plan Hardening

**Reference**: [01 §Acceptance Criteria](01-requirements.md)
**Objective**: Pin the spelling and run the plan-wide acceptance.

- [ ] 4.3.1 Write implementation tests for the `(*(&(out[i]))).a` shape, the untouched handle spelling, the byte-identical existing spellings for identifier/member/index bases, the expression-form grouping, and byte-identical re-emission for programs without the shape — `compiler/tests/slice_temporary_integrity_impl_test.rs`
- [ ] 4.3.2 Plan-wide hardening: the base-vs-head emission sweep over the corpus in every profile with the strict unit builds (corpus emission byte-identical; retyped multi-extent test-suite pins expected, every diff traced to its requirement), the memory-safety sweep across all changed shapes, and the full `make verify`
- [ ] 4.3.3 Final plan review and close-out: correctness plus the memory-safety and emission-integrity auditors (and the semantics reviewer over the plan diff's evaluation-order surfaces) over the plan diff; update the AGENTS.md test-naming map and sync the roadmap (per the roadmap skill); record the final evidence

**Deliverables**:
- [ ] Implementation suite
- [ ] Emission and memory sweeps clean
- [ ] `make verify: all legs passed`
- [ ] Docs and roadmap updated

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (row-typed views)
    ↓
Phase 2 (deferred consume preparation) ── row moves through literals use the row-typed element
    ↓
Phase 3 (root and receiver binding)
    ↓
Phase 4 (drop spelling + plan hardening)
```

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per phase; `make verify` at completion)
3. ✅ No new warnings or errors; all emitted units strict-clean
4. ✅ No dead code — the removed recovery gate and unwrapping helper leave no unused path
5. ✅ Security — no input, network, or persistence surface touched; not applicable beyond the
   compiler's own strict builds
6. ✅ Documentation updated (AGENTS.md test map, roadmap)
7. ✅ Code reviewed per the repository's quality profile with no open critical/major finding
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
