# Ambiguity Register: Clorraine v1 language completion

> **Status**: ✅ GATE PASSED — 39 items: 32 resolved, 7 explicitly deferred
> **Last Updated**: 2026-09-24 21:04
> **CodeOps Artifact Schema**: 1

Every row below was raised during the Phase 1 interview and resolved by the
user, or explicitly deferred. The plan completes three v1 language surfaces
that the corpus-subset MVP staged out: **generics**, **lambdas (stages 1-4)**,
and **volatile intrinsics with the per-chip MMIO wrappers**. It edits the frozen
specification only where the user authorized it (AR-3).

> **Items added during execution** are tagged `(runtime)`. AR-40 and AR-41 were
> raised during Phase 1 execution and resolved by the user on 2026-09-25; they
> are implemented by tasks 1.2.10 and 1.2.11.

## Register

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| AR-1 | Scope | Planning form and tracking | Full feature plan + roadmap row / lightweight task mini-plan / plan folder only | Full feature plan at `plans/clorraine-v1-language-completion/`, using the existing `PLAN-04` roadmap row | ✅ Resolved |
| AR-2 | Scope | What PLAN-04 covers | Generics + lambdas + volatile/MMIO / that plus `embed` and `comptime` / generics only / lambdas only | Generics completeness, lambdas (stages 1-4), and volatile intrinsics + MMIO wrappers. `embed` and `comptime` are excluded | ✅ Resolved |
| AR-3 | Scope | Which frozen-spec artifacts may change | Authorize targeted edits with rulings / no spec edits | Targeted `spec/` and `spec/tests/` edits are authorized; each is recorded here, and `spec/check.sh` must stay green | ✅ Resolved |
| AR-4 | Scope | Two silent miscompiles exist (generic structs, volatile intrinsics) | Fix inside PLAN-04 / separate bugfix first / leave | Fix both inside PLAN-04 as first-class, spec-test-backed tasks | ✅ Resolved |
| AR-5 | Naming | Plan folder and document set | Varies | Plan `plans/clorraine-v1-language-completion/` with `03-01-generics`, `03-02-lambdas`, `03-03-volatile-mmio`, `03-04-spec-goldens` | ✅ Resolved |
| AR-6 | Naming | Test file names and placement | New suites / extend existing stage suites | New `compiler/tests/v1_language_spec_test.rs` (behavior) and `compiler/tests/v1_language_impl_test.rs` (internals); compile-fail goldens stay under `spec/tests/compile-fail/` | ✅ Resolved |
| AR-7 | Naming | MMIO stdlib module name | `std.amiga.custom` / `std.amiga.customchips` / other | `std.amiga.custom`, matching `spec/appendix-amiga.md` section 4 | ✅ Resolved |
| AR-8 | Naming | Generic-struct C symbol | `Cl<Name>__<typekey>` / other | `Cl<Name>__<typekey>`, reusing the existing deterministic function-instance key scheme (P11); the typedef uses the same name. The key is the canonical rendered type key; this plan adds no new hashing, and P11's over-long-key hash (Ch 17 §7) is a separate, pre-existing concern outside this plan | ✅ Resolved |
| AR-9 | Naming | Lifted-lambda symbol naming | `cl_lambda_<N>` / module-qualified | `cl_lambda_<N>` file-scope `static` funcs with a stable counter (P7, Ch 17 section 8) | ✅ Resolved |
| AR-10 | Behavioral | The L3/L4/L5 gap: how a capturing closure is passed/called | Callable generic bound / defining scope only / unbounded parameter | A capturing closure is accepted where the target is a generic callable-bound parameter `F: func(...)` and is monomorphized into the instance; it is E10400 only where a concrete `func(...)` value type is required. Supplying an explicit `func(...)` type argument for `F` makes the argument a concrete `func(...)` value, so a capturing closure there is E10400; the examples leave `F` to inference | ✅ Resolved |
| AR-11 | Behavioral | May a capturing closure be bound to a local? | Inline-only (matches golden) / local binding allowed | Inline-only. A capturing closure is immediately invoked or passed inline; binding it to a local, field, global, or array, returning it, or capturing it is E10404. Ch 06 section 6.1's `let bump = ...` example is corrected | ✅ Resolved |
| AR-12 | Behavioral | Lambda stage semantics in scope | Stages 1-4 / stage 1 only | Stage 1 capture-less (plain function pointer); stage 2 copy/borrow captures; stage 3 `[borrow x]`; stage 4 callable-bound generic integration | ✅ Resolved |
| AR-13 | Behavioral | Capture copy/borrow/write rules | Per spec L2 / variant | Duplicable captures copy; non-duplicable captures borrow read-only; writing a captured binding needs `[borrow x]` (E10402); moving a capture is E10401; `take` captures are E10403 | ✅ Resolved |
| AR-14 | Behavioral | Closure ephemerality diagnostics | Per spec L3/L6 / variant | A capturing closure cannot be returned, stored in a field/global/array, or captured by another closure; that is E10404 | ✅ Resolved |
| AR-15 | Behavioral | Callable bounds and callable-only values | Implement E10303/E10304 / leave inert | Implement both: a type argument that violates a `F: func(...)` bound is E10303; storing or returning a callable-only type-parameter value is E10304 | ✅ Resolved |
| AR-16 | Behavioral | Ephemeral type arguments (E10306) | Closure types only / broader set | Closure types are the only ephemeral type arguments (L3); a generic aggregate instantiated with one is E10306 | ✅ Resolved |
| AR-17 | Behavioral | Generic-struct monomorphization | One concrete struct per storable instantiation (deduped) / byte-blob | One concrete C struct per distinct storable instantiation, deduped by a canonical type key (G3, P11, Ch 07 section 9) | ✅ Resolved |
| AR-18 | Behavioral | Instantiation recursion cap (E10305) | Cover generic structs / functions only | The cap covers generic instantiation, including generic structs; the G5 size/count report is deferred (AR-33) | ✅ Resolved |
| AR-19 | Behavioral | Volatile intrinsic semantics | C `volatile` dereference / explicit fence | Emit a C `volatile`-qualified pointee access with no folding, caching, or reordering (T7) | ✅ Resolved |
| AR-20 | Behavioral | Volatile module-level `let` | Add the C qualifier / leave as-is | Emit C `volatile` on the module-level `static` global (T7, IH-3) | ✅ Resolved |
| AR-21 | Behavioral | Reconciling the staged codes after implementation | Retire E90002; keep E90000 for `Array<move-only>` / re-scope E90002 to v1.1 | Retire E90002 (reserved, no trigger, per N2); E90000 stays only for the non-duplicable `Array` element case and loses the lambda trigger | ✅ Resolved |
| AR-22 | Behavioral | `[take x]` capture routing | E10403 / keep E90002 | `[take x]` reports E10403, its real code, not E90002 | ✅ Resolved |
| AR-23 | Data model | Capturing-closure value representation | Two-word struct / heap environment | A two-word callable (function pointer + environment pointer); the environment lives on the enclosing frame; the lifted func takes it (P7, Ch 17 section 8) | ✅ Resolved |
| AR-24 | Data model | Generic-struct type representation | Reuse `TyKind::Struct(name, args)` / new kind | Reuse the existing `TyKind::Struct(name, args)`; no new type kind | ✅ Resolved |
| AR-25 | Error handling | Diagnostic set changes | Listed set / variants | Add triggers for E10303, E10304, E10306, and E10400-E10404; retire E90002; no new volatile code (E10604 already gates unsafe access) | ✅ Resolved |
| AR-26 | Verification | The verify command | `make check` then `make verify` / `make check` only | Every task runs `make check`; each phase and the final task run `make verify` | ✅ Resolved |
| AR-27 | Verification | Volatile test legs | Host + hosted-amiga / hosted-amiga only | Host test passes a real local address to prove read/write semantics; hosted-amiga builds the MMIO module and runs the safe wrapper under `vamos`/FS-UAE | ✅ Resolved |
| AR-28 | Verification | MMIO wrapper breadth | Minimal normative set / broad coverage / intrinsics only | `std.amiga.custom` with the appendix's register bases and wrappers (dmacon, cop1lc, color) plus tests; broader coverage is deferred (AR-39) | ✅ Resolved |
| AR-29 | Technical | Extending monomorphization | Extend `Monomorphization` / a parallel pass | Extend the existing `Monomorphization` to record struct instances alongside function instances; codegen consumes one set | ✅ Resolved |
| AR-30 | Technical | `type_key`/`c_type` for instantiated structs | Include the arguments / leave name-only | `type_key` includes a struct's type arguments; `c_type` emits the instantiated name; the name-only path remains for non-generic structs | ✅ Resolved |
| AR-31 | Technical | Phase ordering | Generics then lambdas then volatile / other | Generics first (lambdas stage 4 depends on the callable bound), then lambdas, then the independent volatile/MMIO phase | ✅ Resolved |
| AR-32 | Documentation | Spec chapters and goldens to edit | Listed set / variants | `spec/06-functions.md` (section 6.1, L-7, diagnostics), `spec/grammar.ebnf.md` section 8, `spec/14-diagnostics.md`, `spec/tests/compile-fail/` (un-skip and retire), `spec/tests/staged/README.md`, `spec/check.sh`, plus `AGENTS.md` and `plans/00-roadmap.md` | ✅ Resolved |
| AR-33 | Deferred | G5 opt-in instantiation size/count report | Include now / defer | Deferred to the developer-tooling plan. Not implemented by this plan | ⏸ Deferred |
| AR-34 | Deferred | Owning-element drop glue so `Array<move-only>` works | Include now / defer | Deferred; `Array<move-only element>` keeps the honest `E90000` staged error. Not implemented by this plan | ⏸ Deferred |
| AR-35 | Deferred | Data-inclusion `embed` semantics | Include now / defer | Deferred to a later plan. Not implemented by this plan | ⏸ Deferred |
| AR-36 | Deferred | `comptime` semantics | Include now / defer | Deferred (GF22 stages the semantics). Not implemented by this plan | ⏸ Deferred |
| AR-37 | Deferred | C-hook trampolines for capturing closures (stage 5) | Include now / defer | Deferred to v1.1 (L5). Not implemented by this plan | ⏸ Deferred |
| AR-38 | Deferred | ARM64 profile and 64-bit `usize`/`isize` | Include now / defer | Deferred to the future ARM64 port (A5a). Not implemented by this plan | ⏸ Deferred |
| AR-39 | Deferred | Broader MMIO coverage, `std.amiga.exec` message/signal APIs, and the library-build entry (MOD-29) | Include now / defer | Deferred to later plans. Not implemented by this plan | ⏸ Deferred |
| AR-40 (runtime) | Technical | A function instance with a non-scalar type argument names an invalid C symbol: the call site keys the instance with `type_key` (`func(...)` -> `fn`), but prototype/body emission uses `Instance.args.join("_")` from `Ty::display()` (`func(i32): i32`), and `type_from_key` cannot rebuild a function signature. ST-4 (`apply<i32, func(i32): i32>`) surfaces it | Store `Instance.args` as `Ty` and derive every instance symbol from `type_key` (recommended) / keep display strings and add a parallel `Vec<Ty>` for substitution / sanitize display strings into identifiers and store each signature separately | ✅ Resolved (user, 2026-09-25): store `Instance.args` as `Ty`; every instance symbol comes from `type_key`; `type_from_key` is removed (task 1.2.10) |
| AR-41 (runtime) | Behavioral | ST-3 imports `Pair` from another module, but the fixture does not `export` it; MOD-5/MOD-11 require export (E11102) while STR-21/G4 forbid exporting a generic struct (E10302). As written, ST-3 and R16's cross-module clause cannot both hold under the frozen spec | Amend STR-21/G4 so a generic struct may be exported, enabling the cross-module composition R16 requires (recommended) / rule the ST-3 fixture defective and remove its cross-module import / rule cross-module generic structs unsupported and drop R16's cross-module clause | ✅ Resolved (user, 2026-09-25): a generic struct may be exported; generic functions stay non-exportable (E10302). Authorizes spec edits to `spec/07-structs.md`, `spec/06-functions.md`, `spec/14-diagnostics.md`, `spec/grammar.ebnf.md`, `spec/decisions.md`, and `spec/17-codegen.md` (task 1.2.11) |
| AR-42 (runtime) | Behavioral | Task 1.2.8 (E10306) is sequenced in Phase 1, but its golden passes a capturing lambda to a generic aggregate; a capturing lambda reports E90002 before any closure type exists, and `TyKind::Closure` is added in Phase 2 (task 2.2.1). Phase 1 therefore cannot green E10306 without pulling closure typing forward | Move E10306 to Phase 2 and re-stage its golden until the closure type exists (recommended) / pull `TyKind::Closure` and minimal lambda typing into Phase 1 / special-case a capturing lambda in a generic-aggregate argument to report E10306 without a closure type | ✅ Resolved (user, 2026-09-25): move E10306 to Phase 2. Its golden carries `// stage: 2` and stays out of `ASSERTED_CODES` until task 2.2.10 |

