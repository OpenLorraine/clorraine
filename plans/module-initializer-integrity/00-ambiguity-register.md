# Ambiguity Register: Module Initializer Integrity

> **Status**: ✅ GATE PASSED — all 16 items resolved (user decisions + auto-design provenance)
> **Last Updated**: 2026-10-11 02:10
> **Batch**: F of the 37-defect mandate (DEF-130 – DEF-136); user-confirmed split E → F → G
> **Auto-design**: active — root invocation ID `module-initializer-integrity-2026-10-11`,
> policy version 1; reserved decisions (spec text, new user-visible behavior, acceptance
> criteria) remain user-owned and none is open.

| # | Category | Ambiguity / Gap | Options Presented | Resolution | Status |
|---|----------|-----------------|-------------------|------------|--------|
| 1 | Scope | Which defects batch F closes | The 9-row module-initializer cluster / a different partition of the 37 defects | **User (standing):** the E → F → G split; F = DEF-130 – DEF-135, DEF-137, DEF-143, DEF-152; D stays parked until no defects are left | ✅ Resolved |
| 2 | Technical | Architecture: the module-initializer pipeline is a second lowerer (`codegen/program.rs` `lower_global` ~600 lines) that duplicates the body path and owns no IR, no releases, and no own-stage checks. Full unification (synthesize init functions in the own stage) would fix the class by construction but re-emits every global program | A) unify into own-stage synthetic init functions / B) extend the existing codegen init path with focused own-stage walks | **AI — delegated by --auto-design (PD-1):** B — the smallest viable change, matching the eval-emission-integrity precedent; A changes every global program's emission and is a cross-cutting refactor beyond the mandate (recorded in the index as excluded machinery) | ✅ Resolved |
| 3 | Behavioral | DEF-131 (lambda half): a lambda-literal callee in an initializer has no lifted function and the spec restricts initializers in no way that excludes closures (`spec/10-modules.md` §6) | Lift initializer lambdas (a focused own-stage pass reusing `lift_lambda`) / reject lambda initializers with a new code | **AI — delegated by --auto-design (PD-2):** lift; the frozen surface allows closures, a rejection would need a new code and a user ruling, and capture entries in an initializer can only name module names (already `E10406`), so init lambdas are capture-less | ✅ Resolved |
| 4 | Behavioral | DEF-152: accepted programs write through `const` globals in initializers (`i32(G++)`) and emit invalid C (`S[0]++`) | Run the existing write-mutability checks over initializer expressions / leave the shapes accepted | **AI — delegated by --auto-design (PD-3):** run the checks — conformance to the frozen mutability rules (`spec/03-ownership.md`, the function-body forms report today); no existing test pins the accepted shapes (verified by search) | ✅ Resolved |
| 5 | Behavioral | DEF-132: how generic instances of initializer calls are recorded | Walk initializer expressions in the existing AST-level `generics::collect` and record calls / a separate instance pass | **AI — delegated by --auto-design (PD-2):** extend the existing walk; the instance machinery already keys everything from the recorded call list | ✅ Resolved |
| 6 | Behavioral | DEF-133: how the init path passes a dot-call receiver | Mirror the body's `lower_dot_receiver`/`func_field_receiver` semantics (receiver as argument zero or the func-field receiver) / fold the receiver into the arguments | **AI — delegated by --auto-design (PD-2):** mirror the body path exactly | ✅ Resolved |
| 7 | Behavioral | DEF-134: the init struct-literal emission form for fixed-array fields | The body's statement-form store with `memcpy` (shape-gated by the existing `struct_literal_needs_stores` predicate) / keep compound literals and special-case arrays | **AI — delegated by --auto-design (PD-2):** mirror the body path, shape-gated | ✅ Resolved |
| 8 | Behavioral | DEF-135: `str`-destination wrapping of an owned `String`/concat in an initializer | Mirror the body's `store_text` wrapping (`cl_string_view`) at the init store / change the global's C type | **AI — delegated by --auto-design (PD-2):** mirror the body's wrap; the global type is frozen | ✅ Resolved |
| 9 | Behavioral | DEF-143: the init `.len` on a qualified fixed-array field yields 0 because the receiver type is the head's | Fold by walking the projection chain (mirror `qualified_structural_len`) / lower through the intrinsic renderer | **AI — delegated by --auto-design (PD-2):** fold structurally like the body path; the intrinsic renderer's `len` on a struct is what yields `0` | ✅ Resolved |
| 10 | Behavioral | DEF-130: which initializer temporaries release, and when | Init locals whose type owns release at the init function's end in reverse creation order, null-guarded / reject owned initializer temporaries | **AI — delegated by --auto-design (PD-3):** release at the function end; the global itself stays duplicable (`E11106`, `spec/10-modules.md` MOD-25), so only intermediates release; rejecting would narrow the accepted surface without a spec basis | ✅ Resolved |
| 11 | Behavioral | DEF-137: the body `sizeof(i32[])` spelling misses its slice typedef | Collect written slice type arguments from body IR intrinsic values (mirroring the init-side collector arm) / restrict written type args to initializers | **AI — delegated by --auto-design (PD-2):** collect in the body path too; both spellings define the same type | ✅ Resolved |
| 12 | Error handling | Diagnostic codes for the fixes | Reuse the existing codes only / add new codes | **AI — delegated by --auto-design:** existing codes only (`E10023` and the global mutability family for DEF-152; no code for the emission/release fixes); a new code would be a reserved registry change | ✅ Resolved |
| 13 | Non-functional | Byte-identity: every fix changes emissions only for its defect shape | Shape-gated changes preserving byte-identical re-emission of unaffected initializers / blanket re-pinning | **AI — delegated by --auto-design:** shape-gated; the existing `eval_emission_integrity`/`store_emission_integrity` init pins stay green untouched, and the emission auditor re-diffs the corpus per phase | ✅ Resolved |
| 14 | Non-functional | Verify commands for the plan | `make check` + `make verify` + scoped suites | **User (standing):** confirmed in prior batches' registers; unchanged | ✅ Resolved |
| 15 | Process | Phase review dispatch | AGENTS.md matrix by change surface | **Convention (project policy):** codegen/emission and symbol changes → correctness + emission-integrity; ownership/release → + memory-safety; language-visible checks → + semantics; recorded per phase in the execution plan | ✅ Resolved |
| 16 | Technical | Complexity: does any fix add a layer, dependency, harness, or cross-cutting refactor? | None — fixes extend the existing init lowerer, the AST generics walk, the body-equivalent predicates, and reuse `lift_lambda`; a larger support surface would be the Design A unification | **AI — delegated by --auto-design:** None; no complexity escalation, no challenger packet; Design A stays excluded (AR-2) | ✅ Resolved |

