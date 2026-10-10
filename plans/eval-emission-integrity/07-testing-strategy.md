# Testing Strategy: Evaluation and Emission Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Every covered behavior | one or more ST cases (compile/run/emit assertions) |
| Memory-sensitive shapes (DEF-95, 96, 121, 125, 127) | host ASan probes |
| New/edited diagnostics (E10043, E10042, E10212) | registry row + golden + driver assertion + spec case |
| Emission-order and name fixes (DEF-112, 120, 128) | strict builds, byte-identity pins, corpus legs |

- Test names state behavior ("rejects … once", "builds and runs", "releases once").
- Integration coverage is the suite itself: every accepted case builds under `-Wall -Wextra
  -Werror` (or the freestanding flags) and runs. E2E: the corpus and profile legs at close-out
  (AR-B14).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the requirements ([01](01-requirements.md)), the component specs
> ([03-01](03-01-module-initializer-integrity.md) – [03-05](03-05-ownership-lifetimes.md)), the
> frozen specification, and the register entries ([00-ambiguity-register.md](00-ambiguity-register.md)).
> Expectations are the ruled behavior, never the current implementation.
>
> **IMMUTABLE ORACLE RULE:** if the implementation does not match an ST case, the implementation
> is wrong — not the test. A case that already passes before its phase's implementation is a
> deliberate regression pin (recorded in the red-phase evidence).

### Phase 1 — Module-initializer integrity (ST-1 – ST-7)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | whole-value init `let a: i32[2] = mk();` (module binding) | accepted; strict build; runs (reads `a[1]`) | R1 |
| ST-2 | element projection `let b: i32 = mk()[1];` | accepted; builds; runs | R1 |
| ST-3 | member projection `let c: i32 = mkBox().v;` | accepted; builds; runs | R1 |
| ST-4 | user-fn argument `eat(mk())` and intrinsic argument `slice(mk(), 0, 2)` | accepted; builds; runs | R1 |
| ST-5 | conditional arm `let d: i32[2] = pick() ? mk() : y;` (each branch) | accepted; builds; runs both branches | R1 |
| ST-6 | DEF-121 counting `let v: u8 = s[idx()];` | builds; `idx` runs once; ASan-clean (baseline: global-buffer-overflow) | R2 |
| ST-7 | `let x: i32[] = [1, 2];` vs `let y: i32[] = slice(arr, 0, 2);` | one `E10200`, no TU; the `slice()` form accepted and runs | R3 |

### Phase 2 — Strict-build emission shapes (ST-8 – ST-16)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-8 | DEF-76 generic `len(mkArr(x))` | builds under `-Werror`; runs (structural extent) | R4 |
| ST-9 | DEF-97 reproduced from `codegen_run_spec_test` | builds; runs; the recorded suite green | R5 |
| ST-10 | DEF-80 owned struct empty return `return;` | builds; runs (zeroed fields) | R6 |
| ST-11 | DEF-90 freestanding `bool` program | host no-libc and m68k link builds; runs | R7 |
| ST-12 | DEF-98 view to an uncontracted extern | builds; runs (extern size) | R8 |
| ST-13 | DEF-112 `__cl_ret` parameter and `__env` local | builds; runs (values intact) | R9 |
| ST-14 | DEF-120 `let rows: P[][2] = …` | builds; runs (row lengths) | R10 |
| ST-15 | DEF-120 pointer-element slice control | builds; unchanged emission (byte-identity pin) | R10 |
| ST-16 | DEF-128 bounded field call `use<func(i32): i32>(Holder{call: addOne})` | builds; runs (exit 6) | R11 |

