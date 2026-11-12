# Ambiguity Register: Generated-C Robustness Bundle

> **Status**: ✅ GATE PASSED — all 12 items resolved
> **Last Updated**: 2026-11-12 14:35
> **Authority mode**: auto-design (root invocation
> `codeops-20261112-1415-generated-c-robustness`, policy version 1)

## Register

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| 1 | Behavioral | A module-level or imported value `std` makes `std.fmt.text(7)` a member path, but the emitted C spells `std.fmt.text` and fails | module path wins / value head wins / reject the spelling | **Module path wins** per MOD-13; only a **local** may shadow a module name (MOD-16). The intrinsic (or a real `std.fmt.text` declaration) fires | ✅ Resolved |
| 2 | Technical | Where the value-head gate lives | sema only / own only / both | Remove the module-level gate from both stages and from the initializer walk; keep the lexical shadow through `is_local`/`is_dot_call` | ✅ Resolved |
| 3 | Technical | The module initializer resolves `std.fmt.text` as a func-typed global before the text recognition | reorder / guard the callable resolver / leave | Compute `init_text_intrinsic_name` before `init_callable_value`; when it names the intrinsic, build the `$text` call directly and skip the callable/receiver | ✅ Resolved |
| 4 | Behavioral | An owned temporary argument to `text(...)` leaks in a function body | take / borrow-materialize / leave | Treat the intrinsic parameter as a borrow: materialize the owned temporary and release it at function scope. A named place stays a copy and remains readable. The initializer walk already releases its argument | ✅ Resolved |
| 5 | Technical | Where the body borrow lives | synthetic mode in `lower_call` / a new IR node / codegen cleanup | Synthetic `Mode::Borrow` for the intrinsic call; reuses the existing argument materialization and release path | ✅ Resolved |
| 6 | Behavioral | `__cl_itmp<n>` init locals can collide with a user `ffi` name | reserve the prefix / rename the user / leave | Reserve the generated prefix with the existing `is_taken_symbol` scan, exactly as `__cl_ctmpl` does | ✅ Resolved |
| 7 | Edge cases | A lexical `std` binding must keep the member-call behavior | keep / also make it the intrinsic | Keep: MOD-16 lets a local shadow a module name; `is_local`/`is_dot_call` already do this | ✅ Resolved |
| 8 | Edge cases | A user `std.fmt` module that declares `text` | declaration wins / intrinsic wins | The declaration wins; `table.function`/`qualified_decl_kind`/`qualified_ffi_signature`/`table.global` run before the intrinsic path | ✅ Resolved |
| 9 | Behavioral | #31 contradicts the #29 R2/IT-13 pin that a module-level value head is a member path | update the pin / keep the pin / leave the contradiction | **Update the pin**: the module path wins per MOD-13. Edit the #29 IT-13 and ST-15 comment, mark #29 findings §5 item 2 superseded, and add a task | ✅ Resolved |
| 10 | Scope | Whether #16 is part of this bundle | bundle / separate | Bundle: a small generated-C naming fix with an independent regression pin | ✅ Resolved |
| 11 | Non-functional | Verification and regression budget | full gate + 1% guard / correctness only | Full gate (`check`, `asan`, `ubsan`, `bench-shape`, `bench` + `bench-compare`, `verify`) against `0fdb96c`; every scored output equal and no scored metric above 1% | ✅ Resolved |
| 12 | Naming | Plan and suite names | — | `generated-c-robustness`; `generated_c_robustness_{spec,impl}_test.rs` | ✅ Resolved |

## Delegated Resolution Records

All decisions are eligible technical consequences of the frozen specification
and the confirmed bug reports #16, #30, #31. Root invocation
`codeops-20261112-1415-generated-c-robustness`.

**AR-1, AR-2, AR-7, AR-9 — qualified precedence.** Ch 10 MOD-13 says qualified
access `A.B.f()` is always available, with or without an import. MOD-16 says a
**local** may shadow an outer local or a module name; it does not give a
module-level or imported value that power. The current compiler follows this
for ordinary qualified calls (`is_dot_call` checks `module_prefix_len` before a
global head), but the `std.fmt.text` intrinsic gate used
`name_is_value("std")`, which includes module-level values. That made the
intrinsic decline and left the path to the member-call emission, which spelled
the literal `std`. Removing the module-level gate makes the module path win; a
lexical `std` still reaches the member path through `is_local`/`is_dot_call`.
This supersedes the #29 R2/IT-13 clause, which assumed a module-level value
head shadows the path; the #29 pin and comment are updated.

**AR-3 — initializer order.** `init_callable_value` resolves
`std.fmt.text` as a func-typed global field before the text recognition. The
initializer must decide the intrinsic first; otherwise a module-level `std`
still wins in the initializer even after the body is fixed. When
`init_text_intrinsic_name` returns `$text`, the call is built directly and the
callable/receiver path is skipped.

**AR-4, AR-5 — owned text argument.** `text` reads its argument and, for a
`String`, copies it. A named place must stay a copy; an owned temporary must
be released. The intrinsic call has no declared modes, so `lower_call` never
materializes the temporary. A synthetic `Mode::Borrow` lets the existing
`materialize_owned_argument` bind the temporary and register its function-scope
release, while a place-rooted value stays untouched. The initializer walk has
its own binding and already releases the argument (verified with ASan), so it
is unchanged.

**AR-6 — init-local naming.** `next_init_local` formats `__cl_itmp<n>` without
checking user symbols. `__cl_ctmpl` already advances its counter past
`is_taken_symbol`; the init-local name follows the same contract.

**AR-8, AR-10 to AR-12 — declarations, scope, verification, naming.** A real
`std.fmt.text` function/`ffi`/global wins; no spec/stdlib/ABI edit; #16 is a
small independent generated-C fix and joins the bundle; the full gate runs on
the pinned SDK image; FS-UAE legs use private Xvfb; the performance base is
`0fdb96c`.

## Reopen Triggers

- A module-level or imported `std` still hides the `std.fmt.text` module path,
  in a body or an initializer, or a lexical `std` stops shadowing it (AR-1–AR-3,
  AR-7).
- A named-place `text(s)` argument is moved or released by the call (AR-4).
- An owned body-level `text` temporary still leaks (AR-4).
- An init local still collides with a user `ffi` name (AR-6).
- The #29 IT-13 pin is not updated, or the #29 suite fails (AR-9).
- `make bench-compare` shows a scored regression above 1% (AR-11).
