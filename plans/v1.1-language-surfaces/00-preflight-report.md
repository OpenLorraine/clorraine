# Plan Preflight: v1.1 Language Surfaces

> **Status**: ✅ PREFLIGHT PASSED WITH NOTES — 19 findings (3 critical, 14
> major, 2 minor); all resolved in the revised execution plan
> **Iteration**: 1 (first scan; the plan was revised in the same pass)
> **Artifact**: implementation plan at `plans/v1.1-language-surfaces/`
> (`00-index.md`, `01-requirements.md`, `02-current-state.md`,
> `03-01-box.md`, `03-02-freedby.md`, `03-03-hook-trampolines.md`,
> `07-testing-strategy.md`, `99-execution-plan.md`)
> **Codebase Grounded**: 30+ source files and test-harness files examined
> **Last Updated**: 2026-11-08
> **Same-session review**: ⚠️ the plan was authored in this session. An
> independent challenger (general subagent, `ses_e4721693affe2U2tCCZwI4yV9g`)
> reviewed it against the code; its findings are incorporated below.

## Codebase Context Summary

**Tech Stack:** Rust workspace, C99 runtime shims, embedded Clorraine stdlib,
Docker SDK (`m68k-amigaos-gcc`, vbcc, `vamos`), host `cc` strict builds.

**Architecture:** resolve → type table → sema → ownership/borrow IR →
single-TU C emission. The emitter has `Loaded`, `SemaOutput`, and
`OwnershipOutput`; it does not have `ProgramInfo`. Test files are separate
integration crates with duplicated helpers; there is no shared support module.

**Key Files Examined:** `compiler/src/ffi.rs`, `own.rs`, `own/program.rs`,
`own/call.rs`, `own/lower.rs`, `own/generics.rs`, `types/*`, `sema/*`,
`codegen/{drop,program,calls,types}.rs`, `resolve/mod.rs`, `stdlib.rs`,
`spec/check.sh`, `compiler/tests/{amiga_assets_impl,amiga_exec_spec}_test.rs`,
`plans/00-roadmap.md`.

**Reference Verification:** 52 task references mapped; 44 verified; 8 corrected
(the E10105 check site, the `boxAlloc` lowering site, the `mentions_type_param`
site, the `own/generics.rs` arms, the shared test support, the appendix
section, the `OwnershipOutput` deleter map, and the command spellings).

## Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|---|---|---|
| 1 | Ambiguities | 0 | — |
| 2 | Implicit Assumptions | 2 | 🟠 major |
| 3 | Logical Contradictions | 2 | 🔴 critical |
| 4 | Completeness Gaps | 5 | 🔴 critical |
| 5 | Dependency Issues | 3 | 🟠 major |
| 6 | Feasibility Concerns | 4 | 🟠 major |
| 7 | Testability | 1 | 🟠 major |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 1 | 🟡 minor |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 1 | 🔴 critical |
| 12 | Consistency | 1 | 🟡 minor |
| 13 | Codebase Alignment | 3 | 🟠 major |

## Summary by Severity

| Severity | Count | Status |
|---|---|---|
| CRITICAL | 3 | all resolved |
| MAJOR | 14 | all resolved |
| MINOR | 2 | all resolved |
| OBSERVATION | 0 | — |

---

## Critical Findings

### PF-PLAN-001: No task implements `E10105` 🔴 CRITICAL

**Dimension:** 4 — Completeness / 13 — Codebase Alignment
**Location:** 1.1.1, 1.2.x, 1.3.3, 6.1.2.
**Codebase Evidence:** `ffi::check_items` has no `TypeTable` (`ffi.rs:21`);
`ProgramInfo` stores none (`own/program.rs:75-95`); the caller has
`sema.table` (`own.rs:286-287`); `signature_param_type` uses alias-blind
`resolve_ty` (`ffi.rs:106-108`). AR-66 had no task.
**Problem:** R6.1 was unmapped; the BX-16 spec case and the compile-fail golden
could not pass.
**Resolution:** Added 1.2.9 (table/module/alias/recursion check) and 1.2.11
(registry + compile-fail files); 1.1.1 now covers Box-in-ffi and callback
nesting.

### PF-PLAN-002: Phase 0 baseline is dirty and predates the revised docs 🔴 CRITICAL

