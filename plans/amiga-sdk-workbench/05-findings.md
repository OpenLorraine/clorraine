# Findings: Amiga SDK Bundle 8 — Workbench and Services

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: In progress — discovery findings recorded; implementation and review follow

## Discovery findings

These are the facts the licensed media and the real 2.0/3.1 machines
established before planning. They drive the scope and the gate design.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-1 | 🟠 Major | The environment has licensed Kickstart ROMs but no Workbench ADF/HDF. `commodities.library`, `locale.library`, `rexxsyslib.library`, `clipboard.device`, `serial.device`, `parallel.device`, `printer.device`, and `nonvolatile.library` are absent from the KS2.05/KS3.1 ROM scans. | **Accepted** — SDK-WBK-003/004/005/006/007 are deferred and nonvolatile is split to SDK-WBK-010 (deferred), with the ROM evidence in 02-current-state; no wrapper is shipped without runtime proof. |
| RV-2 | 🟠 Major | FS-UAE host mouse/keyboard injection reaches the emulator's own F12 menu but not the emulated Workbench screen in the headless setup, so a double-click-driven AppMessage is not reliable as an automated gate. | **Accepted** — the live AppIcon is proven with a native Workbench capture; the AppMessage path is exercised by a test-only fixture that posts three real `struct AppMessage`s (icon/window/menu) to the demo's public port. This limitation is documented. |
| RV-3 | 🟢 Info | The ROM `workbench.library` private `StartWorkbench` vector (LVO -42) returns 1 and opens the `Workbench` public screen on KS2.05 and KS3.1. | **Used by the test harness only** — `workbench-start.c` mirrors `C:LoadWB`; the public SDK never exposes the vector. |
| RV-4 | 🟠 Major | The bare ROM has a default `KeyMap` object (`AskKeyMapDefault` non-null) but no loaded `DEVS:keymaps` layout: `MapRawKey` returned 0 for all raw codes 0..127. | **Accepted** — the keymap call is library-call verified; the demo reports `key 0`. A Workbench installation with `DEVS:keymaps` would return text. |
| RV-5 | 🟡 Minor | `RemoveAppIcon`/`RemoveAppWindow`/`RemoveAppMenuItem` return a BOOL whose high bits are unspecified (the probe printed values like `0x4efc`). | **Handled** — the wrapper removers are `void` and treat the object pointer as the ownership token; no caller inspects the BOOL. |
| RV-6 | 🟡 Minor | The 1.3 NDK lacks the 2.0 `clib/wb_protos.h`, `clib/keymap_protos.h`, `clib/cardres_protos.h`, and `resources/card.h` headers, so the shared shim cannot include them unconditionally. | **Handled** — a `CL_HOSTED_AMIGA_20` codegen define guards the 2.0-only shim section; the 1.3 build preprocesses it out and never links the 2.0 symbols. |
| RV-7 | 🟢 Info | `FindConfigDev` reports two Commodore boards on both models (manufacturer 2011, products 82 and 4), and `card.resource` is present on both with status 0. | **Used by the gates** — both gates pin `exp 2 man 2011 prod 82 card 1 status 0` in the `svc` line. |
| RV-8 | 🟠 Major | The `clib` stubs for `AddAppIconA`, `GetDefDiskObject`, `FindConfigDev`, `MapRawKey`, and `ReadCardStatus` use the NDK base globals (`WorkbenchBase`, `IconBase`, `ExpansionBase`, `KeymapBase`, `CardResource`), which `libstubs.a` only defines as zero. | **Fixed in the plan** — the 2.0 shim assigns each base from `OpenLibrary`/`OpenResource` before the call; the impl suite pins the assignments. |
| RV-9 | 🟠 Major | `CreatePort` does not copy the port name and `openScreen` does not copy the screen title; passing a temporary `toCString(stringOf(...))` leaves a dangling name. | **Fixed in the plan** — the demo declares `portName`/`screenTitle` `String`s before the port/screen; the AppIcon/AppMenuItem titles are copied into their records. |
| RV-10 | 🟠 Major | The original gate design proved the Workbench row only on KS3.1 while the row is min OS 2.0. | **Fixed in the plan** — the A600/KS2.05 gate runs the same Workbench and services sequence; the A1200 gate adds the capture comparison. |
| RV-11 | 🟡 Minor | A stale report, capture, or icon file could satisfy a poll after a failed run. | **Fixed in the plan** — both gates clear and rebuild the directory HD before every run and assert the start/ready handshakes. |

## Implementation findings

Added during execution.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| — | — | — | — |

## Closeout verification

| Gate | Result |
|---|---|
| `cargo test --lib --test amiga_workbench_spec_test --test amiga_workbench_impl_test` | pending |
| `make sdk-a600-services` | pending |
| `make sdk-a1200-workbench` | pending |
| `make check` | pending |
| `make verify` | pending |
| `make sdk-registry` | pending |
| GitHub issue #9 | pending |
