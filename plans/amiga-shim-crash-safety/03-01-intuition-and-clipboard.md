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

After the caller's `CMD_READ` succeeds (AR-10, AR-11):
1. Save `got = io_Actual`.
2. Unless `got != length` (the offset is then already past the end): issue a skip read —
   `CMD_READ`, `io_Data = NULL`, `io_Length = 0x7FFFFFFF`, offset and clip ID untouched.
3. Issue exactly one finishing read — `CMD_READ`, `io_Data = NULL`, `io_Length = 1`, offset and
   clip ID untouched. The device sees `io_Offset` past the end and ends the read session. This
   holds after a skip read that returned 0 bytes too (a caller buffer exactly the clip size):
   the skip read always returns less than its `io_Length`, so it leaves `io_Offset` "beyond EOF"
   (clipboard.doc CMD_READ RESULTS), and only the read after it is past the end.
4. Return `got`. Errors of steps 2–3 do not change the result.

The `CBD_CURRENTREADID` step and the bounded retry of the caller's read stay as they are
(spec §14.18). A failed caller read returns -1 as today, with no finishing reads, and is retried
only for `CBERR_OBSOLETEID`. A small static helper (`cl_clipboard_finish_read`) holds steps 2–3.

### Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Caller read fails (not obsolete ID) | -1, no finishing reads | AR-11 |
| Obsolete ID | bounded retry of the caller read, as today | AR-11 |
| Skip or finishing read fails | ignored; return the caller count | AR-11 |
| Zero-length caller buffer (`got == length == 0`) | skip read + finishing read | AR-10 |
| Caller buffer exactly the clip size (the skip read returns 0) | skip read + finishing read | AR-10 |
| The fix does not end the hold-off on the 3.1 or 2.04 device | reopen AR-10 (zero `io_ClipID` first read; after an exact fit, stop after the zero-count skip read; spec ruling if needed) | AR-10 |

## Testing Requirements

- Spec: ST-1, ST-2 (`vamos`), ST-4 (A1200), ST-5 (A1200 and A600), ST-6 (A1200) in
  [07](07-testing-strategy.md).
- Impl: the Intuition open precedes the first Intuition call in both entries; the version check;
  the finishing sequence (saved count, skip read, one NULL read, no offset reset).
