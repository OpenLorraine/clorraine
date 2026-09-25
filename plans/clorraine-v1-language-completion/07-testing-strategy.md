# Testing Strategy: Clorraine v1 language completion

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Type/sema/codegen paths (`sema`, `types`, `ir`, `codegen`) | 90% of changed lines exercised |
| Stdlib module (`std.amiga.custom`) | covered by the corpus leg |
| Spec and golden data | `./spec/check.sh` green |

- Test names state behavior.
- E2E is the existing corpus + hosted-amiga/freestanding legs; no new harness is
  built (AR-26).
- The `[feature].spec.test.[ext]` / `[feature].impl.test.[ext]` split is
  realized as `compiler/tests/v1_language_spec_test.rs` and
  `compiler/tests/v1_language_impl_test.rs` (AR-6).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These cases are derived from the frozen specification, the ledger decisions,
> and the Ambiguity Register — never from imagined implementation behavior. If
> the implementation disagrees, the implementation is wrong.

### Generics

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-1 | `Pair<i32> { first: 1, second: 2 }` used twice and `Pair<u8>` once; compile and run | runs; the emitted C declares `ClPair__i32` once and `ClPair__u8` once | G1, G3, P11; AR-17 |
| ST-2 | `Pair<Pair<i32>>` literal; compile and run | runs; nested key resolves and the emitted C compiles | G1, G3; AR-17 |
| ST-3 | a struct declared in module `A` and instantiated as `Pair<i32>` in module `B` | compiles and runs | G1, M2, M10; AR-17 |
| ST-4 | `apply<i32, func(i32): i32>(5, addOne)` | compiles and runs, returning the applied value | G6, G-1 |
| ST-5 | `apply<i32, i32>(5, 3)` | `error E10303: type argument 'i32' does not satisfy 'F: func(...)'` | G-1; AR-15 |
| ST-6 | `func keep<T, F: func(T): T>(x: T, f: F): F { return f; }` | `error E10304: value of type parameter 'F' cannot be stored or returned` | G-4; AR-15 |
| ST-7 | `makeHolder([borrow counter] (n: i32) => { counter = counter + n; })` | `error E10306: 'Holder' cannot be instantiated with an ephemeral type argument` | G-8, L3; AR-16 |
| ST-8 | `nest<T>` instantiates `nest<Wrapper<T>>` recursively | `error E10305: instantiation of 'nest' is recursive beyond the limit` | G-7; AR-18 |
| ST-9 | `export func identity<T>(x: T): T { ... }` | `error E10302: generic 'identity' cannot be exported` | G-5 |
| ST-27 | a generic struct whose instantiation recurses, `struct Nest<T> { next: Nest<Wrapper<T>>; }` | `error E10305: instantiation of 'Nest' is recursive beyond the limit` (the cap covers generic-struct instantiation) | G5, G-7; AR-18 |

### Lambdas

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-10 | `let inc: func(i32): i32 = (x: i32) => x + 1; inc(4)` | compiles and runs, returning `5` | L1, L5, P7 |
| ST-11 | `apply2(3, (x: i32) => x + 1)` with `f: func(i32): i32` | compiles and runs, returning `5` | L1, L5 |
| ST-12 | inline `[base] (x: i32) => x + base` passed to a callable bound, with `F` left to inference | runs; the capture is a copy and `base` is unchanged afterward (the explicit `func(i32): i32` type argument is the ST-18 E10400 case) | L2; AR-10 |
| ST-13 | inline read-only capture of a non-duplicable value, passed to a callable bound | runs; the captured owner is not moved | L2; AR-10 |
| ST-14 | inline `[borrow counter] (n: i32) => { counter = counter + n; }` invoked | runs; `counter` is mutated through the borrow | L2 |
| ST-15 | `([s] () => consume(s))()` where `s: String` and `consume` takes `take` | `error E10401: cannot move captured 's' in a lambda` | L-3; AR-13 |
| ST-16 | `([counter] () => { counter = counter + 1; })()` | `error E10402: cannot write captured 'counter' without '[borrow counter]'` | L-4; AR-13 |
| ST-17 | `([take s] () => s)()` | `error E10403: 'take' captures are not supported` | L-5; AR-22 |
| ST-18 | `apply(1, [base] (x: i32) => x + base)` where `apply` takes `func(i32): i32` | `error E10400: capturing lambda cannot be used as 'func(...)'` | L5; AR-10 |
| ST-19 | `let f = [base] (x: i32) => x + base;` | `error E10404: closures are ephemeral and cannot be stored, returned, or captured` | L3, L-6; AR-11, AR-14 |

