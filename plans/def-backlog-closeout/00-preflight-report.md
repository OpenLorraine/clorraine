# Preflight Report: Def Backlog Closeout

> **Status**: ✅ PASSED — all 7 findings resolved (3 major, 4 minor)
> **Iteration**: 2 (fix verification + bounded re-scan)
> **Artifact**: implementation plan at `plans/def-backlog-closeout/` (scan-start ref `6bb9b40`)
> **Codebase Grounded**: 30+ source files examined, 41 mapped references verified, plus host
> `clc`/ASan probes and one independent adversarial challenger
> **Last Updated**: 2026-10-04 00:37

```
SAME-SESSION REVIEW: This artifact was created in the current session.
Same-agent bias risk is elevated. An independent challenger reviewed the
high-stakes batch (see Hardening); consider a fresh-session audit for the
next artifact revision.
```

## Codebase Context Summary

**Tech Stack:** Rust (std-only workspace, no crates), C99 emitted translation unit, Clorraine
`.clo` sources; pinned Docker SDK toolchain.
**Architecture:** staged compiler — lex/parse/load/resolve/sema/own/ir/codegen — with the
ownership stage building a typed CFG IR and the emitter rendering one C TU per program.
**Key Files Examined:** `compiler/src/own/lower.rs`, `own/call.rs`, `own/loops.rs`,
`own/statements.rs`, `own/expr.rs`, `own/flow.rs`, `own/liveness.rs`, `own.rs`, `ir.rs`,
`ir/place.rs`, `codegen/types.rs`, `codegen/calls.rs`, `codegen/expr.rs`, `codegen/function.rs`,
`codegen/drop.rs`, `codegen/guards.rs`, `codegen/cleanup.rs`, `sema/expr.rs`, `sema/mod.rs`,
`resolve/mod.rs`, `compiler/tests/*`, `spec/{14-diagnostics,16-borrow-rules,17-codegen}.md`,
`spec/check.sh`.

## Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 1 | 🟡 |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 1 | 🟡 |
| 4 | Completeness Gaps | 0 | — |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | (2, secondary) | 🟠 |
| 7 | Testability | 0 | — |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | (2, secondary) | 🟠 |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 1 | 🟡 |
| 13 | Codebase Alignment | 4 (primary) + 3 secondary | 🟠 |

## Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 3 | ✅ all resolved |
| MINOR | 4 | ✅ all resolved |
| OBSERVATION | 0 | — |

---

## PF-001: DEF-67 regression claim contradicts the required fix 🟠 MAJOR

**Dimension:** 13 Codebase Alignment (secondary: 3 Consistency)
**Location:** `03-02-expression-temporaries.md` (Integration Points), `07-testing-strategy.md`
ST-11, `99-execution-plan.md` task 2.1.2
**Codebase Evidence:** on `6bb9b40`, `return i32(len(stringOf("abc")))`,
`let n: i32 = i32(len(stringOf("abc")))`, and `i32(len(storeAt<String>(mkString, 1)))` all emit
an inline `cl_string_view(...)` with no binding and no release; each leaks 12 bytes under host
ASan (`cc -fsanitize=address`, `ASAN_OPTIONS=detect_leaks=1`). R5 requires these to be bound and
released exactly once. The literal ST-11 spelling `len(stringOf("abc"))` also fails to compile
(E10200, needs the explicit `i32(...)` cast).
**The Problem:** the plan's "unchanged emitted shape"/"byte-identical" expectation for these call
results contradicts the mandated R5 fix; an immutable-oracle spec test written to it would lock in
the leak. The claim survives only for the inner `storeAt` body shapes asserted by the prior impl
test.
**Options:** A (chosen) reword to behavior regression + inner-body shape pins, byte-identity pins
on genuinely unaffected programs, fix the cast; B keep as written (rejected: contradicts R5).
**Recommendation:** Option A — doc-only; probed evidence decisive.
**User Decision:** Resolved — user accepted recommendation (Option A), 2026-10-03.
**Iteration 2:** ST-11 rewritten to the local-place regression (`let n: i32 = i32(len(s));`); ST-8
now carries the `storeAt<String>(...)` call-result form; `03-02` distinguishes the unchanged
`storeAt` body from the changed call site; task 2.1.2 records the call-result leak.

## PF-002: DEF-65 call-site lowering keyed on the argument type; widened slots cannot write through 🟠 MAJOR

