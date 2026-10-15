# Preflight Report: conditional-projection-residue-ii

> **Status**: ✅ PASSED WITH NOTES — 0 critical/major; 7 minor findings corrected in place
> (auto-design delegated); 1 observation recorded
> **Iteration**: 1 (first scan; the minors were corrected during this scan and the edited
> sections re-read)
> **Audit target**: `plans/conditional-projection-residue-ii/` — the full implementation plan
> (12 documents: index, ambiguity register, requirements, current state, five design docs,
> findings ledger, testing strategy, execution plan); flat layout
> **Artifact hash**: `766d8c28605a89b0e159a672c90b1d31accc181b0a8f88a44076b6d59cb1627f`
> (scan start) → `e208ed9e5ffb9d77d89348194bf4911695d9c51a262aef28dfca2e7c8b584e7d`
> (after the corrections; both hashes cover the 12 plan documents, excluding this report).
> The folder is untracked, so these are content hashes over the sorted files, not git blobs
> **Head at scan start**: `a4620847621949a0d00ca86491a656c83b049af8` ("docs(plans): close the
> conditional-projection-residue batch") — the plan's own declared planning baseline; the
> tracked tree is clean, only untracked plans/ and `.opencode/` tooling
> **Scope mode**: strict · **Modification set (authorized by the dispatch)**: the 12 plan
> documents + this report
> **Mode**: auto-design — standing batch mandate; all resolutions below are eligible
> testing-strategy / plan-text corrections within the confirmed scope; no reserved-authority
> choice (spec text, user-visible behavior, acceptance criteria) arose
> **Same-session bias**: none — the plan was authored in earlier sessions; this scan is
> fresh-context and re-probed every checkable planning claim firsthand

### Codebase Context Summary

**Repository:** the Clorraine compiler (Rust workspace, std-only; `clc` emits one C99
translation unit; `clorraine` is the driver). **Toolchains used for probing:** container
GCC 12.2 (`cc` inside `make -C docker exec`), host GCC 15.2; ASan via host `cc
-fsanitize=address -no-pie`; compiler binary `target/debug/clc` (shared between host and
container through the mounted repo). **Files examined:** the 12 cited compiler modules
(`own/lower.rs`, `own/place.rs`, `own/statements.rs`, `own/expr.rs`, `own/call_check.rs`,
`own/liveness.rs`, `own/loops.rs`, `own.rs`, `codegen/function.rs`, `codegen/guards.rs`,
`codegen/cleanup.rs`, `ir.rs`), the batch-I suite pair (6 013 + 3 110 lines), the three
pinned oracle suites (`module_initializer_integrity_spec_test.rs`,
`callable_release_integrity_spec_test.rs`, `checker_conformance_spec_test.rs`), six spec
chapters (`03`, `04`, `14`, `16`, `17`, grammar spot-checks), `plans/00-roadmap.md`, the
batch-I ledger and its preflight report. **Reference verification:** all ~32 cited code
sites resolve to the named functions; the 27 roadmap rows' statuses, the 27 ST cases, the
33 tasks, and all 17 AR entries were mapped. Two site-map entries are approximate and the
plan already says the executor re-resolves them (`own/expr.rs` :564/:885/:940 name the
element-store/judgment region; `ir.rs` :299 is the `Statement` enum head; the two variants
are at :302/:314).

**Probe evidence (planning head = scan head).** Probe sources: exact batch-I scratch
sources where they survive (`build/memory-audit/**`, `build/cpr-probe/**`,
`build/p3s-probes/**`), otherwise re-authored from the recorded shape; all scratch copies
lived under `/tmp/opencode/cpr2/` (removed after this scan). Every claim below is a
measured command result, not a document reading.

| Row(s) | Claim under test | Probe result at `a462084` | Verdict |
|--------|------------------|---------------------------|---------|
| `DEF-303` (🔴) | `String[2]` conditional-root return segfaults | strict build OK both toolchains; run exit 139; ASan SEGV in `cl_string_len` (`__cl_ret = __cl_tmp1;` at emitted line 635: array assignment through the C-adjusted parameter) | **red confirmed** |
| `DEF-294` (🟠) | toolchain split on the mixed-arm loop receiver | container GCC 12.2: build + run exit 0; host GCC 15.2: `-Werror=sequence-point`; emission shows `__cl_tmp0` in an unsequenced comma expression | **split confirmed** |
| `DEF-295` (🟠) | loop-condition member projection double-frees | run abort 134 both toolchains; ASan `attempting double-free` in `cl_free` | **red confirmed** |
| `DEF-310` (🟠) | bare raw-deref store silently dropped | accepted; build OK; run returns as if the store never ran (write dropped) | **red confirmed** |
| `DEF-286` (🟠) | cross-frame out-slot pass-through accepted + UAF | accepted; ASan heap-use-after-free at the caller's read; container/host run values differ (nondeterministic freed byte) | **red confirmed** |
| `DEF-288` (🟠) | container escape accepted + UAF (return and global spellings) | both accepted; ASan heap-use-after-free on each caller read | **red confirmed** |
| `DEF-306` residual | element-read base emits `lvalue required` | strict build fails both toolchains (`lvalue required as unary '&' operand`) | **red confirmed** |
| `DEF-306` landed | store/return/iterable distribution green control | store-twin program strict-builds and returns the expected 247 (`-9`), ASan-clean; the `u8[4]` return pin is committed (`_impl_test.rs:2808-2829`) | **control confirmed** |
| `DEF-307` | whole-array conditional iterable marks the arms moved | `for (borrow x in (c ? b1 : b2))` over `String[2]` arrays: `E10020: 'b1' was moved and cannot be used`; the `u8[4]` twin accepted and value-correct | **red confirmed** |
| `DEF-308` residual | guarded update operand fails the strict build | `g && ok(f(i++, i))` and `c ? f(i++, i) : 0`: `-Werror=sequence-point` on **both** toolchains | **red confirmed** |
| `DEF-308` landed | plain loop-condition sibling green | `while (ok(f(i++, i)))` strict-builds and computes the left-to-right values | **control confirmed** |
| `DEF-278`/`289`/`296`/`301` | verified fixed (out of scope) | all four re-authored probes: strict build, expected exit values, ASan-clean | **confirmed fixed** |
| `DEF-281` | take/return residual leak; read-position leak | take/return probe: 66 B/6 allocs leaked; four-element read loop: 48 B/4 allocs (single read: 12 B/1 alloc) | **red confirmed** |
| `DEF-282` | two identical reports at one span; cross-site pair | nested mixed return: two identical `E10030` at 12:5; `gfieldcall`: `E10036` at the callee return + `E10033` at the store | **red confirmed** |
| `DEF-283` | internal generated name rendered | `E10030: borrow of '__cl_owned1' does not live long enough` (deferred mixed return) | **red confirmed** |
| `DEF-291` | false rejection of an all-null composite store | out-slot: false `E10030` (`statement temporary`); global twin: `E10033` | **red confirmed** |
| `DEF-285` | runtime-index element store strands the owner / skips the loan | single store: 11 B/1 alloc leaked; loop spelling: 33 B/3 allocs | **red confirmed** |
| `DEF-292` | carrier literal's taken arm leaks | 22 B/2 allocs leaked; run values correct | **red confirmed** |
| `DEF-293` | discarded non-owning arm leaks the root residual | **only with a residual sibling field**: `struct Box { v, w }` → 11 B/1 alloc; the sole-field twin is clean | **red confirmed with a caveat (PF-003)** |
| `DEF-290` | stale literal-carrier loan falsely rejects the backing write | `E10031: cannot modify 'b' while it is shared-borrowed` at the overwrite-following write | **red confirmed** |
| `DEF-297` | bare discarded literal leaks | accepted, but the emitted discard statement is invalid C (`(void)({ __cl_tmp1 });` → `expected ';'` on both toolchains) — not the recorded leak | **red confirmed, different manifestation (PF-003)** |
| `DEF-298` | guarded conditional fixed-array field emits invalid C | `int-conversion` / `missing-braces` on both toolchains | **red confirmed** |
| `DEF-299` | conditional-root field projection emits invalid C | `lvalue required as unary '&' operand` on both toolchains (DEF-298's diagnostic class) | **red confirmed (diagnostic differs from the plan text — PF-003)** |
| `DEF-300` | guarded `len` over a literal view-array field emits an incomplete initializer | `missing-braces` / `missing-field-initializers` on both toolchains | **red confirmed** |
| `DEF-302` | conditional of call-filled literals drops the call arguments | `too few arguments to function` on both toolchains | **red confirmed** |
| `DEF-309` | array fill evaluates per element | `let xs: i32[3] = [; i++];` → exit 3 (three evaluations); the spec requires once (`spec/04-expressions.md:66`) | **red confirmed** |
| `DEF-304`/`DEF-305` | guarded fallback / array-literal element gate emit invalid C | **not reproduced** with re-authored shapes (both built and ran clean); the filed probe sources are gone | **unverified (see below)** |

**Other verified facts.** The batch-I suite pair is green at the head (focused
`cargo test --test …` runs in the pinned container): spec 88/88, impl 52/52 (the plan's
"additions only" baseline). The pinned oracles exist and are green:
`callable_release_integrity_spec_test.rs:1773`
(`should_reject_a_mixed_conditional_view_return`, exactly one `E10030`),
`checker_conformance_spec_test.rs:1857` (the `a compiler-generated owner` phrase),
`module_initializer_integrity_spec_test.rs:1138-1195` (module destinations **retain** the
backing; the tests record release-at-init-end as the defect they close),
`conditional_projection_residue_spec_test.rs` `fixediterowned` (the in-place conditional
iterable). The suite helper supports `RDC_ASAN` (`_spec_test.rs:181-184`). The frozen
rules cited by the fixes were spot-verified: `spec/04` §3 (strict left-to-right; the fill
"evaluates once"), `spec/04:88` (a dereferenced raw pointer is a valid assignment target),
`spec/16:194` BR-8 / `spec/03:238` OW-10 (a loan must not escape its origin),
`spec/14:18-20` (dependent errors are suppressed). The four planned fix hooks exist at the
cited sites (`bind_fixed_array_root`, `pin_update_value`, `contains_update`,
`check_escaping_view_store`, `mark_read_conditional_move`, `argument_owner`,
`emit_fixed_array_return_arm`, `fixed_array_field_needs_stores`, `has_deferred_fill`,
`Statement::StorePointer`/`Assign`). Scope accounting is exact: at the head all 27
in-scope rows are `Backlog ⬜`, `DEF-278`/`289`/`296`/`301` are `Done ✅` with the
recorded evidence, the 1-🔴/5-🟠/21-🟡 mix matches the roadmap severities, the phases
partition the 27 rows without duplicates or omissions (7+4+8+5+3), and each ST case names
one row. No roadmap row exists for this plan itself (tracker rows stop at PLAN-14); the
roadmap-sync step is therefore inert and the dispatch forbids roadmap edits.

## Findings and resolutions

### PF-001: ST-3 does not name the `DEF-286` rejection code — 🟡 MINOR

**Dimension:** 7 Testability (also AR-6)
**Location:** `07-testing-strategy.md` ST-3; `03-01-view-owners-loans-stores.md` §3; AR-6
**Codebase Evidence:** the direct twin of the cross-frame shape — `fill(borrow slot: str?)
{ let b = …; slot = b.v; }` — reports `error E10030: borrow of 'b' does not live long
enough` on both toolchains (probe `p286_direct`); batch-I PF-005 fixed the out-slot code as
`E10030`; the registry template is `spec/14-diagnostics.md:58`.
**The Problem:** ST-3 said "Rejected: one report naming the escaping store (BR-8)" with no
code. A suite pin must assert the code; AR-6's "rejects like its direct twin would" plus
the batch-I resolution fixes it as `E10030` (not `E10033`, whose trigger is a field or a
global — `spec/14-diagnostics.md:61`).
**Options:** A) name `E10030` (the measured direct-twin code); B) leave the code for the
phase to discover (a rejection-shaped case with no determinate oracle).
**Recommendation:** A — grounded in the twin's measured behavior and the batch-I
precedent.
**Decision:** ✅ Resolved — auto-design delegated (policy v1). Applied to ST-3, 03-01 §3.

