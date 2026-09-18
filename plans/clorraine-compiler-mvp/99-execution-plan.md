# Execution Plan: Clorraine Compiler MVP

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-18 11:59
> **Progress**: 18/84 tasks (21%)
> **CodeOps Artifact Schema**: 1

## Overview

Nine phases: the validation corpus (Task 0), the four accepted spikes, then
the compiler stages — front end, name resolution, types/constants, ownership
and FFI, codegen and runtime shims, the standard-library slice — and finally
the cross-toolchain suites and packaging. Every compiler phase follows the
mandatory specification-first ordering: spec tests → red phase → implement →
green phase → impl tests → full verify. The golden files under `spec/tests/`
are the primary spec-test data (AR-23, AR-44).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 0 | Validation corpus (Task 0) | 7 |
| 1 | Accepted Phase 2 spikes | 16 |
| 2 | Front end, diagnostics, CLI, harness skeleton | 10 |
| 3 | Name resolution and modules | 6 |
| 4 | Types, constants, structs, enums | 8 |
| 5 | Ownership, borrows, FFI contracts, generics | 9 |
| 6 | Code generation, profiles, runtime shims | 10 |
| 7 | Standard-library slice | 7 |
| 8 | Cross-toolchain suites, corpus E2E, packaging | 11 |

