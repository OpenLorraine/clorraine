# Phase 7 Review: Standard-Library Slice

> **Document**: 15-phase7-review.md
> **Parent**: [Index](00-index.md)

Post-phase independent review of the standard-library slice. Two reviewers ran
in parallel on the phase diff against baseline tree
`ebcb3a7c44a4001ea1ddcc020b050b97f91d4c3b`: a correctness reviewer and a
memory-safety/semantics auditor. Verified state at review time: `make check`
green (255 unit, 28 codegen-run, 2 compile-fail, 1 determinism, 6 run-fault,
44 stage, 8 stdlib impl).

## Merged findings

| ID | Severity | Title | Location | Trigger | Minimal fix |
|---|---|---|---|---|---|
| RV-501 | 🔴 CRITICAL | A user `func slice`/`rawSlice`/`rawStr` is accepted and silently replaced by the compiler intrinsic | `load/mod.rs:285-303`, `own/call.rs:143-157` | Declare `func slice(...)` and call it: exit 0, emits the intrinsic instead of the user function. `len` is correctly rejected. | Reject any user `func`/`intrinsic` named a compiler intrinsic, and gate intrinsic lowering on the callee being the compiler-known helper. |
| SA-501 | 🔴 CRITICAL | A `take` argument is not moved out at the call site, so caller and callee free the same handle | `own/expr.rs:479`, `own/statements.rs:532-545` | `let s = stringOf("abcd"); eat(s);` with `func eat(take s: String)` → ASan double free; `FreeBuffer` UAF. | After lowering a call, emit a move-out (`= NULL`/`cl_zero`) for every non-duplicable `take` place argument, matching `Statement::Move`. |
| SA-502 | 🔴 CRITICAL | `cl_string_new` capacity arithmetic overflows/truncates, then writes past a tiny block | `shim_host.c:194` (`cl_alloc(unsigned int)`), `shim_hosted_amiga.c:283` | `padLeft("x", 4294967295, 46)` → ASan heap-buffer-overflow. | Size the arithmetic in `size_t`; trap E1 when `cap` exceeds the representable allocation. |
| SA-503 | 🔴 CRITICAL | Runtime guards are hoisted out of `&&`/`||`, so the right operand is guarded even when it is not evaluated | `codegen/expr.rs:30-35`, `codegen/control.rs:279,302` | Corpus `string-parser`: `while (i < total && input[i] != ' ')` traps E11211 at `i == total`. | Emit the right operand's guard inside its conditional evaluation (e.g. `left ? (guard, right) : false`), not before the whole expression. |
| SA-504 | 🔴 CRITICAL | The guard and the `slice` builder re-evaluate side-effecting operands, so the check can be bypassed | `codegen/expr.rs:102-165`, `codegen/calls.rs:31-36,143-152` | `a[d()]` with `d()` returning `0,0,5` → ASan stack overflow; `10/d()` → SIGFPE; `slice(src, lo(), hi())` → heap overflow. | Lower each guarded operand to a compiler temp once and use the temp in both the guard and the operation. |
| RV-504 | 🟠 MAJOR | `text(v)` has no type, so the `String`→`str` widening is skipped and invalid C is emitted | `sema/call.rs:67-76`, `codegen/expr.rs:92` | `println(text(7))` → `cc` rejects (`ClStr` vs `void *`). | Register the compiler-known `text` as returning `String`. |
| RV-505 | 🟠 MAJOR | Qualified stdlib access without an import (MOD-13) emits an undefined bare call | `driver.rs:132-135`, `codegen/expr.rs:429,497` | `std.mem.bufferNew(4)` with no import: exit 0, emitted C calls undefined `bufferNew`. | Inject the stdlib modules named by qualified references, or diagnose an unknown qualifier. |
| RV-506 | 🟠 MAJOR | A call through a `func`-typed field emits `f(receiver, args)` (undefined symbol) | `own/call.rs:35-40`, `codegen/expr.rs:481-498` | `struct Cb { f: func(i32): i32; } … c.f(0)` → emitted `f(c, 0)`. | Emit `((base).field)(args)` without prefixing the receiver. |
| SA-505 | 🟠 MAJOR | `text`/templates for `u8`/`i8`/`u16`/`i16`/`usize`/`isize`/`f32` emit undefined `cl_text_<key>` | `codegen/calls.rs:509-521`; shims define only `i32/u32/f64/bool/str/hex_u32` | `let n: usize = 42; println($"n={n}");` → implicit declaration; breaks corpus `buffer-container`/`file-copier`. | Define the missing `cl_text_*` in both shims, or normalize the conversion type. |
| SA-506 | 🟠 MAJOR | Owned temporaries are never dropped, so template/call results leak | `codegen/expr.rs:519`, `shim_host.c:275-290`, `ir.rs` temps | `let d = bytes(stringOf("hello"));` → LSan leak. | Materialize owned call results in temp locals registered in the statement cleanup. |
| SA-507 | 🟠 MAJOR | `slice` on a fixed array emits `(arr).ptr`/`(arr).len` → uncompilable C | `codegen/calls.rs:35,149` | `slice<u8>(a, 0, 2)` with `a: u8[4]` → `cc` "not a structure or union". | Build a `ClSlice_T` from a fixed-array operand, or reject the coercion in sema. |
| RV-502 | 🟠 MAJOR | False W10000 on a local used only as a dot-call receiver | `own/expr.rs:415-418`, `own/call.rs:22-36` | `let n = xs.len();` → `warning W10000: unused local 'xs'`; fatal under `--deny-warnings`. | Evaluate the receiver in `check_call_expr` instead of dropping it. |
| RV-503 | 🟠 MAJOR | The dot-call receiver is not borrow-checked, so "no reallocation while a view is live" is unenforced | `own/expr.rs:415-422` | `let v = xs.get(0); xs.push(1);` compiles; the free-call form is correctly E10032. | Reuse the argument ownership check for the receiver at index 0. |
| SA-508 | 🟠 MAJOR (deferred) | The host profile accepts `std.amiga.*` imports and emits unlinkable C | `driver.rs:119-151` | `clc --profile host` on corpus `interrupt-counter`: exit 0, undefined `cl_delay_ticks`. | Reject profile-unavailable modules with E11200. **Already scheduled as Phase 8 task 8.1.1.** |
| RV-507 | 🟡 MINOR | `intrinsic func <reserved>` in a module named like a stdlib module bypasses E20006 | `load/mod.rs:285` | A file starting `module std.mem;` then declaring `rawSlice` compiles. | Track the item's origin, not the module name. |
| RV-508 | 🟡 MINOR | A `func` FFI parameter lowers to `void *`, not a C function pointer | `abi.rs:59`, `codegen/types.rs:36,81` | `cl_add_handler(APTR)` instead of a function-pointer type. | Map `TyKind::Func` to a C function-pointer declarator, or record an AR for the deviation. |
| SA-509 | 🟡 MINOR | Buffer growth arithmetic can wrap; `cl_buffer_write` does not bound `off` | `shim_host.c:146,162,134`, `shim_hosted_amiga.c:235,251,223` | 32-bit `needed = len + n`/`off + n`/`cap *= 2` can wrap; `off` untrusted. | Overflow-checked arithmetic; validate `off`/`n`; trap on wrap. |
| SA-510 | 🟡 MINOR | The interrupt handler is passed as `void *`, the server return is unchecked, and the token can wrap to 0 | `shim_hosted_amiga.c:111-129` | Non-ISO function-pointer→object-pointer conversion; unchecked `AddIntServer`. | Typed `void (*)(void)` parameter; check the server status; skip token 0 on wrap. |
| RV-509 | 🟡 MINOR (pre-existing) | Two shared loans of one owner are rejected (`Loans::conflict` ignores the requested kind) | `borrow.rs:92` | `let v1 = bytes(s); let v2 = bytes(s);` → E10032; Ch 16 §1 permits any number of shared reads. | Root is Phase 5; `conflict(owner, Shared)` should test only for a live exclusive loan. |

