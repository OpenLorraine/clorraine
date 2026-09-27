# Testing Strategy: Non-Lexical Lifetimes

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Liveness analysis (`compiler/src/own/liveness.rs`) | unit-tested: every rule decision and every graph shape |
| Checker integration (hooks, removals) | every ST case, plus the whole existing suite and corpus |
| Emitted C | unchanged; the accepted programs are built and run |

- Test names state behavior: `should … when …`, or the repo's existing
  `a_…_is_…` style.
- E2E coverage is the existing `make verify` corpus and run-fault legs; no new
  harness (AR-4, AR-23).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Expectations are derived from `spec/16-borrow-rules.md` (rules, accepted and
> rejected examples, the v2 phase table) and the Ambiguity Register, never from
> the implementation. The `Source` column lives here; the in-code comments of
> the resulting tests state the behavior in plain language.

All snippets assume these helpers unless stated otherwise:

```clorraine
func observe(v: u8[]): void {}
func observeStr(v: str): void {}
func sink(take s: String): void {}
func reset(borrow s: String): void {}
func viewOf(s: str): str { return s; }
```

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | ---------------- | -------------------------- | ------ |
| ST-1 | `s = stringOf("x"); v = bytes(s); observe(v); sink(s);` | accepted — no diagnostics; the loan's last use is `observe(v)` | `16-borrow-rules.md:256-270` (accepted form), AR-3 |
| ST-2 | `s = stringOf("x"); v: str = s; if (len(v) == 0) { return; } s = stringOf("y");` | accepted — the widened loan ends at `len(v)` | `16-borrow-rules.md:253`, AR-3, AR-11 |
| ST-3 | `s = stringOf("x"); v = viewOf(s); observeStr(v); reset(s);` | accepted — the exclusive lend follows the shared loan's last use | `16-borrow-rules.md:253` |
| ST-4 | `s = stringOf("x"); v = bytes(s); sink(s); observe(v);` | rejected — exactly one `E10022`, message `cannot move 's' while it is borrowed` | `16-borrow-rules.md:250`, `14-diagnostics.md:53` |
| ST-5 | `s = stringOf("x"); v = bytes(s); s = stringOf("y"); observe(v);` | rejected — exactly one `E10031`, message `cannot modify 's' while it is shared-borrowed` | `16-borrow-rules.md:248-250`, `14-diagnostics.md:59` |
| ST-6 | `b = bufferNew(16); v = bufferBytes(b); w = bufferBytes(b); observe(v); observe(w);` | rejected — exactly one `E10032`, message `'b' is already borrowed` | `16-borrow-rules.md:188-196` (BR-3), `14-diagnostics.md:60` |
| ST-7 | `s = stringOf("x"); v = bytes(s); i = 0; while (i < 2) { observe(v); if (i == 0) { s = stringOf("y"); } i = i + 1; }` | rejected — `E10031`: the next iteration's `observe(v)` is reachable through the back edge | AR-3, AR-15 |
| ST-8 | `s = stringOf("x"); v = bytes(s); if (c) { observe(v); } sink(s);` | accepted — after the `if`, no use of `v` is reachable | AR-3 (branch precision) |
| ST-9 | `s = stringOf("x"); v = bytes(s); if (c) { sink(s); } observe(v);` | rejected — `E10022`: the use after the branch is reachable from the move | AR-3 |
| ST-10 | `held: str; { s = stringOf("x"); held = s; observeStr(held); }` | accepted — every use precedes the inner scope's release | AR-3, AR-8 |
| ST-11 | `held: str; { s = stringOf("x"); held = s; } observeStr(held);` | rejected — `E10030`, message `borrow of 's' does not live long enough` | `16-borrow-rules.md:247-252`, `14-diagnostics.md:58` |
| ST-12 | `s = stringOf("x"); a = bytes(s); b = a; observe(a); observe(b); sink(s);` | accepted — the alias shares the loan and both uses precede the move | AR-5 |
| ST-13 | `s = stringOf("x"); a = bytes(s); b = a; sink(s); observe(b);` | rejected — `E10022`: the alias use is reachable from the move | AR-5 |
| ST-14 | `s = stringOf("x"); v: str = s; s = stringOf(v);` | accepted — the right-hand side reads `v` before the store; evaluation order decides | AR-7 |
| ST-15 | `s = stringOf("x"); v: str = s; s = stringOf(v); observeStr(v);` | rejected — `E10031`: the use is ordered after the store | AR-7 |
| ST-16 | `s = stringOf("x"); v: str = s; defer { observeStr(v); }` | accepted — the defer runs on the scope's exit edge before the owner's release | AR-6, AR-8 |
| ST-17 | `s = ...; t = ...; v = bytes(s); if (c) { v = bytes(t); } sink(s); observe(v);` | rejected — exactly one `E10022`: the view still aliases `s` on the other path | AR-5 (join carries both loans) |
| ST-18 | the mirror of ST-17 with the rebind on the `else` branch | rejected — exactly one `E10022` | AR-5 |
| ST-19 | `s = ...; t = ...; a: str = s; if (c) { a = t; } s = stringOf("z"); observeStr(a);` | rejected — exactly one `E10031` | AR-5 |
| ST-20 | `i = 0; while (i < 2) { let s = ...; let v: str = s; observeStr(v); i = i + 1; }` | accepted — the next iteration's use is reached from a fresh creation before the release | AR-8 (release-free reachability) |
| ST-21 | `defer { let s = ...; let v: str = s; observeStr(v); }` | accepted — the defer's own use precedes its owner's release on the same edge | AR-6, AR-8 |

