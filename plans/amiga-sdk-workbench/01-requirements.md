# Requirements: Amiga SDK Bundle 8 — Workbench and Services

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1–R10

## Feature Overview

Bundle 8 adds the advanced-tier Workbench and system-service surface that a
Kickstart 2.0+ machine can run from ROM. It provides Workbench application
objects (`AppIcon`, `AppWindow`, `AppMenuItem`) and the `AppMessage` reply path,
the icon library's `DiskObject` round-trip, expansion-board enumeration, the
`card.resource` status, and a raw-key keymap call. Two gates prove the surface
on an A1200/KS3.1 (AGA/68020/2M) and an A600/KS2.05 (ECS/68000/1M); both run
the Workbench and services programs, and the A1200 gate also compares the
native Workbench capture.

The Workbench-disk services (commodities, locale, ARexx, clipboard,
serial/parallel/printer) and `nonvolatile.library` are deferred with the
empirical ROM evidence in [02-current-state](02-current-state.md) and
[05-findings](05-findings.md). The raw-key mapping is runtime-proven with the
ROM default keymap (`key 1`).

## Functional Requirements

### Must Have

- **R1 — Workbench application objects.** `std.amiga.workbench` is available
  only on `hosted-amiga` (2.0+); `host`, `hosted-amiga-13`, and `freestanding`
  reject it with `E11200`. It exports:
  - `AppIcon`, `addAppIcon(borrow port: MsgPort, title: cstring, id: u32,
    userData: u32): AppIcon?`, `removeAppIcon(take icon: AppIcon): void`, and a
    generated drop. The shim copies the title into the AppIcon record and frees
    it on removal; the default `DiskObject` is owned until `RemoveAppIcon`. The
    icon is a `WBTOOL` default with `do_Type = 0`, positioned at `(400, 140)`
    with `WBAPPICONA_PropagatePosition`.
  - `AppWindow`, `addAppWindow(borrow window: Window, borrow port: MsgPort,
    id: u32, userData: u32): AppWindow?`, `removeAppWindow(take window:
    AppWindow): void`, and a generated drop. The window must outlive the
    AppWindow; the demo uses a window on its own custom screen. The row proves
    the AppWindow lifecycle; a Workbench-screen drop zone is out of scope.
  - `AppMenuItem`, `addAppMenuItem(borrow port: MsgPort, title: cstring,
    id: u32, userData: u32): AppMenuItem?`, `removeAppMenuItem(take item:
    AppMenuItem): void`, and a generated drop. The shim copies the title into
    the item record.
  - `AppMessage`, `waitAppMessage(borrow port: MsgPort): AppMessage?`,
    `appMessageType`, `appMessageId`, `appMessageUserData`, `appMessageClass`,
    `appMessageArgCount`, `replyAppMessage(take message: AppMessage): void`,
    and a generated drop that replies an unhandled message.
  - `savePublicScreen(name: cstring, path: cstring): bool`, which locks a
    public screen by name, writes the native `CSCR` capture, and unlocks it.
- **R2 — Icon library.** `std.amiga.icon` is available only on
  `hosted-amiga`; the other profiles reject it with `E11200`. It exports:
  - `DiskObject`, `getDiskObject(name: cstring): DiskObject?`,
    `putDiskObject(name: cstring, borrow icon: DiskObject): bool`,
    `freeDiskObject(take icon: DiskObject): void`, and a generated drop.
  - `getDefDiskObject(kind: i32): DiskObject?`, `diskObjectType(borrow icon):
    i32`, `setDiskObjectType(borrow icon, kind: i32): bool`.
  - `ICON_DISK` (1), `ICON_DRAWER` (2), `ICON_TOOL` (3), `ICON_PROJECT` (4),
    `ICON_GARBAGE` (5), `ICON_DEVICE` (6), `ICON_KICK` (7), `ICON_APPICON` (8).
  - `get`/`put` use the base name without `.info`. `putDiskObject` returns
    `false` on failure; the caller owns the file and its contents.
- **R3 — Expansion, card, and keymap.** `std.amiga.expansion` is available only
  on `hosted-amiga`; the other profiles reject it with `E11200`. It exports:
  - `expansionBoardCount(): i32`,
    `expansionBoardManufacturer(index: i32): u16`,
    `expansionBoardProduct(index: i32): u8`,
    `expansionBoardSerial(index: i32): u32`,
    `expansionBoardKind(index: i32): u8`.
  - `cardPresent(): bool`, `cardStatus(): u32`.
  - `mapRawKey(code: u16, qualifier: u16): String?`.
  - Out-of-range indexes return 0; a missing `card.resource` makes
    `cardPresent` false and `cardStatus` 0; a missing keymap library or an
    unmapped key makes `mapRawKey` null.
