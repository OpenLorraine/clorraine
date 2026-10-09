# Testing Strategy: Borrow/Generic Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

The repo's convention applies (specification tests derived from the frozen spec plus implementation
tests for internals); no numeric coverage target is set beyond the plan's acceptance criteria
(AR-12). Every ST-case below is a concrete input → expected-behavior pair traced to a spec rule or
an AR entry.

- Test names state behavior (`should ... when ...`), matching the existing suites.
- The host ASan oracle (build the emitted C with `-fsanitize=address`, run with leak detection)
  covers the ownership shapes; the strict host build (`-std=c99 -O0 -Wall -Wextra -Werror`)
  covers the emitted-C shapes.
- E2E: `N/A` for a new harness — the existing `make verify` legs remain the acceptance run; the
  spec suite's compile-and-run cases are the end-to-end path for this change surface.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the frozen specification and the Ambiguity Register. The immutable
> oracle rule applies: a failing spec test means the implementation is wrong, never the test.

### P1 — Borrow-store rules (03-01)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-1 | `func fill(take v: String, borrow slot: String): void { slot = v; }` plus a caller | Rejected with exactly one `error E10034` naming `slot` | spec/03 OW-16, spec/06 FN-7 · AR-2, AR-13 |
| ST-2 | `func setF(borrow slot: String): void { slot = stringOf("x"); }` plus a caller | Rejected with exactly one `E10034` (the offense is the target release, not the right side) | spec/03 OW-16 · AR-2 |
| ST-3 | `func copy(borrow v: String, borrow slot: String): void { slot = v; }` plus a caller | Rejected with exactly one `E10034` at the assignment site (no duplicate from the two judging paths) | spec/03 OW-16 · AR-2, AR-13 |
| ST-4 | `func fill<T>(take v: T, borrow slot: T): void { slot = v; }` called with `String` | Rejected with exactly one `E10034` for the owning instantiation | spec/03 OW-16 · AR-2 |
| ST-5 | The same body called with `i32` | Accepted; the call warns `W10001` (a copy is written) | spec/decisions D9 · AR-2 |
| ST-6 | `func fillString(take v: String, borrow slot: String?): void { slot = v; }` and the `borrow slot: T[]?` instance (`p3r26` shape) called and run | Accepted; the run is ASan-clean and the caller observes the stored value | spec/16 out-parameter carve-out · AR-2 |
| ST-7 | `func self(borrow slot: String?): void { slot = slot; }` plus a caller | Rejected with `error E10020` naming `slot` (the local self-assignment diagnostic) | spec/03 E10020 rule · AR-8 |
| ST-8 | `func set(borrow x: raw u8?): void { x = null; }` called with a place, and `func bump(borrow p: i32)` called the same way | The nullable out-slot call produces no diagnostics; the `i32` call warns `W10001` | spec/decisions D9, spec/16 · AR-9 |

### P2 — Instance keys (03-02)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-9 | `struct Wrap<T> { v: T; }` used as `Wrap<String>` and `Wrap<String?>` in one program | Compiles; the emitted C defines two distinct types; the strict host build passes; a run reads both fields | spec/07 §9 · AR-4 |
| ST-10 | `pick<String>` and `pick<String?>` function instances in one program | Both compile and run (the function-instance spelling is unchanged) | spec/17 §7 · AR-4, AR-6 |
| ST-11 | `Outer<Wrap<String?>>` versus `Outer<Wrap<String>>`, and a `String?[]` slice, in one program | Compiles; distinct symbols throughout; the strict host build passes | spec/07 §9 · AR-4 |
| ST-12 | Emitting the same program twice | The two emissions are byte-identical | spec/17 determinism · AR-4 |

### P3 — Nullable view comparison (03-03)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-13 | `xs == null` and `null == xs` for `xs: u8[]?`, with `!=` forms | Accepted (both operand orders) | spec/decisions T8 · AR-7 |
| ST-14 | `if (xs != null) { return i32(xs[0]); }` and the `== null` else form, run | Accepted; the narrowed read returns the first element (run exit code pinned) | spec/decisions T8, spec/02 TY-6 · AR-7 |
| ST-15 | `xs == ys` (two views) and `xs < ys` | `E10222` on the view operand, unchanged | spec/04 operator table · AR-7 |
| ST-16 | A non-nullable `u8[]` compared with `null` via `==` and `!=`, run | Accepted; `==` yields false and `!=` true at run time | spec/decisions T8 · AR-7 |

### P4 — May-lend callee identity (03-04)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-17 | `func demo(give: func(borrow String): String)` calling `len(give(s))`, with a module function `give` in scope | Accepted; the host ASan run reports no leak (the owned result is bound and released once) | spec/03 loan rules · AR-3 |
| ST-18 | The receiver case (a func-typed field call whose field name equals a module lender) | Accepted; ASan-clean (existing behavior stays) | spec/03 loan rules · AR-3 |
| ST-19 | A genuine module lender call `lend(s)` with the owned argument; the result used and the argument released once | Accepted; ASan-clean; one release of `s` | spec/03 reborrow rules · AR-3 |
| ST-20 | A non-func parameter named like a module callee plus a genuine module call of that name in the same body | Accepted; the call reaches the module function (run exit code pinned) | AR-3 |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/borrow_generic_integrity_spec_test.rs` | ST-1 – ST-20 | P1–P4 |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/borrow_generic_integrity_impl_test.rs` | Emitted shapes for the accepted out-slot stores (target release absent/legal), the `.ptr` comparison shape, the distinct struct typedefs and unchanged function-instance symbols, the suffixed IR/emitted names for colliding bindings, one-report counts, and byte-identical re-emission | High |

### Integration Tests

The per-phase `make check` (workspace suites) is the integration surface; the base-versus-head
emission sweep for P2 is the emission-integrity review's artifact (AGENTS.md dispatch).

### End-to-End Tests

`N/A` — no new harness; `make verify` (all legs) is the final acceptance run (AR-12).

## Test Data

### Fixtures Needed

None beyond inline programs; the suites reuse the repo's existing compile-and-run and ASan probe
patterns (`codegen_run_spec_test.rs`, `def_backlog_closeout_spec_test.rs`).

### Mock Requirements

None — real compiler, real host C compiler, real AddressSanitizer.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration tests pass
- [ ] No regressions in existing tests
