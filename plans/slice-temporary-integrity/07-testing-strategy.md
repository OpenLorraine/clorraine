# Testing Strategy: Slice and Temporary Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Every requirement R1 – R8 | At least one specification case, red on the pre-fix tree (or a documented justified pass) |
| Changed lowering/emission shapes | At least one implementation test pinning the emitted C |
| Memory-safety shapes | Host AddressSanitizer probes, clean (exit 0 or the pinned code, no sanitizer output) |
| Regression surface | Existing suites untouched and green; corpus emission byte-identical except traced diffs |

- Test names state behavior: `should [expected behavior] when [condition]`.
- Integration coverage is the three-profile build inside the existing suites plus `make verify`
  at plan completion.
- The repo has no separate E2E harness for these shapes; `make verify`'s hosted-amiga and FS-UAE
  legs are the end-to-end acceptance.

### Host ASan red/green command (leak-only cases)

ST-8 – ST-11 fail pre-fix only by leaks, and `assert_no_asan` is vacuous without the sanitizer
(the harness gate is unused in the container). Their red/green evidence comes from the host leg:

```
./target/debug/clc --profile host -o <scratch>/prog.c <program>.clo
cc -std=c99 -O0 -g -fsanitize=address -o <scratch>/prog <scratch>/prog.c
ASAN_OPTIONS=detect_leaks=1:symbolize=0 <scratch>/prog
```

or `RDC_ASAN=1 cargo test --test slice_temporary_integrity_spec_test` where the harness gate
applies. A pure leak exits 0 without the sanitizer, so `make check` alone is not red evidence for
those cases (preflight PF-007).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These test cases derive from the frozen specification, the filed defect reproductions, and the
> approved Ambiguity Register entries. They define expected behavior BEFORE any implementation
> exists. The immutable-oracle rule applies: if the implementation does not match a case, the
> implementation is wrong, not the test.

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | ---------------- | -------------------------- | ------ |
| ST-1 | `sink(slice<i32[3]>(grid, 0, 2))` passes a fixed `i32[2][3]` grid (`[1,2,3],[4,5,6]`) to `sink(xs: i32[][3])`; `sink` sums `xs[i][1]` over `len(xs)` | Exit code 7 (2 + 5); strict host build clean; host-ASan clean. **Justified pass**: green pre-fix (behavior pin, not red evidence; preflight PF-012) | R1 / spec/08-arrays-strings.md:28 / AR-3 |
| ST-2 | In a generic helper `storeRows<T>(make: func(): T[2][2], i: usize): T[2]`, instantiated `String`: `let row: T[2] = slice<T[2]>(make(), 0, 2)[i];` over a two-row grid, returning the row | The row holds the selected row's values; the view source releases once; exit code 2 (the row length); host-ASan clean (red pre-fix: double free) | R1 / DEF-70 filing (SA-003/PE-001) / AR-3 |
| ST-3 | A concrete row move through a view over a named local owner: `let row: String[2] = slice<String[2]>(grid, 0, 2)[i];` | `error E10026` (the v1 partial-move limit; unchanged for concrete instantiations) | R1 / existing element rule / AR-3 |
| ST-4 | `return slice<i32[2]>(xs, 0, 2)[1][0];` with `xs: i32[2][2] = [[1,2],[3,4]]` | Exit code 3; strict host build clean (red pre-fix: the strict build fails at the mis-typed nested access; the exact diagnostic depends on the element spelling) | R1 / DEF-70 filing (SA-003/PE-001; nested-access symptom SA-001/SA-005) / AR-3 |
| ST-5 | `sink(slice<i32>(grid, 0, 2))` with `grid: i32[2][3]` | `error E10200` for the explicit type argument that disagrees with the source's row (pre-fix: accepted); the diagnostic message matches the implemented direct report | R1 / AR-9, preflight PF-002 |
| ST-6 | In a generic helper instantiated with `String` and with `i32`: `Box<T> { v: make()[pick()] }` where `make(): T[2]` returns two values and `pick()` returns 1 while recording that it ran | `pick()` observed exactly once; the field binds slot 1; for the owning instantiation the first element releases once; exit code 0; host-ASan clean (red pre-fix: the generic form never runs `pick()`) | R2 / DEF-72 filing / AR-4 |
| ST-7 | In a generic helper, for a duplicable (`i32`) and an owning (`String`) instantiation: `Box<T> { w: f(), v: make()[i] }` where `f()` records 1 then returns 1 and `make()` records 2 (`i = 0`) | Call trace is 12 in both instantiations; exit code 12; host-ASan clean (red pre-fix: trace 21 for the generic form) | R3 / spec/04-expressions.md §3 / AR-4, preflight PF-009 |
| ST-8 | `Box { v: mk().a }` where `mk()` returns a struct with owning `a` and `b` | `.a` binds into the box; `.b` releases at statement end; exit code 0 (the case program checks the bound value); host-ASan clean (red pre-fix: 12-byte leak — ASan-red) | R4 / DEF-74 filing / AR-4 |
| ST-9 | `sink(mkfrom(mka()).a)` where `mkfrom` takes a `String` and returns a struct with owning `a` and `b` | The take argument's place empties; `.a` reaches `sink`; `.b` releases once; exit code 2 (the sink result); host-ASan clean (red pre-fix: 12-byte leak — ASan-red) | R5 / DEF-71 filing / AR-5 |
| ST-10 | `flag && sink((c ? mk1() : mk2()).a)` exercised with `(flag, c)` = (1,1), (1,0), (0,1), (0,0) | Every path host-ASan clean (red pre-fix: 10-byte leak — ASan-red); each chosen call's sibling field releases once; the skipped path runs no extra call | R5 / DEF-71 filing / AR-5 |
| ST-11 | `return i32(spawn().count);` and the regression `return i32(len(spawn()));` | Exit code 5 (the element count); the member form host-ASan clean (red pre-fix: 36 bytes in 2 allocations — ASan-red); the `len` form clean both ways | R6 / DEF-89 filing / AR-5 |
| ST-12 | `out: Pair[4]` where `Pair` has an owning field: `out[target()] = mkvArray()[rhsIndex()];` and `out[target()] = Pair { ... };` | Strict host build accepts both units (red pre-fix: pointer-arrow error); the selected element/literal stores; the replaced element releases once; host-ASan clean | R7 / DEF-75 filing / AR-8, AR-10 |
| ST-13 | `let r: Pair = mkfrom(mka());` (whole-result adoption), its loop form `while (...) { let r: Pair = mkfrom(mka()); ... }`, and `len(mkfrom(mka()).a)` | All host-ASan clean (red pre-fix: the loop form double-frees; the projection leaks); the borrowed projection releases once; exit code reflects the adoption values | R5 / preflight PF-003 / AR-5 |