### Phase 3 — Value typing and checker conformance (ST-17 – ST-25)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-17 | DEF-111 `takeBox<Box>(b, 3)` with `len(b.s)` | builds; returns the field length (7), not the fallback | R13 |
| ST-18 | DEF-99 the reproduced growable-slice `len` shape | builds; the real length (not 0) | R12 |
| ST-19 | DEF-108 `let x = null;` | accepted; strict build; runs | R14 |
| ST-20 | DEF-108 pairing `if (x == s)` with a `str` | accepted; runs | R14 |
| ST-21 | DEF-107 `a == b` on two `str` | one `E10222`; no TU | R15 |
| ST-22 | DEF-107 untyped-operand spelling `let x; x = 5; if (x == s)` | one `E10222` | R15 |
| ST-23 | DEF-118 `for (borrow c in s)` with `s: str` | one `E10043`; fixed-array and slice loops accepted (control) | R16 |
| ST-24 | DEF-118 write-through `c = 65;` inside the loop | one `E10043` | R16 |
| ST-25 | DEF-129 `x.foo(5)` with `x: T` | one `E10212`; concrete and callable-bound controls accepted | R17 |

### Phase 4 — Write-mutability rules (ST-26 – ST-31)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-26 | value update on `const xs: u8[]` | one `E10023`; no TU | R18 |
| ST-27 | value update on an unmarked parameter | one `E10024` | R18 |
| ST-28 | value update on `borrow s: str` | one `E10042` (unchanged pin) | R18 |
| ST-29 | `c[0] = 65;` on `borrow c: cstring` | one `E10042` | R19 |
| ST-30 | statement forms unchanged (const assign `E10023`; writable `u8[]` write accepted and runs) | one report / accepted | R18 |
| ST-31 | whole-view rebind and writable slice-write controls | accepted; run | R19 |

### Phase 5 — Ownership lifetimes and releases (ST-32 – ST-41)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-32 | DEF-95 `let s: str = (Box { v: mk().a }).v;` per loop | runs; ASan-clean (baseline: one leak per iteration) | R20 |
| ST-33 | DEF-95 contrast: the existing `String`-destination loop | runs; ASan-clean (control) | R20 |
| ST-34 | DEF-96 deferred `break` preempting an ordinary return | runs; ASan-clean (baseline leak) | R21 |
| ST-35 | DEF-96 ordinary-return control | byte-identical; runs | R21 |
| ST-36 | DEF-125 `use(copier)` with `f(stringOf("x"))` | runs; ASan-clean (baseline: 10-byte leak) | R22 |
| ST-37 | DEF-126 block-bodied return of an owning capture | one `E10401`; no TU | R23 |
| ST-38 | DEF-126 block-bodied take of an owning capture | one `E10401` | R23 |
| ST-39 | DEF-127 expression-bodied `c ? x : x` at `outer<String>` | one `E10401` | R24 |
| ST-40 | DEF-127 concrete and take-argument variants | one `E10401` each | R24 |
| ST-41 | DEF-127 duplicable control `outer<i32>` | accepted; runs | R24 |

## Test Categories

### Specification Tests

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/eval_emission_integrity_spec_test.rs` | ST-1 – ST-41 (per phase) | all five component specs |

### Implementation Tests (edge cases, internals)

> Written AFTER each phase's implementation, in `compiler/tests/eval_emission_integrity_impl_test.rs`:
> the destination-carrying emission per init position, the shared guard operand, the absent
> `__cl_owned` binding, the `cl_zero_*` spellings, the seeded de-dup spelling, the typedef order,
> the bound-field call shape, the projection spelling per instance, the one-report counts and
> spans, the owner binding and release counts, the preemption-edge drop, and byte-identity where
> pinned.

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `./spec/check.sh` | registry + goldens | the E10043 golden and the edited rows participate |
| the affected recorded suites | `codegen_run_spec_test`, `store_emission_integrity_*`, `defer_integrity_*` | the recorded shapes build green |
| `make check` / `make verify` | workspace + cross-toolchain legs | the repo's acceptance gates (AR-B14) |
| host ASan probes | `clc` + `cc -fsanitize=address` | the memory oracle for R2, R20–R22, R24 |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| N/A | — | the compiler's integration oracle is the emitted C built and run inside each suite (AR-B14); no new E2E harness |

## Test Data

### Fixtures Needed

Inline program sources per case (the established suite style); the freestanding leg uses the
existing profile harness.

### Mock Requirements

None — the real compiler binary and the host `cc` are used.

## Verification Checklist
- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation where the baseline misbehaves (red phase; passing pins recorded with justification)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] Host ASan probes clean for the memory cases
