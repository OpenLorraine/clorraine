# Execution Plan: Clorraine build/run driver (`clorraine`)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-23 08:11
> **Progress**: 3/14 tasks (21%)
> **CodeOps Artifact Schema**: 1

## Overview

Add a `clorraine` driver binary that builds and runs Clorraine programs by
orchestrating the existing emit-only `clc` and the profile toolchain, hiding the
C step. Three phases follow the mandatory specification-first ordering:
specification tests and the red phase, the implementation, then the green
phase, unit tests, and full verification.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Specification tests first | 3 |
| 2 | Implement the driver | 7 |
| 3 | Green phase, unit tests, docs | 4 |

**Total: 14 tasks across 3 phases** (no fabricated hour estimates)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
> The task checkboxes below are the single source of truth for progress. Mark a
> task `[~]` on implementation and promote it to `[x]` only on a passing verify;
> update the Progress header and Last Updated stamp after every task. Timestamps
> come from `date '+%Y-%m-%d %H:%M'`.

---

## Phase 1: Specification tests first

> **Phase baseline tree**: 873155cbfb04aa75ba2cb7e1b6d4b05c53abb25d
> **Expected modification set**: `compiler/Cargo.toml`, `compiler/src/lib.rs`, `compiler/src/clorraine/**`, `compiler/src/clorraine_main.rs`, `compiler/tests/clorraine_driver_spec_test.rs`, `plans/clorraine-driver/99-execution-plan.md`
> **Scope mode**: strict

### Step 1.1: Test scaffolding, spec tests, and the red phase

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-DRV-1..12, ST-DRV-17 · [03-01](03-01-driver-cli.md), [03-02](03-02-build-run.md) · AR-2, AR-15, AR-20
**Objective**: encode the driver's observable behaviour before any behaviour exists.

- [x] 1.1.1 Wire the `clorraine` binary skeleton so the spec tests can compile: add the `[[bin]]` target, `pub mod clorraine;`, a module root whose `run` stub reports "not implemented", and the thin bin entry — `compiler/Cargo.toml`, `compiler/src/lib.rs`, `compiler/src/clorraine/mod.rs`, `compiler/src/clorraine_main.rs` ✅ (completed: 2026-09-23 08:08)
- [x] 1.1.2 [spec-author] Write `compiler/tests/clorraine_driver_spec_test.rs` from ST-DRV-1..12 and ST-DRV-17; each case spawns the built `clorraine` binary and asserts the concrete input/output ✅ (completed: 2026-09-23 08:11)
- [x] 1.1.3 Run the suite and record the red phase (every case fails against the not-implemented stub) ✅ (completed: 2026-09-23 08:11)

**Deliverables**:
- The `clorraine` binary skeleton (test scaffolding, no behaviour)
- The driver spec-test file
- A recorded red phase

**Verify**: failures are the intended not-implemented-driver failures.

---

## Phase 2: Implement the driver

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Expected modification set**: `compiler/Cargo.toml`, `compiler/src/lib.rs`, `compiler/src/clorraine/**`, `compiler/src/clorraine_main.rs`, `Makefile`, `plans/clorraine-driver/99-execution-plan.md`
> **Scope mode**: strict

### Step 2.1: CLI and environment

**Reference**: [03-01 §Command surface, §Dispatch, §Environment](03-01-driver-cli.md) · AR-3, AR-7, AR-8, AR-12, AR-19, AR-23, AR-24, AR-26, AR-27
**Objective**: parse, validate, help/version, and the SDK/missing-tool checks.

- [ ] 2.1.1 Implement `cli` (commands, flags, defaults, usage errors, help, version) — `compiler/src/clorraine/cli.rs`
- [ ] 2.1.2 Implement `environment` (`sdk_present`, `missing_tool`, parameterised for tests) — `compiler/src/clorraine/environment.rs`

### Step 2.2: Toolchain, build, run, emit

**Reference**: [03-02 §Toolchain table, §Artifact layout, §Pipelines, §Handoff, §Capture](03-02-build-run.md) · AR-5, AR-6, AR-9, AR-10, AR-11, AR-16, AR-17, AR-18, AR-25
**Objective**: emit C in process, build, run, capture, and the FS-UAE handoff.

- [ ] 2.2.1 Implement `toolchain` (profile→C command table, runner, handoff text) — `compiler/src/clorraine/toolchain.rs`
- [ ] 2.2.2 Implement `build` (`emit_c` via `clc::driver::run`, `compile`, the ICE report) — `compiler/src/clorraine/build.rs`
- [ ] 2.2.3 Implement `run` (host exec, `vamos`, capture, exit-code propagation) — `compiler/src/clorraine/run.rs`
- [ ] 2.2.4 Implement `emit` (stdout / `-o`) and the artifact-path helper — `compiler/src/clorraine/mod.rs`, `compiler/src/clorraine/artifacts.rs`

### Step 2.3: Make passthrough

**Reference**: [01-requirements.md](01-requirements.md) R11 · AR-14
**Objective**: one-command use from the repository root.

- [ ] 2.3.1 Add `make run FILE=…` and `make build FILE=…` targets that call `clorraine` in the container — `Makefile`

**Deliverables**:
- A working `clorraine` binary for `host` and `hosted-amiga`
- The Make passthrough

**Verify**: `make check` builds; targeted ST cases progress.

---

## Phase 3: Green phase, unit tests, docs

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Expected modification set**: `compiler/src/clorraine/**` (unit tests), `AGENTS.md`, `plans/00-roadmap.md`, `plans/clorraine-driver/99-execution-plan.md`
> **Scope mode**: strict

- [ ] 3.1.1 Run the driver spec tests; fix until green
- [ ] 3.2.1 Write the in-module unit tests (ST-DRV-13..16: environment, toolchain assembly, ICE shape) — `compiler/src/clorraine/*.rs`
- [ ] 3.3.1 Run the full fast verify (`make check`)
- [ ] 3.4.1 Document the driver in `AGENTS.md`, add the `PLAN-02` roadmap row, and run `make verify` — `AGENTS.md`, `plans/00-roadmap.md`

**Deliverables**:
- Driver spec tests green
- Unit tests
- Documentation and roadmap updated

**Verify**: `make check`; then `make verify`.

---

## Dependencies

```
Phase 1 (spec tests, red)
    ↓
Phase 2 (implement)
    ↓
Phase 3 (green, impl tests, docs)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check`, `make verify`)
3. ✅ No warnings/errors; `cargo build --workspace` warning-free
4. ✅ No dead code — no unused parameters, functions, or modules
5. ✅ Security held — argument-array invocation, no shell, no network, no Docker socket
6. ✅ `AGENTS.md` and `plans/00-roadmap.md` updated
7. ✅ `clc` and the frozen specification unchanged
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
