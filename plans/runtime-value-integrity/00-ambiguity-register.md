# Ambiguity Register: Runtime Value Integrity

> **Status**: ✅ GATE PASSED — all 18 items resolved
> **Last Updated**: 2026-09-29 14:29

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | Which defects this plan implements | DEF-23 – DEF-29 / include PLAN-13 / include PLAN-05 + PLAN-13 | DEF-23 – DEF-29 (filed defects only) | ✅ Resolved |
| 2 | Scope | How the work is structured | one cumulative plan / program of three focused plans / one plan per defect | One cumulative plan; one preflight and one execution thread | ✅ Resolved |
| 3 | Technical | Phase order | proposed cluster order (materialization → callable fields → runtime conversions/deferral) / strict severity order / conversions first | Proposed cluster order: Phase 1 DEF-24 + DEF-23 + DEF-25, Phase 2 DEF-26 + DEF-27, Phase 3 DEF-28 + DEF-29 | ✅ Resolved |
| 4 | Naming & terminology | Plan folder name | runtime-value-integrity / residual-defect-sweep / aggregate-call-integrity | `runtime-value-integrity` | ✅ Resolved |
| 5 | Behavioral | DEF-23: which temporaries get statement-end release | (A) the spec rule for every owning compiler temporary — scalar and array — on the evaluated path only / (B) release array temporaries only, keeping the scalar leak | User accepted recommendation: Option A — every owning temporary, statement end, reverse creation order, evaluated paths only (spec/17-codegen.md:80; D5/O2) | ✅ Resolved |
| 6 | Behavioral | DEF-24: `for (x in mk())` semantics | (A) accept and materialize the call result into a compiler temporary released at loop end / (B) reject call-rooted iterables with a diagnostic | User accepted recommendation: Option A — accept and materialize once; spec/05 §9 defines the loop over an indexable value | ✅ Resolved |
| 7 | Behavioral | DEF-25: which conditional fixed-array values are supported | (A) duplicable and owned arrays, lowered as a branch that evaluates only the chosen arm with move semantics for owned arms / (B) duplicable arrays only; diagnose an owned conditional | User accepted recommendation: Option A — both duplicable and owned; only the chosen arm evaluates, owned arms move | ✅ Resolved |
| 8 | Behavioral | DEF-26: receiver guard scope | (A) guard a runtime-index receiver exactly where arguments are guarded on all three call paths (expression statement, fixed-array assign, fixed-array return) / (B) reject runtime-index field receivers | User accepted recommendation: Option A — guard on all three paths | ✅ Resolved |
| 9 | Behavioral | DEF-27: func-pointer ABI scope | (A) fix every `func(...)` spelling with a fixed-array return (fields, locals, parameters, discarded calls) / (B) fix struct fields only | User accepted recommendation: Option A — every func value spelling plus call destinations | ✅ Resolved |
| 10 | Technical | DEF-28: panic message conversion | (A) runtime takes `(message pointer, length)` across all three shims; no NUL assumption / (B) NUL-terminate: `String` via its layout, `str` via a copied buffer | User accepted recommendation: Option A — runtime takes pointer + length | ✅ Resolved |
| 11 | Technical | DEF-29: deferred `take` move-out | (A) extend the guard-operand materialization to carry the move-out on the evaluated path / (B) restructure short-circuit lowering into explicit branch statements | User accepted recommendation: Option A — extend the materialization value | ✅ Resolved |
| 12 | Scope | The frozen specification oracle | keep `spec/`, the grammar, and the diagnostics registry unchanged / authorize a specification change | User accepted recommendation: keep the frozen oracle; no spec, grammar, or registry change | ✅ Resolved |
| 13 | Security & compliance | Security surface of the change | compiler-internal only / a new input, secret, or data path | User accepted recommendation: compiler-internal — no user input, secrets, persistent data, or network surface | ✅ Resolved |
| 14 | Integration points | Verification commands | `make check` per task, `make verify` at close (project standard) / other | User accepted recommendation: `make check` per task, `make verify` at close | ✅ Resolved |
| 15 | Naming & terminology | Component documents and test suites | `03-01` materialization, `03-02` callable fields, `03-03` runtime boundary; `compiler/tests/runtime_value_integrity_spec_test.rs` and `_impl_test.rs` / other names | User accepted recommendation: the three component documents and the two suite names | ✅ Resolved |
| 16 | Scope | DEF-25: which expression positions the conditional fix covers | (A) every position that stores a fixed-array conditional value (initializer, assignment, return, argument, materialized read) through the shared value-assignment path / (B) store forms only, other positions recorded as a limitation | User accepted recommendation: Option A — all value positions through the shared path | ✅ Resolved |
| 17 | Behavioral | DEF-28: which panic message types are in scope | (A) `str`, `String`, and string literals; other types keep today's behavior / (B) also restrict other types with a new diagnostic | User accepted recommendation: Option A — the three string forms; no new diagnostic | ✅ Resolved |
| 18 | Technical | DEF-23: how the statement-end release is emitted, which temporaries it covers, and what happens to a view derived from a released temporary | (A) a dedicated statement-position release over the destination materializations only, plus statement-scoped temporary loan provenance so a use of a derived view after its statement reports `E10030` / (B) bind owning temporaries at function scope and record the D5/O2 timing deviation | User accepted recommendation: Option A — dedicated statement-position release, destination-only release set, temporary loan provenance with the existing `E10030` rule (gate packet approved in `00-preflight-report.md`) | ✅ Resolved |

