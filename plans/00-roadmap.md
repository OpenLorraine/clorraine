# Roadmap: Clorraine

> **Feature-Set**: Clorraine
> **Status**: In Progress
> **Created**: 2026-09-17
> **Last Updated**: 2026-09-25 23:20
> **Progress**: n/a (plan-based; no RDs drafted yet)
> **CodeOps Artifact Schema**: 1

## Legend

⬜ Backlog · ✏️ RD Drafted · 🔎 RD Preflighted · 📋 Plan Created · 🔬 Plan Preflighted · 🔄 Executing · ✅ Done · ⛔ Blocked · ⏸️ Deferred

## Tracker

| ID | Title | RD | Plan | Stage | Status | Last Updated | Depends-on / Blocker |
|----|-------|----|------|-------|--------|--------------|----------------------|
| PLAN-01 | Clorraine compiler MVP | — | [clorraine-compiler-mvp](clorraine-compiler-mvp/00-index.md) | Done | ✅ | 2026-09-21 | — |
| PLAN-02 | Clorraine build/run driver | — | [clorraine-driver](clorraine-driver/00-index.md) | Done | ✅ | 2026-09-23 | PLAN-01 |
| PLAN-03 | Freestanding (LorraineOS) profile | — | [clorraine-freestanding-profile](clorraine-freestanding-profile/00-index.md) | Done | ✅ | 2026-09-24 | depends on PLAN-01 |
| PLAN-04 | Clorraine v1 language completion | — | [clorraine-v1-language-completion](clorraine-v1-language-completion/00-index.md) | Done | ✅ | 2026-09-25 | depends on PLAN-01 |
| PLAN-05 | Developer tooling and install path | — | — | Backlog | ⬜ | 2026-09-23 | depends on PLAN-04 |
| PLAN-06 | LorraineOS bring-up (downstream) | — | — | Backlog | ⬜ | 2026-09-23 | depends on PLAN-03; separate repository |
| PLAN-07 | ARM64 port | — | — | Backlog | ⬜ | 2026-09-23 | depends on PLAN-06 |
| PLAN-08 | Amiga graphics (screen + drawing) | — | [clorraine-graphics](clorraine-graphics/00-index.md) | Done | ✅ | 2026-09-24 | depends on PLAN-01 |
| PLAN-09 | Clorraine demo video toolkit | — | [clorraine-demo-video](clorraine-demo-video/00-index.md) | Done | ✅ | 2026-09-25 | depends on PLAN-01 |
| PLAN-10 | Generic field integrity (DEF-06–DEF-10) | — | [generic-field-integrity](generic-field-integrity/00-index.md) | ⛔ Blocked (was: Executing) | ⛔ | 2026-09-25 | waits on DEF-11; Phase 4 pending |
| ↳ DEF-11 | Self-referential struct-literal reassignment double-frees | — | [partial-move-drop-coherence](partial-move-drop-coherence/00-index.md) | Executing | 🔄 | 2026-09-25 | fix before PLAN-10 Phase 4 |

## Backlog / Known defects

