# Execution Plan: Def Backlog Closeout

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-04 11:15
> **Progress**: 23/44 tasks (52%)
> **CodeOps Artifact Schema**: 1

## Overview

Close the seven filed defects DEF-63 – DEF-69 in six phases: take/move-out integrity, expression
temporaries, borrow out-slot ABI, defer condition release, the capture diagnostic, and close-out.
Each implementation phase follows the specification-first ordering (spec tests → red phase →
implement → green phase → impl tests → host-ASan probes → phase review) from
`07-testing-strategy.md`.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Take/move-out integrity (DEF-63, DEF-64, DEF-66) | 9 |
| 2 | Expression temporaries (DEF-67) | 7 |
| 3 | Borrow out-slot ABI (DEF-65) | 8 |
| 4 | Defer condition release (DEF-68) | 7 |
| 5 | Capture diagnostic (DEF-69) | 7 |
| 6 | Close-out | 6 |

**Total: 44 tasks across 6 phases** (no fabricated hour estimates — scope is bounded by the
task-size criteria: each task is independently testable and reviewable)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for progress.
> Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated stamp
>    after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed first,
>    else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line. The
>    plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks current
> means progress is invisible after crashes, context resets, or session handoffs.

---

## Phase 1: Take/move-out integrity (DEF-63, DEF-64, DEF-66)

> **Phase baseline tree**: `12b02b668370bcb93594c557ece0de9c72f8fc5b`
> **Expected modification set**: `compiler/tests/def_backlog_closeout_spec_test.rs`,
> `compiler/tests/def_backlog_closeout_impl_test.rs`, `compiler/src/own/lower.rs`,
> `compiler/src/own/call.rs`, `compiler/src/own/statements.rs`, `compiler/src/codegen/*` (only if
> an emitter gap is proven), `build/closeout/*` probe artifacts (gitignored). Scope mode: `strict`.

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-take-moveout-integrity.md) · [AR-2, AR-8, AR-9, AR-15](00-ambiguity-register.md)
**Objective**: Pin the three defects' required behavior before any implementation.

- [x] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-7 — `compiler/tests/def_backlog_closeout_spec_test.rs` ✅ (completed: 2026-10-04 00:50)
- [x] 1.1.2 Run the spec suite and record the red phase: ST-1/3 leak under the host ASan probe; ST-4 also loses the root call and the run-time index pin; ST-5/6 leak only for each form the reproduction gate confirms, otherwise they pass and close as already-fixed; ST-2/7 pass pre-implementation (justified: unchanged behavior); record the exact evidence ✅ (completed: 2026-10-04 00:50)

**Deliverables**:
- [x] Specification suite with ST-1 – ST-7
- [x] Red-phase evidence recorded in this plan

**Verify**: `make check` (the new suite compiles; expected failures recorded)

**Red-phase evidence (2026-10-04 00:49)** — `cargo test --test def_backlog_closeout_spec_test`
in the pinned container (log `/tmp/opencode/verify-1.1.2-red.log`):

| Case | Red state |
| ---- | --------- |
| ST-1 | FAILED — the emitted guard has no member zeroing; host ASan leaks the call result's sibling |
| ST-3 | FAILED — the run-time index is not pinned and the root call is lost |
| ST-4 | FAILED — the constant-index aggregate literal also loses the root call; plain run segfaults |
| ST-5 | FAILED — both start forms leave a dangling element (exit 107 / 49) |
| ST-6 | FAILED — the slice take aborts with `free(): double free detected` |
| ST-2, ST-7 | PASSED — justified regression pins (unguarded take already binds/zeroes/releases once; `E10026` rejection intact) |

Correction vs. this task's first wording: ST-4 is red too (constant-index aggregate literals lose the
root call on the consume path), so the justified-pass set is ST-2/ST-7 only. The DEF-66 reproduction
gate confirms both slice forms reproduce; no form closes as already-fixed.

### Step 1.2: Implementation

**Reference**: [03-01](03-01-take-moveout-integrity.md) §1–§3
**Objective**: Fix the guarded member-root take, the aggregate-literal consume, and the slice element-typing gap only if it reproduces.

- [x] 1.2.1 Bind the guarded call-rooted take argument through the deferred `Materialize` temp — `compiler/src/own/lower.rs` ✅ (completed: 2026-10-04 00:54)
- [x] 1.2.2 Prepare the aggregate-literal consume path (settle, index chain, root bind) and emit the base fill explicitly — `compiler/src/own/lower.rs` ✅ (completed: 2026-10-04 00:55)
- [x] 1.2.3 Run the DEF-66 reproduction probes (let-store, take, return forms under host ASan); for each reproducing form recover the slice element's type from the settled slice base so the existing move-out path applies, otherwise record the already-fixed evidence — `compiler/src/own/lower.rs`, `compiler/src/sema` ✅ (completed: 2026-10-04 00:56)
- [x] 1.2.4 Run the spec suite and record the green phase ✅ (completed: 2026-10-04 00:58)

**Deliverables**:
- [x] DEF-63, DEF-64 fixes and the DEF-66 decision with evidence
- [x] `make check` green

