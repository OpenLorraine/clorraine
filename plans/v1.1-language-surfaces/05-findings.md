# Findings: v1.1 Language Surfaces

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-08

This ledger records the starting evidence, the red/green evidence for every
phase, the gate logs, the review findings, and the residual limits. It is
updated as the execution plan progresses; a claim without a recorded command
or output is not evidence.

## 1. Baseline

| Check | Result |
|---|---|
| Starting tree | `a7dbdd5` — `docs(tests): describe compiler robustness cases in plain language` |
| Working tree | clean before the plan work |
| `git rev-list --count origin/main..HEAD` | `0` |
| `make check` | exit 0; fuzzer smoke `cases=28 host=28 sanitizer=16 target=16 failures=0` |
| Only release tag | `v1.0.0` |
| Baseline log | `build/v11/make-check-baseline.log` (ignored) |

## 2. Requirements and Plan Gates

| Gate | Result |
|---|---|
| Requirements preflight | ✅ PASSED WITH NOTES — 23 findings (7 critical, 14 major, 2 minor); 21 resolved, 1 accepted (pre-existing `Array<str>` hole), 1 incorporated (`Box<static str>`) |
| Plan preflight | ✅ PASSED — 19 findings (3 critical, 14 major, 2 minor); all resolved |
| Requirements commit | `8a91bc5` — `docs(plans): specify the v1.1 language surfaces` |
| Revised docs commit | recorded by 0.1.4 |

## 3. Red Evidence

Each phase records the exact failing output before the fix. The expected
starting behavior is in [Current State](02-current-state.md).

### 3.1 Box type model

| Case | Starting behavior |
|---|---|
| bare `Box<i32>` | `E10219: 'Box' is not declared` |
| user `struct Box<T>` | accepted |
| `Box {}` / `Box(x)` / `Box<str>` | `E10219` before the name resolves; after 1.2.5 they are the accepted cases the new checks close |
| `Box<i32>` in `ffi` | `E10219` before the name resolves; the generic-aggregate `Wrap<i32>` case emits `ClWrap` with no typedef |

### 3.2 Box allocation and drop

Records the first implementation attempt's output (the API/type are absent on
the base).

### 3.3 X1 ordinary deleters

| Case | Starting behavior |
|---|---|
| valid `take T -> void` ordinary deleter | `E10102` |
| two modules with the same handle and different deleters | accepted; validator and emitter disagree |

### 3.4 L5 hooks

| Case | Starting behavior |
|---|---|
| capturing lambda at an unmarked `ffi` callback | accepted; emits `ClClosure_N` where a function pointer is expected; strict C build fails |
| capturing lambda at a `borrow` callback | same invalid C (the marker is not special yet) |
| view/borrow/fixed-array callback | accepted; the lifted lambda's C spelling differs from the ABI |

## 4. Gate Log

| Gate | Command | Result | Log |
|---|---|---|---|
| Baseline | `make check` | exit 0 | `build/v11/make-check-baseline.log` |
| Requirements preflight | document audit | passed with notes | [report](00-preflight-report-requirements.md) |
| Plan preflight | document audit | passed | [report](00-preflight-report.md) |

The phase gate logs are appended as each phase completes.

## 5. Review Findings

The per-phase reviewers are dispatched by the execution plan:

| Phase | Reviewers | Result |
|---|---|---|
| 1 | correctness + emission-integrity | pending |
| 2 | correctness + memory-safety | pending |
| 3 | correctness + memory-safety | pending |
| 5 | correctness + memory-safety + emission-integrity + semantics | pending |
| 7 | final cross-phase | pending |

## 6. Residual Limits

The v1.1 limits to document in `docs/known-limitations.md`:

- `Box<T>` allocation failure traps; `Box<T>?` is an optional slot, not an
  allocation result.
- A boxed value cannot be moved out; `boxSet` replaces it and `boxWith` lends
  it read-only. A raw cast of a Box outside `std.mem` is rejected.
- Direct Box FFI crossing is rejected (`E10105`); use `boxWith`.
- Freestanding has no Box (`std.mem` is not supported).
- The C hook environment is caller-stack only; the callback is synchronous.
- A capturing hook is not supported in a generic function body.
- Hook callback signatures are restricted to ABI-identical types; views,
  borrow move-only structs, fixed arrays, and fixed-array returns are
  rejected.
- A pre-existing `Array<str>` view-storage hole is outside this feature; the
  Box path (`Box<Array<str>>`) is closed by `E10227`.