### Resolution Notes

**AR-1 (user, standing):** the batch split was confirmed when the mandate began; F closes the
9 module-initializer defects. The loop continues to batch G and then the parked batches.

**AR-2 (delegated):** Design A (own-stage init functions) was second-guessed against
`codegen/program.rs:756` (`emit_init_functions`), `:1560` (`lower_global`), and the existing
byte-identity pins (`eval_emission_integrity_impl_test.rs` init group). It would change the
emitted C of every program with a runtime-initialized global — a cross-cutting refactor whose
cost is not justified by 9 conformance fixes; it is recorded as excluded machinery in the index.

**AR-4 (delegated):** the frozen mutability rules (`spec/03-ownership.md` §const, the
`E10023`/`E10042` registry rows) make a write through a `const` global an error; the own stage
simply never visits initializer expressions (`own.rs:107-138` walks `Item::Func` only). No
existing test asserts the accepted shapes (search over `compiler/tests/` found no
`i32(G++)`/`S[0]++` pin), so running the checks flips no pinned assertion.

### Delegated Decision Provenance

#### PD-1 — Init-path extension over unification

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanism (where the fixes land); stays inside the user-selected
  mandate (fix the 9 filed defects); changes no accepted program's semantics beyond the defects
Objective: close DEF-130 – DEF-135, 137, 143, 152 with the smallest viable change
Decision: extend the codegen init path (call resolution, aggregate stores, view wrap, len fold,
  release registry) and add two focused own-stage passes (initializer lambda lifting + generic
  instance collection; a check-only write walk); no pipeline unification
