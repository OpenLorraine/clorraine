# Ambiguity Register: Clorraine Amiga graphics

> **Status**: ✅ GATE PASSED — 36 items: 30 resolved, 6 explicitly deferred; AR-36 was opened and resolved during execution
> **Last Updated**: 2026-09-24 15:01
> **CodeOps Artifact Schema**: 1

Every row below was raised during the Phase 1 interview and resolved by the
user, or explicitly deferred. The plan adds a minimal graphics surface to the
hosted-amiga standard library: an owned custom screen and filled/line drawing.
It edits the frozen specification only where the user authorized it (AR-3).

> **Items added during execution** are tagged `(runtime)`.

## Register

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| AR-1 | Scope | Planning form and tracking | Full feature plan + roadmap row / lightweight mini-plan | Full feature plan at `plans/clorraine-graphics/` with a new `PLAN-08` roadmap row | ✅ Resolved |
| AR-2 | Scope | What the feature covers | Screen+draw / window-only / both; primitive sets; modules | Custom-screen lifecycle plus drawing: `setColor`, `fill`, `draw` (line), `clear`, and screen open/close | ✅ Resolved |
| AR-3 | Scope | Which frozen-spec artifacts may change | Targeted edits / appendices only / no spec change | Targeted `spec/` edits authorized: `spec/appendix-amiga.md`, `spec/appendix-stdlib.md`, and a new corpus program under `spec/tests/corpus/graphics/`; `spec/check.sh` must stay green | ✅ Resolved |
| AR-4 | Scope | v1 boundary (deliberately excluded) | Broad surface / bounded surface | Out of scope: windows and event handling, text/fonts, bitmaps/blit, sprites/copper/custom chips, `SetRGB32`, multiple simultaneous screens | ✅ Resolved |
| AR-5 | Naming | Module names | `std.amiga.intuition`+`std.amiga.graphics` / `screen`+`draw` | `std.amiga.intuition` (screen lifecycle) and `std.amiga.graphics` (drawing) | ✅ Resolved |
| AR-6 | Naming | Screen type and its contract | Owned `Screen` handle / exposed `RastPort` | `export handle Screen freedBy cl_close_screen;` — the screen is owned; the `RastPort` is never exposed | ✅ Resolved |
| AR-7 | Naming | Drawing function names | `setColor`/`fill`/`draw`/`clear` / `rectFill`/`line`/`clearScreen` | `setColor`, `fill`, `draw`, `clear` | ✅ Resolved |
| AR-8 | Naming | `openScreen` shape | Positional / `ScreenSpec` struct | `openScreen(width: i32, height: i32, depth: u8, title: cstring): Screen?` | ✅ Resolved |
| AR-9 | Naming | Shim (ffi) symbol names | `cl_*` snake case / variants | `cl_open_screen`, `cl_close_screen`, `cl_set_color`, `cl_fill`, `cl_draw`, `cl_clear` | ✅ Resolved |
| AR-10 | Naming | Corpus program location | `spec/tests/corpus/graphics/` / `examples/` | `spec/tests/corpus/graphics/program.clo` with `expected-output.txt` and `README.md` | ✅ Resolved |
| AR-11 | Naming | Test file names | New files / extend existing suites | New `compiler/tests/graphics_spec_test.rs` (behavior) and `compiler/tests/graphics_impl_test.rs` (internals) | ✅ Resolved |
| AR-12 | Behavioral | Coordinate, pen, and color types | `i32`/`u8` / `i16` coordinates | Coordinates `i32`; pen index `u8`; `setColor` components `u8` (0-255) mapped internally to the Amiga 4-bit palette | ✅ Resolved |
| AR-13 | Behavioral | `draw` semantics | Arbitrary line / horizontal-vertical only | Arbitrary line between two points (the `graphics.library` `Draw` primitive) | ✅ Resolved |
| AR-14 | Behavioral | Out-of-range pen handling | Check and fault / caller contract | `pen` and `setColor` indices are a caller contract; the shim passes them through and out-of-range indices are undefined by this wrapper (clipping is for coordinates, not pens — PF-003). No compiler check is added | ✅ Resolved |
| AR-15 | Behavioral | `clear` semantics | Full screen / region | `clear(screen, pen)` fills the whole screen rectangle with `pen` | ✅ Resolved |
| AR-16 | Behavioral | Screen-open failure | `null` / status struct | `openScreen` returns `null` on failure; the caller must null-check (existing nullable rules, E10010) | ✅ Resolved |
| AR-17 | Behavioral | Screen release | Owned drop / manual close | The generated drop calls `cl_close_screen` exactly once; the deleter is null-safe | ✅ Resolved |
| AR-18 | Data model | Screen handle representation | Opaque pointer / integer | An opaque pointer handle emitted as `ClScreen` (`void *`); drawing takes `borrow screen` | ✅ Resolved |
| AR-19 | Verification | The verify command | `make check` then `make verify` / `make check` only | Every task runs `make check`; each phase and the final task run `make verify` (the commands come from `AGENTS.md`) | ✅ Resolved |
| AR-20 | Verification | Host-profile behavior | Reject with E11200 / skip | The graphics program is Amiga-only: under `--profile host` it is rejected with the profile-surface diagnostic `E11200`, exactly like `interrupt-counter` | ✅ Resolved |
| AR-21 | Verification | FS-UAE leg | Extend `corpus-fs-uae.sh` / a new target | Extend `docker/scripts/corpus-fs-uae.sh` to build and run the graphics program with `RUN_CAPTURE=1`, wait for the `graphics: ok` sentinel, and diff against `expected-output.txt` | ✅ Resolved |
| AR-22 | Verification | Spec-test surface | New files / extend | `graphics_spec_test.rs` carries the ST-cases (host rejection, hosted-amiga compile, sentinel output); `graphics_impl_test.rs` covers the shim/table internals | ✅ Resolved |
| AR-23 | Technical | Library opening | Open per call / open once at first use | The shim opens `graphics.library` and `intuition.library` once, on the first screen open, and holds the bases at file scope | ✅ Resolved |
| AR-24 | Technical | Shim C surface and warnings | New includes / reuse | Add `<proto/graphics.h>`, `<proto/intuition.h>`, `<intuition/screens.h>`, `<graphics/rastport.h>`; the shim must stay clean under `-Wall -Wextra -Werror` | ✅ Resolved |
| AR-25 | Technical | Profile gating | New rule / existing rule | The existing `Profile::supports_module` already rejects `std.amiga.*` on host and freestanding; no change is needed | ✅ Resolved |
| AR-26 | Technical | Complexity escalation | None / new harness | No material support surface is introduced. The rejected larger mechanisms: a screenshot-golden diff harness (AR-34) and a `ScreenSpec` struct (AR-8). The Complexity Escalation Gate is not triggered | ✅ Resolved |
| AR-35 | Technical | `std.amiga.graphics` must import `Screen` from `std.amiga.intuition`, and no embedded stdlib module imports another today; `inject_stdlib` injects in one pass | Verify first / transitive-injection fallback / merge modules / require both imports | Verify first: the program imports both modules (the API forces it, since only `intuition` makes a `Screen`). If the load stage does not resolve an embedded module's own import, make `inject_stdlib` transitive — a small compiler change, pre-approved as part of this plan's modification set. Merging the modules is the rejected smaller fallback | ✅ Resolved |
| AR-36 | Technical (runtime) | The `vbcc` build-only leg (`make verify`) now fails: `<proto/intuition.h>` pulls in `devices/timer.h`, where vbcc warning 53 ("struct/union member needs identifier") fires on a system header. GCC `-Wall -Wextra -Werror` is clean; only the leg's crude `grep -qi warning` scan trips. Introduced by the committed spike's includes | (A) Scope the vbcc warning scan to the generated translation unit (`program.c`) / (B) Drop `<proto/intuition.h>` and call `OpenScreen`/`CloseScreen` through manual library vectors / (C) Suppress vbcc warning 53 in the shim around the include / (D) Leave `make verify` failing | (A) Scope the vbcc warning scan to warnings that reference the generated `program.c`; NDK system-header warnings are outside the leg's contract. The scan still fails on any warning in the emitted C | ✅ Resolved |
| AR-27 | Documentation | Spec and corpus docs | Listed set / variants | Update the `intuition`/`graphics` row in `spec/appendix-amiga.md` from "later" to v1 with the wrapped surface, add the module sections to `spec/appendix-stdlib.md`, and add `spec/tests/corpus/graphics/README.md` | ✅ Resolved |
| AR-28 | Documentation | Video integration | Add a chapter / plan only | Extend `demo/story.sh` with the graphics program and an emulator capture as a new chapter | ✅ Resolved |
| AR-29 | Deferred | Intuition **windows** and the event loop | Include now / defer | Deferred to a later plan. Not implemented by this plan | ⏸ Deferred |
| AR-30 | Deferred | Text and fonts (`Move`/`Text`, `TextAttr`) | Include now / defer | Deferred. Not implemented by this plan | ⏸ Deferred |
| AR-31 | Deferred | Bitmaps, off-screen images, and blit | Include now / defer | Deferred. Not implemented by this plan | ⏸ Deferred |
| AR-32 | Deferred | Sprites, copper, and custom-chip display setup | Include now / defer | Deferred; overlaps `PLAN-04`'s `std.amiga.custom`. Not implemented by this plan | ⏸ Deferred |
| AR-33 | Deferred | `SetRGB32` and palette beyond 4 bits | Include now / defer | Deferred. Not implemented by this plan | ⏸ Deferred |
| AR-34 | Deferred | Screenshot-golden diff harness | Include now / defer | Deferred (a material test-infrastructure surface). The screen is captured for the video only | ⏸ Deferred |

