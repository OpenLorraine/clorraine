# Ambiguity Register: Amiga SDK Bundle 6 — Intuition UI

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Status**: ✅ PLANNING GATE PASSED — 25 planning items resolved; 0 runtime items open
> **Mode**: normal (every material decision confirmed by the user)
> **Last Updated**: 2026-11-05 17:02

## Scope baseline

Bundle 6 (`6-intuition`) from `docs/amiga-sdk/registry.tsv`: screens/windows
(must), gadgets and menus (must), IDCMP event loop (must), requesters and
alerts (should), pointer and palette (could), gadtools (should, 2.0+), ASL
requesters (should, 2.0+), BOOPSI (could, 3.0+), and the GUI demo (must). No
language, grammar, diagnostic, or ABI change.

## Register

| # | Category | Decision / question | Resolution | Owner | Status |
|---|---|---|---|---|---|
| AR-1 | Scope | Which registry rows belong to Bundle 6? | Implement SDK-INT-001–005 and 009. Defer SDK-INT-006 (gadtools, 2.0+), SDK-INT-007 (ASL, 2.0+), and SDK-INT-008 (BOOPSI, 3.0+). Registry result 6/9 done, 3 deferred. | User | ✅ Resolved |
| AR-2 | Feature gaps | Which gadget kinds? | Classic 1.3 `GTYP_BOOLGADGET` buttons and a `GTYP_STRGADGET` text field only. No proportional gadget. | User | ✅ Resolved |
| AR-3 | Integration points | How are IDCMP events exposed? | A public `WindowEvent` value struct with kind, raw code, qualifier, mouse coordinates, gadget id, and menu id, plus exported `EVENT_*` constants. | User | ✅ Resolved |
| AR-4 | Data & state | Who owns window titles, gadget labels, and menu names? | The shim copies every string into handle-owned memory and frees it on close. The caller's `cstring` only has to live for the call. | User | ✅ Resolved |
| AR-5 | UX & presentation | How is the event loop proven on real 1.3? | A real interactive demo driven by `xdotool` clicks and typing in the A500 smoke, with a deterministic report line and goldens. | User | ✅ Resolved |
| AR-6 | Integration points | Where does window drawing and text live? | Extend `std.amiga.graphics` with `windowFill`/`windowDraw`/`windowClear` and `std.amiga.text` with `windowText`; do not duplicate the drawing surface in Intuition. | User | ✅ Resolved |
| AR-7 | Feature gaps | What pointer surface? | A shim-owned 16x16 arrow allocated in chip memory, shown and hidden per window. Palette entries use the existing `setColor` on the screen. | User | ✅ Resolved |
| AR-8 | Technical unknowns | How is a window built and who owns its gadgets? | Open the window first, then append gadgets with the 1.3 `AddGList`/`RefreshGList`. The `Window` handle owns every gadget, menu, string buffer, label, and pointer allocation. No separate builder handle. | User (design package) | ✅ Resolved |
| AR-9 | Scope ambiguities | Which window system gadgets? | Drag bar, depth gadget, and close gadget. Enable `WFLG_GIMMEZEROZERO`. No sizing, zoom, backdrop, or Workbench windows. | User (design package) | ✅ Resolved |
| AR-10 | Naming & terminology | What is the public window/menu API? | `Window`, `openWindow`, `closeWindow`, `windowWidth`, `windowHeight`, `addButton`, `addString`, `windowString`, `addMenu`, `addMenuItem`, `waitEvent`, `pollEvent`, `askYesNo`, `showAlert`, `setPointer`. The `WindowEvent` fields follow the repository's snake_case convention: `mouse_x`, `mouse_y`. | User (design package) | ✅ Resolved |
| AR-11 | Behavioral gaps | What is the coordinate contract? | `openWindow` width/height are the outer window size. `windowWidth`/`windowHeight` return the inner drawable area. Drawing, text, and gadget coordinates are inner-window pixels. | User (design package) | ✅ Resolved |
| AR-12 | Behavioral gaps | Which IDCMP classes and loop functions? | Fixed set: close, gadget-up, menu-pick, mouse-buttons, raw/vanilla key, active, and inactive. `waitEvent` blocks; `pollEvent` does not. Refresh is acknowledged inside the shim because the layer-backed window is not resizable. | User (design package) | ✅ Resolved |
| AR-13 | Behavioral gaps | How are requesters and alerts built? | `askYesNo(window, body)` uses `AutoRequest` with fixed Yes/No labels; `showAlert(body)` uses `DisplayAlert` with `RECOVERY_ALERT`. Both block and copy the body. | User (design package) | ✅ Resolved |
| AR-14 | Edge cases | What are the size and text limits? | Title/label/menu text 64 bytes; string buffer 64 characters; requester/alert body 28 bytes (224 pixels at Topaz 8, which fits the 240-pixel requester and the 320-pixel alert with its x=20 offset). Over-limit, invalid geometry, and unknown ids return `null`, `false`, `-1`, or 0 as appropriate. Label text wider than its gadget is clipped by Intuition and documented. | User (design package) + 1.3 fit | ✅ Resolved |
| AR-15 | Behavioral gaps | What happens on allocation or partial failure? | Every public call is null-safe; value-returning calls return their documented failure value (`null`, `false`, `-1`, or 0), and void calls are no-ops. No panic; every partial allocation is released before returning. `closeWindow` is idempotent through the move-only drop. | User (design package) | ✅ Resolved |
| AR-16 | Data & state | What are the id rules? | Gadget and menu-item ids must be at least 1. The event fields use 0 for "none". Unknown ids return false from add/read calls. | User (design package) | ✅ Resolved |
| AR-17 | Compatibility | Which OS calls are allowed? | Only the Kickstart 1.3 vector set: `OpenWindow`, `CloseWindow`, `WindowToFront`, `ActivateWindow`, `AddGList`, `RefreshGList`, `SetMenuStrip`, `ClearMenuStrip`, `ItemAddress`, `AutoRequest`, `DisplayAlert`, `SetPointer`, `ClearPointer`, `BeginRefresh`, `EndRefresh`, `WaitPort`, `GetMsg`, `ReplyMsg`, `AllocMem`, and `FreeMem`. No 2.0+/3.0+ call. | User (design package) + 1.3 SFD | ✅ Resolved |
| AR-18 | UX & presentation | What does the GUI demo do? | A 640x256x3 light screen with a 576x208 window at (32,32), one button, one string field, two menus, two Yes/No requesters (string Return and Help → About), and the built-in pointer. The demo drains the queue with `pollEvent`, uses the inner-size, drawing, and text calls, exits through File → Quit, and counts `req` only on a Yes answer. The report line is `gui: ok win 1 gad 2 menu 2 str 3 req 2`. Revised by AR-25: the interactive alert path uses `askYesNo`. | User (design package) + AR-25 | ✅ Resolved |
| AR-19 | Behavioral gaps | What is the A500 smoke sequence? | Capture the native held frame; type `abc` + Return and answer Yes with Left Amiga+V; select Hello with Right Amiga+H; select About with Right Amiga+A and answer Yes; quit with Right Amiga+Q; read `DF0:gui.txt`. The run is keyboard-only on a private Xvfb display and uses one-byte handshake files. Revised by AR-25. | User (design package) + AR-25 | ✅ Resolved |
| AR-20 | Non-functional gaps | What is the verification gate? | `make check`, `make verify` with the `sdk-a500-gui` leg, and `make sdk-registry`. The six earlier A500 gates stay green. Goldens: `docs/amiga-sdk/a500-gui.png` and the `DF0:gui.txt` report line; there is no alert golden because `showAlert` is spec-tested only (AR-25). | User (design package) + AR-25 | ✅ Resolved |
| AR-21 | Naming & terminology | Where is the plan and what is it called? | `plans/amiga-sdk-intuition/`, title “Amiga SDK Bundle 6: Intuition UI”. | User (design package) | ✅ Resolved |
| AR-22 | Stakeholder conflicts | What is out of scope and which specialists are needed? | Out: Workbench windows, resize/zoom, submenus/checkmarks/command keys, gadget updates, custom pointer data, `std.amiga.exec` changes, 2.0+ gadtools/ASL, BOOPSI, and any language/grammar/diagnostic/ABI change. Specialist: none; the bundle is C shim plus embedded stdlib, reviewed with the correctness/memory-safety packet and its blast-radius and test-integrity steps. | User (design package) | ✅ Resolved |
| AR-23 | Feature gaps | `WindowEvent?` is illegal because the language rejects nullable structs (`E10011`; Bundle 5 F-2). What replaces it? | `waitEvent` and `pollEvent` return a non-nullable `WindowEvent`. Add `EVENT_NONE = 0`; `pollEvent` and an invalid window return that sentinel. The public value struct is unchanged. | User (design package) + language constraint | ✅ Resolved |
| AR-24 | Consistency | `setColor` documents pens `0..(1 << depth) - 1`, but the pointer uses pens 16–19 on a depth-4 screen. | The 1.3 `ColorMap` has 32 entries: pens 0–15 are the playfield colors on a depth-4 screen and 16–31 are the sprite/pointer registers. Correct the `setColor` and appendix wording to `0..31` with that split. This is a documentation correction, not a behavior change. | User (design package) + 1.3 ColorMap fact | ✅ Resolved |
| AR-26 | Compatibility | How should the frozen appendices describe the Intuition surface across the two hosted profiles? | One classic 1.3 API documented for both `hosted-amiga-13` and `hosted-amiga`. The 2.0+ additions (gadtools, ASL requesters, and BOOPSI) stay deferred and are not documented as available. The appendix text states that the KS1.3 A500 gate is the runtime proof and the 2.0+ profile is compile-verified. This removes the stale claims that Intuition is 2.0+ only, that 1.3 rejects it, and that windows are outside v1. | User (2026-11-05 directive: proceed with the recommended one-surface wording) | ✅ Resolved |

