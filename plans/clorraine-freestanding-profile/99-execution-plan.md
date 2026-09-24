# Execution Plan: Freestanding profile

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-24 08:29
> **Progress**: 15/19 tasks (79%)
> **CodeOps Artifact Schema**: 1

## Overview

Implement the `freestanding` profile: accept it in the compiler, embed a
no-libc runtime with its own `_start`, gate the standard-library surface, and
give the driver a two-artifact build/run flow. Four phases follow the mandatory
specification-first ordering: specification tests and the red phase, the
profile and runtime, the driver and specification edits, then the green phase,
implementation tests, documentation, and full verification.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Specification tests first | 4 |
| 2 | Profile and runtime | 6 |
| 3 | Driver, specification, and goldens | 5 |
| 4 | Green phase, implementation tests, docs | 4 |

**Total: 19 tasks across 4 phases** (no fabricated hour estimates)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
> The task checkboxes below are the single source of truth for progress. Mark a
> task `[~]` on implementation and promote it to `[x]` only on a passing verify;
> update the Progress header and Last Updated stamp after every task. Timestamps
> come from `date '+%Y-%m-%d %H:%M'`.

---

## Phase 1: Specification tests first

> **Phase baseline tree**: 8bd4f9b55edb3043a17d8bb6df5d0816b50ca4e1
> **Expected modification set**: `compiler/tests/freestanding_spec_test.rs`, `compiler/tests/stage_spec_test.rs`, `spec/tests/corpus/freestanding/**`, `plans/clorraine-freestanding-profile/99-execution-plan.md`
> **Scope mode**: strict

### Step 1.1: Encode the freestanding behaviour before it exists

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-1, ST-3..ST-7, ST-11..ST-21 · [03-01](03-01-profile-runtime.md), [03-02](03-02-driver.md) · AR-7, AR-9, AR-10, AR-11, AR-13, AR-20, AR-27
**Objective**: write the specification tests, including the m68k link check and
the host no-libc run, and record the red phase.

- [x] 1.1.1 [spec-author] Write `compiler/tests/freestanding_spec_test.rs`: the harness runs the built `clc` and then `cc`/`m68k-amigaos-gcc` directly, asserts the m68k link exits 0, and asserts the ST-1, ST-3..ST-7, and ST-11..ST-21 expectations — `compiler/tests/freestanding_spec_test.rs` ✅ (completed: 2026-09-24 08:24)
- [x] 1.1.2 [spec-author] Update the freestanding branch of the profile-selection test to expect acceptance instead of `E11200` — `compiler/tests/stage_spec_test.rs` ✅ (completed: 2026-09-24 08:24)
- [x] 1.1.3 Add the freestanding console corpus inputs and recorded output — `spec/tests/corpus/freestanding/program.clo`, `spec/tests/corpus/freestanding/expected-output.txt` ✅ (completed: 2026-09-24 08:24)
- [x] 1.1.4 Run the suite and record the red phase: every freestanding case fails because the profile is still rejected, and no `host`/`hosted-amiga` case regresses ✅ (completed: 2026-09-24 08:24)

**Deliverables**:
- The freestanding spec-test harness
- The updated profile-selection test
- The corpus inputs
- A recorded red phase

**Verify**: the only failures are the intended profile-not-implemented ones.

---

## Phase 2: Profile and runtime

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Expected modification set**: `compiler/runtime/shim_freestanding.c`, `compiler/src/profile.rs`, `compiler/src/driver.rs`, `compiler/src/codegen.rs`, `compiler/src/codegen/program.rs`, `compiler/src/types/mod.rs`, `compiler/src/codegen/calls.rs`, `spec/02-types.md`, `plans/clorraine-freestanding-profile/99-execution-plan.md`
> **Scope mode**: strict

### Step 2.1: Add the profile and the no-libc runtime

**Reference**: [03-01](03-01-profile-runtime.md) §New Types, §The runtime shim, §Build flags · AR-3, AR-4, AR-5, AR-9, AR-10, AR-13, AR-14, AR-18, AR-19
**Objective**: a real third profile with a self-contained runtime.

- [x] 2.1.1 Create the freestanding runtime shim: portable `memcpy`/`memset`/`memcmp`/`strlen`, the console and trap helpers, the file-I/O trap, the arch-guarded platform layer, and `_start` — `compiler/runtime/shim_freestanding.c` ✅ (completed: 2026-09-24 08:29)
- [x] 2.1.2 Add `Profile::Freestanding` with `parse`, the `implemented` query, `supports_module`, `shim`, and the 1024-byte `stack_frame_limit` — `compiler/src/profile.rs` ✅ (completed: 2026-09-24 08:29)
- [x] 2.1.3 Make `Profile` the single source of truth for the name check; delete the duplicate `match` and keep the dormant unimplemented-profile `E11200` branch — `compiler/src/driver.rs` ✅ (completed: 2026-09-24 08:29)
- [x] 2.1.4 Add the freestanding arm to the null/empty-state spelling and emit the ABI typedefs for freestanding too — `compiler/src/codegen.rs`, `compiler/src/codegen/program.rs` ✅ (completed: 2026-09-24 08:29)
- [x] 2.1.5 Allow a `static str` literal where a `cstring` is expected and emit it as a C string literal, so `openFile("x", 1005)` compiles (AR-31 runtime) — `compiler/src/types/mod.rs`, `compiler/src/codegen/calls.rs`, `spec/02-types.md` ✅ (completed: 2026-09-24 08:29)

### Step 2.2: Green the profile and runtime tests

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-1, ST-3..ST-7, ST-11..ST-14, ST-16, ST-17
**Objective**: the emitted C links and runs on both legs.

