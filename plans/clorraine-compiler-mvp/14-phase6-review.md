# Phase 6 Review: Code Generation, Profiles, Runtime Shims

> **Document**: 14-phase6-review.md
> **Parent**: [Index](00-index.md)
> **Phase baseline tree**: e4f9b86f161b33470ef224fc1dfa7561712a0bcf
> **Phase diff**: the codegen, ABI, profile, IR, and shim changes of tasks 6.1.1-6.3.3
> **Reviewers**: one independent correctness reviewer (RV-400+) and one
> independent memory-safety/semantics auditor (SA-400+), run in parallel under
> strict scope on 2026-09-20.

## Summary

`make check` is green (241 unit; 2 compile-fail, 1 determinism, 6 run-fault,
31 stage) and the host and hosted-amiga shims compile warning-free. The phase is
**not** clean: the suite never compiles the emitted C for FFI declarations,
element loops, nested-scope normal fall-through, `defer`, `take` arguments,
returns of owned places, or reassignment, so several serious defects escape it.
Both reviewers independently found the same core gaps. Merged result:
**9 critical, 8 major, 1 minor.**

The two reviewers' IDs are kept; a merged row lists both where they overlap.

## Critical findings

| Merged | IDs | Finding |
|---|---|---|
| C1 | RV-400, SA-400 | A scope's cleanup does not run on normal fall-through, only on an early exit: a leaked drop and an unused cleanup label that fails `-Werror=unused-label`. `codegen/function.rs:272-284,621-638`. |
| C2 | RV-401 | A `defer` in a scope that owns no drop is skipped on `return` (the return jumps only to a drop-bearing rung). `codegen/function.rs:293-297,587`. |
| C3 | RV-402, SA-406 | `ffi` declarations emit no prototypes and `abi::ffi_type` is never called, so FFI calls are implicit declarations with the wrong ABI; fails `-Werror`. `codegen/program.rs:181-207`, `abi.rs:27`. An ABI-prototype spec case from `03-06` is also missing. |
| C4 | RV-403, SA-411 | The element `for (e in xs)` lowers to a body that never advances or binds the element and never exits. `own/loops.rs:254-301`. |
| C5 | SA-401 | `break`/`continue` from a scope that owns a drop emit a plain `break;`/`continue;`, skipping the drop (leak) and leaving an unused label. `codegen/control.rs:176-191`. |
| C6 | SA-402 | A `take` argument is not zeroed after the call, so the source is freed again at scope exit (double free). `codegen/function.rs:507-518`. |
| C7 | SA-403 | `return <owned place>` copies to the result without zeroing the source, so the callee drops it and the caller frees it again (double free / dangling pointer). `codegen/function.rs:570-594`. |
| C8 | SA-404 | A negative signed index passes the bounds guard (`i >= n`), so `xs[-1]` reads out of bounds; signed slices also fail `-Werror=sign-compare`. `codegen/expr.rs:128-143`. |
| C9 | SA-405 | A store through a run-time index drops the index (hardcodes `[0]`) with no bounds guard, and writable slices use a `const` element pointer so `borrow s: u32[]` writes do not compile. Blocks the corpus `fillPattern`. `codegen/expr.rs:249-256`, `codegen/program.rs:72`. |

## Major findings

