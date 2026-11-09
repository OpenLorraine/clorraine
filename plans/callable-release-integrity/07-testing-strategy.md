# Testing Strategy: Callable Release Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Ownership and identity (138, 140, 165) | Reproduction red at HEAD; ASan/strict-build green after |
| Callable effect modes (155 – 157, 166) | Twin parity (same reports/behavior as the module spelling) + ASan runs |
| Owner releases (139, 158, 159, 163, 164) | ASan runs + diagnostic parity for the view-array case |
| Possibly-null widening (141) | The `E10010` report + accepted controls |
| Regression safety | Byte-identity/strict-build sweeps outside the intended deltas |

Spec-first ordering applies: each phase's specification tests are written first and must fail on
the pre-fix head (AR-14); expectations come from the frozen rules and the filed reproductions,
never from imagined implementation behavior.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Fixtures mirror the filed reproductions (`plans/eval-emission-integrity/05-findings.md`); each
> is re-derived against the committed compiler in the red phase before its fix.

### Generated identities (`callable_release_integrity_spec_test.rs`)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-1 | A user local spelled `__cl_owned0` plus a call passing an owned temporary to a `borrow` parameter | Accepted; strict build passes; runs with correct values; ASan-clean (red at HEAD: double-free) | R1, AR-2 |
| ST-2 | `Holder<func(i32): i32>` and `Holder<func(u32): u32>` instances of one generic struct | Accepted; strict build passes; runs; two distinct struct definitions (impl pin) (red at HEAD: redefinition) | R3, AR-4 |
| ST-3 | `([s] () => i32(len(s)))()` inline capturing lambda | Accepted; strict build passes; runs; one env struct definition (impl pin) (red at HEAD: duplicate definition) | R12, AR-5 |

### Callable effect modes

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-4 | Func-typed field calls vs their module twins: `take` (move + `E10020` on reuse), `borrow` (the twin's `E10032` in the conflicting shape), plain with an owned temporary argument | Twin parity per pair; accepted runs ASan-clean | R5, AR-6 |
| ST-5 | A module-level `func(take String): i32` global call (move + reuse `E10020`) and an owned temporary argument | Twin parity; ASan-clean | R6, AR-6 |
| ST-6 | A `Param`-typed callee at an owning instance (owned temporary released) and a parameter shadowing a same-named `take` module function | ASan-clean; no false `E10020`; the module twin still reports its `E10020` | R7, AR-6 |
| ST-7 | `((p: T) => Holder<T> { v: p })(x)` reached at a `T = String` instance, its block-bodied control, and a lend-bodied lambda with an owned temporary argument | The filed shape and its controls report exactly one `E10034` (module-twin parity; red at HEAD: the expression-bodied shape is accepted and double-frees); the owned temporary argument to a lend-bodied lambda is accepted, runs, and is ASan-clean (red at HEAD: 10 B leaked) | R13, AR-6, AR-17 |

### Owner releases

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-8 | `H { call: addOne, s: stringOf("x") }.call(5)` and `makeH().call(5)` | ASan-clean; values correct (red at HEAD: leaks) | R2, AR-7 |
| ST-9 | A view binding from an owned projection temporary inside a `defer` body; a func-value call with an owned temporary inside a `defer` | ASan-clean per execution (red at HEAD: leaks) | R8, AR-7 |
| ST-10 | `(c ? b : makeBox()).v` and `c ? (P { a, b }).a : (Q { a, b }).a` in a loop | ASan-clean; the both-call and both-place spellings unchanged | R9, AR-7 |
| ST-11 | `func make(): str[1] { return […temporary…]; }` vs the single-view spelling of the same derivation; plus an array derived from exactly one lend parameter | Diagnostic parity between the two rejecting spellings (same code, count per site); the lend-derived control builds and runs | R10, AR-8 |
| ST-12 | `make();` as an expression statement (owned result discarded) in a loop | ASan-clean (red at HEAD: leak) | R11, AR-7 |

### Possibly-null widening

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-13 | `let s: String? = null; len(s);` (the widening use), the narrowed control, and the non-nullable control | One `E10010` at the use with the existing message/help; controls build and run (red at HEAD: the first is accepted and segfaults) | R4, AR-9 |

## Test Categories

### Specification Tests

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/callable_release_integrity_spec_test.rs` | ST-1 – ST-13 | All four design docs |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/callable_release_integrity_impl_test.rs` | Generated-binding absence from the source name map; the C rename spelling; func keys; env definition counts; mode vectors per callee kind; release counts and shapes; report counts; byte-identity pins | High |
| `compiler/src/**` `mod tests` | Unit tests where the mechanism lives (keys, lowering) | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Corpus + strict-build sweep | All profiles | Every accepted corpus unit builds; no head-only failure; byte-identity outside intended deltas |
| `RDC_ASAN=1` suite | The new spec cases | Every accepted defect shape runs ASan-clean |

## Test Data

### Fixtures Needed

| Fixture | Used by | Content |
| ------- | ------- | ------- |
| The 13 filed reproductions | ST-1 – ST-13 | Derived from `05-findings.md`; re-derived red at HEAD |
| Twin controls | ST-4 – ST-6 | The module spelling of each callee shape |

### Mock Requirements

None — real binaries, real compiler, host ASan inside the pinned container.

## Verification Checklist

- [x] All 13 specification cases defined with concrete input/output pairs
- [x] Every case traces to a requirement or AR entry
- [x] Specification tests written BEFORE implementation; red phase recorded per phase
- [x] Specification tests pass after implementation (green phase)
- [x] Implementation tests written for internals and edge cases
- [x] `make check` and `make verify` pass; the strict-build sweep shows no head-only failure
- [x] No regressions: byte-identity outside the intended deltas
