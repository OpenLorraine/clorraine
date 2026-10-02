# Preflight Report: Residual Defect Completion

> **Status**: ✅ PREFLIGHT PASSED — all 7 findings resolved
> **Iteration**: 1 (first scan; fixes applied and verified in a bounded re-check)
> **Artifact**: Implementation plan at `plans/residual-defect-completion/`
> **Artifact Hash**: `30c2655cd7d0dd2bb22d50d8d9d2d91596e6a36d4485b47bda27876c86f3bbc0` (sha256 over the 10 docs, sorted; post-fix)
> **Codebase Grounded**: 24 source/test files examined, 30+ references mapped (all key claims verified)
> **Codebase Baseline**: HEAD `d438347`
> **Last Updated**: 2026-10-02 09:32

```
SAME-SESSION REVIEW: This artifact was created in the current session.
Same-agent bias risk is elevated. Consider running preflight in a new session
for maximum review independence.
```

One independent challenger reviewed the full CRITICAL/MAJOR batch (blind to the lead's
picks) per the recommendation-hardening protocol; it converged on both recommendations.

### Codebase Context Summary

**Tech Stack:** Rust workspace, std-only, two binaries (`clc`, `clorraine`); emits one C99
translation unit; tests are Rust integration suites plus sanitizer probes.
**Architecture:** lex → parse → load → resolve → sema → own (checking, then lowering into an
ownership-annotated IR) → codegen (guards embedded as C comma expressions; ownership replayed).
**Key Files Examined:** `codegen/types.rs`, `codegen/calls.rs`, `codegen/expr.rs`,
`codegen/program.rs`, `codegen/function.rs`, `codegen/guards.rs`, `codegen/drop.rs`,
`codegen/control.rs`, `codegen.rs`, `ir.rs`, `ir/guards.rs`, `own.rs`, `own/lower.rs`,
`own/generics.rs`, `own/call.rs`, `own/call_check.rs`, `own/expr.rs`, `own/statements.rs`,
`types/mod.rs`, plus `call_value_integrity_impl_test.rs`, `runtime_value_integrity_impl_test.rs`,
`ownership_drop_spec_test.rs`, `codegen_run_spec_test.rs`, and the six repros in
`/tmp/opencode/def-repros/`.
**Reproductions re-confirmed on `d438347`:** DEF-50 `ClGrid__i32` redefinition; DEF-52
`unknown type name 'ClT'`; DEF-54 `ClInner__String` passed where `ClInner__String *` expected;
DEF-51 LeakSanitizer leak report.
**Key Observations:**
- Ownership checking and lowering are two separate passes (`statements.rs:220` then `:235-302`),
  which constrains fixes that need a lowering-created place during checking.
- Closure value/call/environment rendering is site-keyed (`expr.rs:574-632`, `types.rs:298-300`),
  so per-instance lambda specialization has a wider surface than IR provenance alone.
- The recorded reproductions match the plan's failure table; no existing test pins array-folded
  symbols or `ClSlice_` typedefs, so the extent-key change has no hidden pin conflicts.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 0 | — |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 1 (PF-002) | 🟠 |
| 4 | Completeness Gaps | 2 (PF-001, PF-006) | 🟠 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 1 (PF-002) | 🟠 |
| 7 | Testability | 2 (PF-003, PF-004) | 🟡 |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 0 | — |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 1 (PF-005) | 🟡 |
| 13 | Codebase Alignment | 3 (PF-001, PF-002, PF-007) | 🟠 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 2 | all resolved (PF-001, PF-002) |
| MINOR | 4 | all resolved (PF-003 … PF-006) |
| OBSERVATION | 1 | resolved (PF-007) |

---

## PF-001: DEF-52 edit surface omits the closure rendering and type-spelling files 🟠 MAJOR

**Dimension:** 4 Completeness Gaps · 13 Codebase Alignment (Impact Blindness)
**Location:** `03-02-lifted-lambda-instances.md` §Implementation Details; `02-current-state.md`
§Relevant Files (R3 rows); `99-execution-plan.md` task 2.2.2.
**Codebase Evidence:**
- `compiler/src/codegen/expr.rs:574-580` — `closure_value` renders a capture-less lambda as
  exactly `cl_lambda_{site}`; a capturing one as `((ClClosure_{site}){ cl_lambda_{site}, ... })`.
