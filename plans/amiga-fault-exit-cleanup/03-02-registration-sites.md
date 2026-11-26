# Registration sites: Amiga fault-exit cleanup

> **Document**: 03-02-registration-sites.md
> **Parent**: [Index](00-index.md)

## Overview

This document owns the per-kind table: which handle kinds register, how (embedded node **E** or
side record **S**), and where each record is removed. The mechanism is
[03-01](03-01-exit-registry.md). Line numbers are at HEAD `1462063e`; the executor re-locates by
function name.

## Rules applied to every row

- **Register after success.** If registration fails, undo the acquisition and return the
  existing failure (AR-9 rule 4).
- **Each kind gets one thunk**, `static void cl_release_<kind>(void *)`, which calls the existing
  release logic (AR-9 rule 2, AR-20).
- **Storage.** **E** applies when the handle is a `cl_*` wrapper the shim allocates; the node is
  a new `cl_res` member. **S** applies when the handle is a raw OS pointer (AR-8).
- **Remove where the resource is really freed or handed off**, not in a user-facing close that
  can defer (AR-9 rule 3).
- **No separate records for internals.** Sub-resources a wrapper owns (its private port, IORequest,
  buffer, gadgets, menus, title copy, pointer image, visual info) are released by the wrapper's
  release, as today. Acquisitions balanced within one call (temporary locks,
  `LockPubScreen`/`UnlockPubScreen` in `_20.c:490/495`, `card.resource`) stay as they are.
- **Excluded:** `File` (`std.io`, libnix `fopen`) and malloc-backed values (`Buffer`, `String`),
  which libnix releases after the hook (AR-9 rule 5). `Task` uses the worker phase
  ([03-03](03-03-worker-faults.md)), not the registry (AR-8).

## Base shim — both hosted profiles (`shim_hosted_amiga.c`)

