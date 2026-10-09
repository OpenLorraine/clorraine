# Testing Strategy: Defer Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Changed lowering and emission paths | 100% of the ST cases below, each red at its phase baseline and green at its phase end |
| Emitted shapes | Implementation pins for every new or changed spelling; byte-identical re-emission |
| Regressions | All existing suites green; the corpus emission sweep byte-identical (or traced); host-ASan clean |

- Test names state behavior: `should [expected behavior] when [condition]`.
- Integration/E2E: the repository's existing legs are the integration level — `make check` per
  phase and `make verify` (host, hosted-amiga, freestanding, FS-UAE) at the end. No new harness is
  built (a compiler E2E suite would duplicate the corpus legs; `N/A` with that concrete reason).
- The host AddressSanitizer path (`run_memory` + `assert_no_asan`) is the memory oracle; the
  in-container full-suite `RDC_ASAN` run is timing-flaky (documented 30 s startup flake) and is
  retried alone, never gated (AR-13).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Expectations are derived from the frozen rules — `spec/05-statements.md` §13 (`defer` is code
> motion; runs on `return`, `break`, `continue`; joins the cleanup ladder in reverse registration
> order) and §15 (every exit edge runs the ladder for its scope) and `spec/17-codegen.md` §4
> CL-1–CL-7 — plus the reproduced defect evidence in `02-current-state.md`. They define behavior
> before any implementation exists.

