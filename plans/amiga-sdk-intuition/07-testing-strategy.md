# Testing Strategy: Amiga SDK Bundle 6 — Intuition UI

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)
> **Covers**: ST-1–ST-18, IT-1–IT-12

## Testing Overview

The host has no Intuition. The Rust specification suite therefore proves the
public surface compiles on both hosted profiles, the host rejects the Amiga-only
module, and the emitted C cross-builds with the 1.3 and 2.0 runtimes. The
interactive behavior runs on the real A500/KS1.3 machine through the GUI smoke.
Implementation tests pin the C shim, the emitted call shape, and the ownership
invariants.

The spec cases below are tagged **Host** (Rust suite), **A500** (GUI smoke), or
**Impl** (Rust implementation suite). The A500 cases are defined before
implementation in this document; their first smoke run is the red phase.

## Specification Test Cases

### Window and events

| # | Venue | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|---|
| ST-1 | Host | A program imports `Screen`, `openScreen`, `Window`, `openWindow`, `closeWindow`, `windowWidth`, `windowHeight`, `waitEvent`, `pollEvent`, `WindowEvent`, and every `EVENT_*` constant; opens a 320x200x4 screen and a window; draws with `windowFill`/`windowDraw`/`windowClear`/`windowText`; calls both event functions; closes the window | Compiles with zero diagnostics on `hosted-amiga` and `hosted-amiga-13`; emits C | R1–R3, R12 |
| ST-2 | Host | The same program is compiled for `host` | Exit 1 with `E11200`; no C emitted. This is a regression guard: the host already rejects `std.amiga.*` | R12 |
| ST-3 | Host | The full window/event surface is cross-built with `m68k-amigaos-gcc` under `nix13` and `nix20` | Both builds pass `-std=c99 -m68000 -fwrapv -Wall -Wextra -Werror -lamiga -lm` | R1–R3, R12 |
| ST-4 | A500 | The demo drains the queue with `pollEvent` until `EVENT_NONE`, then runs `waitEvent`; the smoke clicks the close gadget | The drain terminates; the close gadget produces `EVENT_CLOSE` and the loop exits | R3, AR-23 |
| ST-5 | Impl + A500 | The shim receives `IDCMP_REFRESHWINDOW`; the alert covers and restores the custom screen | The shim calls `BeginRefresh`/`EndRefresh` and replies the message; no refresh event reaches the caller; the loop continues after the alert | R3, AR-12 |

### Gadgets and menus

| # | Venue | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|---|
| ST-6 | Host | The full gadget/menu surface compiles on both hosted profiles and cross-builds under `nix13`/`nix20` | Zero diagnostics; both C builds pass with `-Werror` | R4–R6, R12 |
| ST-7 | A500 | The string gadget is activated, `abc` + Return is typed, and the smoke answers Yes with Left Amiga+V | `EVENT_GADGET` with `gadget == 2`; `windowString` returns 3; the requester opens; `req` becomes 1. The button is proven visually in the held frame; the string field proves the shared `GADGETUP` path | R4, AR-2, AR-25 |
| ST-8 | A500 | The string gadget starts empty; the smoke types `abc` and presses Return; `windowString` reads it into a 64-byte buffer | `EVENT_GADGET` with id 2; `windowString` returns 3 and the buffer contains `abc` | R5, AR-2 |
| ST-9 | A500 | Two menus are added; the smoke right-clicks Help → About and File → Hello | `EVENT_MENU` with the About id 20 and the Hello id 10; the status labels change | R6, AR-10 |
| ST-10 | A500 + Impl | An overlong label, an id 0, a string `maxChars` 0, a gadget outside the inner window, and an overlong menu title are attempted | Every call returns `false`, `-1`, or 0 as documented; no partial gadget appears; the demo prints `gui: err` if a check fails | R11, AR-14–AR-16 |

### Requesters, alerts, and pointer

