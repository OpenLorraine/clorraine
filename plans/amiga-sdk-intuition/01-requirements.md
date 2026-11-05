# Requirements: Amiga SDK Bundle 6 — Intuition UI

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1–R13

## Feature Overview

Bundle 6 adds the Intuition application surface that the A500/KS1.3 baseline
needs: a window on a custom screen, classic boolean and string gadgets, a menu
bar, an IDCMP event loop, a Yes/No requester, a recovery alert, and the built-in
pointer. Drawing and text stay in the existing `std.amiga.graphics` and
`std.amiga.text` modules, gaining window variants. A GUI demo proves the surface
on the real machine.

The bundle does not change the language, grammar, diagnostics, or ABI. It adds
no 2.0+ library call to the 1.3 build.

## Functional Requirements

### Must Have

- **R1 — Window lifecycle.** `openWindow(screen, x, y, width, height, title)`
  opens a window on the caller's custom screen and returns an owned `Window?`,
  or `null` on failure. `width`/`height` are the outer window size;
  `windowWidth`/`windowHeight` return the inner drawable size. The window uses a
  drag bar, a depth gadget, a close gadget, `WFLG_GIMMEZEROZERO`, and no sizing,
  zoom, backdrop, or Workbench mode. The screen must outlive the window; the
  demo declares the screen first so the drop order closes the window first.
  `closeWindow` and the generated drop close the window exactly once.
- **R2 — Window drawing and text.** `windowFill`, `windowDraw`, and
  `windowClear` in `std.amiga.graphics` draw into the window's `RastPort`;
  `windowText` in `std.amiga.text` draws counted text at inner-window
  coordinates. The window layer clips every call.
- **R3 — Event loop.** `waitEvent` blocks for the next IDCMP event;
  `pollEvent` returns `EVENT_NONE` when the queue is empty. Both return a
  `WindowEvent` with `kind`, raw `code`, `qualifier`, inner-window
  `mouse_x`/`mouse_y`, `gadget` id, and `menu` id. The kinds are `EVENT_NONE`,
  `EVENT_CLOSE`, `EVENT_GADGET`, `EVENT_MENU`, `EVENT_MOUSE`, `EVENT_KEY`,
  `EVENT_ACTIVE`, and `EVENT_INACTIVE`. Refresh events are acknowledged inside
  the shim and are not delivered.
- **R4 — Buttons.** `addButton(window, x, y, width, height, label, id)` appends
  a classic `GTYP_BOOLGADGET` to an open window and returns `true` on success.
  The label is copied. A click that releases over the button delivers
  `EVENT_GADGET` with the caller's id.
- **R5 — String gadget.** `addString(window, x, y, width, height, label,
  initial, maxChars, id)` appends a classic `GTYP_STRGADGET` with a shim-owned
  buffer. `windowString(window, id, buf)` copies the current text into the
  caller's buffer and returns the copied byte count. Enter over the gadget
  delivers `EVENT_GADGET` with the caller's id.
- **R6 — Menus.** `addMenu(window, title)` appends a menu header and returns its
  0-based index or `-1`. `addMenuItem(window, menu, label, id)` appends an
  enabled textual item and returns `true` on success. A menu pick delivers
  `EVENT_MENU` with the caller's item id. No submenus, checkmarks, toggles, or
  command-key shortcuts.
- **R7 — Requesters and alerts.** `askYesNo(window, body)` shows a modal
  `AutoRequest` inside the window with fixed Yes/No labels and returns `true`
  for Yes. `showAlert(body)` shows a modal `DisplayAlert` with
  `RECOVERY_ALERT`. Both copy the body and block until the user answers.
- **R8 — Pointer and palette.** `setPointer(window, true)` installs a built-in
  16x16 arrow allocated in chip memory; `setPointer(window, false)` clears it.
  The arrow uses sprite colors 16–19, which the caller sets through the existing
  `setColor(screen, pen, r, g, b)` before or after opening the window. The
  `ColorMap` has 32 entries: pens 0–15 are the depth-4 playfield colors and
  16–31 are the sprite/pointer registers.
- **R9 — GUI demo.** `examples/amiga/gui.clo` opens a 320x200x4 custom screen
  and a window, adds one button and one string field, adds two menus, shows one
  Yes/No requester, shows one alert, installs the pointer, and runs a
  deterministic event loop. It drains the queue with `pollEvent`, uses the
  inner-size and drawing calls, and exits through the window close gadget. It
  prints `gui: ok win 1 gad 2 menu 2 str 3 req 1` after the screen and window
  close.
- **R10 — A500 acceptance.** `make sdk-a500-gui` builds the demo in release
  mode, boots it on the real KS1.3 ROM under FS-UAE as an A500, drives the
  window with `xdotool` (focus, pointer move, right-button menu gesture, typing,
  close-gadget click), compares the held frame, the alert frame, and the report
  line to committed goldens, and fails on any mismatch.

### Should Have

- **R11 — Errors, limits, and ownership.** Titles, labels, and menu text are at
  most 64 bytes; a string field is at most 64 characters; requester and alert
  bodies are at most 28 bytes. Invalid geometry, an unknown id, an id below 1,
  an over-limit string, or an allocation failure returns the call's failure
  value: `null` for `openWindow`, `false` for the boolean add/answer calls, `-1`
  for `addMenu`, and 0 for `windowString`; the void drawing, alert, pointer, and
  close calls are no-ops on invalid input. Every partial allocation is released.
  No public call panics or leaks.
