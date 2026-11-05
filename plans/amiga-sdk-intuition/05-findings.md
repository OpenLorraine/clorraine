# Findings: Amiga SDK Bundle 6 — Intuition UI

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: In progress (Phase 1 recorded)

## Phase 1 review — window and events

Independent review: one correctness reviewer and one read-only memory-safety
auditor, both fresh-context subagents. Their packets included the Phase 1 task
and deliverable lines, `03-01-window-and-events.md`, the original goal and
smallest viable design, the strict scope baseline, the phase diff surface, and
the blast-radius and test-integrity steps.

### Correctness findings

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-1 | 🟠 Major | `compiler/src/stdlib.rs` listed only the window functions; `WindowEvent`, the eight `EVENT_*` exports, `waitEvent`, and `pollEvent` were missing from the export table. | **Fixed** — the table now lists every module export in source order. `amiga_intuition_spec_test` and the host/1.3/2.0 cross-builds re-run green. |
| RV-2 | 🟡 Minor | Some impl assertions were token-presence checks over the whole emitted C, so the shim definitions could satisfy them without the generated wrapper behavior. | **Fixed** — the routing case now asserts the generated wrapper call shapes (`return cl_open_window(...)`, `cl_window_fill(window, ...)`, `cl_window_next_event(window, 1)`, and so on); the close case inspects the `cl_close_window` block only; the size case asserts GZZ return lines. |
| RV-3 | 🟡 Minor | IT-6 did not pin the zeroed `last_event` or the unknown-class reply-and-skip path. | **Fixed** — the event case asserts `memset(&handle->last_event, 0, ...)` and `if (!cl_window_fill_event(handle, msg))`. |
| RV-4 | 🟡 Minor | The host-rejection case compiled only `openScreen`, not the full new surface. | **Fixed** — it now compiles `FULL_WINDOW_SURFACE` for `host` and asserts `E11200` with no C. |

### Memory-safety findings

The auditor extracted the changed C block into two host AddressSanitizer probes
(`probe_window.c`, `probe_text.c`) and ran them clean: 12 window cases and 4
text cases, no ASan/LSan output, no wrong allocation size, no double free, no
message leak, and every received message replied exactly once. The probes are
host copies of the C block; the real m68k Intuition path is target-only.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| MS-1 | 🟠 Major | The `Window` doc said “declare the screen before the window so the reverse drop order releases the window first”. That is true for locals but false for struct fields, which drop in declaration order. A struct with the screen field first would close the screen before the window on KS1.3 and could use-after-free. | **Fixed** — `std.amiga.intuition.clo`, `03-01`, and AR-8 now state both rules (locals: screen first; struct fields: window first) and warn against returning a window whose screen is local. A new impl case pins the emitted local and struct cleanup order. |
| MS-2 | 🟡 Minor | The new doc omitted the escape warning that `std.amiga.video` gives for `Display`: returning a `Window` from a function whose locals own the `Screen` closes the screen at return. | **Fixed** — the `Window` doc now carries the warning; the deeper lifetime tie remains the preflight-accepted residual. |
| MS-3 | 🟡 Minor | Phase 1’s window/event path is compile/cross-build-only; the real A500 smoke that runs it is Phase 4. Host ASan covers extracted copies, not the real Intuition path. | **Accepted** — the target proof is Phase 4’s `make sdk-a500-gui`; the plan records the static-only status until then. |

### Phase 1 verification

| Gate | Result |
|---|---|
| `cargo test --test amiga_intuition_spec_test` | 5/5 green; both cross-builds pass with `-Wall -Wextra -Werror` |
| `cargo test --test amiga_intuition_impl_test` | 6/6 green; routing, wrapper types, flags, event mapping, close order, drop order |
| Host ASan probes (extracted C block) | clean |
| Target behavior | deferred to Phase 4’s A500 GUI smoke |

## Phase 2 review — gadgets and menus

