# Current State: Closure Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The lambda surface is lifted by `compiler/src/own/lower.rs` (`lift_lambda :399`, `declare_captures
:575`) into `cl_lambda_<site>` functions with `struct ClEnv_<site>` environments; the sema stage
records each site's captures (`ClosureSite` at `compiler/src/sema/expr.rs:238`), validates the
capture list (`check_lambda` entries at `:140-174`), and applies the capture rules E10401–E10405
(`spec/14-diagnostics.md:129-133`). The codegen emits the environment struct and the call
(`compiler/src/codegen/program.rs:284-349`, `codegen/expr.rs:663-736`) and renders environment
fields (`codegen/expr.rs:176-187`). Callable bounds are checked at generic call sites
(`sema/call.rs:91-129`, E10303/E10304).

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/sema/expr.rs` | capture-list validation (`:140-174`), E10405 direct-read check (`:101-117`), E10401 capture-move rule (`:256-276`) | DEF-92 relay + DEF-93 module-name checks; DEF-104 return judgment |
| `compiler/src/sema/mod.rs` | `place_root_name` (`:603-614`, one-segment `Qualified` only), E10402 write rule (`:397-414`), name scopes/floors (`:128-137`, `:369-381`) | DEF-94 dotted root; scope/depth facts for the DEF-92/93 checks |
| `compiler/src/sema/call.rs` | generic call checking (`:91-129`) and the bounded set (`callable_bound_params`) | DEF-110 rejection for an unbounded type parameter |
| `compiler/src/own/lower.rs` | `lower_captures` (`:365-397`), `declare_captures` (`:575-625`) | DEF-109 capture masking; DEF-92/93 emission fall out of the accepted forms |
| `compiler/src/own/instance_check.rs` | per-instance duties (`RequiresStorableSlot` etc.) | DEF-104 capture-return duty audit |
| `spec/14-diagnostics.md` | the diagnostics registry | the authorized `E10406` and `E10307` rows (AR-2, AR-3) |
| `spec/tests/compile-fail/` | compile-fail goldens | `E10406_module_capture.clo`, `E10307_unbounded_call.clo` (AR-12) |
| `compiler/tests/compile_fail_spec_test.rs` | asserted-code list | register both new codes (AR-12) |
| `compiler/tests/closure_integrity_{spec,impl}_test.rs` | new suites | all phases (AR-14) |

### Baseline Probes (verified)

Each probe was compiled with `target/debug/clc --profile host -o out.c prog.clo`; accepted units
were built with `cc -std=c99 -O0 -g -Wall -Wextra -Werror` and, where memory matters,
`-fsanitize=address -no-pie` with leak detection.

| Probe | Shape | Baseline result |
| ----- | ----- | --------------- |
| def92 | `let n = 2; let m = 3; ([n] () => ([m] () => m)())()` | accepted; `cc` fails: `too few arguments to function 'cl_lambda_1'; expected 1, have 0` (the lowered capture list drops `m`, so the call renders without the environment) |
| def92-ok | `([n, m] () => ([m] () => m)())()` | accepted; builds and runs (the relay works when the outer lists it) — the fix's accepted spelling |
| def93 | `const base: i32 = 7; ([base] () => base)()` | accepted; `cc` fails: `too few arguments to function 'cl_lambda_0'` (the module const resolves in sema but not in `lower_captures`) |
| def94 | `let p: P = P { x: 1 }; ([p] () => { p.x = 2; })();` | rejected with `E10024: cannot write through read-only parameter 'p'`; the required report is `E10402` |
| def104 | `func outer<T>(x: T): T { return ([x] () => x)(); }` at `T = Wrap<String>`, called `outer<Wrap>(w)` | accepted; ASan: `attempting double-free` |
| def109 | local `run: func(borrow String): String = run` (same name as the module lender), `([run] () => run(stringOf("ab")))()` | accepted; runs; ASan: `11 byte(s) leaked in 1 allocation(s)` (the may-lend gate resolves the unmasked name to the module lender and skips binding the result) |
| def110 | `func g<T>(x: T): i32 { return x(5); }` instantiated with a struct | accepted; `cc` fails: `called object 'x' is not a function or function pointer` |

## Gaps Identified

### Gap 1: the nested relay is neither rejected nor relayed (DEF-92)

