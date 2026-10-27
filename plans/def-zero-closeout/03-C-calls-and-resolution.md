# Batch C — Calls and Resolution

> **Document**: 03-C-calls-and-resolution.md
> **Parent**: [Index](00-index.md)
> **Rows**: 22

## Scope

The rows listed below are fixed together because they share an owning stage and regression helpers. Each row keeps its own ST case and its own acceptance evidence.

## Per-Row Specification

### DEF-169 — The func type key's fixed separators can textually collide with user identifiers (minor)

- **Observed:** 🟡 minor, fail-closed (never a runnable wrong program), introduced by the callable-release-integrity Phase 1 key grammar (PRE-3): `struct borrow_i32` and `func(borrow i32)` share the key `fn_borrow_i32__i32`, so the instance redefines a struct and the strict build fails (the same class as the documented `Q`-marker limitation, DEF-103); recorded in the `types/keys.rs` key documentation.
- **Fix:** an escaping scheme for key components is an ABI decision with its own trigger.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-173 — A same-named function in another loaded module supplies modes to a current-module func-typed global (minor)

- **Observed:** 🟡 minor, pre-existing (base == head; identical diagnostics and leak), surfaced by the callable-release-integrity Phase 2 emission audit (EIA PE-5): with `mark(take String)` loaded from another module, `let g: func(String): i32 = lenOf; g(temp)` takes the foreign function's modes (a false `E10020`, or an accepted 11-byte leak). Cause: the modes lookup consults the program-wide simple-name function table before the global.
- **Fix:** a global callee's modes come from the global's own identity only.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-174 — A call-site marker on a closure-call argument is accepted (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), surfaced by the callable-release-integrity Phase 2 reviews (correctness P-2, emission M04): `((p: String) => i32(len(p)))(take stringOf("x"))` is accepted while the module, field, and bound twins report `E10039`.
- **Fix:** compare the written marker against the lambda signature's modes at the closure-call site.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-198 — A call-site marker in an initializer call is accepted (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), surfaced by the batch F Phase 1 semantics review (PR-3): `let g: i32 = f(take stringOf("ab"));` is accepted where the body twin reports one `E10039`; the initializer call check never judges marker matches.
- **Fix:** run the call-marker check over initializer calls.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-203 — An initializer dot call on a global receiver is accepted where the body rejects (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), surfaced by the batch F Phase 1 re-review (N1): an initializer accepts a dot call that borrows a global receiver (`E10038 cannot borrow global` in the body twin); the initializer check walk is planned for the batch's Phase 3.
- **Fix:** include the global-borrow rule when the initializer check walk lands.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-205 — A body func-typed global named like a view builder is intercepted by the intrinsic gate (minor)

- **Observed:** 🟡 minor, pre-existing (base == head; the initializer twin is fixed by batch F Phase 1), surfaced by the batch F Phase 1 emission re-audit (P3): a body call of a func-typed global named `slice` emits `((ClSlice_unknown){...})` and fails the strict build; the gate checks callable fields but not callable bindings (`own/call.rs:308-355`).
- **Fix:** exclude callable bindings at the body intrinsic gate.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-212 — A local used only as a store index is reported `W10000 unused local` (minor)

- **Observed:** 🟡 minor, pre-existing (base and head identical), surfaced by the batch F Phase 4 correctness review (P-1): `xs[i + 1] = 9;` warns `unused local` for `i` although the emitted C reads it in the store pointer and its bounds guard; a read position does not warn.
- **Fix:** Match the concrete twin and the frozen specification; remove the invalid C or wrong value.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-214 — A call of a non-callable member or value is accepted (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), surfaced while scoping DEF-148: `s.x(3)` (a field that exists but is not callable) emits `x(s, 3)` and `n()` (an `i32` local) emits `n()`, both failing the strict build; PF-016's field-wins rule resolves the field, but no diagnostic covers the call.
- **Fix:** reject a call whose callee resolves to a non-callable value (the code choice needs a ruling; out of batch G's scope).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-236 — A call through a func-typed value with an arity mismatch emits invalid C (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), surfaced by the batch G Phase 3 arity work: `let f: func(i32): i32 = g; f(1, 2)` is accepted and the strict build fails `too many arguments to function 'f'`; the func-value path passes no callee name, so the `E10308` rule covers only calls that name their callee.
- **Fix:** report the arity for a value callee (a closure literal and a bound parameter call share the gap).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-239 — The `Array` dot-call arm bypasses the zero-parameter candidate rule (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), from the batch G Phase 3 correctness review: `xs.arrayNew()` is accepted (the `std.mem` `Array` arm accepts any visible function) and emits `cl_std_mem_arrayNew(xs)` with an unsubstituted `ClArray__T`; the free-function path applies the candidate rule only.
- **Fix:** require the receiver-taking parameter on the `std.mem` arm too.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-240 — A module-qualified direct call bypasses the arity rule (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), from the batch G Phase 3 correctness review: `rv.p26c.addOne(1, 2)` and `std.text.toCString(s, 1)` are accepted and fail the C prototype; the qualified-callee path resolves the free function but never reaches the named-call branch's count check.
- **Fix:** report `E10308` on the qualified path with the member's name, and route the qualified call through the same `ffi` argument checks (the Phase 3 semantics re-review's SR-004: `probe.Open(view, 0)` is accepted and fails the strict build).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-241 — A raw receiver's callable field emits a dot instead of an arrow (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), from the batch G Phase 3 correctness review: `p.foo(1)` through `raw S` emits `p.foo(1)` where C wants `p->foo(1)`; the E10212 expansion correctly leaves the resolving field alone.
- **Fix:** render a field call through a raw base with the implied dereference.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-244 — A never-called `ffi` declaration's parameter types are never validated (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), from the batch G Phase 3 semantics re-review (PE-3): `ffi { func F(x: i32[0]): i32; }` never called is accepted with no `E10226`, where an ordinary declaration reports; the phase's cache validates at the first call site only.
- **Fix:** validate the ffi declarations once at the module check.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-254 — `--max-errors 0` silently accepts rejected programs (minor)