Independent review: one correctness reviewer and one read-only memory-safety
auditor. The auditor extracted the changed C block into a host ASan harness with
fault injection; the normal path and the menu/event paths were clean, and one
allocation-failure sweep found the leak below.

### Correctness findings

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-1 / MS-1 | 🟠 Major | `addString` stored `string_buffer` before `string_info`; `cl_window_free_gadget` freed the buffer only inside `if (node->string_info != NULL)`. Five partial-allocation paths leaked the 33–65-byte edit buffer. The ASan fault sweep reproduced five leaks (245 bytes). | **Fixed** — `cl_gadget_node` stores `string_buffer_size`, set when the buffer is allocated; the free path frees it unconditionally, before `StringInfo`. The fault sweep is clean; the impl case pins `FreeMem(node->string_buffer, node->string_buffer_size)`. |
| RV-2 | 🟠 Major | `addMenu` linked `NextMenu` and `addMenuItem` linked `FirstItem`/`NextItem` before `ClearMenuStrip`, so an attached strip could be traversed while its chain was being mutated. | **Fixed** — the strip is detached before any menu/item link; `cl_window_menu_link` rebuilds `Menu.NextMenu`; `cl_window_menu_set` attaches only a detached strip. The impl case pins detach-before-link and link-before-set. |
| MS-2 | 🟠 Major | `addMenu` immediately called `SetMenuStrip` with `FirstItem = NULL`; Intuition does not accept a menu with an empty item chain. | **Fixed** — `addMenu` appends the header but does not attach it. The first `addMenuItem` detaches, links the item and the `NextMenu` chain, and attaches. A menu with no items stays invisible; the demo adds File's items before Help. |
| RV-3 | 🟡 Minor | The `SetMenuStrip` false path was treated as reachable; the 3.x autodoc says it always waits and returns true. Its free-before-reattach order was unsafe if it ran. | **Retained as a defensive branch** — the 1.3 prototype returns `BOOL`. The detach-before-link refactor makes the false path unlink while detached, rebuild, reattach, then free; no dangling `MenuStrip`. 03-02/07 now explain the defensive status. |
| RV-4 | 🟡 Minor | `ItemAddress` was called with `MENUNULL` (0xFFFF), the cancelled-menu code. | **Fixed** — `cl_window_menu_id` returns 0 for `MENUNULL` before `ItemAddress`; the impl case pins it. |
| RV-5 | 🟡 Minor | Phase 2 impl assertions omitted `AddGList`/`RefreshGList`, the `IAddress` lookup, and the render wiring; the token checks could pass with the calls deleted. | **Fixed** — the cases now assert `AddGList`, `RefreshGList`, `cl_window_gadget_id(handle, msg->IAddress)`, `GadgetRender`/`SelectRender`/`GadgetText`/`SpecialInfo`, the menu detach/link/set order, and the string-buffer size. |
| RV-6 | 🟡 Minor | ST-6 added one menu and one item, so the second-menu index and `NextMenu` append branch were only token-pinned. | **Fixed** — `FULL_GADGET_SURFACE` now adds a second menu and item (`help` index 1). The runtime index is still proven by the Phase 4 demo. |

### Phase 2 fix re-review

The memory-safety auditor re-ran the host ASan fault-injection probe after the
fixes. The addString sweep, button/menu allocation fail points, false
`SetMenuStrip` paths, `MENUNULL`, and the normal close were all clean:
`live_allocs=0`, `size_mismatches=0`, `double_frees=0`, no dangling
`Window->MenuStrip`, and a mutant that drops the string-buffer size assignment
aborts as expected.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| MS-1 (re-review) | 🟡 Minor | The impl cases pinned fix tokens but not the behavior: deleting `string_buffer_size`, re-adding a `SetMenuStrip` in `addMenu`, or moving the `MENUNULL` check after `ItemAddress` would still pass. No repo suite references `RDC_ASAN`. | **Fixed for the pins** — the impl case now asserts `node->string_buffer_size`, checks the `addMenu` body contains no `SetMenuStrip`, and asserts `MENUNULL` before `ItemAddress`. The host ASan probe remains an auditor artifact; the target path is covered by the A500 gate (accepted residual below). |
| MS-2 (re-review) | 🟡 Minor | `cl_window_menu_link` linked headers with `FirstItem == NULL`, so a tolerant `SetMenuStrip` could install a chain with an empty menu. | **Fixed** — `cl_window_menu_head` returns the first non-empty menu and `cl_window_menu_link` skips empty headers; `cl_window_menu_set` refuses a chain with no non-empty menu. The probe’s `empty-trailing-tolerant` shape is no longer reachable. |