- **R12 — Compatibility and gating.** The new surface compiles on both
  `hosted-amiga-13` and `hosted-amiga`; the host profile still rejects
  `std.amiga.intuition` with `E11200`. The 1.3 build calls only Kickstart 1.3
  vectors. No existing public API behavior changes; the `setColor` pen-range
  wording is a documentation correction. No language, grammar, diagnostic, or
  ABI change.
- **R13 — Documentation and tracking.** Update `spec/appendix-amiga.md`,
  `spec/appendix-stdlib.md` (including the `setColor` 0–31 pen range),
  `AGENTS.md`, the registry/coverage/bundle tables, issue #7, the programme map,
  the roadmap, and the specialist check. Registry Bundle 6 ends at 6/9 done with
  the three advanced rows deferred.

### Won't Have (Out of Scope)

- Workbench windows or Workbench integration (Bundle 8).
- Window sizing, zoom, backdrop, or iconify.
- Gadtools, ASL requesters, and BOOPSI (SDK-INT-006/007/008; 2.0+/3.0+).
- Submenus, checkmarks, toggles, command-key shortcuts, or menu enable/disable.
- Gadget update, enable/disable, or label replacement after creation.
- Custom pointer image data or pointer animation.
- Changes to `std.amiga.exec`, `std.amiga.layers`, or the native-video display.
- Any language, grammar, diagnostic, or ABI change.

## Technical Requirements

### Performance

- The demo must fit the 512 KB chip-RAM A500 with a 320x200x4 screen, one
  window layer, two gadgets, two menus, and one 64-byte pointer. The release
  build is the acceptance build.
- Every event loop iteration replies its IntuiMessage before returning; no
  queue grows without bound.

### Compatibility

- Only the KS1.3 Intuition vector set is used: `OpenWindow`, `CloseWindow`,
  `AddGList`, `RefreshGList`, `SetMenuStrip`, `ClearMenuStrip`, `ItemAddress`,
  `AutoRequest`, `DisplayAlert`, `SetPointer`, `ClearPointer`, `BeginRefresh`,
  `EndRefresh`, `WaitPort`, `GetMsg`, `ReplyMsg`, `AllocMem`, and `FreeMem`.
- The `hosted-amiga` profile compiles the same source; no 1.3-only conditional
  is visible to the caller.
- The existing `std.amiga.exec` 1.3 gate stays closed.

### Security

- Every copied string is bounded by its documented limit and NUL-terminated in
  shim-owned storage. No caller pointer is retained after the call returns.
- Geometry is validated against the screen and inner-window bounds before
  Intuition sees it; a rejected call allocates nothing that outlives it.
- No authentication, network, file, or user-data path is added.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
|---|---|---|---|---|
| Registry rows | All core / must only / all nine | All core, 6/9 | Matches Bundles 1–5; the 2.0+/3.0+ rows cannot run on the baseline | AR-1 |
| Gadget kinds | Buttons / buttons+string / +proportional | Buttons + string | Covers click and keyboard input with 1.3 structures | AR-2 |
| Event API | Struct / opaque handle / raw codes | Struct | Copyable, no drop, matches `IlbmInfo` and `JOY_*` | AR-3 |
| String ownership | Shim copies / caller-owned / mixed | Shim copies | Prevents dangling Intuition pointers | AR-4 |
| A500 proof | Real clicks / synthetic / both | Real clicks | Proves real IDCMP and input | AR-5 |
| Drawing home | Graphics+text / Intuition / screen only | Graphics+text | Keeps the existing module split | AR-6 |
| Pointer | Built-in / custom data / defer | Built-in arrow | Smallest 1.3-safe surface | AR-7 |
| Window construction | Open-then-add / builder | Open-then-add | `AddGList` is in the 1.3 SFD; no second lifecycle | AR-8 |
| System gadgets | Close+drag+depth / close only / +sizing | Close+drag+depth | Useful window without resize complexity | AR-9 |
| Palette range | Keep 0–15 doc / document 0–31 / new sprite call | Document 0–31 | The `ColorMap` has 32 entries; no behavior change | AR-24 |

> **Traceability:** every scope decision cites the register entry that resolved
> it in `00-ambiguity-register.md`.

## Acceptance Criteria

1. All seventeen execution tasks complete and marked in the execution plan.
2. Both hosted profiles compile the full surface; the host profile rejects
   `std.amiga.intuition` with `E11200`; both cross-builds pass with
   `-Wall -Wextra -Werror`.
3. The A500 smoke drives the real window, gadgets, menu, requester, alert, and
   pointer, and matches `docs/amiga-sdk/a500-gui.png`,
   `docs/amiga-sdk/a500-gui-alert.png`, and `docs/amiga-sdk/a500-gui-line.png`.
4. The report line is `gui: ok win 1 gad 2 menu 2 str 3 req 1`.
5. `make check`, `make verify`, and `make sdk-registry` pass; the six earlier
   A500 gates stay green.
6. Registry Bundle 6 is 6/9 done with SDK-INT-006/007/008 deferred; issue #7 is
   closed; the programme and roadmap show Bundle 6 done.
7. No language, grammar, diagnostic, or ABI change.