**Dimension:** 3 — Contradiction / 11 — Ordering
**Location:** 0.1.1-0.1.3, Phase 0 baseline.
**Codebase Evidence:** `git status` shows the revised requirements/register
modified and the plan untracked after `8a91bc5`; `05-findings.md` did not
exist. `00-index.md` linked to the untracked plan.
**Problem:** Phase 1 would build on an uncommitted baseline.
**Resolution:** 0.1.3 creates the findings ledger; 0.1.4 commits the revised
plan documents and records the new baseline. Phase 1 starts only with a clean
tree. (Completed at the end of this preflight.)

### PF-PLAN-003: Phase 4 accepts a capturing `borrow` hook before emission exists 🔴 CRITICAL

**Dimension:** 3 — Contradiction / 11 — Ordering
**Location:** 4.2.2/4.2.3 vs 5.3.x.
**Codebase Evidence:** the base `check_ffi_arguments` accepts a capturing
lambda and the emitter writes `ClClosure_N` where a function pointer is
expected (`sema/call.rs:1546-1603`; `02-current-state.md` §3). Accepting the
`borrow` hook in Phase 4 would create the invalid-C form before the
trampoline/wrapper exists.
**Problem:** a commit boundary would accept source that emits invalid C.
**Resolution:** Phase 4 now rejects **all** capturing `func` arguments,
including `borrow`, and adds the ABI check. Phase 5.2.3 enables the `borrow`
acceptance and the `HookCall` record only after the lowering/emission tasks.

## Major Findings

### PF-PLAN-004: Phase 2.2.3 emits the alloc helper before it is defined and names the wrong lowering file 🟠 MAJOR

**Dimension:** 5 — Dependency / 13 — Codebase Alignment
**Location:** 2.2.2/2.2.3.
**Codebase Evidence:** `boxAlloc<T>()` is an ordinary `Call`, lowered in
`own/call.rs::lower_intrinsic_call` (`own/call.rs:780-860`), not
`own/lower.rs`; an unhandled intrinsic emits `0` (`codegen/calls.rs:501`). The
`cl_box_alloc__<key>` definition was in 2.3.2.
**Resolution:** 2.2.2 now adds the intrinsic declaration, sema/lowering/emitter
arm, and the `cl_box_alloc__<key>` definition together; the file list is
`own/call.rs`. The phase commits only at 2.3.4, after the drop glue exists.

### PF-PLAN-005: The order red evidence cannot run on the starting tree 🟠 MAJOR

**Dimension:** 7 — Testability
**Location:** 5.1.3.
**Codebase Evidence:** the base accepts a capturing hook but emits an
incompatible `ClClosure_N`, so the C build fails before the order bug can run.
The design's reproducible order proxy is the generic-bound
`hook(bump(n), [n] ...)` case (AR-37a).
**Resolution:** 5.1.3 records the base `ClClosure` C mismatch and the generic
proxy; the hook order assertion is validated by a scratch revert after 5.2.1
(testing-strategy step 4), not by a base run.

### PF-PLAN-006: Phase 1.1.2's red evidence is factually wrong 🟠 MAJOR

**Dimension:** 2 — Implicit Assumptions
**Location:** 1.1.2.
**Codebase Evidence:** `resolve_name` predeclares only `String`
(`resolve/mod.rs:465-470`), so every bare `Box` spelling is `E10219` on
`a7dbdd5`; a user `struct Box<T>` is accepted. The accepted `Box {}`/`Box(x)`/
`Box<str>` cases can only appear after 1.2.5 resolves the name.
**Resolution:** 1.1.2 now records E10219/user-Box red on the base and the
later red cases against the first implementation attempt.

### PF-PLAN-007: Code-site misattributions and missing `own/*` arms 🟠 MAJOR

**Dimension:** 13 — Codebase Alignment
**Location:** 1.2.1, 1.2.6, 1.2.7.
**Codebase Evidence:** `mentions_type_param` is in `own/lower.rs:6934`;
`is_concrete`/`record_struct_type` are in `own/generics.rs:150-195`; the arity
check belongs in `sema/items.rs:318-333`, not `sema/call.rs`.
**Resolution:** 1.2.1/1.2.6/1.2.10 now cite the real files and add
`own/generics.rs`, `own/place.rs`, `own/expr.rs`, `own/statements.rs`, and
`own/instance_check.rs`; an impl pin covers `Box<Wrap<i32>>`.

### PF-PLAN-008: Several BX/X1/HK cases had no task 🟠 MAJOR

