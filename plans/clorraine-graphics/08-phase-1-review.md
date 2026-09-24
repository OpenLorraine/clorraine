# Phase 1 Review: Clorraine Amiga Graphics

> **Document**: 08-phase-1-review.md
> **Parent**: [Index](00-index.md)
> **Dispatch**: phase-reviewer (correctness, semantics, concurrency) · scope mode: strict
> **Baseline tree**: 8e4723e51b70b00e3379d0801b3cbbe7dc3fd865
> **Reviewed revision**: aa0b58e
> **Verification evidence**: `make check` green (15/15 test binaries); `make verify` all legs passed
> **Date**: 2026-09-24 15:01

## Summary

Independent review of the Phase 1 diff (9 files: the two stdlib modules, the
module registry, the hosted-amiga shim, the two graphics test files, the corpus
script scoping fix, and the plan/register updates). No critical or major
findings. Six minor findings, all recorded below; minor findings are
report-only under the strict profile.

## Findings

| # | Severity | Finding | Location | Evidence | Recommended fix |
|---|----------|---------|----------|----------|-----------------|
| RV-001 | 🟡 Minor | Importing `std.amiga.graphics` without `std.amiga.intuition` leaves the transitive `Screen` import unresolved: the frozen module emits four `W10001` ("unnecessary lend of duplicable 'screen'") warnings and the emitted C omits `typedef void *ClScreen`. Ownership therefore differs by import path. The realistic API path always imports both (only `intuition` makes a `Screen`), which is why the ST-2 probe passed. | `compiler/src/driver.rs:121` (`inject_stdlib` is single-pass); warnings at `stdlib/std.amiga.graphics.clo:28,36,44,49` | `./clc --profile hosted-amiga` on a graphics-only import prints `W10001`; the both-imports programs are clean | Apply the AR-35 fallback: make `inject_stdlib` transitive (loop to a fixpoint over injected modules' own imports) and add a spec case importing only `std.amiga.graphics` |
| RV-002 | 🟡 Minor | `generated_c_contains_no_unsafe_construct` is vacuous: the compiler never emits the token `unsafe` in any C, so the assertion cannot fail. | `compiler/tests/graphics_impl_test.rs` (`generated_c_contains_no_unsafe_construct`) | `grep -rn unsafe compiler/src` shows no codegen emission | Assert the real contract instead (wrapper bodies call the shims; the app calls only `cl_std_amiga_*` wrappers), or drop the test |
| RV-003 | 🟡 Minor | The "six shims" and drop assertions can pass on prototypes/boilerplate rather than the real call/cleanup sites: the symbols also appear in the ffi prototypes, and the drop guard is emitted before the first assignment as well as at cleanup. | `compiler/tests/graphics_impl_test.rs` (`generated_c_uses_the_six_shims_through_the_wrappers`, `the_screen_drop_is_null_safe`) | Emitted C contains the prototype declarations and the init-time guard | Anchor on call sites (e.g. `cl_set_color(screen,`) and the cleanup-label guard |
| RV-004 | 🟡 Minor | `setColor` documents `pen` as "0-31" unconditionally, but the valid range is `0..(1<<depth)-1`; a pen outside the ColorMap is undefined. | `stdlib/std.amiga.graphics.clo:23`; shim `compiler/runtime/shim_hosted_amiga.c:615` | `openScreen` takes `depth` (1-5); `SetRGB4` takes the index passed through | State the depth-dependent range (0-15 for depth 4, 0-31 for depth 5) |
| RV-005 | 🟡 Minor | The scoped vbcc scan matches only line-located C-frontend warnings; a `warning` line that names `program.c` without `in line N of "…"` (driver/assembler diagnostics) would be ignored. | `docker/scripts/corpus-amiga.sh:81` | The vbcc log uses the located format for the observed warnings | Also fail on non-header `warning` lines that name `program.c` |
| RV-006 | 🟡 Minor | The `title` lifetime contract is documentation-only and untested; an inline temporary (`openScreen(..., toCString(stringOf("x")))`) leaves `Screen.DefaultTitle` dangling. The test programs are safe only because `title` is bound before `screen`. | `stdlib/std.amiga.intuition.clo:24-35`; shim `compiler/runtime/shim_hosted_amiga.c:601-604` | Intuition stores the pointer; the contract is stated but not enforced | Document the gap explicitly in the user-facing appendix; a `borrow`/owned title parameter is the only real enforcement (deferred by PF-004) |

## Checked clean

- `cl_clear` bounds: `(LONG)screen->Width - 1` sign-extends a `WORD` to 32-bit
  with no narrowing or undefined behavior.
- `Screen`/`RastPort` are null-checked before use; the drop closes a non-null
  screen once and is null-safe.
- `GfxBase`/`IntuitionBase` are correctly typed for the NDK proto macros and
  opened before any call.
- No concurrency surface exists (no task/thread module), so the non-atomic lazy
  base initialization is unreachable.

## Rulings

The user approved fixing RV-001 through RV-005; all were applied and verified
(`make check` 15/15; `make verify` all legs, vbcc warning-free).

| # | Ruling | Action |
|---|--------|--------|
| RV-001 | Fixed | `inject_stdlib` now runs to a fixpoint over injected modules' own imports (`compiler/src/driver.rs`); an implementation test compiles a graphics-only import and asserts the `ClScreen` handle typedef is emitted. An unused import is unavoidable in the single-module case, so the regression lives as an implementation test rather than a diagnostics-free spec case. |
| RV-002 | Fixed | The vacuous `no unsafe` assertion was replaced with call-site assertions that a prototype cannot satisfy. |
| RV-003 | Fixed | The shim assertions now anchor on call sites (`cl_set_color(screen,`) and the drop test requires every `cl_close_screen(screen)` call to sit inside the `if (screen)` guard. |
| RV-004 | Fixed | `setColor` documentation now states the depth-dependent pen range (`0..(1 << depth) - 1`). |
| RV-005 | Fixed | The vbcc scan now fails on any `warning` line naming the emitted `program.c`, located or not. |
| RV-006 | Deferred | Documentation-only; enforcement of the title lifetime is deferred by the plan's PF-004 decision. The contract stays documented in the module and the shim. |
