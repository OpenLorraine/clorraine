# Testing Strategy: Store-Emission Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Every covered behavior | one or more ST cases (compile/run/emit assertions) |
| Memory-sensitive shapes (DEF-113 family) | host ASan probes per shape (AR-13) |
| Emission shapes (DEF-115, DEF-116) | strict C build + run + emitted-shape pins |
| Reports (DEF-113, DEF-114) | exact code, message fragment, and count per site |

- Test names state behavior ("reports ... once", "builds and runs", "evaluates the offset once").
- Integration coverage is the suite itself: every accepted case builds the emitted C under the
  strict flags and runs it. E2E: N/A — the compiler's integration oracle is the emitted C plus
  the run (no new harness is built; AR-13).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the requirements ([01](01-requirements.md)), the component specs
> ([03-01](03-01-conditional-store-judgment.md) – [03-04](03-04-module-initializers.md)), the
> frozen specification (BR-5, Ch 02 §6, Ch 17), and the register entries
> ([00-ambiguity-register.md](00-ambiguity-register.md)). Expectations are the ruled behavior,
> never the current implementation.
>
> **IMMUTABLE ORACLE RULE:** if the implementation does not match an ST case, the implementation
> is wrong — not the test. A case that already passes before its phase's implementation is a
> deliberate regression pin (recorded in the red-phase evidence).

### Phase 1 — Conditional-store judgment (DEF-113)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | `func st(borrow slot: String, pick: bool) { let other: String = stringOf("bbbb"); slot = pick ? slot : other; }` | rejected: exactly one `E10034`, message names `slot`; no translation unit | BR-5; AR-4, AR-5 |
| ST-2 | the generic `st<T>(borrow slot: T, pick: bool, take other: T) { slot = pick ? slot : other; }` instantiated at `String` | rejected: exactly one `E10034` naming `slot` (the per-instance duty) | AR-4, AR-5 |
| ST-3 | the same generic instantiated at `i32` | accepted; strict build; runs; exit equals the computed value | AR-5 |
| ST-4 | `func f(borrow slot: String, take a: String, take b: String) { slot = pick ? a : b; }` | rejected: exactly one `E10034` naming `slot` (unchanged behavior) | AR-5 |
| ST-5 | `func f(borrow slot: String, other: String) { slot = other; }` called with an owned argument | rejected: exactly one `E10034` naming `other` (unchanged) | AR-5 |
| ST-6 | the generic conditional with a read-only lend arm (`other: T` unmarked) at `String` | rejected: exactly one `E10034` naming `other` (unchanged) | AR-5 |
| ST-7 | `func g(borrow slot: String?, pick: bool) { let other: String? = stringOf("bbbb"); slot = pick ? slot : other; }` called with `pick = true` | accepted; strict build; runs; ASan-clean | AR-5 |
| ST-8 | the generic conditional shape instantiated at `String?` | accepted (the nullable out-slot carve-out) | AR-5 |
| ST-9 | `func f(borrow slot: String) { slot = slot; }` | rejected: exactly one `E10034` naming `slot` (unchanged) | AR-4 |