## Runtime ambiguities

| # | Category | Decision / question | Options / evidence | Status |
|---|---|---|---|---|
| AR-25 | Integration points (runtime) | How should the A500 GUI acceptance drive input and prove the menu pick? | Resolved by an automated keyboard-driven run on a private Xvfb display. FS-UAE receives XTEST keys there; the menu picks use the documented Right Amiga command keys, so no pointer positioning is needed. The app writes a native `CSCR` capture and one-byte `DF0:gui-*` handshake files; the smoke polls the raw ADF for each handshake and for the final report line. Left Amiga is host `Super_L`; Right Amiga is host `Menu`/`RSUPER`; the earlier `Alt_R` attempt was host Alt, not an Amiga key. `DisplayAlert` is not driven headlessly: it opens a private modal screen the harness cannot reliably focus, so the interactive About item uses `askYesNo` while `showAlert` stays spec-tested. Evidence: `make sdk-a500-gui` passes with `gui: ok win 1 gad 2 menu 2 str 3 req 2` and `docs/amiga-sdk/a500-gui.png`. The uinput helper was prototyped and rejected because Xvfb does not hot-plug it; keyboard-only automation is smaller and reproducible. | ✅ Resolved — automated Xvfb keyboard smoke; `make sdk-a500-gui` |


