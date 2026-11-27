# Current State: Amiga shim crash-safety

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

All line numbers are at HEAD `9cef1491` (after `amiga-fault-exit-cleanup`). The assessment
baseline `e24a8752` had the same defects at the lines its findings cite.

## Existing Implementation

### What Exists

- **Library bases.** The shim defines its own `GfxBase` and `IntuitionBase`
  (`shim_hosted_amiga.c:1620-1621`), so libnix does not auto-open them. Bases are opened through
  `cl_lib_open` (`:355-373`), which records the bases it opened in a 20-slot table (`:346`) that
  the exit hook closes. `IntuitionBase` is opened only by `cl_open_screen_mode` (`:1643-1644`) and
  `cl_open_workbench_window` (`:1949-1950`).
- **Exit registry.** Every owned handle has a record; the exit hook releases them newest first
  (`:6950-7019`). `cl_own_ports_hold` (`:3409-3420`) walks the registry by release function.
  Layers (`:6758,6781`), regions (`optional.c:992`), guides (`optional_net.c:215`), guide
  messages (`optional_net.c:276`), and clipboards (`optional_services.c:742`) are registered.
- **Screen text.** `cl_text_draw` (`:6695-6713`) sets the font and pen on `&screen->RastPort`,
  then `Move`/`Text`, and restores the font. Screens come only from a tagless `OpenScreen`
  (`:1638-1670`), so their bitmaps are planar. The shim never calls `SetDrMd`, `SetBPen`, or
  `SetSoftStyle`.
- **Layers and regions.** `cl_layer` (`:6562-6570`) holds the `Layer`, its own `Layer_Info`, sizes,
  and its record. `cl_layer_free` (`:6789-6813`) calls `DeleteLayer`, then `DisposeLayerInfo`.
  `cl_layer_begin_update`/`end_update` (`:6911-6929`) keep no state. Regions (hosted-amiga only):
  `cl_free_region` (`optional.c:999-1011`) disposes; `cl_layer_set_clip`/`clear_clip`
  (`:1093-1113`) install the region or NULL and ignore the result.
- **AmigaGuide.** `cl_guide` and `cl_guide_message` (`optional_net.c:153-164`); `cl_guide_close`
  (`:222-240`) calls `CloseAmigaGuide`; the getters and `cl_guide_reply` (`:283-319`) read or reply
  the raw `AmigaGuideMsg`, and already treat a NULL message as 0 / no reply.
- **Clipboard.** `cl_clipboard_read` (`optional_services.c:796-821`): `CBD_CURRENTREADID`, one
  `CMD_READ` sized to the caller's buffer, bounded `CBERR_OBSOLETEID` retry, return `io_Actual`.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/runtime/shim_hosted_amiga.c` | base shim | `cl_show_alert`; `cl_text_draw`; `cl_layer` update flag and free; registry walk helper |
| `compiler/runtime/shim_hosted_amiga_20.c` | 2.0+ suffix | `cl_save_public_screen` |
| `compiler/runtime/shim_hosted_amiga_optional.c` | regions | `cl_free_region`, set/clear clip |
| `compiler/runtime/shim_hosted_amiga_optional_net.c` | AmigaGuide | message → guide link, close, orphan |
| `compiler/runtime/shim_hosted_amiga_optional_services.c` | clipboard | finish the read |
| `stdlib/std.amiga.{text,intuition,workbench,regions,amigaguide,clipboard}.clo` | doc comments | AR-16 |
| `docker/scripts/a1200-addons-smoke.sh` | A1200/KS3.1 gate | six probes, end marker, settle |
| `docker/scripts/a500-fault-cleanup-smoke.sh` | A500/KS1.3 gate | text-clip probe, capture compare |
| `docs/known-limitations.md` | limits | §4, §5, colour-font clip limit |

## Gaps Identified

### Gap 1: Intuition through a NULL base (DEF-587)
**Current Behavior:** `cl_show_alert` (`:2962-2988`) calls `DisplayAlert`, and
`cl_save_public_screen` (`_20.c:532-547`) calls `LockPubScreen`, with whatever `IntuitionBase` is.
Before any screen or window is opened it is NULL. Reproduced in planning: under
`vamos -O intuition.library=mode:off` both crash calling through A6=0 (rc 120); the verifier
reproduced `savePublicScreen` on A1200/KS3.1 and `showAlert` on A500/KS1.3 ("Software error -
task held").
**Required Behavior:** FR-1. **Fix Required:** [03-01 §Intuition](03-01-intuition-and-clipboard.md#intuition-on-demand).

### Gap 2: unclipped screen text (DEF-585)
**Current Behavior:** `Text()` on an unlayered `RastPort` is not clipped; a box outside the
bitmap writes past the bitplanes. The verifier's 200 off-screen calls hang an A1200
(`probes/WS5/verify-9.2.4/ttext.clo`).
**Required Behavior:** FR-2. **Fix Required:** [03-02](03-02-text-clipping.md).

### Gap 3: unfinished clipboard read (DEF-588)
**Current Behavior:** the read never goes past the end of the clip, so the device holds off writes.
On A1200/KS3.1, a write after a read on the same handle never returns; that trace's read returned
0 bytes (`plans/independent-assessment/verification/verdicts-9.2.4.md:31`).
**Required Behavior:** FR-3. **Fix Required:** [03-01 §Clipboard](03-01-intuition-and-clipboard.md#clipboard-read).

### Gap 4: borrowed OS objects outlive their owner (DEF-593)
**Current Behavior:** `freeRegion` disposes a region a layer still references; a layer is deleted
with a user clip installed (the layers autodoc says this MUST NOT happen); `closeAmigaGuide`
leaves message handles that later read or reply into the closed context. The verifier's region
probe showed no failure (memory not reused); the guide half was not attempted.
**Required Behavior:** FR-4, FR-5. **Fix Required:** [03-03](03-03-borrowed-objects.md).

## Dependencies

### Internal Dependencies
- The exit registry and hook from `amiga-fault-exit-cleanup` (done): record walk, newest-first
  release, `cl_lib_open`.
- `docker/scripts/cscr2png.py` and the ImageMagick `compare` already used by `sdk-a500-gui`.

### External Dependencies
- Licensed media under `~/amiga`: Kickstart 1.3 + Workbench 1.3.5 HDF (A500 gate); Kickstart 3.1
  A1200 ROM + Workbench 3.1 and Fonts ADFs (A1200 gate). Nothing new.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The NULL skip read does not end the read on the 3.1 device | Low | High | ST-5/ST-6 red/green; AR-10 reopen trigger |
| The clipped text differs from the unclipped rendering | Med | Med | ST-10 capture comparison; AR-4 reopen trigger |
| The superset box misses ink pixels | Low | High | union of every field reading (AR-5); impl test of the box rules |
| A new A1200 probe hangs and hides later probes | Med (red phase) | Low | probes run after the existing legs, each writes its own file (AR-13, AR-14) |
| A changed shim line breaks an impl-test pin | Med | Low | update only intended pins |
| Library table full | Low | Med | no new base (AR-2) |

### Out-of-scope observations

See [00-ambiguity-register §Out-of-scope observations](00-ambiguity-register.md#out-of-scope-observations-strict-scope-not-planned):
screen `fill`/`draw`, the failed clipboard write, the public-screen lock across a Ctrl-C, and a
layer outliving its bitmap. Reported, not planned (AR-1).