**Dimension:** 13 Codebase Alignment (secondary: 6 Feasibility, 9 Edge Cases)
**Location:** `03-03-borrow-outslot-abi.md` §2–§3, AR-3/AR-10, R4, ST-14, `99` task 3.2.2
**Codebase Evidence:** `compiler/src/codegen/calls.rs:165-182` decides `pointer` with
`is_pointer_param(&self.substitute(&arg.ty), …)` — the argument type, not the parameter type.
With the widened predicate (`codegen/types.rs:30-32`), an instrumented build produced: callee
`ClString *` but call passes `ClString` → SEGV; `borrow str?` with a `String` argument → C type
error. Both programs are accepted on `6bb9b40`. AR-10's materialized `cl_string_view` temporary
compiles but discards the callee's write, contradicting R4/ST-14; `spec/16-borrow-rules.md:102-107`
defines the out-slot as an in-out ownership slot.
**The Problem:** phase 3 as written would ship ill-typed C and could not pass its own ST-14;
`03-03`'s "four consumers follow the same predicate automatically" is false for the named-call
pointer decision.
**Options:** A (chosen) key the call-site decision on the substituted parameter type, require an
exact-typed place for out-slots, reject implicit widening (`E10200`), rewrite ST-14; B copy the
temporary back (new codegen mechanism, no spec basis); C narrow to `String?`/`File?` only (leaves
`str?` write-loss).
**Recommendation:** Option A. AR-10's mechanism is unworkable for out-slots; the finding reopens
that decision with new evidence.
**User Decision:** Resolved — user accepted recommendation (Option A), 2026-10-03.
**Iteration 2:** `03-03` §2–§4 rewritten (parameter-type keying + exact-place rule), Error
Handling and testing updated, header marks AR-10 superseded; R4 rewritten; `00-index`,
`02-current-state`, `07` ST-12/ST-14 and the probe matrix, and `99` tasks 3.2.1/3.2.2/3.3.1
updated.

## PF-003: DEF-66's fix targets the wrong mechanism — the real cause is the Unknown element type 🟠 MAJOR

**Dimension:** 13 Codebase Alignment (secondary: 6 Feasibility)
**Location:** `03-01-take-moveout-integrity.md` §3, `99` tasks 1.2.3/1.3.1, ST-5/6
**Codebase Evidence:** the filed generic store form still reproduces on `6bb9b40` (probe:
`heap-use-after-free`, exit 1). The slice base is already materialized (`own/lower.rs:38-54`), so
the reachable place roots at the settled view temp whose pointer already includes the slice start
(`codegen/calls.rs:278-299`) — the originally suspected offset problem does not arise. The direct
`slice(...)` element read carries no type, so the derived place is `Unknown`; `Statement::MoveOut`
is skipped when the place type does not need a drop (`codegen/function.rs:450-459`,
`codegen/drop.rs:261-267`), the temporary release frees the element, and the take form infers
`cl_consume__unknown` (independent challenger, instrumented build).
**The Problem:** the planned `bind_ownerless_projection_root`/`value_place` Intrinsic-root arms are
unreachable and would not repair either failure mode.
**Options:** A (chosen) recover the element type from the settled slice base (existing
`iterated_element`, `compiler/src/types/mod.rs:375`, already used at `own/lower.rs:626`); keep the
reproduction gate; add a non-zero-start case and a take-form build check; B keep the plan's
mechanism (rejected: unreachable); C defer DEF-66 (filed defect stays open).
**Recommendation:** Option A.
**User Decision:** Resolved — user accepted recommendation (Option A), 2026-10-03.
**Iteration 2:** `03-01` §3 and `02` Gap 3 rewritten around element-type recovery; ST-5 gains the
non-zero start, ST-6 the well-typed build; the risks table records the recovery-scope risk;
`99` tasks 1.2.3/1.3.1 renamed to the element-typing mechanism.

## PF-004: Invalid DEF-65 test recipe (E10034) 🟡 MINOR

**Dimension:** 13 Codebase Alignment
**Location:** `00-index` Quick Reference, R4, ST-12
**Codebase Evidence:** probe — `func fill(v: String, borrow slot: String?) { slot = v; }` fails
with `E10034` ("cannot release or store borrowed parameter 'v'"; `spec/16-borrow-rules.md` BR-5).
The prior exemption test assigned only `null`.
**The Problem:** the plan's usage example and ST-12 program were not valid as written.
**Options:** A (chosen) use `take v` (probe-verified compile); B have the callee assign a fresh
value.
**Recommendation:** Option A.
**User Decision:** Resolved — user accepted recommendation (Option A), 2026-10-04.
**Iteration 2:** Quick Reference and ST-12 now use `fill<T>(take v: T, …)`.