| Kind (`std.amiga.*`) | Acquire (fn :line) | Release / remove at | Store | Notes |
|---|---|---|---|---|
| Screen (`intuition`) | `cl_open_screen_mode` :1335 | `cl_close_screen` :1393 | S | sprites detach first (existing :1397) |
| Window (`intuition`) | `cl_open_window` :1504, `cl_open_workbench_window` :1588 | `cl_window_destroy` :1670 (not `cl_close_window`; the close defers while `console_refs`/`ui_refs` > 0, :1709-1712) | E (`cl_window`) | public-screen lock released inside destroy (:1685) |
| MsgPort (`exec`) | `cl_create_msg_port` :3000, `cl_create_public_port` :3011 | `cl_delete_msg_port` :3015 | S | in-flight rule ([03-01 §Messages in flight](03-01-exit-registry.md#messages-in-flight)) (AR-15, AR-21) |
| NewMessage (`exec`) | `cl_create_message` :3025 | `cl_free_message` :3082; **hand-off** `cl_put_msg` :3046 moves it to the in-flight set | E (`cl_message`) | AR-9 rule 3 |
| Message (`exec`, received) | `cl_get_msg` :3065 (also leaves the in-flight set) | `cl_reply_msg` :3072, `cl_release_message` :3091; replying a program-created message is a **hand-off** back into the in-flight set ([03-01 §Messages in flight](03-01-exit-registry.md#messages-in-flight)) | S | same pointer as an earlier NewMessage uses a different thunk (AR-20) |
| Library (`exec`, user) | `cl_open_library` :3247 | `cl_close_library` :3251 | S | separate from the library-base table |
| Device (`exec`) | `cl_open_device` :3392, `cl_open_console` :3419 | `cl_device_close` path of `cl_close_device` :3428 | E (`cl_device`) | pending I/O aborted in release (existing :3361-3364); see [03-04](03-04-ctrl-c.md) for `pending` |
| Lock (`dos`) | `cl_dos_lock` :223 | `cl_dos_unlock` :232 | S | [03-01 Example 1](03-01-exit-registry.md#example-1-a-raw-pointer-kind-dos-lock) |
| PathInfo, VolumeInfo (`dos`) | `cl_dos_pathinfo_new`, `cl_dos_volumeinfo_new` | `cl_dos_pathinfo_free` :272, `cl_dos_volumeinfo_free` :372 | E | |
| ChipBuffer, Bitmap (`video`) | `cl_video_chip_alloc` :3700, `cl_video_bitmap_new` :3830 | `cl_video_chip_free` :3710, `cl_video_bitmap_free` :3847 | E | |
| Copper (`video`) | `cl_video_copper_new` :4286 | `cl_video_copper_free` :4312; **hand-off** `cl_video_display_set_copper` :4519 (success and failure paths) | E | AR-9 rule 3 |
| Display (`video`) | `cl_video_display_open_at` :4452 | `cl_video_display_free` :4614 | E | `LoadView(old)` restores the system view |
| Sprite (`sprite`) | `cl_sprite_new` :4800 | `cl_sprite_free` :4834 | E | restores the SPRITES flag when the last attached sprite of a screen goes ([03-01 §Chip state](03-01-exit-registry.md#chip-state)) (AR-14) |
| SpriteAnimation (`sprite`) | `cl_sprite_animation_new` :5003 | `cl_sprite_animation_free` :5053 | E | |
| Audio, Sample (`audio`) | `cl_audio_open` :5246, `cl_sample_new` | `cl_audio_free` :5377, `cl_sample_free` :5531 | E | |
| Timer (`timer`) | `cl_timer_open` :5589 | `cl_timer_free` :5642 | E | gains abort-before-close ([03-04](03-04-ctrl-c.md)) |
| Font (`text`) | `cl_font_open` :5725, `cl_font_open_disk` :5748 | `cl_font_free` :5762 | E | `defaultFont` (borrowed, `owns=0`) is not registered |
| Layer (`layers`) | `cl_layer_new` :5864 | `cl_layer_free` :5891 | E | |

## 2.0+ suffix — `hosted-amiga` only (`shim_hosted_amiga_20.c`)

| Kind | Acquire | Release / remove at | Store |
|---|---|---|---|
| DiskObject (`icon`) | `GetDiskObject` :29, `GetDefDiskObject` :55 | `cl_free_disk_object` :42 | S |
| AppIcon (`workbench`) | `cl_add_app_icon` (:159) | `cl_remove_app_icon` :174 | E (`cl_app_icon`) |
| AppMenuItem (`workbench`) | :218 | `cl_remove_app_menu_item` :231 | E |
| AppWindow (`workbench`) | :470 | `cl_remove_app_window` :474 | S |
| AppMessage (`workbench`, received) | `GetMsg` :256 | `cl_reply_app_message` :296 | S |

## Optional suffixes — `hosted-amiga` only

| File | Kinds (release fn :line) | Store |
|---|---|---|
| `_optional.c` | Semaphore (`cl_free_semaphore` :53), TagList (:122), DateStamp (:222), Hook (:330), UtilityBlock (:449), UtilityPool (:468), Args (:690), EClock (:827), Region (:938), DTObject (:1144) | E |
| `_optional_ui.c` | GadgetList (`cl_gt_free` :258), FileRequester (`cl_asl_free` :488), BoopsiObject (`cl_boopsi_dispose` :622) | E |
| `_optional_services.c` | Commodity (:118), Locale (:164), RexxMessage (`cl_rexx_delete` :410; **hand-off** `cl_rexx_send` :462-472 frees the wrapper and the packet moves to `cl_rexx_lifetimes`), Clipboard (:641), Serial (:781), Parallel (:873), Printer (:940), NonvolatileData (:1012) | E |
| `_optional_net.c` | Socket (`cl_socket_close` :129), AmigaGuide (:211), GuideMessage (`cl_guide_reply` :291) | E |
| `_optional_rtg.c` | RtgScreen (`cl_rtg_close_screen` :193; a failed `p96CloseScreen` keeps today's leak), RtgBitmap (:293) | E |

- `ObtainSemaphore` state is not a resource: the semaphore is private, and workers cannot block,
  so freeing an obtained semaphore at exit has no waiter.
- ARexx packets still awaiting a reply follow [03-01 §Messages in flight](03-01-exit-registry.md#messages-in-flight).
  Replied packets queued on the program's own reply port are freed (AR-15, AR-21).

## Library bases

Every site below calls `cl_lib_open(&Base, name, version)` instead of `OpenLibrary`
([03-01 §Library bases](03-01-exit-registry.md#library-bases)) (AR-9 rule 9).

| File | Bases (open site :line) |
|---|---|
| `shim_hosted_amiga.c` | `GfxBase` (:1307, :1604, :3682), `IntuitionBase` (:1311, :1600), `DiskfontBase` (:5742), `LayersBase` (:5859) |
| `_20.c` | `IconBase` :18, `WorkbenchBase` :88, `ExpansionBase` :324, `KeymapBase` :423 |
| `_optional.c` | `UtilityBase` :28 (recorded only where the shim opens it), `LowLevelBase` :853, `GfxBase` :911, `LayersBase` :914, `DataTypesBase` :1076 |
| `_optional_ui.c` | `GadToolsBase` :38, `AslBase` :46 |
| `_optional_services.c` | `CxBase` :76, `LocaleBase` :145, `RexxSysBase` :253, `NVBase` :985 |
| `_optional_net.c` | `SocketBase` :48, `AmigaGuideBase` :162 |
| `_optional_rtg.c` | `P96Base` :35, `GfxBase` :38 |

`TimerBase` (`_optional.c:763`) is copied from an open device request and is closed with that
device; it is not a library base.

## Interrupt servers

`cl_add_handler`/`cl_add_port_handler` keep their static slots and install the hook on first use;
phase 1 removes a live server ([03-01 §Exit phases](03-01-exit-registry.md#exit-phases)). They
add no registry record (AR-8).

## Testing Requirements

- Spec: ST-1..ST-6 and ST-3b cover representative kinds on `vamos` (Lock, PathInfo, MsgPort,
  NewMessage, a replied own Message, the user `Library`). ST-10..ST-13, ST-15, and ST-17 cover the FS-UAE kinds (servers, Display,
  Window and its console `Device`, Font and the diskfont base, Timer). ST-19 covers the optional suffix kinds on `vamos`
  ([07](07-testing-strategy.md)).
- Impl: one shape test per file that every acquire site in the table is followed by its
  registration and every listed remove point unregisters
  ([07 §Implementation Tests](07-testing-strategy.md#implementation-tests-edge-cases-internals)).