**Verify**: `make check`

**DEF-66 evidence (2026-10-04 00:56)** — all eight spec cases pass in the container. The fix sits
in `compiler/src/own/lower.rs` alone (the index lowering recovers an unknown element type from the
settled base); no `compiler/src/sema` change was needed — a mechanical narrowing of the planned
modification set. Host-ASan probes: the let-store form and the return form, each isolated by
binding the result in `main`, exit 3 with no sanitizer output; the take form exits 3 clean. The
literal `i32(len(storeSlice(...)))` probe still leaks 12 bytes, which is the DEF-67
expression-temporary shape in `main` (the call result passed to `len` is never released, visible in
the emitted C), not a DEF-66 failure; that leak closes in Phase 2, and the close-out probe runner
(task 6.1.2) re-runs the literal form.

A prior implementation test pinned the pre-fix inline spelling of a guarded ownerless take
(`residual_defect_completion_impl_test.rs`). That pin described the DEF-63 limitation this plan
fixes, so the test now pins the deferred root bind instead: the root call is assigned to a
statement temporary before the callee reads its member, the member is emptied on the evaluated
path, and no eagerly assigned owned local appears. Necessary correction, not a weakened oracle.

### Step 1.3: Implementation Tests & Hardening

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) · [AR-15](00-ambiguity-register.md)
**Objective**: Pin the emitted shapes and release counts; prove memory safety at task time.

- [x] 1.3.1 Write implementation tests for the guard zeroing shape, the aggregate fill/index slot, and the slice element move-out with its regression pins — `compiler/tests/def_backlog_closeout_impl_test.rs` ✅ (completed: 2026-10-04 01:01)
- [x] 1.3.2 Run the ST-1/3/5/6 host-ASan probes from the 07 matrix and `cargo test` for both new suites ✅ (completed: 2026-10-04 01:03)
- [x] 1.3.3 Dispatch the phase review — correctness, memory-safety, and emission-integrity auditors per the `AGENTS.md` matrix; every packet carries the blast-radius (every call site, exhaustive match, and parallel path of the changed construct) and test-integrity (an assertion must fail without the fix) steps; resolve in-scope findings; one bounded re-review of a fix diff ✅ (completed: 2026-10-04 01:38)

**Phase 1 review findings (2026-10-04 01:10)** — three reviewers dispatched in parallel on the
phase diff (baseline `12b02b66`); correctness via the generic reviewer role, memory safety and
emission integrity via the project auditors.

| ID | Severity | Finding | Status |
| -- | -------- | ------- | ------ |
| RV-001 + SA-004 | 🟠 MAJOR | `consume_place`'s new preparation emits the root bind/move eagerly, so an aggregate-literal field whose later expression consumes a projection evaluates before earlier field expressions: `Box { w: f(), v: mk().a }` runs `mk()` before `f()` (trace 21 vs. the required 12, `spec/04-expressions.md` §3); the same applies to an indexed-assignment right-hand side. Introduced by task 1.2.2 | ✅ resolved — user chose: restrict the preparation to element projections; the field projection keeps its inline read |
| SA-001 | 🔴 CRITICAL | The new index type recovery types a nested multi-extent slice access (`slice(xs,0,2)[1][0]`, `xs: String[2][2]`) as an owned scalar and releases caller storage: host ASan double free; baseline ran a wrong value without corruption. Introduced by task 1.2.3; the underlying slice view is already mistyped for multi-extent sources | ✅ resolved — user chose: gate the recovery to single-extent slice sources (baseline behavior restored; view defect filed) |
| SA-003 + PE-001 | 🔴 CRITICAL | Multi-extent slice row moves and multi-dimensional slice views double-free (`let row: T[2] = slice(rows,0,2)[i]`); reproduces identically on the baseline — pre-existing, unfiled | 📋 filed as DEF-70 |
| SA-002 | 🟠 MAJOR | Take/consume roots that are not a direct call still leak (a root call with its own `take` argument; a conditional-of-calls root); identical on the baseline — pre-existing, unfiled | 📋 filed as DEF-71 |
| OB-1 | 🟠 MAJOR | An impure index in an aggregate literal (`Box<T> { v: make()[pick()] }`) still reads slot 0 and never calls `pick()`; the baseline also lost the root call — pre-existing guard-fill gap, unfiled | 📋 filed as DEF-72 |
| PE-003 | 🟡 MINOR | A moved slot is zeroed twice (`cl_zero` then the move-out NULL); no memory impact | report-only |
| RV-002 | 🟡 MINOR | In-container `RDC_ASAN=1` is unreliable in this environment (libasan DEADLYSIGNAL loops); host ASan remains the oracle per the testing strategy | report-only (environment) |
| OB-5 | 🟡 MINOR | C argument-evaluation order is relied on for two side-effecting call arguments; identical on the baseline | report-only |
| RV-004 | 🟡 MINOR | The field-order case asserted a clean sanitizer run while the restored member read leaves the call result's sibling leak; the assertion would fail under `RDC_ASAN` | ✅ fixed — the case pins order and value under the strict host build |
| SA-005 | 🔴 CRITICAL | Re-review: the SA-001 gate checked only the immediate slice source's recorded type, so a slice of a slice (or a two-step view binding) recorded a single-extent type, recovered an owned element, and double-freed again | ✅ resolved — user chose: recover only for fixed single-extent slice sources; both repros are baseline-identical and host-ASan clean |
| RV-003 | 🟠 MAJOR | Re-review: the nested multi-extent memory pin passed on the pre-fix compiler (AddressSanitizer exits 1, so `is_some()` held, and the message spells `double-free`) | ✅ fixed — asserts exit 0 and no `AddressSanitizer`; red on the pre-fix compiler per the re-review's exact-flag run |
| PE-001r | 🟠 MAJOR | Residual of the order fix: element-projection consumes stay eager (`Box<T> { w: f(), v: make()[i] }` → trace 21; the baseline crashed) | 📋 filed as DEF-73 |
| PE-002 | 🟠 MAJOR | The authorized member revert restores the pre-existing direct-call member-consume leak (`Box { v: mk().a }`, 12 B host ASan; baseline identical) | 📋 filed as DEF-74 |
| PE-003r | 🟠 MAJOR | Pre-existing invalid C: a call-indexed assignment (`out[target()] = mkvArray()[rhsIndex()]`, also a struct-literal RHS) emits `*(&(out[i])).a` and fails the strict build | 📋 filed as DEF-75 |