### PF-002: `DEF-287`'s "release or retain" either/or contradicts the pinned module-destination rule — 🟡 MINOR

**Dimension:** 1 Ambiguities
**Location:** `03-01-view-owners-loans-stores.md` §4; `07-testing-strategy.md` ST-4
**Codebase Evidence:** `module_initializer_integrity_spec_test.rs:1138-1157`
(`should_retain_a_cstring_backing`: "The view keeps the backing for the program's lifetime,
so nothing frees it at the init function's end (red at the review base: the backing was
freed and `main` read freed storage)"); the view-array and struct-view retention tests at
:1160-1195. Probe `p287b`: `let g: str = mk().v;` leaks 11 B/1 allocation at the head and
reads back correctly; a release at init end would dangle `g`.
**The Problem:** The design deferred to "the existing destination rule" but presented both
outcomes as acceptable, and ST-4 repeated the either/or. Read literally, the wrong branch
(release at init end) is the exact defect the committed retention suite closes; ST-4's
expectation had no post-init read-back that would catch it.
**Options:** A) state **retention** for the module's lifetime (the pinned rule) and require
the read-back; B) keep the either/or (records a behavior the oracle rejects).
**Recommendation:** A — the oracle determines the branch; no user ruling needed.
**Decision:** ✅ Resolved — auto-design delegated (policy v1). Applied to 03-01 §4 and
ST-4.