## Non-findings
- The template builder frees only owned placeholder strings and never a borrowed
  one; the builder and the ladder cannot both free one object (the only defect
  there is the SA-506 leak).
- `FreeBuffer`/`FreeString`/`Close` are null-safe; generated drops empty the
  place, so no double drop was found beyond SA-501.
- `cl_isnan` bit-copy, `cl_zero`, the `ClStrPart` array, and the `BPTR`↔`FILE *`
  round-trip are sound on host and the 32-bit target.

## Rulings

User ruling (2026-09-21): **fix all except SA-508** — the six criticals, the
five majors (RV-502/503/504/505/506, SA-505/506/507), and the five minors
(RV-507/508/509, SA-509/510), each with a regression case. SA-508 stays with
Phase 8 task 8.1.1 (the profile-availability diagnostic). After the fixes
verify, the single permitted scoped re-review runs on the fix diff.

## Fix batch (commit `fae1664`)

All 18 findings were fixed; `make check` green (255 unit, 38 codegen-run,
2 compile-fail, 1 determinism, 6 run-fault, 44 stage, 8 stdlib impl), and the
corpus `string-parser`/`buffer-container`/`file-copier` host legs match their
expected output. SA-510's server-return check was reported impossible because
the pinned NDK declares `AddIntServer`/`RemIntServer` as returning `void`.