Test integrity independently re-checked: every fix-bound spec case fails on the baseline (6/8 red
before), the impl suite pins the new shapes (5/7 red before), and the updated guarded-take pin is
red on the old code. `make check` passes on the head.

**Fix evidence (2026-10-04 01:20)** — spec-first regression coverage added and verified red before
the fix:

| Fix | Red before | Green after |
| --- | ---------- | ----------- |
| Field evaluation order (`should_evaluate_aggregate_fields_left_to_right_when_a_later_field_consumes_a_projection`, spec) | exit 21 (reversed trace) | exit 12 (source order) |
| Nested multi-extent access (`should_not_free_caller_storage_when_a_nested_multi_extent_slice_access_runs`, impl) | `free(): double free detected` | exit 0, no sanitizer output, byte-identical to the baseline emission for the shape |

Both suites green together (9 spec + 8 impl); `make check` green; host ASan probes clean. The
pre-existing defects are filed as DEF-70 – DEF-72 in the roadmap. One bounded re-review is
dispatched on the fix diff.

**Re-review outcome (2026-10-04 01:38)** — the single permitted re-review confirmed both fixes and
found one gate escape plus test-integrity gaps. Resolutions: the recovery gate now requires a
fixed single-extent slice source (chained and two-step view sources are baseline-identical and
host-ASan clean); the nested memory pin asserts exit 0 and no sanitizer report; the field-order case
pins order under the strict host build. The residual element-projection order case and the two
pre-existing emission defects are filed as DEF-73 – DEF-75. `make check` green (42 suites). No
further re-review remains under the protocol; the targeted re-verify above is the final check.

**Deliverables**:
- [x] Implementation suite for the three shapes
- [x] Clean probe matrix
- [x] Phase review with no open critical/major finding

**Verify**: `make check`

**Probe evidence (2026-10-04 01:03)** — host ASan over `build/closeout/`:

| Probe | Exit | Sanitizer output |
| ----- | ---- | ---------------- |
| `def63_guarded_member.clo` (both guard paths) | 7 | none |
| `def64_aggregate_index.clo` | 3 | none |
| `def66_slice_store.clo` | 3 | none |
| `def66_slice_take.clo` | 3 | none |

Both new suites pass together (8 spec + 7 impl). Probe-shape note: `def66_slice_store.clo` binds the
returned element before measuring it, because the literal `i32(len(storeSlice(...)))` spelling carries
the still-open expression-temporary leak (the DEF-67 shape) in the caller; the close-out runner
(task 6.1.2) uses the literal spelling after Phase 2, when that leak closes.

---

## Phase 2: Expression temporaries (DEF-67)

> **Phase baseline tree**: `143cef93c5a4a483c167134a8411355e25a6ece5`
> **Expected modification set**: `compiler/tests/def_backlog_closeout_spec_test.rs`,
> `compiler/tests/def_backlog_closeout_impl_test.rs`, `compiler/src/own/lower.rs`,
> `compiler/src/own/call.rs`, `build/closeout/*` probe artifacts (gitignored). Scope mode: `strict`.

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-expression-temporaries.md) · [AR-6](00-ambiguity-register.md)
**Objective**: Pin the release behavior of an ownerless call result used as a borrow argument.