### PF-003: carried red evidence mismatches the planning head for three unprobed rows — 🟡 MINOR

**Dimension:** 13 Codebase Alignment (stale assumptions)
**Location:** `07-testing-strategy.md` ST-10/ST-11/ST-13; `03-02-…md` §3;
`02-current-state.md` P2 `DEF-297`
**Codebase Evidence:** three measured instances at the head:
(i) `DEF-293`: the ST-10 shape over `struct Box { v: String; }` is ASan-clean; only the
residual-sibling shape `{ v, w }` leaks 11 B/1 allocation (probes `p293` vs `p293c`, the
latter from the filed `build/memory-audit/phase4/p06c_de293.clo`);
(ii) `DEF-297`: the filed probe (`build/memory-audit/p5/d297_bare_discarded_literal.clo`)
is accepted but its emitted C fails the strict build on both toolchains —
`(void)({ __cl_tmp1 });` → `error: expected ';' before '}' token` — not the recorded
"42 B/4 allocs leaked";
(iii) `DEF-299`: the filed probe (`z2_fixedarray_cond_field.clo`) fails with `lvalue
required as unary '&' operand`, not DEF-298's `int-conversion`/`missing-braces`.
**The Problem:** AR-3 explicitly did not probe these rows ("carry the roadmap's recorded
reproduction and base/head evidence; the phase red-runs re-verify each"). Two ST inputs
as literally written cannot flip (DEF-293 without the residual field; DEF-297's expected
"run exit 0" against a strict-build failure), and one red diagnostic is misstated. The
spec-author step writes ST cases *before* the red run, so the mismatch costs a red-run
cycle.
**Options:** A) correct the three red columns to the measured manifestations and state the
re-derivation duty; B) leave the batch-I wording and let the red run re-derive.
**Recommendation:** A — the corrections are measured facts; they keep the ST table
writable-to-red.
**Decision:** ✅ Resolved — auto-design delegated (policy v1). Applied to ST-10, ST-11,
ST-13, 03-02 §3, 02-current-state.

