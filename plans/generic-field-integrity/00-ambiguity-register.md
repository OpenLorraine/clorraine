# Ambiguity Register: Generic Field Integrity

> **Status**: ✅ GATE PASSED — all 23 items resolved
> **Last Updated**: 2026-09-26 02:45

## Scope

This plan fixes the pre-existing defects **DEF-06…DEF-10** recorded in
`plans/00-roadmap.md`. All share one root cause: a generic struct instance's
type arguments are not substituted into its field types in every compiler
stage. The plan implements reported defects only; it adds no new language
feature.

## Register

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| AR-1 | Scope | Which of DEF-06…DEF-10 the plan covers | (a) all five; (b) majors only; (c) soundness+leak only | (a) all five DEF-06–DEF-10 | ✅ Resolved |
| AR-2 | Behavioral | How DEF-09 treats an empty struct literal after discovering `spec/07-structs.md:44` STR-7 defines `Point {}` as the explicit zero value | (a) codegen-only, keep empty literals valid (STR-7); (b) also forbid empty literals (spec edit); (c) defer DEF-09 | (a) codegen-only; keep `Point {}` valid for every struct; fix the zero-value initializer to substitute instance arguments | ✅ Resolved |
| AR-3 | Technical | Whether to consolidate the duplicated "zip params with args, then substitute" block | (a) one small shared helper; (b) per-site fixes only | (a) one small shared helper | ✅ Resolved |
| AR-4 | Testing | Test depth and whether the frozen `spec/` may change | (a) impl/regression tests only, no `spec/` or `*_spec_test.rs` edits; (b) also spec goldens; (c) also a runtime drop test | (a) implementation and specification-derived regression tests placed in the existing suites (`sema::regressions`, `types::table` unit tests, `v1_language_impl_test.rs`), written before the fix; no `spec/` directory edits | ✅ Resolved |
| AR-5 | Technical | Verification commands for the plan's Verify lines | (a) `make check` per task, final `make verify`; (b) `make check` only | (a) `make check` per task; final `make verify` | ✅ Resolved |
| AR-6 | Naming | Name and placement of the shared substitution helper | (a) free fn `types::substitute_instance` + `TypeTable::instance_field`; (b) free fn only; (c) TypeTable method only | (a) free fn `substitute_instance(declared, params, args)` plus `TypeTable::instance_field(base, field) -> Option<Ty>` | ✅ Resolved |
| AR-7 | Technical | `TypeTable::offset_of` takes a struct name, so it cannot see an instance's arguments | (a) change signature to `offset_of(&Ty, field)`; (b) add a separate `offset_of_instance` | (a) change to `offset_of(&Ty, field)`; update the `consts/mod.rs` caller and the unit tests | ✅ Resolved |
| AR-8 | Naming | Plan folder slug | (a) `generic-field-integrity`; (b) `generic-field-substitution`; (c) `generic-struct-fields` | (a) `plans/generic-field-integrity/` | ✅ Resolved |
| AR-9 | Behavioral | DEF-06 required behavior for an overlay reached through a generic instance | Substitute instance arguments in the overlay walk so the generic form matches the non-generic form | `func takes(o: Outer<Regs>)` with `Outer<T>{inner:T}` and a `volatile`-bearing `Regs` is `E10207`, exactly like non-generic `Outer` (grounded: `sema/mod.rs:315` ignores args; non-generic probe errors, generic probe is accepted) | ✅ Resolved |
| AR-10 | Behavioral | DEF-07 required behavior for a generic struct holding a handle | Substituting makes `Box<String>` move-only so a second use is the moved-value error | `let a = Box<String>{…}; let b = a; use(a)` reports `E10020` (`spec/14-diagnostics.md:51`); grounded: `types/table.rs:360` ignores args | ✅ Resolved |
| AR-11 | Behavioral | DEF-08 required behavior for drop of a generic struct's handle field | The field's handle is released on scope exit | The emitted C for a `Box<String>` local contains `FreeString(b.value)` (grounded: `codegen/drop.rs:44,72` read declared fields without substitution) | ✅ Resolved |
| AR-12 | Behavioral | DEF-09 required behavior for a nested generic zero value | A valid `Pair<Pair<i32>> {}` must emit C that compiles | The initializer substitutes the instance argument, emitting nested zero states (`{ {0}, {0} }`) instead of `{ 0, 0 }`; grounded: `codegen/expr.rs` `struct_literal` default-state path uses only `self.substitute` | ✅ Resolved |
| AR-13 | Behavioral | DEF-10 scope: generic `sizeof`/`offsetof`, ownership field type, raw member type | Substitute in `size_of`/`offset_of`, the ownership field read, and route raw member access through the same substitution | `const N = sizeof(Box<i32>)` folds to `4` (now `E10224`); `offsetof(Box<i32>, value)` folds to `0`; `r.second` for `r: raw Pair<u8>` is typed `u8` (now silently `Unknown`); grounded at `types/table.rs:399,412`, `own/expr.rs:204`, `sema/expr.rs` `field_type` | ✅ Resolved |
| AR-14 | Edge cases | Behavior for nested instances, arrays, `raw` wrappers, and a generic function whose parameter name collides with the struct's | The helper substitutes by name within the instance's own parameter list; the enclosing function's substitution is applied on top in the emitter; existing depth limits are preserved | Design: `substitute_instance` zips the declaration's own parameter names with the instance's arguments; callers that already substitute the enclosing scope keep doing so (codegen applies `self.substitute` after). Depth limits of 32 already bound `duplicable_at`/`size_of_at`/`overlay_reached` | ✅ Resolved |
| AR-15 | Edge cases | A generic instance used with no or too few arguments | Unreachable on a valid program: the checker reports `E10300`/`E10301` first; the helper leaves an unbound parameter unchanged | `substitute_instance` returns the declared type unchanged when `params` or `args` is empty and `substitute` leaves an unbound parameter in place (no panic, no corruption) | ✅ Resolved |
| AR-16 | Integration points | Interfaces affected by the new helper and the `offset_of` signature change | Internal only; no public CLI or artifact contract changes | The helper lives in `compiler/src/types/`; only compiler-internal callers in `sema`, `own`, `codegen`, and `consts` are touched | ✅ Resolved |
| AR-17 | Security & compliance | User-input, injection, auth, or data-protection surface | N/A | The change is compiler type-checking and emission; there is no external input, network, auth, or persisted-data surface | ✅ Resolved |
| AR-18 | Non-functional | Performance, scalability, or availability impact | N/A | The helper performs the same per-field substitution work the stages already do; compiler run time and memory are unchanged in order | ✅ Resolved |
| AR-19 | UX & presentation | Diagnostics wording for the affected errors | Reuse the existing registry messages | `E10020`, `E10207`, and `E10224` keep their `spec/14-diagnostics.md` text; no new diagnostic is introduced | ✅ Resolved |
| AR-20 | Data & state | Data model, ownership, or format change | N/A | No data model or persisted format is involved | ✅ Resolved |
| AR-21 | Stakeholder conflicts | Competing priorities or permission boundaries | N/A | Single maintainer, no conflicting stakeholders | ✅ Resolved |
| AR-22 | Technical (complexity escalation) | Is the shared helper a material support surface requiring the escalation gate? | (a) small helper, no escalation; (b) full escalation packet | (a) A single pure function plus one `TypeTable` convenience used by existing call sites is the "small helper" the shared gate exempts; no new layer, dependency, harness, or infrastructure | ✅ Resolved |
| AR-23 (runtime) | Technical (scope) | Task 4.1.1 found two pre-existing inline `params.zip(args)` → `substitute` blocks outside the plan's declared file scope (`own/generics.rs`, `codegen/program.rs`), while R7 and Success Criterion 4 say the duplicated blocks are gone | (a) consolidate both onto `substitute_instance`; (b) leave as pre-existing debt; (c) consolidate only `own/generics.rs` | (a) consolidate both — user ruling 2026-09-26 02:45; the two files join the Phase 4 modification set | ✅ Resolved |

