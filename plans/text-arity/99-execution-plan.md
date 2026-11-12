# Task: Enforce the one-argument `text` intrinsic

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 8/8 tasks (100%)
> **Reasoning**: low — one arity check and one dead codegen fallback, with an
> existing diagnostic and an existing test harness. This task executes before
> PLAN-32 because both edit the compiler-known `text` branch.

## Objective

Close issue #24: `text` is the compiler-known one-argument conversion
intrinsic (`spec/appendix-stdlib.md` §`text(v)`), but the checker accepts any
argument count and codegen silently ignores extra arguments. `text(1, side())`
prints `1` without running `side()`, and `text()` lowers to `cl_text_i32()`,
which fails the C build.

**Smallest viable design:** require exactly one argument in the existing
compiler-known `text` branch with the frozen E10308 arity diagnostic and the
registry help `pass one argument per parameter`; remove the zero-argument
codegen fallback. No new diagnostic, no new intrinsic, no spec edit.

## Tasks

- [x] T-01 Write `compiler/tests/text_arity_spec_test.rs`: `text()` and
  `text(1, side())` (with `func side(): i32 { println("side"); return 1; }`)
  are rejected with E10308 and write no C; `text(7)` runs and prints `7`; the
  two-argument program's side effect never runs.
- [x] T-02 Run the suite against `bb3956c` and record the red evidence (both
  wrong-arity programs are accepted; `text()` fails the C build).
- [x] T-03 Add the exact-one-argument check to the compiler-known `text`
  branch (`compiler/src/sema/call.rs:276`) with
  `wrong number of arguments for 'text': expected 1, found N` and the frozen
  help `pass one argument per parameter`.
- [x] T-04 Remove the zero-argument fallback from `text_call` and make the
  argument mandatory (`compiler/src/codegen/expr.rs:1561`); update all three
  call sites: the finalizer `$text` sentinel, the explicit `text` call, and the
  template placeholder renderer.
- [x] T-05 Add `compiler/tests/text_arity_impl_test.rs`: E10308 at the call
  span, no C, and the one-argument `cl_text_i32` shape unchanged.
- [x] T-06 Run the independent correctness and emission-integrity reviews on
  the diff and fix every critical/major finding — verify: the verdicts are
  recorded in the task findings below. ✅ (completed: 2026-11-12 05:45)
- [x] T-07 Run the regression suites and `make check` — verify: the #25/#26/
  #27/#28 suites and the workspace tests are green. ✅ (completed: 2026-11-12 05:45)
- [x] T-08 Run the full acceptance gate (`make asan`, `make ubsan`,
  `make bench-shape`, `make bench-base COMMIT=bb3956c`, `make bench`,
  `make bench-compare`, `make verify`); record the result in the roadmap row
  and close #24.

**Verify**: `make check`; `make asan`; `make ubsan`; `make bench-compare`;
`make verify` — all green on the combined final tree (`864dab3`).

## Key decisions

| Decision | Outcome |
|----------|---------|
| Diagnostic | E10308 with the frozen message/help |
| Rejection timing | Sema; no C is written |
| Extra argument effects | Rejected, never silently dropped; the program does not run |
| Codegen fallback | Removed; `text_call` takes a mandatory argument |
| Spec/ABI | No change; the appendix already specifies `text(v)` |
| Benchmark | No codegen shape change for a valid one-argument call; `bench-compare` still runs as evidence |

## Findings

| Review | Verdict | Findings |
|--------|---------|----------|
| Correctness | no critical/major after fix | RV-001 fixed; RV-002 fixed; RV-003 fixed |
| Emission integrity | PASS | no findings against the diff |

**RV-001 (MAJOR) — a module-level value named `text` was hijacked.**
`is_local` covers only lexical bindings, so `let text: func(i32, i32): String
= impl2;` reached the compiler-known branch and the new E10308 check rejected a
valid call. `text_name_is_shadowed` now exempts a module-level `Global`/`Const`
named `text` and an imported real global; an imported function is already
resolved by `free_function`, and the intrinsic import has no table entry. ST-6
(`at6`) pins `text(1, 2)` on a module value.

**RV-002 (MINOR) — the module-initializer case was confounded.** at5 used a
non-duplicable module `String`, which reports E11106 on the base. It now uses
`let n: i32 = i32(len(text()));`, which the base accepts and lowers to a
zero-argument `cl_text_i32()`.

**RV-003 (MINOR) — unreachable predicate arm.** The imported-function lookup
in `text_name_is_shadowed` duplicated `free_function`; only the global lookup
remains, and the doc comment says so.

**Emission audit (PASS).** The valid one-argument shape is byte-identical on
host and hosted-amiga; all three `text_call` sites take exactly one argument;
the `$text` sentinel never reaches C; strict builds and `bench-shape` pass.
Two pre-existing shapes are out of this task's scope: an aliased/qualified
`text` spelling (`import { text as t }`) and an unformattable `text(enum)`/
`text(struct)` argument. The latter belongs to PLAN-32; the former is recorded
for the closeout follow-up.