### PF-004: ST-20's dual-toolchain assertion vs AR-10's control branch and the suite's single `cc` — 🟡 MINOR

**Dimension:** 7 Testability
**Location:** `07-testing-strategy.md` ST-20; AR-10
**Codebase Evidence:** the batch-I suite harness compiles with one `cc`
(`conditional_projection_residue_spec_test.rs:99-120`) — GCC 12.2 when run via
`make check` in the container, GCC 15.2 on the host; probe `p294` reproduced the split
(container: build + run exit 0; host: `-Werror=sequence-point`). AR-10's control branch
says a control outcome "requires the phase to record why the remaining host warning does
not represent undefined behavior" — i.e. the host leg may legitimately keep warning.
**The Problem:** ST-20 asserted "the strict build passes on the pinned container **and the
host toolchain**". Under the control outcome the host leg cannot be asserted, and in any
outcome one suite run can only assert its own environment's `cc`. The immutability rule
for spec cases needs a determinate assertion.
**Options:** A) the suite asserts the environment's strict build + the run values + the
emission pin; both legs are the AR-10 recorded measurement; B) pre-commit to the fixed
outcome (pre-decides the measurement).
**Recommendation:** A — keeps AR-10's evidence-led branch and the harness reality.
**Decision:** ✅ Resolved — auto-design delegated (policy v1). Applied to ST-20.
(Related: ST-22's "build on both toolchains" is satisfiable because both toolchains reject
the guarded-update spelling today and the fix has no control branch; the same
environment's-leg reality applies and is covered by the phase measurement.)

### PF-005: task file lists omit modules the designs name — 🟡 MINOR

**Dimension:** 4 Completeness / 12 Consistency
**Location:** `99-execution-plan.md` tasks 1.2.1, 3.2.1, 4.2.1
**Codebase Evidence:** `compiler/src/own.rs` exists (67 KB module root; 03-01's integration
points name "`own/program.rs`/`own.rs`"); 03-03's integration points list
`own/statements.rs` ("the arm gates", `prepare_store`/`check_return`); 02's P4 row names
"`own/statements.rs` the store statement lowering", 03-04 lists `ir.rs`
(`Statement::Assign`/`StorePointer` at `ir.rs:302/314`) and `codegen/control.rs`. Probes
confirm those are the store-lowering and statement-shape sites (`Statement::StorePointer`
emitted from `own/expr.rs:1944/1961`).
**The Problem:** the reviewer packets are built from the task file lists; the omitted
modules would leave the store lowering and the deref statement shape outside the declared
surface. Same class as batch-I PF-012/PF-013.
**Options:** A) extend the three lists; B) rely on the runtime-deviation protocol (AR-15)
— which is for genuine surprises, not anticipated sites.
**Recommendation:** A.
**Decision:** ✅ Resolved — auto-design delegated (policy v1). Applied to 1.2.1, 3.2.1,
4.2.1.