### Phase 2 verification

| Gate | Result |
|---|---|
| `cargo test --test amiga_intuition_spec_test` | 9/9 green; both cross-builds pass with `-Werror` |
| `cargo test --test amiga_intuition_impl_test` | 11/11 green; gadget/menu routing, shapes, chain, limits, free order |
| Host ASan fault sweep | normal/menu/event clean; the five string-buffer leaks are fixed and re-swept clean |
| Target behavior | deferred to Phase 4’s A500 GUI smoke |

## Phase 3 review — requesters, alerts, and pointer

Pending.

## Phase 3 review — requesters, alerts, and pointer

Independent review: one correctness reviewer and one read-only memory-safety
auditor. The auditor extracted the Phase 3 block into a host ASan probe (18
cases, including every allocation fail point) and ran it clean; a mutant that
removed both body-copy frees leaked 228 bytes, confirming the probe is live.

### Correctness findings

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-1 | 🔴 Critical | `DisplayAlert` was passed a plain C string. The 1.3/2.x autodoc requires an AlertMessage: a 16-bit x, an 8-bit y, the text, a text NUL, and a zero continuation byte (two NULs total). The plain 41-byte body made the first bytes coordinates and read the continuation byte one past the allocation. | **Fixed** — `cl_show_alert` builds a zeroed 33-byte `CL_ALERT_MESSAGE_MAX` buffer with `{0x00, 0x14, 0x14}` (x=20, y=20), copies the 28-byte body at offset 3, and calls `DisplayAlert(1, message, 20)`. The plan's `03-03` now documents the format; the impl case pins the header, text, double NUL, and free. |
| RV-2 | 🟠 Major | `AutoRequest` was called with width/height 0. The 1.3 autodoc uses the dimensions directly; only V36 ignores them. Zero would give no text area or fall back to `DisplayAlert`. The plan's “zero flags keep the standard pens” was also wrong: `pFlag`/`nFlag` are IDCMP flags. | **Fixed** — the call is `AutoRequest(..., 0, 0, 240, 40)`. The 240x40 requester fits the 272-pixel window and the 28-byte (224-pixel) body. `03-03` explains the IDCMP flags and the 1.3 sizing rule. |
| RV-3 / MS-1 | 🟡 Minor | Body-copy lifetime and frees were not pinned; deleting either `FreeMem` still passed. | **Fixed** — the impl case pins `FreeMem(body_copy, CL_WINDOW_BODY_MAX + 1)` and `FreeMem(message, CL_ALERT_MESSAGE_MAX)`, and asserts the show-alert order (header, `DisplayAlert`, free). |
| RV-4 | 🟡 Minor | The pointer allocation guard and exact-once close free were not pinned. | **Fixed** — the impl case pins `if (handle->pointer == NULL)`, the 64-byte define, and the single close `FreeMem(handle->pointer, CL_WINDOW_POINTER_BYTES)` in order. |
| RV-5 | 🟡 Minor | IT-9's forbidden-name list omitted ASL and BOOPSI. | **Fixed** — added `AllocAslRequest`, `AslRequest`, `NewObject`, `DoMethod`, `SetAttrs`, and `OpenScreenTagList`. |
| RV-6 | 🟡 Minor | The pointer template did not decode to a coherent arrow; rows 11–15 were a disconnected diagonal. | **Fixed** — the template is a coherent 16x16 upper-left arrow (solid head and shaft); the A500 held-frame golden captures it. |
| RV-7 | 🟡 Minor | `setColor` passed pens 32+ to `SetRGB4` despite the documented caller error. | **Fixed** — the shim is a no-op for `pen >= 32`; the graphics doc says so; the impl case pins `pen < 32`. |