## Resolution Notes

**AR-2:** The scope keeps the plan bounded to the three surfaces the corpus
subset staged but the ledger marks v1: generics (G1-G8), lambdas (L1-L5,
stages 1-4), and volatile access (T7). `embed` (Ch 13) and `comptime` (GF22)
are v1-debated and were excluded by the user to keep the plan reviewable.

**AR-10 / AR-11 (the L3/L4/L5 gap):** The ledger says closures are "called and
passed" (L3), only capture-less lambdas match `func(...)` (L5), and stage 4 is
`F: func(...)` generics (L4) — but it never says how a capturing closure is
passed. The user resolved both sides: a capturing closure may fill a generic
callable-bound parameter and be monomorphized, but it may not be bound to a name
at all. `E10404_closure_stored.clo` already pins the local-binding case as
E10404, and Ch 17 section 8 explains why (the environment is valid only for the
synchronous call); Ch 06 section 6.1's `let bump = ...` example is the artifact
that must be corrected.

**AR-21 / AR-32:** Implementing capture-less and capturing lambdas removes every
trigger for E90002 ("lambda captures are not yet implemented") and for E90000's
lambda use. Per N2 a removed code stays reserved, so E90002 is retired with no
trigger and its golden is removed; E90000 keeps the non-duplicable `Array`
element trigger (AR-34). The registry, Ch 06 L-7, `spec/tests/staged/README.md`,
and `spec/check.sh` are updated to match.

