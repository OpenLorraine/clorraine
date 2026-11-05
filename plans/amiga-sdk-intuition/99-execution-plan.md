# Execution Plan: Amiga SDK Bundle 6 — Intuition UI

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-05 17:02
> **Progress**: 16/17 tasks (94%) — Phase 4 closeout; full verify and issue #7 pending
> **CodeOps Artifact Schema**: 1

## Overview

Four phases: pin and implement the window/event core, add classic gadgets and
menus, add the requester/alert/pointer surface, then build the GUI demo and prove
it on the real A500/KS1.3 machine. Specification tests come before
implementation in every phase.

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Window, drawing/text, and IDCMP events | 5 |
| 2 | Buttons, string gadget, and menus | 4 |
| 3 | Requester, alert, and pointer | 4 |
| 4 | GUI demo, A500 acceptance, closeout | 4 |

**Total: 17 tasks across 4 phases.**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of
> truth** for progress. Every task line appears exactly once in this document.
> The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the
>    Last Updated stamp after EVERY task — never batch updates. Only `[x]` counts
>    as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]`
>    task is resumed first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on
>    the same line. The plan lifecycle is `Ready`, `Executing`, `Done`, or
>    `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep
> the marks current means progress is invisible after crashes, context resets, or
> session handoffs.

---

## Phase 1: Window, drawing/text, and IDCMP events

> **Reasoning**: high — a new Intuition window lifecycle, private wrapper, and
> asynchronous event path on a 1.3 machine.

### Step 1.1: Window and event core

**Reference**: [03-01](03-01-window-and-events.md) · AR-8–AR-12, AR-17, AR-23

- [x] 1.1.1 [spec-author] Write `compiler/tests/amiga_intuition_spec_test.rs` for ST-1–ST-3: full window/event surface on both hosted profiles, host rejection (regression guard), and `nix13`/`nix20` cross-builds; run it red ✅ (completed: 2026-11-05 10:56; red evidence: 4 `E11102` unresolved-export failures, host-rejection guard passed)
- [x] 1.1.2 Add `Window`, `openWindow`, `closeWindow`, `windowWidth`, and `windowHeight` to `stdlib/std.amiga.intuition.clo`; add the `cl_window` open/close shim (title/wrapper only in this phase) to `compiler/runtime/shim_hosted_amiga.c`; update `compiler/src/stdlib.rs` ✅ (completed: 2026-11-05 10:57; verified: window-only program emits C and cross-builds under `nix13 -Werror`)
- [x] 1.1.3 Add `windowFill`, `windowDraw`, and `windowClear` to `stdlib/std.amiga.graphics.clo` and `windowText` to `stdlib/std.amiga.text.clo`; correct the `setColor` doc to the 0–31 playfield/sprite range; add the window-RastPort shim helpers; update the three export tables ✅ (completed: 2026-11-05 10:58; verified: window drawing/text program emits C and cross-builds under `nix13 -Werror`)
- [x] 1.1.4 Add `WindowEvent`, `EVENT_*`, `waitEvent`, `pollEvent`, and the seven event accessors to `stdlib/std.amiga.intuition.clo`; add the IDCMP loop, `GZZMouseX`/`GZZMouseY` mapping, and refresh auto-ack to the shim; run ST-1–ST-3 green ✅ (completed: 2026-11-05 10:59; `amiga_intuition_spec_test` 5/5 green, both cross-builds pass with `-Werror`)
- [x] 1.1.5 Add the window/event implementation cases (IT-1 window/event part, IT-2, IT-3, IT-6 window/event part, IT-8 window part) to `compiler/tests/amiga_intuition_impl_test.rs`; run them green ✅ (completed: 2026-11-05 10:59; 5/5 impl cases green)

**Verify**: `cargo test --test amiga_intuition_spec_test --test amiga_intuition_impl_test`

---

## Phase 2: Buttons, string gadget, and menus

