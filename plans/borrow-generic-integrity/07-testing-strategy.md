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
- The host ASan oracle covers the ownership shapes and the P4 leak: run the memory cases with
  `make -C docker exec CMD="RDC_ASAN=1 cargo test --test borrow_generic_integrity_spec_test"`
  (the env belongs inside the container command; a plain run cannot observe a leak). Run memory
  probes individually — full-suite sanitizer runs in the container are timing-flaky. The strict
  host build (`-std=c99 -O0 -Wall -Wextra -Werror`) covers the emitted-C shapes.
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
| ST-3 | `func copy(borrow v: String, borrow slot: String): void { slot = v; }`, the generic `copy<T>(borrow v: T, borrow slot: T)` at `String`, and the mixed conditional `slot = c ? local : v`, each plus a caller | Rejected with exactly one `E10034` per site (the spellings pass at baseline too — dedupe regression pins) | spec/03 OW-16 · AR-2, AR-13, PF-004 |
| ST-4 | `func fill<T>(take v: T, borrow slot: T): void { slot = v; }` called with `String` | Rejected with exactly one `E10034` for the owning instantiation | spec/03 OW-16 · AR-2 |
| ST-5 | The same body called with `i32` | Accepted; the call warns `W10001` (a copy is written) | spec/decisions D9 · AR-2 |
| ST-6 | `func fillString(take v: String, borrow slot: String?): void { slot = v; }`, the `borrow slot: T[]?` instance (`p3r26` shape), and `copy<T>(borrow v: T, borrow slot: T)` at `T = String?`, each called and run | Accepted; the runs are ASan-clean (`RDC_ASAN`) and the caller observes the stored value (the concrete clauses pass at baseline; the generic-nullable clause is red — it is rejected `E10034` today; PF-006) | spec/16 out-parameter carve-out · AR-2, PF-006 |
| ST-7 | `func self(borrow slot: String?): void { slot = slot; }` plus a caller; and the projected form `if (xs != null) { xs[0] = xs[0]; }` on `borrow xs: T[]?` at `String` (a P3 clause) | Both rejected with `error E10020` naming the binding (the local moved-value message) | spec/03 E10020 rule · AR-8, PF-005 |
| ST-8 | `func set(borrow x: raw u8?): void { x = null; }` called with a place; `func setIt<T>(take v: T, borrow slot: T)` called as `setIt(p, borrow r)` with `raw u8?` arguments (inferred) and with explicit `<raw u8?>`; and `func bump(borrow p: i32)` called the same way | The nullable out-slot calls produce no diagnostics (explicit and inferred); the `i32` call warns `W10001` | spec/decisions D9, spec/16 · AR-9, PF-002 |
| ST-21 | The generic `moveout<T>(a: T)` with `let b: T = a;` (by-value and `borrow` source forms) at `T = String?` | Rejected with exactly one `E10034` naming the parameter — the nullable exemption covers only a store into the callee's out-parameter slot (phase-1 review: SR-001, MS-001) | spec/03 OW-16 · AR-2, PF-006 |
| ST-22 | The generic `gid<T>(borrow v: T): T { return v; }` at `T = String?` | Rejected with exactly one `E10034` naming the parameter; the concrete spelling's aliasing release bug is filed as DEF-101 (phase-1 review: SR-001, MS-001) | spec/03 OW-16, spec/06 FN-7 · AR-2 |
| ST-23 | The generic `put<T>(borrow slot: T, v: T)` at `T = String?` with a by-value (`v: T`) or `const` source | Rejected with exactly one `E10034` naming the source — the exemption needs both a writable lend source and a slot destination (phase-1 re-review: MS-003) | spec/03 OW-16 · AR-2 |

### P2 — Instance keys (03-02)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-9 | `struct Wrap<T> { v: T; }` used as `Wrap<String>` and `Wrap<String?>` in one program | Compiles; the emitted C defines two distinct types; the strict host build passes; a run reads both fields | spec/07 §9 · AR-4 |
| ST-10 | `pick<String>` and `pick<String?>` function instances, plus `pick<Wrap<String>>` and `pick<Wrap<String?>>` (nested) in one program | All compile and run; top-level spellings unchanged, the nested pair gains distinct `Q` keys (today the nested pair collides — the strict build fails; PF-003) | spec/17 §7 · AR-4, AR-6, PF-003 |
| ST-11 | `Outer<Wrap<String?>>` versus `Outer<Wrap<String>>`, and the function-instance slice pair `pick<String[]?>` versus `pick<String[]>` (a struct field cannot hold a view — `E10205` — so the slice witness rides a type argument), in one program | The nested struct pair gains distinct symbols (both key `Outer__Wrap__String` today — a strict-build redefinition); the top-level slice pair stays distinct; a nullable element type is not expressible — `String?[]` is a syntax error (PF-009) | spec/07 §9 · AR-4, PF-009, AR-15 |
| ST-12 | Emitting the same program twice | The two emissions are byte-identical | spec/17 determinism · AR-4 |

### P3 — Nullable view comparison (03-03)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-13 | `xs == null` and `null == xs` for `xs: u8[]?`, with `!=` forms | Accepted (both operand orders) | spec/decisions T8 · AR-7 |
| ST-14 | `if (xs != null) { return i32(xs[0]); }` and the `== null` else form, run | Accepted; the narrowed read returns the first element (run exit code pinned) | spec/decisions T8, spec/02 TY-6 · AR-7 |
| ST-15 | `xs == ys` (two slice views) and `xs < ys` | `E10222` on the view operand, unchanged | spec/04 operator table · AR-7 |
| ST-16 | A non-nullable `u8[]` compared with `null` via `==` and `!=`, run | Accepted; `==` yields false and `!=` true at run time | spec/decisions T8 · AR-7 |

### P4 — May-lend callee identity (03-04)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-17 | `func demo(give: func(borrow String): String)` calling `len(give(s))`, with a module function `give` in scope | Accepted; the host ASan run reports no leak (the owned result is bound and released once) | spec/03 loan rules · AR-3 |
| ST-18 | The receiver case (a func-typed field call whose field name equals a module lender) | Accepted; ASan-clean (existing behavior stays) | spec/03 loan rules · AR-3 |
| ST-19 | A genuine module lender call `lend(s)` with the owned argument; the result used and the argument released once | Accepted; ASan-clean; one release of `s` | spec/03 reborrow rules · AR-3 |
| ST-20 | The module lender `give` called by its written name in one function (no colliding binding in scope, returning the lender's value), plus a sibling function declaring a non-func parameter named `give` | Accepted; the module call reaches the lender (run exit code pins its value) and the sibling function's emitted C builds | AR-3, PF-008 |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/borrow_generic_integrity_spec_test.rs` | ST-1 – ST-20 | P1–P4 |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/borrow_generic_integrity_impl_test.rs` | Emitted shapes for the accepted out-slot stores (target release absent/legal), the emitted `.ptr` comparison, the distinct struct typedefs, top-level function-instance byte pins and the nested `Q` key change, the suffixed IR/emitted names for colliding bindings (including the updated pins of the two existing tests), a loop-body store with exactly one `E10034`, the `Q`-marker limitation note, and byte-identical re-emission | High |

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
