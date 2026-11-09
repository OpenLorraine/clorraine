# Testing Strategy: Borrow-View Moves and Shadowed Narrowing

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| The borrow-view move guard (`own/place.rs`) | every ST case below |
| The declaration order (`own/statements.rs`) | every ST case below |
| Regression guards | reads/lends/writes through the view, the copy-form binding, borrow parameters, un-narrowed uses |

- Test names state behavior: `should [expected behavior] when [condition]`.
- End-to-end coverage: the close-out phase runs `make verify` over every
  profile and toolchain leg.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These test cases are derived EXCLUSIVELY from the requirements
> (`01-requirements.md`), the component specs (`03-01`, `03-02`), and the
> frozen specification. They define expected behavior BEFORE any implementation
> exists.
>
> **IMMUTABLE ORACLE RULE:** Do NOT modify these expectations to match the
> implementation. If the implementation does not match a spec test case, the
> implementation is wrong — not the test.
>
> The `Source` column lives **in this plan document** — it is not a code
> comment. When the executor turns an ST case into a test, the test's in-code
> traceability comment quotes the behavior in **plain language** and never the
> `ST-` id or a `plans/` path.

> **Case numbering:** ST identifiers are stable across plan revisions; a case
> added during review takes the next unused number and sits after the cases of
> its phase. ST-13 pins the unchanged borrow-parameter guard (Phase 1) and
> ST-14 pins the `== null` else-branch shadow (Phase 2).

### Phase 1 — Borrow-view moves (`03-01`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | `eat(e)` (`take` parameter) inside `for (borrow e in xs)` for `xs: String[2]` | `E10026` (`partial moves are limited to top-level fields in v1`) | R1 · AR-3, AR-9 · `spec/03` §3–4, `spec/08` §5 |
| ST-2 | `let s: String = e;` inside the same loop | `E10026` | R1 · AR-4 · `spec/03` §4 |
| ST-3 | `let s: String = p.name;` where `p` is a borrow element of a struct array | `E10026` | R1 · AR-4 · `spec/03` §4 |
| ST-4 | `return e;` from a function returning `String`, inside the loop | `E10026` | R1 · AR-4 · `spec/03` §4 |
| ST-5 | `Holder { s: e }` aggregate-literal field inside the loop | `E10026` | R1 · AR-4 · `spec/03` §4 |
| ST-6 | `vlen(e)` lend plus a field read inside a borrow element loop over `String[2]` | accepted; the sum of both lengths (`3`) is returned | R4 · AR-13 · `spec/08` §5 |
| ST-7 | `row[0] = row[0] + 10;` inside `for (borrow row in m)` for `m: i32[2][3]` | accepted; `m[0][0] + m[1][0] == 25` (writes reach the array) | R4 · AR-13 · `spec/08` §5 |
| ST-8 | `let copy: i32[3] = row;` inside `for (row in m)` (copy-form binding) | accepted; the copy is an owned value | R4 · AR-13 · `spec/08` LOOP-1 |
| ST-13 | A move out of a `borrow` parameter (not a view): a `take` argument, an owned-local binding, or a return through it | `E10034` (`cannot release or store borrowed parameter '{name}'`) | R4 · AR-13 · `spec/16` §2, §5 (BR-5) · `spec/14` |

### Phase 2 — Shadowed narrowing (`03-02`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-9 | `let b: Buffer? = bufferNew(8); if (b != null) { let b: Buffer = b; }` | accepted (no `E10010`) | R2 · AR-5 · `spec/16` |
| ST-10 | The same with `const b: Buffer = b;` | accepted (no `E10010`) | R3 · AR-6 · `spec/16` |
| ST-11 | `use(b)` on a nullable binding with no `!= null` narrowing | `E10010` (`value 'b' may be null here`) | R4 · AR-13 · `spec/14` |
| ST-12 | `if (b != null) { use(b); }` (no shadowing) | accepted | R4 · AR-13 · `spec/16` |
| ST-14 | `if (b == null) { return 1; } else { let b: Buffer = b; ... }` (shadow in the else branch) | accepted (no `E10010`) | R2 · AR-5 · `spec/02` §5, `spec/16` §2 |

### Recorded red-phase predictions

The suite is expected to fail before the fixes exactly as follows; a deviation
is an oracle or test defect, never a reason to change a case.

| State | Cases | Observed pre-fix (2026-09-29, host) |
| ----- | ----- | ----------------------------------- |
| Fails: missing rejection | ST-1 – ST-5 | `clc` accepts every move form |
| Fails: spurious diagnostic | ST-9, ST-10, ST-14 | `E10010` on the shadowing initializer |
| Passes pre-fix (guards) | ST-6 – ST-8, ST-11 – ST-13 | lends, writes, copy-form bindings, the borrow-parameter rule, and the existing null rules already correct |

## Test Categories

### Specification Tests (from ST cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/borrow_view_and_shadowing_spec_test.rs` | ST-1 – ST-14 | `03-01`, `03-02` |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/borrow_view_and_shadowing_impl_test.rs` | Single-report checks for the view rejection and the shadowing diagnostic; the shadowing binding is usable after its declaration; the borrow-parameter `E10034` path is unchanged; unit coverage in `own::tests` for the view guard and the copy-form binding | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | all | Spec check plus the full Rust workspace test run |
| `make verify` | all | Every hosted-amiga and freestanding leg |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Cross-profile build | `make verify` | all legs passed |

## Test Data

- Scratch directories per case, following the existing helper pattern in
  `array_value_integrity_spec_test.rs` (`ScratchDir`, `emit_host`,
  `build_host_c`, `run_with_timeout`, `run_host`, `assert_exit`,
  `assert_rejected`, `assert_trap`).
- Timeout for every run case: 30 seconds.
- No mocks; the real compiler and C toolchain run.

## Verification Checklist

- [x] All specification test cases (ST-*) defined with concrete input/output pairs
- [x] Every ST case traces to a requirement, spec doc, or AR entry
- [x] Specification tests written BEFORE implementation
- [x] Specification tests verified to FAIL before implementation (red phase)
- [x] All specification tests pass after implementation (green phase)
- [x] Implementation tests written for edge cases and internals
- [x] All unit / integration / E2E tests pass
- [x] No regressions in existing tests
- [x] Test coverage meets goals