**Current Behavior:** `check_lambda` validates each capture entry with `name_type`
(`sema/expr.rs:141-142`) and no enclosing-scope floor check, so the inner lambda's `[m]` resolves
through all scopes; `lower_captures` then drops the unresolvable capture (`own/lower.rs:372-374`)
and the call renders without the environment. The E10405 direct-read check (`sema/expr.rs:101-117`)
never sees the entry.
**Required Behavior:** the entry is a read of `m` in the enclosing lambda's context; an enclosing
lambda that does not list `m` reports E10405 at the entry (AR-4, AR-9).
**Fix Required:** extend the capture-entry validation with the enclosing-lambda reachability rule
(03-01).

### Gap 2: a module-level name is accepted in a capture list (DEF-93)

**Current Behavior:** the entry resolves to the global's type (`sema/mod.rs:464-469`) and is
accepted; the lowering then drops it and the call is environment-less.
**Required Behavior:** module-level bindings need no capture; the entry reports the new E10406
(AR-2, AR-8).
**Fix Required:** the same validation pass distinguishes module-level bindings (03-01).

### Gap 3: dotted capture targets miss the write and move rules (DEF-94)

**Current Behavior:** `p.x` parses as a multi-segment `Qualified` (`parse/expr.rs:439-450`) and
`place_root_name` returns `None` for it (`sema/mod.rs:610`), so the E10402 rule never fires and
the write reaches the ownership pass, which reports E10024.
**Required Behavior:** the root segment is recognized; the dotted write reports E10402 (AR-6).
**Fix Required:** return the first segment for any `Qualified` (03-01).

### Gap 4: a captured owner returned from the lambda aliases and double-frees (DEF-104)

**Current Behavior:** a bare `T` is duplicable at the generic definition
(`types/table.rs:404-424`), so the capture copies; at an owning instance the environment field and
the source alias one allocation and the return releases it twice (ASan double-free). The instance
audit has no capture-ownership rule (`instance_check.rs:12-16`).
**Required Behavior:** the return of a by-value non-duplicable capture is a move and reports
E10401 — directly for a concrete spelling and per owning instance for a generic one (AR-5, AR-11).
**Fix Required:** judge the return position in the capture-move rule and record the generic case
as a per-instance duty (03-02).

### Gap 5: a called func-typed capture resolves to a module lender (DEF-109)

**Current Behavior:** `declare_captures` sets `ir_name` to the source name
(`own/lower.rs:598-600`), bypassing `unique_ir_name`'s callee masking (`own.rs:878-898`), so the
may-lend gate (`own/lower.rs:1564-1589`) resolves the call to a same-named module lender, skips
binding the owned result, and leaks it. The comment claiming "its IR spelling is never read" is
false for a called func-typed capture.
**Required Behavior:** the call resolves to the capture; the result releases once (AR-7, AR-10).
**Fix Required:** route the capture declaration through the masking (03-02).

### Gap 6: an unbounded type-parameter call is accepted (DEF-110)

**Current Behavior:** `sema/call.rs:118` continues only for callable-bound parameters; a bare `T`
falls through with no diagnostic, and codegen emits `x(5)` on a non-callable value.
**Required Behavior:** callability comes only from a callable bound (G-1/G-6); the call reports
the new E10307 (AR-3).
**Fix Required:** reject the unresolvable call in the generic-call check (03-03).

## Dependencies

### Internal Dependencies

- The per-instance duty machinery (`own/instance_check.rs`) is the established path for a generic
  judgment that depends on the substitution (DEF-104); the storable-slot duty is the reference.
- The capture masking pattern is established for parameters, fields, and lambda parameters
  (`own.rs:836-898`); the capture position joins it.
- The compile-fail driver (`compiler/tests/compile_fail_spec_test.rs`) and `./spec/check.sh` are
  established; the two new codes plug in (AR-12).

### External Dependencies

- None. The workspace stays std-only.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The E10405 relay check over-reports a legal shape (for example an already-listed outer name) | Low | Med | ST-2 pins the accepted spelling; the existing capture suites are the regression net |
| The E10406 check rejects a capture a working program relies on | Low | Med | Every module-name capture currently emits a broken call — no working program regresses |
| The per-instance E10401 duty diverges from the concrete spelling | Low | Med | ST-10/ST-11 pin both spellings against the same code; the duty mirrors the storable-slot audit |
| The capture masking changes a diagnostic spelling | Low | Low | The source name stays for diagnostics (`own/lower.rs:599`); impl pins check the masked IR name |
| An existing accepted pattern stops compiling | Low | Med | The full `make check` per phase; the closure suites listed in 01's acceptance criteria |