**AR-29 / AR-30:** `Monomorphization` (`compiler/src/ir/mono.rs`) records
function instances only, and `type_key`/`c_type`
(`compiler/src/codegen/types.rs`) drop a struct's type arguments, so a struct
that stores `T` emits an invalid `ClT` field with exit 0. The fix extends the
existing instance set and the type-key path rather than adding a parallel pass.

**AR-26:** The project's commands come from `AGENTS.md`: `make check`
(`spec/check.sh` plus `cargo test --workspace` in the SDK container) and
`make verify` (adds the hosted-amiga GCC/`vamos`, vbcc, and FS-UAE legs). No new
verify command is invented.

## Gate Record

- Systematic scan across all 12 categories completed.
- 39 rows: 32 resolved by an explicit user decision or an agreed improvement;
  7 explicitly deferred (AR-33-AR-39) and excluded from executable plan content.
- No item introduces a material support surface beyond ordinary compiler,
  stdlib, and spec-golden work. The broader mechanisms considered (a generic
  `Array<move-only>` drop-glue path, a C-hook trampoline, a heap closure
  environment) were deferred in favour of the smaller paths, so the Complexity
  Escalation Gate is not triggered.
- The user confirmed the complete register on 2026-09-24; the header reads
  `✅ GATE PASSED`.
