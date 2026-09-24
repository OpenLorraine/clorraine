# Current State: Clorraine Amiga Graphics

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)
> **Analysis baseline**: worktree at 2026-09-24 14:37. This includes the
> uncommitted `spikes/05-graphics/` spike **and its three production edits**:
> the new `stdlib/std.amiga.graphics.clo`, the `compiler/src/stdlib.rs`
> registration, and the four graphics functions in
> `compiler/runtime/shim_hosted_amiga.c`. The plan reconciles those edits rather
> than starting from an empty tree (PF-001).

## Existing Implementation

### What Exists

| Artifact | State | Relevance |
|---|---|---|
| `spikes/05-graphics/` | Working spike | Proves the contract: a `Screen` handle, `setColor`/`fill`, and drawing on a real Kickstart under FS-UAE (`results/screen.png`) |
| `stdlib/std.amiga.graphics.clo` (spike, untracked) | Working single module | Owns `Screen` and `openScreen`, `setColor`, `fill`, and the `cl_*` ffi block; this plan relocates `Screen` and adds `draw`/`clear` (PF-001) |
| `compiler/src/stdlib.rs` (spike edit) | Working registration | `std.amiga.graphics` is already registered; this plan adds `std.amiga.intuition` and moves `Screen`/`openScreen` (PF-001) |
| `compiler/runtime/shim_hosted_amiga.c` (spike edit) | Working shim | Already defines `cl_open_screen`, `cl_set_color`, `cl_fill`, `cl_close_screen`; this plan adds `cl_draw`/`cl_clear` (PF-001) |
| `stdlib/std.amiga.dos.clo` | Implemented | The `delayTicks` primitive the demo uses |
| `stdlib/std.amiga.interrupt.clo` | Implemented | The VBlank dispatcher; the pattern for a stdlib module with an `ffi` block |
| `compiler/src/stdlib.rs` | Implemented | The embedded-module registry: `SOURCES` (bodies) and `MODULES` (export tables) |
| `compiler/src/profile.rs` | Implemented | `Profile::supports_module` gates `std.amiga.*` to hosted-amiga |
| `compiler/runtime/shim_hosted_amiga.c` | Implemented | The hosted-amiga C shim; already calls the exec interrupt server and DOS `Delay` |
| `spec/tests/corpus/interrupt-counter/` | Implemented | The only Amiga-only corpus program; the pattern the graphics corpus program reuses |
| `docker/scripts/corpus-fs-uae.sh` | Implemented | The sentinel-and-diff FS-UAE runner for the interrupt-counter |
| `spec/appendix-amiga.md` §1 | Spec | The `intuition`/`graphics` row reads "windows, drawing | later" |
| `spec/appendix-stdlib.md` §6–§8 | Spec | The Amiga module documentation sections |

### Relevant Files

| File | Purpose | Changes Needed |
|---|---|---|
| `stdlib/std.amiga.intuition.clo` | Screen lifecycle | Create (AR-1, AR-5) |
| `stdlib/std.amiga.graphics.clo` | Drawing calls | Rewrite the spike module: import `Screen`, add `draw`/`clear` (AR-1, AR-5; PF-001) |
| `compiler/src/stdlib.rs` | Module registry | Update the spike entry: add `std.amiga.intuition`, move `Screen`/`openScreen` out of `graphics` (AR-5; PF-001) |
| `compiler/runtime/shim_hosted_amiga.c` | C runtime | Reconcile: includes and four functions exist; add `cl_draw`/`cl_clear` (AR-9, AR-23, AR-24; PF-001) |
| `AGENTS.md` | Project routing | Add the new test files and stdlib/shim surface (PF-006) |
| `spec/appendix-amiga.md` | Platform appendix | Move the `intuition`/`graphics` row from "later" to v1; describe the surface (AR-27) |
| `spec/appendix-stdlib.md` | Stdlib appendix | Add the two module sections (AR-27) |
| `spec/tests/corpus/graphics/` | Graphics corpus | Create program, golden output, README (AR-10, AR-27, AR-28) |
| `compiler/tests/corpus_spec_test.rs` | Corpus harness | Classify `graphics` as Amiga-only (host rejection `E11200`) (AR-20) |
| `docker/scripts/corpus-fs-uae.sh` | FS-UAE leg | Add the graphics build/run/diff leg (AR-21) |
| `compiler/tests/graphics_spec_test.rs` | Spec tests | Create (AR-11, AR-22) |
| `compiler/tests/graphics_impl_test.rs` | Impl tests | Create (AR-11, AR-22) |
| `demo/story.sh`, `demo/tape/` | Story video | Add a graphics chapter (AR-28) |
| `plans/00-roadmap.md` | Roadmap | Add `PLAN-08` (AR-1) |