## PF-005: Nullable duplicable-struct out-slot ambiguity 🟡 MINOR

**Dimension:** 1 Ambiguities (secondary: 13)
**Location:** AR-3 vs `03-03` predicate; ST-15
**Codebase Evidence:** the predicate `Mode::Borrow && (ty.nullable || (Struct && !duplicable))`
makes `borrow slot: Point?` a pointer, while AR-3's note says the duplicable-struct by-value path
stays unchanged; ST-15 pinned only a non-nullable struct.
**The Problem:** two readings for a nullable duplicable struct; a by-value out-slot would keep the
lost-write bug for that kind.
**Options:** A (chosen) the by-value deviation means non-nullable only; nullable out-slots of any
kind become pointers; pin a `Point?` case; B carve duplicable structs out entirely (rejected:
leaves the defect).
**Recommendation:** Option A.
**User Decision:** Resolved — user accepted recommendation (Option A), 2026-10-04.
**Iteration 2:** `03-03` states the non-nullable carve-out, the error table splits nullable from
non-nullable, ST-15 gains the `Point?` case, and task 3.3.1 names the nullable duplicable struct.

**Iteration 3 (runtime, 2026-10-04):** Superseded by runtime AR-19 — structs cannot be nullable
(TY-4/E10011), so the `Point?` sub-case was removed from ST-15, `03-03`, and task 3.3.1 per the
user's ruling; the by-value deviation applies to every struct borrow.

## PF-006: Red-phase wording vs the DEF-66 reproduction gate 🟡 MINOR

**Dimension:** 3 Logical Contradictions
**Location:** `99` task 1.1.2
**Codebase Evidence:** AR-2 makes DEF-66 reproduce-first; the task asserted ST-5/6 "fail or leak"
before implementation even though a clean form must close as already-fixed.
**The Problem:** contradiction between the red-phase gate and the reproduce-first decision.
**Options:** A (chosen) reproduction-gated red-phase wording; B fix regardless (rejected by AR-2).
**Recommendation:** Option A.
**User Decision:** Resolved — user accepted recommendation (Option A), 2026-10-04.
**Iteration 2:** task 1.1.2 now reads "ST-5/6 leak only for each form the reproduction gate
confirms, otherwise they pass and close as already-fixed".

## PF-007: Inaccurate release-mechanism sentence 🟡 MINOR

**Dimension:** 12 Consistency
**Location:** `03-02` Integration Points
**Codebase Evidence:** `materialize_owned_argument(value, take = false)`
(`own/lower.rs:1401-1432`) binds a body-scope owned local; `pending_releases` covers the guarded
`Materialize` path (`own/lower.rs:646-679`, `own.rs:1058-1073`).
**The Problem:** "the binding reuses `pending_releases`" is false for the statement path.
**Options:** A (chosen) distinguish the statement local from the guard-deferred release; B leave.
**Recommendation:** Option A.
**User Decision:** Resolved — user accepted recommendation (Option A), 2026-10-04.
**Iteration 2:** the sentence now reads "The statement path binds through the existing
owned-argument local; the guard path reuses the deferred `Materialize` release".

---

## Hardening

One independent challenger (`general`, read-only, instrumented workspace copy outside the repo)
received the three MAJOR findings without the recommendations and returned: PF-001 CONFIRM;
PF-002 CONFIRM and stronger (SEGV/compile error reproduced; AR-10 unsatisfiable for out-slots);
PF-003 REFUTE — its offset model was wrong and the real cause is the Unknown element type. The
challenger's verdicts are incorporated above; PF-003 was rewritten around its diagnosis and my
own probe plus `codegen/function.rs:450-459`/`drop.rs:261-267` reading confirm it.

## Iteration 2 Verification

- All seven findings verified fixed in the edited plan documents (details per finding).
- Bounded re-scan of the 13 dimensions over the edited target: no new findings.
- Stale-phrase scan (`materialize view conversions`, `byte-identical`, `slice-root
  place-derivation`, `reuses pending_releases`): none outside this report and the register.
- `AR-10` now appears only in the register (creation-time audit trail) and the `03-03` header
  supersession note; the register itself is not rewritten, preserving the decision trail.
- The spec-modification set is unchanged (registry row + golden + `ASSERTED_CODES` for `E10405`);
  the PF-002 rejection reuses the already-registered `E10200`.

**Verdict: ✅ PASSED — all 3 MAJOR and all 4 MINOR findings resolved.**
