# Ambiguity Register: Generic Substitution Guards

> **Status**: ✅ GATE PASSED — all 19 items resolved
> **Last Updated**: 2026-09-26 21:40

## Scope

Fix **DEF-12**: both declaration-site guards are blind to a type substituted for
a type parameter, so two illegal programs compile.

- **View fields (`E10205`)**: `struct Wrap<T> { v: T; }` used as `Wrap<str>` or
  `Wrap<u8[]>` is accepted; the frozen rule bans a view-typed field
  (`spec/02-types.md:184` TY-10). The guard reads the declared type, where `T`
  hides the concrete kind (`compiler/src/sema/items.rs:39`).
- **Fill literals (`E10215`)**: `func fillIt<T>(a: T): T[2] { return [; a]; }`
  called as `fillIt<String>(s)` emits two aliases of one handle and double-frees
  (AddressSanitizer-confirmed). The guard skips a type-parameter element
  (`compiler/src/sema/expr.rs:706-708`); the frozen rule needs a duplicable fill
  value (`spec/08-arrays-strings.md:50` AL-1).

The concrete instantiation set already exists after lowering
(`Monomorphization`, `compiler/src/ir/mono.rs:106`); the fix audits it. No
`spec/` change is needed: the frozen rules already forbid both programs.

## Register

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| AR-1 | Scope | Plan routing, slug, and mapping to the recorded defect | (a) full plan `plans/generic-substitution-guards/`, `Implements: DEF-12`; (b) lightweight task mini-plan | (a) full plan; implementation-only fix (no `spec/`, no `stdlib/`) | ✅ Resolved |
| AR-2 | Behavioral | Fill `[; a]` whose element is a bare type parameter `T` | (a) audit the concrete instance; reject `fillIt<String>`, keep `fillIt<i32>` and inferred calls; (b) reject at the declaration (also `fillIt<i32>`) | (a) audit at instantiation | ✅ Resolved |
| AR-3 | UX | Diagnostic text and spans for the instantiation-time errors | (a) reuse the registry text/help; field declaration span for view fields, enclosing statement for fills; (b) instantiation-aware message (spec/14 text grant); (c) exact literal/use-site spans | (a) reuse registry text | ✅ Resolved |
| AR-4 | Behavioral | View-field predicate depth | (a) mirror the existing check exactly (`str` or slice field type); (b) deep ban on aggregates of views (`str[2]`) | (a) mirror exactly | ✅ Resolved |
| AR-5 | Scope | Two adjacent pre-existing gaps: transitive generic calls emit an undefined C symbol; a struct named only in `sizeof(Wrap<str>)` is not emitted | (a) keep both out of scope and record them; (b) fix the `sizeof` instance discovery too; (c) also add transitive instance collection | (a) keep both out; record as observations | ✅ Resolved |
| AR-6 | Technical | Where the check runs and lives | (a) new focused module `compiler/src/own/instance_check.rs`, called from `own::check` after the monomorphization is collected; (b) extend `own/generics.rs` (674 lines) | (a) new module | ✅ Resolved |
| AR-7 | Behavioral | Which types the audit examines, to avoid double-reporting | Only a declared field/element type that **mentions a type parameter** is audited; a fully concrete illegal case stays sema-owned, so one site reports once. Recommended: yes | User accepted recommendation: audit only parameter-mentioning types, reusing `mentions_type_param` | ✅ Resolved |
| AR-8 | Behavioral | Error granularity and ordering across instances | One `E10205` per illegal instance-field and one `E10215` per illegal instance, deduplicated by instance identity, emitted in monomorphization order (deterministic). Recommended: yes | User accepted recommendation: one error per illegal instance, deterministic order | ✅ Resolved |
| AR-9 | Behavioral | Coverage of the instance set | Explicit and inferred type arguments; discarded results; locals, parameters, returns, globals, temporaries; nested generic struct instances reached through the field fixpoint; prefix+fill forms; an uninstantiated generic is not checked and emits nothing. Recommended: yes | User accepted recommendation: full recorded instance set; uninstantiated generics unchecked | ✅ Resolved |
| AR-10 | Technical (complexity escalation) | DEF-12 records that a sound fix "needs an instantiation-time check, which is new machinery" | (a) no escalation: a focused audit over the already-collected `Monomorphization`, no new layer, dependency, harness, generalized abstraction, or framework; (b) generalized per-instance rule replay | User accepted recommendation: (a) no escalation — the audit reuses `Monomorphization`, `mentions_type_param`, and `TypeTable::duplicable`; it is two specific guards, not a rule engine | ✅ Resolved |
| AR-11 | Behavioral | Constructs that could carry `T` into a fill outside `Item::Func` bodies | Lambdas are capture-less or inline-only and cannot name `T`; `comptime` generics are `E90001`; so the audit over function instances is complete. Recommended: yes | User accepted recommendation: function-instance audit is complete | ✅ Resolved |
| AR-12 | Compatibility | Spec, stdlib, corpus, and artifact contracts | No `spec/` edit (the rules already ban both programs); no stdlib or corpus change; no CLI/ABI change; valid programs emit byte-identical C. Recommended: yes | User accepted recommendation: implementation and tests only | ✅ Resolved |
| AR-13 | Non-functional | Performance and determinism of the audit | Bounded by instances × body size, run once per compile; no performance target change; deterministic instance order keeps diagnostics stable. Recommended: yes | User accepted recommendation: bounded, deterministic audit | ✅ Resolved |
| AR-14 | Security & compliance | Input/injection/data-protection surface | N/A: compiler-internal check; it closes a double-free (and the view-field escape) before code generation. Recommended: yes | User accepted recommendation: N/A, security improves | ✅ Resolved |
| AR-15 | Testing | Oracles, placement, and ordering | Spec diagnostic cases in `compiler/tests/ownership_drop_spec_test.rs`; plain build-and-run controls (no ASan, per the DEF-11 flakiness finding) in `compiler/tests/ownership_drop_impl_test.rs`; unit tests in the new module; spec tests red before implementation. Recommended: yes | User accepted recommendation: spec diagnostics + build-and-run controls + module unit tests, red-first | ✅ Resolved |
| AR-16 | Technical | Verify commands | `make check` per task; `make verify` before completion | User: `make check` per task; `make verify` before completion | ✅ Resolved |
| AR-17 | Behavioral | Do the declaration-site guards change? | No: the sema guards stay as they are (concrete cases, `E10215` extent/prefix checks); the audit adds only the parameterized cases, so no diagnostic is duplicated. Recommended: yes | User accepted recommendation: sema guards unchanged | ✅ Resolved |
| AR-18 | Naming | Module and helper names | `compiler/src/own/instance_check.rs`; `check_view_fields`, `check_fill_instances`, `check` entry; a shared `struct_declarations` map helper in `generics.rs`. Recommended: yes | User accepted recommendation: the stated names | ✅ Resolved |
| AR-19 | Scope | Three further pre-existing instance-discovery gaps found during preflight: a discarded statement-level call's result type is not recorded; a nested generic struct reached only from an IR-discovered outer instance is not field-expanded; a fill inside a lifted lambda body under a generic instance is not monomorphized | (a) keep all three out of scope and record them; (b) extend instance discovery | User accepted recommendation: (a) keep out — recorded as out-of-scope via the preflight report (PF-002, PF-003) | ✅ Resolved |