### Code Analysis

The machinery is already proven. The spike's module (`stdlib/std.amiga.graphics.clo`,
discussed in the register) declared `export handle Screen freedBy cl_close_screen;`
with an `ffi` block, and the shim opened the libraries and drove the RastPort.
The key existing patterns the plan reuses:

- **Module registration** — `SOURCES` + `MODULES` in `compiler/src/stdlib.rs:35` and `:95`; a stdlib test asserts every embedded source parses, names its table module, and that the shim defines every `ffi` symbol its modules bind (`compiler/src/stdlib.rs:397`).
- **`ffi` handle contract** — `stdlib/std.io.clo:19` (`handle File freedBy Close;`) and `:27` (the `ffi` block) show the exact shape the graphics modules use.
- **Opaque handle lowering** — a `handle` with no contract entry becomes the opaque `Cl<Name>` pointer typedef; the shim returns `void *` (`compiler/runtime/shim_hosted_amiga.c:179`).
- **Profile gating** — `Profile::supports_module` (`compiler/src/profile.rs:73`) returns false for `std.amiga.*` on host.
- **Amiga-only corpus** — `compiler/tests/corpus_spec_test.rs` treats `interrupt-counter` as the program the host profile must reject with `E11200`; `docker/scripts/corpus-fs-uae.sh` runs it with `RUN_CAPTURE=1`, waits for a sentinel, and diffs.

## Gaps Identified

### Gap 1: No screen lifecycle module

**Current Behavior:** The spike module fuses screen lifecycle and drawing into one provisional module.
**Required Behavior:** `std.amiga.intuition` owns the `Screen` handle and `openScreen`; the drop closes exactly once. (AR-5, AR-6, AR-17)
**Fix Required:** Create the module and register it.

### Gap 2: No drawing module

**Current Behavior:** The spike has only `setColor` and `fill`.
**Required Behavior:** `std.amiga.graphics` with `setColor`, `fill`, `draw`, and `clear`. (AR-7, AR-15)
**Fix Required:** Create the module and register it.

### Gap 3: No `draw`/`clear` shim, and the spike surface is provisional

**Current Behavior:** The shim has `cl_open_screen`, `cl_set_color`, `cl_fill`, `cl_close_screen` only.
**Required Behavior:** Add `cl_draw` and `cl_clear`; keep `-Wall -Wextra -Werror` clean. (AR-9, AR-24)
**Fix Required:** Extend `compiler/runtime/shim_hosted_amiga.c`.

### Gap 4: No graphics corpus program or CI leg

**Current Behavior:** Graphics exists only as an untested spike.
**Required Behavior:** A corpus program with a golden, host rejection `E11200`, and an FS-UAE leg. (AR-10, AR-20, AR-21)
**Fix Required:** Create the corpus program and extend the corpus and FS-UAE harnesses.

### Gap 5: The spec still says "later"

**Current Behavior:** `spec/appendix-amiga.md` §1 marks `intuition`/`graphics` as "later"; the stdlib appendix has no graphics sections.
**Required Behavior:** Both appendices document the v1 surface. (AR-3, AR-27)
**Fix Required:** Targeted appendix edits; `spec/check.sh` stays green.

## Dependencies

### Internal Dependencies

- `std.amiga.dos` for `delayTicks` (the demo's wait), already implemented.
- `compiler/src/stdlib.rs` registration and the profile gating, already implemented.
- The corpus and FS-UAE harness patterns, already implemented for `interrupt-counter`.

### External Dependencies

- The pinned SDK toolchain (`m68k-amigaos-gcc`, NDK 3.2 headers, `-lamiga`) — already the hosted-amiga build toolchain.
- FS-UAE and a real Kickstart in `docker/.env` for the emulator legs — already required by `make verify`.
- No new tool, crate, or build script.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `graphics.library`/`intuition.library` stubs missing from `-lamiga` | Low | High | The spike already linked and ran them under FS-UAE (`spikes/05-graphics/results/screen.png`) |
| A shim warning breaks every hosted-amiga build under `-Werror` | Medium | High | Build the corpus and demo programs in CI; the spike shim was warning-clean |
| `SetRGB4` index vs bitplane depth confusion | Low | Medium | `setColor` pen is a caller contract; the corpus uses pens within the screen depth (AR-14) |
| FS-UAE leg flakiness / no ROM | Medium | Medium | Reuse the existing sentinel-timeout pattern; the real-ROM policy is already in `docker/README.md` |
| Corpus harness change regresses `interrupt-counter` | Low | Medium | The graphics program is added alongside it; `make verify` covers both |