| # | Venue | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|---|
| ST-11 | A500 | The string gadget's Return calls `askYesNo`; the smoke answers Yes with Left Amiga+V | Returns `true`; `req` becomes 1; the demo writes `DF0:gui-answered`; the loop continues | R7, AR-13, AR-25 |
| ST-12 | A500 | Help → About calls `askYesNo` via Right Amiga+A; the smoke answers Yes with Left Amiga+V | `req` becomes 2; the demo writes `DF0:gui-about`; the custom screen/window content is restored. `showAlert` is not driven interactively and stays covered by the host/spec test (AR-25) | R7, AR-13, AR-20, AR-25 |
| ST-13 | A500 | `setPointer(window, true)` is called and the held frame is captured at a fixed pointer position | The held frame shows the custom 16x16 arrow | R8, AR-7 |
| ST-14 | Host | The full requester/alert/pointer surface compiles on both hosted profiles and cross-builds under `nix13`/`nix20` | Zero diagnostics; both C builds pass with `-Werror` | R7–R8, R12 |

### A500 acceptance and project gates

| # | Venue | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|---|
| ST-15 | A500 | `make sdk-a500-gui` boots the demo and captures the native held frame | The `CSCR` capture converts to 1280x512 and matches `docs/amiga-sdk/a500-gui.png` | R9–R10 |
| ST-16 | A500 | The smoke types `abc` + Return, answers Yes, selects Hello with Right Amiga+H, selects About with Right Amiga+A, answers Yes, then quits with Right Amiga+Q | `DF0:gui.txt` reads `gui: ok win 1 gad 2 menu 2 str 3 req 2`; every handshake file is present | R9–R10, AR-25 |
| ST-17 | Verify | `make verify` runs after the new leg | The six earlier A500 gates (hello, Boing, sprites, paddle, text, assets) still pass | R10, R13 |
| ST-18 | Verify | `make check`, `make verify`, and `make sdk-registry` run at closeout | Exit 0; Bundle 6 is 6/9 done; SDK-INT-006/007/008 are deferred; coverage is regenerated | R13 |

## Implementation Test Cases

| # | Case | Expected |
|---|---|---|
| IT-1 | Window/event routing | Emitted C calls `cl_open_window`, `cl_close_window`, `cl_window_width/height`, `cl_window_next_event`, the seven event accessors, `cl_window_fill/draw/clear`, and `cl_window_text_draw`. The gadget, menu, requester, and pointer names are added to this case in 2.1.4 and 3.1.4 |
| IT-2 | Shim types and zeroing | The shim defines `cl_window`, `cl_gadget_node`, `cl_menu_node`, and `cl_menu_item_node`; every allocation uses `MEMF_CLEAR` or `memset`; every `Next*`/`ITextFont` starts null; prototypes use `void *` handles |
| IT-3 | Window flags | `NewWindow` uses `CUSTOMSCREEN`, `DetailPen = 1`, `BlockPen = 0`, the five `WFLG_*` flags, the nine IDCMP flags, and `WFLG_GIMMEZEROZERO` |
| IT-4 | Gadget structures | `AddGList`, `RefreshGList`, `GTYP_BOOLGADGET`, `GTYP_STRGADGET`, the `Image` pair with `Width`/`Height`/`Depth = 1`/`PlanePick = 0`, the string `Border` + `WORD[10]`, and `StringInfo.MaxChars = maxChars + 1` appear as specified |
| IT-5 | Menu structures and chain | `SetMenuStrip`, `ClearMenuStrip`, `ItemAddress`, `MENUENABLED`, `ITEMTEXT`, `ITEMENABLED`, `Menu.FirstItem`, and the rebuilt `Menu.NextMenu` chain appear; the strip is detached before the item/chain link; a false `SetMenuStrip` unlinks and reattaches; `MENUNULL` is rejected before `ItemAddress` |
| IT-6 | Event mapping | `BeginRefresh`/`EndRefresh`, `ReplyMsg`, the `IAddress` gadget lookup, the `ItemAddress` menu lookup, `GZZMouseX`/`GZZMouseY`, the `EVENT_*` table, and a zeroed `last_event` are present; an unknown event is replied and skipped |
| IT-7 | Requester/alert/pointer | The 1.3 eight-argument `AutoRequest`, `DisplayAlert`, `SetPointer`, `ClearPointer`, `MEMF_CHIP`, the 64-byte pointer, and the `FreeMem` on close are present |
| IT-8 | Free order, sizes, and drop order | Close clears the pointer, clears the menu strip, closes the window, then frees gadget/menu nodes, string buffers (`string_buffer_size`/`MaxChars`, even when `StringInfo` allocation failed), labels (65), `Image`/`Border`/`XY`, `StringInfo`, pointer (64), title (65), and wrapper exactly once. Locals close the window before the screen; a struct that holds both declares the window field first and closes it before the screen |
| IT-9 | Forbidden 2.0+ names | The Intuition shim contains no `OpenWindowTags`, `OpenWindowTagList`, `EasyRequest`, `CreateGadget`, `SetRGB32`, `GetVisualInfo`, ASL, or BOOPSI call |
| IT-10 | Limits | The shim bounds title/label 64, string 64, body 28, id >= 1 and unique, negative/oversized geometry, and inner-window bounds; rejected calls free partial allocations; `windowString` uses `NumChars`, never `strlen` |
| IT-11 | Demo shape | `examples/amiga/gui.clo` imports the full surface, binds the screen title, opens the font, contains the five negative checks, uses `pollEvent`, `windowWidth/Height`, the drawing calls, the close gadget, and the exact report-line format |
| IT-12 | Module table and scope | `compiler/src/stdlib.rs` exports the new names; `compiler/src/profile.rs` is unchanged; no file under `compiler/src/{lex,parse,ast,sema,own,ir,codegen,types,consts}` changes |

