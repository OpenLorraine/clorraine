# Design: Libraries and Devices

> **Document**: 03-02-libraries-and-devices.md
> **Parent**: [Index](00-index.md)
> **Covers**: R3, R4, R6, R9 (library/device part), AR-7–AR-10, AR-12, AR-15

## Public API

```clorraine
module std.amiga.exec;

import { Window } from std.amiga.intuition;

export handle Library freedBy cl_close_library;
export handle Device  freedBy cl_close_device;

/// Generic exec device commands from `exec/io.h`.
export const CMD_INVALID: u16 = 0;
export const CMD_RESET:   u16 = 1;
export const CMD_READ:    u16 = 2;
export const CMD_WRITE:   u16 = 3;
export const CMD_UPDATE:  u16 = 4;
export const CMD_CLEAR:   u16 = 5;
export const CMD_STOP:    u16 = 6;
export const CMD_START:   u16 = 7;
export const CMD_FLUSH:   u16 = 8;

ffi stdexec {
    func cl_open_library(name: cstring, version: u16): Library?;
    func cl_close_library(take library: Library): void;
    func cl_library_version(borrow library: Library): u16;
    func cl_library_revision(borrow library: Library): u16;

    func cl_open_device(name: cstring, unit: i32, flags: u32): Device?;
    func cl_open_console(borrow window: Window, unit: u8): Device?;
    func cl_close_device(take device: Device): void;
    func cl_device_command(borrow device: Device, command: u16): bool;
    func cl_device_set_buffer(borrow device: Device, data: raw u8,
                              length: u32): bool;
    func cl_device_set_offset(borrow device: Device, offset: u32): bool;
    func cl_device_do_io(borrow device: Device): i32;
    func cl_device_send_io(borrow device: Device): bool;
    func cl_device_wait_io(borrow device: Device): i32;
    func cl_device_check_io(borrow device: Device): bool;
    func cl_device_abort_io(borrow device: Device): void;
    func cl_device_actual(borrow device: Device): u32;
    func cl_device_error(borrow device: Device): i32;
}

export func openLibrary(name: cstring, version: u16): Library? {
    return cl_open_library(name, version);
}
export func closeLibrary(take library: Library): void {
    cl_close_library(library);
}
export func libraryVersion(borrow library: Library): u16 {
    return cl_library_version(library);
}
export func libraryRevision(borrow library: Library): u16 {
    return cl_library_revision(library);
}

export func openDevice(name: cstring, unit: i32, flags: u32): Device? {
    return cl_open_device(name, unit, flags);
}
export func openConsole(borrow window: Window, unit: u8): Device? {
    return cl_open_console(window, unit);
}
export func closeDevice(take device: Device): void {
    cl_close_device(device);
}
export func deviceCommand(borrow device: Device, command: u16): bool {
    return cl_device_command(device, command);
}
export func deviceSetBuffer(borrow device: Device, borrow data: u8[]): bool {
    unsafe {
        return cl_device_set_buffer(device, raw u8(data.ptr), u32(len(data)));
    }
}
export func deviceSetOffset(borrow device: Device, offset: u32): bool {
    return cl_device_set_offset(device, offset);
}
export func deviceDoIO(borrow device: Device): i32 {
    return cl_device_do_io(device);
}
export func deviceSendIO(borrow device: Device): bool {
    return cl_device_send_io(device);
}
export func deviceWaitIO(borrow device: Device): i32 {
    return cl_device_wait_io(device);
}
export func deviceCheckIO(borrow device: Device): bool {
    return cl_device_check_io(device);
}
export func deviceAbortIO(borrow device: Device): void {
    cl_device_abort_io(device);
}
export func deviceActual(borrow device: Device): u32 {
    return cl_device_actual(device);
}
export func deviceError(borrow device: Device): i32 {
    return cl_device_error(device);
}
```

## Library Semantics

`openLibrary(name, version)` calls `OpenLibrary(name, version)`. A version of
0 opens any version; a version above the installed library returns `null`.
The handle is the `struct Library *` base, so no wrapper allocation is needed.
`closeLibrary` calls `CloseLibrary` exactly once; the generated drop is
null-safe. `libraryVersion`/`libraryRevision` read `lib_Version` and
`lib_Revision`, returning 0 for a null handle.

The demo opens `exec.library` at version 33 and reports the installed version
(34 on Kickstart 1.3). On `hosted-amiga` the same call is compile-verified; the
version may be higher and is not pinned there.

## Device Handle

```c
typedef struct cl_device {
    struct IOStdReq request;
    struct MsgPort *port;
    int pending;
} cl_device;
```

The handle owns one `struct IOStdReq` and one private reply port created with
`CreatePort(NULL, 0)`. The record is allocated with
`MEMF_PUBLIC | MEMF_CLEAR`; `request.io_Message.mn_ReplyPort` is set before
`OpenDevice`. The reply port is required for `DoIO`/`SendIO`/`WaitIO`; without
it the device has no port to reply to. `closeDevice` aborts and waits for a
pending request, calls `CloseDevice` when a device was opened, deletes the
reply port, and frees the handle.

One `struct IOStdReq` is the standard request block for `trackdisk.device`,
`keyboard.device`, `input.device`, and the other block-structured 1.3 devices.
Devices that need a larger request (`timer.device` `timerequest`,
`audio.device` `IOAudio`) keep their typed modules.

## Console Helper

`console.device` is not a plain generic device: its `struct ConUnit` is bound
to a `struct Window`, and `OpenDevice` needs the caller's window in `io_Data`
with `sizeof(struct Window)` in `io_Length`. `openConsole(window, unit)`:

1. borrows a `Window` from `std.amiga.intuition`;
2. reads the shim's `cl_window.window` pointer;
3. allocates the device handle and reply port;
4. sets `io_Data = window`, `io_Length = sizeof(struct Window)`;
5. calls `OpenDevice("console.device", unit, request, 0)`.

The caller must keep the window open until `closeDevice` returns. The helper
is the only typed device path; there is no separate console module.

## Request State Machine

| Call | Pending state | Behavior |
|---|---|---|
| `deviceCommand` | idle | sets `io_Command`, returns `true`; `false` for a null handle or a pending request |
| `deviceSetBuffer` | idle | sets `io_Data`/`io_Length`, returns `true`; `false` for a null handle, a null buffer with a non-zero length, or a pending request |
| `deviceSetOffset` | idle | sets `io_Offset`, returns `true`; `false` for a null handle or a pending request |
| `deviceDoIO` | idle | runs `DoIO` and returns `io_Error` |
| `deviceDoIO` | pending | returns -1 without sending |
| `deviceSendIO` | idle | runs `SendIO`, marks pending, returns `true` |
| `deviceSendIO` | pending | returns `false` without sending |
| `deviceWaitIO` | pending | runs `WaitIO`, clears pending, returns `io_Error` |
| `deviceWaitIO` | idle | returns the last `io_Error` |
| `deviceCheckIO` | pending | `CheckIO` non-null clears pending and returns `true`; otherwise `false` |
| `deviceCheckIO` | idle | returns `false` |
| `deviceAbortIO` | pending | calls `AbortIO` and leaves the request pending; the caller waits |
| `deviceAbortIO` | idle | no-op |
| `deviceActual` | any | returns `io_Actual`, or 0 for a null handle |
| `deviceError` | any | returns `io_Error`, or -1 for a null handle |
| `closeDevice` | pending | aborts, waits, clears pending, then closes, deletes the port, and frees |
| `closeDevice` | idle | closes, deletes the port, and frees |

The caller owns the data buffer. It must stay alive until `deviceWaitIO` or
`deviceCheckIO` reports completion, or `closeDevice` returns. The `Device`
handle never frees it. A buffer used for `CMD_READ` must be writable storage
the caller keeps live; a `u8[N]` local is the normal form.

## Device Coverage

| Device | Demo use | Expected result |
|---|---|---|
| `trackdisk.device` unit 0 | `CMD_READ` 512 bytes at offset 0 with `DoIO`, then again with `SendIO`/`WaitIO` | error 0, `io_Actual` 512 for both |
| `console.device` unit 0 | `openConsole(window, 0)`, `CMD_WRITE` the 13-byte line `exec console\n` with `DoIO` | error 0, `io_Actual` 13 |
| `keyboard.device` unit 0 | `CMD_READ` into a 64-byte `InputEvent` buffer with `SendIO`, then `AbortIO`, then `WaitIO` | `io_Error` is `IOERR_ABORTED` (-2) on an idle keyboard |
| `input.device` unit 0 | open and close | non-null handle |

The demo runs on the boot floppy, so `trackdisk.device` unit 0 is the ADF.
Offset 0 avoids any ambiguity about the device's offset unit. The keyboard
request is guaranteed to be pending because no key is pressed in the headless
run; the abort path is the deterministic proof of `SendIO`/`AbortIO`/`WaitIO`.

## Shim Details

- Add `#include <exec/io.h>` to the shim; `struct IOStdReq` is not declared by
  `exec/ports.h`.
- Every Exec call that takes a request takes
  `(struct IORequest *)&device->request`; the explicit cast keeps the strict
  `-Werror` build clean.
- `OpenDevice` is called with the reply port already installed.
- On any open failure, delete the reply port and free the handle.
- The `ffi` takes `data: raw u8, length: u32`, not `borrow data: u8[]`: an
  array parameter crosses the FFI as one `APTR` pointer, so the length must be
  an explicit argument. The public `deviceSetBuffer` wrapper uses
  `unsafe { cl_device_set_buffer(device, raw u8(data.ptr), u32(len(data))) }`,
  matching `windowString`. The shim signature is
  `BOOL cl_device_set_buffer(void *handle, APTR data, ULONG length)` and
  rejects a null pointer with a non-zero length.
- `cl_device_command` and `cl_device_set_offset` return `BOOL` and reject a
  null handle or a pending request.
- `cl_device_wait_io` on an idle handle returns `request.io_Error`; only a null
  handle returns -1.
- `cl_device_error` on a null handle returns -1.

## Failure Values

| Call | Failure value |
|---|---|
| `openLibrary`, `openDevice`, `openConsole` | `null` |
| `libraryVersion`, `libraryRevision` | 0 |
| `deviceDoIO` | -1 for a null handle or a pending request |
| `deviceWaitIO` | -1 for a null handle; otherwise `io_Error` |
| `deviceCommand`, `deviceSetBuffer`, `deviceSetOffset`, `deviceSendIO`, `deviceCheckIO` | `false` |
| `deviceActual` | 0 |
| `deviceError` | -1 for a null handle |

`unit` must be 0..255; `flags` is passed through unchanged. Every partial
allocation (reply port, request, handle) is released before a failure return.
The generated drops are exactly-once and null-safe.

## Test Hooks

The specification test compiles the full library/device surface on both hosted
profiles and cross-builds it with `nix13`/`nix20`. The implementation test pins
the emitted shim calls, the `CMD_*` constants, the reply-port setup, the
pending-state branches, the close-time abort path, and the console helper's
window fields. The A500 demo is the runtime proof for the device paths.