- `compiler/src/codegen/expr.rs:598-619` — `closure_call_with_destination` builds the callee
  text from `site` only.
- `compiler/src/codegen/expr.rs:629-632` — `environment_literal` renders `struct ClEnv_{site}`.
- `compiler/src/codegen/types.rs:298-300` + `:60` — `closure_type(id) = ClClosure_{id}`, used for
  locals, parameters, and fields; `type_key` at `:292` likewise keys by id.
- `compiler/src/codegen/program.rs:807-828` — `private_function_references` pushes
  `cl_lambda_{site}` for every lifted function.

**The Problem:** The plan correctly specifies per-instance specialization (one
`cl_lambda_<site>__<key>` and one `ClEnv_<site>__<key>` per needed instance), and R3/ST-9 require
a capturing lambda whose capture type mentions the generic parameter. But the changed-file map
(02 and 03-02) names only `ir.rs`, `own/lower.rs`, `own/generics.rs`, `program.rs`, and
`function.rs`. The literal symbol and struct spellings live in `expr.rs` (value, call, env) and
`types.rs` (closure type), so without those edits even the capture-less primary repro (ST-7)
still emits `cl_lambda_{site}` and two instances share `ClClosure_{site}` with incompatible
field types.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Extend 03-02 and the Phase 2 tasks to name `codegen/expr.rs` and `codegen/types.rs`, state that the active specialization shapes closure value/call/env spelling and the `ClClosure` name, and add an impl assertion for the specialized names | Plan becomes executable as written; ST-9 gets a designed path; keeps per-instance policy | Touches two more files (already required) |
| B | Narrow R3/ST-9 to capture-less lambdas; file capturing parameter-typed lambdas as a new residual | Smaller phase | Contradicts AR-2 and ST-9; even capture-less still needs `expr.rs`; splits an approved fix |
| C | Leave as written; the executor discovers the surfaces at red/green time | No plan edit | The executable contract omits required surfaces for both ST-7 and ST-9 |

**Recommendation:** Option A. Grounded: `expr.rs`/`types.rs` contain the exact spellings the
required behavior changes, and AR-2's per-instance policy cannot be met without them. (Challenger
refinement: state the behavior and name the surfaces, but allow either a rendering-context field
or a key-carrying type remap.)
**Related:** AR-2, AR-4; R3; ST-7, ST-9.
**User Decision:** Option A — extend the DEF-52 surfaces. Applied to 03-02 (new
`codegen/types.rs`/`codegen/expr.rs` bullets, corrected `ClosureTy` citation, rendering test
requirements), 02 (types.rs and expr.rs rows), and 99 (tasks 2.2.2 + new 2.2.3).

## PF-002: DEF-53's check-to-lowering handoff is not realizable as written 🟠 MAJOR

**Dimension:** 3 Logical Contradictions · 6 Feasibility Concerns · 13 Codebase Alignment
**Location:** `03-03-call-result-ownership.md` §Proposed Changes item 2 and §Changed functions;
`02-current-state.md` §Relevant Files (`call_check.rs` row); `99-execution-plan.md` task 3.2.2.
**Codebase Evidence:**
- `compiler/src/own/statements.rs:220` calls `self.eval(value)` for a `let`, and lowering happens
  later at `:235-302`; the same two-pass order holds for conditions (`:542` vs `:549`) and
  returns (`:821` vs `:879`).
- `compiler/src/own/call_check.rs:162-178` — `check_arguments` runs during `eval`; the
  maybe-owning take decision at `:281-305` uses `place_target(expr)`, which is `None` for
  `mk()[i]` (call root).
- `compiler/src/own/call.rs:205-212` — ownerless argument binding happens in `lower_call`
  (lowering) via `materialize_owned_argument` (`lower.rs:1133-1197`); the take branch binds only
  place-needing values and conditionals.

