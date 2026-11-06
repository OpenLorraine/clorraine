# Workbench Services: Bundle 9

> **Document**: 03-04-workbench-services.md
> **Parent**: [Index](00-index.md)

## Overview

This component implements `SDK-WBK-003` commodities, `SDK-WBK-004` locale, `SDK-WBK-005` ARexx,
`SDK-WBK-006` clipboard, `SDK-WBK-007` serial/parallel/printer, and `SDK-WBK-010` nonvolatile.
All modules are `hosted-amiga` only. Commodities/locale/ARexx/clipboard/devices are 2.0/2.1;
nonvolatile is 3.1. [AR-7, AR-10, AR-22–AR-27, AR-31, AR-39]

## Implementation Details

### `std.amiga.commodities` (2.0+)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `Commodity` | `handle freedBy cl_close_commodity` | Owns the `CxObj` broker; copies strings |
| `openCommodity` | `(name: cstring, title: cstring, description: cstring, borrow port: MsgPort, unique: bool, priority: i32): Commodity?` | `NewBroker` with `NB_VERSION`, `NBU_UNIQUE`, clamped `nb_Pri`; port borrowed |
| `closeCommodity` | `(take commodity): void` | `DeleteCxObjAll`; frees copies |

The broker lifecycle is the acceptance surface: open, reject a duplicate with the same name,
close, reopen. The caller must close the commodity before deleting the borrowed port. The
input-event/custom-hook path is not exposed. [AR-22]

### `std.amiga.locale` (2.1+)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `Locale` | `handle freedBy cl_close_locale` | Owns `struct Locale *` |
| `openLocale` | `(name: cstring): Locale?` | Empty name uses `OpenLocale(NULL)` |
| `closeLocale` | `(take locale): void` | `CloseLocale` |
| `localeLanguageName` | `(borrow locale): String?` | Copied string |
| `localeCountryCode` | `(borrow locale): u32` | `loc_CountryCode` |
| `localeGmtOffset` | `(borrow locale): i32` | `loc_GMTOffset` |
| `localeDateTimeFormat` | `(borrow locale): String?` | Copied date-time pattern |

The A600/KS2.05 gate installs `locale.library` from Workbench 2.1. The gate asserts all four
fields are present (`name 1 cc 1 gmt 1 fmt 1`). [AR-23]

### `std.amiga.arexx` (2.0+)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `RexxMessage` | `handle freedBy cl_rexx_delete` | Wraps `struct RexxMsg *`; tracks created/received |
| `createRexxMessage` | `(borrow reply: MsgPort, extension: cstring, host: cstring): RexxMessage?` | `CreateRexxMsg`; copies extension/host |
| `deleteRexxMessage` | `(take message): void` | Shim-created: release args/result/copies then `DeleteRexxMsg`; foreign: reply only |
| `setRexxAction` | `(borrow message, action: i32): bool` | Sets `rm_Action` |
| `setRexxArg` | `(borrow message, index: i32, value: cstring): bool` | Copies an argument; index 0..15 |
| `sendRexxMessage` | `(borrow port: MsgPort, take message): void` | `PutMsg`; consumes |
| `receiveRexxMessage` | `(borrow port: MsgPort): RexxMessage?` | `GetMsg`; full packet length, then `IsRexxMsg` or shim record; short/foreign messages are replied |
| `replyRexxMessage` | `(take message): void` | `ReplyMsg`; consumes |
| `reclaimRexxMessage` | `(take message): void` | Shim-created: release args/result/copies then `DeleteRexxMsg`; foreign: reply |
| `rexxMessageAction` | `(borrow message): i32` | `rm_Action` |
| `rexxMessageResult1` | `(borrow message): i32` | `rm_Result1` |
| `rexxMessageResult2` | `(borrow message): i32` | `rm_Result2` |
| `rexxMessageArg` | `(borrow message, index: i32, borrow buf: u8[]): i32` | Copies an argument; -1 for absent |

The gate creates a message with a private reply port, sends it to the application port, receives,
inspects, replies, receives the reply, and reclaims it. The reply port stays open until reclaim.
`DeleteRexxMsg` does not release argument strings, so the shim deletes each owned argstring
directly; it does not call `ClearRexxMsg`, which crashed after the reply on the licensed 2.05
library. [AR-24]

### `std.amiga.clipboard` (2.0+)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `Clipboard` | `handle freedBy cl_close_clipboard` | Owns `IOClipReq` and reply port |
| `openClipboard` | `(unit: i32): Clipboard?` | `OpenDevice("clipboard.device", unit, ...)` |
| `closeClipboard` | `(take clipboard): void` | Close/delete/free |
| `clipboardWrite` | `(borrow clipboard, borrow data: u8[]): bool` | `io_ClipID=0`, `CMD_WRITE`, `CMD_UPDATE`; retries obsolete ID |
| `clipboardRead` | `(borrow clipboard, borrow data: u8[]): i32` | `CBD_CURRENTREADID` + `CMD_READ`; at most buffer length |
| `clipboardClear` | `(borrow clipboard): bool` | Zero-length write/update |