## Resolution Notes

**AR-2 / AR-4:** The surface is deliberately the smallest that makes a real
animated demo: open a custom screen, set colors, fill and line-draw, clear, and
let the screen drop. Windows, text, bitmaps, and chipset work are named as
out-of-scope so the plan stays reviewable.

**AR-6 / AR-18:** The spike validated this exact contract: `export handle Screen
freedBy cl_close_screen;` with `borrow screen` drawing calls, an opaque-point
handle lowering to `ClScreen`, and a null-safe drop. The `RastPort` stays inside
the shim.

**AR-13:** `draw` is the `graphics.library` `Draw` primitive, which draws an
arbitrary line between two points; a horizontal/vertical-only restriction would
be a new rule with no precedent, so the primitive's own meaning is used.

**AR-20 / AR-21:** The graphics program joins `interrupt-counter` as the second
Amiga-only corpus program. The existing corpus test already special-cases that
program (host rejection with `E11200`), and `corpus-fs-uae.sh` already uses the
sentinel-and-diff pattern; the graphics leg extends that script rather than
adding a new harness (AR-26).

**AR-26:** The reviewed larger mechanisms were a `ScreenSpec` struct (AR-8) and a
screenshot-golden harness (AR-34); both were rejected in favour of the smaller
path, so no explicit complexity-escalation approval is required.

**AR-35:** The module split (AR-5) requires a cross-module import, which the
embedded standard library has not needed before. The plan verifies the current
one-pass injection with a compile check first; the approved fallback is a small
`inject_stdlib` change to follow an injected module's own imports. That compiler
change is part of this plan's modification set only if the compile check fails.
The probe passed, so no `inject_stdlib` change was made.

**AR-36 (runtime):** The added graphics includes make every hosted-amiga
translation unit pull `<proto/intuition.h>`, whose `devices/timer.h` fires vbcc
warning 53 on a system struct. The leg's warning scan now keys on warnings that
reference the generated `program.c`, so a real warning in the emitted C still
fails the leg while third-party header noise does not.

## Gate Record

- Systematic scan across all 12 categories completed.
- 35 rows: 29 resolved by an explicit user decision or an agreed improvement;
  6 explicitly deferred (AR-29–AR-34) and excluded from executable plan content.
- No item introduces a material support surface beyond ordinary stdlib, shim,
  spec-golden, and script work.
- The user confirmed the complete register and preflight iteration 2 passed, so
  the header reads `✅ GATE PASSED`.