- [x] 2.1.1 [spec-author] Write specification tests from ST-8 – ST-11 — `compiler/tests/def_backlog_closeout_spec_test.rs` ✅ (completed: 2026-10-04 01:50)
- [x] 2.1.2 Run the spec suite and record the red phase (ST-8/9/10 and the call-result cases leak; ST-11's local-place regression passes, justified) ✅ (completed: 2026-10-04 01:50)

**Deliverables**:
- [x] Specification suite with ST-8 – ST-11
- [x] Red-phase evidence

**Verify**: `make check`

**Red-phase evidence (2026-10-04 01:50)** — host ASan over `build/closeout/` with the pre-fix
compiler; each probe mirrors its spec case:

| Probe | Exit | Sanitizer output |
| ----- | ---- | ---------------- |
| `def67_len_call.clo` (ST-8a) | 1 | 12-byte leak |
| `def67_len_store.clo` (ST-8b) | 1 | 10-byte leak |
| `def67_dot_len.clo` (ST-9) | 1 | 10-byte leak |
| `def67_len_guard.clo` (ST-10, true path) | 1 | 12-byte leak |
| `def67_storeat_regression.clo` (ST-11) | 3 | none — justified pass (a local place needs no owner) |

The spec suite itself runs green in a plain build (14 passed); the leak cases are red only under the
sanitizer, so this matrix is the red-phase evidence. The in-container `RDC_ASAN=1` run stays
timing-flaky in this environment; the host probes are the deterministic oracle.

### Step 2.2: Implementation

**Reference**: [03-02](03-02-expression-temporaries.md) §Proposed Changes
**Objective**: Bind and release the intrinsic argument and dot-`len` receiver.

- [x] 2.2.1 Route intrinsic arguments and dot-`len` receivers through the owned-argument binding, guard-aware — `compiler/src/own/lower.rs`, `compiler/src/own/call.rs` ✅ (completed: 2026-10-04 01:53)
- [x] 2.2.2 Run the spec suite and record the green phase ✅ (completed: 2026-10-04 02:01)

**Deliverables**:
- [x] DEF-67 fix
- [x] `make check` green

**Verify**: `make check`

**Green-phase evidence (2026-10-04 02:01)** — host ASan over `build/closeout/` with the fixed
compiler:

| Probe | Exit | Sanitizer output |
| ----- | ---- | ---------------- |
| `def67_len_call.clo` (ST-8a) | 3 | none |
| `def67_len_store.clo` (ST-8b) | 1 | none |
| `def67_dot_len.clo` (ST-9) | 1 | none |
| `def67_len_guard.clo` (ST-10, true and false paths) | 3 | none |
| `def67_storeat_regression.clo` (ST-11) | 3 | none |

Both suites green together; `make check` green (42 suites). Two binding corrections came out of
the full gate. First, the binding requires an owning or maybe-owning argument, so a duplicable
fixed-array argument (`len([1, 2])`) stays inline — the strict build rejected an unused local the
first cut emitted. Second, a call that may reborrow an owned argument (`outer<String>(...)`
returning its lend parameter) is not bound: the argument's owner already releases that storage, so
binding the result double-freed. The same lend shape double-frees on the baseline when the
argument travels through a named call (`sink(outer<String>(s))` aborts, baseline-identical)
— pre-existing, noted for the phase review. The nested multi-extent memory pin now follows the
`RDC_ASAN` contract: the in-container sanitizer run is timing-flaky in this environment and is not
a gate, while the plain relaxed build still catches a double free through the C library abort. The
pin's red state is preserved (an abort leaves no exit code, failing the exit-zero assertion).

### Step 2.3: Implementation Tests & Hardening

- [x] 2.3.1 Write implementation tests for the intrinsic-argument release shape and the guard deferral — `compiler/tests/def_backlog_closeout_impl_test.rs` ✅ (completed: 2026-10-04 02:02)
- [x] 2.3.2 Run the ST-8/10/11 host-ASan probes and `cargo test` for both suites ✅ (completed: 2026-10-04 02:02)
- [x] 2.3.3 Dispatch the phase review (blast-radius and test-integrity steps included); resolve in-scope findings; one bounded re-review of a fix diff ✅ (completed: 2026-10-04 09:00)

**Phase 2 review findings (2026-10-04 02:20)** — three reviewers dispatched in parallel on the phase
diff (baseline `143cef93`): correctness via the generic reviewer role, memory safety and emission
integrity via the project auditors.

| ID | Severity | Finding | Status |
| -- | -------- | ------- | ------ |
| SA-006 | 🔴 CRITICAL | A conditional intrinsic argument with a place arm is adopted and moved: `len(flag ? s : stringOf("x"))` reads the moved-out `s` (SIGSEGV, exit 139); guarded (`flag && len(flag2 ? s : ...)`) and dot variants reproduce. Baseline clean. The shared owned-conditional store adopts a borrow argument; the same helper defect pre-exists for named calls (`sink(flag ? s : call())` crashes on the baseline too) | ✅ resolved — user chose the combined fix: a conditional with a place-rooted arm stays an inline read; spec and impl cases added; the fresh-arm leak that remains is baseline-identical (10 bytes) |
| SA-007 | 🔴 CRITICAL | The may-lend gate matches only a top-level call, so `len(flag ? t : outer<String>(t))` binds the alias and double-frees (baseline clean) | ✅ resolved — the may-lend check now follows conditional arms and requires a receiver-less call; the conditional may-lend repro is host-ASan clean |
| PE-004 | 🔴 CRITICAL | Binding an already-materialized take-call result copies the transfer temporary without emptying it (`__cl_owned1 = __cl_tmp0`); re-running the statement (loop) frees `__cl_tmp0` and then `__cl_owned1` (double free, exit 134). Non-loop forms carry the stale alias latently; baseline leaked but ran | ✅ resolved — a transfer temporary now registers its own single release instead of gaining a second owner; the loop repro is host-ASan clean and an impl case pins the absence of the alias |
| RV-005 | 🟡 MINOR | The may-lend gate lacks the receiver/local-callee guard the named-call path applies, so a func-field call whose field name collides with a module lender under-binds (leak, baseline-identical) | ✅ fixed — the may-lend check requires a receiver-less call |
| PE-005 | 🟡 MINOR (pre-existing) | A generic `len` argument emits `i32 __cl_owned0[2]` set-but-unused and fails `-Werror`; byte-identical on the baseline | 📋 filed as DEF-76 |
| Note | — | Test-integrity: the guard test's `flag && ((` assertion passes on both base and head; the duplicable and may-lend tests are pins for the later corrections, not DEF-67 oracles; ST-8b has no structural implementation pin; ST-11 is a documented regression pin | ✅ strengthened — the guard assertion now pins the read through the materialized temporary; the correction pins are documented |
| N-1 | 🔴 CRITICAL | Bounded re-review: the conditional place gate checked only the immediate arms, so a nested mixed conditional (`a ? mk1() : (b ? s : mk2())` with the deep arm chosen) still adopted the deep place arm — host-ASan heap-use-after-free across the depth-2/3, dot, field, and guard variants | ✅ resolved — user chose the recursive any-arm place predicate; depth-2/3/dot probes host-ASan clean; depth-2 impl pin added |
| RR-P2-001 | 🟡 MINOR | Re-review: the may-lend gate lacks the local-callee guard, so a func-typed parameter named like a module lender under-binds (13-byte leak, baseline-identical on all revisions) | 📋 filed as DEF-77 |
| Note | — | Bounded re-review verdicts: emission integrity clean (every hunk intended; strict builds unchanged; base↔head corpus diff expected-only); correctness clean apart from RR-P2-001; memory safety reported N-1, which the recursive fix closes | — |

**Fix evidence (2026-10-04 02:38)** — spec-first regression coverage added and verified red before
the fix:

| Fix | Red before | Green after |
| --- | ---------- | ----------- |
| Mixed conditional place arm (`should_read_the_chosen_arm_when_len_reads_a_conditional_with_a_place_arm`, spec) | SIGSEGV (exit 139, runtime flag) | exit 46 |
| Conditional argument shape (`should_not_bind_a_conditional_argument_when_an_arm_roots_in_a_place`, impl) | `__cl_owned` binding present | no binding, exit 4 |
| Take-result loop alias (`should_release_the_take_result_once_when_len_reads_it_across_loop_iterations`, impl) | ` = __cl_tmp` alias present | no alias, exit 4 |

Host-ASan probes: the conditional may-lend repro (`p53`) and the loop take repro are clean; the
mixed-conditional probes (`p41`, `p52`) no longer crash and their fresh-arm leak is
baseline-identical (10 bytes, pre-existing). Both suites green together (15 spec + 15 impl);
`make check` green (42 suites). One bounded re-review of this fix diff is dispatched.

**Re-review outcome and N-1 fix (2026-10-04 09:00)** — the single permitted re-review confirmed
the four findings fixed and the emission and correctness lenses came back clean, but the memory
lens found one remaining critical: the conditional place gate judged only the immediate arms, so
a nested mixed conditional still adopted the deep place arm (heap-use-after-free). The gate now
uses a recursive predicate: any reachable arm that roots in a place keeps the whole conditional
an inline read.

| Fix | Red before | Green after |
| --- | ---------- | ----------- |
| Nested conditional place arm (`should_not_bind_a_nested_conditional_argument_when_a_deep_arm_roots_in_a_place`, impl) | `__cl_owned` binding present | no binding, exit 44 |

Targeted re-verify (the one re-review budget is used; no third pass) — host ASan over the nested
family and the earlier probes:

| Probe | Exit | Sanitizer output |
| ----- | ---- | ---------------- |
| `n1_depth2.clo` (place arm one level down) | 44 | none |
| `n1_depth3.clo` (place arm two levels down) | 44 | none |
| `n1_dot.clo` (nested dot receiver) | 44 | none |
| `p53_cond_lend.clo` (conditional may-lend) | 44 | none |
| `loop_take.clo` (take result across iterations) | 40 | none |
| `p41`/`p52` (mixed conditional, both paths) | 1 | 10-byte fresh-arm leak, baseline-identical |

Both suites green together (15 spec + 16 impl); `make check` green (42 suites). The minor
func-typed-parameter under-bind is filed as DEF-77.
| Note | — | Accepted: the named-call may-lend double free is baseline-identical and out of scope; in-container AddressSanitizer flakiness is RV-002; the corpus emission diff is expected-only (20 hunks, all bindings/releases) and every emitted unit builds under the strict warning set | — |

**Deliverables**:
- [x] Implementation suite for the new shapes
- [x] Clean probe matrix
- [x] Phase review with no open critical/major finding

**Verify**: `make check`

**Probe evidence (2026-10-04 02:02)** — host ASan over `build/closeout/` with the fixed compiler:

| Probe | Exit | Sanitizer output |
| ----- | ---- | ---------------- |
| `def67_len_call.clo` (ST-8) | 3 | none |
| `def67_len_store.clo` (ST-8, stored element) | 1 | none |
| `def67_dot_len.clo` (ST-9) | 1 | none |
| `def67_len_guard.clo` (ST-10) | 3 | none |
| `def67_storeat_regression.clo` (ST-11) | 3 | none |

Both suites green together (13 impl + 14 spec).

---

## Phase 3: Borrow out-slot ABI (DEF-65)

> **Phase baseline tree**: `2b2de6a0f5301958ea125aaa1625a6b269b17a0f`
> **Expected modification set**: `compiler/tests/def_backlog_closeout_spec_test.rs`,
> `compiler/tests/def_backlog_closeout_impl_test.rs`, `compiler/src/codegen/types.rs`,
> `compiler/src/codegen/calls.rs`, `compiler/src/codegen/expr.rs` (added by runtime AR-20),
> `compiler/src/sema/*` (only if the out-slot argument check needs a sema change),
> `build/closeout/*` probe artifacts (gitignored). Scope mode: `strict`.

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-borrow-outslot-abi.md) · [AR-3](00-ambiguity-register.md) · [PF-002](00-preflight-report.md)
**Objective**: Pin the pointer lowering and write-through behavior of nullable borrow out-slots.