**Dimension:** 4 — Completeness / 7 — Testability
**Location:** 1.1.1, 2.1.1, 3.1.1, 4.1.1, 5.1.2.
**Codebase Evidence:** the testing strategy lists BX-1..BX-23,
X1-1..X1-16, HK-1..HK-17; the design matrices add `static str`,
`Array<str>`, raw-cast, ffi-callback, `Wrap`, `[borrow x]`, short-circuit, and
capture-less generic-body cases.
**Resolution:** The task lists now name the missing cases (BX-10/BX-22/BX-23,
X1-3/9/13/16, HK-6/14/15, hosted-amiga Box, m68k layout); the coverage matrix
maps each requirement to tasks.

### PF-PLAN-009: The emitter cannot read the stored deleter identity 🟠 MAJOR

**Dimension:** 5 — Dependency
**Location:** 3.2.1/3.2.3.
**Codebase Evidence:** `OwnershipOutput` carries only functions,
monomorphization, and init lambdas (`own.rs:77-89`); the emitter has no
`ProgramInfo`; `deleter_symbol` scans `loaded` (`codegen/drop.rs:273-298`).
**Resolution:** 3.2.1 adds a `deleter_identities` map to
`OwnershipOutput`; 3.2.3 consumes it. The file list includes `own.rs` and
`codegen/drop.rs`.

### PF-PLAN-010: `ProgramInfo.functions` cannot detect an ambiguous simple name 🟠 MAJOR

**Dimension:** 2 — Implicit Assumptions
**Location:** 3.2.2.
**Codebase Evidence:** the map is filled with `or_insert` and keyed by simple
name (`own/program.rs:78-79,113-114`); `user_function` returns the first entry
(`own/program.rs:170-208`). R7.2 requires ambiguity to be invalid.
**Resolution:** 3.2.2 now counts candidates across `loaded.modules` (or builds
a multimap) and rejects ambiguity; a test covers it.

### PF-PLAN-011: The extra-C build helper is assigned to the wrong crate 🟠 MAJOR

**Dimension:** 6 — Feasibility
**Location:** 5.1.1.
**Codebase Evidence:** each `compiler/tests/*.rs` is a separate integration
crate with duplicated helpers; there is no shared support module. The host
behavior and `E11215` cases live in the spec crate.
**Resolution:** 5.1.1 adds `compiler/tests/support/mod.rs` with the scratch,
compiler, extra-C, and `vamos` helpers; both suites include it.

### PF-PLAN-012: The spec chapter list is wrong and incomplete 🟠 MAJOR

**Dimension:** 13 — Codebase Alignment
**Location:** 6.1.1.
**Codebase Evidence:** `appendix-stdlib.md` §18 is Audio, not Box; the staged
Box entry is §12. Ch 01, 02, 15 have the reserved-name/type/profile rules;
Ch 16 §4 has the elision candidate.
**Resolution:** 6.1.1 now updates Ch 01, 02, 03, 06, 07, 10, 11, 12, 14, 15,
16, 17, the appendix §12 plus a new Box/std.mem section, and lineage.

### PF-PLAN-013: The spec tests were scheduled after the implementation 🟠 MAJOR

**Dimension:** 11 — Ordering
**Location:** 6.1.2 vs 1.2.7/1.2.9.
**Codebase Evidence:** `spec/check.sh:90-186` requires the compile-fail files
for every live code; the implementations land in Phase 1.
**Resolution:** 1.2.11 now adds the `E10227`/`E10105` registry entries and
compile-fail files with the implementation; 5.3.5 adds the `E11215` exclusion
and Rust case; 6.1.2 only updates the prose/help and confirms the check.

### PF-PLAN-014: The roadmap row is updated only at the end 🟠 MAJOR

**Dimension:** 11 — Ordering
**Location:** 6.1.3.
**Codebase Evidence:** R13.6 requires RD Drafted → Plan Preflighted →
Executing → Done; the plan had no Phase 0 roadmap task.
**Resolution:** 0.1.4 updates PLAN-19 to Plan Preflighted; the execution rule
requires a stage update at every phase boundary; 6.1.3 records Done.

### PF-PLAN-015: Several Verify lines are not real project commands 🟠 MAJOR