- **R4 — Public message port.** `std.amiga.exec` adds
  `createPublicPort(name: cstring): MsgPort?`, which creates a named port
  (`CreatePort(name, 0)`) and uses the existing `MsgPort` drop. `CreatePort`
  does not copy the name, so the caller keeps the `cstring` alive until the
  port is deleted. The name must be non-empty and unique; the caller drains the
  port before the drop.
- **R5 — Test fixtures.** Two C fixtures are test infrastructure, not public
  API:
  - `docker/scripts/workbench-start.c` opens `workbench.library`, calls the
    private `StartWorkbench` vector (LVO -42), waits for the `Workbench` public
    screen, and exits. It exists because `C:LoadWB` is on the unavailable
    Workbench disk.
  - `docker/scripts/workbench-post.c` starts the demo as an asynchronous
    process, finds the `ClorraineWbTest` public port, posts three
    `struct AppMessage`s (`AMTYPE_APPICON` id 7 user 42, `AMTYPE_APPWINDOW` id
    8 user 43, `AMTYPE_APPMENUITEM` id 9 user 44; class 0, no args), waits for
    the three replies, and writes `posted` to `CNEXT:wb-post.txt`.
  The public SDK never exposes the private vector or the synthetic messages.
- **R6 — Demos.** `examples/amiga/workbench.clo`:
  1. declares the `portName` and `screenTitle` `String`s and opens a 320x80x3
     screen and a 300x64 window;
  2. creates the public port `ClorraineWbTest`;
  3. adds the AppIcon `Clorraine`, the AppWindow on the custom window, and the
     AppMenuItem `Clorraine`;
  4. waits ten ticks for the AppIcon redraw and captures the Workbench public
     screen to `CNEXT:workbench.cscr`;
  5. writes `CNEXT:wb-ready.txt` containing `ready`;
  6. waits for three AppMessages;
  7. writes `CNEXT:wb.txt` with the pinned line
     `wb: ok icon 1 win 1 menu 1 types 8 7 9 ids 7 8 9 users 42 43 44 classes 0 0 0 args 0 0 0`;
  8. replies each message, removes the application objects, and closes the
     window and screen. The demo writes files only; it does not print, so the
     fixture's redirected stdout stays clean.
  `examples/amiga/services.clo` prints `svc: ok icon 3 exp 2 man 2011 prod 82
  card 1 status 0 key 1`; the gate redirects the output to `CNEXT:svc.txt`.
- **R7 — Gates.** `make sdk-a1200-workbench` builds the Workbench and services
  demos and the two C fixtures, boots an A1200/KS3.1 directory HD, runs
  `workbench-start`, `workbench-post` (which starts the Workbench demo), and
  `services`, polls `wb-start.txt`, `wb-ready.txt`, `wb.txt`, `wb-post.txt`,
  `wb-done.txt`, and `svc.txt`, converts the native capture with
  `docker/scripts/cscr2png.py`, and compares it to
  `docs/amiga-sdk/a1200-workbench.png`; it also compares the pinned AppIcon
  crop to `docs/amiga-sdk/a1200-workbench-icon.png`.
  `make sdk-a600-services` builds both demos and the fixtures, boots an
  A600/KS2.05 directory HD, runs the same Workbench and services sequence, and
  asserts `wb.txt`, `wb-post.txt`, and `svc.txt`; it does not compare a capture
  on 2.05. Both gates clear the HD before the run, use a private Xvfb display,
  and open no window on the user's desktop.

### Should Have

- **R8 — Errors, limits, and ownership.** Every public call is null-safe.
  `addAppIcon`/`addAppWindow`/`addAppMenuItem` return `null` on a null
  port/title/window, a missing library, or an allocation failure. The shim
  owns a copy of the AppIcon/AppMenuItem title and frees it on removal.
  `waitAppMessage` blocks on a non-null port and returns the next message; a
  null port returns `null`. `appMessage*` accessors return 0 for a null message.
  `replyAppMessage` and the generated drop reply exactly once. `remove*` is
  null-safe and consumes the handle. `getDiskObject`/`getDefDiskObject` return
  `null`; `putDiskObject` returns `false` for a null name or icon;
  `diskObjectType` returns 0 for null; `setDiskObjectType` rejects kinds outside
  1..8. `savePublicScreen` returns `false` for a null/unknown screen or an
  unwritable path. The AppWindow must be removed before its window closes.
- **R9 — Compatibility and gating.** The new modules compile on
  `hosted-amiga` only; the host, `hosted-amiga-13`, and freestanding profiles
  reject them with `E11200` and emit no C. The 2.0 baseline uses the V37
  functions (`OpenLibrary`, `GetDefDiskObject`, `AddAppIconA`, `AddAppWindowA`,
  `AddAppMenuItemA`, `MapRawKey`, `FindConfigDev`); `card.resource` is opened
  defensively because it is optional. The `WBAPPICONA_PropagatePosition` tag is
  V44 and is ignored on V37/V40, where `do_CurrentX`/`do_CurrentY` position the
  icon. The 1.3 A500 gates stay green; no language, grammar, diagnostic, or ABI
  change. The 2.0+ shim suffix is appended only for `hosted-amiga`; the 1.3
  emit does not contain it.