### Phase 1 — Per-Site Deferred Copies (03-01)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-1 | `f(): let i = 0; while (i < 2) { defer { if (i == 2) { return 9; } } i = i + 1; if (i == 1) { continue; } } return 0;` — `main` returns `f()` | exit 9; host-ASan clean (baseline: exit 0 — the fall-through site emits no copy) | §15; AR-7 |
| ST-2 | `f(): let i = 1; { defer { return 7; } defer { if (i == 99) { return 0; } } } return 0;` (the filed DEF-87 shape, probe `d87d`) | exit 7 (baseline: exit 0 — the outer site's copy is suppressed) | §13, §15; AR-7 |
| ST-3 | module counter `count = 0`; `f(c): { defer { count = count + 1; } if (c) { return 1; } } return 0;` — `main`: `let a: i32 = f(true) + f(false); return count * 10 + a;` | exit 21 (two defer runs, observably counted; baseline recorded at the phase — the return edge's rung emits no copy) | §15; AR-7, AR-15 |
| ST-4 | module counter `runs = 0`; `f(): let i = 0; while (i < 4) { defer { runs = runs + 1; } i = i + 1; if (i == 1) { continue; } if (i == 3) { break; } } return runs * 10 + i;` | exit 33 (three defer runs, one per site; baseline recorded at the phase) | §15; AR-7 |

### Phase 2 — Bounded Closure and Ambient Context (03-02)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-5 | the filed DEF-84 shape (probe `d84b`): `do { defer { if (i == 1) { continue; } } i = i + 1; } while (i < 3); return i;` | the strict host C build accepts the emitted unit and the run exits 3 (baseline: strict build fails, `label '__cl_b2' used but not defined`) | §13, §15, C99 contract; AR-6 |
| ST-6 | the while form of ST-5 | exit 3 (baseline: exit 1 — the copy swallowed the loop) | §13, §15; AR-6 |
| ST-7 | `f(): let i = 0; let n = 0; while (i < 5) { defer { if (i == 2) { break; } } n = n + 1; i = i + 1; } return n * 10 + i;` | exit 22 (the deferred break leaves the loop on the iteration where `i == 2`; baseline recorded at the phase) | §13 ("They run on `return`, `break`, and `continue`"); AR-6 |

### Phase 3 — Deferred Return Continuation and Ordering (03-03)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-8 | `f(): defer { return 7; } return 0;` (probe `d85`) | exit 7 (baseline: hang, exit 124) | §13; AR-5 |
| ST-9 | `f(): let a: String = stringOf("abc"); defer { return i32(len(a)); } return 0;` (probe `d85c`) | exit 3; host-ASan clean — the remaining drop still runs (baseline: hang) | CL-3, CL-6; AR-5 |
| ST-10 | `f(x): defer { switch (x) { case 1: { return 5; } default: { } } } return 0;` — `main` returns `f(1) * 10 + f(2)` | exit 50 (baseline: hang) | §13; AR-5 |
| ST-11 | `f(): let i = 1; defer { return 7; } defer { if (i == 99) { i = 0; } } return 0;` (probe `d87`) | exit 7 (baseline: hang) | §13; AR-5 |
| ST-12 | module counter `runs = 0`; `f(): defer { runs = runs + 1; } defer { return 7; }` — `main` returns `f() * 10 + runs` | exit 71 (the older defer still runs after the deferred return; baseline: hang) | CL-3; AR-5 |
| ST-13 | `f(): defer { let s: String = stringOf("x"); return i32(len(s)); } return 0;` (probe `d88`) | exit 1; host-ASan clean (baseline: segfault, exit 139) | §15, CL-3; AR-8 |
| ST-14 | `f(): let i = 0; while (i < 1) { let s: String = stringOf("abc"); return i32(len(s)); } return 0;` (the root-cause shape, probe `d88c`) | exit 3; host-ASan clean (baseline: segfault, exit 139) | CL-3; AR-8 |
| ST-15 | `f(): defer { return stringOf("abcdefgh"); } return stringOf("x");` — `main`: `let s: String = f(); return i32(len(s));` | exit 8; host-ASan clean — the superseded body value is released before the deferred store (added with the Phase 3 review fix; the reviewers' spec-cited reproduction) | §15, 04 §4 (assignment to a live owner); AR-4 |

> **⚠️ AUTHORING RULE:** Derive expectations from the frozen rules above. Do NOT imagine or infer
> what the implementation will produce. Each phase's red task records the exact baseline failure
> per case (the values marked "baseline" were verified in `02-current-state.md`; ST-4 and ST-7
> record theirs at the phase).

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation. Filed as `defer_integrity_spec_test.rs`.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/defer_integrity_spec_test.rs` | ST-1 – ST-4 | 03-01 per-site copies |
| `compiler/tests/defer_integrity_spec_test.rs` | ST-5 – ST-7 | 03-02 bounded closure and context |
| `compiler/tests/defer_integrity_spec_test.rs` | ST-8 – ST-14 | 03-03 deferred return |

Memory shapes (ST-1, ST-2, ST-3, ST-9, ST-12, ST-13, ST-14) additionally run through
`run_memory` + `assert_no_asan`; the strict host build is asserted by `run_host` on every case
(the "strict build accepts" expectation is explicit for ST-5).

### Implementation Tests (edge cases, internals)

> Written AFTER implementation. Filed as `defer_integrity_impl_test.rs`.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/defer_integrity_impl_test.rs` | One copy per cleanup site (count the deferred guard in the emitted C); the first-copy label spelling unchanged and later copies suffixed; the bounded copy contains no outer block; the `continue;`/`break;` spelling | High |
| `compiler/tests/defer_integrity_impl_test.rs` | The continuation shape and remaining-actions order; the buffered-cleanup order for value returns; the rung path unchanged | High |
| `compiler/tests/defer_integrity_impl_test.rs` | Byte-identical re-emission for the defect programs; defer-free programs byte-identical to the baseline | High |
| `compiler/src/**/tests.rs` (focused) | Copy-ordinal determinism; the out-edge rung guard predicate | Medium |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | whole workspace | Every existing suite plus the new ones, per phase |
| `make verify` | host + hosted-amiga + freestanding + FS-UAE | Final acceptance |

### End-to-End Tests

`N/A` — the corpus legs of `make verify` are the end-to-end level for this compiler; adding a
second harness would duplicate them (template reason recorded).

## Test Data

### Fixtures Needed

- None beyond the inline programs; the harness helpers follow the existing suites
  (`emit_host`, `run_host`, `run_memory`, `assert_exit`, `assert_no_asan`, `ScratchDir`).

### Mock Requirements

- None. The programs are self-contained Clorraine sources; the host C compiler and sanitizer are
  real.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase, per phase task)
- [ ] All specification tests pass after implementation (green phase, per phase task)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] Test coverage meets goals