**Dimension:** 7 — Testability
**Location:** Phase 1-6 Verify lines.
**Codebase Evidence:** the real targets are `make check`, `make asan`,
`make ubsan`, `make corpus-amiga`, `make differential`, `make bench-shape`,
`make verify`; there is no "scoped make asan" or "m68k cross-build/vamos"
make target. Focused runs are `cargo test --test ...`.
**Resolution:** Every Verify line now uses a real command or
`cargo test --test v11_language_surfaces_{spec,impl}_test`, with
`RDC_ASAN=1` for the memory runs.

### PF-PLAN-016: Phase reviews were deferred to the end 🟠 MAJOR

**Dimension:** 6 — Feasibility / quality policy
**Location:** 7.1.6 only.
**Codebase Evidence:** AGENTS.md requires per-phase correctness + memory
reviews for ownership/drop, plus emission-integrity for IR/codegen/symbol
changes and semantics for language-rule changes.
**Resolution:** 1.3.3 (type/emission), 2.3.3 (Box drop), 3.3.3 (X1 release),
5.4.4 (hook ABI/codegen/semantics), and 7.1.6 (final cross-phase) now carry
the reviews; 7.1.7 records the authorized ruling.

### PF-PLAN-017: The Box layout test would require a Box value too early 🟠 MAJOR

**Dimension:** 9 — Edge Case
**Location:** 1.3.1.
**Codebase Evidence:** a `StructWithBox` value triggers `needs_drop` and
emits `cl_box_drop__...`, which does not exist until Phase 2.
**Resolution:** 1.3.1 now uses type-only constants
(`const N: usize = sizeof(StructWithBox)`, `offsetof`, an extent) and does not
construct a `StructWithBox` value.

## Minor Findings

### PF-PLAN-018: Task/AR/document inconsistencies 🟡 MINOR

**Dimension:** 12 — Consistency
**Location:** 4.2.3, AR-24d/AR-68, AR-66, R13.2, Success Criterion 6.
**Resolution:** 4.2.3 now cites `sema/call.rs`; the duplicate AR-24d was
removed (AR-68 owns the layout decision); 1.2.9 references AR-66; R13.2 and
Success Criterion 6 now say "two new compile-fail tests + the `E11215`
exclusion and Rust case".

### PF-PLAN-019: The plan header and phase counts disagreed 🟡 MINOR

**Dimension:** 12 — Consistency
**Location:** header, phase table, checkboxes.
**Resolution:** The plan now reads `2/73` with per-phase counts 4/16/11/9/5/16/4/8
matching the checkbox lines.

## Verdict

All critical and major findings are resolved in the revised execution plan.
The two minor findings are resolved by wording/count corrections. The plan is
specification-first, every R1-R13 requirement maps to at least one task, and
the phase boundaries keep the tree buildable and reviewed.

**Verdict:** ✅ PREFLIGHT PASSED — all 19 findings resolved.

## Applied Revisions

| Finding | Tasks changed | Revision |
|---|---|---|
| PF-PLAN-001 | 1.1.1, 1.2.9, 1.2.11 | E10105 check + spec/golden |
| PF-PLAN-002 | 0.1.3, 0.1.4 | ledger + revised docs commit |
| PF-PLAN-003 | 4.2.2, 5.2.3 | reject capturing until emission |
| PF-PLAN-004 | 2.2.2 | alloc helper + correct lowering file |
| PF-PLAN-005 | 5.1.3 | base C mismatch + generic proxy |
| PF-PLAN-006 | 1.1.2 | corrected base red evidence |
| PF-PLAN-007 | 1.2.1, 1.2.6, 1.2.10 | real sites + own arms |
| PF-PLAN-008 | 1.1.1, 2.1.1-2.1.3, 3.1.1, 4.1.1, 5.1.2 | missing cases |
| PF-PLAN-009 | 3.2.1, 3.2.3 | OwnershipOutput deleter map |
| PF-PLAN-010 | 3.2.2 | counted ambiguity |
| PF-PLAN-011 | 5.1.1 | shared support module |
| PF-PLAN-012 | 6.1.1 | corrected chapter list |
| PF-PLAN-013 | 1.2.11, 5.3.5, 6.1.2 | spec-first ordering |
| PF-PLAN-014 | 0.1.4, execution rule | roadmap stage |
| PF-PLAN-015 | all Verify lines | real commands |
| PF-PLAN-016 | 1.3.3, 2.3.3, 3.3.3, 5.4.4, 7.1.6 | per-phase reviews |
| PF-PLAN-017 | 1.3.1 | type-only layout test |
| PF-PLAN-018 | 4.2.3, register | consistency |
| PF-PLAN-019 | header/table | counts |