- [x] 3.1.1 [spec-author] Write specification tests from ST-12 – ST-15 — `compiler/tests/def_backlog_closeout_spec_test.rs` ✅ (completed: 2026-10-04 10:55)
- [x] 3.1.2 Run the spec suite and record the red phase (ST-12/13 double-free or lost write; ST-14 lost write; ST-15 null-start and by-value pins pass, justified; the `Point?` sub-case was removed per runtime AR-19) ✅ (completed: 2026-10-04 10:55)

**Deliverables**:
- [x] Specification suite with ST-12 – ST-15
- [x] Red-phase evidence

**Verify**: `make check`

**Red-phase evidence (2026-10-04 10:55)** — `cargo test --test def_backlog_closeout_spec_test` in the
pinned container (log `/tmp/opencode/verify-3.1.2-red.log`): 17 passed, 5 failed; the 15 prior cases
and the field-order regression stay green.

| Case | Red state |
| ---- | --------- |
| ST-12 `String?` | FAILED — expected exit 3; `free(): double free detected in tcache 2` |
| ST-13 `File?` | FAILED — expected exit 0; `free(): double free detected in tcache 2` |
| ST-14 `str?` write-through | FAILED — the emitted C does not build: `incompatible types when assigning to type 'ClStr' from type 'void *'`; `invalid operands to binary == (have 'ClStr' and 'void *')` |
| ST-14 `str?` widened argument | FAILED — the front end accepted the program instead of reporting `E10200` |
| ST-14 `String?` widened argument | FAILED — the front end accepted the program instead of reporting `E10200` |
| ST-15 null start | PASSED — justified pin (exit 1, clean) |
| ST-15 by-value duplicable struct | PASSED — justified pin (exit 1) |
| ST-15 `Point?` | removed per runtime AR-19 (TY-4/E10011: structs cannot be nullable) |