No `CLIPS:` assign is needed; the device owns the clip store. The gate writes/reads a short string
and asserts the byte count and value. [AR-25]

### `std.amiga.devices` — serial, parallel, printer (2.0+)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `Serial` | `handle freedBy cl_close_serial` | Owns `IOExtSer` and reply port |
| `openSerial` | `(unit: i32, baud: u32): Serial?` | Sets `io_Baud`/`io_SerFlags`, opens, issues `SDCMD_SETPARAMS` |
| `serialWrite` | `(borrow serial, borrow data: u8[]): i32` | `CMD_WRITE` |
| `serialRead` | `(borrow serial, borrow data: u8[]): i32` | `CMD_READ` |
| `closeSerial` | `(take serial): void` | Abort/close/free |
| `Parallel` | `handle freedBy cl_close_parallel` | Owns `IOExtPar` and reply port |
| `openParallel` | `(unit: i32, flags: u32): Parallel?` | Sets `io_ParFlags`, opens, issues `PDCMD_SETPARAMS` |
| `parallelWrite` | `(borrow parallel, borrow data: u8[]): i32` | `CMD_WRITE`; device write |
| `closeParallel` | `(take parallel): void` | Close/free |
| `Printer` | `handle freedBy cl_close_printer` | Owns a `union printerIO` and reply port |
| `openPrinter` | `(unit: i32): Printer?` | `OpenDevice("printer.device", unit, ...)` |
| `printerWrite` | `(borrow printer, borrow data: u8[]): i32` | `PRD_RAWWRITE`; result or -1 |
| `closePrinter` | `(take printer): void` | Close/free |

Serial and parallel are typed handles; the runtime gate proves open/close and the
data paths are implementation-tested. Printer open/close is gate-proven;
`PRD_RAWWRITE` is impl-tested and printer output is not claimed. The generic
`openDevice` rejects audio, timer, serial, parallel, and clipboard; the typed
printer handle uses `union printerIO`. [AR-26, AR-31]

### `std.amiga.nonvolatile` (3.1)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `NonvolatileData` | `handle freedBy cl_free_nv_data` | Owns `GetCopyNV` data |
| `getNonvolatile` | `(app: cstring, item: cstring): NonvolatileData?` | `GetCopyNV(app, item, TRUE)` |
| `freeNonvolatileData` | `(take data): void` | `FreeNVData` |
| `nonvolatileSize` | `(borrow data): u32` | `SizeNVData` |
| `nonvolatileRead` | `(borrow data, borrow buf: u8[]): i32` | Copies min(size, buffer) |
| `storeNonvolatile` | `(app: cstring, item: cstring, borrow data: u8[]): bool` | `StoreNV(..., (len + 9) / 10, TRUE) == 0` |
| `deleteNonvolatile` | `(app: cstring, item: cstring): bool` | `DeleteNV(..., TRUE) == 0` |

`StoreNV` length is in tens of bytes, not bytes; the shim rounds up. The A600/KS3.1 gate performs a
23-byte store/get/delete round-trip. [AR-27, AR-39]

## Error Handling

- Acquisition returns `null`; booleans false; counts -1 on failure.
- Duplicate commodity names return null.
- Clipboard retries obsolete ID a bounded number of times; reads never overflow the caller buffer.
- Serial/parallel/printer close abort and reap pending IO before closing.
- Nonvolatile missing keys return null; delete of a missing key returns false.

## Tests

- Compile all six surfaces on `hosted-amiga`; reject host/1.3/freestanding with `E11200`.
- Cross-build `nix20`; pin base globals (`CxBase`, `LocaleBase`, `RexxSysBase`, `NVBase`).
- Pin `NewBroker`/`CxBroker`/`DeleteCxObjAll`, `OpenLocale`/`CloseLocale`,
  `CreateRexxMsg`/`DeleteRexxMsg`/`IsRexxMsg`/`PutMsg`/`GetMsg`/`ReplyMsg`,
  `OpenDevice`/`CMD_READ`/`CMD_WRITE`/`CMD_UPDATE`/`CBD_CURRENTREADID`/`SDCMD_SETPARAMS`/`PDCMD_SETPARAMS`/
  `PRD_RAWWRITE`, `GetCopyNV`/`FreeNVData`/`SizeNVData`/`StoreNV`/`DeleteNV`.
- Runtime: A600/KS2.05 for commodities/locale/ARexx/clipboard/serial/parallel/printer;
  A600/KS3.1 for nonvolatile.
- Gate lines: `cmd: ok broker 1 dup 1 reopen 1`; `locale: ok name 1 cc 1 gmt 1 fmt 1`;
  `rexx: ok action 1`; `clip: ok n 4`; `ser: ok open 1`; `par: ok open 1`; `prt: ok open 1`;
  `nv: ok n 23 size 30`.
