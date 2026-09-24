# Roadmap: Clorraine

> **Feature-Set**: Clorraine
> **Status**: In Progress
> **Created**: 2026-09-17
> **Last Updated**: 2026-09-24 14:37
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
| PLAN-04 | Clorraine v1 language completion | — | — | Backlog | ⬜ | 2026-09-23 | depends on PLAN-01 |
| PLAN-05 | Developer tooling and install path | — | — | Backlog | ⬜ | 2026-09-23 | depends on PLAN-04 |
| PLAN-06 | LorraineOS bring-up (downstream) | — | — | Backlog | ⬜ | 2026-09-23 | depends on PLAN-03; separate repository |
| PLAN-07 | ARM64 port | — | — | Backlog | ⬜ | 2026-09-23 | depends on PLAN-06 |
| PLAN-08 | Amiga graphics (screen + drawing) | — | [clorraine-graphics](clorraine-graphics/00-index.md) | Executing | 🔄 | 2026-09-24 | depends on PLAN-01 |

## Backlog / Known defects

| ID | Title | Detected | Status | Notes |
|----|-------|----------|--------|-------|
| DEF-01 | Compiler accepts a bare `return;` in a non-void function | 2026-09-23 | Fixed 2026-09-23 | Reports `E10200` (`spec/05-statements.md:191`); regression in `sema::regressions::a_bare_return_is_allowed_only_in_a_void_function`. |
| DEF-02 | `for (let i = 0; i < n; i++)` drops the loop step | 2026-09-23 | Fixed 2026-09-23 | The step block is no longer dropped: `i++`/`++i` lower to a step assignment emitted in the C `for` header; regression in `codegen_run_spec_test::a_c_style_for_runs_its_step`. |
| DEF-03 | Compound assignment stores the right-hand side (`x += 2` → `x = 2`) | 2026-09-23 | Fixed 2026-09-23 | All compound operators and `++`/`--` now read-modify-write; an impure element index is evaluated once. Regressions in `codegen_run_spec_test::compound_assignment_combines_with_the_current_value`, `::a_compound_element_store_evaluates_its_index_once`, and `::a_postfix_increment_yields_the_old_value`; the increment operand place rule is pinned by `sema::regressions::an_increment_operand_must_be_a_place`. |
| DEF-04 | A discarded expression drops an embedded side effect (`f() + 1;`, `j++ + 1;`) | 2026-09-23 | Fixed 2026-09-23 | A discarded value expression now lowers to a new `Discard` statement, so an embedded call or update runs while `&&`/`||`/`?:` keep their short-circuit guard. The same purity rule now treats `++`/`--` as effectful, so an increment used as a guarded index (`a[i++]`) runs once. Regressions in `codegen_run_spec_test::a_discarded_expression_still_runs_its_side_effects`, `::a_discarded_short_circuit_keeps_its_guard`, and `::an_increment_index_is_evaluated_once`. |
