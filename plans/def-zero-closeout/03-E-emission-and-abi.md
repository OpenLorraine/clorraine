# Batch E — Emission and ABI

> **Document**: 03-E-emission-and-abi.md
> **Parent**: [Index](00-index.md)
> **Rows**: 37

## Scope

The rows listed below are fixed together because they share an owning stage and regression helpers. Each row keeps its own ST case and its own acceptance evidence.

## Per-Row Specification

### DEF-194 — A constant-bounds `slice` of a runtime view emits a guard that fails the strict build (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), surfaced by the Phase 4 reviews (N-4): `slice(v, 0, 0)` on a runtime view emits a constant `0 > v.len` guard that trips `-Werror=type-limits` at the driver's C build.
- **Fix:** fold the constant comparison or keep the guard in the runtime helper.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-199 — A closure-call argument needing the `str` widening emits the raw handle (minor)

- **Observed:** 🟡 minor, pre-existing (base == head, body and initializer alike), surfaced by the batch F Phase 1 semantics review (PR-4): `((s: str) => i32(len(s)))(stringOf("hello"))` emits a `ClString` argument to the `ClStr` parameter and the strict build fails; the named-call path widens.
- **Fix:** apply the `store_text` widening in the closure-call argument rendering.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-200 — A qualified function value reference emits a C member path (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), surfaced by the batch F Phase 1 emission audit (P5): `let f: func(): i32 = A.B.helper;` emits `A.B.helper` where the module symbol `cl_A_B_helper` is required; the call spelling works.
- **Fix:** render a qualified function value as its module symbol.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-201 — A constant zero divisor inside a lambda body fails the strict build (minor)

- **Observed:** 🟡 minor, pre-existing (base == head, body and initializer alike), surfaced by the batch F Phase 1 emission audit (P7): a literal `1 / 0` in a lambda body emits `/ 0`, rejected under `-Werror=div-by-zero`.
- **Fix:** fold or reject constant zero divisors before emission.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-202 — A func-typed fixed-array global emits an invalid C declarator (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), surfaced by the batch F Phase 1 re-review (P3): `let fs: func(): i32[2]` (a fixed-array-returning func-typed array global) emits `static i32 (*)(void) name[2]`, so `fs[1]()` cannot build at any ref; the call text improves with the callable fix but the declarator stands.
- **Fix:** render the element declarator for func-typed arrays.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-206 — A fixed-array struct field built from an array literal re-emits an element guard (minor)

