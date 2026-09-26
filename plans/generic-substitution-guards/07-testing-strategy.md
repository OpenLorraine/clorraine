# Testing Strategy: Generic Substitution Guards

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

The project's oracle is behavioral: a specification test drives the built `clc`
binary and asserts the diagnostic or the accepted output. The plan targets:

| Area | Target |
| ---- | ------ |
| Substituted view-field observations | Every reject form and every accept control below |
| Substituted fill observations | Every reject form and every accept control below |
| Scoping and regression | The sema-clean `Pair<str, u8[]>` two-error case, the declaration-site controls ST-8/ST-14, and the module unit tests for the parameter-mention gate |
| Build-and-run controls | The two legal programs in `ST-15`, `ST-16` |

The repository names its suites by role, not by the generic
`[feature].spec.test.[ext]` pattern (see `AGENTS.md`): specification tests live
in `compiler/tests/ownership_drop_spec_test.rs`, implementation tests in
`compiler/tests/ownership_drop_impl_test.rs`, and module unit tests in
`compiler/src/own/instance_check.rs` (AR-15).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These cases derive from `01-requirements.md`, `03-01-instantiation-checks.md`,
> the frozen rules (`spec/02-types.md` TY-10, `spec/08-arrays-strings.md` AL-1),
> and the Ambiguity Register. Expectations come from the specification, never
> from the current implementation. They are an immutable oracle: a failing case
> means the implementation is wrong, not the test.

### View-field instantiations (`E10205`)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-1 | `struct Wrap<T> { v: T; }`; `let w: Wrap<str> = Wrap<str> { v: "hi" };` | Exit 1 with `error E10205: view-typed field 'v' is not allowed` | R1; TY-10; AR-4 |
| ST-2 | Same struct; `let b: Wrap<u8[]> = Wrap<u8[]> { v: bytes(s) };` | Exit 1 with `error E10205: view-typed field 'v' is not allowed` | R1; TY-10; AR-4 |
| ST-3 | Same struct; a nullable view argument, `Wrap<str?>` | Exit 1 with `error E10205: view-typed field 'v' is not allowed` | R1; TY-10; AR-4 |
| ST-4 | `struct Outer<T> { w: Wrap<T>; }`; `let o: Outer<str> = Outer<str> { w: Wrap<str> { v: "hi" } };` | Exit 1 with one `error E10205: view-typed field 'v' is not allowed` (the nested `Wrap<str>` is recorded because the literal names it; IR-only nested instantiations are out of scope, AR-19) | R1, R4; AR-9, AR-19 |
| ST-5 | `func make<T>(v: T): Wrap<T> { return Wrap<T> { v: v }; }`; `let w: Wrap<str> = make<str>("hi");` | Exit 1 with `error E10205: view-typed field 'v' is not allowed` (the concrete return type records the instance) | R1, R4; AR-9 |
| ST-6 | Controls: `Wrap<static str>`, `Wrap<Buffer>`, `Wrap<String>`, and `Wrap<u8[2]>` values | Exit 0, no error, and the emitted C passes `cc -std=c99 -fsyntax-only` | R1, R6; AR-4 |
| ST-7 | `struct Pair<T, U> { a: T; b: U; }`; `let p: Pair<str, u8[]> = Pair<str, u8[]> { a: "hi", b: bytes(stringOf("hi")) };` | Exit 1 with exactly two `E10205` lines (fields `a` and `b`), both from the audit; the program is clean at the declaration site | R1, R3; AR-7, AR-8 |
| ST-8 | Declaration-site control: `struct Mixed<T> { part: u8[]; v: T; }` used as `Mixed<String>` | Exit 1 with exactly one `E10205` line for `part` and no line for `v`; the type checker's error stops the audit (PF-004) | R3; AR-7, AR-17 |