> **⚠️ AUTHORING RULE:** Expectations are derived from the specification and the filed
> reproductions above. Do not infer them from the implementation. If an expectation cannot be
> determined, stop and add the ambiguity to `00-ambiguity-register.md`.

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation. Filed as `slice_temporary_integrity_spec_test.rs`.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/slice_temporary_integrity_spec_test.rs` | ST-1 – ST-13 | All four components |

The suite reuses the established harness pattern: compile with `clc`, build the emitted host C
under the strict flags, run bounded under host AddressSanitizer with leak detection (the
`run_memory`/`assert_no_asan` pattern of `compiler/tests/def_backlog_closeout_spec_test.rs`), the
host ASan command above for the leak-only cases, and assert diagnostics for the compile-fail rows.

### Implementation Tests (edge cases, internals)

> Written AFTER implementation. Filed as `slice_temporary_integrity_impl_test.rs`.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/slice_temporary_integrity_impl_test.rs` | View typedef key and pointer declarator; `zero_view_helper` key and a nullable row view plus an empty `return;` from a view-returning function; builder and index spellings; deferred consume fill/move-out shapes and the consumer-side emptying at the indexed store; store-form selection for deferred consumes and deferred base fills; root-bind and receiver-bind shapes (read-position placement); loop-condition pin migration; element-store drop spelling with byte-identical existing spellings; migrated flat-name pins; byte-identical re-emission | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Existing suites | All | `cargo test --workspace` in the container (`make check`) |
| Corpus | DEF-70/75 emission | No corpus program uses multi-extent slices or the changed store shape, so corpus emission stays byte-identical; multi-extent users in the test suites are retyped, and every diff traces to its requirement (preflight PF-010) |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Full acceptance | `make verify` at plan completion | All legs pass, including the hosted-amiga corpus and FS-UAE legs |

## Test Data

### Fixtures Needed

- Programs are written in-suite (the repo's existing pattern); no external fixture files.
- Host AddressSanitizer is the memory oracle via the command above; the container `RDC_ASAN` path
  stays unused per the documented environment limitation.

### Mock Requirements

None. The compiler, the host C compiler, and the sanitizer are real.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry; justified passes are documented
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (or recorded as justified passes)
- [ ] Red-phase evidence recorded with the host-ASan command for the leak-only cases
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] The migrated flat-name pins pass with the row-typed key