**The Problem:** 03-03 says the checker will "see a place (the bound temporary)" and that the
plan can "remove the suppression only where the binding guarantees it". The checker runs before
lowering, so a lowering-created temporary cannot be visible to `place_target` at check time, and
the checker's suppression is in fact still required to keep the generic body accepted (it is what
prevents `E10026` for the maybe-owning case; the concrete case at `call_check.rs:296-304` stays
rejected). As written, the DEF-53 mechanism invites a naive edit that either duplicates emitted
statements or never empties the taken element from the bound temporary, reintroducing the double
free.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Specify a checker→lowering duty record (checker marks the ownerless element take; lowering binds and emits the move-out), pinned by an impl test | Explicit cross-pass invariant | Extra cross-pass state for information lowering can compute locally |
| B | Keep the checker suppression unchanged; implement binding plus element move-out entirely in lowering; correct 03-03/02/99 wording and pin with sanitizer + release-count assertions | Smallest change; keeps the suppression that preserves R7/ST-14; no new state | The coupling is implicit; the take binding must not be gated behind the speculative-pass guards (`lower.rs:1172-1173`) |
| C | Leave as written; the executor resolves the ordering | No edit | The stated mechanism is wrong; risk of a wrong first implementation |

**Recommendation:** Option B — the suppression is necessary and the binding information is fully
available during lowering (`take` plus `maybe_owns`), so no checker change is required; correct
the mechanism text and keep the planned ASan/impl pins. (Challenger: converged; strongest
counter-argument is the implicit checker→lowering coupling, mitigated by the planned failure and
release-count pins.)
**Related:** AR-4; R5, R7; ST-13, ST-14.
**User Decision:** Option B — checker suppression unchanged; lowering binds the root and emits
the move-out. Applied to 03-03 (§Proposed Changes item 2, §Changed functions, §Integration
Points), 02 (call_check/expr rows, Gap 5), and 99 (task 3.2.2 targets `lower.rs`).

## PF-003: ST-1's expected exit code contradicts its input 🟡 MINOR

**Dimension:** 7 Testability · 4 Completeness Gaps
**Location:** `07-testing-strategy.md` ST-1 (line 34); the same program in
`03-01-type-keys-and-call-arguments.md` §Code Examples and the recorded repro
`/tmp/opencode/def-repros/def50_symbol.clo`.
**Codebase Evidence:** `/tmp/opencode/def-repros/def50_symbol.clo` returns
`a.value[0] + b.value[0]` with `a.value = [1, 2]` and `b.value = [1, 2, 3]`, so the value is
`1 + 1 = 2`. ST-1 says "the program exits `1`".
**The Problem:** A specification test written literally from ST-1 asserts the wrong exit code,
so the case would be recorded as failing after a correct fix. The input is otherwise correct.
**Options:** Single viable resolution: change ST-1's expected output to `2` (rejected
alternative: change the program to return `1` — it would diverge from the recorded reproduction
and weaken the cross-check of 03-01's example).
**Recommendation:** Correct ST-1 to "the program exits `2`".
**User Decision:** Applied — ST-1 now expects exit `2`.

## PF-004: Four ST cases lack concrete expected values 🟡 MINOR

**Dimension:** 7 Testability
**Location:** `07-testing-strategy.md` ST-9 (line 47), ST-13 (line 56), ST-15 (line 63), ST-16
(line 64).
**Codebase Evidence:** The recorded reproductions fix the values: `def51_guard_elements.clo`
returns `2` for the ST-15 condition (i reaches 2, then `i < 2` fails); `def53_ownerless_take.clo`
returns the callee's value; ST-9's captured string is unspecified.
**The Problem:** "Returns the value the callee computes", "returns the captured string's
length", and "exits with the loop's count" are not concrete input→output pairs, which the plan's
own testing checklist requires. The spec-test author must guess the numbers, and a wrong guess
becomes an immutable-oracle failure.
**Options:** Single viable resolution: state the exact expected exit values (ST-9: name the
captured string and its length; ST-13: give the callee body and its result; ST-15/ST-16: `2`).
**Recommendation:** Fill in the concrete expected values as above.
**User Decision:** Applied — ST-9 exits `3`; ST-13 returns `0` with the release and ASan
assertions; ST-15 and ST-16 exit `2`.

## PF-005: The DEF-53 reproduction is described with a call-site marker the recorded repro omits 🟡 MINOR