- **R10 — Documentation and tracking.** Add `spec/appendix-stdlib.md` §13.2
  (public port) and §14 (icon/Workbench/expansion); update
  `spec/appendix-amiga.md` with the advanced-tier surface and the media
  limitation; update `spec/15-profiles.md` (exec on both hosted profiles; the
  2.0+ modules on `hosted-amiga` only), `docs/known-limitations.md`,
  `AGENTS.md`, `docs/amiga-sdk/registry.tsv` (split `SDK-WBK-008` into
  expansion/keymap/card and the deferred `SDK-WBK-010` nonvolatile; 4 done,
  6 deferred), `bundles.tsv`, `coverage.md`, issue #9,
  `plans/amiga-sdk/00-index.md`, `plans/00-roadmap.md`, and
  `codeops/specialist-check.json`.

### Won't Have (Out of Scope)

- Commodities broker (`commodities.library` is not ROM-resident).
- `locale.library` (2.1; not ROM-resident).
- ARexx (`rexxsyslib.library` and RexxMast are Workbench media) and a public
  `FindPort` wrapper.
- Clipboard device (`clipboard.device` is not ROM-resident).
- Serial, parallel, and printer devices (not ROM-resident; their extended
  request types stay out of the generic device surface).
- `nonvolatile.library` (split to `SDK-WBK-010`, deferred; not ROM-resident).
- AppMessage argument names and locks, `WBStartup`, BOOPSI, gadtools, ASL,
  datatypes, and Workbench drag-and-drop interaction.
- A Workbench-screen AppWindow or drop zone; the AppWindow proof is lifecycle
  on the demo's custom window.
- Host mouse/keyboard injection into the emulated Workbench screen; the
  AppMessage path uses the fixture instead.
- Any language, grammar, diagnostic, or ABI change.

## Technical Requirements

### Performance

- The Workbench demo uses one public port, three application objects, three
  messages, and one 640x256 native capture. The demo allocates its screen, window, AppIcon record/title, and default
  `DiskObject`; the Workbench objects are small and the 1 MB A600 budget is
  covered by the A600 gate.
- The services demo performs one icon round-trip, one board enumeration, one
  card query, and one keymap call.

### Compatibility

- `workbench.library`, `icon.library`, `keymap.library`, and
  `expansion.library` are ROM-resident on KS2.05 and KS3.1.
- `StartWorkbench` is a private V37+ vector; the fixture is test-only and
  version-checked by the gate (`workbench.library` >= 37).
- `card.resource` is opened with `OpenResource`; absence is a valid result.
- The emitted C remains valid C99 and builds with `-mcrt=nix20` under
  `-Wall -Wextra -Werror`; the `hosted-amiga-13` build preprocesses the 2.0
  section out.

### Security

- Names, titles, and paths come from the program, not untrusted runtime input.
  The wrappers still validate null names, empty public-port names, and
  out-of-range indexes, and never index with an unchecked value.
- The AppMessage fixture is test-only and posts fixed, bounded messages.

## Acceptance Criteria

1. `compiler/tests/amiga_workbench_spec_test.rs` passes: the full surface
   compiles on `hosted-amiga`, the host and 1.3 profiles reject it with
   `E11200`, and the full surface cross-builds with `nix20`.
2. `compiler/tests/amiga_workbench_impl_test.rs` passes: emitted shim calls,
   ownership/drop routing, public-port creation, AppMessage accessors, icon
   round-trip shapes, expansion/card/keymap calls, the NDK base assignments,
   the 2.0 shim guard, and the demo/fixture shapes are pinned.
3. `make sdk-a1200-workbench` passes on the real A1200/KS3.1:
   `CNEXT:wb.txt` is
   `wb: ok icon 1 win 1 menu 1 types 8 7 9 ids 7 8 9 users 42 43 44 classes 0 0 0 args 0 0 0`,
   `CNEXT:wb-post.txt` is `posted`, `CNEXT:svc.txt` is the services line, and
   the native Workbench capture and AppIcon crop match
   `docs/amiga-sdk/a1200-workbench.png` and
   `docs/amiga-sdk/a1200-workbench-icon.png`.
4. `make sdk-a600-services` passes on the real A600/KS2.05 with the same
   `wb.txt`, `wb-post.txt`, and `svc.txt` lines; it does not compare a capture.
5. `make check`, `make verify`, and `make sdk-registry` pass; the earlier A500
   gates stay green.
6. Registry Bundle 8 is 4 done / 6 deferred with the media evidence (003–007
   and the split-out 010); issue #9 is closed; the programme index and roadmap
   show Bundle 8 done and Bundle 9 next.
7. No language, grammar, diagnostic, or ABI change.
