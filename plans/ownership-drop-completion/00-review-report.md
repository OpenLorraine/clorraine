# Phase Review Report: Ownership and Drop Model Completion

> **Document**: 00-review-report.md
> **Parent**: [Index](00-index.md)
> **Profile**: strict defaults (no structured `codeops/codeops.json`): independent
> correctness review for every non-trivial executed phase, base lenses
> correctness + maintainability + standards.

## Phase 1 — Array classification, nullability, and element moves

**Reviewed revision**: `7426a11`
**Phase diff**: source changes in `compiler/src/types/mod.rs`,
`compiler/src/types/table.rs`, `compiler/src/types/tests.rs`,
`compiler/src/borrow.rs`, `compiler/src/own/expr.rs`,
`compiler/src/own/lower.rs`, `compiler/tests/ownership_drop_spec_test.rs`.

### Findings and rulings

| ID | Severity | Finding | Ruling |
|----|----------|---------|--------|
| RV-001 | 🟠 MAJOR | Routing `is_view`/`is_elided_view` through `is_slice` removed the fixed-array-return diagnostics (`E10035`/`E10036`), so a program the base revision rejected (`func pair(): String[2]`) is now accepted and emits invalid C. The by-value return lowering was deferred to Phase 2, leaving a window on `main`. | **Accepted — pull the fixed-array-return lowering forward** (user ruling). The `03-04` lowering lands now, as Step 1.4, with its specification tests first. |
| RV-002 | 🟡 MINOR | `is_fixed_array` is vacuously true for an empty extent list while `is_slice` is false; the asymmetry is untested. | Report-only. Addressed with the documented empty-extent assertion in the `types` unit test. |

| RV-006 | 🟠 MAJOR | (re-review) A `return inner();` where `inner` returns a fixed array was accepted and emitted `__cl_ret = inner();` — the destination was not passed and C arrays are not assignable. | **Accepted — fix now** (user ruling): the returned call writes into `__cl_ret` via `call_with_destination`. Fixed. |
| RV-007 | 🟠 MAJOR | (re-review) A discarded fixed-array-returning call `inner();` emitted no destination, failing to compile. | **Accepted — fix now** (user ruling): a statement-local destination is materialized and released. Fixed. |
| RV-008 | 🟡 MINOR | Nested fixed-array return is unlowered (one-dimension `extents.first()`); the plain nested-literal gap is scheduled for Phase 2 (03-05). | Report-only; the Phase 2 nested-literal emission covers the shared root cause. |
| RV-009 | 🟡 MINOR | The owning-array run oracle catches a double free but not a leak; the positive ownership-transfer claim is not observable. | Report-only. |

### Verification asks (confirmed by the reviewer)

| Ask | Result |
|-----|--------|
| The updated `types::table` unit test is an implementation test, not a spec oracle | ✅ `#[cfg(test)]` in `types/table.rs` |
| The slice-duplicability correction matches the specification | ✅ `spec/03-ownership.md:20-21`, AR-22 |
| New unit tests are not tautological | ✅ consumer-level and predicate-level coverage |
| Spec-test expectations derive from `spec/` | ✅ E10020/E10011/lend behavior |
| `make check` re-run independently | ✅ PASS |

### Review evidence

- Verify at review: `make check` — `spec check: OK (27 files, 103 compile-fail tests)`; 324 library unit tests pass; all integration suites pass.
- Re-review after the RV-001 fix: one scoped pass on the fix diff (Step 1.4), per the single-re-review budget.

## Phase 2 — Nested-array literals

**Reviewed revision**: `3095145`
**Phase diff**: `30988fc..3095145` — `compiler/src/codegen/function.rs`
(`emit_array_assign_to`), `compiler/tests/ownership_drop_spec_test.rs`,
`compiler/tests/ownership_drop_impl_test.rs`.

### Findings and rulings

| ID | Severity | Finding | Ruling |
|----|----------|---------|--------|
| RV-010 | 🟠 MAJOR | An array literal with an explicit prefix **and** a fill silently dropped the fill: `emit_array_assign_to` returned as soon as `elements` was non-empty, so indices `elements.len()..extent` were never written. The nested form (`u8[3][2] = [[1,2]; [7,8]]`) changed from a loud invalid-C failure before the phase into valid C with wrong values (exit `0`, not `8`); the 1-D form (`u8[5] = [1,2; 9]`) was a pre-existing silent defect masked by the `0` fill. | **Accepted — fix both forms now** (user ruling): the fill loop runs from `elements.len()` to the extent for the scalar and nested fills. Added the build-and-run case `a_prefix_and_fill_literal_fills_the_remaining_elements` (1-D and nested, exit `17`). |
| F4 | 🔵 OBSERVATION | A move-only nested fill (`String[3][2] = [; [a, b]]`) now compiles and aliases the moved temporaries (currently leaks; would double-free once Phase 3 adds element drop). | Report-only. AR-10 resolved to reject a move-only fill with `E10215`; Phase 4's guard reads `result_element` (the base element), so it covers the nested form. No action now. |
| F3 | 🔵 OBSERVATION | A multi-extent fixed-array return of a **place** copies one extent (`__cl_ret[i] = d[i];`, a C array assignment); a non-literal array element likewise emits array assignment. | Report-only; pre-existing and outside the nested-literal scope (the fixed-array-return component is one-dimension, RV-008). |