### Fill instantiations (`E10215`)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-9 | `func fillIt<T>(a: T): T[2] { return [; a]; }`; `let cs: String[2] = fillIt<String>(s);` (explicit argument) | Exit 1 with `error E10215: array fill requires a fixed extent` | R2; AL-1; AR-2 |
| ST-10 | The same function called with the argument inferred: `fillIt(s)` where `s: String` | Exit 1 with `error E10215: array fill requires a fixed extent` | R2, R4; AR-9 |
| ST-11 | A prefix+fill body, `func pair<T>(a: T, b: T): T[2] { return [a; b]; }`, called as `pair<String>(x, y)` | Exit 1 with `error E10215: array fill requires a fixed extent` | R2; AR-2 |
| ST-12 | Controls: `fillIt<i32>(7)`, an inferred fill over `u8`, and `let xs: u8[4] = [; 0];` | Exit 0, no error, and the emitted C passes `cc -std=c99 -fsyntax-only` | R2, R6; AR-2 |
| ST-13 | `func rep<T>(w: Wrap<T>): Wrap<T>[2] { return [; w]; }` called as `rep<String>(w)` | Exit 1 with `error E10215: array fill requires a fixed extent` (the substituted `Wrap<String>` is move-only although `Wrap<T>` looked duplicable at the declaration) | R2; AR-2 |
| ST-14 | Declaration-site control: a concrete move-only fill inside a generic function, `func f<T>(x: T): i32 { let xs: String[2] = [; stringOf("x")]; return 0; }`, instantiated as `f<i32>(1)` | Exit 1 with exactly one `E10215` line from the declaration-site guard; the type checker's error stops the audit (PF-004) | R3; AR-7, AR-17 |

### Build-and-run controls

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-15 | A program that constructs a `String[2]` with `fillIt<i32>` and returns an element | The host C compiler builds the emitted C and the program runs to a clean exit — no double-free abort | R6; AR-15 |
| ST-16 | A program whose `Wrap<String>` and `Wrap<static str>` values are dropped normally | The host C compiler builds the emitted C and the program runs to a clean exit — no double-free abort | R6; AR-15 |

> **⚠️ AUTHORING RULE:** derive expectations from the specification documents
> above, never from the implementation. If an expected output cannot be
> determined from the spec, that is an ambiguity — add it to the register and
> resolve it before defining the case.

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation. Filed as
> `compiler/tests/ownership_drop_spec_test.rs`.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/ownership_drop_spec_test.rs` | ST-1..ST-14 | Instantiation audit |

Every rejection case asserts the exact diagnostic line and exit code 1; every
acceptance case asserts exit 0, no `error ` line, and valid C. ST-7 needs a
line-count assertion for its exactly-two requirement (the existing
`assert_reports` checks presence only), and ST-8/ST-14 are declaration-site
controls that pass before and after the implementation (PF-004). The
parameter-mention gate itself is pinned by the module unit tests.

### Implementation Tests (edge cases, internals)

> Written AFTER implementation. Filed as
> `compiler/tests/ownership_drop_impl_test.rs` and in the new module.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/ownership_drop_impl_test.rs` | ST-15, ST-16 build-and-run controls (plain build, no sanitizer; a double free aborts in the C library) | High |
| `compiler/src/own/instance_check.rs` | Unit tests: the parameter-mention gate, a declaration-named nested generic struct instance, a multi-extent fill, a duplicable fill control, and a value reached through a terminator | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | Every suite and `spec/check.sh` | The audit must not regress the existing specification, codegen-run, determinism, corpus, or driver suites |

### End-to-End Tests

N/A — the project's acceptance run (`make verify`) executes the hosted-amiga
corpus, vbcc, FS-UAE, and run-fault legs. The audit is a front-end rule with no
emulator interaction, so the plan reuses the existing acceptance run instead of
building a new harness (AR-15).

## Test Data

### Fixtures Needed

None beyond inline source snippets; the existing suites write a temporary
`.clo` per case and drive the built `clc` binary.

### Mock Requirements

None. The `clc` binary and the host C compiler are real objects.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec rule, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] `make verify` green before completion
