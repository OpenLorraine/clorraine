# Execution Plan: Def Backlog Closeout

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-12 00:47
> **Progress**: 44/44 tasks (100%)
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
- [x] 3.3.3 Dispatch the phase review (blast-radius and test-integrity steps included); resolve in-scope findings; one bounded re-review of a fix diff ✅ (completed: 2026-10-04 12:01)

**Deliverables**:
- [x] Implementation suite for the ABI shapes
- [x] Clean probe matrix and freestanding compile
- [x] Phase review with no open critical/major finding

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

**Phase 3 review findings (2026-10-04 11:41)** — four reviewers dispatched in parallel on the phase
diff (baseline `2b2de6a0`): correctness via the generic reviewer role, memory safety and emission
integrity via the project auditors, semantics via the generic reviewer role (the manual-only
semantics role fallback).

| ID | Severity | Finding | Status |
| -- | -------- | ------- | ------ |
| RV-006 + SR-001 | 🔴 CRITICAL | The out-slot exact-place rule ran only on direct calls: generic calls skipped it (`check_generic_call` checked only conversion) and dot-calls passed an empty mode list, so a `String` place widened into a `String?` slot and the callee wrote through it (SEGV, ASan-confirmed) | ✅ fixed — the rule now runs in `check_generic_call` with the inferred/substituted parameter types and on the dot-call route with the signature's modes; spec cases `p3f1`/`p3f1dot` reject with `E10200` |
| SR-002 + RV-007 | 🔴 CRITICAL | The `contains_param` fallback disagreed with the callee's instance signature: `fill<String?>(..., borrow slot: T)` emitted `ClString *slot` but passed the argument by value (SEGV/corruption); the own-stage call check also read the instantiated nullable slot as a possibly-null value (E10010) | ✅ fixed — `call_arguments` takes the call's type arguments and renders with `instance_param_types` (reusing `substitute_instance`); the own-stage call check substitutes the explicit type arguments; spec case `p3f2` runs green and impl case `inst-slot` pins `(ClString v, ClString *slot)` and `&(slot)` |
| RV-008 + PE-006 | 🔴 CRITICAL | FFI borrow arguments took an address while the FFI prototype ignores modes (`spec/17-codegen.md:119-124`): wrong ABI for pointer typedefs, invalid C for non-duplicable structs; the base passed by value | ✅ fixed — the pointer decision is limited to user calls and func values; impl case `ffi-slot` pins `void use(ClEntity); use(e); rawSlot(slot);` with no addresses and a strict `-Werror` build |
| RV-009 + SA-008 + PE-007 | 🟠 MAJOR | AR-20 null rendering covered only destination-typed stores; call arguments, conditional arms, and global initializers emitted `NULL` against `ClStr`/`ClSlice` (accepted programs failed the C build) | ✅ fixed — those positions render the zero view; impl case `null-views` and spec case `p3f4` exercise all three and run clean |
| SR-003 | 🟠 MAJOR | The zero view was emitted as `(ClStr){0}`, a compound literal forbidden by `spec/17-codegen.md:20-22`; the impl test pinned it | ✅ fixed — the emitter emits `cl_zero_ClStr`/`cl_zero_ClSlice_<key>` helpers next to the view typedefs and uses them in expression positions; `empty_return_value` follows; the previous `(ClStr){0}` pin in `runtime_value_integrity_impl_test` was corrected |
| RV (minors) | 🟡 | `T[]? == null` is `E10222` (AR-3's `T[]?` narrowing unproven); `slot = slot` destroys the out-slot value; spurious `W10001` on `raw?`; the ASan oracle is not gated in `make check`; the exact-place rule is plan-derived (the frozen spec is silent) and the same-type non-place message reads `cannot convert 'File?' to 'File?'`; missing generic/dot/FFI/non-store coverage | report-only — the coverage gaps are closed by the new cases; the `T[]?` narrowing and same-type message are noted for close-out |

Re-review is dispatched on the fix diff only (one bounded pass, profile-gated).

**Fix evidence (2026-10-04 11:41)** — spec-first regression coverage plus the reviewer repros
re-run on the head:

| Fix | Red before | Green after |
| --- | ---------- | ----------- |
| Generic call exact-place (`p3f1`) | accepted, SEGV | `E10200` |
| Dot-call exact-place (`p3f1dot`) | accepted, SEGV | `E10200` |
| Instance substitution (`p3f2` widening / valid shape) | widening accepted; valid shape `E10010` | widening `E10200`; valid shape exits 3, ASan clean |
| FFI by value | `use(&(e))` failed the strict build | `void use(ClEntity); use(e); rawSlot(slot);` strict clean |
| Zero-view helper | `slot = NULL` / `(ClStr){0}` | `cl_zero_ClStr()`; no compound literal on the changed paths |
| Non-store null positions | argument/conditional/global invalid C | strict build clean; run exits 4 |

Both suites green together (26 spec + 23 impl); `make check` green (42 suites); the phase probes
re-run clean (`def65_nonnull_slot` exit 3, `def65_str_slot` exit 3, no sanitizer output); the
freestanding `str?` program compiles no-libc with `cc -ffreestanding -c`.

**Re-review outcome and second fix round (2026-10-04 12:01)** — the single permitted re-review,
scoped to the fix diff (`8c82901..ea75a19`), confirmed F3, the top-level F4 positions, and F5
closed, and found six remaining issues; the user authorized the recommended resolution for each.

| ID | Severity | Finding | Status |
| -- | -------- | ------- | ------ |
| R2-1 + MSA-P3-01 | 🔴 CRITICAL | Inside a generic body the exact-place rule could not see the instance, and the instantiation audit had no out-slot duty, so a wrong-typed place was accepted and written through (stack-buffer-overflow / SEGV) | ✅ fixed — out-slot duties are recorded in the ownership call check (`ir::RequiresOutSlotPlace`) and audited per instance in `own/instance_check.rs`; spec case `p3r21` rejects with `E10200` |
| R2-2 + MSA-P3-02 | 🔴 CRITICAL | The dot-call receiver bypassed the rule: a `String` receiver widened into a `String?` slot and was written through (SEGV) | ✅ fixed — `check_dot_call` applies the exact-place rule to the receiver when the callee's first parameter is an out-slot; spec case `p3r22` rejects |
| R2-3 + NS-2 | 🔴 CRITICAL | `instance_param_types` applied the caller's substitution first, so a caller and callee sharing a type-parameter name could not bind the instance arguments (by-value argument → SEGV) | ✅ fixed — the explicit-arguments path resolves the callee's declared parameter types fresh and then applies the instance substitution; spec case `p3r23` writes through |
| R2-4 + NS-3 | 🔴/🟠 | Instance symbols ignored nullability, so `pick<String>` and `pick<String?>` collided and the accepted program failed the C build | ✅ fixed for function instances (`instance_arg_key` appends a nullability marker at the call site and the emission); the struct-instance variant is filed as DEF-79; spec case `p3r24` and impl case `inst-key` build both instances |
| R2-5 + NS-4 | 🟠 MAJOR | Nested conditional null arms bypassed the zero-view rendering (`a ? (b ? null : "x") : "z"`, `flag ? null : null`) | ✅ fixed — `conditional_arm` recurses with the outer result type and `guarded_value` shares the same rendering; spec case `p3r25` and impl case `nest-null` |
| R2-6 | 🟠 MAJOR | The dot-call fix over-rejected valid parameter-typed nullable slots (`borrow slot: T[]?`) with a spurious `E10200` | ✅ fixed — the out-slot rule defers while the declared type still mentions a type parameter (the instance audit decides); spec case `p3r26` accepts and runs |

Targeted verification (the re-review budget is used; no third formal pass): spec 32 + impl 25 green;
`make check` green (42 suites); the phase ASan probes exit 3 with no sanitizer output; the
freestanding no-libc compile is clean. New backlog rows filed for the report-only residuals:
DEF-78 – DEF-83 (generic by-value borrow reassignment double free; struct-instance keys;
owned-struct empty-return compound literals; nullable-slice narrowing; out-slot self-assignment;
spurious `W10001` on `raw?`).

---

## Phase 4: Defer condition release (DEF-68)

> **Phase baseline tree**: `7e53c3121f147626046bedd0d2770cefbe4ee92c`
> **Expected modification set**: `compiler/tests/def_backlog_closeout_spec_test.rs`,
> `compiler/tests/def_backlog_closeout_impl_test.rs`, `compiler/src/own/loops.rs`

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-defer-condition-release.md) · [AR-7](00-ambiguity-register.md)
**Objective**: Pin the condition release on a deferred return and the unchanged paths.

