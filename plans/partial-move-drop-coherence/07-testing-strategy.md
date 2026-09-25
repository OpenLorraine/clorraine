# Testing Strategy: Partial-Move / Drop Coherence

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Ownership mark for a consumed literal field | 100% of the new branches (duplicable, owned place, type-parameter guard, deeper path, borrowed param) |
| Lowering (`consume_place`) | 100% of the new branches (duplicable, owned, type-parameter guard) |
| Memory safety | Every move-out form is ASan clean |
| Non-move behavior | The assignment fast path is preserved |

E2E: the run suite compiles and runs emitted C; ASan is added as a stricter run
oracle in a dedicated implementation suite (AR-4).

### Test placement (AR-4)

| Suite | Kind | ST cases |
| ----- | ---- | -------- |
| `compiler/tests/stage_spec_test.rs` | emitted-C + diagnostics | ST-1, ST-2, ST-6, ST-7, ST-9, ST-10 |
| `compiler/tests/codegen_run_spec_test.rs` | compile-and-run | ST-4, ST-8 |
| `compiler/tests/partial_move_impl_test.rs` (new implementation suite) | ASan compile-and-run | ST-1, ST-2, ST-3, ST-5 |

The ASan oracle lives in its own implementation suite, not the spec suite, so the
immutable spec oracle stays sanitizer-independent. It sets `ASAN_OPTIONS=detect_leaks=0`
(the oracle targets memory errors — double free and use-after-free — not leaks),
and asserts exit 0 with no `AddressSanitizer` output.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Expectations derive from `spec/17-codegen.md` P3 (a move is a copy plus a zero
> of the source; the zeroed source is the drop flag) and the ownership chapters,
> never from implementation behavior.

| # | Input / Scenario | Expected Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | `let t = b.value; b = Box<String> { value: t };` (generic) | The emitted C zeroes `b.value`/`t`; ASan clean | `spec/17-codegen.md:182-199`, AR-1 |
| ST-2 | `b = Box<String> { value: b.value };` (generic) | The emitted C zeroes `b.value` and has no `__cl_save` for the handle; ASan clean | `spec/17-codegen.md:182-199`, AR-1 |
| ST-3 | `struct Label { name: String }` with the ST-2 form (non-generic) | ASan clean | AR-11 |
| ST-4 | `f = Dup(borrow f)` | Emits `open/dup/close/close` (control) | AR-12 |
| ST-5 | `p = Pair<String> { first: p.first, second: p.second };` | Moves both fields; ASan clean | AR-14 |
| ST-6 | `p = Pair<String> { first: p.first, second: p.first };` (an owned field) | Reports `E10020` on the second read | `spec/03-ownership.md:123-134`, AR-14 |
| ST-7 | `Box { value: o.inner.s }` (a deeper path) | Reports `E10026` (v1 top-level limit) | `spec/03-ownership.md:123-134`, AR-15 |
| ST-8 | `b = Box<String> { value: stringOf("y") };` (RHS does not read `b`) | The emitted assignment keeps the existing drop-then-store order (control) | AR-8 |
| ST-9 | The ST-2 form | The emitted C zeroes the moved field and contains no `__cl_save` for it | AR-3 |
| ST-10 | `Box { value: b.value }` where `b` is a `borrow` parameter | Reports `E10034` | `spec/16-borrow-rules.md`, AR-15 |

## Test Categories

### Specification-Derived Tests

| Test File | ST Cases | Component |
| --------- | -------- | --------- |
| `compiler/tests/stage_spec_test.rs` | ST-1, ST-2, ST-6, ST-7, ST-9, ST-10 | emitted C + diagnostics |
| `compiler/tests/codegen_run_spec_test.rs` | ST-4, ST-8 | lowering + codegen |
| `compiler/tests/partial_move_impl_test.rs` | ST-1, ST-2, ST-3, ST-5 | ASan runtime |

### ASan Oracle

A helper in `compiler/tests/partial_move_impl_test.rs` compiles the emitted C with
`cc -fsanitize=address -g`, runs it with `ASAN_OPTIONS=detect_leaks=0`, and asserts
exit 0 and no `AddressSanitizer` output. It covers ST-1, ST-2, ST-3, and ST-5. The
host `cc` inside the SDK container provides ASan (confirmed).

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/src/own/tests.rs` | A duplicable literal field is not moved; a type-parameter field is left untouched | High |
| `compiler/tests/stage_spec_test.rs` | The moved field is zeroed and no stale save appears | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `determinism_spec_test` | full pipeline | Emitted C byte-identity across rebuilds |
| `corpus_spec_test` | full pipeline | Non-move corpus programs unchanged |

## Test Data

### Fixtures Needed

None beyond inline Clorraine snippets written into the existing Rust test
helpers. A `Box<T>`/`Label` owning struct, a `Pair<T>` owning struct, and a
`String` value are enough.

### Mock Requirements

None. The deleter is the real `FreeString`, observed through ASan.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a `spec/` rule or the diagnostics registry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] ASan is clean for every move-out form
- [ ] Implementation tests written for internals
- [ ] No regressions in existing tests (assignment fast path preserved)
- [ ] No `spec/` file modified
