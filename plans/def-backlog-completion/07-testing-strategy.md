# Testing Strategy: Def Backlog Completion

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The project's oracle is the frozen specification under `spec/` plus the per-defect reproduction
evidence in [02-current-state.md](02-current-state.md). The suites follow the repository convention:

| Suite | File | Role |
| ----- | ---- | ---- |
| Specification | `compiler/tests/def_backlog_completion_spec_test.rs` | Behavior derived from the spec and the requirements; an immutable oracle |
| Implementation | `compiler/tests/def_backlog_completion_impl_test.rs` | Internals: emitted C shapes, report counts, determinism, pin updates |

Percentage coverage targets are not meaningful for a compiler oracle; the coverage obligation per
phase is every listed ST case plus the emitted-shape pins (AR-10, AR-12).

**Memory-case procedure (PF-010).** A plain host run cannot observe a pure leak and cannot reliably
observe a container UAF, so the memory cases are verified twice:

- **Red phase**: the acceptance cases fail to compile with the expected diagnostic (ST-1/2/4/21–23,
  ST-26); the double-free cases abort on glibc where the shape reaches `free` (ST-7/8); and a
  pre-fix **host** AddressSanitizer probe (emitted C built with `-fsanitize=address`, run with
  `ASAN_OPTIONS=detect_leaks=1:symbolize=0`) records the use-after-free/leak for ST-9/10/12/13/15
  and ST-20/25.