## Pending confirmation

_(none — the gate passed)_

## Bulk acceptance

The user accepted the recommendations for AR-7, AR-8, AR-9, AR-11, AR-12,
AR-13, AR-14, AR-15, AR-17, and AR-18 in one explicit decision. Each row records
the exact option that was accepted; none was decided by silence or inference.

## Resolution Notes

**AR-1:** Reconnaissance reproduced both defects on the current compiler; the
plan mirrors the local convention for a defect fix (`partial-move-drop-coherence`
implemented `DEF-11` as a full plan).

**AR-2:** The spec requires "a duplicable fill value" (`AL-1`). A bare `T` is
unproven, `String` is provably not duplicable, and `i32` is. The audit reports
the provably illegal instantiation and keeps the duplicable one accepted.

**AR-4:** The audit mirrors `compiler/src/sema/items.rs:39` exactly
(`matches!(ty.kind, TyKind::Str) || ty.is_slice()`), so it rejects
`Wrap<str>`/`Wrap<u8[]>` at any nullability while `static str`, `cstring`,
`raw`, `func`, handles, and fixed arrays stay accepted, exactly as the
declaration-site check does today.

**AR-10:** Registered because the recorded defect calls the fix "new machinery".
No Complexity Escalation Gate stop is needed: the check is a focused read of the
already-collected `Monomorphization` set, reusing `mentions_type_param`
(`compiler/src/own/lower.rs:698`) and `TypeTable::duplicable`
(`compiler/src/types/table.rs:362`); it adds no layer, dependency, harness,
generalized abstraction, or infrastructure.

**AR-3 (PF-005 clarification):** for `E10215` the audit uses the declaration-site
fill help `a fill cannot repeat a move-only element; construct each element`
(`compiler/src/sema/expr.rs:713`), not the combined help on the
`spec/14-diagnostics.md:99` registry row.

**AR-19:** Surfaced by preflight iteration 1 (PF-002, PF-003) while auditing
R4/R5. Each case already emits C that the host C compiler rejects (an undeclared
`ClWrap__str`, or an unsubstituted `ClT`), so no silent double-free is left
open; the plan's two guards are unaffected. The audit keeps reading only the
recorded set. Fixing discovery/monomorphization is a separate change. AR-19
supersedes AR-9's coverage wording: discarded results and IR-only nested
instances are not covered.

> **Traceability:** Every design decision, scope decision, and error-handling
> strategy in the plan documents back-references the AR entry that resolved it.
