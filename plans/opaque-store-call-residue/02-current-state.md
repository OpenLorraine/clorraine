# Current State

Every row is reproduced at the planning baseline `766e9b9` with the host profile and, where the
program is accepted, a strict C build. The observed result is the sanitizer report, the
strict-build failure, or the unexpected acceptance.

| Row | Repro (filed) | Observed at `766e9b9` | Site |
|-----|---------------|-----------------------|------|
| `DEF-525` | `id<T>(take h: T): T { let a: Box; let s: str = a.v; h.names[0] = s; return h; }` at `T = Holder` | accepted; `a.v` is released at the callee's exit and the caller reads the freed bytes (ASan heap-use-after-free); `h.names[0] = a.v` emits `ClStr = ClString` and strict C rejects it | `own/expr.rs:1220-1395` (the local store records no view loan when `target_ty` is unknown); `own/statements.rs:1599-1690` (the return consults only recorded loans) |
| `DEF-526` | `let r: str = ""; ([p, borrow r] () => { r = p; })(); store(r, borrow slot);` | rejected `E10030` at the call, although `r` holds the caller's parameter view | `own/call_check.rs:372-405` (`mark_written_captures` marks every write-borrow local frame-backed) |
| `DEF-527` | `let v: (func(i32): i32)[] = slice(a, 0, 2);` | accepted; emits `typedef struct { i32 (*)(i32) *ptr; usize len; } ClSlice_fn_i32__i32;`, which strict C rejects | `codegen/program.rs:181-192` (manual `c_type(element) + " *ptr"`); `codegen/types.rs:309-310` (`declarator`'s `Raw` arm appends `*` to a function type) |
| `DEF-528` | `let f = c ? [x]() => x : [x]() => x;` | rejected with `E10200: cannot convert 'closure(): i32' to 'closure(): i32' implicitly` plus the storage rule; a capture-less conditional is accepted | `types/mod.rs:907-928` (`conditional_result`; two closure kinds differ by site id); `sema/ops.rs:424-440` (the else-arm conversion report) |
| `DEF-395` | `let x: String = w.pick(s2, s3);` with `pick<T>(take a: Wrap<T>, ...): T` | accepted; emits `ClT __cl_tmp1 = 0;`; strict C rejects `unknown type name 'ClT'` | `sema/call.rs:531-575` (`check_dot_call` returns `signature.returns` after the generic branch computes the bindings) |
| `DEF-396` | `probe.qgl.id(5)` and `probe.qselfm.id<i32>("x")` | the first falsely reports `E10200`; the second is accepted and the C calls `cl_id__i32(ClStr)`; a wrong arity and a wrong type-argument count are accepted | `sema/call.rs:40-60` (a `Qualified` callee falls through to `check_value(callee)`; `call_name` matches only `Name`) |
| `DEF-401` | `b.pick<i32>(c)` on a `Box` receiver | accepted; the explicit argument is dropped before `check_dot_call`, so the parameter and return judgments use inference, and the C mismatches | `sema/call.rs:33-38` (`check_call` drops the type arguments on the `Member` route); `sema/call.rs:509` (`check_dot_call` has no type-argument parameter) |
| `DEF-406` | `outer<T>(take a: Wrap<String>, ...) { return a.pick(b, c); }` | rejected one `E10200` at the receiver/call, while the `outer<U>` twin accepts | the same unsubstituted dot-call return (`DEF-395`): the returned `T` is judged against `String` at `outer<i32>` |
| `DEF-407` | `for (v in spec.batchr.d407b.id<i32[2]>(a))` | accepted; emits `Cl#element of T v`; strict C rejects the `#` and the unknown element | `sema/call.rs:40-60` (qualified generic return unsubstituted); `own/loops.rs:530-537` / `sema/stmt.rs:211-228` (a `Param` iterable takes the element placeholder) |
| `DEF-402` | `let v: i32 = spec.batchr.d402.id<i32>(20);` | **green at the baseline**: emits `cl_id__i32(20)` and the strict build accepts; the collector gap no longer reproduces | `own/generics.rs` initializer walk (verification only) |

## Additional observations

- At `T = Box`, `h.v = s` (a view source) emits `ClString = ClStr` and strict C rejects it; no
  per-instance conversion is judged.
- At `T = Box`, `h.v = a.v` (an owned local source) emits the move `h.v = a.v; a.v = NULL;` and
  is ASan-clean; that is the concrete owned twin's behavior.
- A by-value `String` parameter stored into an opaque member is rejected `E10034` at the
  definition (the checker assumes an owned destination). That precision loss is not part of
  this batch; the filed cases use a local owner or a view.
- The `DEF-526` safe twin is exactly the batch-Q conservative rule's opposite side: a captured
  write of a parameter-backed view into a local. The batch-Q `capturedestination` case (a
  local-owner view) must keep rejecting.

## Test placement

| Rows | Suite |
|------|-------|
| `DEF-525`, `DEF-526`, `DEF-527` | `compiler/tests/eval_emission_integrity_spec_test.rs` |
| `DEF-528` | `compiler/tests/closure_integrity_spec_test.rs` |
| `DEF-395`, `DEF-401`, `DEF-406`, `DEF-407` | `compiler/tests/borrow_generic_integrity_spec_test.rs` |
| `DEF-396`, `DEF-402` | `compiler/tests/checker_conformance_spec_test.rs` |

The same suites' `_impl_test.rs` pairs receive the emission pins.
