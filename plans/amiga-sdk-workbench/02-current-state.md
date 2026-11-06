# Current State: Amiga SDK Bundle 8 — Workbench and Services

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)
> **Analysis date**: 2026-11-05

## Existing Implementation

### What Exists

| Area | Current state | Evidence |
|---|---|---|
| Profiles | `hosted-amiga` (2.0+), `hosted-amiga-13` (1.3), `host` (tests), `freestanding` | `compiler/src/profile.rs` |
| Exec | Private ports, tasks, libraries, generic devices, signals | `stdlib/std.amiga.exec.clo`; `compiler/tests/amiga_exec_spec_test.rs` |
| Intuition | Classic screens, windows, gadgets, menus, IDCMP, requesters, pointer | `stdlib/std.amiga.intuition.clo`; `compiler/tests/amiga_intuition_spec_test.rs` |
| Text | `String`, `stringOf`, `toCString`, `bytes` | `stdlib/std.text.clo` |
| Public ports | Absent; `createMsgPort` creates a private port only | `stdlib/std.amiga.exec.clo` |
| Icon library | Absent | no `std.amiga.icon` module |
| Workbench | Absent | no `std.amiga.workbench` module |
| Expansion/card/keymap | Absent | no `std.amiga.expansion` module |
| A500 gates | Hello, Boing, sprites, paddle, text, assets, GUI, Exec | `Makefile`; `docker/scripts/a500-*-smoke.sh` |
| Advanced gate | `hosted-amiga` is compile/cross-build verified; no 2.0+ runtime gate exists | `spec/appendix-amiga.md` |

### Relevant Files

| File | Role |
|---|---|
| `stdlib/std.amiga.exec.clo` | Gains `createPublicPort` |
| `stdlib/std.amiga.workbench.clo` | New Workbench module |
| `stdlib/std.amiga.icon.clo` | New icon module |
| `stdlib/std.amiga.expansion.clo` | New expansion/card/keymap module |
| `compiler/runtime/shim_hosted_amiga.c` | Shared C shim implementations |
| `compiler/runtime/shim_hosted_amiga_20.c` | 2.0+ Workbench/icon/expansion suffix |
| `compiler/src/stdlib.rs` | Module source and export tables |
| `compiler/src/profile.rs` | `hosted-amiga`-only gating |
| `compiler/tests/amiga_workbench_spec_test.rs` | New specification suite |
| `compiler/tests/amiga_workbench_impl_test.rs` | New implementation suite |
| `examples/amiga/workbench.clo` | A1200 Workbench demo |
| `examples/amiga/services.clo` | A600 services demo |
| `docker/scripts/workbench-start.c` | Test-only Workbench start fixture |
| `docker/scripts/workbench-post.c` | Test-only AppMessage fixture |
| `docker/scripts/a1200-workbench-smoke.sh` | A1200/KS3.1 gate |
| `docker/scripts/a600-services-smoke.sh` | A600/KS2.05 gate |
| `docs/amiga-sdk/a1200-workbench.png` | Native Workbench capture golden |
| `docs/amiga-sdk/a1200-workbench-icon.png` | Live AppIcon crop golden |

## Empirical ROM and Media Analysis

The licensed Amiga Forever media in this environment contains Kickstart ROMs
but **no Workbench ADF/HDF**. The ROMs were decrypted locally with the
licensed `rom.key` and scanned with `romtool`; a probe program ran on FS-UAE
A1200/KS3.1 and A600/KS2.05. The evidence is reproducible from
`build/amiga-sdk/wb-probe/` (gitignored).

### ROM-resident libraries and devices

| Module | A600/KS2.05 | A1200/KS3.1 | Used by Bundle 8 |
|---|---|---|---|
| `workbench.library` | 37.132 | 40.5 | AppIcon/AppWindow/AppMenuItem, `StartWorkbench` |
| `icon.library` | 37.11 | 40.1 | `DiskObject` |
| `keymap.library` | 37.2 | 40.4 | `MapRawKey` |
| `expansion.library` | 37.50 | 40.2 | `FindConfigDev` |
| `utility.library` | 37.3 | 40.1 | not used here (Bundle 9) |
| `gadtools.library` | 37.110 | 40.4 | not used here (Bundle 9) |
| `card.resource` | present | present | `ReadCardStatus` |
| `commodities.library` | absent | absent | deferred |
| `locale.library` | absent | absent | deferred |
| `rexxsyslib.library` | absent | absent | deferred |
| `clipboard.device` | absent | absent | deferred |
| `serial.device` | absent | absent | deferred |
| `parallel.device` | absent | absent | deferred |
| `printer.device` | absent | absent | deferred |
| `nonvolatile.library` | absent | absent | deferred |