### Resolution Notes

**AR-1:** The seven filed defects are one ownership/codegen program; PLAN-05/PLAN-13 are separate programs and PLAN-06/PLAN-07 are deferred by roadmap decision.

**AR-2:** One cumulative plan matches the `array-value-integrity` precedent (four defects, one plan, phased). This plan follows the same shape with three feature phases plus close-out.

**AR-3:** Grouping by shared fix surface avoids reworking the same code twice; the critical DEF-24 leads its cluster because its materialization needs the temporary-release design (DEF-23).

**AR-4:** The name states the surface: runtime values (materialized temporaries, callable values, runtime messages) and their integrity.

**AR-5:** `spec/17-codegen.md:80` and `spec/decisions.md:28` (D5, O2) require each temporary to be dropped at statement end in reverse creation order (`spec/decisions.md:28` states the statement-end point; the reverse order is stated at `spec/17-codegen.md:80`, `spec/17-codegen.md:212-213`, and `spec/decisions.md:369` (PF-040)). An unevaluated guard path leaves its temporary in the emitted empty state, so its release is a no-op, matching how a moved-out binding is released. A view derived from such a temporary is a loan of that temporary, so a use after its statement is the existing `E10030` loan-lifetime error (AR-18).

**AR-6:** `spec/05-statements.md` §9 defines the element loop over an indexable value with no place requirement, so accepted call-rooted iterables must compile; the loop materializes the call result once before its first iteration. A call-rooted iterable whose type mentions a generic parameter follows the existing `mentions_type_param` guard of the guard-operand materializer: it keeps today's inline behavior and is recorded as a limitation rather than declaring a temporary the uninstantiated body cannot spell.

**AR-7:** `spec/05` §9 allows a copy loop only for duplicable elements (E10040 for non-duplicable without `borrow`). For an owned conditional array, evaluating only the chosen arm and moving it keeps the unchosen source untouched, which is the semantics the store path needs.

**AR-8:** Bounds traps are the established runtime guard (`E11211`); `compiler/src/codegen/guards.rs:54-59` already guards a receiver in the expression-guard path, so the three statement/assignment/return paths are a consistency fix, not a new rule.

**AR-9:** `compiler/src/codegen/types.rs:58-73` renders every `func` pointer without the hidden out-param ABI, so fields and ordinary func values fail the same way. Fixing the declarator and the call destination detection once covers the filed spellings (fields, locals, parameters, and discarded calls). A closure struct's `fn` field and a lifted lambda's own signature are built inline with `c_type` (`codegen/program.rs:267-283`, `codegen/function.rs:198-220`) and are out of this plan's scope; the accepted-but-invalid form is filed as `DEF-30`.

**AR-10:** `spec/17-codegen.md:344` pins the panic behavior ("print then exit/abort"), not the C signature of the runtime helper, so aligning all three shims on pointer + length is an implementation change inside the frozen specification.

**AR-11:** The eager part is the `Statement::MoveOut` emitted at lowering time (`compiler/src/own/lower.rs:95-107`); carrying it on the materialized guard value and emitting it on the guarded path preserves short-circuit semantics without restructuring lowering.

**AR-12:** No `spec/`, grammar, or diagnostics-registry change; the diagnostics `E10026`, `E11211`, and the existing rows stay canonical.

**AR-13:** The changes alter checker and emitter decisions only; no input parsing, storage, or network path.

**AR-14:** From `AGENTS.md`, as the preceding plans used.

**AR-15:** The component documents follow the defect clusters; the suites follow the repo's `<feature>_spec_test.rs` / `_impl_test.rs` naming.

**AR-16:** Fixed-array values are stored through the shared assignment path in the accepted positions that route through it (initializer, plain assignment, and a read that needs a place is materialized through the same path first), the return position is handled by the fixed-array return emitter with the same branch shape, and a conditional argument whose arms are place-rooted stays inline as a forwarding value (the place-projection predicate is extended so no owning copy is bound). An indexed element target (`rows[i] = ok ? a : b`) and a nested-literal row store are separate accepted forms that emit invalid C today and are filed as `DEF-31`; a conditional argument with a call arm is filed as `DEF-32`.

**AR-17:** The filed defect is the `str`/`String` panic message; restricting other argument types would introduce a diagnostic the frozen registry does not own.

**AR-18:** The release is emitted by a dedicated statement-position path because `Statement::Drop` is a scope cleanup in the emitter (`codegen/cleanup.rs:39-52`) and would run at scope end, not statement end; the pending list drains after each statement and before every terminator. Only the destination materializations join the list (the guard `ValueKind::Materialize` temporaries, the iterable temporary, and the conditional read-position temporary); the take-call result and aggregate-literal-adoption temporaries are transfer temps whose value the destination already owns and are excluded, so the release cannot double-free. A view derived from a released temporary carries a loan of that temporary; the existing liveness rule reports `E10030` when a use has no release-free path. The added surface (temporary ownership in view provenance plus the release path) was approved through the gate packet in `00-preflight-report.md` (PF-002).