> **Reasoning**: high — classic 1.3 gadget construction, string-buffer ownership,
> and menu-strip reattachment.

### Step 2.1: Gadgets and menus

**Reference**: [03-02](03-02-gadgets-and-menus.md) · AR-2, AR-8, AR-10, AR-14–AR-17

- [x] 2.1.1 Add ST-6 to `amiga_intuition_spec_test.rs`: full gadget/menu surface on both profiles and cross-builds; the A500 cases ST-7–ST-10 are added to the demo/smoke in 4.1.1; run ST-6 red ✅ (completed: 2026-11-05 11:15; red evidence: 4 `E11102` unresolved-export failures)
- [x] 2.1.2 Add `addButton`, `addString`, and `windowString` to `stdlib/std.amiga.intuition.clo`; add the gadget node, `AddGList`/`RefreshGList`, `Image`/`Border` render pair, and `StringInfo` buffer to the shim; update `compiler/src/stdlib.rs` ✅ (completed: 2026-11-05 11:17; button/string symbols recognized; full surface green after the menu task)
- [x] 2.1.3 Add `addMenu` and `addMenuItem` to `stdlib/std.amiga.intuition.clo`; add the linked menu nodes, `SetMenuStrip`/`ClearMenuStrip`, `ItemAddress`, and the unlink-on-failure path to the shim; update the export table; run ST-6 green ✅ (completed: 2026-11-05 11:17; gadget spec 4/4 green, both cross-builds pass with `-Werror`)
- [x] 2.1.4 Add the gadget/menu implementation cases (IT-4, IT-5, IT-10, IT-1 gadget extension) to `amiga_intuition_impl_test.rs`; run them green ✅ (completed: 2026-11-05 11:18; impl 11/11 green; full spec 9/9 green)

**Verify**: `cargo test --test amiga_intuition_spec_test --test amiga_intuition_impl_test`

---

## Phase 3: Requester, alert, and pointer

> **Reasoning**: high — modal 1.3 requester/alert calls, chip-memory pointer
> ownership, and free-on-close.

### Step 3.1: Requesters, alerts, and pointer

**Reference**: [03-03](03-03-requesters-pointer.md) · AR-7, AR-13–AR-17

- [x] 3.1.1 Add ST-14 to `amiga_intuition_spec_test.rs`: full requester/alert/pointer surface on both profiles and cross-builds; the A500 cases ST-11–ST-13 are added to the demo/smoke in 4.1.1; run ST-14 red ✅ (completed: 2026-11-05 11:38; red evidence: 4 `E11102` unresolved-export failures)
- [x] 3.1.2 Add `askYesNo` and `showAlert` to `stdlib/std.amiga.intuition.clo`; add the 1.3 eight-argument `AutoRequest` (240x40) and the AlertMessage-formatted `DisplayAlert` shim with 28-byte bounded body copies; update `compiler/src/stdlib.rs` ✅ (completed: 2026-11-05 11:39; corrected after the Phase 3 review)
- [x] 3.1.3 Add `setPointer` to `stdlib/std.amiga.intuition.clo`; add the chip-memory arrow, `SetPointer`/`ClearPointer`, and the close free path; update the export table; run ST-14 green ✅ (completed: 2026-11-05 11:39; requester spec 4/4 green, both cross-builds pass with `-Werror`)
- [x] 3.1.4 Add the requester/alert/pointer implementation cases (IT-7, IT-9, IT-8 full, IT-10 full, IT-1/IT-6 requester extension) to `amiga_intuition_impl_test.rs`; run them green ✅ (completed: 2026-11-05 11:39; impl 14/14 green)

**Verify**: `cargo test --test amiga_intuition_spec_test --test amiga_intuition_impl_test`

---

## Phase 4: GUI demo, A500 acceptance, and closeout

> **Reasoning**: high — real KS1.3 interaction, empirical click calibration, and
> the programme closeout.

### Step 4.1: Demo, target proof, and closeout

