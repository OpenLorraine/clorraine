# Testing Strategy: Partial-Move / Drop Coherence

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Move lowering (`consume_place`) | 100% of the new branches (duplicable, owned, owned-and-nested) |
| Assignment ordering | A moved-field reassignment and a borrow reassignment |
| Memory safety | Every move-out form is ASan clean |
| Non-move behavior | Byte-identical emitted C (existing suites) |

E2E: the run suite already compiles and runs emitted C; ASan is added as a
stricter run oracle (AR-4).

### Test placement (AR-4)

| Suite | Kind | ST cases |
| ----- | ---- | -------- |
| `compiler/tests/stage_spec_test.rs` | emitted-C assertions | ST-8, ST-9 |
| `compiler/tests/codegen_run_spec_test.rs` | compile-and-run + ASan | ST-1..ST-7 |
| `compiler/src/own/tests.rs` | lowered-IR / diagnostic unit | ST-10 |

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Expectations derive from `spec/17-codegen.md` P3 (a move is a copy plus a zero
> of the source; the zeroed source is the drop flag) and the ownership chapters,
> never from implementation behavior.

| # | Input / Scenario | Expected Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | `let t = b.value; b = Box<String> { value: t };` (generic) | Frees once; ASan clean | `spec/17-codegen.md:182-199`, AR-1 |
| ST-2 | `b = Box<String> { value: b.value };` (generic) | Frees once; ASan clean | `spec/17-codegen.md:182-199`, AR-1 |
| ST-3 | `struct Label { name: String }` with the ST-2 form (non-generic) | Frees once; ASan clean | AR-11 |
| ST-4 | `f = Dup(borrow f)` | Emits `open/dup/close/close`; frees once | AR-12 |
| ST-5 | `let s = stringOf("x"); s = s;` | Frees once | AR-13 |
| ST-6 | `p = Pair<String> { first: p.first, second: p.second };` | Moves both fields; frees each once | AR-14 |
| ST-7 | `consume(b.value)` where `consume` is `take s: String` | `b.value` is zeroed after the call; frees once | AR-1 |
| ST-8 | A program with no value-consuming move-out | Emitted C is byte-identical to the baseline | AR-8 |
| ST-9 | The ST-2 form | Emitted C contains no `__cl_save` and zeroes the moved field | AR-3 |
| ST-10 | `consume(b.value)` lowered IR | Contains a `Move`/`MoveOut` for the field | AR-1 |

## Test Categories

### Specification-Derived Tests

| Test File | ST Cases | Component |
| --------- | -------- | --------- |
| `compiler/tests/codegen_run_spec_test.rs` | ST-1..ST-7 | lowering + codegen |
| `compiler/tests/stage_spec_test.rs` | ST-8, ST-9 | emitted C |
| `compiler/src/own/tests.rs` | ST-10 | lowering IR |

### ASan Oracle

A helper compiles the emitted C with `cc -fsanitize=address -g` and runs it,
asserting exit 0 and no `AddressSanitizer` output. It reuses the existing emit
helpers in `codegen_run_spec_test.rs` and is applied to ST-1, ST-2, ST-3, ST-5,
and ST-6. The headless host `cc` inside the SDK container provides ASan
(confirmed).

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/src/own/tests.rs` | The `take`-field projection is lowered as a move | High |
| `compiler/tests/stage_spec_test.rs` | The moved field is zeroed in the emitted C | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `determinism_spec_test` | full pipeline | Emitted C byte-identity across rebuilds |
| `corpus_spec_test` | full pipeline | Non-move corpus programs unchanged |

## Test Data

### Fixtures Needed

None beyond inline Clorraine snippets written into the existing Rust test
helpers. A `Box<T>`/`Label` owning struct and a `String` value are enough.

### Mock Requirements

None. The deleter is the real `FreeString`, observed through ASan and the
existing run-output oracle.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a `spec/` rule or the diagnostics registry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] ASan is clean for every move-out form
- [ ] Implementation tests written for internals
- [ ] No regressions in existing tests (non-move emitted C unchanged)
- [ ] No `spec/` file modified