### Phase 3 fix re-review

The memory-safety auditor re-ran the host ASan probe on the fixed block. The
AlertMessage is exactly 33 bytes (`3 + 28 + 2`), the 28-byte body leaves one byte
for the text NUL and one for the continuation NUL, `AutoRequest` is 240x40, and
the pointer/body allocations are freed with their exact sizes. A `+4` alert
mutant produced a one-byte heap overflow; the fixed `+5` is clean. Both suites
stay green.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| MS-1 (re-review) | 🟡 Minor | The `if (handle->pointer == NULL)` pin matched both the outer allocation guard and the inner failure guard, so deleting either still passed. | **Fixed** — the case pins the two branches with their distinct bodies: the allocation block and the `return` block. |
| MS-2 (re-review) | 🟡 Minor | The pin used the bare `CL_ALERT_MESSAGE_MAX` token, not the `+5` definition, so a `+4` mutant passed. | **Fixed** — the case pins `#define CL_ALERT_MESSAGE_MAX (CL_WINDOW_BODY_MAX + 5)`. |

### Phase 3 verification

| Gate | Result |
|---|---|
| `cargo test --test amiga_intuition_spec_test` | 13/13 green; both cross-builds pass with `-Werror` |
| `cargo test --test amiga_intuition_impl_test` | 14/14 green; requester/alert/pointer routing, calls, pointer size, forbidden names |
| Host ASan probe | 18 cases clean; body-copy mutant leak detected; no shim defect |
| Target behavior | deferred to Phase 4’s A500 GUI smoke |

## Phase 4 — GUI demo, A500 acceptance, and closeout

### A500 harness finding (runtime)

`examples/amiga/gui.clo` builds for `hosted-amiga-13` in release mode and the
A500 window renders: the screen/title bars, the window, the `Image`-rendered
Press button, the `Border`-rendered string field, the green status text, and the
custom pointer are visible. A synthetic key reaches the IDCMP loop (a temporary
`EVENT_KEY` status changed to `key`), and the menu strip is attached
(`strip 0xc1c228`, `FirstItem` set) after both menus are built.

The acceptance harness is blocked:

- the host Xwayland session does not deliver XTEST events to FS-UAE, so host
  `xdotool` clicks/keys do not reach the Amiga;
- an Xvfb display does deliver keyboard/button events, but FS-UAE's mousehack
  pointer control is not reliably positionable with `xdotool`, so the Press
  button and menu headers cannot be clicked deterministically;
- the menu bar is not visible in the captured frame even though the strip is
  attached, so the Help→About path cannot be clicked.

This is recorded as AR-25 with four options (human-driven run, uinput helper,
self-driving synthetic events, or a reduced visual gate). Phase 4 tasks 4.1.2–
4.1.4 are `[!]` until the user decides. No code or test in Phases 1–3 depends on
the unresolved harness choice.

## Phase 4 — GUI demo and automated A500 acceptance

