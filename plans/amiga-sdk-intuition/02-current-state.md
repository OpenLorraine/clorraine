# Current State: Amiga SDK Bundle 6 — Intuition UI

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

| Area | Current state | Gap for Bundle 6 |
|---|---|---|
| `std.amiga.intuition` | `Screen` handle and `openScreen(width, height, depth, title)`; the generated drop calls `cl_close_screen` (`stdlib/std.amiga.intuition.clo:14-36`) | No window, gadget, menu, event, requester, or pointer surface |
| `std.amiga.graphics` | `setColor`, `fill`, `draw`, `clear`, all taking `borrow screen: Screen` (`stdlib/std.amiga.graphics.clo:15-51`) | No window-relative drawing |
| `std.amiga.text` | `Font`, `openFont`, `openDiskFont`, `defaultFont`, `freeFont`, `textWidth`, `textHeight`, `drawText(screen, ...)` (`stdlib/std.amiga.text.clo:17-78`) | No window-relative text |
| C shim screen block | `cl_open_screen`, `cl_set_color`, `cl_fill`, `cl_draw`, `cl_clear`, `cl_close_screen` (`compiler/runtime/shim_hosted_amiga.c:1050-1150`) | No window, gadget, menu, event, requester, or pointer helpers |
| C shim text block | `cl_font_*`, `cl_text_width`, `cl_text_height`, `cl_text_draw` on a `Screen` RastPort (`compiler/runtime/shim_hosted_amiga.c:3309-3458`) | No window RastPort text helper |
| `std.amiga.exec` | Message ports, messages, and signals; gated off on 1.3 until Bundle 7 (`compiler/src/profile.rs:90-112`; `spec/appendix-amiga.md:16-18`) | Bundle 6 must not depend on it |
| `std.amiga.layers` | Clipped layers over a custom display bitmap, not an Intuition screen (`stdlib/std.amiga.layers.clo:1-7`) | Not used by the window path |
| Native video | `Display`, `Bitmap`, copper, sprites, and the whole custom-display path (`stdlib/std.amiga.video.clo`) | Not used by the window path |
| Demos | hello, Boing, sprites, paddle, scroller, image viewer | No GUI demo |
| Registry | Bundle 6 has nine rows, all `missing` (`docs/amiga-sdk/registry.tsv:49-57`) | No plan, implementation, tests, or acceptance |

### Relevant Files

| File | Purpose | Changes Needed |
|---|---|---|
| `stdlib/std.amiga.intuition.clo` | Screen lifecycle | Add `Window`, gadgets, menus, events, requester, alert, pointer |
| `stdlib/std.amiga.graphics.clo` | Screen drawing | Add window drawing functions |
| `stdlib/std.amiga.text.clo` | Screen text | Add window text |
| `compiler/src/stdlib.rs` | Module registration and export table (`:101-108`, `:395-400`) | Extend the intuition, graphics, and text export lists |
| `compiler/src/profile.rs` | Module gating (`:100-108`) | No change; Intuition is already on both hosted profiles |
| `compiler/runtime/shim_hosted_amiga.c` | Runtime shim | Add the window/gadget/menu/event/requester/pointer C block and window drawing/text |
| `compiler/tests/amiga_intuition_spec_test.rs` | New specification suite | Full surface, host rejection, 1.3/2.0 cross-builds |
| `compiler/tests/amiga_intuition_impl_test.rs` | New implementation suite | Prototypes, calls, ownership, limits, forbidden 2.0+ names |
| `examples/amiga/gui.clo` | New GUI demo | Window, gadgets, menus, requester, alert, pointer, line |
| `docker/scripts/a500-gui-smoke.sh` | New smoke | Boot, interact, capture frame and line |
| `Makefile` | Build/verify targets | Add `sdk-a500-gui` and the `verify` leg |
| `spec/appendix-amiga.md` | Platform appendix | Replace the “no windows” scope note |
| `spec/appendix-stdlib.md` | Stdlib appendix | Document the new API |
| `docs/amiga-sdk/*`, `plans/amiga-sdk/00-index.md`, `plans/00-roadmap.md` | Programme status | Mark Bundle 6 done, 6/9, issue #7 closed |

### Code Analysis

The current Intuition surface is deliberately minimal. `openScreen` returns the
raw `struct Screen *` and `cl_close_screen` is null-safe
(`compiler/runtime/shim_hosted_amiga.c:1140-1150`). The drawing helpers cast the
handle back to `struct Screen *` and drive `screen->RastPort`. The text helper
temporarily sets the font on the screen RastPort and restores the old font
(`compiler/runtime/shim_hosted_amiga.c:3440-3458`). Bundle 6 follows the same
pattern but needs a private wrapper for each window, because a window owns more
than its `struct Window *`: gadget nodes, menu nodes, copied labels, string
buffers, and the pointer allocation.