**Total: 84 tasks across 9 phases** (no fabricated hour estimates — scope is
bounded by the task-size criteria in the quality checklist)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of
> truth** for progress. Every task line appears exactly once in this document.
> The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the
>    Last Updated stamp after EVERY task — never batch updates.
> 4. **Resume** by scanning top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>`.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 0: Validation Corpus (Task 0)

> **Phase baseline tree**: a798591ac32201831a1e4c1c13708e567a98ccfe
> **Expected modification set**: `spec/tests/corpus/**`, `plans/clorraine-compiler-mvp/99-execution-plan.md`
> **Scope mode**: strict

### Step 0.1: Author the four golden programs

**Reference**: [03-01-corpus.md](03-01-corpus.md) · AR-3, AR-28, AR-29
**Objective**: create the acceptance target and positive golden suite before
any compiler code exists.

- [x] 0.1.1 Write `spec/tests/corpus/file-copier/program.clor` and `expected-output.txt` per 03-01 §1 ✅ (completed: 2026-09-18 00:19)
- [x] 0.1.2 Write `spec/tests/corpus/interrupt-counter/program.clor` and `expected-output.txt` per 03-01 §2 ✅ (completed: 2026-09-18 00:20)
- [x] 0.1.3 Write `spec/tests/corpus/string-parser/program.clor` and `expected-output.txt` per 03-01 §3 ✅ (completed: 2026-09-18 00:20)
- [x] 0.1.4 Write `spec/tests/corpus/buffer-container/program.clor` and `expected-output.txt` per 03-01 §4 ✅ (completed: 2026-09-18 00:21)
- [x] 0.1.5 Write the four `README.md` rule-coverage, warning-audit, and deliberate-avoidance records per 03-01 ✅ (completed: 2026-09-18 00:22)

**Deliverables**:
- Four programs with byte-exact expected outputs
- Four coverage records

**Verify**: every program reviewed against 03-01 and the cited rules; no
diagnostics are expected from the eventual compiler (AR-43).

---

### Step 0.2: Hand-check and user checkpoint

**Reference**: 03-01 "Hand-Check Procedure" · AR-31
**Objective**: prove the corpus is valid against the frozen specification and
get the user's explicit acceptance before front-end work.

- [x] 0.2.1 Run the full line-by-line hand-check; complete every coverage record and list all friction in the four READMEs ✅ (completed: 2026-09-18 00:23)
- [x] 0.2.2 Present the corpus and friction list to the user; record the ruling (accept / revise). Any specification amendment requested by the friction pauses the phase ✅ (completed: 2026-09-18 09:09)

**Deliverables**:
- Completed coverage records
- Recorded user ruling

**Verify**: user acceptance recorded in the phase notes; `./spec/check.sh` stays green.

---

## Phase 1: Accepted Phase 2 Spikes

> **Phase baseline tree**: b6a74c7a7263eaab0f068c5843303710a12ccd64
> **Expected modification set**: `spikes/**`, `docs/findings-phase2.md`, `plans/clorraine-compiler-mvp/99-execution-plan.md`
> **Scope mode**: strict

### Step 1.1: Spike S1 — vbcc `#line`

**Reference**: [03-02-spikes.md](03-02-spikes.md) §S1 · AR-8, AR-41 · PF-042
**Objective**: measure how far vbcc honors `#line` before the emitter relies on it.

- [x] 1.1.1 Create `spikes/01-vbcc-line/demo.c` and its Makefile per S1 ✅ (completed: 2026-09-18 11:48)
- [x] 1.1.2 Run the vbcc compile, the decoded-line-table inspection, and the `-DERR` diagnostic check; capture raw output ✅ (completed: 2026-09-18 11:50)
- [x] 1.1.3 Write the S1 decision note: the confirmed or refuted `#line` behavior and any required spec ruling request ✅ (completed: 2026-09-18 11:51)

**Deliverables**:
- S1 fixture and raw results
- S1 decision note

**Verify**: `make -C spikes/01-vbcc-line` exits successfully; raw output saved for the findings doc.

---

### Step 1.2: Spike S2 — generated-C FFI ABI

**Reference**: 03-02 §S2 · PF-002
**Objective**: compile and run the exact emitted prototypes against the NDK.

- [x] 1.2.1 Create `spikes/02-ffi-abi/ffi_abi.c` and Makefile per S2 ✅ (completed: 2026-09-18 11:53)
- [x] 1.2.2 Compile with GCC and vbcc; run under `vamos` through the SDK; capture ABI observations and release counters ✅ (completed: 2026-09-18 11:54)
- [x] 1.2.3 Write the S2 decision note: ABI facts confirmed, any prototype change required ✅ (completed: 2026-09-18 11:55)

**Deliverables**:
- S2 fixture and raw results
- S2 decision note

**Verify**: `make -C spikes/02-ffi-abi` exits successfully; release counters balance.

---

### Step 1.3: Spike S3 — arithmetic wrapping and shifts

**Reference**: 03-02 §S3 · PF-008 · P12, B7a
**Objective**: fix the wrapping strategy for GCC (`-fwrapv`) and vbcc (unsigned idiom).

- [x] 1.3.1 Create `spikes/03-arithmetic/arith_gcc.c`, `arith_vbcc.c`, the expected table, and the Makefile per S3 ✅ (completed: 2026-09-18 11:56)
- [x] 1.3.2 Run the host and GCC/`-fwrapv` legs (plus the no-flag leg) and diff against the table ✅ (completed: 2026-09-18 11:57)
- [x] 1.3.3 Run the vbcc leg with the unsigned idiom and diff against the table ✅ (completed: 2026-09-18 11:57)
- [x] 1.3.4 Write the S3 decision note: the exact vbcc emission idiom (or proof none is needed) ✅ (completed: 2026-09-18 11:58)

**Deliverables**:
- S3 fixtures, expected table, raw diffs
- S3 decision note consumed by 03-06

**Verify**: `make -C spikes/03-arithmetic` exits successfully on all legs.

---

### Step 1.4: Spike S4 — run-fault patterns

**Reference**: 03-02 §S4 · PF-020 · AR-32, AR-36
**Objective**: make the six `spec/tests/run/` cases executable patterns with a
proven first line and skipped ladder.

- [x] 1.4.1 Create `spikes/04-run-faults/traps.c` with the host and Amiga shim variants ✅ (completed: 2026-09-18 11:59)
- [ ] 1.4.2 Run the host legs; assert the exact first lines and the skipped cleanup counter
- [ ] 1.4.3 Run the Amiga legs under `vamos`; map each of the six run cases to its fixture pattern
- [ ] 1.4.4 Write the S4 decision note: the shim contract codegen will implement

**Deliverables**:
- S4 fixture, raw outputs, six-case mapping
- S4 decision note

**Verify**: `make -C spikes/04-run-faults` exits successfully on all legs.

---

### Step 1.5: Findings and checkpoint

**Reference**: 03-02 "Deliverables and Findings" · AR-31, AR-41
**Objective**: consolidate the four spikes and obtain the user's ruling.

- [ ] 1.5.1 Write `docs/findings-phase2.md` with one section per spike (goal, commands, raw output, decision, open questions)
- [ ] 1.5.2 Present the findings to the user; record the ruling; any required specification amendment pauses codegen work

**Deliverables**:
- `docs/findings-phase2.md`
- Recorded user ruling

**Verify**: the findings name every codegen rule the spikes confirm or change; `./spec/check.sh` stays green.

---

## Phase 2: Front End, Diagnostics, CLI, Harness Skeleton

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_

### Step 2.1: Specification tests and scaffolding

**Reference**: [03-03-frontend.md](03-03-frontend.md) · AR-20, AR-21, AR-22, AR-24, AR-26 · ST-LEX-*, ST-PARSE-*, ST-CF-LEX
**Objective**: create the workspace and write the first spec tests before any
compiler logic.

- [ ] 2.1.1 Create the root `Cargo.toml` workspace, the `compiler/` crate (stub binary), and the root `Makefile` with `check` (spec check + container `cargo test`)
- [ ] 2.1.2 [spec-author] Write `compiler/tests/compile_fail_spec_test.rs` (golden discovery, `clorc` spawn, Ch 14 template wildcard matching, skip list) and the `stage_spec_test.rs` helper with the ST-LEX-1..2 and ST-PARSE-1..4 cases
- [ ] 2.1.3 Run the new spec tests and record the red phase (failures expected: no compiler behavior exists)

**Deliverables**:
- Workspace, crate stub, root Makefile
- Spec-test drivers and the recorded red phase

**Verify**: `make check` runs and shows the expected failures (red), not build errors in the test driver itself.

---

### Step 2.2: Implement diagnostics, lexer, parser, CLI skeleton

**Reference**: 03-03 "Diagnostics", "Lexer", "Parser", "CLI Skeleton" · Ch 01, grammar §10-§11
**Objective**: make the front end produce the E20xxx and parser-only diagnostics.

- [ ] 2.2.1 Implement `src/span.rs` and `src/diag.rs` (diagnostic value, renderer, warning policy, `--max-errors`)
- [ ] 2.2.2 Implement `src/lex.rs` (all tokens, literals, templates, comments, lexical diagnostics)
- [ ] 2.2.3 Implement `src/ast.rs` and `src/parse.rs` (recursive descent + Pratt, disambiguation table, diagnostic-only starts)
- [ ] 2.2.4 Implement `src/driver.rs` CLI skeleton (`--profile`, files, diagnostics, exit codes; empty translation unit on success until Phase 6)

**Deliverables**:
- Compiler modules producing the lexer/parser diagnostics
- CLI honoring AR-21/AR-22

**Verify**: `make check` stays green for the scaffold; unit builds pass.

---

### Step 2.3: Green phase and implementation tests

**Reference**: 03-03 "Testing Requirements" · ST-CF-LEX, ST-PARSE-*
**Objective**: pass the in-scope syntax goldens and add internals tests.

- [ ] 2.3.1 Run the compile-fail driver on the E20xxx files plus E10204/E10208/E10501/E90005; fix until green
- [ ] 2.3.2 Write lexer and parser impl tests (tokenization, templates, disambiguation, recovery)
- [ ] 2.3.3 Run the full fast verify (`make check`)

**Deliverables**:
- In-scope syntax goldens green
- Implementations tests in `src/lex.rs`, `src/parse.rs`

**Verify**: `make check`.

---

## Phase 3: Name Resolution and Modules

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_

### Step 3.1: Specification tests first

**Reference**: [03-04-semantics.md](03-04-semantics.md) "Name Resolution" · ST-NAME-1..2, ST-CF-MOD
**Objective**: write the name/module cases before implementing them.

- [ ] 3.1.1 [spec-author] Add the ST-NAME cases to `stage_spec_test.rs` and confirm the in-scope golden subset (E10219, E11101-E11105, W10006); run and record the red phase

**Deliverables**:
- New spec cases and recorded red phase

**Verify**: the new cases fail for the intended reason (unimplemented resolution), not driver errors.

---

### Step 3.2: Implement loading and resolution

**Reference**: 03-04 "Loading and Module Identity", "Name Resolution" · Ch 10 · AR-25
**Objective**: resolve every name in the program.

- [ ] 3.2.1 Implement `src/load.rs` (module table, merged modules, imports, embedded stdlib table, E11101-E11105, W10006)
- [ ] 3.2.2 Implement `src/resolve.rs` (scopes, declaration-before-use, shadowing, qualified access, dot-call candidates, E10219/E20006)

**Deliverables**:
- Resolved symbol graph consumed by later phases

**Verify**: `make check` builds; targeted ST cases progress.

---

### Step 3.3: Green phase and implementation tests

- [ ] 3.3.1 Run the name/module spec cases and golden files; fix until green
- [ ] 3.3.2 Write `load.rs`/`resolve.rs` impl tests (merge, imports, scopes, init-edge cases)
- [ ] 3.3.3 Run the full fast verify (`make check`)

**Deliverables**:
- Name/module goldens green
- Implementation tests

**Verify**: `make check`.

---

## Phase 4: Types, Constants, Structs, Enums

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_

### Step 4.1: Specification tests first

**Reference**: 03-04 "Type System", "Constant Evaluation" · ST-TYPE-1..3, ST-CONST-1, ST-ENUM-1, ST-CF-TYPE, ST-CF-STR
**Objective**: write the type/const/enum spec cases and confirm the golden subset.

- [ ] 4.1.1 [spec-author] Add the ST-TYPE/ST-CONST/ST-ENUM cases and template cases (E10500/E10501/E10503) to `stage_spec_test.rs`; confirm the E102xx scope (E10213/E10214 excluded, AR-10); run and record the red phase

**Deliverables**:
- New spec cases and recorded red phase

**Verify**: failures are the intended unimplemented checks.

---

### Step 4.2: Implement the type layer

**Reference**: 03-04 "Type System" · Ch 02, Ch 04, Ch 07, Ch 09 · AR-11, AR-13
**Objective**: type every expression and declaration in the corpus subset.

- [ ] 4.2.1 Implement `src/types.rs` (`Ty`, duplicability, conversions T1a, unification, casts, `sizeof`/`offsetof` folding support)
- [ ] 4.2.2 Implement expression and statement checking in `src/sema.rs` (operators, conditions, assignment, indexing, calls, dot-calls, intrinsics)
- [ ] 4.2.3 Implement struct, enum, and struct-literal checking (STR-*, EN-*, E10203/E10205/E10206/E10207/E10208/E10210/E10211/E10212) and template typing (E10500/E10501/E10503)
- [ ] 4.2.4 Implement `src/consts.rs` (constant contexts, folding with runtime semantics, E10202/E10215/E10216/E10220/E10223/E10224/E10226)

**Deliverables**:
- Typed AST with constant values resolved
- Generic declaration checks (E10300/E10301/E10302/E10305)

**Verify**: `make check` builds; targeted ST cases progress.

---

### Step 4.3: Green phase and implementation tests

- [ ] 4.3.1 Run the type/const/enum spec cases and golden files; fix until green
- [ ] 4.3.2 Write `types.rs`, `sema.rs`, and `consts.rs` impl tests (conversion matrix, fold/fault cases)
- [ ] 4.3.3 Run the full fast verify (`make check`)

**Deliverables**:
- Type/const/enum goldens green
- Implementation tests

**Verify**: `make check`.

---

## Phase 5: Ownership, Borrows, FFI Contracts, Generics

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_

### Step 5.1: Specification tests first

**Reference**: [03-05-ownership.md](03-05-ownership.md) · ST-OWN-1..6, ST-FFI-1..2, ST-CF-OWN, ST-CF-FFI, ST-CF-UNSAFE
**Objective**: write the ownership/FFI spec cases and confirm the golden subset.

- [ ] 5.1.1 [spec-author] Add the ST-OWN/ST-FFI cases to `stage_spec_test.rs`; confirm the E100xx/E101xx/E106xx scope; run and record the red phase

**Deliverables**:
- New spec cases and recorded red phase

**Verify**: failures are the intended unimplemented checks.

---

### Step 5.2: Implement the ownership layer

**Reference**: 03-05 "Typed Control-Flow IR" through "FFI Contracts" · Ch 03, Ch 12, Ch 16, Ch 17 §2
**Objective**: produce the typed control-flow IR with every drop, defer, and
loan decision explicit.

- [ ] 5.2.1 Implement `src/ir.rs` lowering to CFGs with the Ch 17 §2 statement set
- [ ] 5.2.2 Implement `src/own.rs` definite assignment and nullability narrowing (E10021, E10010-E10012, F9)
- [ ] 5.2.3 Implement move state, partial moves, and drop planning (E10020, E10022, E10025, E10026; O3-O7, O2)
- [ ] 5.2.4 Implement `src/borrow.rs` loan collection, conflicts, elision, and reborrow (E10030-E10036, E10038, E10039, E10041; EL-1..6, O12)
- [ ] 5.2.5 Implement `src/ffi.rs` contract checking (E10100-E10104; CN-6..CN-16) and `unsafe`/raw checks (E10600-E10604)
- [ ] 5.2.6 Implement generics monomorphization inputs: instantiation set, copy obligations, dedupe keys (V9/V9a, G3, E10305)

**Deliverables**:
- Checked IR with explicit drops/defers and the instantiation set

**Verify**: `make check` builds; targeted ST cases progress.

---

### Step 5.3: Green phase and implementation tests

- [ ] 5.3.1 Run the ownership/FFI spec cases and golden files; fix until green
- [ ] 5.3.2 Write `ir.rs`/`own.rs`/`borrow.rs`/`ffi.rs` impl tests and run the full fast verify (`make check`)

**Deliverables**:
- Ownership/FFI goldens green
- Implementation tests (IR snapshots, joins, reborrow, contracts)

**Verify**: `make check`.

---

## Phase 6: Code Generation, Profiles, Runtime Shims

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_

### Step 6.1: Specification tests first

**Reference**: [03-06-codegen.md](03-06-codegen.md) · ST-CG-1..8, ST-CLI-1..3, ST-RUN-1..6
**Objective**: write the codegen/CLI/determinism spec cases before the emitter.

- [ ] 6.1.1 [spec-author] Add the ST-CG/ST-CLI cases to `stage_spec_test.rs` and the run-fault driver (`run_fault_spec_test.rs`, `determinism_spec_test.rs`); run and record the red phase

**Deliverables**:
- Codegen/CLI/run-fault spec drivers and recorded red phase

**Verify**: failures are the intended missing-emitter failures.

---

### Step 6.2: Implement emitter, ABI, profiles, shims

**Reference**: 03-06 "Emitted File Structure" through "Driver" · Ch 17 · Ch 15 · AR-5, AR-19, AR-27, AR-35
**Objective**: emit the single C99 translation unit for both profiles.

- [ ] 6.2.1 Implement `src/codegen.rs` core: type emission, naming/mangling, declarations, globals
- [ ] 6.2.2 Implement functions, statements, the cleanup ladder, moves/zero, `defer`, and `#line` placement
- [ ] 6.2.3 Implement expressions, traps, templates desugaring, and the wrapping/masking strategy confirmed by spike S3
- [ ] 6.2.4 Implement `src/abi.rs`, FFI prototype emission, module init functions, and the entry stub
- [ ] 6.2.5 Implement `src/profile.rs`, `compiler/runtime/shim_host.c`, and the driver's output writing
- [ ] 6.2.6 Implement `compiler/runtime/shim_hosted_amiga.c` (exec/dos, fault output, VBlank dispatcher surface)

**Deliverables**:
- Byte-deterministic C output for both profiles
- Runtime shims with the 03-07 API

**Verify**: `make check` builds; targeted ST cases progress.

---

### Step 6.3: Green phase and implementation tests

- [ ] 6.3.1 Run the codegen/CLI spec cases, the determinism test, and the six run-fault cases on host; fix until green
- [ ] 6.3.2 Write `codegen.rs`/`abi.rs` impl tests (fragments, ladder, naming, guards)
- [ ] 6.3.3 Run the full fast verify (`make check`)

**Deliverables**:
- Codegen/CLI/run-fault goldens green
- Implementation tests

**Verify**: `make check`.

---

## Phase 7: Standard-Library Slice

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_

### Step 7.1: Specification tests first

**Reference**: [03-07-stdlib.md](03-07-stdlib.md) · ST-STDLIB-1..6, ST-WARN-1, ST-CF-WARN, ST-CF-DENY
**Objective**: write the stdlib and warning spec cases before implementing.

- [ ] 7.1.1 [spec-author] Add the ST-STDLIB cases to `stage_spec_test.rs` and the warning cases; add `spec/tests/compile-fail/W10000_deny_warnings.clor` and update the suite README (AR-34); run and record the red phase

**Deliverables**:
- Stdlib/warning spec cases and recorded red phase
- Warning-escalation golden

**Verify**: failures are the intended missing-stdlib failures.

---

### Step 7.2: Implement the stdlib slice

**Reference**: 03-07 "Compiler-Known Intrinsic Helpers" through "Shim C API" · Ch 10, appendix stdlib · AR-15, AR-46, AR-47, AR-49, AR-50, AR-51, AR-52, AR-53
**Objective**: the AR-15/AR-46/AR-47 slice compiles warning-free and supports
the corpus.

- [ ] 7.2.1 Implement the intrinsic helpers (`rawSlice`, `rawStr`, `slice`, `len` cases) plus `stdlib/std.mem.clor`
- [ ] 7.2.2 Implement `stdlib/std.text.clor` and `stdlib/std.fmt.clor`
- [ ] 7.2.3 Implement `stdlib/std.io.clor` and the shim IO/print additions
- [ ] 7.2.4 Implement `stdlib/std.amiga.dos.clor`, `stdlib/std.amiga.interrupt.clor`, and the dispatcher shim

**Deliverables**:
- Six stdlib modules embedded via `stdlib.rs`
- Shim API completed

**Verify**: `make check` builds; targeted ST cases progress.

---

### Step 7.3: Green phase and implementation tests

- [ ] 7.3.1 Run the stdlib/warning spec cases and golden files; fix until green (stdlib compiles warning-free under both profiles)
- [ ] 7.3.2 Write stdlib impl tests (buffer/array/format/file/span behaviors) and run the full fast verify (`make check`)

**Deliverables**:
- Stdlib/warning goldens green
- Implementation tests

**Verify**: `make check`.

---

## Phase 8: Cross-Toolchain Suites, Corpus E2E, Packaging

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_

### Step 8.1: Corpus host leg

**Reference**: 03-01 "Corpus Runner Interface" · ST-CORPUS-1..4
**Objective**: the corpus passes end to end on the fast profile.

- [ ] 8.1.1 Wire `corpus_spec_test.rs` to compile and run the three host programs and assert the interrupt-counter E11200 diagnostic; fix until green

**Deliverables**:
- Corpus host leg green (including the E11200 assertion)

**Verify**: `make check`.

---

### Step 8.2: Hosted-amiga legs

**Reference**: 03-01 "Corpus Runner Interface", Ch 17 §13 · AR-4, AR-5
**Objective**: the corpus builds and runs on the real target.

- [ ] 8.2.1 Add the `corpus-amiga` Make target: GCC build plus `vamos` run for file-copier, string-parser, buffer-container, with output diff
- [ ] 8.2.2 Add the vbcc build-only leg for file-copier
- [ ] 8.2.3 Add the interrupt-counter FS-UAE leg (`RUN_CAPTURE=1`); on environment failure, record the build-only fallback and request a user ruling
- [ ] 8.2.4 Add the hosted-amiga run-fault leg (ST-RUN-7) via `vamos`

**Deliverables**:
- Cross-toolchain targets and captured outputs
- Recorded fallback ruling if needed

**Verify**: `make verify` (or the targeted `make corpus-amiga`) exits successfully and diffs match.

---

### Step 8.3: Suite completeness

**Reference**: 07 "Compile-fail suites" · ST-CF-*, AR-10, AR-23, AR-34
**Objective**: the whole in-scope golden set is green and the skips are honest.

- [ ] 8.3.1 Run the full compile-fail scope; fix residual failures; confirm the skip report lists each excluded file with its reason and the aggregate count
- [ ] 8.3.2 Confirm the warning-escalation case and all ST-WARN cases pass in the default and `--deny-warnings` modes

**Deliverables**:
- Full in-scope suite green with skip report
- Warning escalation green

**Verify**: `make check`.

---

### Step 8.4: Orchestration and project docs

**Reference**: 02 "Gaps" · AR-7, AR-38, AR-40
**Objective**: the project's verify commands and guidance are persistent.

- [ ] 8.4.1 Complete the root Makefile: `check` (spec + fast tests) and `verify` (check + corpus/run-fault cross legs)
- [ ] 8.4.2 Create `AGENTS.md` with the analyze-project skill (commands, layout, Rust test-naming map)
- [ ] 8.4.3 Create/refresh `plans/00-roadmap.md`, update `README.md` status, and finalize `docs/findings-phase2.md` cross-references

**Deliverables**:
- Root Makefile targets documented in AGENTS.md
- Roadmap and README status current

**Verify**: `make verify` from a clean container run.

---

### Step 8.5: Final acceptance

**Reference**: [01-requirements.md](01-requirements.md) "Acceptance Criteria" · AR-31
**Objective**: hand the finished MVP to the user.

- [ ] 8.5.1 Run the full `make verify` and present the acceptance summary (corpus outputs, suite counts, spike results, known limitations) for the user's ruling

**Deliverables**:
- Final evidence bundle
- Recorded user acceptance or a named defect list

**Verify**: all eight acceptance criteria of `01-requirements.md` checked off.

---

## Dependencies

```
Phase 0 (corpus) ──► Phase 1 (spikes)
        │                  │
        └──────┬───────────┘
               ▼
Phase 2 (front end + harness)
               ▼
Phase 3 (names) ─► Phase 4 (types/consts) ─► Phase 5 (ownership/FFI)
                                                    ▼
Phase 6 (codegen + shims) ─► Phase 7 (stdlib) ─► Phase 8 (suites/E2E/packaging)
```

Phase 0 and Phase 1 both gate the compiler phases (AR-31); Phase 8 depends on
all of them.

---

## Success Criteria

**The MVP is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check`; `make verify` for cross legs)
3. ✅ No warnings/errors in compiler, stdlib, or emitted C
4. ✅ No dead code — no unused crates, modules, functions, or parameters
5. ✅ Security requirements of `01-requirements.md` met (untrusted source,
   no shell, no path resolution from source, no network)
6. ✅ Documentation updated (`AGENTS.md`, `docs/findings-phase2.md`,
   `plans/00-roadmap.md`, `README.md`)
7. ✅ Specification untouched except by explicit user ruling (AR-31)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