### Correctness and runtime findings

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-8 | 🔴 Critical | `cl_open_window` rejected every window wider than 320 or taller than 256, so the 640x256 demo's 576x208 window returned NULL (`gui: err 3`). The limit was an implementation guard, not an Intuition or chip-RAM limit. | **Fixed** — the artificial cap is gone; the screen-bounds check remains. `make sdk-a500-gui` opens the window. |
| RV-9 | 🔴 Critical | The native capture read the 1.3 `ColorMap` as three bytes per entry. Kickstart 1.3 stores each color as a big-endian 12-bit `0x0RGB` word, so the first capture had the wrong palette. | **Fixed** — read `UWORD` entries and expand the four-bit components. The committed golden has the intended palette. |
| RV-10 | 🔴 Critical | String gadgets set only `GACT_IMMEDIATE`. Kickstart sends `IDCMP_GADGETUP` only for a `GACT_RELVERIFY` string gadget, so Return deactivated the field silently and the requester never opened. | **Fixed** — `GACT_IMMEDIATE \| GACT_RELVERIFY`; the impl pin is updated. |
| RV-11 | 🟠 Major | `cl_window_add_menu_item_internal` set `item->Command` for a shortcut and then cleared it unconditionally with `item->Command = 0`, so Right Amiga+H/A/Q never fired. | **Fixed** — the clear is removed; the impl case pins `COMMSEQ` and `Command` and rejects the stray clear. |
| RV-12 | 🟡 Minor | The first capture wrote a 491 KB PPM through `fputc`; the floppy write blocked the event loop for minutes. | **Fixed** — compact `CSCR` bitplanes (61 KB) and plane writes; the smoke enables FS-UAE turbo floppy mode. |
| RV-13 | 🟡 Minor | Host XTEST on Xwayland does not reach FS-UAE, and `Alt_R` was host Alt rather than an Amiga key. | **Fixed** — private Xvfb display; `Super_L` is Left Amiga and `Menu` is Right Amiga. `make sdk-a500-gui` is headless. |
| RV-14 | 🟡 Minor | `DisplayAlert` opens a private modal screen that the headless harness cannot reliably focus and dismiss, so the About path stalled. | The interactive About item uses `askYesNo`; `showAlert` remains implemented, documented, and spec-tested. Recorded in AR-25. |
| RV-15 | 🟡 Minor | The first per-event debug markers created one file per key on the emulated floppy and stalled the app. | Removed; the final demo writes only five one-byte handshake files at real state transitions. |

### Phase 4 verification

| Gate | Result |
|---|---|
| `cargo test --test amiga_intuition_spec_test` | 13/13 green; both cross-builds pass with `-Werror` |
| `cargo test --test amiga_intuition_impl_test` | 14/14 green |
| `make sdk-a500-gui` | Pass; native frame matches `docs/amiga-sdk/a500-gui.png`; line `gui: ok win 1 gad 2 menu 2 str 3 req 2` |
| `make sdk-registry` | OK (79 capabilities, 10 bundles); coverage regenerated |

## Closeout — full gate and issue closeout

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-16 | 🟠 Major | The five custom-display A500 smokes started polling 30 s after launch and kept the first FS-UAE window ID. Under a loaded headless run the demo could finish before the first capture, or FS-UAE could recreate the window, so the gate failed intermittently (`paddle: never saw the paddle screen`, `boing: held display never matched`). | **Fixed** — poll from 10 s every 2 s, re-find the `FS-UAE` window on each iteration, retry a failed capture, and report a missing window separately. `make sdk-a500-boing` and `make sdk-a500-paddle` pass under Xvfb and on the host display; the full `make verify` passes headlessly. |
| RV-17 | 🟡 Minor | The plan's success criteria still said `req 1` and expected an alert golden after AR-25 moved the interactive path to `askYesNo`; R9/R10, AR-18/19/20, ST-7/11/12/15/16, and the preflight PF-011/PF-016 rows carried the old mouse/alert flow. | **Fixed** — one consistency pass now records the 640x256x3 demo, two requesters, keyboard-only acceptance, `req 2`, and no alert golden; the alert path stays spec-tested. |

### Closeout review findings