**Dimension:** 12 Consistency
**Location:** `02-current-state.md:95` ("calling `sink<T>(take mk()[i])`") and
`07-testing-strategy.md` ST-13; versus `03-03-call-result-ownership.md` §Code Examples
(`sink<T>(mk()[i])`) and the recorded repro
`/tmp/opencode/def-repros/def53_ownerless_take.clo` (no call-site marker).
**Codebase Evidence:** Call-site markers do parse (`compiler/src/parse/expr.rs:219-226` calls
`parse_mode`), so both spellings are accepted; the recorded reproduction is the authority for the
"confirmed reproduction" column, and it has no `take`.
**The Problem:** The current-state section claims the confirmed reproduction calls
`sink<T>(take mk()[i])`, which is not the file that was run; ST-13 then inherits the embellished
spelling. This is documentation drift, not a broken test, but it weakens the audit trail.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Align 02 and ST-13 to the recorded repro spelling (`sink<T>(mk()[i])`) | Matches the evidence exactly | None material |
| B | Keep the marker and state that it is equivalent and parses | Explicit about the equivalence | Still diverges from the recorded repro description |

**Recommendation:** Option A.
**User Decision:** Applied — 02's reproduction table and ST-13 now use `sink<T>(mk()[i])`,
matching the recorded repro.

## PF-006: The `type_key` unit tests are assigned to the integration suite that cannot call them 🟡 MINOR

**Dimension:** 4 Completeness Gaps
**Location:** `99-execution-plan.md` task 1.3.1 versus `03-01-type-keys-and-call-arguments.md`
§Testing Requirements.
**Codebase Evidence:** `type_key` is `pub(super)` (`compiler/src/codegen/types.rs:273`), visible
only inside `codegen`; its existing unit tests live in `#[cfg(test)] mod tests` at
`types.rs:326-397`. An integration test in `compiler/tests/` cannot call it.
**The Problem:** Task 1.3.1 names only `compiler/tests/residual_defect_completion_impl_test.rs`
for "symbol-key unit shapes", while 03-01 correctly calls for unit tests in the `types.rs` test
module. An executor following the task list could skip the unit tests or attempt an
uncompilable import. The integration suite can still assert emitted symbol names; both belong in
the task.
**Options:** Single viable resolution: name both surfaces in task 1.3.1 (add the `types.rs`
in-module unit tests for scalar/multi-extent/nested/slice/struct element keys, and keep the
emitted-symbol assertions in the integration suite).
**Recommendation:** Update task 1.3.1 to name `compiler/src/codegen/types.rs` as well.
**User Decision:** Applied — task 1.3.1 now names both the `types.rs` module test block and the
integration suite.

## PF-007: `03-02` cites the semantic closure record where it means the compact closure type 🔵 OBSERVATION

**Dimension:** 13 Codebase Alignment (citation precision)
**Location:** `03-02-lifted-lambda-instances.md` §Integration Points.
**Codebase Evidence:** `compiler/src/types/mod.rs:166-175` is `ClosureSite` (id, params, returns,
captures — the semantic record); the `TyKind::Closure` payload is `ClosureTy` at
`types/mod.rs:122-129` (id, params, returns).
**The Problem:** The sentence "The `Closure` type carries only a site id" points at the wrong
range; it is otherwise accurate and does not change the design.
**Recommendation:** Point the citation at `types/mod.rs:122-129`.
**User Decision:** Applied — 03-02 now cites `types/mod.rs:122-129`.

---

## Verdict

**✅ PREFLIGHT PASSED — all 7 findings resolved** (PF-001 … PF-007). Iteration-2 bounded
verification re-read the edited sections: the stale claims (site-only closure rendering,
"checker sees a place", call-site `take` reproduction, wrong ST-1 exit, closure citation) are
absent; the new surfaces appear in 02/03-02/03-03/99/00-index; the execution plan holds 34 tasks.
Post-fix artifact hash: `30c2655cd7d0dd2bb22d50d8d9d2d91596e6a36d4485b47bda27876c86f3bbc0`.

Roadmap sync: the six DEF rows advance to `Plan Preflighted` (🔬). A plan-creation or execution
step that consumes this artifact should compare its revision with the recorded hash; a mismatch
makes this pass stale and requires a targeted re-check of the changed sections.