## Re-review of the fix diff

The single permitted re-review confirmed RV-504, RV-505, RV-506, SA-501,
SA-502, SA-503 (guard placement), SA-505, SA-507, RV-502, RV-503, RV-507,
RV-508, SA-509, SA-510, and RV-509 resolved. It rejected the batch on three
defects introduced by the fix and three partial residuals:

| ID | Severity | Title | Location | Evidence |
|---|---|---|---|---|
| RV-603 | 🔴 CRITICAL | An owned temporary is declared in an inner block, so a view stored in an outer struct dangles after the block ends | `own/lower.rs:234-263` | `struct S { v: u8[]; }`; `{ s = S{ v: bytes(stringOf("hi")) }; }` then use `s.v[0]` → ASan heap-use-after-free. Pre-fix it leaked (the view stayed valid), so this is a SA-506 regression. |
| RV-602 | 🟠 MAJOR | Materialized operands are hoisted out of `&&`/`||`/`?:`, so a side effect on an unevaluated operand still runs | `own/lower.rs:38-51,96-100,125-133`, `codegen/guards.rs:953-994` | `x = flag && (a[next()]==99)` with `flag=false` calls `next()` once. Partially re-opens SA-504. |
| RV-601 | 🟠 MAJOR | A call through a `func`-typed field applies the same-named global function's `take` move-out, leaking the argument | `own/call.rs:168-174,304-318`, `own/lower.rs:63-82` | A field call `c.f(s)` where a global `func f(take s: String)` exists emits `s = NULL` although the field callee took nothing → LSan leak. |
| SA-504r | 🟠 MAJOR | The assignment-target index is still re-evaluated (the read path was materialized, the store path was not) | `own/expr.rs:332-340`, `codegen` indexed store | `a[next()] = 99` emits `if (next() >= 4) trap; ... a[next()] ...`; a `next()` returning `1` then `100` passes the guard and writes out of bounds (UBSan). |
| SA-506r | 🟠 MAJOR | Owned results in template placeholders still leak | `own/lower.rs:163-175`, `codegen/expr.rs:389,453` | `$"{text(7)}-{stringOf("abc")}"` leaks both source Strings because `text_call` copies an owned String and never drops the source. |
| RV-501r | 🟡 MINOR | A `comptime func` named a reserved intrinsic is not rejected | `load/mod.rs:305-309` | `comptime func slice<T>(…)` is accepted and a call emits the intrinsic. `check_reserved_declarations` covers `Item::Func`/`Item::Intrinsic`, not `Item::Comptime`. |

Adjacent pre-existing gaps noted by the re-review but not caused by this diff:
a local `func` value with a global's name resolves to the global symbol;
`text()` with zero args and `text(struct)` emit invalid C.

### Residual ruling

User ruling (2026-09-21): **fix all six** — RV-603, RV-602, RV-601, SA-504r,
SA-506r, RV-501r — each with a regression case. The single permitted scoped
re-review is spent, so this follow-up is verified by `make check` plus targeted
ASan/UBSan probes, with no further independent review.