The `Point?` sub-case was written from PF-005's superseded resolution and removed after the runtime
AR-19 ruling; the suite now derives only from the frozen spec's nullable kinds.

### Step 3.2: Implementation

**Reference**: [03-03](03-03-borrow-outslot-abi.md) §Proposed Changes
**Objective**: Widen the declaration predicate and key the call-site pointer decision on the parameter type with exact-typed out-slot places.

- [x] 3.2.1 Add `ty.nullable` to the pointer-parameter predicate (declaration side) — `compiler/src/codegen/types.rs` ✅ (completed: 2026-10-04 11:12)
- [x] 3.2.2 Key the named-call pointer decision on the substituted parameter type, require an exact-typed place for out-slot arguments (reject implicit widening), run the blast-radius pass over all four predicate consumers plus the `out_slot` condition, and add nullable-view null handling per runtime AR-20 — `compiler/src/codegen/calls.rs`, `compiler/src/sema`, `compiler/src/codegen/expr.rs`, `compiler/src/codegen/types.rs` ✅ (completed: 2026-10-04 11:12)
- [x] 3.2.3 Run the spec suite and record the green phase ✅ (completed: 2026-10-04 11:12)

**Deliverables**:
- [x] DEF-65 fix
- [x] Blast-radius evidence over the four consumers
- [x] `make check` green

**Verify**: `make check`