The 1.3 NDK in the pinned image provides the required classic structures and
vectors. The 1.3 SFD
(`/opt/amiga/m68k-amigaos/ndk/lib/sfd13/intuition_lib.sfd`) lists
`OpenWindow`, `CloseWindow`, `AddGList`, `RefreshGList`, `SetMenuStrip`,
`ClearMenuStrip`, `ItemAddress`, `AutoRequest`, `DisplayAlert`, `SetPointer`,
`ClearPointer`, `BeginRefresh`, and `EndRefresh`. The 1.3 header defines
`GTYP_BOOLGADGET`, `GTYP_STRGADGET`, `StringInfo`, `Window.UserData`,
`WFLG_*`, and `IDCMP_*`. No 2.0+ call is needed.

## Gaps Identified

### Gap 1: No window handle

**Current Behavior:** Only a custom screen can be opened; there is no window,
window-relative drawing, or window text.

**Required Behavior:** `openWindow` returns an owned `Window?` on a custom
screen; `windowWidth`/`windowHeight` report the inner area; `closeWindow` and
the drop free the window and its owned allocations.

**Fix Required:** Add a `cl_window` wrapper and the window shim block, plus
`windowFill`/`windowDraw`/`windowClear`/`windowText`.

### Gap 2: No gadgets or menus

**Current Behavior:** No classic gadget or menu structures are created.

**Required Behavior:** `addButton` and `addString` append classic gadgets to an
open window; `addMenu`/`addMenuItem` build and attach a menu strip; events carry
the caller's ids.

**Fix Required:** Add shim-owned gadget and menu nodes, `AddGList`/`RefreshGList`
and `SetMenuStrip`/`ClearMenuStrip` calls, bounded label copies, and the
`StringInfo` buffer protocol.

### Gap 3: No event loop

**Current Behavior:** `std.amiga.exec` has ports and signals but is gated off on
1.3; there is no IDCMP surface.

**Required Behavior:** `waitEvent`/`pollEvent` return a `WindowEvent` value with
the documented kinds and payload; IntuiMessages are replied; refresh is
auto-acknowledged.

**Fix Required:** Add the IDCMP port handling to the window shim and map
`IntuiMessage` fields and addresses to the public event struct.

### Gap 4: No requesters, alerts, or pointer

**Current Behavior:** None of these exist.

**Required Behavior:** `askYesNo`, `showAlert`, and `setPointer` provide the
minimal 1.3 surface, with copied text and chip-memory pointer storage.

**Fix Required:** Add `AutoRequest`, `DisplayAlert`, `SetPointer`, and
`ClearPointer` wrappers with bounded text copies and free-on-close.

## Dependencies

### Internal Dependencies

- `std.amiga.intuition` (screen and window ownership) is the base for graphics
  and text window variants.
- `std.amiga.graphics` and `std.amiga.text` import `Window` from Intuition; the
  dependency stays one-way.
- The A500 smoke reuses the Bundle 0–5 FS-UAE harness, ROM decryption, ADF
  layout, and screenshot comparison.
- The GUI demo uses the existing ROM Topaz font and `std.text` string helpers.

### External Dependencies

- Kickstart 1.3 Intuition vectors and structures in the pinned SDK image.
- FS-UAE and the licensed Amiga Forever ROM/`rom.key` for the A500 gate.
- `xdotool`, ImageMagick, and the existing X display for interaction and
  capture.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A text-only boolean gadget may not draw its box on 1.3 | Medium | Medium | Use a classic `Image` pair for the button and a `Border` for the string field; verify the first A500 frame |
| `xdotool` coordinates depend on the FS-UAE window geometry | Medium | Medium | Calibrate once on the first run, pin the offsets, and keep the last screenshot on failure |
| `DisplayAlert` may switch to the default screen | Low | Medium | Keep the alert body short; capture the line only after the alert returns |
| A window layer plus gadgets exceeds 512 KB in debug | Medium | High | Build the acceptance demo with `--release`; keep the layout and strings small |
| `AddGList` coordinates differ under `WFLG_GIMMEZEROZERO` | Low | Medium | Validate against the inner dimensions and verify the first frame; document the coordinate rule |
| Input events are asynchronous and can arrive out of order | Medium | Medium | Drive a fixed sequence with polling and retries; count only the events the demo requires |
