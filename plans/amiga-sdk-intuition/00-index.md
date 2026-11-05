# Amiga SDK Bundle 6: Intuition UI Implementation Plan

> **Feature**: Windows, gadgets, menus, IDCMP events, requesters, alerts, and a pointer on the A500/KS1.3 Intuition surface, proven by a GUI demo.
> **Status**: Executing — 13/17 tasks; Phase 4 blocked on AR-25 (A500 input/menu acceptance)
> **Created**: 2026-11-05 10:25
> **Implements**: —
> **CodeOps Artifact Schema**: 1

## Overview

Bundle 6 extends `std.amiga.intuition` from the existing custom-screen lifecycle
to the smallest useful Intuition application surface: a window on a custom
screen, classic boolean and string gadgets, a menu bar, an IDCMP event loop,
a Yes/No requester, a recovery alert, and the built-in pointer. The new drawing
and text calls live in the existing `std.amiga.graphics` and `std.amiga.text`
modules, which keep their current ownership split. A GUI demo proves the whole
surface on the real A500/Kickstart 1.3 machine.

The plan implements SDK-INT-001 through SDK-INT-005 and SDK-INT-009. The three
advanced registry rows (gadtools 2.0+, ASL 2.0+, BOOPSI 3.0+) stay deferred
because the acceptance baseline is KS1.3.

## Minimum-Sufficient Baseline

**Original goal:** add Intuition windows, gadgets, menus, an event loop,
requesters/alerts, and a pointer that run on the A500/KS1.3, with a GUI demo.

**Smallest viable design:** open a window on the existing custom `Screen`, append
classic 1.3 gadgets with `AddGList`/`RefreshGList`, attach a classic menu strip,
and expose the IDCMP messages the demo needs as one copyable `WindowEvent` value.
The window handle owns every gadget, menu, label, string buffer, and pointer
allocation, so the caller never frees Intuition-owned memory directly.

**Excluded machinery:** a window builder lifecycle, a generic widget framework,
gadtools/ASL/BOOPSI, submenus, gadget update/enable APIs, custom pointer image
formats, Workbench windows, and any language or ABI change. The existing
spec/impl split and FS-UAE boot/capture harness are reused; the GUI smoke adds
interaction steps (focus, pointer moves, right-button menu gesture, typing) to
the existing pattern rather than a new runner.

**Approved complexity:** none beyond the direct surface above.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| PF | [Preflight Report](00-preflight-report.md) | Independent audit, findings, and accepted residuals |
| 00 | [Index](00-index.md) | This document — overview and navigation |
| 01 | [Requirements](01-requirements.md) | Feature requirements and scope |
| 02 | [Current State](02-current-state.md) | Analysis of the current Intuition, graphics, and text surface |
| 03-01 | [Window and Events](03-01-window-and-events.md) | Window lifecycle, drawing/text, IDCMP event loop |
| 03-02 | [Gadgets and Menus](03-02-gadgets-and-menus.md) | Classic buttons, string gadget, menu bar |
| 03-03 | [Requesters and Pointer](03-03-requesters-pointer.md) | Yes/No requester, alert, built-in pointer, palette |
| 03-04 | [GUI Demo and Smoke](03-04-gui-demo-and-smoke.md) | Demo layout, event flow, A500 interaction, goldens |
| 05 | [Findings](05-findings.md) | Phase review ledger and accepted residuals |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec cases, impl cases, verification |
| 99 | [Execution Plan](99-execution-plan.md) | Phases, tasks, and progress |

## Quick Reference

### Usage sketch

```clorraine
import { Screen, openScreen, Window, openWindow, addButton, addMenu,
         addMenuItem, waitEvent, WindowEvent, EVENT_CLOSE,
         setPointer } from std.amiga.intuition;
import { setColor, windowClear } from std.amiga.graphics;
import { String, stringOf, toCString } from std.text;

func run(): i32 {
    // The screen keeps this pointer; it must outlive the screen.
    let screenTitle: String = stringOf("GUI");
    let screen: Screen? = openScreen(320, 200, 4, toCString(screenTitle));
    if (screen == null) { return 1; }
    setColor(screen, 0, 0, 0, 0);
    // The window copies this title, so a temporary is safe here.
    let window: Window? = openWindow(screen, 24, 24, 272, 136,
                                      toCString(stringOf("Clorraine GUI")));
    if (window == null) { return 2; }
    windowClear(window, 0);
    let added: bool = addButton(window, 8, 28, 80, 18,
                                toCString(stringOf("Press")), 1);
    let fileMenu: i32 = addMenu(window, toCString(stringOf("File")));
    let item: bool = addMenuItem(window, fileMenu,
                                 toCString(stringOf("Quit")), 11);
    setPointer(window, true);
    let event: WindowEvent = waitEvent(window);
    if (event.kind == EVENT_CLOSE) { return 0; }
    return 0;
}
```

### Key decisions

| Decision | Outcome |
|---|---|
| Rows | Implement 001–005 and 009; defer 006–008 |
| Gadgets | Classic boolean buttons and one string gadget |
| Events | Public `WindowEvent` struct; blocking and polling loops |
| Strings | Shim copies titles and labels; handles free them |
| Drawing | Window variants in the existing graphics/text modules |
| Pointer | Built-in chip-memory arrow; palette stays on `setColor` |
| Demo | Real `xdotool` interaction on the A500, deterministic line |

## Specialist Agents

**None** — the capability-gap check found no gap a standing specialist would
close.

_Detection evidence: the bundle adds only C shim functions
(`compiler/runtime/shim_hosted_amiga.c`), embedded Clorraine module wrappers
(`stdlib/std.amiga.intuition.clo`, `std.amiga.graphics.clo`,
`std.amiga.text.clo`), module registration (`compiler/src/stdlib.rs`), and
smoke/test files. It changes no compiler stage, ownership rule, IR shape, symbol
name, or emitted ABI. The phase review still follows the repository packet for a
C shim/stdlib surface: an independent correctness and memory-safety pass with a
blast-radius step (every window, gadget, menu, string, pointer, and error path)
and a test-integrity step (each assertion fails without its fix). The m68k shim
cannot run under the host AddressSanitizer auditor; the A500 runtime gates are
the execution oracle._

## Related Files

| File | Change |
|---|---|
| `stdlib/std.amiga.intuition.clo` | Window, gadget, menu, event, requester, alert, and pointer surface |
| `stdlib/std.amiga.graphics.clo` | Window drawing variants |
| `stdlib/std.amiga.text.clo` | Window text variant |
| `compiler/src/stdlib.rs` | Module export tables |
| `compiler/runtime/shim_hosted_amiga.c` | Intuition window/gadget/menu/event/requester/pointer shim |
| `compiler/tests/amiga_intuition_spec_test.rs` | Specification tests |
| `compiler/tests/amiga_intuition_impl_test.rs` | Implementation tests |
| `examples/amiga/gui.clo` | GUI demo |
| `docker/scripts/a500-gui-smoke.sh` | A500 interaction smoke |
| `Makefile` | `sdk-a500-gui` target and `verify` leg |
| `spec/appendix-amiga.md`, `spec/appendix-stdlib.md` | Surface documentation |
| `docs/amiga-sdk/` | Registry, bundle, coverage, held/alert/line screenshots |
| `plans/00-roadmap.md`, `plans/amiga-sdk/00-index.md` | Programme status |