### Verification asks (confirmed by the reviewer)

| Ask | Result |
|-----|--------|
| The derived element type matches the flat multi-extent encoding | ✅ `Ty::array(element, extents[1..])`; no off-by-one |
| The recursion consumes one extent per level and terminates | ✅ |
| The 1-D path and the `__cl_ret` out-parameter path are unchanged | ✅ |
| The new tests fail on the pre-change code for the right reason | ✅ `cc` rejects the old `d[0] = { 1, 2 };` |
| `make check` re-run independently | ✅ PASS |

### Review evidence

- Verify at review: `make check` — `spec check: OK (27 files, 103 compile-fail tests)`; all suites pass.
- One scoped re-review follows on the RV-010 fix diff, per the single-re-review budget.

#### Re-review after the RV-010 fix (`3095145..65851e1`)

RV-010 is resolved for every enumerated form (1-D scalar, nested, empty prefix,
`prefix == extent`, `prefix > extent` rejection, 3-D, deeper) with no new
CRITICAL/MAJOR issue. The `prefix` bound is captured before the `if let` shadow
and both loops use it; no index is written twice or skipped.

| ID | Severity | Finding | Ruling |
|----|----------|---------|--------|
| NEW-1 | 🔵 OBSERVATION | When `prefix == extent` the fill loop is empty but `emit_guards(fill)` still runs, so a guard can trap for a fill value nothing copies. Pre-fix this path returned before the fill. | Report-only. Chapter 04 orders the fill to evaluate once, so emitting the guard is defensible; changing it could drop a required evaluation. |
| NEW-2 | 🔵 OBSERVATION | A non-literal nested fill with a prefix now reaches the scalar path (`d[1] = v;`) and is invalid C — the same pre-existing gap as F3, whose reach this fix widens from the empty-prefix form. | Report-only; whole fixed-array-element assignment is out of scope (AR-28). |

## Phase 3 — Array element drop glue, `set`, and un-stage `E90000`

**Reviewed revision**: `088a312`
**Phase diff**: `821b6a4..088a312` — `compiler/src/codegen/drop.rs`,
`compiler/src/sema/mod.rs`, `compiler/src/sema/tests.rs`,
`compiler/tests/ownership_drop_impl_test.rs`,
`compiler/tests/stage_spec_test.rs`, `spec/check.sh`,
`spec/tests/staged/`.

### Findings and rulings

| ID | Severity | Finding | Ruling |
|----|----------|---------|--------|
| RV-011 | 🟠 MAJOR | Removing `E90000` also admitted `Array<fixed-array>` (for example `Array<String[2]>`). The growable drop arm uses `c_type(element)`, which strips the extents, so the emitted pointer type and the index depth are wrong: the front end accepts the program and `cc` rejects the C (`dereferencing 'void *' pointer`) or the stride mis-addresses rows. | **Accepted — reject a fixed-array `Array` element** (user ruling). A narrow `check_sequence_element` reports `E90000` for that element only; `E90000` is now live again (AR-3 amended), removed from the exclusion list, and covered by `spec/tests/compile-fail/E90000_array_fixed_element.clo`. Fixed. |
| RV-012 | 🟡 MINOR | `set_releases_the_replaced_element_in_the_emitted_c` split on `cl_set__String`, whose first hit is the forward declaration, so the checked chunk also included the caller's scope-exit destructor — a false positive. | **Accepted — fix now**: split on `static void cl_set__String` to isolate the definition. Fixed. |
| F3 | 🔵 OBSERVATION | A slice of a move-only element (`Buffer[]` param, or `Array<Buffer>.get`) emits `ClSlice_*` typedefs before the element typedef, so the C is invalid. Pre-existing and also reachable at the baseline; the phase makes it reachable through `Array<move-only>`. | Report-only. `Array<Buffer>` acceptance (ST-8) and `Array<String>` `set` (ST-17) do not call `get`/`set` on a handle/struct element with a slice result, so the phase's cases are sound. A separate fix is needed for the slice typedef order. |
| F4 | 🔵 OBSERVATION | `03-02` still described changing `stdlib/std.mem.clo` for `set`, which the phase does not do now that PF-007 is treated as a false positive. | **Accepted — fix now**: the section now records the emitted-drop verification (AR-31). Fixed. |

### Verification asks (confirmed by the reviewer)

| Ask | Result |
|-----|--------|
| Fixed-array drop one extent per level, row-major, exactly once (2-D/3-D/deeper) | ✅ ASan clean |
| Growable drop: buffer read once, elements then buffer, idempotent guard, unique locals | ✅ ASan clean |
| Special `Array` arm and generic struct walk are mutually exclusive (no double free) | ✅ |
| `emits_drop` stays false for a fixed array | ✅ |
| `E90000` has no other live trigger and no stale code reference | ✅ (one intended trigger added by RV-011) |
| `make check` re-run independently | ✅ PASS |

