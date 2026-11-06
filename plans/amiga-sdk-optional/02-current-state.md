# Current State: Amiga SDK Bundle 9 — Optional Add-ons and Closeout

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)
> **Analysis date**: 2026-11-06

## Existing Implementation

### What Exists

| Area | Current state | Evidence |
|---|---|---|
| Profiles | `hosted-amiga` accepts every registered module; `hosted-amiga-13` has an explicit 1.3 allowlist; host/freestanding reject Amiga modules | `compiler/src/profile.rs:85-118` |
| Emission | Shared 1.3-safe shim, then a 2.0 suffix appended after the generated program | `compiler/src/profile.rs:15-18,129-139`; `compiler/src/codegen.rs:497-513` |
| Exec | Ports, messages, tasks, libraries, generic devices, signals | `stdlib/std.amiga.exec.clo`; `compiler/runtime/shim_hosted_amiga.c:2671-3087` |
| Intuition | Screens, windows, classic gadgets, menus, IDCMP, requesters, pointer | `stdlib/std.amiga.intuition.clo`; `compiler/runtime/shim_hosted_amiga.c:1110-2641` |
| Text | `openFont`, `openDiskFont`, measure/draw; diskfont path implemented but deferred for media | `stdlib/std.amiga.text.clo:17-91`; `shim_hosted_amiga.c:5387-5424` |
| Audio/input | Audio playback, joystick decode/read | `stdlib/std.amiga.audio.clo`; `stdlib/std.amiga.input.clo` |
| Layers | Layer lifecycle, fill/clear/text, begin/end update | `stdlib/std.amiga.layers.clo`; `shim_hosted_amiga.c:5346-5705` |
| DOS | Delay/ticks, file seek/length, lock, path/volume info, directories | `stdlib/std.amiga.dos.clo` |
| Workbench | AppIcon/AppWindow/AppMenuItem/AppMessage, icon library, expansion/card/keymap | `stdlib/std.amiga.workbench.clo`, `icon.clo`, `expansion.clo`; `shim_hosted_amiga_20.c` |
| Gates | A500 ADF gates, A600/KS2.05 and A1200/KS3.1 directory-HD gates, Workbench fixtures | `docker/scripts/a500-*.sh`, `a600-services-smoke.sh`, `a1200-workbench-smoke.sh` |
| Media | Licensed Amiga Forever Workbench ADFs/HDFs under `~/amiga`; container mount `/amiga:ro` | `docker/.env:7`; `docker/compose.yaml:19,32` |

### Relevant Files

| File | Role | Changes Needed |
|---|---|---|
| `stdlib/std.amiga.text.clo` | Diskfont path | No API change; runtime gate only |
| `stdlib/std.amiga.*.clo` | New modules | 19 new modules (AR-8) |
| `compiler/runtime/shim_hosted_amiga.c` | Shared 1.3-safe shim | Clipboard guard; event-id fallback |
| `compiler/runtime/shim_hosted_amiga_20.c` | Existing 2.0 suffix | Unchanged |
| `compiler/runtime/shim_hosted_amiga_optional.c` | New optional suffix | All 2.0+ Bundle 9 C code |
| `compiler/src/profile.rs` | Suffix concatenation | Join 2.0 and optional suffixes |
| `compiler/src/stdlib.rs` | Source/export tables | Register 19 modules |
| `compiler/tests/amiga_optional_spec_test.rs` | New spec suite | Compile/reject/cross-build/appendix cases |
| `compiler/tests/amiga_optional_impl_test.rs` | New impl suite | Shim/ownership/drop/gate pins |
| `examples/amiga/*.clo` | New demos | Per-capability acceptance programs |
| `docker/scripts/workbench-media.sh` | New media helper | Staging, case, assigns, `AddDataTypes` |
| `docker/scripts/a500-diskfont-smoke.sh` | New gate | A500/KS1.3 diskfont |
| `docker/scripts/a600-addons-smoke.sh` | New gate | A600/KS2.05 services/intuition |
| `docker/scripts/a600-ks31-addons-smoke.sh` | New gate | A600/KS3.1 lowlevel/nonvolatile |
| `docker/scripts/a1200-ks30-addons-smoke.sh` | New gate | A1200/KS3.0 BOOPSI/pool |
| `docker/scripts/a1200-addons-smoke.sh` | New gate | A1200/KS3.1 datatypes/network/guide |
| `Makefile` | Build/gate wiring | New targets and `verify` legs |
| `docs/amiga-sdk/registry.tsv` | Registry | Reassign 20 rows; baselines/status/evidence |
| `docs/amiga-sdk/bundles.tsv` | Bundle row | Bundle 9 floor 1.3/OCS, done + plan path |
| `docs/amiga-sdk/coverage.md` | Generated view | Regenerate |
| `spec/appendix-stdlib.md`, `spec/appendix-amiga.md` | Frozen appendices | Additive updates only |
| `docs/known-limitations.md`, `AGENTS.md` | Limits and workflow | Update media/network notes |

## Empirical Media Analysis

The Amiga Forever disc contains the full Workbench media sets. The following were copied to
`~/amiga` and checksum-verified against the source:

| Media | Files | Use |
|---|---|---|
| Workbench 1.3.5 | Installed HDF + 1.3.4 ADFs | `diskfont.library`, `diamond.font`; the gate uses the HDF |
| Workbench 2.04 | Workbench/Extras/Fonts/Install ADFs | ASL, commodities, ARexx, clipboard, serial/parallel/printer |
| Workbench 2.1 | Workbench/Extras/Fonts/Install/Locale ADFs | `locale.library`, 2.1 baseline |
| Workbench 3.0 | Workbench/Extras/Fonts/Install/Locale/Storage ADFs | BOOPSI classes, pool/datatypes-era support |
| Workbench 3.1 | Workbench/Extras/Fonts/Install/Locale/Storage ADFs | lowlevel, datatypes, AmigaGuide, nonvolatile |
| Installed HDFs | `workbench-135`, `-211`, `-311` | Convenience bootable volumes |

Key contents verified with `xdftool`:

- `amiga-os-134-workbench.adf`: `Libs/diskfont.library`, `Fonts/diamond.font` sizes 12/20.
- `amiga-os-204-workbench.adf`: `Libs/asl.library`, `commodities.library`, `rexxsyslib.library`;
  `Devs/clipboard.device`, `serial.device`, `parallel.device`, `printer.device`.
- `amiga-os-210-workbench.adf`: `Libs/locale.library`.
- `amiga-os-300-workbench.adf`: `Libs/datatypes.library`, `amigaguide.library`,
  `Classes/DataTypes/ilbm.datatype`, `Classes/Gadgets/colorwheel.gadget`.
- `amiga-os-310-workbench.adf`: `Libs/lowlevel.library`, `datatypes.library`,
  `amigaguide.library`, `nonvolatile.library`, `Classes/DataTypes`, `Classes/Gadgets`,
  `C/AddDataTypes`, `Devs/DataTypes`.
- `nonvolatile.library` appears only in the 3.1 Workbench ADF; the row is 3.1.
- `datatypes.library`’s object API (`NewDTObjectA`) is V40; the row is 3.1.
- `CreatePool`/`AllocPooled` are V39; the pool module and `SDK-EXE-007` are 3.0.
- `colorwheel.gadget` exists on 3.0 and 3.1, but its `WHEEL_Hue`/`WHEEL_HSB` set/get returned 0 in runtime probes; the BOOPSI acceptance uses the built-in `strgclass`, whose `STRINGA_LongVal` set/get round-trips. [AR-16]

FS-UAE 3.1.66 includes an emulated `bsdsocket.library` (`bsdsocket_library=1`). The guest
`connect()` is proxied to the host `connect()`, so the gate uses `127.0.0.1` with
`--network host`, not the slirp alias. A probe opened version 4 and created a TCP socket. [AR-6,
AR-28]

## Gaps Identified

### Gap 1: No 2.0+ Exec/utility/pool surface

No semaphores, hooks, utility tags/date, or OS pool wrappers exist. [AR-11, AR-12, AR-13]

### Gap 2: No gadtools/ASL/BOOPSI

The intuition module stops at classic gadgets/requesters; the advanced libraries are absent from
stdlib and the shim. [AR-14, AR-15, AR-16]

### Gap 3: No ReadArgs/datatypes/regions/EClock/lowlevel

The existing DOS/text/layers/audio/input modules cover only their 1.3-safe surfaces. [AR-17–AR-21]

### Gap 4: No Workbench-disk services

Commodities, locale, ARexx, clipboard, typed serial/parallel/printer, and nonvolatile are absent;
their media is now present. [AR-22–AR-27]

### Gap 5: No network/AmigaGuide

No socket or AmigaGuide module exists. [AR-28, AR-29]

### Gap 6: Registry/tracking mismatch

Issue #10 lists only the three `9-optional` rows; 18 open rows live in older bundles. The closeout
reassigns all 20 open rows and regenerates the issue. [AR-3]

## Dependencies

### Internal Dependencies

- Existing shim handles/ownership patterns (`cl_device`, `cl_timer`, `cl_layer`, `cl_app_icon`).
- Existing test harness (`ScratchDir`, `cross_build`, `shim()`, `assert_caller_cleanup`).
- Existing gate scripts and `workbench-start.c`.
- `xdftool`/`amitools` in the SDK image.

### External Dependencies

- Licensed Amiga Forever Workbench media under `~/amiga` (read-only).
- FS-UAE 3.1.66 with `bsdsocket_library`, `serial_port`, and `parallel_port` options.
- `m68k-amigaos-gcc` and the NDK headers in the SDK image.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Extended device request overrun | Low | High | Reject clipboard only; printer uses `union printerIO`; typed handles |
| Hook callback lifetime | Medium | High | Capture-less scalar callback; no state beyond `h_Data`; audited exception |
| Pool block/pool lifetime | Medium | High | Refcounted pool record; safe `u8[]` read/write; gate both free orders |
| Printer gate cannot prove raw output | Medium | Medium | Gate open/close; `PRD_RAWWRITE` is impl-tested; do not overclaim |
| BOOPSI object/window lifetime | Medium | Medium | Attach/detach + docs: object before window; gate follows |
| ASL modal input is flaky headlessly | Medium | Medium | `asl-ready`/`asl-done` handshakes; xdotool focus/retry like A500 GUI |
| Network host is not reachable | Low | High | `--network host` + host `127.0.0.1` echo; probe before gate |
| AmigaGuide async message shape | Low | Medium | Keep `NewAmigaGuide` alive; check message results; close |
| Media extraction path/case | Low | Medium | Stage, copy named trees, remove `S/`, explicit `LIBS: ... ADD Classes` |
| Scope too large for one plan | Medium | Medium | Phase by group; each phase independently green; reviews before closeout |
