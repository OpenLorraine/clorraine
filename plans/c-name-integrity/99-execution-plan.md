# Execution Plan: C Name Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-28 (preflight amendments)
> **Progress**: 0/44 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

The plan has five phases:
1. The name list and source-name escaping for locals and parameters (#61).
2. Fields, captures, and enum constants (#61, the rest of "every").
3. Generated symbols (#64).
4. Line mapping (ASM-WS3-005).
5. Driver paths, docs, and the final gate (ASM-WS3-006).

Every feature phase follows the specification-first order. Each phase's docs step changes only
documentation.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Name list; locals and parameters | 10 |
| 2 | Fields, captures, enum constants | 10 |
| 3 | Generated symbols | 8 |
| 4 | Line mapping | 8 |
| 5 | Driver paths, docs, final gate | 8 |

**Total: 44 tasks across 5 phases.**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated
>    stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks
> current means progress is invisible after crashes, context resets, or session handoffs.

Rules for every task:
- Commits (`--auto-commit`): one commit and push per phase through the git-commit skill in push
  mode, after the phase gate and review, so `main` never carries a red spec test. Gates run with
  `IMAGE=clorraine-sdk:windows-cli`.
- Known limitations (AGENTS.md prime directive): before each phase commit, re-read
  `docs/known-limitations.md` and `site/guide/known-limitations.md` against the phase's change,
  make the edits [03-04 §Docs](03-04-driver-paths-and-docs.md#docs-ar-14-ar-22-ar-23-agentsmd-prime-directive)
  lists for that phase (and any other it makes stale), and otherwise write "known limitations:
  unchanged" in the commit message.
- Source, tests, and doc comments restate rationale in plain language. They never cite AR/ST/ASM/
  DEF/PF identifiers or `plans/` (AGENTS.md).
- Probe and scratch output goes under `build/tmp`, under `timeout` and `ulimit -f 1048576`.
- Review dispatch per phase (AGENTS.md, codegen and symbol-name changes): correctness reviewer,
  memory-safety auditor, emission-integrity auditor; plus `amiga-platform-reviewer` in Phases 1
  and 4 ([00-index §Specialist Agents](00-index.md#specialist-agents)). Each packet carries the
  blast-radius and test-integrity steps.
- Pins: impl and unit pins that change are updated as intended and listed in the phase notes; a
  spec-test pin conflict stops the phase for a ruling (AR-25).
- rc24 completeness command (Phases 1 and 5): extract or copy the rc24 root under `build/rc24/`
  (gitignored, inside the container's `/work` mount), then run
  `CLORRAINE_RC24_ROOT=/work/build/rc24/<root> make -C docker exec IMAGE=clorraine-sdk:windows-cli PASSTHROUGH=CLORRAINE_RC24_ROOT CMD="cargo test --test c_name_integrity_impl_test"`.
  The test fails, not skips, when the variable is set and the root is unusable; if the rc24 GCC
  does not run inside the image, mark the task `[!]` and stop for the user.

---

## Phase 1: Name list; locals and parameters

> **Phase baseline tree**: _(recorded by exec-plan)_
> **Reasoning**: xhigh — a cross-leg data extraction plus a naming change every program's C goes through.

### Step 1.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-1..ST-6, ST-11, ST-25 · [03-01](03-01-source-names.md) · AR-2, AR-5, AR-17..AR-19

- [ ] 1.1.1 [spec-author] Write the fixtures (probes 340 and 341 copies; the ST-3..ST-6, ST-11, and ST-25 programs) and the spec tests for ST-1..ST-6, ST-11, ST-25 with the three-profile runner and the vbcc build (skip, one retry on `No config file!`, exit status only), SDK skip — `compiler/tests/c_name_integrity_spec_test.rs`, `compiler/tests/fixtures/c_name_integrity/`
- [ ] 1.1.2 Red phase: `cargo test --test c_name_integrity_spec_test`; record each case failing except ST-11 and ST-25 (guards)

### Step 1.2: Implementation

**Reference**: [03-01 §Proposed Changes](03-01-source-names.md#proposed-changes) · AR-5..AR-7, AR-17..AR-19, AR-26

- [ ] 1.2.1 Write the sample program and run the AR-19 extraction (macros `-dM`, functions `-aux-info`, typedefs/objects/enumerators by the 03-01 scan) for every leg in the SDK image and against the rc24 root; record the exact commands in the notes below; write the sorted union — `compiler/src/codegen/c_names.txt`
- [ ] 1.2.2 Implement the escape classes, the lazily parsed list with binary search, and `escape` (`_0` rule, `""` → `_0`); retire the `cl_` keyword prefix — `compiler/src/codegen.rs`
- [ ] 1.2.3 Add `generated: bool` to `ir::Local` (set from the binding; `Builder::add_local` defaults to `false`); add the source-name entry point and route non-generated locals (`function.rs:197-203,296-304`) and every parameter list through it, with the counter skipping class members; keep generated locals on plain `unique_local` and the not-in-table fallbacks on plain `sanitize` — `compiler/src/{ir,own,codegen}.rs`, `compiler/src/own/lower.rs`, `codegen/{expr,function}.rs`
- [ ] 1.2.4 Green phase: ST-1..ST-6, ST-11, ST-25 pass

### Step 1.3: Implementation tests and hardening

**Reference**: [07 §Implementation Tests](07-testing-strategy.md#implementation-tests-edge-cases-internals)

- [ ] 1.3.1 Write the completeness test (every available leg, the scanner's recall against `-aux-info`, rc24 with `CLORRAINE_RC24_ROOT`, sorted, unique, `_0` invariant), the escape-function tests (incl. `""`, triple shadowing), and the generated-name tests (incl. generated vs source `__cl_owned0`, every source local in the table) — `compiler/tests/c_name_integrity_impl_test.rs`
- [ ] 1.3.2 Update the pins that intentionally change (`codegen.rs:1300-1307`, the doc comment at `codegen_ownership_impl_test.rs:381`, any stdlib parameter spelling such as `index`, `socket`) and list them below — `compiler/tests/`, `compiler/src/codegen.rs`
- [ ] 1.3.3 Known limitations: re-evaluate both files; update `docs/known-limitations.md` §7 for escaped locals and parameters — `docs/known-limitations.md`, `site/guide/known-limitations.md` if stale
- [ ] 1.3.4 Phase gate: `make check`; the rc24 completeness command

**Deliverables**:
- [ ] ST-1..ST-6, ST-11, ST-25 green; the list committed and complete for every leg

**Verify (gate)**: `make check` and the rc24 completeness command

---

## Phase 2: Fields, captures, enum constants

> **Phase baseline tree**: _(recorded by exec-plan)_
> **Reasoning**: high — many emission sites must spell one member name the same way.

### Step 2.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-7..ST-10 · [03-01](03-01-source-names.md) · AR-2, AR-15, AR-16, AR-18

- [ ] 2.1.1 [spec-author] Add the ST-7..ST-10 fixtures and spec tests (ST-10 in the default and `--release` builds) — `compiler/tests/c_name_integrity_spec_test.rs`, `compiler/tests/fixtures/c_name_integrity/`
- [ ] 2.1.2 Red phase: run the declared spec file; record ST-7..ST-10 failing

### Step 2.2: Implementation

**Reference**: [03-01 §Source-name entry point](03-01-source-names.md#source-name-entry-point-ar-17) · AR-16, AR-18

- [ ] 2.2.1 Add the member-spelling helper; spell struct members through it at the definition (`program.rs:886-889`, names at `:954`) and the expression sites (`expr.rs:66-70,205-210,583-636,711-735`) — `compiler/src/codegen/{program,expr}.rs`
- [ ] 2.2.2 Spell members through the helper at every other site (drops, guards, `offsetof`, `program.rs:1468,1551,2781`, `function.rs:3148`); record the grep of `\.\{|->\{|offsetof` over `compiler/src/codegen/` and its result in the notes as the site audit — `compiler/src/codegen/{drop,guards,calls,function,program}.rs`
- [ ] 2.2.3 Spell capture fields through the helper at the environment definition and every capture read — `compiler/src/codegen/{program,expr,function}.rs`
- [ ] 2.2.4 Emit each enum constant use as its parenthesized folded value and remove the `#define` loop; update the seven enum-spelling pins (`compiler_robustness_impl_test.rs:449-450,465-466,576-577`, `checker_conformance_impl_test.rs:1510`) — `compiler/src/codegen/{program,expr}.rs`, `compiler/tests/{compiler_robustness,checker_conformance}_impl_test.rs`
- [ ] 2.2.5 Green phase: ST-1..ST-11, ST-25 pass

### Step 2.3: Implementation tests and hardening

- [ ] 2.3.1 Add impl tests: keyword-named and class-3 fields through every member form (07 list) built with the strict host and m68k flags; a capture through each capture site; update and list pins — `compiler/tests/c_name_integrity_impl_test.rs`
- [ ] 2.3.2 Known limitations: re-evaluate both files; update §7 for fields, captures, and enum constants emitted as values
- [ ] 2.3.3 Phase gate: `make check`

**Verify (gate)**: `make check`

---

## Phase 3: Generated symbols

> **Phase baseline tree**: _(recorded by exec-plan)_
> **Reasoning**: high — symbol reservation with loops that must terminate and stay exact.

### Step 3.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-12..ST-17, ST-26, ST-27 · [03-02](03-02-generated-symbols.md) · AR-3, AR-8, AR-20

- [ ] 3.1.1 [spec-author] Add the ST-12..ST-17, ST-26, ST-27 fixtures and spec tests (multi-file sources passed together to `clc`) — `compiler/tests/c_name_integrity_spec_test.rs`, `compiler/tests/fixtures/c_name_integrity/`
- [ ] 3.1.2 Red phase: run the declared spec file; record ST-12..ST-16, ST-26, ST-27 failing (ST-16 (b) with the duplicate `cl_lambda_0__i32`), ST-17 passing (guard)

### Step 3.2: Implementation

**Reference**: [03-02 §Proposed Changes](03-02-generated-symbols.md#proposed-changes) · AR-20

- [ ] 3.2.1 Build the instance symbol map before emission in monomorphization order (earlier instances, helper families, shim set, fixed prelude helpers) and route the four sites through it — `compiler/src/codegen.rs`, `codegen/{program,expr,function}.rs`
- [ ] 3.2.2 Add the trigger families and the fixed prelude names; build the exact generated sets (zero helpers per emitted struct unit plus `cl_zero_ClStr`); make `is_taken_symbol`, the instance loop, and `check_ffi_collisions` use them — `compiler/src/codegen.rs`
- [ ] 3.2.3 Green phase: ST-1..ST-17, ST-25..ST-27 pass

### Step 3.3: Implementation tests and hardening

- [ ] 3.3.1 Add impl tests: every generated family in the exact sets, a `__user` candidate equal to a real helper (an unmoved `struct Rec__user` beside `module zero; func ClRec`), an `ffi` literal equal to an instance or an emitted helper (E11101), loop termination; update and list pins — `compiler/tests/c_name_integrity_impl_test.rs`
- [ ] 3.3.2 Known limitations: re-evaluate both files; add the §7 instance and helper rows and narrow the §11 "Hook symbol names" and "Box helper names" rows
- [ ] 3.3.3 Phase gate: `make check`

**Verify (gate)**: `make check`

---

## Phase 4: Line mapping

> **Phase baseline tree**: _(recorded by exec-plan)_
> **Reasoning**: medium — directive placement plus a new profile accessor for the suffix.

### Step 4.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-18..ST-20 · [03-03](03-03-line-mapping.md) · AR-9, AR-21

- [ ] 4.1.1 [spec-author] Add the ST-18..ST-20 spec tests — `compiler/tests/c_name_integrity_spec_test.rs`
- [ ] 4.1.2 Red phase: run the declared spec file; record ST-18..ST-20 failing

### Step 4.2: Implementation

**Reference**: [03-03 §Proposed Changes](03-03-line-mapping.md#proposed-changes) · AR-21

- [ ] 4.2.1 Add the suffix-pieces accessor beside the unchanged `shim_suffix()` (same `include_str!` constants, join-equality unit test) and write `#line 1 "compiler/runtime/<file>"` before each named piece in the emitter's suffix step — `compiler/src/profile.rs`, `compiler/src/codegen.rs`
- [ ] 4.2.2 Record the lambda span on `ClosureSite` and the hook lambda span on its hook call site; write the per-item `#line` for initializers, functions and instances, lifted lambdas, hook support, and the entry stub — `compiler/src/sema/mod.rs`, `compiler/src/types/mod.rs`, `compiler/src/codegen/{program,function}.rs`
- [ ] 4.2.3 Green phase: ST-1..ST-20, ST-25..ST-27 pass; `stage_spec_test`, `windows_cli_spec_test`, and `amiga_optional_spec_test` still pass

### Step 4.3: Implementation tests and hardening

- [ ] 4.3.1 Add the every-item-has-a-directive impl test over one corpus program per profile; update and list pins — `compiler/tests/c_name_integrity_impl_test.rs`
- [ ] 4.3.2 Known limitations: re-evaluate both files; add the runtime-file `#line` mapping and its debugger residual to §7
- [ ] 4.3.3 Phase gate: `make check`; `make corpus-amiga`

**Verify (gate)**: `make check && make corpus-amiga`

---

## Phase 5: Driver paths, docs, final gate

> **Phase baseline tree**: _(recorded by exec-plan)_
> **Reasoning**: medium — a contained driver change, docs, then the full matrix.

### Step 5.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-21..ST-24 · [03-04](03-04-driver-paths-and-docs.md) · AR-10, AR-24

- [ ] 5.1.1 [spec-author] Add the ST-21..ST-24 driver spec tests (working folder set as in `clorraine_driver_spec_test.rs:58-66`) — `compiler/tests/c_name_integrity_spec_test.rs`
- [ ] 5.1.2 Red phase: run the declared spec file; record ST-21..ST-23 failing, ST-24 passing (guard)

### Step 5.2: Implementation

**Reference**: [03-04 §Driver path rewrite](03-04-driver-paths-and-docs.md#driver-path-rewrite-ar-24) · AR-24

- [ ] 5.2.1 Add the pure relative-path function (taking `HostOs`) and rewrite source paths inside the working folder in the driver for `build`, `run`, and `emit` — `compiler/src/clorraine/platform.rs`, `compiler/src/clorraine/`
- [ ] 5.2.2 Green phase: ST-1..ST-27 pass

### Step 5.3: Implementation tests, docs, and the plan gate

- [ ] 5.3.1 Add the driver impl tests (both host families' separators, canonicalization failure, outside folder, a relative path holding a quote or backslash still escaped in `#line`) — `compiler/tests/c_name_integrity_impl_test.rs`
- [ ] 5.3.2 Known limitations and notices: re-evaluate both files; add the driver rule and the residual list to §7, and the provenance note — `docs/known-limitations.md`, `site/guide/known-limitations.md` if stale, `THIRD_PARTY_NOTICES.md`
- [ ] 5.3.3 Final gate: `make check`, `make asan`, `make ubsan`, `make corpus-amiga`, `make bench-shape`, `make differential`, `make bench-compile`, `make toolchain-check TOOLCHAIN=<rc24 root>`, the rc24 completeness command
- [ ] 5.3.4 Close out: roadmap rows (plan, DEF-589, DEF-592) and the plan status; issues are closed only with the user's approval — `plans/00-roadmap.md`, `plans/c-name-integrity/00-index.md`

**Deliverables**:
- [ ] ST-1..ST-27 green; docs and notices updated; final gate green

**Verify (gate)**: `make check && make asan && make ubsan && make corpus-amiga && make bench-shape && make differential && make bench-compile && make toolchain-check TOOLCHAIN=<rc24 root>` and the rc24 completeness command

---

## Dependencies

```
Phase 1 (list, escape, entry point)
    ↓
Phase 2 (fields, captures use escape; enum constants as values)
    ↓
Phase 3 (generated symbols)
    ↓
Phase 4 (line mapping)
    ↓
Phase 5 (driver, docs, final gate)
```

Phases 3 and 4 do not depend on Phase 2's code, but run in order so each phase's emission diff is
reviewed against a stable base.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ The AR-12 final gate passes
3. ✅ No warnings or errors on the strict legs
4. ✅ No dead code: `is_c_keyword` and the `cl_` prefix path removed or folded into the classes; the enum `#define` loop removed
5. ✅ No user text reaches a shell; absolute paths stay out of driver-built C
6. ✅ `docs/known-limitations.md` (and `site/guide/known-limitations.md` where stale) and `THIRD_PARTY_NOTICES.md` updated, phase by phase
7. ✅ Each phase reviewed per AGENTS.md dispatch
8. ✅ Post-completion project re-analysis (exec-plan)

## Notes

- Extraction commands (1.2.1): _(recorded during execution)_
- Member-site audit (2.2.2): _(recorded during execution)_
- Pins updated: _(listed per phase during execution)_
