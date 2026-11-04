# Execution Plan: Amiga Library Surface

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-03
> **Progress**: 16/16 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Three phases: the `std.amiga.exec` wrapper, the two acceptance programs, and the appendix /
verification close-out. The work is specification-first for the wrapper: the expected host
rejection, emitted shim calls, and program transcripts are written before the implementation.

## Phase 1: `std.amiga.exec` wrapper

**Tasks**: 6

- [x] 1.1.1 Write ST-1-ST-5, ST-7, and ST-9 in `compiler/tests/amiga_exec_spec_test.rs`; record the red phase
- [x] 1.1.2 Add the port/message/signal shim functions to `compiler/runtime/shim_hosted_amiga.c`
- [x] 1.1.3 Add `stdlib/std.amiga.exec.clo` with the wrapper and ownership contracts
- [x] 1.1.4 Register the module in `compiler/src/stdlib.rs` (`SOURCES` and `MODULES`)
- [x] 1.1.5 Run the wrapper cases green; add the stdlib registration unit test
- [x] 1.1.6 Phase review (correctness + memory-safety lens on the shim)

**Verify**: `make -C docker exec CMD="cargo test --test amiga_exec_spec_test"`; `make check`

## Phase 2: Acceptance programs

**Tasks**: 5

- [x] 2.1.1 Add `examples/amiga/exec-self-test.clo`, `examples/amiga/message-drop.clo`, and their expected transcripts
- [x] 2.1.2 Port Hamurabi to `examples/amiga/hamurabi.clo` with the deterministic steward policy
- [x] 2.1.3 Add the hand-checked expected transcript
- [x] 2.1.4 Build and run both programs on hosted-amiga; fix every implementation/FFI gap they expose
- [x] 2.1.5 Phase review (correctness + semantics + memory-safety)

**Verify**: the two hosted-amiga runs; `make check`

## Phase 3: Appendix and close-out

**Tasks**: 5

- [x] 3.1.1 Add the `std.amiga.exec` section to `spec/appendix-stdlib.md`
- [x] 3.1.2 Re-review `spec/appendix-amiga.md` and `spec/appendix-stdlib.md` against the delivered API
- [x] 3.1.3 Run `./spec/check.sh` and re-record both appendices `stable (v1.0)`
- [x] 3.1.4 Run `make verify` and record the evidence
- [x] 3.1.5 Phase review and plan close; update the roadmap

**Verify**: `make verify`; `./spec/check.sh`

## Dependencies

Phase 1 → Phase 2 → Phase 3. The two programs cannot run until the wrapper exists; the
appendices cannot be re-reviewed until the wrapper and programs are final.

## Success criteria

1. All 16 tasks complete.
2. `std.amiga.exec` builds and runs on hosted-amiga; the host profile rejects it.
3. Both programs match their expected transcripts.
4. The appendices are re-reviewed and remain `stable (v1.0)`.
5. `make check`, `make verify`, and `./spec/check.sh` pass.