| Merged | IDs | Finding |
|---|---|---|
| M1 | RV-404 | Two modules with a same-named function collide: calls bind the wrong module's symbol and only one prototype is emitted. `codegen.rs:73,187-192,348-353`, `codegen/expr.rs:433`. |
| M2 | RV-405, SA-409 | Assignment to a live owned target overwrites it without dropping the old value (O3 leak). `own/expr.rs:245-307`, `codegen/function.rs:439-454`. |
| M3 | RV-406, SA-407 | Signed `<<` is emitted without the unsigned idiom, which is undefined behavior and not covered by `-fwrapv`. `codegen/expr.rs:397-403`. |
| M4 | RV-407 | Switch fall-through emits C rejected by `-Werror=implicit-fallthrough`. `codegen/control.rs:413-452`. |
| M5 | RV-408 | Unused parameters and never-called private functions emit C rejected by `-Werror=unused-parameter`/`-Wunused-function`. `codegen/function.rs:113-147`, `codegen/program.rs:395-430`. |
| M6 | RV-409 | A guard-bearing loop test degrades to `BranchShape::Plain` and emits `goto` outside the ladder, violating Ch 17 §1 and the AR-67 decision. `codegen/control.rs:258-279`. |
| M7 | SA-408 | A module-global initializer is emitted without the arithmetic guards, so `let q = 10 / zero;` traps with undefined behavior instead of E11210. `codegen/program.rs:258-270`. |
| M8 | SA-410 | Two `defer`s run in registration order instead of reverse registration order (CL-3). `codegen/function.rs:355-362`. |

## Minor findings

| Merged | IDs | Finding |
|---|---|---|
| m1 | SA-412 | An FFI enum is always emitted as `i32` regardless of its declared backing type; latent until C3 is fixed. `abi.rs:74-76`. |

## Re-review of the fix diff

The single permitted re-review confirmed C1-C3, C5-C9, M2-M8, and m1 resolved
and judged the M3 narrowing (signed `>>` stays signed) correct. It rejected the
fix batch on four remaining critical defects (two introduced by the M2 fix) and
three majors:

| ID | Severity | Finding |
|---|---|---|
| RV-430 | 🔴 CRITICAL | An owned local in a looping scope is double-freed: per-iteration cleanup drops without emptying, and the next iteration's pre-assignment drop frees the dangling handle. Introduced by the M2 fix. `codegen/function.rs:618`. |
| RV-431 | 🔴 CRITICAL | An assignment whose right-hand side reads its own target (`f = Dup(borrow f)`) drops the old value before evaluating the RHS, a use-after-free. Introduced by the M2 fix. `codegen/function.rs:493,518`. |
| RV-434 | 🔴 CRITICAL | Two modules with the same-named generic function: only the first template is emitted and both callers bind one symbol, a silent miscompile. Generic instances are not module-qualified; M1 fixed only the non-generic path. `codegen/program.rs:391`. |
| RV-435 | 🔴 CRITICAL | The hosted-amiga translation unit cannot compile: the emitted fixed-width typedefs (`LONG`/`ULONG`/`BOOL`/`STRPTR`) clash with `exec/types.h`, which the hosted shim includes. Blocks the Phase 8 hosted-amiga leg. `codegen/program.rs:38-47`. |
| RV-432 | 🟠 MAJOR | `for (borrow e in xs)` write-back is lost on `break`/`continue`, a silent wrong result. `own/loops.rs:383-388`. |
| RV-433 | 🟠 MAJOR | Two modules exporting a same-named function, one imported: the call cannot be disambiguated and emits an implicit declaration; fails `-Werror`. `codegen.rs:367`. |
| RV-436 | 🟠 MAJOR | An assigned-but-unread local emits `-Werror=unused-but-set-variable`. `codegen/function.rs`. |
| RV-437 | 🟡 MINOR | Two lines were mangled into one by an automated edit; cosmetic. `codegen/types.rs:53`, `own/expr.rs:687`. |
| RV-438 | 🟡 MINOR | Defers always run before drops within a scope, so CL-3 interleaving is not honoured (the two-defer reverse order is correct). `codegen/function.rs:393-404`. |

`make check` is green and the new `codegen_run_spec_test.rs` genuinely compiles
and runs the emitted C; the memory-safety regressions RV-430/RV-431 are
reproducible under ASan.

## Residual rulings

User ruling (2026-09-20): **fix all** nine residuals. RV-434 is resolved by
detecting the cross-module same-named generic collision and reporting it,
without changing the naming scheme or the `identical_generic_instantiations_share_one_symbol`
oracle; RV-435 by emitting the fixed-width ABI typedefs for the host profile
only. This is the final fix pass — no further independent review runs; the fix
is verified by `make check` plus targeted ASan probes and regression tests.
