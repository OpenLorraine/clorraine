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
    func cl_device_set_buffer(borrow device: Device, length: u32): bool;
    func cl_device_write_buffer(borrow device: Device, data: raw u8,
                                length: u32): bool;
    func cl_device_set_offset(borrow device: Device, offset: u32): bool;
    func cl_device_do_io(borrow device: Device, data: raw u8,
                         length: u32): i32;
    func cl_device_send_io(borrow device: Device): bool;
    func cl_device_wait_io(borrow device: Device, data: raw u8,
                           length: u32): i32;
    func cl_device_check_io(borrow device: Device, data: raw u8,
                            length: u32): bool;
    func cl_device_abort_io(borrow device: Device): void;
    func cl_device_actual(borrow device: Device): u32;
    func cl_device_error(borrow device: Device): i32;
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
version may be higher and is not pinned there. The demo lets the generated
drop close the library, so the drop path runs on the A500.

## Device Handle

```c
typedef struct cl_device {
    struct IOStdReq request;
    struct MsgPort *port;
    APTR buffer;             /* device-owned asynchronous buffer */
    ULONG buffer_length;
    cl_window *window_ref;   /* console window reference, or NULL */
    int pending;
} cl_device;
```

The handle owns one `struct IOStdReq`, one private reply port created with
`CreatePort(NULL, 0)`, an optional device-owned asynchronous buffer, and an
optional console-window reference. The record is allocated with
`MEMF_PUBLIC | MEMF_CLEAR`; `request.io_Message.mn_ReplyPort` is set before
`OpenDevice`. `closeDevice` aborts and waits for a pending request, calls
`CloseDevice`, frees the buffer, deletes the reply port, frees the handle, and
releases the console window.

Only the standard `IOStdReq`-sized devices are accepted. `audio.device`,
`timer.device`, `serial.device`, and `parallel.device` need larger requests and
keep their typed modules; `openDevice` rejects them rather than opening them
with a short request.

## Synchronous and Asynchronous Buffers

- **Synchronous** — `deviceDoIO(device, buffer)` borrows the caller's buffer
  for the call, sets `io_Data`/`io_Length`, runs `DoIO`, and returns
  `io_Error`. A read writes directly into the caller's buffer; the borrow
  lasts the call, so no pointer outlives the caller's scope. The demo uses this
  for the console write and the first trackdisk read.
- **Asynchronous** — `deviceSetBuffer(device, length)` reserves a
  device-owned buffer; `deviceWriteBuffer(device, data)` copies a write
  payload into it; `deviceSendIO(device)` starts the request;
  `deviceWaitIO(device, dest)` or `deviceCheckIO(device, dest)` waits or
  checks, copies up to `len(dest)` bytes of the result out, and returns the
  status. The buffer is freed on close. This keeps the device from holding a
  caller pointer after a function returns (MS-1). The demo uses this for the
  second trackdisk read and the aborted console read.

## Console Helper

`console.device` is not a plain generic device: its `struct ConUnit` is bound
to a `struct Window`, and `OpenDevice` needs the caller's window in `io_Data`
with `sizeof(struct Window)` in `io_Length`. `openConsole(window, unit)`:

1. borrows a `Window` from `std.amiga.intuition`;
2. increments the window's console refcount;
3. allocates the device handle and reply port;
4. sets `io_Data = window`, `io_Length = sizeof(struct Window)`;
5. calls `OpenDevice("console.device", unit, request, 0)`.

`closeWindow` on a window whose console refcount is nonzero defers the actual
`CloseWindow` and wrapper free until `closeDevice` releases the last console
reference. The console can therefore never use a freed Intuition window
(MS-2), and the caller is free to close the `Window` handle first.

## Request State Machine