### PF-006: ST-1/13/16/17/18 expected-behavior columns under-specify the value read-back — 🟡 MINOR

**Dimension:** 7 Testability
**Location:** `07-testing-strategy.md` ST-1, ST-13, ST-16, ST-17, ST-18
**Codebase Evidence:** the batch-I memory lens's own recorded trap: "the cases read only
the view's length, so a silent UAF hid — the fix-round cases read the backing's bytes"
(batch-I ledger, phase 2). The critical ST-16 pin asserted only "run exit 0; ASan-clean",
so a wrong per-arm copy that happens to exit 0 would pass the acceptance case; the impl
pins (task 3.3.1) cover shapes but the spec cases are the acceptance oracle.
**Options:** A) add the read-back clause to each case; B) rely on the impl pins alone.
**Recommendation:** A — one clause each; the established suite convention.
**Decision:** ✅ Resolved — auto-design delegated (policy v1). Applied (copied-scalar
read-back in ST-1; field/element read-backs in ST-13/16/17/18).

### PF-007: "committed batch-I cases untouched" vs a legitimate emission-delta re-pin — 🟡 MINOR

**Dimension:** 12 Consistency
**Location:** `07-testing-strategy.md` testing-overview bullet; AR-12; the P3 risk table
**Codebase Evidence:** the committed pins `should_copy_a_conditional_fixed_array_return_per_arm`
(`conditional_projection_residue_impl_test.rs:2808-2829`, exact
`memcpy(__cl_ret, &(b1.xs), …)` shapes) and
`should_re_emit_the_fixed_array_programs_byte_identically` (:2853-2865) cover the
fixed-array return surface the `DEF-303` fix touches.
**The Problem:** the plan required the committed cases stay green *and* untouched, but if a
fix legitimately changes a pinned shape the plan named no sanctioned action — the executor
would be stuck between the oracle and the "untouched" wording.
**Options:** A) clarify: stay green as the oracle; a legitimate delta is re-pinned in the
phase with the emission-differential evidence recorded; B) no change (latent blocker on a
legitimate delta).
**Recommendation:** A — makes the legitimate path explicit without weakening the oracle.
**Decision:** ✅ Resolved — auto-design delegated (policy v1). Applied to the
testing-overview bullet.

### PF-008: recorded observations — 🔵 OBSERVATION