### Phase 2 — Immutable-view writes (DEF-114)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-10 | `func f(borrow s: str): i32 { s[0] = 65; return 0; }` | rejected: exactly one `E10042`, message names `s`; no translation unit | Ch 02 §6; AR-2, AR-6 |
| ST-11 | a local `let t: str = "abc"; t[0] = 65;` | rejected: exactly one `E10042` naming `t` | AR-6 |
| ST-12 | `struct S { tag: static str; n: i32; }` with `s.tag[0] = 65;` | rejected: exactly one `E10042`; the message names the field path | AR-6 |
| ST-13 | `s[0] += 1;` on a `borrow s: str` parameter | rejected: exactly one `E10042` | AR-6 |
| ST-14 | `s[0]++;` on a `borrow s: str` parameter | rejected: exactly one `E10042` | AR-6 |
| ST-14b | `i32(s[0]++)` in value position on a `borrow s: str` parameter (the update form's value position; added in the Phase 2 review remediation) | rejected: exactly one `E10042` | AR-6 |
| ST-15 | a module-level `let g: str = "abc";` with `g[0] = 65;` in `main` | rejected: exactly one `E10042` naming `g` | AR-6 |
| ST-16b | a local `let t: str = "abc";` with `t[0] = 65;` in the same function (the same local-write program as ST-11; both rows are pinned by one case, AR-20) | rejected: exactly one `E10042` naming `t` | AR-6 |
| ST-16 | `let t: str = "abc"; t = "xyz"; return i32(len(t));` | accepted; strict build; runs (exit 3) | AR-6 |
| ST-17 | `func f(borrow xs: u8[]): i32 { xs[0] = 65; return 0; }` called with `borrow slice(a, 0, 2)` on a `u8[2]` | accepted; strict build; runs (exit 0) | AR-6 |
| ST-18 | the golden program `E10042_write_immutable_view.clo` compiled directly | rejected: exactly one `E10042` (mirrors `./spec/check.sh`) | AR-2, AR-12 |

### Phase 3 — Typedef order (DEF-115)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-19 | `struct P { x: i32; y: i32; }` with `slice(arr, 0, 2)` and a read plus a write (`ps[0].x = 5`) | accepted; strict build; runs; exit reflects the write | Ch 17; AR-3, AR-8 |
| ST-20 | `enum E: u8 { A = 1, B = 2 }` with a slice of an `E[2]` | accepted; strict build; runs | AR-3, AR-8 |
| ST-21 | `struct S { e: E; n: i32; }` (struct with an enum field) | accepted; strict build; runs | AR-3, AR-8 |
| ST-22 | a closure capturing a slice: `let s0: i32[] = slice(a, 0, 2); return ([s0] () => i32(s0[1]))();` | accepted; strict build; runs | AR-8 |

### Phase 4 — Module initializers (DEF-116)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-23 | `let backing: i32[2] = [1, 2]; let view: i32[] = slice(backing, 0, 2);` with `i32(len(view))` in `main` | accepted; strict build; runs (exit 2) | AR-9, AR-10 |
| ST-24 | `let m: i32[2][2] = [[1, 2], [3, 4]];` | accepted; strict build; runs (`return m[1][0]` → exit 3) | AR-9 |
| ST-25 | `let a: i32[3] = [1, 2; 7];` | accepted; strict build; runs (`return a[2]` → exit 7) | AR-9 |
| ST-26 | `let backing: i32[2] = [1, 2]; let b: i32[2] = backing;` | accepted; strict build; runs (`return b[1]` → exit 2) | AR-9 |
| ST-27 | `let view: i32[] = slice(backing, 1, 3);` on a `i32[3]` with a runtime length and element check | accepted; strict build; runs (len 2; `view[0]` is `backing[1]`) | AR-10 |
| ST-28 | `let v: i32[] = slice(backing, bump(), 3);` with `bump()` counting its calls | accepted; strict build; runs; `bump` ran exactly once and the view starts at the returned offset | AR-11 |
| ST-29 | `let a: i32[2] = pick() ? x : y;` with `x`, `y` module arrays and `pick()` counting its calls | accepted; strict build; runs; `pick` ran exactly once; the element comes from the chosen arm | AR-18 |
| ST-29b | a module conditional whose condition reads out of range (`backing[idx] > 0 ? x : y` with `idx = 5`; added in the Phase 4 review remediation) | accepted by the front end; the run traps `error E11211: index out of range` — the materialized condition keeps its bounds guard | AR-9, AR-18 |
| ST-30 | a module-level `let n: usize = backing.len();` on an `i32[3]` backing | accepted; strict build; runs (exit 3) | AR-10 |

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation per phase, from this document only.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/store_emission_integrity_spec_test.rs` | ST-1 – ST-9 (P1), ST-10 – ST-18 (P2, including ST-14b and ST-16b), ST-19 – ST-22 (P3), ST-23 – ST-30 (P4, including ST-29b) | all four component specs |

### Implementation Tests (edge cases, internals)

> Written AFTER implementation, in `compiler/tests/store_emission_integrity_impl_test.rs`:
> the emitted/reported shape per instance (one report per site), the duty naming, the emitted
> typedef order positions, the fill statement order and the init-local materialization shape, and
> byte-identical re-emission.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/store_emission_integrity_impl_test.rs` | report counts/names, emission-order pins, init shapes, re-emission determinism | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `./spec/check.sh` | registry + goldens | the new `E10042` golden participates in the frozen-suite check |
| `make check` / `make verify` | workspace + cross-toolchain legs | the repo's acceptance gates (AR-13) |

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
- [x] All specification test cases (ST-*) defined with concrete input/output pairs
- [x] Every ST case traces to a requirement, spec doc, or AR entry
- [x] Specification tests written BEFORE implementation
- [x] Specification tests verified to FAIL before implementation (red phase; passing pins recorded with justification)
- [x] All specification tests pass after implementation (green phase)
- [x] Implementation tests written for edge cases and internals
- [x] All unit / integration / E2E tests pass
- [x] No regressions in existing tests
- [x] Host ASan probes clean for the memory cases
