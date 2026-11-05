# Design: Interrupts and Input Devices

> **Document**: 03-03-interrupts-and-input.md
> **Parent**: [Index](00-index.md)
> **Covers**: R5, R6, AR-11, AR-12, AR-16

## Port Interrupt Server

`std.amiga.interrupt` keeps its existing VBlank API and adds a ports server:

```clorraine
module std.amiga.interrupt;

export struct HandlerToken {
    id: u32;
}

ffi stdinterrupt {
    func cl_add_handler(hook: func(): void): u32;
    func cl_add_port_handler(hook: func(): void): u32;
    func cl_remove_handler(id: u32): void;
}

export func addHandler(hook: func(): void): HandlerToken {
    return HandlerToken { id: cl_add_handler(hook) };
}

export func addPortHandler(hook: func(): void): HandlerToken {
    return HandlerToken { id: cl_add_port_handler(hook) };
}

export func removeHandler(t: HandlerToken): void {
    cl_remove_handler(t.id);
}
```

`addHandler` registers on `INTB_VERTB` (vertical blank). `addPortHandler`
registers on `INTB_PORTS` (I/O ports and timers). Each kind has one slot, so a
new registration of the same kind replaces the previous one. The token is
opaque; both kinds draw from one counter so `removeHandler` can find the
owning slot without a public kind field. `removeHandler` is idempotent: a
zero, stale, or already-removed token does nothing.

## Shim Shape

```c
static cl_handler_fn cl_vblank_handler;
static struct Interrupt cl_vblank_server;
static ULONG cl_vblank_token;

static cl_handler_fn cl_ports_handler;
static struct Interrupt cl_ports_server;
static ULONG cl_ports_token;

static ULONG cl_next_token = 1;
```

Both `cl_add_handler` and `cl_add_port_handler` reject a null callback, remove
the previous server of their own kind, store the callback, take the next
non-zero token, fill the `struct Interrupt` (`NT_INTERRUPT`, priority 0, a
stable name), and call `AddIntServer` with `INTB_VERTB` or `INTB_PORTS`.

`cl_remove_handler(id)` checks the VBlank token and then the ports token. When
one matches, it calls `RemIntServer` for that kind and clears the handler and
token. The two token values are unique because they come from one counter, so
at most one slot matches.

The server code is the only code with the machine calling convention:

```c
static void cl_vblank_code(void) { if (cl_vblank_handler != NULL) cl_vblank_handler(); }
static void cl_ports_code(void)   { if (cl_ports_handler   != NULL) cl_ports_handler(); }
```

The handlers run in interrupt context. They must stay allocation-free,
drop-free, and panic-free. State shared with a handler is a `volatile`
module-level `let`.

## Input, Keyboard, and Console Devices

The generic device surface in
[03-02](03-02-libraries-and-devices.md) covers the three devices:

| Device | Demo action | Proof |
|---|---|---|
| `input.device` unit 0 | open and close | non-null `Device`, clean close |
| `console.device` unit 0 | `openConsole(window, 0)` on a 300x64 window, `CMD_WRITE` the 13-byte line, `DoIO`; then `CMD_READ` with `SendIO`, `AbortIO`, `WaitIO` | write error 0 and `io_Actual` 13; read `io_Error` is `IOERR_ABORTED` (-2) |
| `keyboard.device` unit 0 | open and close | non-null `Device`, clean close |

The console path uses the typed `openConsole` helper because the console
device is bound to an Intuition window; see
[03-02](03-02-libraries-and-devices.md). The aborted console read is the
deterministic keyboard-input abort proof: the dedicated window has no pending
input, while the 1.3 `keyboard.device` read completes immediately with a queued
event or an error (RV-1). No separate console or keyboard module is added. The
generic `IOStdReq` is the surface for input and keyboard; a device-specific
request type is out of scope.

## Live Interrupt Check

The demo registers a VBlank handler that increments a `volatile` counter,
waits three ticks with `delayTicks(3)`, confirms the counter advanced, and
removes the handler. It also registers and removes a ports handler. The
report line has no interrupt-count field: the VBlank increment only advances
the demo past its stage check, and the ports interrupt itself is not required
to fire in the headless run. This proves the dispatcher is installed and
callable on the real 1.3 machine without making the report line depend on an
interrupt count.

## Failure and Ownership

| Situation | Result |
|---|---|
| null callback | token 0; no server installed |
| replacing a live handler of the same kind | the old server is removed first |
| removing an unknown or stale token | no-op |
| removing twice | second removal is a no-op |
| a handler that allocates or panics | caller contract violation; documented, not enforced |

The module owns no heap memory; the `struct Interrupt` is static. The handler
function pointer is a plain C callback produced by the compiler for a
capture-less `func(): void`.

The module doc comment is updated with the second server slot; the 1.3 profile
comment in `compiler/src/profile.rs` is updated because the message-port path
uses `CreatePort`, which is 1.3-safe. These comment corrections are part of the
implementation tasks, not new behavior.
