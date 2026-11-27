# Intuition and clipboard: Amiga shim crash-safety

> **Document**: 03-01-intuition-and-clipboard.md
> **Parent**: [Index](00-index.md)

## Overview

Two local fixes: the two Intuition entries that run with no screen or window open the base on
demand (FR-1), and the clipboard read finishes the device's read session (FR-3).

## Intuition on demand

### Current Architecture

`cl_show_alert` (`shim_hosted_amiga.c:2962-2988`) and `cl_save_public_screen`
(`shim_hosted_amiga_20.c:532-547`) use `IntuitionBase` as they find it. The planning scan found no
other handle-less Intuition user, and every handle-less graphics entry already opens `GfxBase`
(AR-2).

### Proposed Changes

- `cl_show_alert`: after the existing body checks and before `AllocMem`, call
  `cl_lib_open(&IntuitionBase, "intuition.library", 33)`; on NULL, return (no alert). The rest of
  the function is unchanged (AR-2).
- `cl_save_public_screen`: after the null checks, call
  `cl_lib_open(&IntuitionBase, "intuition.library", 33)`; return FALSE on NULL or when
  `IntuitionBase->LibNode.lib_Version < 36`; then `LockPubScreen` as today (AR-2).
- A base opened here is recorded by `cl_lib_open` and closed by the exit hook, like the two
  constructors' opens. No table slot is added (`IntuitionBase` is already one of the 20).

### Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| `intuition.library` cannot be opened | `showAlert` no-op; `savePublicScreen` false | AR-2 |
| Intuition below V36 for `savePublicScreen` | false | AR-2 |
| A null or over-long alert body | unchanged no-op (spec appendix-stdlib.md:281) | — |

## Clipboard read

### Current Architecture

`cl_clipboard_read` (`shim_hosted_amiga_optional_services.c:796-821`) gets the current read ID,
issues one `CMD_READ` of `length` bytes at offset 0, and returns `io_Actual`, retrying
`CBERR_OBSOLETEID` up to three times.

### Proposed Changes

*(Runtime AR-18 replaced the planned finishing reads: on the 2.04 and 3.1 devices every read at or
past the end of a clip, and every read that returns 0 bytes, holds off later writes on the unit
until all of its openers close it.)*

A `Clipboard` keeps its `IOClipReq` and reply port and stores its unit, but holds the unit open only
for one operation (AR-18, AR-11):
1. `cl_clipboard_open`: allocate the port and request as today, `OpenDevice` once to prove the unit
   opens (null otherwise), then `CloseDevice`; register the handle.
2. `cl_clipboard_write`: `OpenDevice` (false on failure), the unchanged write/update loop with its
   bounded `CBERR_OBSOLETEID` retry, `CloseDevice`, return the result.
3. `cl_clipboard_read`: `OpenDevice` (-1 on failure), the unchanged `CBD_CURRENTREADID` and one
   `CMD_READ` of `length` bytes at offset 0 with the bounded retry, `CloseDevice`, return
   `io_Actual` or -1. No read is issued past the end of the clip.
4. `cl_clipboard_close`: free the request, port, and handle; no `CloseDevice`.

Two static helpers (`cl_clipboard_attach`, `cl_clipboard_detach`) open and close the unit.

### Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Per-call `OpenDevice` fails | write false, read -1 | AR-18 |
| Caller read fails (not obsolete ID) | -1 | AR-11 |
| Obsolete ID | bounded retry of the caller read, as today | AR-11 |
| A read returns 0 bytes (empty unit, discarded raw clip) | the call's `CloseDevice` ends the device's hold-off when no other program holds the unit open | AR-18 |
| Another program holds the unit open during a zero read | writes stay held off until it closes the unit (device behaviour; documented limit) | AR-18 |

## Testing Requirements

- Spec: ST-1, ST-2 (`vamos`), ST-4 (A1200), ST-5 (A1200 and A600), ST-6 (A1200) in
  [07](07-testing-strategy.md).
- Impl: the Intuition open precedes the first Intuition call in both entries; the version check;
  the per-operation open and close around each write and read; no read past the end.