## Open items

None. Every row is resolved. The only deferrals are the three 2.0+/3.0+
registry rows named in AR-1, with their minimum-OS reason.

## Resolution notes

**AR-4:** The existing `openScreen` title remains caller-owned; changing that
legacy contract is outside this bundle. All new Intuition-owned strings are
copied, so a freed `String` cannot leave Intuition pointing at released memory.

**AR-8:** `AddGList`/`RefreshGList` are present in the pinned 1.3 SFD
(`/opt/amiga/m68k-amigaos/ndk/lib/sfd13/intuition_lib.sfd`), so the smallest
design is open-then-add rather than a second builder lifecycle. The window
borrows the screen without a lifetime parameter, so the screen must stay alive
until the window closes. Locals drop in reverse declaration order (screen first,
window second); struct fields drop in declaration order (window field first,
screen field second); a function must not return a window whose screen is a
local.

**AR-12:** The window uses `WFLG_GIMMEZEROZERO`, so its layer preserves content
when it is obscured. Auto-acknowledging refresh is valid because the demo has no
sizing gadget and never changes the window geometry.

**AR-17:** The 1.3 SFD also lists `BeginRefresh`, `EndRefresh`, `SetMenuStrip`,
`ClearMenuStrip`, `ItemAddress`, `AutoRequest`, `DisplayAlert`, `SetPointer`,
and `ClearPointer`; the plan pins the list in `03-01` and `03-03`.

**AR-23:** The language has no nullable struct value, so the event result uses a
sentinel kind instead of `WindowEvent?`. `EVENT_NONE` is 0; the seven real kinds
start at 1. This preserves the user's value-struct choice and adds no handle or
drop.

**AR-24:** The existing `setColor` shim passes the pen straight to `SetRGB4` on
the screen's 32-entry `ColorMap`. The doc's depth-based 0–15 range is correct for
the bitmap but incomplete for the sprite/pointer registers; the correction is
documentation-only and adds no API.
