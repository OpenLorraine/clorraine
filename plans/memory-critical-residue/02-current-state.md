# Current State

Every row is reproduced at the planning baseline `6a67880` with the host profile, strict C
build, and `RDC_ASAN=1` (AddressSanitizer with leak detection). The observed result is the
sanitizer's report, the strict-build failure, or the unexpected acceptance. The `id(take h)`
identity return is the one shape batch P changed: it now reports `E10030` at the callee return
even though the caller's storage outlives the return.

| Row | Repro (filed) | Observed at `6a67880` | Site |
|-----|---------------|-----------------------|------|
| `DEF-508` | `store(make(a.v).names, borrow slot)` | accepted; ASan heap-use-after-free | `own/call_check.rs:652-801` (`frame_owned_view_owner` judges a whole call expression, not a member/index chain rooted at one); `own/place.rs:2422-2471` (`call_carrier_source`) |
| `DEF-509` | `makePair(x.v, y.v)` with two lend sources | accepted; ASan heap-use-after-free | `own/place.rs:2481-2515` (`single_lend_candidate` returns `None` for several candidates) |
| `DEF-510` | `((ys, j) => ys[j])(xs, 0)` and `(c ? first : second)(xs, 0)` | inline lambda accepted; ASan heap-use-after-free; conditional call accepted; strict C build fails (`(c ? first : second)(args)` is not C) | `own/place.rs:2554-2575` (`callable_call_signature` resolves no lambda or conditional); `own/call_check.rs:19-121` (no value-callee branch for either); `own/call.rs:296-367` (`lower_call` builds no callable receiver) |
| `DEF-511` | `([a, borrow slot] () => { slot = a.v; })()` | accepted; ASan heap-use-after-free | `own/place.rs:905-917` and `own/expr.rs:1836-1845` (a by-value capture is a `Param` with mode `None`, so it is treated as caller-backed) |
| `DEF-512` | `id(take h1)` and `makeHolder("AA", b)` where the result views `b` | the identity return is now falsely rejected `E10030`; the two-argument helper is accepted and UAFs | `own/place.rs:2481-2515` (`take` and non-candidate owned arguments are excluded); `own/place.rs:872-896` (`carrier_return_owners` treats a returned take parameter as frame-dying) |
| `DEF-514` | `f<T>(c, take h: T) { sink(take (c ? h.name : null)); }` at `T = H { name: String }` | accepted; ASan double-free; emitted C passes `(c ? h.name : NULL)` without the per-arm move-out | `own/lower.rs:5546-5551` and `:2758-2774` (`conditional_owns_place_arms` requires every arm, so a `null` arm blocks the per-arm carrier) |
| `DEF-515` | `sink(take (unsafe (c ? h.name : null)))` | accepted; ASan double-free | `own/expr.rs:168-190` (`unresolved_conditional_place_arms` matches only a top-level conditional); the lowering predicate as `DEF-514` |
| `DEF-518` | `f<T>(c, h: T) { let arr: String[1] = [c ? h.name : h.name]; }` at `T = H { name: String }` | accepted; ASan double-free | `own/expr.rs:776-818` (the array-literal element path records no type-parameter conditional duty) |
| `DEF-519` | `return S { h: c ? makeFrom(a) : Holder { ... } }` | accepted; ASan heap-use-after-free | `own/call_check.rs:652-801` (no struct/array literal descent); `own/statements.rs:1790-1813` (the return adds only a whole-expression frame owner) |
| `DEF-521` | `S { name: unsafe (c ? h.name : null) }` at `T = H { name: String }` | accepted; ASan double-free | `own/expr.rs:128-154` (`type_param_conditional_sources` matches only a top-level conditional) |
| `DEF-523` | `let n: Inner = h.name; h.name.x = 7; ... h.name.s ...` (generic and concrete) | accepted; ASan SEGV / double free | `own/place.rs:152-263` (`write_place` clears every moved path sharing the first projection and never judges a moved prefix) |

## Test placement

The batch reuses the suites that already own the neighboring shapes:

| Rows | Suite |
|------|-------|
| `DEF-508`, `509`, `510`, `511`, `512`, `519` | `compiler/tests/eval_emission_integrity_spec_test.rs` |
| `DEF-514`, `515`, `518`, `521`, `523` | `compiler/tests/borrow_generic_integrity_spec_test.rs` |

The same suites' `_impl_test.rs` pairs receive the emission pins.