### Review evidence

- Verify at review: `make check` — `spec check: OK`; all suites pass.
- The reviewer ran AddressSanitizer on `Array<String>`, `Array<Owned>`, `Array<Array<String>>`, `Array<Buffer>`, and nested fixed arrays: no leak and no double free.
- One scoped re-review follows on the RV-011 fix diff, per the single-re-review budget.

#### Re-review after the RV-011 fix (`088a312..f933b1b`)

The re-review confirmed the annotation cases and the `E90000` wiring, but found
RV-011 was not fully closed and two minor defects.

| ID | Severity | Finding | Ruling |
|----|----------|---------|--------|
| RV-011 (re-open) | 🟠 MAJOR | The guard was syntactic: it matched only an `Array<...>` annotation. `arrayNew<String[2]>()` and the inferred `let xs = arrayNew<String[2]>()` build `Array<fixed-array>` by substitution and were still accepted with invalid C. | **Accepted within the existing ruling** (reject fixed-array elements). The check now runs on the resolved `Array<...>` type as `Sema::reject_fixed_array_sequence`, called from `resolve_type` and from `check_generic_call`, so both the annotation and the generic-substitution paths are rejected. Verified: annotation, `arrayNew<String[2]>()`, and nested `arrayNew<Array<u8[2]>>()` report `E90000`; `Array<Buffer>`/`Array<String>`/`Array<Owned>`/`Array<Array<i32>>` stay accepted. |
| RV-012 (re-open) | 🟡 MINOR | `.split("static void cl_set__String").nth(1)` returned the code between the prototype and the definition, not the body. | **Accepted — fix now**: the definition is the last occurrence, so the test uses `.last()`. Fixed. |
| RV-013 | 🟡 MINOR | The syntactic check called `resolve_type` on the element, re-running element validation and duplicating element-level diagnostics. | **Accepted — fix now**: the resolved-type check inspects the already-resolved element, so element diagnostics run once. Fixed. |
| F4 | 🔵 OBSERVATION | The first diagnostic line names the element type in the `'{feature}'` slot; that matches the Chapter 14 template but reads as a type. | Report-only; the template fixes the wording. |

## Phase 4 — View-field and fill enforcement

> **Phase baseline**: `5a8dd91df8fa6c3724192b5be0695d2eea99c41f`
> **Review status**: ✅ Complete — one correctness review (strict profile, base lenses); no re-review required (no critical/major fix applied)

### Findings and rulings

| ID | Severity | Finding | Ruling |
|----|----------|---------|--------|
| RV-402 | 🟠 MAJOR | A generic fill instantiated with a move-only element double-frees: `func fillIt<T>(a: T): T[2] { return [; a]; }` called with `String` is accepted and emits two aliases of one handle. Reproduced independently: AddressSanitizer reports "attempting double-free". The guard deliberately skips a type-parameter element, so a substituted element is never checked. | **Deferred** (user ruling): recorded as `DEF-12` in the roadmap; a sound fix needs an instantiation-time check, a new mechanism, and is planned separately. |
| RV-401 | 🟡 MINOR | `Wrap<str>` / `Wrap<u8[]>` fields compile after substitution, bypassing the view-field ban; the same substitution blind spot as RV-402. | **Deferred** with RV-402 (`DEF-12`). |
| RV-403 | 🟡 MINOR | The codegen-run test's doc comment still described the rewritten case as a struct field. | **Accepted — fixed**: the comment now names the local view binding. |
| RV-404 | 🔵 OBSERVATION | `check_array_literal`'s doc omitted the duplicable-element rule. | **Accepted — fixed** for doc parity. |
| RV-405 | 🔵 OBSERVATION | The slice-extern test name says "uncontracted extern" although a slice-only signature is not uncontracted. | Report-only. |
| RV-406 | 🔵 OBSERVATION | The `fix603` regression asserted only stdout; a freed byte can still read `c=98` under the allocator's cache. | **Accepted — hardened**: the test now also asserts each owned temporary's release occurs after the read in the emitted C. |

### Verification asks (confirmed by the reviewer)

| Ask | Result |
|-----|--------|
| Slice view fields (direct, nullable) report `E10205`; `cstring`/`static str`/fixed arrays stay accepted | ✅ |
| Move-only fills report `E10215` (1-D, multi-D, prefix+fill); duplicable fills unchanged | ✅ for concrete elements; generic `T` deferred (RV-402) |
| The fixed-array `ffi` predicate reaches `E10100` and `E10101`; a slice stays accepted; no first-party breakage | ✅ (no stdlib/corpus fixed-array `ffi` parameter) |
| `fix603` preserves the temporary-lifetime regression | ✅ rewritten and hardened |
| `make check` re-run independently | ✅ PASS |

### Review evidence

- Verify at review: `make check` — `spec check: OK`; all 20 suites pass.
- Independent verification of RV-402 by the reviewing agent and the parent: ASan reports the double free.
- Base lenses only (correctness, maintainability, standards); no security or performance risk tags are active, so no auditor dispatch.
- No re-review required: the accepted fixes are minor documentation and test hardening; the major finding was deferred, not fixed.
