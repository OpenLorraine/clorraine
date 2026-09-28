# Roadmap: Clorraine

> **Feature-Set**: Clorraine
> **Status**: In Progress
> **Created**: 2026-09-17
> **Last Updated**: 2026-09-28 11:03
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
| PLAN-05 | Developer experience and delivery | — | — | Backlog | ⬜ | 2026-09-26 | after the language completion program; see Scope decisions |
| PLAN-06 | LorraineOS bring-up (downstream) | — | — | Deferred | ⏸️ | 2026-09-26 | external repository; the OS does not exist; Clorraine's freestanding ABI (PLAN-03) is done |
| PLAN-07 | ARM64 target port | — | — | Deferred | ⏸️ | 2026-09-26 | not planned; the v1 target is m68k/AmigaOS-compatible; aarch64 host builds stay supported |
| PLAN-08 | Amiga graphics (screen + drawing) | — | [clorraine-graphics](clorraine-graphics/00-index.md) | Done | ✅ | 2026-09-24 | depends on PLAN-01 |
| PLAN-09 | Clorraine demo video toolkit | — | [clorraine-demo-video](clorraine-demo-video/00-index.md) | Done | ✅ | 2026-09-25 | depends on PLAN-01 |
| PLAN-10 | Generic field integrity (DEF-06–DEF-10) | — | [generic-field-integrity](generic-field-integrity/00-index.md) | Done | ✅ | 2026-09-26 | DEF-06–DEF-10 fixed |
| ↳ DEF-11 | Self-referential struct-literal reassignment double-frees | — | [partial-move-drop-coherence](partial-move-drop-coherence/00-index.md) | Done | ✅ | 2026-09-26 | fixed by partial-move-drop-coherence |
| PLAN-11 | Ownership and drop model completion | — | [ownership-drop-completion](ownership-drop-completion/00-index.md) | Done | ✅ | 2026-09-26 | 48/48; every R1–R10 traces to a passing test; DEF-12 (generic-substitution blind spot) deferred |
| ↳ DEF-12 | Generic substitution guards | — | [generic-substitution-guards](generic-substitution-guards/00-index.md) | Done | ✅ | 2026-09-26 | 13/13; every R1–R8 traces to a passing ST case |
| PLAN-12 | Non-lexical lifetimes (NLL, v2) | — | [non-lexical-lifetimes](non-lexical-lifetimes/00-index.md) | Done | ✅ | 2026-09-27 | 4/4 phases done (22/22); strict review closed FR-001/FR-001b/FR-002, NEW-001, F1–F12; `make verify: all legs passed` |
| ↳ DEF-13 | `for (borrow e in xs)` double-frees a droppable element | — | [codegen-ownership-safety](codegen-ownership-safety/00-index.md) | Done | ✅ | 2026-09-28 | 🔴 critical, fixed by the element-alias design (AR #2); 20/20 tasks; ASan probes clean; `make verify: all legs passed` |
| ↳ DEF-14 | Local shadowing miscompiles (locals keyed by source name) | — | [codegen-ownership-safety](codegen-ownership-safety/00-index.md) | Done | ✅ | 2026-09-28 | 🔴 critical, fixed by unique binding identity (AR #3); 20/20 tasks; ASan probes clean; `make verify: all legs passed` |
| ↳ DEF-15 | Multi-extent element loops bind the base element, not the row | — | — | Backlog | ⬜ | 2026-09-27 | 🔴 critical, observed during codegen-ownership-safety recon; different root cause, needs a dedicated plan |
| ↳ DEF-16 | Reading an owned array element as a view argument double-frees | — | [element-loop-integrity](element-loop-integrity/00-index.md) | Done | ✅ | 2026-09-28 | 🔴 critical, fixed and verified: element reads and their projections lend instead of materializing (`projects_from_place`); ASan-clean, `make verify` all legs passed |
| ↳ DEF-17 | Nested element loops hang (step-block mis-selection) | — | [element-loop-integrity](element-loop-integrity/00-index.md) | Done | ✅ | 2026-09-28 | 🔴 critical, fixed and verified: the step block travels on `BranchShape::Loop` and the emitter prints it directly; every nested `for` form covered, `make verify` all legs passed |
| ↳ DEF-18 | A `take`/move through a borrow element view steals the element | — | — | Backlog | ⬜ | 2026-09-28 | 🟠 major, found in codegen-ownership-safety preflight: element bindings are `BindingKind::Local`, so the parameter move rules do not apply (`own/place.rs:185`) |
| ↳ DEF-19 | `len` on an owned `String` silently yields 0 | — | — | Backlog | ⬜ | 2026-09-28 | 🟠 major, found in codegen-ownership-safety Phase 2: the `len` lowering falls through to `0` for a `String` (`codegen/calls.rs:166`); the specification's implicit `String`→`str` widening should apply |
| ↳ DEF-20 | `if (x != null) { let x: T = x; }` spuriously reports E10010 | — | — | Backlog | ⬜ | 2026-09-28 | 🟡 minor, found in the codegen-ownership-safety Phase 2 re-review: `check_nonnull_expr` resolves the initializer place after the shadowing declaration (`own/expr.rs:251`); diagnostic-only and pre-existing |
| ↳ DEF-21 | A projection over an array-returning call emits invalid C | — | — | Backlog | ⬜ | 2026-09-28 | 🔴 critical, found in element-loop-integrity planning: `mk()[0]` emits `mk()[0]` on a hidden out-parameter call (`codegen/function.rs:74`); the base needs materializing before the projection |
| PLAN-13 | Amiga library surface | — | — | Backlog | ⬜ | 2026-09-26 | may not add language features; files a dependency on PLAN-11 |

## Scope decisions (v1.0)

The completion program targets a **v1.0 core**: a rule-complete language and a borrow checker at NLL precision. Everything else is deferred so the target stays bounded.

| Area | v1.0 (in) | Deferred |
| ---- | --------- | -------- |
| Borrow/ownership | Ownership-model features (PLAN-11), then **NLL** (PLAN-12) | Polonius (v3) — only if NLL proves insufficient |
| Language | Every non-staged rule and the standard-library surface the corpus needs | `comptime` (E90001), pools/arenas (E90004), `stores`/`transfers` (E90003), nested templates (E90005), user contract tables (W10005), C-hook trampolines (v1.1) |
| Targets | m68k/AmigaOS-compatible (`hosted-amiga`, `freestanding`) and the `host` test profile | ARM64 target (PLAN-07); 64-bit `usize`/`isize`; LorraineOS (PLAN-06) |
| Amiga surface | `std.amiga.*` growth, including `std.amiga.exec`, wider custom-chip, and intuition/graphics extras (PLAN-13) | — |
| Delivery (PLAN-05) | Amiga-developer-first DX; VSCode first (TextMate grammar, then LSP staged); `--explain` and machine-readable diagnostics; a fetch/detect SDK installer | Project manifest / package manager; watch mode, deep LSP, debugger |

**Delivery direction (PLAN-05):** the installer **fetches or detects-and-fetches** every component from its upstream source (the NDK from Aminet, the C toolchain from upstream or our GPL-compliant prebuilt) — it never redistributes the NDK or other restrictively licensed third-party binaries. The `host` profile remains a zero-friction on-ramp (a single binary plus any C compiler). A **licence decision** is a hard blocker for any public release.

**Host support:** `clc` builds and runs on x86_64 and aarch64 developer machines (host-only); this is kept and tested, independent of any ARM64 target.

## Backlog / Known defects

| ID | Title | Detected | Status | Notes |
|----|-------|----------|--------|-------|
| DEF-01 | Compiler accepts a bare `return;` in a non-void function | 2026-09-23 | Fixed 2026-09-23 | Reports `E10200` (`spec/05-statements.md:191`); regression in `sema::regressions::a_bare_return_is_allowed_only_in_a_void_function`. |
| DEF-02 | `for (let i = 0; i < n; i++)` drops the loop step | 2026-09-23 | Fixed 2026-09-23 | The step block is no longer dropped: `i++`/`++i` lower to a step assignment emitted in the C `for` header; regression in `codegen_run_spec_test::a_c_style_for_runs_its_step`. |
| DEF-03 | Compound assignment stores the right-hand side (`x += 2` → `x = 2`) | 2026-09-23 | Fixed 2026-09-23 | All compound operators and `++`/`--` now read-modify-write; an impure element index is evaluated once. Regressions in `codegen_run_spec_test::compound_assignment_combines_with_the_current_value`, `::a_compound_element_store_evaluates_its_index_once`, and `::a_postfix_increment_yields_the_old_value`; the increment operand place rule is pinned by `sema::regressions::an_increment_operand_must_be_a_place`. |
| DEF-04 | A discarded expression drops an embedded side effect (`f() + 1;`, `j++ + 1;`) | 2026-09-23 | Fixed 2026-09-23 | A discarded value expression now lowers to a new `Discard` statement, so an embedded call or update runs while `&&`/`||`/`?:` keep their short-circuit guard. The same purity rule now treats `++`/`--` as effectful, so an increment used as a guarded index (`a[i++]`) runs once. Regressions in `codegen_run_spec_test::a_discarded_expression_still_runs_its_side_effects`, `::a_discarded_short_circuit_keeps_its_guard`, and `::an_increment_index_is_evaluated_once`. |
| DEF-05 | Callable-bound generic rejects a concrete function when `T` is inferred from a generic-struct field | 2026-09-25 | Fixed 2026-09-25 | A field read of an instantiated generic struct leaked the raw type parameter `T` instead of substituting the instance's arguments, so `apply<T, F: func(T): T>(p.first, addOne)` with `p: Pair<i32>` reported `E10303` and `takesI32(p.first)` with `p: Pair<u8>` was silently accepted. Field access, the dot-call callable-field lookup, the struct-literal field check, and the emitter's field-type tracking now substitute the instance arguments; regressions in `sema::regressions::a_generic_struct_field_yields_its_substituted_type`, `v1_language_impl_test::a_generic_field_feeds_a_callable_bound`, and `::a_callable_field_of_a_generic_struct_checks_its_substituted_signature`. |
| DEF-06 | Generic struct instance bypasses the by-value overlay ban | 2026-09-25 | Fixed 2026-09-26 | `overlay_reached` (`compiler/src/sema/mod.rs`) walked a struct's declared field types without substituting instance arguments, so `Outer<T> { inner: T }` used as `Outer<CustomRegs>` was accepted while the non-generic form is `E10207`. It now substitutes via `substitute_instance`, so the generic form reports `E10207` too. Regression in `sema::regressions::an_overlay_reached_through_a_generic_field_is_rejected`. |
| DEF-07 | Generic struct duplicability ignores its type arguments | 2026-09-25 | Fixed 2026-09-26 | `TypeTable::duplicable_at` did not substitute, so `Box<T> { value: T }` used as `Box<String>` was treated as duplicable and its handle was copied without `E10020`. It now substitutes each field via `substitute_instance`, making `Box<String>` move-only. Regressions in `types::table::a_generic_struct_holding_a_handle_is_move_only`, `own::tests::a_generic_struct_holding_a_handle_is_move_only`, and `::a_generic_struct_holding_a_copy_value_is_duplicable`. |
| DEF-08 | Drop not emitted for a generic struct holding a handle | 2026-09-25 | Fixed 2026-09-26 | `codegen/drop.rs` read declared field types without substituting, so `Box<String>` never emitted the `String` drop and leaked. `emit_drop_expr` and `emits_drop` now substitute the instance arguments (then the enclosing generic context). Regression in `v1_language_impl_test::a_generic_struct_handle_field_is_dropped`. |
| DEF-09 | Nested generic zero value emits C that fails `-Werror` | 2026-09-25 | Fixed 2026-09-26 | `codegen/expr.rs::struct_literal` filled an omitted field from the enclosing substitution only, so `Pair<Pair<i32>> {}` (a valid zero value, `spec/07-structs.md:44` STR-7) emitted `(ClPair__Pair__i32){ 0, 0 }` and the driver reported a failed C compile. The omitted-field state now substitutes the instance argument via `field_type`/`instance_field`, emitting nested zero states. Regressions in `v1_language_impl_test::a_nested_generic_zero_value_emits_nested_zero_states`, `::a_plain_zero_value_stays_valid`, and `::a_non_generic_owning_struct_is_unchanged`. |
| DEF-10 | Remaining unsubstituted generic-field reads | 2026-09-25 | Fixed 2026-09-26 | The ownership `member_type` (`compiler/src/own/expr.rs`), the checker's raw-pointer member access (`sema/expr.rs`), and `size_of_at`/`offset_of` (`compiler/src/types/table.rs`) read declared field types without the instance substitution. All now route through the shared `substitute_instance`/`TypeTable::instance_field` (and `offset_of` takes the instance `Ty`). Regressions in `types::table::a_generic_struct_size_substitutes_its_type_argument`, `::a_generic_struct_field_offset_substitutes_its_type_argument`, and `sema::regressions::a_field_reached_through_a_raw_pointer_is_typed`, `::a_raw_base_peels_to_its_pointee_for_a_member`, `::a_nested_generic_field_is_its_substituted_instance`. |
| DEF-11 | Self-referential struct-literal reassignment double-frees | 2026-09-25 | Fixed 2026-09-26 | `b = Box<String> { value: b.value };` emitted a double free because an owned field nested in an aggregate literal was copied without zeroing the source, and the assignment save path then dropped the aliased handle. An owned literal field is now consumed as a move (copy plus zero source), so the source is emptied before the literal is built; the assignment fast path releases the empty old target. Fixed by `plans/partial-move-drop-coherence/` (ownership mark + `consume_place`, including the postfix `(b).value` spelling). Regressions: `partial_move_impl_test`, `stage_spec_test` (`a_moved_binding_is_emptied_when_consumed_by_a_literal`, `a_literal_reading_its_target_empties_the_field`, `consuming_a_literal_field_twice_reports_the_moved_field`, `consuming_a_nested_path_reports_the_partial_move_limit`, `consuming_a_borrowed_param_field_reports_the_borrow_move`), `own::tests`. |
| DEF-12 | Generic substitution bypasses the view-field and fill guards | 2026-09-26 | Fixed 2026-09-26 | `Wrap<str>` / `Wrap<u8[]>` fields and a generic fill `[; a]` instantiated with a move-only type are accepted; the fill double-frees (AddressSanitizer-confirmed during the PLAN-11 Phase 4 review). The guards run at the declaration site, so a substituted kind is invisible. Fixed by [generic-substitution-guards](generic-substitution-guards/00-index.md): the recorded instantiations are audited for the parameterized cases only. |
