# Testing Strategy: Borrow-Store Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Core rules (self-assign, null requirement, bounds guard, return move-out) | every ST case below |
| Emitted shapes (statement spellings, guard lines, return forms) | implementation pins per phase |
| Memory safety | ASan probes on every accepted owning shape (`RDC_ASAN`, `-no-pie`) |

> **Memory-oracle note:** the in-container `RDC_ASAN=1` full-suite run is a convenience run, not a
> gate — it is timing-flaky in this environment (same policy as the previous cycles). The phase
> memory oracle is the per-case host probe: emit with `clc`, build the TU with
> `cc -std=c99 -g -fsanitize=address -no-pie`, and run under `ASAN_OPTIONS=detect_leaks=1`.

- Test names state behavior: `should [expected behavior] when [condition]`.
- Integration: the per-phase `make check` workspace run; E2E: `make verify` (all legs) at plan
  completion — no new harness (AR-11).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These test cases are derived exclusively from the requirements (`01-requirements.md`), the
> component specs (`03-XX`), the frozen specification, and the Ambiguity Register. They define
> expected behavior BEFORE any implementation exists.
>
> **IMMUTABLE ORACLE RULE:** do NOT modify these expectations to match the implementation. If the
> implementation does not match a spec test case, the implementation is wrong — not the test.

### P1 — Self-assign rules (03-01)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-1 | `func copySelf<T>(borrow slot: T): void { slot = slot; }` called as `copySelf<String>(borrow s)` | Rejected with exactly one `error E10034` naming `slot` (the concrete owning spelling's diagnostic) — red at baseline (accepted; ASan use-after-free and double free) | spec/03 OW-16 · AR-2, AR-7 |
| ST-2 | `func same<T>(borrow p: T): void { p = p; }` called as `same<i32>(n)` with `let n: i32 = 1`, run | Accepted; runs to exit 1 (the duplicable instance copies) — green pin | spec/decisions D9 · AR-7 |
| ST-3 | The same `copySelf` body called as `copySelf<String?>(borrow s)` with `let s: String? = stringOf("ab")` | Rejected with exactly one `error E10020` naming `slot` (the out-slot self-assign message) — red at baseline | spec/03 E10020 rule · AR-7 |
| ST-4 | The concrete `func copySelf(borrow slot: String): void { slot = slot; }` | Rejected with exactly one `E10034` — green pin of the concrete rule | spec/03 OW-16 · AR-2 |
| ST-5 | `if (xs != null) { xs[i] = xs[i]; }` on a `i32[]?` slot with `let i: i32 = 0` | Rejected with exactly one `E10020` naming `xs` — green pin (the filed "unguarded" premise does not reproduce) | spec/03 E10020 rule · AR-4 |
| ST-6 | `if (xs != null) { xs[1] = xs[0]; }` on the same slot, run | Accepted; runs to exit 0 (the distinct-literal pair is not a self-assign) — green pin | spec/03 E10020 rule · AR-4 |
| ST-7 | `if (xs != null) { xs[i] = xs[j]; }` with distinct local indices `i`, `j` | Rejected with exactly one `E10020` — the conservative comparison pinned (AR-4 user ruling) | spec/03 E10020 rule · AR-4 |
| ST-23 | `if (xs != null) { xs[0] = xs[0]; }` on an `i32[]?` slot | Rejected with exactly one `E10020` — the constant-index pair (green pin) | spec/03 E10020 rule · AR-4 |
| ST-24 | `if (xs != null) { xs[0] = xs[i]; xs[i] = xs[0]; }` on the same slot, run | Accepted; runs to exit 0 — the mixed constant/runtime pairs stay accepted (green pin) | spec/03 E10020 rule · AR-4 |
| ST-25 | `xs[i] = xs[i];` on a non-nullable `borrow xs: i32[]` root, run | Accepted; runs to exit 0 — the non-nullable element self-assign is out of the out-slot carve-out (green pin) | spec/03 E10020 rule · AR-9 |
| ST-26 | The `copySelf` body called as `copySelf<u8[]>(borrow v)` (a non-nullable view substitution), run | Accepted; runs clean — a non-nullable view instance stays accepted (green pin) | spec/decisions D9 · AR-7 |
| ST-27 | The `copySelf` body called as `copySelf<u8[]?>(borrow v)` (a nullable view substitution) | Rejected with exactly one `E10020` naming `slot` — nullability takes precedence over the view skip, matching the concrete `borrow u8[]?` spelling (red at baseline: accepted) | spec/03 E10020 rule · AR-7 |

### P2 — Out-slot return (03-02)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-8 | `func cid(borrow v: String?): String? { return v; }` called with a non-null slot; the caller checks `s == null` after the call and reads `len(r)`; run under ASan | Accepted; the caller's slot is null, the returned value holds the payload (exit 3); ASan-clean (no double free, no leak) — red at baseline (double free) | spec/06 §4, spec/16 in-out slot · AR-3 |
| ST-9 | The guarded arm `func cid(borrow v: String?): String? { if (v != null) { return v; } return null; }`, run with a non-null and a null slot | Accepted; the non-null call returns the payload (exit 3) and the null call returns null (exit 9); ASan-clean — red at baseline (the guarded return takes the same direct branch as ST-8; plain and ASan double free) | spec/decisions T8 · AR-3 |
| ST-10 | `return xs[0]` and `return xs[i]` through a `String[]?` slot (projected returns): narrowed (`if (v != null) { … }`) | Rejected with exactly one `E10026` each — green pin of the v1 element-move limit; the unnarrowed spelling reports the read path's `E10010` in addition (also unchanged) | spec/03 element-move limit · AR-8 |
| ST-11 | `func gid(borrow v: String): String { return v; }` (read-only lend, non-nullable) | Rejected `E10034` — green pin | spec/03 OW-16 · AR-3 |
| ST-12 | The generic `gid<T>(borrow v: T): T` at `T = String?` | Rejected with exactly one `E10034` — green pin (the borrow-generic-integrity rule is unchanged) | spec/03 OW-16 · AR-3 |