**Dimension:** 12 Consistency
**Location:** `00-ambiguity-register.md` AR-6; `03-01-…md` §3/§1;
`03-03-…md` overview; `07-testing-strategy.md` ST-27
**Items:** (a) `03-01` §1's "48 B/4 allocations per evaluation" — measured 12 B per
element read (48 B/4 across the probe's four reads); corrected in place. (b) AR-6/03-01
label the rule "BR-8 (Ch 03 §7)": BR-8 is the Ch 16 rule id
(`spec/16-borrow-rules.md:194`); Ch 03 §7 carries the same rule as OW-10
(`spec/03-ownership.md:238`). (c) `spec/17-codegen.md:59-84` is cited for the "hidden
fixed-array destination ABI / per-arm destination store"; those lines are the declaration
mapping table — the batch-I PF-024 recorded the same observation. (d) ST-27's case name
`iterableark` is probably `iterablemark`.
**Recommendation:** (a) applied; (b)-(d) recorded only — optional wording, no behavioral
consequence; kept to hold the diff minimal.
**Decision:** recorded; (a) applied. No further action required.

## Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|------------------|
| 1 | Ambiguities | 1 (PF-002) | 🟡 |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 0 | — |
| 4 | Completeness Gaps | 1 (PF-005) | 🟡 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 0 | — |
| 7 | Testability | 3 (PF-001, PF-004, PF-006) | 🟡 |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 0 | — |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 2 (PF-007, PF-008) | 🟡 |
| 13 | Codebase Alignment | 1 (PF-003) | 🟡 |

## Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 0 | — |
| MINOR | 7 | all corrected in place (auto-design delegated); edited sections re-read |
| OBSERVATION | 1 | recorded; item (a) applied |

## Plan corrections applied (this scan, all inside the target folder)

| # | File(s) | Correction |
|---|---------|------------|
| PF-001 | `07`, `03-01` | ST-3 and 03-01 §3 name the cross-frame rejection as one `E10030` report (the direct twin's measured code) |
| PF-002 | `03-01`, `07` | DEF-287 stated as **retention** for the module's lifetime (the pinned module-initializer rule); ST-4 gains the post-init read-back |
| PF-003 | `07`, `03-02`, `02` | ST-10 states the residual-sibling requirement; ST-11's red becomes the measured strict-build failure (leak shape re-derived at the red run); ST-13 names the measured diagnostic; 03-02 §3 and 02's `DEF-297` row record the head manifestations |
| PF-004 | `07` | ST-20 asserts the environment's strict build + values + emission pin; both toolchain legs are the AR-10 recording |
| PF-005 | `99` | task 1.2.1 adds `own.rs`; 3.2.1 adds `own/statements.rs`; 4.2.1 adds `own/statements.rs`, `ir.rs`, `codegen/control.rs` |
| PF-006 | `07` | ST-1/13/16/17/18 gain the value/element read-back clauses |
| PF-007 | `07` | the "untouched" wording replaced by the re-pin-with-evidence clause |
| PF-008(a) | `03-01` | the read-position leak stated as 12 B per element read (48 B/4 across the probe's four reads) |

No source, test, roadmap, or other plan folder was touched; nothing was committed.

## Reserved decisions

None outstanding. Every resolution is an eligible testing-strategy/plan-text correction
inside the confirmed scope: no specification or registry text, no new diagnostic code, no
user-visible behavior change beyond the in-scope rows, and no acceptance-criteria change.
The plan's own runtime stop-gates stand as designed and are **not** pre-decided here:
AR-6's `DEF-286` park-if-over-rejecting gate and AR-10's `DEF-294` fixed-vs-control
measurement both return evidence-based outcomes the phase records, and the register
already labels them as reserved-if-triggered.

## Could not verify (with reasons)

| Item | Reason |
|------|--------|
| `DEF-304` and `DEF-305` reds | The filed probe sources were removed with the planning scratch (`build/probe-batch-j/` is absent); re-authored shapes built and ran clean, so the exact filed spellings could not be reproduced. The batch-I evidence stands; ST-17/ST-18 must be written from the phase red-run (the plan's P3 step already re-verifies). |
| AR-3's "the emission is byte-identical to the phase-4 tree where the row was filed" (`DEF-294`) | The phase-4 tree is not at the head; the base/head identity is carried roadmap evidence. The split itself is verified. |
| 03-03 §6's "0 hits across 4593 corpus runs" (the guards statement-path branch) | Instrumentation-level claim from batch-I; not re-run here. |
| The AR-3 probe sources/logs themselves | Removed at planning close as declared; every probed row was independently re-probed instead. |
| Pinned-oracle suite runs beyond the focused pair | `make check`/`make verify` were not run in full (out of a review's scope); the focused residue suites and the code reads above ground the plan. |

## Verdict

**✅ PREFLIGHT PASSED WITH NOTES** — the plan is sound and executable at
`a462084`. All scope, status, severity, phase-partition, dependency, and oracle claims
verified against the actual compiler; every checkable red/control reproduced with the one
documented exception noted above. No critical or major findings. Seven minor findings were
corrected in place under the standing auto-design mandate and the edited sections re-read;
one observation is recorded, with its one factual item corrected. The corrections do not
change the plan's scope, acceptance criteria, or any statement of user-visible behavior.

**Required plan corrections:** none outstanding — the corrections listed above are applied.