Independent closeout review: correctness and maintainability over the appendix
docs and the five patched smoke harnesses. No gate-weakening or functional smoke
defect was found.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-C1 | 🟠 Major | The §8.1 example used `EVENT_NONE` without importing it. | **Fixed** — added `EVENT_NONE` to the import list; the example compiles on `hosted-amiga-13` with no diagnostics. |
| RV-C2 | 🟠 Major | Appendix B still showed the screen-only drawing surface and omitted `windowText`. | **Fixed** — §9 now declares and documents `windowFill`, `windowDraw`, and `windowClear`; §21 documents `windowText`. |
| RV-C3 | 🟠 Major | The plan set still carried the pre-AR-25 criteria (`req 1`, alert golden, mouse flow). | **Fixed** — R9/R10, the acceptance criteria, AR-18/19/20, ST-7/11/12/15/16, the evidence table, and PF-011/PF-016 now record the shipped 640x256x3 keyboard-only flow, `req 2`, and no alert golden. |
| RV-C4 | 🟡 Minor | The example passed a temporary `String` title to `openScreen`, which keeps the pointer. | **Fixed** — the title is bound to a local `String` that outlives the screen; the example compiles cleanly. |
| RV-C5 | 🟡 Minor | The feature index still said 13/17 and blocked on AR-25. | **Fixed** — it now says Done 17/17, accepted, issue #7 closed. |
| RV-C6 | 🟡 Minor | The appendix claimed a pen outside the playfield range has no visible effect, but the drawing calls pass the full `u8` to `SetAPen`. | **Fixed** — the text now states that only `0..(1 << depth) - 1` are defined playfield colors and a higher pen is a caller contract. |
| RV-C7 | 🟡 Minor | The event fields were described as raw message values, but the mouse fields come from `GZZMouseX`/`GZZMouseY`. | **Fixed** — the text now distinguishes the raw code/qualifier from the current inner-window pointer coordinates. |
| RV-C8 | 🟡 Minor | `codeops/specialist-check.json` recorded 60 plans while 61 exist; `amiga-sdk-intuition` was missing. | **Fixed** — the check was rerun over all 61 plan folders; `AGENTS.md` records 61. |
| RV-C9 | 🟡 Minor | AR-26 claimed the appendices distinguish the KS1.3 runtime proof from the 2.0+ compile-only check, but they did not say it. | **Fixed** — both appendices now state that the A500/KS1.3 gate is the runtime proof and `hosted-amiga` is compile-verified. |
| RV-C10 | 🟡 Minor | ST-4/ST-5/ST-9 and PF-012 still described a close-gadget click, right-click menus, and an interactive alert, while the shipped smoke is keyboard-only and uses `askYesNo`. | **Fixed** — the rows now describe the Right Amiga menu shortcuts and File → Quit, mark the close-gadget branch as impl-pinned and visual-only on A500, and state that `showAlert` is spec-tested. |

### Closeout verification

| Gate | Result |
|---|---|
| `make check` | 106 test suites green, 0 failures |
| `make verify` | All legs passed headlessly on Xvfb: A500 smoke/Boing/sprites/paddle/text/assets/GUI, differential, corpus-fs-uae, and run-fault-amiga |
| `make sdk-registry` | OK (79 capabilities, 10 bundles); coverage regenerated |
| `make sdk-issues` | 10 issue bodies; #7 body regenerated |
| GitHub issue #7 | Updated with the checked body, commented, and closed |

## Accepted residuals

| Residual | Reason |
|---|---|
| The interactive About path uses a requester instead of `DisplayAlert`. | A private modal alert screen is not reliably focusable from the headless harness. `showAlert` is covered by the spec suite and the host C probe. |
| The native capture is slower on a real floppy than under FS-UAE turbo mode. | The helper is an acceptance/development tool, not a production hot path; the compact format keeps a normal run bounded. |
| The window has no lifetime tie to its screen. | The language has no lifetime parameter here; the rule is documented and both local/struct drop orders are pinned. |
| The m68k Intuition path cannot run under host ASan. | The A500 gates are the execution oracle; the extracted host probes cover the pure C block. |
| Phase 1 target behavior is proved in Phase 4. | The GUI smoke is the acceptance vehicle; the plan’s phases deliberately build the surface first. |
