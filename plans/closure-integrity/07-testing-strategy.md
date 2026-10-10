# Testing Strategy: Closure Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Every covered behavior | one or more ST cases (compile/run/emit assertions) |
| Memory-sensitive shapes (DEF-104, DEF-109) | host ASan probes (AR-13) |
| New diagnostics (E10406, E10307) | registry row + golden + driver assertion + spec case |
| Reports (E10401, E10402, E10405) | exact code, message fragment, and count per site |

- Test names state behavior ("rejects … once", "builds and runs", "reports the enclosing list").
- Integration coverage is the suite itself: every accepted case builds the emitted C under the
  strict flags and runs it. E2E: N/A — the compiler's integration oracle is the emitted C plus the
  run; no new harness (AR-13).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the requirements ([01](01-requirements.md)), the component specs
> ([03-01](03-01-capture-listing-and-validity.md) – [03-03](03-03-callable-bounds.md)), the frozen
> specification (L-2, L-3, L-4, E10405's listing rule, G-1/G-6), and the register entries
> ([00-ambiguity-register.md](00-ambiguity-register.md)). Expectations are the ruled behavior,
> never the current implementation.
>
> **IMMUTABLE ORACLE RULE:** if the implementation does not match an ST case, the implementation
> is wrong — not the test. A case that already passes before its phase's implementation is a
> deliberate regression pin (recorded in the red-phase evidence).

### Phase 1 — Capture listing and validity (DEF-92, DEF-93, DEF-94)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | `let n = 2; let m = 3; ([n] () => ([m] () => m)())()` | rejected: exactly one `E10405` naming `m`; no translation unit | AR-4, AR-9 |
| ST-2 | `([n, m] () => ([m] () => m)())()` | accepted; strict build; runs (exit = the outer call's value from the nested capture) | AR-4, AR-9 |
| ST-3 | a module `let g: i32 = 5;` with `([g] () => g)()` | rejected: exactly one `E10406` naming `g`; no translation unit | AR-2, AR-8 |
| ST-4 | a module `const base: i32 = 7;` with `([base] () => base)()` | rejected: exactly one `E10406` naming `base` | AR-2 |
| ST-5 | the same module `const` read capture-less: `(() => base)()` | accepted; strict build; runs (exit 7) | AR-2 (negative) |
| ST-6 | `let p: P = P { x: 1 }; ([p] () => { p.x = 2; })();` | rejected: exactly one `E10402` naming `p`; no translation unit; `E10024` absent | AR-6 |
| ST-7 | the same with `[borrow p]` | accepted; strict build; runs; the write reached the owner | AR-6 (control) |
| ST-8 | `([p] () => p.x)()` | accepted; strict build; runs (copy-capture read unchanged) | AR-6 (control) |
| ST-9 | the golden program `E10406_module_capture.clo` compiled directly | rejected: exactly one `E10406` (mirrors `./spec/check.sh`) | AR-12 |

### Phase 2 — Captured-owner integrity (DEF-104, DEF-109)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-10 | `func outer<T>(x: T): T { return ([x] () => x)(); }` instantiated at a struct holding a `String` | rejected: exactly one `E10401` naming `x`; no translation unit; ASan never runs (no unit) | AR-5, AR-11 |
| ST-11 | the concrete owning spelling `func outer(x: Wrap): Wrap { return ([x] () => x)(); }` | rejected: exactly one `E10401` naming `x` | AR-5, AR-11 |
| ST-12 | the same generic shape instantiated at `i32` | accepted; strict build; runs (the copy is sound) | AR-5 (control) |
| ST-13 | a local `run: func(borrow String): String = run` (same name as the module lender), `([run] () => run(stringOf("ab")))()` | accepted; strict build; runs; ASan-clean (the result released once) | AR-7, AR-10 |

### Phase 3 — Callable bounds (DEF-110)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-14 | `func g<T>(x: T): i32 { return x(5); }` instantiated with a struct | rejected: exactly one `E10307` naming `T`; no translation unit | AR-3 |
| ST-15 | `func h<T: func(i32): i32>(x: T, n: i32): i32 { return x(n); }` called with a matching lambda | accepted; strict build; runs (exit = the lambda's result) | AR-3 (control) |
| ST-16 | the golden program `E10307_unbounded_call.clo` compiled directly | rejected: exactly one `E10307` (mirrors `./spec/check.sh`) | AR-12 |

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation per phase, from this document only.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/closure_integrity_spec_test.rs` | ST-1 – ST-9 (P1), ST-10 – ST-13 (P2), ST-14 – ST-16 (P3) | all three component specs |

### Implementation Tests (edge cases, internals)

> Written AFTER implementation, in `compiler/tests/closure_integrity_impl_test.rs`: the report
> count and span per shape (including a three-level relay reporting once), the dotted-root
> resolution, the duty naming per instance, the masked capture IR name with unchanged source-name
> diagnostics, the release counts on ST-13's shape, and byte-identical re-emission.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/closure_integrity_impl_test.rs` | report shapes, duty/spelling internals, re-emission determinism | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `./spec/check.sh` | registry + goldens | the two new goldens participate in the frozen-suite check |
| `make check` / `make verify` | workspace + cross-toolchain legs | the repo's acceptance gates (AR-13) |
| host ASan probes | `clc` + `cc -fsanitize=address` | the DEF-104 (no unit) and DEF-109 (release-once) memory oracle |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| N/A | — | the compiler's integration oracle is the emitted C built and run inside each suite (AR-13); no new E2E harness |

## Test Data

### Fixtures Needed

Inline program sources per case (the established suite style); no fixture files.

### Mock Requirements

None — the real compiler binary (`CARGO_BIN_EXE_clc`) and the host `cc` are used.

## Verification Checklist
- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase; passing pins recorded with justification)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] Host ASan probes clean for the memory cases