- [x] 2.2.1 Run the freestanding spec tests and fix until the compiler, gating, m68k link, and host no-libc run cases pass ✅ (completed: 2026-09-24 08:29)

**Deliverables**:
- A working `freestanding` profile and runtime
- Freestanding spec tests green for the compiler and runtime legs

**Verify**: the Phase 1 red cases now pass; `host`/`hosted-amiga` unchanged.

---

## Phase 3: Driver, specification, and goldens

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Expected modification set**: `compiler/src/clorraine/{mod,cli,toolchain,build,run,artifacts}.rs`, `compiler/src/driver.rs`, `spec/15-profiles.md`, `spec/00-introduction.md`, `spec/17-codegen.md`, `spec/14-diagnostics.md`, `spec/README.md`, `spec/decisions.md`, `spec/check.sh`, `spec/tests/compile-fail/E11200_unimplemented_profile.clo`, `spec/tests/compile-fail/E11200_unavailable_module.clo`, `compiler/tests/stage_spec_test.rs`, `compiler/tests/compile_fail_spec_test.rs`, `plans/clorraine-freestanding-profile/99-execution-plan.md`
> **Scope mode**: strict (the spec set is exactly AR-21)

### Step 3.1: One-command driver support and the specification record

**Reference**: [03-02](03-02-driver.md) §Toolchain rows, §Build and run flow · [03-03](03-03-spec-goldens.md) · AR-7, AR-8, AR-20, AR-21, AR-27, AR-28, AR-30
**Objective**: the driver builds the m68k artifact and runs the host no-libc
artifact; the specification and goldens reflect an implemented profile.

- [x] 3.1.1 Add the freestanding build and run tool rows and the `run_tool` lookup; accept the profile in the CLI and update the help, unknown-profile, and `clc` usage text — `compiler/src/clorraine/toolchain.rs`, `compiler/src/clorraine/cli.rs`, `compiler/src/driver.rs` ✅ (completed: 2026-09-24 08:29)
- [x] 3.1.2 Implement the two-artifact freestanding flow: route the run to the host artifact and require `cc` in `mod.rs`, own the `<stem>.host` path in `artifacts.rs`, and adapt `build`/`run` — `compiler/src/clorraine/mod.rs`, `compiler/src/clorraine/artifacts.rs`, `compiler/src/clorraine/build.rs`, `compiler/src/clorraine/run.rs` ✅ (completed: 2026-09-24 08:29)
- [x] 3.1.3 Apply the authorized specification edits: PR-6 reword, the profile tables, the codegen runtime/matrix rows, the introduction, the README, the decision record, and register the `E11214` runtime code (and add it to `spec/check.sh`'s runtime-fault list) — `spec/15-profiles.md`, `spec/00-introduction.md`, `spec/17-codegen.md`, `spec/14-diagnostics.md`, `spec/README.md`, `spec/decisions.md`, `spec/check.sh` ✅ (completed: 2026-09-24 08:29)
- [x] 3.1.4 Replace the obsolete rejection golden with the `E11200_unavailable_module.clo` golden (host + `std.amiga.dos`) and keep `E11200` in the compile-fail expectation set — `spec/tests/compile-fail/E11200_unimplemented_profile.clo` (removed), `spec/tests/compile-fail/E11200_unavailable_module.clo` (added), `compiler/tests/compile_fail_spec_test.rs` ✅ (completed: 2026-09-24 08:29)
- [x] 3.1.5 Update the root `Makefile` so `PROFILE_ARG` forwards any non-host profile, then run `./spec/check.sh` — `Makefile` ✅ (completed: 2026-09-24 08:29)

**Deliverables**:
- Freestanding driver support
- The specification and goldens updated
- `./spec/check.sh` green

**Verify**: `clorraine build`/`run --profile freestanding` work; `./spec/check.sh` passes.

---

## Phase 4: Green phase, implementation tests, docs

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Expected modification set**: `compiler/**` (unit tests), `AGENTS.md`, `plans/00-roadmap.md`, `plans/clorraine-freestanding-profile/99-execution-plan.md`
> **Scope mode**: strict

- [ ] 4.1.1 Run the full freestanding spec suite green, including the `-O2` m68k link and the link-exit assertion — `compiler/tests/freestanding_spec_test.rs`
- [ ] 4.1.2 Write the in-module unit tests (profile facts, the shim's symbols and its no-libg-libc-symbol constraint, driver branches, toolchain command vectors, CLI text, and the `stdlib.rs` per-profile FFI-binding loop) — `compiler/src/profile.rs`, `compiler/src/driver.rs`, `compiler/src/stdlib.rs`, `compiler/src/clorraine/{toolchain,cli}.rs`
- [ ] 4.1.3 Run the full fast gate (`make check`) — `./spec/check.sh` + `cargo test --workspace`
- [ ] 4.1.4 Document the profile in `AGENTS.md`, update the `PLAN-03` roadmap row, and run the full acceptance gate (`make verify`)

**Deliverables**:
- All freestanding spec and implementation tests green
- Documentation and roadmap updated

**Verify**: `make check`; then `make verify`.

---

## Dependencies

```
Phase 1 (spec tests, red)
    ↓
Phase 2 (profile + runtime)
    ↓
Phase 3 (driver + spec + goldens)
    ↓
Phase 4 (green, impl tests, docs)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check`, `make verify`)
3. ✅ No warnings/errors; `cargo build --workspace` warning-free
4. ✅ No dead code — no unused parameters, functions, or modules
5. ✅ Security held — argument-array tool invocation, no shell, no network, syscall layer is write/exit only
6. ✅ `host` and `hosted-amiga` behaviour unchanged; the specification's remaining rules unchanged
7. ✅ `AGENTS.md` and `plans/00-roadmap.md` updated
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