### Volatile and MMIO

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-20 | `unsafe { writeVolatile8(p, 0x2A); readVolatile8(p) }` over a real local address | reads back `0x2A` on the host profile | T7; AR-19, AR-27 |
| ST-21 | round-trip of `readVolatile16`/`readVolatile32` and matching writes | each reads back the written value | T7; AR-19 |
| ST-22 | `readVolatile32(addr)` outside `unsafe` | `error E10604: volatile access requires 'unsafe'` | T7, E10604 |
| ST-23 | two volatile reads separated by a volatile write to the same address | the second read observes the write (no folding) | T7; AR-19 |
| ST-24 | `volatile let ticks: u32 = 0;` | the emitted C declares a `static volatile u32` global | T7, IH-3; AR-20 |
| ST-25 | importing `std.amiga.custom` under the `host` profile | `error E11200: 'std.amiga.custom' is not available on profile 'host'` | appendix-stdlib §6; AR-7 |
| ST-26 | a hosted-amiga program that imports `std.amiga.custom` and calls `setColor`/`setDma` | the hosted-amiga leg builds it and it runs to completion under `vamos`/FS-UAE | appendix-amiga §4; AR-27, AR-28 |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/v1_language_spec_test.rs` | ST-1..ST-4, ST-10..ST-14, ST-20..ST-24 | Generics, lambdas, volatile (runnable) |
| `compiler/tests/compile_fail_spec_test.rs` | ST-5..ST-9, ST-27, ST-15..ST-19, ST-22, ST-25 | Generics, lambdas, volatile (goldens) |
| `compiler/tests/corpus_spec_test.rs` + `spec/tests/corpus/volatile-mmio/` | ST-26 | MMIO wrapper (hosted-amiga) |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/v1_language_impl_test.rs` | struct-instance dedupe and `type_key` for nested args; closure lift/env naming; volatile qualifier emission; module registration | High |
| in-module `#[cfg(test)] mod tests` | focused unit tests in `ir/mono.rs`, `own/generics.rs`, `codegen/types.rs`, `sema/expr.rs` | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| compile-fail suite | sema + goldens | every un-excluded code asserts its first line and registry message |
| `make verify` hosted-amiga legs | codegen + runtime + stdlib | the emitted C compiles under GCC/vbcc and the corpus runs under `vamos`/FS-UAE |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Volatile read/write | `make run FILE=… PROFILE=host` on the volatile case | expected stdout; exit 0 |
| MMIO wrapper | `make corpus-amiga` including `spec/tests/corpus/volatile-mmio/` | builds, runs, output matches |

## Test Data

### Fixtures Needed

- `.clo` inputs under `spec/tests/corpus/volatile-mmio/` and inline sources in
  the spec suites.
- A golden `spec/tests/compile-fail/E11200_custom_module.clo` that imports
  `std.amiga.custom` under the `host` profile (ST-25, AR-7).
- The existing goldens, un-skipped (AR-25).

### Mock Requirements

None. The host volatile test uses a real local address; no external is mocked
(AR-27).

## Verification Checklist

- [ ] All ST cases defined with concrete input/output pairs
- [ ] Every ST case traces to a spec/ledger/AR source
- [ ] Specification tests written before implementation and verified red
- [ ] All specification tests pass after implementation (green)
- [ ] Implementation tests cover edge cases and internals
- [ ] `make check` passes; `make verify` passes at each phase end
- [ ] No regressions in the existing suites
- [ ] `./spec/check.sh` passes after the spec edits