**Green-phase evidence (2026-10-04 11:12)** — `cargo test --test def_backlog_closeout_spec_test` in the
pinned container (log `/tmp/opencode/verify-3.2.3-green.log`): 22 passed, 0 failed; `make check`
green (42 suites, log `/tmp/opencode/verify-3.2.3-check.log`).

| Case | Green state |
| ---- | ----------- |
| ST-12 `String?` | PASS — callee `ClString *`, write-through, single drop |
| ST-13 `File?` | PASS — callee `BPTR *`, single close |
| ST-14 `str?` | PASS — callee `ClStr *`, write-through; requires the AR-20 correction |
| ST-14 widened `str?` / `String?` arguments | PASS — rejected with `E10200` |
| ST-15 null start / by-value duplicable struct | PASS — unchanged pins |

Blast radius over the four `is_pointer_param` consumers plus the `out_slot` condition:

| Consumer | Result |
| -------- | ------ |
| `signature_with_symbol` (`codegen.rs:473`) | nullable borrow parameters now spell as pointers (`ClString *`, `BPTR *`, `ClStr *`, `ClSlice_T *`) |
| `Emitter::emit_function` (`function.rs:44-51`) | the pointer-parameter set follows the widened predicate, so `place_base` dereferences the out-slot |
| `func_pointer_params` (`types.rs:101-115`) | function-pointer types follow the same rule and match the named-function ABI |
| `call_arguments` (`calls.rs`) | keys on the (substituted) parameter type; a parameter type that still mentions a type parameter falls back to the substituted argument type, keeping generic instances correct (`residual_defect_completion_spec_test` green) |
| `out_slot` (`own.rs:678`) | already selected `Borrow && nullable`; unchanged |

Runtime AR-20 correction: the null literal and null test now handle struct-typed views — a view's
empty state is its zero struct (`(ClStr){0}` in expression position) and a null test reads the
view's `ptr` member. This fixes the caller's `str?` local in ST-14 and enables the `T[]?` out-slot
kind from AR-3. The first cut of the parameter-type keying broke the substituted-generic case in
`residual_defect_completion_spec_test`; the `contains_param` fallback restored it.

### Step 3.3: Implementation Tests & Hardening

- [x] 3.3.1 Write implementation tests for the emitted declarators/arguments per nullable kind, the unchanged by-value path for a duplicable struct, and the rejected widened out-slot argument — `compiler/tests/def_backlog_closeout_impl_test.rs` ✅ (completed: 2026-10-04 11:15)
- [x] 3.3.2 Run the ST-12/13 host-ASan probes, a freestanding compile, and `cargo test` for both suites ✅ (completed: 2026-10-04 11:15)
- [~] 3.3.3 Dispatch the phase review (blast-radius and test-integrity steps included); resolve in-scope findings; one bounded re-review of a fix diff ⏳ (implemented: 2026-10-04 11:15)

**Deliverables**:
- [x] Implementation suite for the ABI shapes
- [x] Clean probe matrix and freestanding compile
- [ ] Phase review with no open critical/major finding

**Verify**: `make check`

**Test and probe evidence (2026-10-04 11:15)** — `cargo test --test def_backlog_closeout_spec_test
--test def_backlog_closeout_impl_test` in the pinned container (log
`/tmp/opencode/verify-3.3.1-tests.log`): 22 spec + 20 impl passed, 0 failed.

| Probe | Exit | Sanitizer output |
| ----- | ---- | ---------------- |
| `def65_nonnull_slot.clo` (ST-12, `String?`) | 3 | none |
| `def65_str_slot.clo` (ST-14, `str?`) | 3 | none |
| `def65_freestanding.clo` (freestanding `str?` out-slot) | host no-libc `cc -ffreestanding -c` OK; `m68k-amigaos-gcc -std=c99 -c` OK (1568-byte object) | — |

The four implementation cases pin the per-kind declarators (`ClString *`, `BPTR *`, `ClStr *`,
`ClSlice_i32 *`), the address-taking call sites, the zero-view null init and `.ptr` null test, the
unchanged by-value `ClPoint` lend, and the `E10200` rejection of both widened and non-place
out-slot arguments.

---

## Phase 4: Defer condition release (DEF-68)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-defer-condition-release.md) · [AR-7](00-ambiguity-register.md)
**Objective**: Pin the condition release on a deferred return and the unchanged paths.

- [ ] 4.1.1 [spec-author] Write specification tests from ST-16 – ST-19 — `compiler/tests/def_backlog_closeout_spec_test.rs`
- [ ] 4.1.2 Run the spec suite and record the red phase (ST-16 leaks; ST-17/18 pass, justified; ST-19 pins the boundary)

**Deliverables**:
- [ ] Specification suite with ST-16 – ST-19
- [ ] Red-phase evidence

**Verify**: `make check`

### Step 4.2: Implementation

**Reference**: [03-04](03-04-defer-condition-release.md) §Proposed Changes
**Objective**: Union the registered defer blocks' return successors into the condition-release append.

- [ ] 4.2.1 Extend `append_condition_releases` with the defer-marker scan — `compiler/src/own/loops.rs`
- [ ] 4.2.2 Run the spec suite and record the green phase

