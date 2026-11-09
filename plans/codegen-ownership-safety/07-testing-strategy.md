# Testing Strategy: Codegen Ownership Safety

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Element-loop lowering and alias emission | every exit path and element shape in ST-1 – ST-7, observed through the duplicable `n` field |
| Binding identity and C name mapping | every MOD-16 form in ST-8 – ST-16 |
| Supporting modules | covered by the existing workspace suite |

- Test names state behavior: `should [expected behavior] when [condition]`.
- Both fixes are end-to-end observable: the tests compile a Clorraine
  program, build the emitted C with `-Wall -Wextra -Werror`, and run it,
  asserting the exit code. A miscompile fails either the C build or the run.
- Element cases must not read owned elements (`vlen(ps[0].name)`): that read
  path has a separate defect (`DEF-16`); the write-through is proven by scalar
  observations plus structural and ASan evidence.
- Host AddressSanitizer probes remain manual acceptance evidence (AR #5); the
  container has no ASan runtime, so the automated suites assert behavior.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These test cases are derived EXCLUSIVELY from the specification
> (`spec/16-borrow-rules.md` section 2, `spec/03-ownership.md` O3,
> `spec/08-arrays-strings.md` section 5 LOOP-1/LOOP-2,
> `spec/10-modules.md` section 5 MOD-15/MOD-16) and the Ambiguity Register.
> They define expected behavior BEFORE any implementation exists.
>
> **IMMUTABLE ORACLE RULE:** Do NOT modify these expectations to match the
> implementation. If the implementation does not match a spec test case, the
> implementation is wrong — not the test.

### Borrow Element Alias (DEF-13)

Every case uses `struct P { name: String; n: i32; }` and observes results
through the duplicable `n` field (`ps[0].n + ps[1].n` survives the fix); before
the fix each case fails for the defect reason — abort, wrong value, or C build
failure. The helper `vlen(s: str): i32` used by the shadowing cases returns
`i32(len(s))`.

> Owned element reads (`vlen(arr[0])`, `vlen(ps[0].name)`) currently hit a
> separate defect in argument materialization (filed as `DEF-16`) and must not
> appear in these cases. The String replacement itself is verified
> structurally and by the host ASan probes (see End-to-End Tests).
>
> ST-6 uses two sequential loops: nested element loops currently hang on a
> separate step-selection defect (filed as `DEF-17`) and are out of scope.

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | `ps: P[2]` with `n = 1, 2`; `fill(borrow ps)` loops `for (borrow p in ps) { p.name = stringOf("zz"); p.n = 4; }`; exit `ps[0].n + ps[1].n` | Exit 8; both elements replaced; no double free | spec 16 §2, spec 03 O3, AR #2/#6/#7 |
| ST-2 | Same array; loop writes `name` and `n = 4`, then `return`s from the loop body | Exit 6 (4 + 2: element 0 replaced, element 1 untouched); today it returns 3 because the write never reaches the element | spec 16 §2, AR #7 |
| ST-3 | Same array; loop writes on each iteration and `break`s after the second write | Exit 8; both writes survive the break | spec 16 §2, AR #7 |
| ST-4 | Same array; first iteration `continue`s without an assignment, second writes `n = 4` | Exit 5 (1 + 4) | spec 16 §2, LOOP-2 |
| ST-5 | Same array; loop replaces `name` twice in one iteration (`stringOf("zz")` then `stringOf("qqqq")`) and sets `n = 8` | Exit 16; two O3 replacements on the element with no use-after-free | spec 03 O3, AR #6 |
| ST-6 | Two arrays `xs`, `ys: P[2]`; two sequential borrow loops (first writes `n = 3`, second writes `n = 4`); exit all four `n` values | Exit 14 (3 + 3 + 4 + 4); both aliases stay correct | spec 16 §2, AR #7 |
| ST-7 | `struct Bag { items: P[2]; }`; `fill(borrow bag)` loops `for (borrow p in bag.items)` setting `p.n = 5`; caller reads `bag.items[0/1].n` | Exit 10 (5 + 5); the write reaches the field's array | spec 16 §2, LOOP-2 |

### Local Identity (DEF-14)

RED signals for the shadowing cases: ST-8 – ST-13 and ST-14 fail the C build
with `-Wall -Wextra -Werror` (unused parameter or type confusion) and ST-9
segfaults; ST-15 fails the C build because the local overwrites the lambda
parameter in `local_names`.

Every case below builds with `-Wall -Wextra -Werror` after the fix; before the
fix each fails the C build, segfaults, or returns the wrong value.

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-8 | MOD-16 example: `pick(LIMIT)` reads the parameter into `doubled`, then `let LIMIT = doubled`; `pick(3)` | Exit 6; the parameter is read before the local is declared | spec 10 §5 MOD-16 |
| ST-9 | Outer `x = "aaaa"`; an `if` block declares a shadowing `x = "b"` and checks its own length; after the block the outer length is returned | Exit 4; the outer value survives the inner block | spec 10 §5 MOD-15/MOD-16 |
| ST-10 | Sibling blocks declare `x: i32` and `x: String`; each block adds its own value; exit the sum | Exit 8 (7 + 1); no cross-block clobber or type confusion | spec 10 §5 MOD-15 |
| ST-11 | `f(n)`: a two-iteration loop (`i` runs 0 then 1) whose body declares `let n = n + i` and accumulates; after the loop returns `acc + n` | Exit 16 for `f(5)`; the parameter stays intact across iterations | spec 10 §5 MOD-16 |
| ST-12 | Outer `x: i32 = 5`; `defer { let x: String = stringOf("inner"); ... }`; the function records the outer `x` before exit | Exit 5; the defer-local's type and value do not clobber the outer binding | spec 10 §5 MOD-16 |
| ST-13 | `f(n)`: `const n: i32 = n + 1; return n;` with `f(3)` | Exit 4; a `const` local shadows the parameter | spec 10 §5 MOD-16 |
| ST-14 | Generic `pick<T>(value: T, fallback: T)` declares `let value: T = fallback` and returns it; `pick(1, 3)` | Exit 3; instantiation keeps distinct identities | spec 10 §5 MOD-16 |
| ST-15 | Callable-bound lambda body: `apply<T, F: func(T): T>(x, f)` called as `apply(i32(3), [base] (x: i32) => { let x: i32 = x + base; return x; })` with `base = 2` | Exit 5; the lifted body's local is distinct from its parameter | spec 10 §5 MOD-16, AR #9 (extended) |
| ST-16 | A local consumes the outer binding of the same name in its own initializer: `let x: String = stringOf("aaaa"); let x: String = x; return vlen(x);` | Exit 4; the outer value moves into the shadowing local once and the outer binding is emptied | spec 10 §5 MOD-15/MOD-16, spec 03 O3 |

> **⚠️ AUTHORING RULE:** Derive expectations from the specification. Do NOT
> imagine or infer what the implementation will produce. If an expected output
> cannot be determined from the spec, that is an ambiguity — add it to the
> Ambiguity Register and resolve it before defining the test case.

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/codegen_ownership_spec_test.rs` | ST-1 – ST-16 | Element alias and local identity |

The suite follows the existing run-spec pattern: compile the source, build the
emitted C with the strict warning set, run the binary, and assert the exit
code; a mismatch reports the run's stderr.

### Implementation Tests (edge cases, internals)

> Written AFTER implementation.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/codegen_ownership_impl_test.rs` | Emitted-C structure for a borrow element loop: no write-back statement, no spurious local for the alias; replacement read-back within the same iteration; a slice iterable (`import { slice } from std.mem`); a sanitized-name collision (`x#1` vs a source `x_1`) and a keyword-named shadow; determinism (compile twice, identical emitted C) | High |
| `compiler/tests/codegen_ownership_impl_test.rs` | A build-only freestanding compile of a borrow-element program when the SDK tools are present (pattern: `clorraine_driver_spec_test.rs:249`), plus a run timeout so a hang fails instead of freezing the suite | Medium |
| `compiler/src/own.rs` `#[cfg(test)] mod tests` | `declare` produces unique spellings under nested and sibling shadowing, and reproduces them deterministically | High |
| `compiler/tests/codegen_ownership_impl_test.rs` | A lambda capture binding does not declare a spurious local (regression for the base-override filter) | Medium |

### Integration Tests

Covered by the existing workspace suite: `make check` runs the stage, compile-fail,
run, corpus, and driver suites. `make verify` adds the hosted-amiga legs.

### End-to-End Tests

Manual host evidence (the container has no ASan runtime; AR #5). Probes live
under gitignored `build/defs/` and are recorded on the task line.

Container emit:
`make -C docker exec CMD="target/debug/clc --profile host -o build/defs/p.c build/defs/p.clo"`

Host build and run:
`clang -std=c99 -O0 -g -fsanitize=address -o /tmp/p.asan build/defs/p.c && ASAN_OPTIONS=detect_leaks=0 /tmp/p.asan; echo $?`

| Scenario | Probe shape | Expected Result |
| -------- | ----------- | --------------- |
| DEF-13 plain loop | `String[2]` borrow loop with no element reads, main returns 0 | Clean exit 0, no double free |
| DEF-13 return path | `fill(borrow xs: String[2])` writes then returns; main returns 0 | Clean exit 0, no double free |
| DEF-13 write-through | Struct cases ST-3 and ST-5 as probes | Exit 8 and 16, no sanitizer report |
| DEF-14 MOD-16 / nested shadow / defer-local | ST-8, ST-9, ST-12 programs | Exit 6, 4, 5; no sanitizer report |

## Test Data

### Fixtures Needed

None beyond the inline Clorraine sources in the test cases.

### Mock Requirements

None; the tests use the real compiler and the real C toolchain.

## Verification Checklist

- [x] All specification test cases (ST-*) defined with concrete input/output pairs
- [x] Every ST case traces to a specification section or AR entry
- [x] Specification tests written BEFORE implementation
- [x] Specification tests verified to FAIL before implementation (red phase)
- [x] All specification tests pass after implementation (green phase)
- [x] Implementation tests written for edge cases and internals
- [x] All unit / integration / E2E tests pass
- [x] No regressions in existing tests
- [x] Off-host build-only compile of a borrow-element program (freestanding leg)
- [x] `make check` green after every GREEN task (RED tasks fail for the defect reason); `make verify` green before completion