> **⚠️ AUTHORING RULE:** derive each test's expectation from the cited spec
> rule and the register, not from the checker. If a snippet needs a helper
> type (`bool c`, `Buffer b`) the packet says so; the standard-library
> signatures used by ST-2/ST-14/ST-15 are `String`/`stringOf(str)`/`bytes` and
> `len`, verified at execution.

Notes:

* The `E10032`/`E10031`/`E10022` rejection shapes (ST-4 – ST-6) also exist as
  frozen goldens; the spec test pins them inside the new suite so the
  monotonicity and single-report expectations are local.
* ST-16's expectation depends on the existing order in which defers run before
  drops (`spec/03-ownership.md`, `compiler/src/own.rs:347-356`); it is an
  acceptance test for that existing rule under NLL, not a new rule.

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE the implementation. Filed as
> `compiler/tests/non_lexical_lifetimes_spec_test.rs`.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `non_lexical_lifetimes_spec_test.rs` | ST-1 – ST-21 | Liveness model and rules |

The file follows the existing integration harness: compile a snippet with the
built `clc` binary and assert either no error codes (accept cases) or the exact
code vector (reject cases), like `compiler/tests/stage_spec_test.rs` and
`compiler/tests/ownership_drop_spec_test.rs`; the in-module unit tests use the
`compiler/src/own/test_support.rs` helper.

### Implementation Tests (edge cases, internals)

> Written AFTER the implementation.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/src/own/liveness.rs` (unit tests) | region shapes on hand-built events and small CFGs: no use ⇒ empty region; a use on one branch only; a use after the conflict; a loop back edge; alias sharing; release-free reachability (a loop-local owner stays accepted); report-once; a deferred block's events stay ordered on its exit edge | High |
| `compiler/tests/non_lexical_lifetimes_impl_test.rs` | accepted ST-1 and ST-10 programs build and run (elements freed once); a conflict is reported exactly once; the diagnostics of one program are identical across two runs | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | whole workspace | every existing spec, implementation, and unit suite stays green (R6) |
| `make verify` | driver, corpus, cross-toolchains | the portable corpus, vbcc leg, FS-UAE programs, and runtime faults are unchanged (R6) |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Accepted NLL program | emit C, build, run in the host profile | exit status and output as specified by the program; no leak or double free under the standard runtime |
| The full acceptance gate | `make verify` | every leg passes |

## Test Data

### Fixtures Needed

- The rewritten golden `spec/tests/compile-fail/E10031_view_widening_mutation.clo`
  (03-03).
- Small hand-built `ir::Function` graphs for the liveness unit tests.
- No other fixtures: the ST snippets are self-contained.

### Mock Requirements

None. The tests use the real compiler stage and the real C toolchain; no mocks
(AR-4).

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests or the corpus
- [ ] Test coverage meets goals
