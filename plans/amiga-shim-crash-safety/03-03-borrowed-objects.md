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

- **`cl_layer`** gains `UBYTE updating`. `cl_layer_begin_update` sets it on every call, whatever
  `BeginUpdate` returns, because a FALSE result still needs `EndUpdate(l, FALSE)` (BeginUpdate
  BUGS). `cl_layer_end_update` still calls `EndUpdate(layer, TRUE)` as today, then clears the flag
  (AR-8). A static `cl_layer_unclip(cl_layer *)` calls `InstallClipRegion(layer, NULL)` when
  `ClipRegion` is non-NULL; it never ends an update.
- **`cl_layer_free`** (the program's free and the exit release) ends an open update first
  (`EndUpdate(layer, FALSE)`, flag cleared), then calls `cl_layer_unclip`, then `DeleteLayer`
  (AR-6, AR-8, layers autodoc). Only this path ends an update: after it the program holds no
  handle, so it cannot call `layerEndUpdate` again. On 1.3 `ClipRegion` is always NULL, so only the
  update check can do anything there.
- **`cl_free_region`** (hosted-amiga) calls `cl_res_each(cl_layer_free, visit, region)`, where
  `visit` runs `cl_layer_unclip` on each layer whose `Layer.ClipRegion` is this region, then
  disposes as today (AR-6, AR-7). It leaves an open update open: ending it would make the
  program's own `layerEndUpdate` unlock a layer that is no longer locked (EndUpdate: "The layer is
  then unlocked").
- **`cl_layer_set_clip`/`cl_layer_clear_clip`** are unchanged (AR-8). The install result stays
  ignored: the layer's `ClipRegion` field is what the walk reads (AR-7).

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
| Layer freed during an open update (incl. a FALSE `BeginUpdate`) | `EndUpdate(layer, FALSE)` first | AR-8 |
| Region freed during an open update | NULL installed (the layers BUGS case: a low-memory cliprect layout glitch for the rest of that update); the program's `layerEndUpdate` ends the update as today | AR-8 |
| `InstallClipRegion` fails (pre-V45, undetectable) | the walk reads `ClipRegion`, so state stays consistent | AR-7 |
| Guide closed with held messages | replied, handles orphaned | AR-9 |
| Getter or reply on an orphan | 0 / frees the handle only | AR-9 |

## Testing Requirements

- Spec: ST-7 (region), ST-8 (guide) on A1200; the existing `sdk-a500-text` and
  `sdk-a500-assets` gates guard the 1.3 layer path (ST-11).
- Impl: `cl_layer_free` ends an open update, then unclips, before `DeleteLayer`; the update flag is
  set on every `layerBeginUpdate` and cleared by `layerEndUpdate`, which still calls
  `EndUpdate(layer, TRUE)`; `cl_free_region` walks (unclip only, no `EndUpdate`) before disposing;
  set/clear clip unchanged; the guide close replies before `CloseAmigaGuide` and orphans.