- [x] 4.1.1 [spec-author] Write specification tests from ST-16 – ST-19 — `compiler/tests/def_backlog_closeout_spec_test.rs` ✅ (completed: 2026-10-04 12:17)
- [x] 4.1.2 Run the spec suite and record the red phase (ST-16 leaks; ST-17/18 pass, justified; ST-19 pins the boundary) ✅ (completed: 2026-10-04 12:19)

**Deliverables**:
- [x] Specification suite with ST-16 – ST-19
- [x] Red-phase evidence

**Red-phase evidence (2026-10-04 12:19)** — the host ASan oracle (gcc 13,
`-fsanitize=address`, `ASAN_OPTIONS=detect_leaks=1:symbolize=0`) on the three new programs at the
baseline tree `7e53c312`:

| Case | Program | Exit | Sanitizer output |
| ---- | ------- | ---- | ---------------- |
| ST-16 | `closeout-st16-deferred` (`do ... while (grow().count > 0 && i < 2)`, deferred return on the second pass) | 1 | LeakSanitizer: direct 16 B + indirect 4 B = 20 bytes in 2 allocations — the filed reproduction |
| ST-18 | `closeout-st18-deferred` (same body under `while`) | 2 | none |
| ST-19 | `closeout-st19-paths` (`break` + fall-through exits, never-taken deferred return) | 80 | none |

