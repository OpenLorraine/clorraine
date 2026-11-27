# Borrowed regions and guide messages: Amiga shim crash-safety

> **Document**: 03-03-borrowed-objects.md
> **Parent**: [Index](00-index.md)

## Overview

The OS keeps two pointers past the borrow that handed them over: a layer's clip region and an
AmigaGuide message's context. The shim makes both safe when the program releases the owner first
(FR-4, FR-5).

## Registry walk helper

`cl_res_each(void (*release)(void *), void (*visit)(void *handle, void *context), void *context)`
in the base shim, next to `cl_own_ports_hold` (`shim_hosted_amiga.c:3409`): calls `visit` for each
live record whose release function is `release`, oldest first, saving the successor before each
call (AR-7). External linkage so the symbol check (ST-3) sees it; nothing on 1.3 calls it, so
section GC drops it there (AR-15).

## Regions

### Proposed Changes

- **`cl_layer`** gains `UBYTE updating`. `cl_layer_begin_update` sets it when `BeginUpdate`
  returns TRUE; `cl_layer_end_update` clears it after `EndUpdate` (AR-8). A static
  `cl_layer_unclip(cl_layer *)` ends an open update (`EndUpdate(layer, FALSE)`, flag cleared) and
  then calls `InstallClipRegion(layer, NULL)` when `ClipRegion` is non-NULL.
- **`cl_layer_free`** calls `cl_layer_unclip` before `DeleteLayer` (AR-6, layers autodoc). On 1.3
  `ClipRegion` is always NULL, so only the update check can do anything there.
- **`cl_free_region`** (hosted-amiga) calls `cl_res_each(cl_layer_free, visit, region)`, where
  `visit` runs `cl_layer_unclip` on each layer whose `Layer.ClipRegion` is this region, then
  disposes as today (AR-6, AR-7).
- **`cl_layer_set_clip`/`cl_layer_clear_clip`** end an open update before
  `InstallClipRegion` (AR-8). The install result stays ignored: the layer's `ClipRegion` field is
  what the walk reads (AR-7).

### Exit order

Newest first: a region created after its layer is freed first and unclips the layer; a layer
created after its region is freed first and unclips itself. Both run the paths above (AR-6).

## AmigaGuide messages

### Proposed Changes

- **`cl_guide_message`** gains `cl_guide *guide`, set in `cl_guide_wait` (AR-9).
- **`cl_guide_close`**, before `CloseAmigaGuide`: `cl_res_each(cl_guide_reply, visit, guide)`,
  where `visit`, for a message whose `guide` is this guide and whose `message` is non-NULL, calls
  `ReplyAmigaGuideMsg(message->message)` and clears `message` and `guide`. The handle stays
  registered (an orphan).
- **Orphans:** the getters already return 0 for a NULL message, and `cl_guide_reply` already skips
  the reply and frees the handle (`optional_net.c:283-319`); no change there.

### Exit order

Messages are newer than their guide, so the exit hook replies them before closing it; the close
then finds no held message.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Region freed while installed in one or more layers | each layer unclipped, then dispose | AR-6, AR-7 |
| Layer freed with a clip installed | unclip, then `DeleteLayer` | AR-6 |
| Unclip during an open update | `EndUpdate(layer, FALSE)` first | AR-8 |
| `InstallClipRegion` fails (pre-V45, undetectable) | the walk reads `ClipRegion`, so state stays consistent | AR-7 |
| Guide closed with held messages | replied, handles orphaned | AR-9 |
| Getter or reply on an orphan | 0 / frees the handle only | AR-9 |

## Testing Requirements

- Spec: ST-7 (region), ST-8 (guide) on A1200; the existing `sdk-a500-text` and
  `sdk-a500-assets` gates guard the 1.3 layer path (ST-11).
- Impl: `cl_layer_free` unclips before `DeleteLayer`; `cl_free_region` walks before disposing; the
  update flag; the guide close replies before `CloseAmigaGuide` and orphans.