| Call | Pending state | Behavior |
|---|---|---|
| `deviceCommand` | idle | sets `io_Command`, returns `true`; `false` for a null handle or a pending request |
| `deviceSetBuffer` | idle | reserves/frees the device-owned buffer; `false` for a null handle, a pending request, or allocation failure |
| `deviceWriteBuffer` | idle | copies into the device buffer; `false` without a buffer, while pending, or when `data` is longer |
| `deviceSetOffset` | idle | sets `io_Offset`, returns `true`; `false` for a null handle or a pending request |
| `deviceDoIO` | idle | sets the caller buffer, runs `DoIO`, restores the device buffer, returns `io_Error` |
| `deviceDoIO` | pending | returns -1 without sending |
| `deviceSendIO` | idle | runs `SendIO` over the device buffer, marks pending, returns `true` |
| `deviceSendIO` | pending | returns `false` without sending |
| `deviceWaitIO` | pending | runs `WaitIO`, clears pending, copies the result out, returns `io_Error` |
| `deviceWaitIO` | idle | returns the last `io_Error` without copying |
| `deviceCheckIO` | pending | `CheckIO` non-null: reaps with `WaitIO`, clears pending, copies the result out, returns `true`; otherwise `false` |
| `deviceCheckIO` | idle | returns `false` |
| `deviceAbortIO` | pending | calls `AbortIO` and leaves the request pending; the caller waits |
| `deviceAbortIO` | idle | no-op |
| `deviceActual` | any | returns `io_Actual`, or 0 for a null handle |
| `deviceError` | any | returns `io_Error`, or -1 for a null handle |
| `closeDevice` | pending | aborts, waits, closes, frees the buffer, deletes the port, releases the window |
| `closeDevice` | idle | closes, frees the buffer, deletes the port, releases the window |

The `CheckIO` success path calls `WaitIO` to reap the reply message; without
it a completed reply stays linked in the private port (RV-C1).

## Device Coverage

| Device | Demo use | Expected result |
|---|---|---|
| `trackdisk.device` unit 0 | `CMD_READ` 512 bytes at offset 0 with `DoIO` over the caller buffer, then again with a device-owned buffer and `SendIO`/`WaitIO` | error 0, `io_Actual` 512 for both |
| `console.device` unit 0 | `openConsole(window, 0)` on a 300x64 window, `CMD_WRITE` the 13-byte line with `DoIO`, then `CMD_READ` with a device-owned buffer and `SendIO`/`AbortIO`/`WaitIO` | write error 0 and `io_Actual` 13; read `io_Error` is `IOERR_ABORTED` (-2) |
| `input.device` unit 0 | open, read the last error, and let the generated drop close it | non-null handle; drop close |
| `keyboard.device` unit 0 | open and close | non-null handle, clean close |

The console read on the dedicated window is the deterministic keyboard-input
abort proof: the window has no pending input, while the 1.3 `keyboard.device`
read completes immediately with a queued event or an error (RV-1). The
keyboard and input devices are open/close coverage.

## Failure Values

| Call | Failure value |
|---|---|
| `openLibrary`, `openDevice`, `openConsole` | `null` |
| `libraryVersion`, `libraryRevision` | 0 |
| `deviceDoIO` | -1 for a null handle, a null buffer with a non-zero length, or a pending request |
| `deviceWaitIO` | -1 for a null handle; otherwise `io_Error` |
| `deviceCommand`, `deviceSetBuffer`, `deviceWriteBuffer`, `deviceSetOffset`, `deviceSendIO`, `deviceCheckIO` | `false` |
| `deviceActual` | 0 |
| `deviceError` | -1 for a null handle |

`unit` must be 0..255; `flags` is passed through unchanged. Every partial
allocation (reply port, request, device buffer, handle) is released before a
failure return. The generated drops are exactly-once and null-safe.

## Shim Details

- Add `#include <exec/io.h>`; `struct IOStdReq` is not declared by
  `exec/ports.h`.
- Every Exec call that takes a request takes
  `(struct IORequest *)&device->request`; the explicit cast keeps the strict
  `-Werror` build clean.
- `OpenDevice` is called with the reply port already installed.
- On an open failure, `cl_device_discard` deletes the reply port, frees the
  buffer and handle, and releases the console window; it does not call
  `CloseDevice`.
- `deviceSetBuffer`/`deviceWriteBuffer` use `raw u8` + `u32` at the FFI because
  an array parameter crosses as one `APTR`; the public wrappers pass
  `raw u8(data.ptr)`/`u32(len(data))`.
- `cl_device_check_io` calls `WaitIO` after `CheckIO` returns the request, so
  the reply is reaped before the request can be reused.
- `cl_device_do_io` restores the device-owned buffer pointer after a
  synchronous call, so a later `SendIO` never uses the caller's temporary
  buffer.
- `cl_window_release` decrements the console refcount and performs the
  deferred close when the last console device closes.

## Test Hooks

The specification test compiles the full library/device surface on both hosted
profiles and cross-builds it with `nix13`/`nix20`. The implementation test pins
the emitted shim calls, the `CMD_*` constants, the reply-port setup, the
pending-state branches, the `CheckIO` reap, the device-buffer copy-out, the
close-time abort path, and the console helper's window fields and refcount. The
A500 demo is the runtime proof for the device paths, and it exercises the
generated Task, Library, and Device drops.