Evidence: codegen/program.rs:756-824 (init function emission, no cleanup), :1560-1691
  (lower_global duplicate), :1700-1826 (init intrinsics); own.rs:107-138 (functions only);
  generics.rs:36-45 (AST walk); the eval-emission-integrity precedent (03-01 §Proposed Changes)
  patched the same path for DEF-117/121/122
Rejected alternatives: (a) own-stage synthetic init functions — changes every global program's
  emission, a cross-cutting refactor (Complexity Escalation Gate would block without the user);
  (b) codegen-only fixes for DEF-131 lambdas and DEF-152 checks — not possible without checker
  machinery
Strongest counterargument: extension keeps two parallel lowerers and the drift that produced
  these 9 defects; a future batch may still unify
Confidence: High — every fix has a body-path contract to mirror and existing pins to protect
Hardening: the emission-integrity auditor re-diffs the corpus per phase; byte-identity pins
  gate the unaffected shapes; the finding was re-derived by reading the actual lowerer
```

#### PD-2 — Mirror the body path for calls, aggregates, and typedefs

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanism; each fix copies a verified body-path contract into the
  init path (or the body collector); no new language behavior
Objective: make an initializer reach the same C shapes the body path already emits
Decision: lambda lift via a synthetic minimal checker context reused by the gen walk; generic
  instances via the AST walk; receiver semantics from `lower_dot_receiver`/`func_field_receiver`;
  struct stores gated by `struct_literal_needs_stores`; `str` wrap from `store_text`; len fold
  from `qualified_structural_len`; slice typedefs from the body's IR intrinsic values
Evidence: own/call.rs:20-74 (`is_dot_call`, `lower_dot_receiver`), :112-155
  (`func_field_receiver`), :288-337 (`structural_len_extent`); own/place.rs:997-1024
  (`qualified_structural_len`); codegen/function.rs:1489-1545 (`struct_literal_needs_stores`,
  `emit_struct_literal_store`), :1937-1941 (`store_text` view wrap); codegen/program.rs:247-258,
  :374-386 (init slice collector), :268-275 (body collector gap); own/lower.rs:372-409, :458-592
  (`lower_lambda`, `lift_lambda`)
Rejected alternatives: per-shape special cases diverging from the body contracts (the exact
  drift these defects are)
Strongest counterargument: mirroring without sharing keeps two code paths in sync by discipline
Confidence: High — the body contracts are pinned by existing suites
Hardening: each mirrored shape gets a spec case asserting the emitted fragment and a run result
```

#### PD-3 — Release registry and the check-only write walk

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanism for the release and check defects; the behavior
  (release intermediates; reject const writes) follows the frozen rules
Objective: DEF-130 releases every owning init temporary once; DEF-152 reports the existing
  mutability codes over initializer expressions
Decision: an init-owner registry in the emitter (`Vec<(String, Ty)>` beside `init_locals`)
  emitting null-guarded `emit_drop_expr` calls in reverse creation order before the init
  function's closing brace; a dedicated check-only walker over initializer expressions that
  reuses the extracted pure predicates (`E10023` global-const writes, `E10042` immutable-view
  writes) and the global place resolution, without a function context
Evidence: codegen/program.rs:756-770 (no cleanup today), :991-1056
  (`materialize_init_local`, the initiation point), :1085-1400 (`materialize_init_value`);
  own/expr.rs:907-1020 (`check_update`, `check_update_value_write`), own/place.rs:230-242
  (`check_global_immutable_view_write`); own/expr.rs:865-899 (`lend_moves_judged` global
  precedent)
Rejected alternatives: (a) a synthetic `FunctionChecker` doing the checks + lift — its write
  methods emit statements, so a check-only use would need emission suppression and shared-state
  care; (b) rejects for the DEF-130 shapes — narrows the accepted surface without basis
Strongest counterargument: the check-only walker duplicates a few predicates; the extraction
  keeps one definition per rule
Confidence: Medium-high — the registry is contained; the walker must cover the write forms the
  initializer grammar admits (update, assign, indexed/member writes)
Hardening: the semantics reviewer probes each write form in both spellings (init and body) for
  one-report parity; the memory auditor checks the release once-per-owner across conditional
  fills
```