## Test Files

| File | ST/IT Cases | Notes |
|---|---|---|
| `compiler/tests/amiga_intuition_spec_test.rs` | ST-1, ST-2, ST-3, ST-6, ST-14 | Compile, host rejection (regression guard), cross-builds |
| `compiler/tests/amiga_intuition_impl_test.rs` | ST-5 (pin), ST-10 (pin), IT-1–IT-12 | Emitted C, shim source, limits, ownership, demo/make shape |
| `docker/scripts/a500-gui-smoke.sh` | ST-4, ST-5, ST-7–ST-13, ST-15, ST-16 | Real KS1.3 interaction, three goldens |
| `make verify` | ST-17 | Six earlier A500 gates |
| `make check`, `make verify`, `make sdk-registry` | ST-18 | Project gates |

## Red/Green Order

1. **Phase 1 red:** write ST-1–ST-3 (host window/event surface) while the
   exports are absent; capture `E11102`.
2. **Phase 1 green:** add the Window handle, drawing/text variants, and event
   loop; run ST-1–ST-3 green.
3. **Phase 2 red:** add ST-6 (host gadget/menu surface); capture `E11102`.
4. **Phase 2 green:** add buttons, string, and menus; run ST-6 green.
5. **Phase 3 red:** add ST-14 (host requester/alert/pointer surface); capture
   `E11102`.
6. **Phase 3 green:** add the three calls; run ST-14 green.
7. **Phase 4 target:** add the demo and smoke; the first A500 run is the red
   phase for ST-4, ST-5, ST-7–ST-13, ST-15, ST-16; capture the goldens and run
   them green.
8. **Closeout:** run ST-17 and ST-18 and update the registry, issue, programme,
   and roadmap.

ST-2 is a host-rejection regression guard: it already passes before the change
because the host profile rejects every `std.amiga.*` module. It is recorded as a
guard, not a red case.

## Verification Commands

| Command | Purpose |
|---|---|
| `cargo test --test amiga_intuition_spec_test` | Host surface, host rejection, cross-builds |
| `cargo test --test amiga_intuition_impl_test` | Shim, emitted C, ownership, limits, demo shape |
| `make sdk-a500-gui` | Real KS1.3 interaction, frame, alert, line |
| `make check` | Spec check, registry test, workspace tests |
| `make verify` | Full acceptance, including the seven A500 gates |
| `make sdk-registry` | Registry/coverage regeneration |

## Evidence to Keep

| Evidence | Path |
|---|---|
| Held GUI frame | `docs/amiga-sdk/a500-gui.png` |
| GUI report line | `DF0:gui.txt` in the ADF, asserted as `gui: ok win 1 gad 2 menu 2 str 3 req 2` |
| Handshake files | `DF0:gui-ready`, `gui-answered`, `gui-hello`, `gui-about`, `gui-quit` (transient ADF files) |
| Last failure screen | `build/amiga-sdk/gui-smoke/screen.png` (gitignored) |
| Spec/impl output | test stdout |
| A500 report line | `gui: ok win 1 gad 2 menu 2 str 3 req 2` |