| ID | Title | Detected | Status | Notes |
|----|-------|----------|--------|-------|
| DEF-01 | Compiler accepts a bare `return;` in a non-void function | 2026-09-23 | Fixed 2026-09-23 | Reports `E10200` (`spec/05-statements.md:191`); regression in `sema::regressions::a_bare_return_is_allowed_only_in_a_void_function`. |
| DEF-02 | `for (let i = 0; i < n; i++)` drops the loop step | 2026-09-23 | Fixed 2026-09-23 | The step block is no longer dropped: `i++`/`++i` lower to a step assignment emitted in the C `for` header; regression in `codegen_run_spec_test::a_c_style_for_runs_its_step`. |
| DEF-03 | Compound assignment stores the right-hand side (`x += 2` → `x = 2`) | 2026-09-23 | Fixed 2026-09-23 | All compound operators and `++`/`--` now read-modify-write; an impure element index is evaluated once. Regressions in `codegen_run_spec_test::compound_assignment_combines_with_the_current_value`, `::a_compound_element_store_evaluates_its_index_once`, and `::a_postfix_increment_yields_the_old_value`; the increment operand place rule is pinned by `sema::regressions::an_increment_operand_must_be_a_place`. |
| DEF-04 | A discarded expression drops an embedded side effect (`f() + 1;`, `j++ + 1;`) | 2026-09-23 | Fixed 2026-09-23 | A discarded value expression now lowers to a new `Discard` statement, so an embedded call or update runs while `&&`/`||`/`?:` keep their short-circuit guard. The same purity rule now treats `++`/`--` as effectful, so an increment used as a guarded index (`a[i++]`) runs once. Regressions in `codegen_run_spec_test::a_discarded_expression_still_runs_its_side_effects`, `::a_discarded_short_circuit_keeps_its_guard`, and `::an_increment_index_is_evaluated_once`. |
| DEF-05 | Callable-bound generic rejects a concrete function when `T` is inferred from a generic-struct field | 2026-09-25 | Fixed 2026-09-25 | A field read of an instantiated generic struct leaked the raw type parameter `T` instead of substituting the instance's arguments, so `apply<T, F: func(T): T>(p.first, addOne)` with `p: Pair<i32>` reported `E10303` and `takesI32(p.first)` with `p: Pair<u8>` was silently accepted. Field access, the dot-call callable-field lookup, the struct-literal field check, and the emitter's field-type tracking now substitute the instance arguments; regressions in `sema::regressions::a_generic_struct_field_yields_its_substituted_type`, `v1_language_impl_test::a_generic_field_feeds_a_callable_bound`, and `::a_callable_field_of_a_generic_struct_checks_its_substituted_signature`. |
| DEF-06 | Generic struct instance bypasses the by-value overlay ban | 2026-09-25 | Open | `overlay_reached` (`compiler/src/sema/mod.rs:315`) walks a struct's declared field types without substituting instance arguments, so `Outer<T> { inner: T }` used as `Outer<CustomRegs>` is accepted while the non-generic form is `E10207`. Found while reviewing the DEF-05 fix; pre-existing. |
| DEF-07 | Generic struct duplicability ignores its type arguments | 2026-09-25 | Open | `TypeTable::duplicable_at` (`compiler/src/types/table.rs:360`) does not substitute, so `Box<T> { value: T }` used as `Box<String>` is treated as duplicable and its handle is copied without `E10020`. Ownership soundness hole; pre-existing. |
| DEF-08 | Drop not emitted for a generic struct holding a handle | 2026-09-25 | Open | `codegen/drop.rs` reads declared field types without substituting, so `Box<String>` never emits the `String` drop and leaks. Pre-existing. |
| DEF-09 | Nested generic zero value emits C that fails `-Werror` | 2026-09-25 | Open | `codegen/expr.rs::struct_literal` fills an omitted field from the enclosing substitution only, so `Pair<Pair<i32>> {}` (a valid zero value, `spec/07-structs.md:44` STR-7) emits `(ClPair__Pair__i32){ 0, 0 }` and the driver reports a failed C compile. Pre-existing. |
| DEF-10 | Remaining unsubstituted generic-field reads | 2026-09-25 | Open | The ownership `member_type` (`compiler/src/own/expr.rs`), raw-pointer member access (typed `Unknown`), and `size_of_at`/`offset_of` (`compiler/src/types/table.rs`) still read declared field types without the instance substitution. Minor; pre-existing. |
| DEF-11 | Self-referential struct-literal reassignment double-frees | 2026-09-25 | Open | `b = Box<String> { value: b.value };` compiles but emits a double free: `emit_assign` (`compiler/src/codegen/function.rs:494`) treats the RHS field read of the target as a borrow, saves the old value, then drops the saved handle that the new value also owns. The non-generic form (`struct Label { name: String }`) already double-frees at baseline; Phase 3's substitution-aware `emits_drop` newly makes it reachable through a generic struct. Root cause is partial-move/drop coherence across the ownership stage and the emitter. Found by the Phase 3 review (RV-001). |