## Resolution Notes

**AR-2:** The first question to the user assumed `Point {}` contradicted STR-6. The
surface-during-authoring check found `spec/07-structs.md:44` STR-7 explicitly makes
`Point {}` the zero value, so the checker's allowance is correct. The user re-ruled
DEF-09 as codegen-only.

**AR-4:** This is a deliberate, user-approved deviation from the default placement of
specification tests. The essential ordering (tests written and verified red before the
fix) is preserved; the tests are specification-derived regressions in the existing
Rust suites, not new `spec/` goldens.

**AR-9..AR-13:** Each was reproduced against the frozen spec before being recorded, and
each expectation traces to an existing `spec/` rule or the diagnostics registry, so no
new behavior is invented.

**AR-14:** Name collision is handled by substituting per instance: for `func f<T>(p:
Pair<T>)`, the instance argument is `Param("T")`, so the field stays `T` and the
emitter's enclosing substitution resolves it to the concrete type.

**AR-23 (runtime):** The Phase 4 audit confirmed the plan's touched modules were
clean, but a broader search found two pre-existing copies of the same substitution
pattern that the plan's file lists had excluded. Because R7 ("exactly one shared
place") and Success Criterion 4 ("the duplicated zip-substitute blocks are gone")
are Must-Have outcomes, the user ruled to consolidate both onto
`substitute_instance`. Both edits are behavior-equivalent refactors (no output
change); the existing suites verify them.