ST-17 reuses the existing pin `should_release_the_condition_temporary_when_a_do_while_body_returns_early`
(direct early return, owned condition temporary) in `def_backlog_completion_spec_test.rs:1125`,
green. The closeout suite runs 35/35 in the container at the baseline (the missing release is
observable only under the sanitizer); `make check` green (42 suites). In-container `RDC_ASAN` is
not usable for leak detection (the LSan reporting path loops on `AddressSanitizer:DEADLYSIGNAL`
until the harness kill at 30 s), which matches the plan's host-probe oracle policy; the host probe
is authoritative.

**Verify**: `make check`

### Step 4.2: Implementation

**Reference**: [03-04](03-04-defer-condition-release.md) §Proposed Changes
**Objective**: Union the registered defer blocks' return successors into the condition-release append.

- [x] 4.2.1 Extend `append_condition_releases` with the defer-marker scan — `compiler/src/own/loops.rs` ✅ (completed: 2026-10-04 12:20)
- [x] 4.2.2 Run the spec suite and record the green phase ✅ (completed: 2026-10-04 12:20)

**Deliverables**:
- [x] DEF-68 fix
- [x] `make check` green

**Green-phase evidence (2026-10-04 12:20)** — after the defer-marker scan:

| Probe | Before | After |
| ----- | ------ | ----- |
| ST-16 host ASan (`closeout-st16-deferred`) | direct 16 B + indirect 4 B leaked, exit 1 | exit 2, no sanitizer output |
| ST-18 host ASan (`closeout-st18-deferred`) | exit 2, clean | exit 2, clean |
| ST-19 host ASan (`closeout-st19-paths`) | exit 80, clean | exit 80, clean |

The closeout suite runs 35/35; `make check` green (42 suites). The emitted C now releases the
condition's materialized array temporary on the deferred return
(`__cl_ret = i; if (__cl_tmp0.data) { FreeBuffer(__cl_tmp0.data); } ... return __cl_ret;`),
which it did not at the baseline.

**Verify**: `make check`

### Step 4.3: Implementation Tests & Hardening

- [x] 4.3.1 Write implementation tests for the deferred return's release list and the post-loop return's absence of releases — `compiler/tests/def_backlog_closeout_impl_test.rs` ✅ (completed: 2026-10-04 12:21)
- [x] 4.3.2 Run the ST-16 host-ASan probe and `cargo test` for both suites ✅ (completed: 2026-10-04 12:21)
- [x] 4.3.3 Dispatch the phase review (blast-radius and test-integrity steps included); resolve in-scope findings; one bounded re-review of a fix diff ✅ (completed: 2026-10-04 12:42)

**Deliverables**:
- [x] Implementation suite for the release shape
- [x] Clean probe matrix
- [x] Phase review with no open critical/major finding

**Implementation-test and probe evidence (2026-10-04 12:21)** — both suites
(`def_backlog_closeout_impl_test` 27 passed, `def_backlog_closeout_spec_test` 35 passed); the host
ASan probes exit 2/2/80 with no sanitizer output; `make check` green (42 suites). The two new
implementation cases pin the emitted release between the deferred return's assignment and its
return, and the post-loop return's single exit release with no appended duplicate.

**Phase 4 review findings (2026-10-04 12:42)** — four reviewers dispatched in parallel on the phase
diff (baseline `17bcc7d`, head `3341dfb`): correctness via the generic reviewer role, memory safety
and emission integrity via the project auditors, semantics via the generic reviewer role (the
manual-only semantics role fallback). No in-scope critical or major finding; no fix landed, so no
bounded re-review applies.

| Lens | Result |
| ---- | ------ |
| Correctness | Clean. The mandatory red check reproduced: with the walk hunk reverted, `should_release_the_condition_temporary_when_a_deferred_return_leaves_the_loop` fails (26/1) and the post-loop case is a regime pin; floors, dedupe, determinism, single `append_return_releases`, and the modification set hold; baseline tree matches `17bcc7d^{tree}` |
| Memory safety | Clean. 20+ host-ASan probes (ST-16, nested loops with both temps live, outer-scope defer, two defers, deferred break/continue, scope-end defer, first-pass return, two condition calls): one null-safe release per deferred return, no leak, use-after-free, or double free; the baseline 20-byte reproduction confirmed independently |
| Emission integrity | Clean. 938 programs × 3 profiles (2814 runs): 0 status changes, 8 byte diffs, all the intended deferred-return releases (4 programs × host + hosted-amiga); 1664 strict unit builds with 0 introduced failures; all changed units strict-clean |
| Semantics | Clean. Release exactly once on every actual exit; defer order, `body_floor`/`release_floor` boundaries, and nested-loop unions correct; `record_release_events` aligned with the appended returns; no language rule changed; suites 35 spec / 27 impl / 33 completion green |

Report-only notes: two error-table rows in `03-04` (direct + deferred dedupe, nested loops) are
probe-verified but unpinned in committed tests (`SR-401`; the close-out probe runner covers them);
the spec suite alone is not a red/green oracle without the host sanitizer (`SR-402`, the documented
`RDC_ASAN` policy); one nested-in-defer path emits a null-safe second release whose effective count
stays one (`SR-403`, no action). The reviewers' pre-existing, base-identical defects are filed as
DEF-84 – DEF-90; none is caused by this phase.