### P3 — View-store guards (03-03)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-13 | `func f(borrow xs: u8[]?): i32 { xs[0] = 1; return 0; }` called with a null view | Rejected with exactly one `error E10010` naming `xs` (help: check `!= null` before the use) — red at baseline (SEGV) | spec/decisions T8/D9, spec/02 TY-6 · AR-5 |
| ST-14 | The same body with `xs[i] = 1` (run-time index) unnarrowed | Rejected with exactly one `E10010` — red at baseline | spec/decisions T8 · AR-5 |
| ST-15 | `if (xs != null) { xs[0] = 7; return i32(xs[0]); }` on `slice(d, 0, 2)`, run | Accepted; exit 7 (the narrowing stays valid across the write) — green pin | spec/02 TY-6 · AR-5 |
| ST-16 | `xs[0] = 7; return i32(xs[0]);` on a non-nullable `borrow xs: u8[]`, run | Accepted; exit 7 — green pin | spec/16 writable lend rules · AR-5 |
| ST-17 | `return i32(xs[0])` on the possibly-null view (the read form) | Rejected `E10010` — green pin of the read rule | spec/decisions T8 · AR-5 |
| ST-18 | `if (xs != null) { xs[0] = 1; }` on `slice(d, 2, 2)` (empty non-null view), run plain and under ASan | Accepted; the run aborts with the exact first stderr line `error E11211: index out of range` (the `run_fault_spec_test.rs` contract: non-success exit + first line); the ASan run traps identically and prints no `AddressSanitizer` report — red at baseline (write proceeds; ASan stack-buffer-overflow) | spec/14 E11211 · AR-6 |
| ST-19 | The same shape with the run-time index (`xs[i] = 1`) on the empty view, run | Accepted; traps with the same contract (`error E11211: index out of range`) — green pin | spec/14 E11211 · AR-6 |
| ST-20 | `if (xs != null) { xs[0] = 9; return i32(xs[0]); }` on `slice(d, 0, 2)`, run | Accepted; exit 9 (the in-bounds guard passes) — green pin | spec/14 E11211 · AR-6 |
| ST-21 | The read form `return xs[0]` on the empty view, run | Accepted; traps with the same contract (`error E11211: index out of range`) — green pin | spec/14 E11211 · AR-6 |
| ST-22 | Emitting the ST-2 (P1), ST-8 (P2), and ST-15 (P3) accepted programs twice each | Each program's two emissions are byte-identical | spec/17 determinism · AR-11 |
| ST-28 | `xs[0]++;` on an unnarrowed `u8[]?` slot (compile only) | Rejected with exactly one `E10010` naming `xs` — the update form is covered by the operand's read walk (green pin) | spec/decisions T8 · AR-5 |
| ST-29 | `if (xs != null) { xs[0] += 1; }` on `slice(d, 2, 2)` (empty non-null view), run plain and under ASan | Accepted; traps with the ST-18 contract — red at baseline (the compound constant store writes out of bounds with no trap) | spec/14 E11211 · AR-6 |
| ST-30 | `if (xs != null) { xs[0] += 1; xs[0]++; return i32(xs[0]); }` on `slice(d, 0, 2)` with `d: i32[2] = [1, 2]`, run | Accepted; exit 3 (the in-bounds compound guards pass and both updates apply) — green pin | spec/14 E11211 · AR-6 |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/borrow_store_integrity_spec_test.rs` | ST-1 – ST-30 (P1: ST-1 – ST-7 and the extension ST-23 – ST-27; P2: ST-8 – ST-12; P3: ST-13 – ST-22 and the extension ST-28 – ST-30) | P1–P3 |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/borrow_store_integrity_impl_test.rs` | The per-instance self-assign report shape (one report, duty naming); the direct-return emission shape (`__cl_return` block-local, `(*v) = NULL;`, the two-return tail); the unchanged direct emission for a view slot; the `Statement::Guards` trap line spelled before a slice element store (plain, compound, and update forms) with no statement constructed for fixed-array-only targets; the unchanged fixed-array element-store emission; the `E10010` shape at the store site (one report for the update form); byte-identical re-emission | High |

### Integration Tests

The per-phase `make check` (workspace suites) is the integration surface; the emission-integrity
review's base-versus-head sweep is the P2/P3 artifact (AGENTS.md dispatch, AR-13).

### End-to-End Tests

`N/A` — no new harness; `make verify` (all legs) is the final acceptance run (AR-11).

## Test Data

### Fixtures Needed

None beyond inline programs; the suites reuse the repo's existing compile-and-run, strict-build,
ASan-probe, and re-emission helpers (`borrow_generic_integrity_spec_test.rs` patterns).

### Mock Requirements

None — real compiler, real host C compiler, real AddressSanitizer.

## Verification Checklist

- [x] All specification test cases (ST-*) defined with concrete input/output pairs
- [x] Every ST case traces to a requirement, spec doc, or AR entry
- [x] Specification tests written BEFORE implementation
- [x] Specification tests verified to FAIL before implementation (red phase)
- [x] All specification tests pass after implementation (green phase)
- [x] Implementation tests written for edge cases and internals
- [x] All unit / integration / E2E tests pass
- [x] No regressions in existing tests