- **Observed:** 🟡 minor, pre-existing (base–head identical), reported by the batch G Phase 5 correctness re-review: `max_errors: Some(0)` drops the first error before the count rises (`diag.rs::push` returns before `error_count += 1`), so the driver reports success and writes a translation unit for a rejected program.
- **Fix:** count every reported error regardless of the retention cap, or reject `--max-errors 0` as invalid.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-349 — The `E10200` help's suggested cast is not host-build-clean (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-K phase-3 correctness review of [residual-major-closeout](residual-major-closeout/00-index.md) (P-5): `usize(s.ptr)` is accepted but the host C cast warns `-Werror=pointer-to-int-cast` — the diagnostic's suggested escape hatch does not build.
- **Fix:** the sanctioned integer spelling for a raw pointer, or a handler-friendly cast emission.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-382 — The argument route loses literal-context typing (minor)

- **Observed:** 🟡 minor, pre-existing (base == head, byte-identical), found by the batch-L phase-6 semantics review of [residual-major-closeout-ii](residual-major-closeout-ii/00-index.md) (L6S-3): `identity<u8>(5)`, `pair(b, 1)`, `consume2<u8>(5)` report `E10200` while their concrete twins accept (a fitting literal must take the parameter's type). Fold into the destination-side literal-kind fix.
- **Fix:** pass the expected context into `check_generic_call`.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-387 — A bare member read matching a visible free function escapes the unknown-member report (minor)

- **Observed:** 🟡 minor, pre-existing (base == head; the instance twin rejects), found by the batch-L phase-7 semantics review of [residual-major-closeout-ii](residual-major-closeout-ii/00-index.md) (L7S-2): `s.foo` beside `func foo(x: str)` is accepted on every concrete base (the emitted C fails `'ClStr' has no member named 'foo'`), while `x.foo` on a substituted scalar rejects `E10212`; the free-function dot-call exemption (`sema/expr.rs:838`) is too broad.
- **Fix:** narrow the exemption to the dot-call receiver resolution, or report at the bare value read.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-388 — An unknown member on a null base falls through unreported (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-L phase-7 semantics review of [residual-major-closeout-ii](residual-major-closeout-ii/00-index.md) (L7S-3): `null.foo` is accepted and emits `NULL.foo` (the strict build fails); `sema/expr.rs:824-828`'s unknown-base fallthrough reports nothing.
- **Fix:** report the null/member misuse with the E10212 class.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-389 — A module-path method-shaped call reports the member error where the base reported the undeclared path (minor)

- **Observed:** 🟡 minor, pre-existing rejection class (rejected at both refs; the diagnostic differs: base `E10219`, head `E10212`), found by the batch-L phase-8 correctness review of [residual-major-closeout-ii](residual-major-closeout-ii/00-index.md) (L8R-4): `spec.rev8.methodpathmod.origin.getx()` is a module-path receiver dot-call — the pre-existing gap around Ch 07's dot-call rule for a module-path receiver; the phase's module-binding work narrows the reported code.
- **Fix:** resolve the module-path receiver's dot-call (the free function) or keep the E10219 selection.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-404 — A dot or qualified call rejects a str literal for a cstring parameter (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), found by the batch-M phase-1 semantics review of [residual-major-completion](residual-major-completion/00-index.md) (L1S-5): `1.put("x")` and `probe.qc.lib.emit("hi")` report `E10200`, while the direct `put(1, "x")` accepts (the literal widening).
- **Fix:** apply the literal widening in the dot-call path and for a named qualified callee.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-405 — An explicit-type-argument generic call rejects a literal the parameter context would type (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), found by the batch-M phase-1 semantics review of [residual-major-completion](residual-major-completion/00-index.md) (L1S-7): `id<u8>(7)` reports `E10200` while `b.id(7)` with `b: u8` accepts. Check whether the P3 `DEF-371` literal-context fix covers this route; close as incidental if so.
- **Fix:** type the written arguments against the explicit type arguments before judging the literal.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-419 — Import-renamed type names do not resolve (minor)

- **Observed:** 🟡 minor, pre-existing, found by the batch-M phase-3 semantics review of [residual-major-completion](residual-major-completion/00-index.md) (L3S-007): `import { Array as A }` + `A<i32>` emits `i32` and fails the strict build.
- **Fix:** honour the module's import bindings in type resolution.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.