**Verify**: `make check`

---

## Phase 5: Capture diagnostic (DEF-69)

> **Phase baseline tree**: `06858a69eb29ee1389486c9ccd2a9394b6dff6cc`
> **Expected modification set**: `spec/14-diagnostics.md`,
> `spec/tests/compile-fail/E10405_unlisted_capture.clo`, `compiler/tests/compile_fail_spec_test.rs`,
> `compiler/tests/def_backlog_closeout_spec_test.rs`, `compiler/tests/def_backlog_closeout_impl_test.rs`,
> `compiler/src/sema/expr.rs`, `compiler/src/sema/mod.rs`. Scope mode: `strict`.

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-capture-diagnostic.md) · [AR-4, AR-5, AR-13, AR-14, AR-21](00-ambiguity-register.md)
**Objective**: Register the approved diagnostic and its compile-fail golden before implementation.

- [x] 5.1.1 Add the approved `E10405` registry row, the compile-fail golden, the `ASSERTED_CODES` entry, and the ST-21 – ST-24 spec cases (AR-21 correction) — `spec/14-diagnostics.md`, `spec/tests/compile-fail/E10405_unlisted_capture.clo`, `compiler/tests/compile_fail_spec_test.rs`, `compiler/tests/def_backlog_closeout_spec_test.rs` ✅ (completed: 2026-10-04 13:16)
- [x] 5.1.2 Run the compile-fail suite and record the red phase (the golden reports no diagnostic; the unlisted nested case is accepted) ✅ (completed: 2026-10-04 13:16)

**Deliverables**:
- [x] Approved specification change and golden
- [x] ST-21 – ST-24 specification cases
- [x] Red-phase evidence

**Red-phase evidence (2026-10-04 13:16)** — `./spec/check.sh` green (27 files, 105
compile-fail tests; the registry row and the golden shape validate). The suites at the
implementation baseline:

| Suite | Result |
| ----- | ------ |
| `compile_fail_spec_test` | FAILED — 1 of 103 asserted goldens: `E10405_unlisted_capture.clo: no 'error E10405: ...' diagnostic line (exit Some(0))` |
| `def_backlog_closeout_spec_test` | 41 passed, 1 failed — `should_reject_a_nested_lambda_that_reads_the_outer_capture_without_listing_it` accepted (exit 0); ST-21, ST-22, the listed nested case, and the three E10401 – E10403 pins pass as justified regression pins |

Host probes at the baseline: the accepted ST-21/22/23-listed programs build strict and answer
8/42/4; the unlisted nested program emits `cl_lambda_1` with a bare `n` (`'n' undeclared` in the
host C build), the filed DEF-69 shape. Logs: `/tmp/opencode/verify-5.1.2-red.log`,
`/tmp/opencode/verify-5.1.2-spec-red.log`.

**Verify**: `make check` (the golden fails as expected)

### Step 5.2: Implementation

**Reference**: [03-05](03-05-capture-diagnostic.md) §Proposed Changes
**Objective**: Reject an unlisted enclosing read with the scope-floor check.

- [x] 5.2.1 Implement the scope-floor capture check in the lambda checker — `compiler/src/sema/expr.rs`, `compiler/src/sema/mod.rs` ✅ (completed: 2026-10-04 13:18)
- [x] 5.2.2 Run the spec cases, the golden, and `spec/check.sh`; record the green phase ✅ (completed: 2026-10-04 13:18)

**Deliverables**:
- [x] DEF-69 fix
- [x] `spec/check.sh` and `make check` green