The ROM scans for KS3.1 A1200/A3000/A4000 and KS3.0 A1200/A4000 show the same
pattern: Workbench, icon, keymap, expansion, utility, and gadtools are
present; the Workbench-disk services are not. `workbench.task` is ROM-resident
on both baselines.

### Probe results

| Probe | A600/KS2.05 | A1200/KS3.1 |
|---|---|---|
| `OpenLibrary("workbench.library", 37)` | non-null | non-null |
| Private `StartWorkbench(0, 0)` (LVO -42) | returns 1 | returns 1 |
| `LockPubScreen("Workbench")` after start | non-null | non-null |
| `GetDefDiskObject(WBTOOL)` | non-null | non-null |
| `PutDiskObject("CNEXT:probe-icon", icon)` | 1 | 1 |
| `GetDiskObject("CNEXT:probe-icon")` | non-null, type 3 | non-null, type 3 |
| `FindConfigDev(NULL, -1, -1)` | 2 boards | 2 boards |
| Board 1 `er_Manufacturer/Product` | 2011 / 82 | 2011 / 82 |
| Board 2 `er_Manufacturer/Product` | 2011 / 4 | 2011 / 4 |
| `OpenResource("card.resource")` | non-null | non-null |
| `ReadCardStatus()` | 0 | 0 |
| `AskKeyMapDefault()` | non-null | non-null |
| `MapRawKey(0x20, 0)` after the `IECLASS_RAWKEY` fix | 1 mapped | 1 mapped |
| `AddAppWindowA` on a custom-screen window | non-null (`objects205`) | non-null (`objects31`) |
| `AddAppMenuItemA` | non-null (`objects205`) | non-null (`objects31`) |

The keymap call maps raw key 0x20 through the ROM default keymap once the
`InputEvent` is zeroed and its class is set to `IECLASS_RAWKEY`; the initial
probe left the event uninitialized and returned 0. The demo reports `key 1` on
both baselines.

### Toolchain and ABI facts

| Fact | Evidence |
|---|---|
| `CreatePort` does not copy the port name | amiga.lib autodoc; the caller keeps the name alive until the port is deleted |
| `openScreen` does not copy the title | `stdlib/std.amiga.intuition.clo`; the caller keeps the title alive until the screen is closed |
| `clib` stubs use base globals | `WorkbenchBase`, `IconBase`, `KeymapBase`, `ExpansionBase`, and `CardResource`; the shim must assign each from `OpenLibrary`/`OpenResource` before the call |
| 1.3 NDK headers | `clib/wb_protos.h`, `clib/keymap_protos.h`, `clib/cardres_protos.h`, and `resources/card.h` are absent (the 2.0 NDK has them); the 2.0 shim section is guarded |
| `PutDiskObject` | writes the `.info` file directly; a failed write is reported but the original is not guaranteed untouched |

### Workbench input limitation

FS-UAE's host mouse/keyboard injection reached the emulator's own F12 menu but
not the emulated Workbench screen in the headless setup, so a real
double-click-driven AppMessage is not reliable as an automated gate. The
bundle uses a test-only fixture to post three real `struct AppMessage`s
(icon/window/menu) to the demo's public port, and proves the live AppIcon with
a native `CSCR` capture and an AppIcon crop.

## Gaps Identified

### Gap 1: No Workbench module

There is no `AppIcon`, `AppWindow`, `AppMenuItem`, or `AppMessage` wrapper.

### Gap 2: No icon library

There is no `DiskObject` read/write/default/type surface.

### Gap 3: No expansion/card/keymap surface

There is no board enumeration, card status, or raw-key mapping.

### Gap 4: No public port

`createMsgPort` creates a private port, so a deterministic AppMessage fixture
has no rendezvous point.

### Gap 5: No 2.0+ runtime gate

`hosted-amiga` is only compile/cross-build verified. The advanced tier needs
real A600/KS2.05 and A1200/KS3.1 gates.

### Gap 6: The Workbench-disk services are unavailable

Commodities, locale, ARexx, clipboard, serial, parallel, printer, and
nonvolatile need Workbench media that is not present. They are deferred rather
than implemented without runtime proof.