**Deliverables**:
- [ ] DEF-68 fix
- [ ] `make check` green

**Verify**: `make check`

### Step 4.3: Implementation Tests & Hardening

- [ ] 4.3.1 Write implementation tests for the deferred return's release list and the post-loop return's absence of releases — `compiler/tests/def_backlog_closeout_impl_test.rs`
- [ ] 4.3.2 Run the ST-16 host-ASan probe and `cargo test` for both suites
- [ ] 4.3.3 Dispatch the phase review (blast-radius and test-integrity steps included); resolve in-scope findings; one bounded re-review of a fix diff

**Deliverables**:
- [ ] Implementation suite for the release shape
- [ ] Clean probe matrix
- [ ] Phase review with no open critical/major finding

**Verify**: `make check`

---

## Phase 5: Capture diagnostic (DEF-69)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-capture-diagnostic.md) · [AR-4, AR-5, AR-13, AR-14](00-ambiguity-register.md)
**Objective**: Register the approved diagnostic and its compile-fail golden before implementation.

- [ ] 5.1.1 Add the approved `E10405` registry row, the compile-fail golden, and the `ASSERTED_CODES` entry — `spec/14-diagnostics.md`, `spec/tests/compile-fail/E10405_unlisted_capture.clo`, `compiler/tests/compile_fail_spec_test.rs`
- [ ] 5.1.2 Run the compile-fail suite and record the red phase (the golden reports no diagnostic)

**Deliverables**:
- [ ] Approved specification change and golden
- [ ] Red-phase evidence

**Verify**: `make check` (the golden fails as expected)

### Step 5.2: Implementation

**Reference**: [03-05](03-05-capture-diagnostic.md) §Proposed Changes
**Objective**: Reject an unlisted enclosing read with the scope-floor check.

- [ ] 5.2.1 Implement the scope-floor capture check in the lambda checker — `compiler/src/sema/expr.rs`, `compiler/src/sema/mod.rs`
- [ ] 5.2.2 Run the spec cases, the golden, and `spec/check.sh`; record the green phase

**Deliverables**:
- [ ] DEF-69 fix
- [ ] `spec/check.sh` and `make check` green

**Verify**: `make check`

### Step 5.3: Implementation Tests & Hardening

- [ ] 5.3.1 Write implementation tests for the nested-capture, global-read, and unchanged capture-mode cases — `compiler/tests/def_backlog_closeout_impl_test.rs`
- [ ] 5.3.2 Run `spec/check.sh` and `cargo test` for both new suites
- [ ] 5.3.3 Dispatch the phase review — correctness, memory-safety, and semantics-reviewer per the `AGENTS.md` language-rule dispatch (blast-radius and test-integrity steps included); resolve in-scope findings; one bounded re-review of a fix diff

**Deliverables**:
- [ ] Implementation suite for the diagnostic edge cases
- [ ] Phase review with no open critical/major finding

**Verify**: `make check`

---

## Phase 6: Close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill)_

### Step 6.1: Verification and Documentation

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria
**Objective**: Prove the whole plan, record the evidence, and close the roadmap rows.

- [ ] 6.1.1 Run `make verify` (all hosted-amiga cross-toolchain legs)
- [ ] 6.1.2 Write and run the close-out probe runner with one run probe per fixed defect — `build/def-backlog-closeout-probes/run-def-backlog-closeout.sh`
- [ ] 6.1.3 Update the `AGENTS.md` test map with both new suites and set roadmap DEF-63 – DEF-69 to Done
- [ ] 6.1.4 Verify acceptance criteria 1 – 8 and confirm the spec-directory diff from the plan-start baseline contains only the approved E10405 change
- [ ] 6.1.5 Dispatch the close-out review (blast-radius and test-integrity steps included)
- [ ] 6.1.6 Resolve close-out findings within scope, write the plan summary into this document, and mark the plan Complete in `00-index.md`

**Deliverables**:
- [ ] All verification passing
- [ ] Close-out probe runner with recorded results
- [ ] Updated `AGENTS.md` and roadmap
- [ ] Close-out review with no open critical/major finding

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (take/move-out)
    ↓
Phase 2 (expression temporaries)
    ↓
Phase 3 (borrow out-slot ABI)
    ↓
Phase 4 (defer condition release)
    ↓
Phase 5 (capture diagnostic)
    ↓
Phase 6 (close-out)
```

Phases 1 – 2 share `compiler/src/own/lower.rs`, so they run in order; phases 3 – 5 touch disjoint
files but stay ordered for review simplicity.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per phase, `make verify` at close-out)
3. ✅ No warnings/errors
4. ✅ No dead code — no unused parameters, functions, classes, or modules
5. ✅ Memory safety proven — the host-ASan probe matrix is clean; no leak, double free, use-after-free, or invalid free remains
6. ✅ Documentation updated — `AGENTS.md` test map and roadmap rows
7. ✅ Code reviewed — per-phase and close-out reviews dispatched with no open critical/major finding
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