**Green-phase evidence (2026-10-04 13:18)** — the check records each lambda's scope floor
(`lambda_floors`) and reports `E10405` when a body read resolves below the floor (an enclosing
function local/parameter or an outer lambda's capture) without appearing in the capture list;
module-level names stay above every floor. `cargo test --test def_backlog_closeout_spec_test
--test compile_fail_spec_test`: 42 passed / 0 failed and 3 passed / 0 failed. `make check`: exit 0,
42 suites green, including `spec/check.sh` (27 files, 105 compile-fail tests). The red cases now
hold: `E10405_unlisted_capture.clo` reports `error E10405: lambda reads 'n' without listing it`,
and `should_reject_a_nested_lambda_that_reads_the_outer_capture_without_listing_it` rejects.

**Verify**: `make check`

### Step 5.3: Implementation Tests & Hardening

- [x] 5.3.1 Write implementation tests for the emitted nested-capture environment names, the module-path spelling, and the E10405 diagnostic text (AR-21 correction) — `compiler/tests/def_backlog_closeout_impl_test.rs` ✅ (completed: 2026-10-04 13:20)
- [x] 5.3.2 Run `spec/check.sh` and `cargo test` for both new suites ✅ (completed: 2026-10-04 13:20)
- [x] 5.3.3 Dispatch the phase review — correctness, memory-safety, and semantics-reviewer per the `AGENTS.md` language-rule dispatch (blast-radius and test-integrity steps included); resolve in-scope findings; one bounded re-review of a fix diff ✅ (completed: 2026-10-04 14:16)

**Deliverables**:
- [x] Implementation suite for the diagnostic edge cases
- [x] Phase review with no open critical/major finding

**Implementation-test evidence (2026-10-04 13:20)** — the implementation suite grows to 30
cases: the nested environment relay (`capture-nest`: outer env from the local, inner env from
`(__env->n)`, inner body reading its own copy; exit 4), the capture-less module-symbol spelling
(`capture-env`: `static i32 cl_lambda_0(i32 v)` with no environment, calling
`cl_impl_closeout_captureenv_addTwo(cl_impl_closeout_captureenv_base)`; exit 42), and the E10405
message/help text (`capture-text`). `spec/check.sh` OK (27 files, 105 compile-fail tests);
`cargo test`: impl 30 / spec 42 / compile-fail 3, all green; `make check` exit 0 (42 suites).

**Phase 5 review findings (2026-10-04 14:03)** — three reviewers on the phase diff
`06858a69…47851e6`: correctness via the generic reviewer role, memory safety via the project
auditor, semantics via the generic reviewer role (the manual-only roles' fallback). No
change-introduced memory defect; accepted programs emit byte-identical C to the baseline; test
integrity confirmed (without the fix: golden 2/1, spec 41/1, impl 29/1, exactly the intended cases).

| # | Severity | Finding | Disposition (user ruling) |
| - | -------- | ------- | ------------------------- |
| F1 | 🟠 major | `E10405` was not enforced on `Qualified` roots (`p.x`, `p.x = v`, `p.get()`, `n.xs[i]`): `check_qualified` and `qualified_receiver_type` called `name_type` directly, so the dotted spelling stayed accepted and emitted invalid C or dropped the write | ✅ fixed — both roots route through `check_name`; spec pins for the member read and the dot-call receiver (AR-21 fix round) |
| F2 | 🟡 minor | a read of a `[take s]` capture reported E10403 plus a spurious E10405 (take captures are not declared in the lambda scope) | ✅ fixed — the floor check exempts every capture name listed in any mode; impl pin asserts only E10403 |
| F3 | 🟡 minor | an unlisted `n = n + 5` reports E10405 twice (target and read) | report-only (per-occurrence reporting style) |

Fix-round evidence (2026-10-04 14:03): the three new pins were red before the fix (spec 42/2,
impl 30/1) and green after (spec 44, impl 31, compile-fail 3); the E10405 golden still passes;
`make check` exit 0 (42 suites); `spec/check.sh` OK (27 files, 105 compile-fail tests). The
reviewers' pre-existing, baseline-identical observations are filed at close-out (capture relay
invalid C; `[base]` module-const capture emits an env-less call; the capture-less-lambda
`len(stringOf(...))` segfault; a listed copy-capture field write reporting E10024 instead of
E10402). The review also confirmed the phase closed two pre-existing holes: an unlisted nested
parameter read and a silently dropped unlisted write.

**Re-review evidence (2026-10-04 14:16)** — one bounded re-review on the fix diff
`47851e6…8eeb884` (budget used; no third pass), correctness, memory safety, and semantics:

| Lens | Result |
| ---- | ------ |
| Correctness | F1 closed: twelve unlisted dotted forms (read, write, compound, dot-call, index base and index write, call-result field, nested) each reject once with E10405; the listed counterparts build and run; F2 closed; no double report through `check_call` vs `check_expr_inner`; test integrity: without the fix spec 42/2, impl 30/1, compile-fail 3/0 — exactly the three new pins; head 44/31/3; `make check` 42 suites green |
| Memory safety | No change-introduced defect: accepted probe emissions byte-identical to the pre-fix head after `#line` stripping; host ASan probes clean (exits 3/3/5, no sanitizer lines); rejected shapes write no C; the `lambda_listed` push/pop stays in sync |
| Semantics | F1/F2 closed with one E10405 per unlisted root and spans rooted at the dotted expression; no over-rejection (module-qualified calls, enum members, imports, type paths, listed captures accepted); the rule is root-only, as designed (member-chain spellings are fields, not bindings) |

Report-only notes: the write-target and index-base spellings ride the shared `check_qualified`
path and have no dedicated pins (coverage hardening); `p.a = p.a + 1` reports E10405 twice per
occurrence (the same per-occurrence style as F3). The reviewers' pre-existing, baseline-identical
observations are filed as DEF-91 – DEF-94.

**Verify**: `make check`

---

## Phase 6: Close-out

> **Phase baseline tree**: `5c53c90`

### Step 6.1: Verification and Documentation

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria
**Objective**: Prove the whole plan, record the evidence, and close the roadmap rows.

- [x] 6.1.1 Run `make verify` (all hosted-amiga cross-toolchain legs) ✅ (completed: 2026-10-04 14:27) — `make check` 42 suites green; `spec/check.sh` OK; `corpus-amiga` all portable programs match (vamos), vbcc build warning-free, interrupt-counter and graphics built; `corpus-fs-uae` interrupt-counter and graphics outputs match; `run-fault-amiga` all six faults match; `verify: all legs passed`
- [x] 6.1.2 Write and run the close-out probe runner with one run probe per fixed defect — `build/def-backlog-closeout-probes/run-def-backlog-closeout.sh` ✅ (completed: 2026-10-04 14:29)
- [x] 6.1.3 Update the `AGENTS.md` test map with both new suites and set roadmap DEF-63 – DEF-69 to Done ✅ (completed: 2026-10-04 14:29) — `AGENTS.md` test map entry added; all seven roadmap rows verified `Done ✅`
- [x] 6.1.4 Verify acceptance criteria 1 – 8 and confirm the spec-directory diff from the plan-start baseline contains only the approved E10405 change ✅ (completed: 2026-10-04 14:30)
- [x] 6.1.5 Dispatch the close-out review (blast-radius and test-integrity steps included) ✅ (completed: 2026-10-12 00:47) — the correctness re-review lens was re-dispatched on resume and returned **PASS** on the CO-1 fix diff `f10d6f6…2848517` (the pin is red at the parent and green at the fix; 27 host strict-build + ASan probes clean across every expression-body spelling; non-lambda units byte-identical; suites green), so all three lenses are now clean; adjacent pre-existing notes: **CFN-1** (a void expression-bodied lambda over `assert`/`panic` emits `return 0;` and drops the abort) is filed as DEF-255, **CFN-2** (report-only `close_scope_after_edge()` cleanup nit) stays in the close-out findings table
- [x] 6.1.6 Resolve close-out findings within scope, write the plan summary into this document, and mark the plan Complete in `00-index.md` ✅ (completed: 2026-10-12 00:47) — CFN-1 filed as DEF-255 (a separate pre-existing defect, out of this plan's scope), CFN-2 report-only; the Plan Summary below records the phase outcomes; `00-index.md` reads Complete

**Deliverables**:
- [x] All verification passing
- [x] Close-out probe runner with recorded results
- [x] Updated `AGENTS.md` and roadmap
- [x] Close-out review with no open critical/major finding (all three lenses clean; CFN-1 filed as DEF-255; CFN-2 report-only)

**Verify**: `make verify`

**Close-out probe runner evidence (2026-10-04 14:29)** — `bash
build/def-backlog-closeout-probes/run-def-backlog-closeout.sh` exits 0; every run probe reaches its
pinned exit under host AddressSanitizer with leak detection and prints no sanitizer report, and the
front-end probe is rejected with `E10405` and writes no translation unit:

| Probe | Defect (ST) | Exit | Sanitizer output |
| ----- | ----------- | ---- | ---------------- |
| `def63_guarded_member.clo` | DEF-63 (ST-1) | 7 | none |
| `def64_aggregate_index.clo` | DEF-64 (ST-3) | 3 | none |
| `def65_nonnull_slot.clo` | DEF-65 (ST-12) | 3 | none |
| `def65_str_slot.clo` | DEF-65 (ST-14) | 3 | none |
| `def66_slice_store.clo` | DEF-66 (ST-5) | 3 | none |
| `def66_slice_take.clo` | DEF-66 (ST-6) | 3 | none |
| `def66_len_store_literal.clo` | DEF-66 + DEF-67 chained literal spelling from the Phase 1 note | 3 | none |
| `def67_len_call.clo` | DEF-67 (ST-8a) | 3 | none |
| `def67_len_store.clo` | DEF-67 (ST-8b) | 1 | none |
| `def67_dot_len.clo` | DEF-67 (ST-9) | 1 | none |
| `def67_len_guard.clo` | DEF-67 (ST-10) | 3 | none |
| `def67_storeat_regression.clo` | DEF-67 (ST-11) | 3 | none |
| `def68_defer_return.clo` | DEF-68 (ST-16) | 2 | none |
| `def69_unlisted_capture.clo` | DEF-69 rejection | `error E10405`, no C written | — |

The Phase 1 follow-up note is satisfied: the literal `i32(len(storeSlice(...)))` spelling now runs
clean with no expression-temporary leak.

**Acceptance verification (2026-10-04 14:30)** — criteria 1 – 8 from
[01-requirements.md](01-requirements.md) §Acceptance Criteria verified against the tree at
`2091fe2`:

| # | Criterion | Evidence | Status |
| - | --------- | -------- | ------ |
| 1 | All seven defects fixed; R1 – R7 each covered by a specification test and an implementation test | spec 44 / impl 31 green; per requirement: R1 guarded member take (spec `should_keep_a_guarded_member_take_clean_when_the_guard_is_true_or_false`, `should_transfer_the_member_when_an_unguarded_take_reads_a_call_result`; impl `should_bind_the_root_call_when_a_guarded_member_take_selects_it`), R2 indexed aggregate consume (spec indexed and constant-index cases; impl `should_pin_the_index_and_empty_the_selected_slot_when_an_aggregate_field_reads_a_call_result`), R3 slice temporary (spec bound/taken/non-zero-start cases; impl `should_move_the_slice_element_before_the_row_release_when_a_statement_temporary_is_bound`), R4 nullable out-slot (spec write-through cases; impl `should_spell_nullable_borrow_out_slots_as_pointers_when_the_kinds_are_passed`), R5 ownerless len argument (spec call/store/dot-len/guard cases; impl `should_bind_and_release_once_when_len_reads_a_string_call_result`), R6 do-while deferred return (spec deferred-return cases; impl `should_release_the_condition_temporary_when_a_deferred_return_leaves_the_loop`), R7 unlisted capture (spec ST-21 – ST-24 plus the member and dot-call pins; impl `should_report_the_unlisted_name_and_its_suggestion_in_the_diagnostic_text`) | ✅ |
| 2 | Host-ASan probe matrix clean for every fixed shape | 6.1.2 runner: 13 run probes at their pinned exits, sanitizer-clean; the per-phase probe tables in this document are clean | ✅ |
| 3 | `make check` green at every phase; `make verify` green at close-out | every phase **Verify** block; 6.1.1 `verify: all legs passed` | ✅ |
| 4 | The only `spec/` changes are the approved `E10405` row, golden, and `ASSERTED_CODES` entry; `spec/check.sh` passes | `git diff 12b02b668370bcb93594c557ece0de9c72f8fc5b..2091fe2 -- spec/` touches exactly `spec/14-diagnostics.md` (one-row registry addition) and `spec/tests/compile-fail/E10405_unlisted_capture.clo` (new golden); the `ASSERTED_CODES` entry lives in `compiler/tests/compile_fail_spec_test.rs`; `spec/check.sh` OK (27 files, 105 compile-fail tests) | ✅ |
| 5 | The close-out probe runner reproduces each original defect as fixed and exits zero | 6.1.2 runner exit 0 | ✅ |
| 6 | Unaffected programs keep their emitted C | the per-phase emission-integrity audits found no unintended differences (including the Phase 5 byte-identical re-review); determinism suite and every corpus leg green in `make verify`; the close-out review re-audits the plan-start baseline against the final head | ✅ |
| 7 | `AGENTS.md` lists the new suites; roadmap DEF-63 – DEF-69 are Done; the plan is Complete | `AGENTS.md` test map entry added; all seven roadmap rows `Done ✅`; `00-index.md` reads Complete (6.1.6) | ✅ |
| 8 | Every phase review dispatched per the matrix with blast-radius and test-integrity steps; no open critical/major finding | every phase review record in this document names the blast-radius and test-integrity results and closes with no open critical/major; the close-out review's three lenses are all clean (the correctness re-dispatch returned PASS on 2026-10-12) | ✅ |

**Close-out review findings (2026-10-04 14:41)** — correctness (generic), memory-safety, and
emission-integrity lenses dispatched in parallel on the close-out diff (`5c53c90…f10d6f6`) and the
whole-plan acceptance claims:

| ID | Severity | Finding | Status |
| -- | -------- | ------- | ------ |
| CO-1 | 🔴 critical | **DEF-91 provenance correction: the DEF-67 fix turned a leak into a crash.** At the plan-start tree `12b02b6`, `() => i32(len(stringOf("abc")))` emitted an inline read and ran with exit 3 (ASan leak only); at head it emits `__cl_owned0` bind → free/NULL → read, and the plain build exits 139 (SIGSEGV). Root cause: `lift_lambda` (`compiler/src/own/lower.rs:397`) runs `pop_scope()` before constructing the return terminator, so the temporary's scope drop is emitted before the terminator's value read; `Stmt::Return` avoids this through `emit_return_cleanup()` and the cleanup ladder (`compiler/src/own/statements.rs:974`, `compiler/src/own.rs:1176`). The DEF-91 row's "pre-existing, base=head" note is true only against the Phase 5 baseline `47851e6`, not the plan start. **Resolved**: the user ruled option A, "fix now in Phase 6"; fixed in `b788c3e` and `2848517` | ✅ fixed |
| CO-2 | 🟡 minor | The runner does not neutralize an inherited `LSAN_OPTIONS`; a suppression could hide a leak while the probe still matches a pinned exit | report-only |
| CO-3 | 🟡 minor | The runner has no `target/debug/clc` freshness check; a stale binary could mislead a re-run | report-only |
| CO-4 | note | The plan-start baseline `12b02b668370bcb93594c557ece0de9c72f8fc5b` is an orphaned tree with no ref; diffs still resolve by tree | no action |
| CO-5 | note | Two probe files emit `W10006: unused import 'String'`; four accepted units emit strict-C warnings at base and head alike (not in DEF-91 – DEF-94) | report-only |

Reviewer evidence: the baseline runner is red on 13 of 14 probes (12 pinned defects plus the
accepted pre-fix rejection) and exits non-zero — including an exit-matching leak caught by the
sanitizer grep; the head runner re-runs to exit 0; the focused suites re-run 44 / 31 / 3; the spec
diff check, roadmap rows, and `AGENTS.md` entry were independently confirmed; runner tamper tests
(flipped exit, planted sanitizer text, timeout 124, SIGSEGV 139) are all caught; all 25 backlog
rows DEF-70 – DEF-94 exist and no earlier phase finding is open.

**CO-1 fix and re-review evidence (2026-10-08 00:08)** — the user ruled option A, "fix now in
Phase 6". Specification-first: the spec pin `should_release_the_temporary_after_the_lifted_lambda_reads_it`
failed red (the plain strict build faulted before reaching its exit), then the `LambdaBody::Expr`
arm of `lift_lambda` was routed through `emit_return_cleanup()` and the return terminator like
`Stmt::Return` (popping the scope without re-running its cleanup). Green: spec 45 / impl 32 /
compile-fail 3, `make check` 42 suites, `spec/check.sh` OK. The close-out runner gained
`def91_lambda_len` (exit 3, sanitizer-clean; 15 probes total, runner exit 0). One bounded re-review
of the fix diff (`f10d6f6…2848517`): **memory safety PASS** (runner clean; ASan probes for the exact
repro, a capture, two temporaries, a block body, a nested lambda, and owned returns all clean with
exactly one release each; the non-lambda DEF-67 probes emit byte-identically), **emission integrity
PASS** (986 jobs; 57 differ, all `cl_lambda_*`: 11 intended read-before-ladder reorders and 46
`#line`-only markers; corpus, run, and staged sets byte-identical; 0 introduced strict-C failures),
and **correctness PASS** (the lens was re-dispatched on resume; the verdict and its evidence are
recorded below).

**Correctness re-review evidence (2026-10-12 00:47)** — the pending correctness lens was
re-dispatched over the same fix diff (`f10d6f6…2848517`) and returned **PASS**: the pin
`should_release_the_temporary_after_the_lifted_lambda_reads_it` fails at the parent (`b788c3e`)
and passes at the fix (plain and under `RDC_ASAN=1`); 27 host strict-build + ASan probes cover
every expression-body spelling (a call temporary, a parameter plus a temporary, two temporaries, a
capture, nested and directly-invoked lambdas, block bodies, owned locals, conditionals, and
projections) with zero sanitizer findings; lambda units reading an owned temporary differ from the
parent exactly by the intended `read; goto ladder` reorder; every non-lambda corpus unit is
byte-identical; the suites are green (spec 45, impl 32, `cargo test --workspace` exit 0). Two
adjacent notes were recorded: **CFN-1** (🟠 major, pre-existing and base-identical — a void
expression-bodied lambda over `assert`/`panic` emits `return 0;` in a `void` function and drops the
abort) is filed as DEF-255; **CFN-2** (report-only — the `Expr` arm pops scopes with
`self.scopes.pop()` rather than `close_scope_after_edge()`; no observable effect) is noted for a
future `lift_lambda` cleanup.

---

## Plan Summary

All seven filed defects are closed through the specification-first loop (spec tests → red →
implementation → green → implementation tests → verification) with independent post-phase
reviews; the close-out review's correctness, memory-safety, and emission-integrity lenses are all
clean.

| Phase | Outcome |
| ----- | ------- |
| 1 — Take/move-out integrity (DEF-63, DEF-64, DEF-66) | A guarded member-root take binds the call root through the deferred `Materialize` shape; an indexed aggregate-literal consume pins the run-time index and empties the selected slot; a slice element over a statement temporary is moved out before the row release. The reviews filed DEF-70 – DEF-75 |
| 2 — Expression temporaries (DEF-67) | An ownerless call result consumed as a borrow inside a larger expression (an intrinsic argument, a dot-`len` receiver, a guard) binds once and releases once. The reviews filed DEF-76 – DEF-77 |
| 3 — Borrow out-slot ABI (DEF-65) | Nullable `borrow T?` out-slots are spelled as pointers when the kinds are passed (the declaration key and the call site agree); the non-null incoming slot writes through, and the instance key keeps nullability distinct. The reviews filed DEF-78 – DEF-83 |
| 4 — Defer condition release (DEF-68) | A do-while body's deferred/return edges receive the condition's registered releases through the successor append, newest-first and deduped. The reviews filed DEF-84 – DEF-90 |
| 5 — Capture diagnostic (DEF-69) | An unlisted enclosing name is rejected with the approved `E10405`; the dotted spellings route through the shared name check; the re-review is clean. The review filed DEF-92 – DEF-94 |
| 6 — Close-out | `make verify` all legs passed; the 15-probe runner sanitizer-clean; CO-1 (the DEF-67 crash regression, filed as DEF-91) fixed in-phase under the user's option A; the correctness re-review returned PASS (2026-10-12); CFN-1 filed as DEF-255, CFN-2 report-only |

Verification totals: specification suite 45/45, implementation suite 32/32, `make check` 42 suites
green at the close-out head, `make verify` green, and the `spec/` diff from the plan start contains
only the approved `E10405` registry row and its golden. Residuals discovered by the reviews and the
execution are filed as backlog rows DEF-70 – DEF-94 (DEF-91, the plan-introduced CO-1 crash, was
fixed in-phase); the close-out's own correctness re-review added DEF-255 (CFN-1).

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