- **Observed:** ` operand and in a lazily evaluated position, where the walk also runs with no guard.
- **Fix:** thread a guard-emission flag through the fixed-array store core for the initializer path, or pre-bind guard-carrying elements.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-208 — A conditional with mixed `str` and `String` arms emits an invalid ternary (minor)

- **Observed:** 🟡 minor, pre-existing in both spellings (base and head fail the strict build identically: `type mismatch in conditional expression`), surfaced by the batch F Phase 2 emission audit (P3): the arms do not share one C type because only the chosen spelling widens to the view.
- **Fix:** widen the `String` arm to the view when the conditional's type is a view.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-217 — A slice of a func-pointer element emits an invalid C declarator (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), from the batch F Phase 4 emission audit (P4): a parameter `(func(i32): i32)[]` emits `typedef struct { i32 (*)(i32) *ptr; usize len; } ClSlice_fn_i32__i32;` — an invalid declarator; the body `sizeof((func(i32): i32)[])` reaches the same invalid typedef.
- **Fix:** the C declarator renderer for a function type under a pointer or field position.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-218 — An explicit `slice<i32[]>` with a discarded result emits a call to an undeclared `slice` (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), from the batch F Phase 4 emission audit (P5): an explicit `slice<i32[]>` whose result is discarded emits a call to the undeclared intrinsic `slice`.
- **Fix:** route the explicit-type-argument builder through the view-builder gate.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-221 — A fixed-array literal base with a run-time index emits a bare brace list (minor)

- **Observed:** 🟡 minor, pre-existing (base == head byte-identical), from the batch G Phase 1 emission audit (P-1) and correctness review (P2): `let g: i32 = [10, 20, 30][pos];` emits `{{ 10, 20, 30 }}[pos]`, which C99 rejects (`expected expression before '{{'`); the phase deliberately keeps fixed-array bases unstored.
- **Fix:** bind an array-literal base into an init local before the access.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-222 — An instance recorded from a type argument is not field-expanded (minor)

- **Observed:** 🟡 minor, pre-existing (base failed one level earlier), from the batch G Phase 1 correctness review (P1): `sizeOf<Box<i32>>()` with `struct Box<T> {{ p: Pair<T>; }}` names `ClPair__i32` undeclared because the field-expansion fixpoint ran before the new recording; the direct spelling `let b: Box<i32>;` fails identically at base and head.
- **Fix:** run the instance field expansion after all recordings (or expand at recording time).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-223 — The global-initializer `sizeof(Wrap<i32>[])` spelling names an undeclared instance (minor)

- **Observed:** 🟡 minor, pre-existing (base == head byte-identical), from the batch G Phase 1 emission audit (P-2): `let n: usize = sizeof(Wrap<i32>[]);` at module level emits `ClSlice_Wrap__i32` naming undeclared `ClWrap__i32` — the initializer twin of DEF-216, whose function-body recording does not cover initializer expressions.
- **Fix:** record a written type argument's struct instances from initializer expressions too.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-224 — The concrete value-form `.len` on `cstring`/`String` (and `len(cstring)`) emits invalid C or 0 (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), surfaced by the batch G Phase 2 reviews (semantics PE-002, correctness): a concrete `s.len` on a `String`/`cstring` value is accepted and emits `request for member 'len' in something not a structure or union`; `len(cstring)` silently emits `0` (the shim's `cl_cstring_len` is unused by the value path).
- **Fix:** route the value-form `.len` through `cl_string_view(x).len`/`cl_cstring_len(x)` or reject the spellings.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-225 — A mixed-arm generic conditional `len` folds to 0 (minor)

- **Observed:** 🟡 minor, pre-existing (base == head byte-identical), from the batch G Phase 2 semantics review (PE-001): `len(flag ? b.s7 : b.s3)` at a `{{ s7: i32[7]; s3: i32[3]; }}` instance is accepted and runs 0, where the concrete spelling is `E10200` (Ch 04 §5 requires same-typed branches).
- **Fix:** judge the arms' extent agreement at the instance and report the conversion diagnostic, so the accepted mixed form cannot reach the const-0 fallback.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-226 — A nested opaque projection chain with a missing deeper field stays accepted and emits invalid C (minor)

- **Observed:** 🟡 minor, pre-existing (base == head byte-identical), from the batch G Phase 2 correctness review: `x.v.foo` with the outer field resolving and the deeper `foo` missing is accepted (only the first segment records an opaque projection) and the emitted C fails `'ClInner' has no member named 'foo'`.
- **Fix:** record the projection chain and resolve each segment through the substituted type.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-227 — A lifted lambda capturing a fixed array emits invalid C (minor)

- **Observed:** 🟡 minor, pre-existing (base == head byte-identical), from the batch G Phase 2 correctness review: `eachLambda<i32[4]>` (a lifted lambda whose environment carries a fixed array) is accepted and the emitted C fails on the environment initialization.
- **Fix:** bind an array capture into the environment through the byte-copy rules; the batch G Phase 5 fix round's projection judgment newly accepts the duplicable array-element projection spelling (`make<O>([o, o2])` with `xs[0].n`), which this same gap renders unbuildable — recorded here as the boundary.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-228 — A generic `raw` instance bypasses the `unsafe` dereference gate (minor)

- **Observed:** 🟡 minor, pre-existing (the phase baseline `5a378fb` and the head behave identically; the intermediate fix-round base `8f0e5b8` rejected the instance with a false `E10212`), from the batch G Phase 2 correctness review: `func get<T>(x: T): i32 {{ return x.foo; }}` instantiated at `raw S` accepts the member read without an `unsafe` block, where the concrete `p.foo` requires one (the audit's raw acceptance mirrors the value-form checker, which has the same hole).
- **Fix:** carry the unsafe gate into the value-form raw member read and the instance audit.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-234 — Fold programs segfault on the host no-libc freestanding leg (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical; base binary segfaults too), from the batch G Phase 2 emission audit (P-6b): programs folding `len` under `-ffreestanding -fno-builtin -nostdlib` on the host segfault at entry.
- **Fix:** diagnose the host no-libc entry/alignment artifact in the freestanding host leg.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-253 — A concrete discarded array literal is accepted and emits C the strict build rejects (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), reported by the batch G Phase 5 correctness review (the RV-002 note): `let s: String = stringOf("ab"); [s];` is accepted and the emitted C fails the strict build (`expected ';' before '}' token`); the capture twin is now rejected by the element recording, the concrete spelling stays accepted-invalid; the same applies to the generic duplicable spellings (`make<i32>` with a discarded `[x];`) at every ref.
- **Fix:** lower or reject a discarded array literal.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-263 — A fixed-array `.len` receiver folds to a constant and never evaluates (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch H Phase 2 correctness review (RV-202): `(c ? mkR(1) : mkR(2)).xs.len()` over a fixed-array field folds to the constant and never runs the receiver, so a panic planted in `mkR` never fires; `spec/08-arrays-strings.md` makes the length compile-time but is silent on the operand's evaluation. Needs a spec ruling on the folded receiver's evaluation before a fix
- **Fix:** Match the concrete twin and the frozen specification; remove the invalid C or wrong value.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-313 — An all-null conditional as a call argument emits invalid C (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-J phase-1 memory audit of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md): `fill(c ? null : null, borrow out)` is accepted and the C fails the strict build (`incompatible type for argument 1`).
- **Fix:** render the all-null argument as the destination empty view (the phase-1 all-null renderer covers the store positions).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-320 — A nested-conditional carrier read-back emits invalid C (minor)

- **Observed:** 🟡 minor, pre-existing (base == head, byte-identical), found by the batch-J phase-2 correctness review (p292_nested): the nested-conditional arm is not descended, so the carrier read-back emits invalid C.
- **Fix:** descend the nested arm in the view positions.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-324 — The guarded literal spellings keep pre-existing invalid-C emissions (minor)

- **Observed:** 🟡 minor, pre-existing (base == head, recorded by the batch-J phase-3 emission audit of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md)): the guarded conditional-root field `Holder { row: (c?a:b).row }` (`lvalue required`), the guarded element conditional with call-filled literal arms (`too few arguments`), and the guarded ternary false-arm literal (`missing-braces`/`Wint-conversion`). The plain and unguarded twins are fixed.
- **Fix:** extend the per-arm routes to the guarded spellings.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-325 — A call-expression array fill re-evaluates per element (minor)

- **Observed:** 🟡 minor, pre-existing (base == head, byte-identical emission), found by the batch-J phase-4 memory audit of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md): a call fill (`[1, 2, 3; f()]`) runs the call once per element instead of once (Ch 04 §3); no memory effect (AL-1 forces a duplicable fill), a wrong-value/side-effect defect. The phase-4 fix pins the update-bearing fill; the call/conditional fill spellings remain.
- **Fix:** pin every update/effect-bearing fill once.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-326 — A deferred-arm fixed-array fill re-evaluates its update per element (minor)

- **Observed:** 🟡 minor, pre-existing (base == head, byte-identical emission), found by the batch-J phase-4 emission audit of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md): the deferred-arm spelling of an update-bearing fill runs the update once per element (exit 5 on both sides); the plain spelling is pinned once by the phase-4 fix.
- **Fix:** pin the deferred fill before the element stores.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-327 — The loop-condition place-borrowing receiver keeps GCC 15's sequence-point false positive (minor)

- **Observed:** 🟡 minor, build-quality residual, found by the batch-J phase-4 correction round of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md): the safe fix keeps the deferred spelling for this one shape (the borrowed place must stay a read), which GCC 15.2 flags as `-Werror=sequence-point` while GCC 12.2 accepts; eager and all-fresh receivers are host-clean.
- **Fix:** a read-only receiver carrier (a plain-copy fill with an unregistered temporary), noted as the follow-up.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-328 — A member over a parenthesized deref is accepted and silently dropped (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-J phase-4 correctness review of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md) (J4R-5): `(*p).n = 5` and `(*p).m += 6` are accepted and emit nothing (the same failure mode as `DEF-310` for an equivalent target spelling); `p.n = 5` and `q[0] = 9` work.
- **Fix:** route the member-over-deref base as a place (or reject the spelling).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-330 — A raw source pointer used as a store value's deref reports a spurious unused-local warning (minor)

- **Observed:** 🟡 minor, pre-existing (verified by a mixed-tree regression test at `0947475` vs `5f2a78b`: the same one `W10000` on both), found while verifying the batch-J phase-4 correction round of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md): `*q = *r` reports `W10000: unused local 'r'` although `r` is dereferenced.
- **Fix:** mark the deref source pointer used in the checker.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-334 — A deferred generated owner whose rolled-back id a later binding re-takes renders that binding's name (minor)

- **Observed:** 🟡 minor, pre-existing class (the removed prefix fallback failed identically), found by the batch-J phase-5 bounded confirmation of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md) (J5C-1): a rolled-back deferred copy's parked id dropped for a later binding and then re-taken renders the later binding's name in the message (the site is always rejected; only the subject is wrong).
- **Fix:** remember the rollback-time display subject, or reserve the parked ids.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-352 — A raw pointer's `.ptr`/`.len` members emit the arrow shape and fail the strict build (minor)

- **Observed:** 🟡 minor, pre-existing (reproduced with a `raw u8` local twin at both refs), found by the batch-K phase-4 F1 correction of [residual-major-closeout](residual-major-closeout/00-index.md): `Make()->ptr`/`Make()->len` (and the `raw u8` local twins) emit the struct-member arrow on a pointer, which `cc -Werror` rejects; the `len` of a pointer also folds to the empty length and leaves a set-but-unused local. Consistent for the ffi result and the plain pointer.
- **Fix:** reject the members on a raw pointer, or type the raw pointee access.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-355 — A struct-literal compound-initializer field never reaches the typed-null lowering (minor)

- **Observed:** 🟡 minor, pre-existing (base == head; both fail the amiga strict build identically), found by the batch-K phase-5 correctness review of [residual-major-closeout](residual-major-closeout/00-index.md) (K5R-P1): `Box { f: n }` at a `File?` field emits `(ClBox){ n }` — `initialization of 'long int' from 'void *'` — while the store path handles the same field.
- **Fix:** route the compound-initializer field through the typed-null lowering.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-358 — The `run_with_timeout` harness never drains its pipes, deadlocking ASan children (minor)

- **Observed:** 🟡 minor, test-infrastructure defect (the container ASan suite flakes on a random 4-9 tests; the base flakes identically; the tests pass in isolation), diagnosed by the batch-K phase-6 semantics review of [residual-major-closeout](residual-major-closeout/00-index.md): `def_backlog_closeout_spec_test.rs:147-174` pipes stdout/stderr and never drains them, so ASan children block in `anon_pipe_write`.
- **Fix:** drain the pipes (a reader thread or inherit).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-392 — The `sizeof` constant fold declines enums, handles, and views (minor)

- **Observed:** 🟡 minor, pre-existing (head and base `E10224` in constant contexts while the value position emits the C spelling), found by the batch-L phase-8 semantics review of [residual-major-closeout-ii](residual-major-closeout-ii/00-index.md) (L8S-6): `const n = sizeof(E)`, an enum field inside a struct extent, a handle, a slice type.
- **Fix:** extend the ABI layout/fold to the enum backing scalar, the handle pointer, and the view struct (or document the exclusion).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-393 — `offsetof` is wrong outside the acceptance checks (minor)

- **Observed:** 🟡 minor, pre-existing (base identical), found by the batch-L phase-8 semantics review of [residual-major-closeout-ii](residual-major-closeout-ii/00-index.md) (L8S-7): the value position emits `0`, the folded offset is unpadded, the field name must also be a binding, and an enum member's `sizeof` emits `0` in the emitted macros (`codegen/program.rs:817-838`, `codegen/calls.rs:443`).
- **Fix:** emit the real C offset/size spellings.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-438 — An eager take deref argument hoists its pointer fill above an earlier argument's read (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-M phase-6 semantics review of [residual-major-completion](residual-major-completion/00-index.md) (L6S-4): `g(idx, take *nextp(p,q,&idx))` hoists the pointer fill so the first argument sees `idx = 1` (Ch 04 §3 :59 argument order).
- **Fix:** apply the argument-position pin discipline.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-482 — An all-fresh guarded read base with mixed call/literal arms has mismatched C arm types (minor)

- **Observed:** 🟡 minor, pre-existing (base C failure of a different kind), found by the batch-N phase-6 semantics review: `g ? (ok ? mkInner() : Inner { row: n2() }).row[0][0] : 0` emits a `?:` with one struct-typed and one value-typed arm.
- **Fix:** make each expression arm yield the target struct value.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-500 — Three strict-build residual classes fail `-Werror` (minor)

- **Observed:** 🟡 minor, pre-existing (base C identical): four null-shaped value programs emit `void *` to `i32` conversions, three loop-receiver programs trip `-Werror=sequence-point`, and the `embed` compile-fail golden is accepted and emits invalid C.
- **Fix:** one emission fix per class with a strict-build pin.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.