- **Green phase**: the suites pass plainly, and the same memory cases rerun with the `RDC_ASAN`
  opt-in (`RDC_ASAN=1`, matching the previous cycle's `run_memory` helper) or through the close-out
  probes; all runs must report neither a double free nor a leak.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These cases derive from the requirements ([01-requirements.md](01-requirements.md)), the fragment
> specs (`03-XX`), the frozen specification under `spec/`, and the resolved findings. They define
> expected behavior BEFORE any implementation exists. Do not modify them to match the
> implementation. In-code traceability comments quote the behavior in plain language, never an
> `ST-`/`AR-`/`PF-` id or a plan path.

### Phase 1 — Generic lend and move-out integrity (DEF-56, DEF-59, DEF-60, DEF-61)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | ---------------- | -------------------------- | ------ |
| ST-1 | `apply<T>(x: T): T { return x; }` instantiated `String` and its result bound to an owned local | Rejected with `E10034`, "cannot release or store borrowed parameter 'x'", at the return of `x` | R1, AR-2; spec/06-functions.md:101, spec/03-ownership.md:292, spec/14-diagnostics.md:62 |
| ST-2 | `outer<T>(x: T): T { return apply<T>(x); }` instantiated `String` | Rejected with exactly one `E10034` (the site in `apply`); the lend passed to a lend parameter is not itself a move | R1, AR-2 |
| ST-3 | `apply<T>(x: T): T { return x; }` instantiated `i32` | Compiles and runs; the result is the argument (the lend is copied) | R1, AR-2; spec/06-functions.md:83-87 |
| ST-4 | `f<T>(take h: Holder<T>, v: T) { h.items.push(v); }` instantiated `String` | Rejected with `E10034` at the take argument `v`, matching the concrete `Holder`/`String` spelling | R2, AR-2; spec/14-diagnostics.md:62 |
| ST-5 | Concrete `f(take h: Holder, v: String) { h.items.push(v); }` | Still rejected with `E10034` (no acceptance regression) | R8; spec/tests/compile-fail/E10034_borrow_released.clo |
| ST-6 | `g<T>(take v: T): T { return v; }` instantiated `String` | Accepted and runs; moving a `take` parameter is a legal move | R8; spec/06-functions.md:39-44 |
| ST-7 | `consume<T>(mk()[i])` with `i = 1`, owning `T`, no guard | Runs; the consumed value is the element at `i`; the caller's temporary empties slot `i`; no double free (sanitizer run clean) | R3, AR-1; 03-02 §Behavior Rules |
| ST-8 | `flag && consume<T>(mk()[i])` with `flag = true`, `i = 1` | Runs and is correct; the short-circuit path is preserved; no double free. With `flag = false` the call does not run and nothing leaks (exit code pinned) | R3, AR-1; 03-02 §Behavior Rules |
| ST-9 | `let s: T = mk()[i]` with `i = 1`, owning `T` | Runs; `s` holds the element at `i` and no read touches freed memory (sanitizer run clean) | R4, AR-5; 03-02 §Behavior Rules |
| ST-10 | `first<T>(mk: func(): T[2]): T { return mk()[i]; }` with `i = 1` | Runs; the returned value is the element at `i`; no use-after-free | R4, AR-5; 03-02 §Behavior Rules |
| ST-11 | Constant-index element **take** (`consume<T>(mk()[0])`) | Behavior and emitted shape unchanged: the move-out empties slot `0`; run is clean | R8, AR-1; 03-02 §Behavior Rules |
| ST-20 | Constant-index element **store and return** (`let s: T = mk()[0]`; `return mk()[0]`), owning `T` | Emitted shape gains the copy-and-zero move (`__cl_tmp0[0] = NULL` before the release); the run changes from a use-after-free to clean | R4, AR-5, PF-004; 03-02 §Behavior Rules |
| ST-21 | A lend stored into a binding or aggregate in a generic body (`let y: T = x;`, `Box<T> { v: x }`) instantiated owning | Rejected with `E10034`; the same shape instantiated `i32` is accepted and runs | R1, AR-2, PF-012; 03-01 §Behavior Rules |
| ST-22 | A concrete-typed lend in a maybe-owning position (`f<T>(v: String): T { return v; }`) instantiated `String` | Rejected with `E10034` (the audit does not skip concrete declared types) | R1, PF-021; 03-01 §Behavior Rules |
| ST-23 | A generic lend element take (`at<T>(p: T[2]) { sink<T>(p[0]); }`) instantiated `String` | Rejected with `E10034`, matching the concrete `sink(p[0])` spelling | R1, PF-026; 03-01 §Behavior Rules |
| ST-24 | A concrete `borrow File?`/`borrow String?` out-param slot reassigned under an owning instance, starting `null` | Accepted and runs; the out-param slot stays exempt from `E10034` (a generic `T?` slot cannot be declared: `E10011`) | R8, PF-027; spec/06-functions.md:141-145 |
| ST-25 | Multi-level runtime indices (`let s: T = mk()[i][j]`; `consume<T>(mk()[i][j])`) with non-zero `i`, `j` | Runs; every level is materialized once and the exact slot is emptied; no double free or use-after-free | R3/R4, PF-023; 03-02 §Behavior Rules |
| ST-26 | A moved lend that is read again in a generic body, instantiated owning (`reuse<T>(x: T) { let y: T = x; sink(borrow y); sink(borrow x); }`) | Reports exactly one `E10034` and exactly one `E10020` (the concrete spelling reports one `E10034`) | R1, PF-024; 03-01 §Behavior Rules |

### Phase 2 — Do-while condition release (DEF-62)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | ---------------- | -------------------------- | ------ |
| ST-12 | `do { i = i + 1; if (i == 2) { return i; } } while (grow()[0].count > 0);` | Runs and returns `2`; the materialized condition temporary is released on the return; no leak (sanitizer run clean) | R5, AR-4; 03-03 §Behavior Rules |
| ST-13 | A do-while condition materializing two owned calls, with an early return at `i == 3` | Runs and returns `3`; both condition temporaries are released on the return; no leak | R5, AR-4, PF-010; 03-03 §Behavior Rules |
| ST-14 | Do-while without an early return, and do-while exited by `break` | Runs with the pinned result; condition temporaries are released exactly once by the existing paths; no leak and no double free | R8, AR-4; 03-03 §Behavior Rules |
| ST-15 | A do-while with an early return at `i == 2` nested inside another do-while's body, both conditions materializing owned calls | Runs and returns `2`; both loops' condition temporaries are released on the return; no leak | R5, AR-4, PF-010; 03-03 §Behavior Rules |

### Phase 3 — Closure C spelling (DEF-57, DEF-58)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | ---------------- | -------------------------- | ------ |
| ST-16 | `apply<T, F: func(T): T>(x: T, f: F)` called from `outer<T>` with a capturing identity lambda; `outer<i32>(41)` | Compiles to valid C and runs, returning `41` (the identity lambda returns its argument); the callee parameter and the call-site value spell the same callable struct | R6, AR-3, PF-003; spec/06-functions.md:210-213 |
| ST-17 | The same program instantiated for two enclosing types, grounded on `outer<String>` (or a `str` local) | Both enclosing specializations emit distinct closure symbols and both run with correct results | R6, AR-3, PF-017; 03-04 §Behavior Rules |
| ST-18 | A non-generic caller passes a capturing closure to a callable-bound parameter | Accepted and runs; the closure spelling stays `ClClosure_{id}` (symbol key `closure_{id}`) | R8, AR-9; 03-04 §Behavior Rules |
| ST-19 | `consume<F: func(): i32>(f: F) { return f(); }` called as `consume([x] () => x)` | Compiles and runs; the emitted call has no trailing separator | R7, AR-3; 03-04 §Behavior Rules |
| ST-27 | A closure-only callable-bound instance (`consume<F: func(): i32>`) instantiated from two different enclosing generics | The two instances emit distinct C symbols and both run; the closure-only case is the one that would collapse without the instance key | R6, AR-3, PF-018; 03-04 §Behavior Rules |

> **⚠️ AUTHORING RULE:** Expectations come from the specification and the requirements, never from
> the implementation. If an expected output cannot be determined, that is an ambiguity to resolve
> before writing the case.

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/def_backlog_completion_spec_test.rs` | ST-1 – ST-27 | All three phases |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/def_backlog_completion_impl_test.rs` | Duty record and report dedupe; `ElementTemp` emitted `__cl_tmp[i] = NULL`; index materialized once; deferred index form under a guard; return source-move ordering; do-while appended release shape; nested-loop release set; closure key shapes and stamp-at-recording-time; empty-argument call text; byte-identical re-emission | High |

Memory-only cases (ST-9/10/12/13/15/20/25) additionally run through the `RDC_ASAN`-gated
`run_memory` helper in the implementation suite.

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Existing workspace suites | All | `make check` (38 suites) stays green; the updated stale pin and the migrated `apply<T>` programs in `residual_defect_completion_{spec,impl}_test.rs` assert the corrected shapes (PF-002) |
| Cross-toolchain legs | codegen | `make verify` (hosted-amiga, fault, freestanding) at close-out |

### End-to-End Tests

N/A — the plan changes compile-time behavior and emitted C; the end-to-end surface is the existing
corpus and graphics legs already covered by `make check`/`make verify`, and no new harness is built
(AR-7).

## Test Data

### Fixtures Needed

- Inline `.clo` sources in the test files (the repository convention).
- The host C compiler already used by the existing run tests; emitted C is written to a temporary
  directory.

### Mock Requirements

None. Tests use the real compiler pipeline, the real C compiler, and real sanitizer runs; no
external service is mocked.

## Verification Checklist

- [ ] All specification test cases (ST-1 – ST-27) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, 03 doc, spec rule, AR entry, or PF finding
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase; memory cases through
  the documented host ASan probe)
- [ ] All specification tests pass after implementation (green phase; memory cases through
  `RDC_ASAN` or the close-out probes)
- [ ] Implementation tests written for emitted shapes, report counts, and determinism
- [ ] All unit / integration tests pass (`make check`)
- [ ] No regressions in existing tests (including the migrated `apply<T>` programs)
- [ ] Close-out host sanitizer run probes clean (5) and rejection probes correct (2)