**Reference**: [03-04](03-04-gui-demo-and-smoke.md) · AR-5, AR-18–AR-22

- [x] 4.1.1 Add `examples/amiga/gui.clo` with the A500 cases ST-4, ST-5, ST-7–ST-13 (poll drain, close gadget, button, string, menus, requester, pointer) and the negative checks; add `docker/scripts/a500-gui-smoke.sh`, the `sdk-a500-gui` Makefile target, and the `verify` leg; compile the demo on both hosted profiles and run the demo-shape implementation case (IT-11/IT-12) — done 2026-11-05; the interactive About path uses `askYesNo` because `DisplayAlert`'s private screen is not headlessly focusable
- [x] 4.1.2 Run `make sdk-a500-gui` on the FS-UAE A500/KS1.3, fix target-only defects, capture the native `docs/amiga-sdk/a500-gui.png`, and pass ST-4, ST-5, ST-7–ST-13, ST-15, ST-16 — done 2026-11-05; private Xvfb keyboard run, ADF handshake, `gui: ok win 1 gad 2 menu 2 str 3 req 2`; no alert golden (see AR-25)
- [x] 4.1.3 Update `spec/appendix-amiga.md`, `spec/appendix-stdlib.md` (including the `setColor` 0–31 range), `AGENTS.md`, `docs/amiga-sdk/registry.tsv` (001–005/009 done with acceptance/evidence/spec columns, 006–008 deferred), `bundles.tsv` (status and plan path), generate `coverage.md` with `make sdk-registry`, update issue #7, `plans/amiga-sdk/00-index.md`, `plans/00-roadmap.md`, and `codeops/specialist-check.json` ✅ (completed: 2026-11-05 17:02; user ruling recorded in AR-26: one classic 1.3 surface documented on both hosted profiles; `spec/check.sh` OK; registry, bundles, coverage, AGENTS, index, and roadmap were already current; issue #7 closes in 4.1.4)
- [~] 4.1.4 Run `make check`, `make verify`, and `make sdk-registry` (ST-17 and ST-18); regenerate the issue body; close #7; commit through the active exec-plan commit mode; confirm a clean tree with no decrypted ROM — `make check`, `make sdk-a500-gui`, and `make sdk-registry` are green; the full `make verify` leg and issue closeout remain

**Verify**: `make check`; `make verify`; `make sdk-registry`; `make sdk-a500-gui`

---

## Dependencies

```
Phase 1 (window/events)
    ↓
Phase 2 (gadgets/menus)
    ↓
Phase 3 (requesters/pointer)
    ↓
Phase 4 (demo/A500/closeout)
```

- Phase 1 has no dependency beyond the existing custom-screen path.
- Phase 2 depends on the window handle and event loop.
- Phase 3 depends on the window handle and event loop.
- Phase 4 depends on all three phases, the licensed ROM/`rom.key`, the host X
  display, and the existing FS-UAE capture harness.

## Success Criteria

1. All 17 tasks complete.
2. The full surface compiles on both hosted profiles, the host rejects
   `std.amiga.intuition` with `E11200`, and both cross-builds pass with
   `-Wall -Wextra -Werror`.
3. The A500 smoke drives the real window, gadgets, menu, requester, alert, and
   pointer, and matches the held-frame, alert, and report-line goldens.
4. The report line is `gui: ok win 1 gad 2 menu 2 str 3 req 1`.
5. `make check`, `make verify`, and `make sdk-registry` pass; the six earlier
   A500 gates stay green.
6. Registry Bundle 6 is 6/9 done with SDK-INT-006/007/008 deferred; issue #7 is
   closed; the programme and roadmap show Bundle 6 done.
7. No language, grammar, diagnostic, or ABI change.

## Completion

In progress: Phases 1–3 are complete and green; Phase 4's demo and A500 gate pass. The remaining closeout items are the frozen-spec appendix ruling, the full `make verify` leg, and issue #7.
(A500 input and menu-bar acceptance).
