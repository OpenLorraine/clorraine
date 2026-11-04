# Design: Amiga Library Surface

> **Document**: 03-design.md
> **Parent**: [Index](00-index.md)

## `std.amiga.exec` API

The module owns the opaque handles and holds every `ffi` declaration, so application code
never writes `ffi` or `unsafe`.

```clorraine
module std.amiga.exec;

export handle MsgPort freedBy cl_delete_msg_port;
export handle NewMessage freedBy cl_free_message;
export handle Message freedBy cl_reply_msg;

ffi stdexec {
    func cl_create_msg_port(): MsgPort?;
    func cl_delete_msg_port(take port: MsgPort): void;
    func cl_create_message(borrow reply: MsgPort, payload: i32): NewMessage?;
    func cl_message_payload(borrow msg: Message): i32;
    func cl_put_msg(borrow port: MsgPort, take msg: NewMessage): void;
    func cl_signals(borrow port: MsgPort): u32;
    func cl_wait_signals(mask: u32): u32;
    func cl_get_msg(borrow port: MsgPort): Message?;
    func cl_reply_msg(take msg: Message): void;
    func cl_free_message(take msg: NewMessage): void;
    func cl_release_message(take msg: Message): void;
}
```

| Wrapper | Semantics |
|---|---|
| `createMsgPort()` | Allocates a private port; `null` on failure |
| `deleteMsgPort(take port)` | Deletes the port; the caller must have drained it |
| `createMessage(reply, payload)` | Allocates a `NewMessage` with `mn_ReplyPort = reply` and the payload |
| `messagePayload(msg)` | Reads the payload |
| `putMsg(port, take msg)` | Queues the message and wakes the port owner |
| `signals(port)` | Returns `1 << mp_SigBit` |
| `waitSignals(mask)` | Calls `Wait(mask)` and returns the received mask |
| `getMsg(port)` | Returns the next message, or `null` |
| `replyMsg(take msg)` | Replies the message to its reply port |
| `freeMessage(take msg)` | Releases an unsent `NewMessage` explicitly |
| `releaseMsg(take msg)` | Frees a fetched `Message` that returned to its reply port |

`NewMessage` and `Message` are move-only. `putMsg` consumes the sent `NewMessage`; `getMsg`
returns an owned `Message`; `replyMsg` consumes it. A message that has come back through a reply
port is freed with `releaseMsg`, not replied a second time. The `NewMessage` drop frees an unsent
message; the `Message` drop replies a fetched one, matching the appendix's ownership rule.

## Shim functions

The hosted-amiga shim adds `#include <exec/ports.h>` and `<exec/memory.h>` and implements:

| C function | AmigaOS calls |
|---|---|
| `cl_create_msg_port` | `CreateMsgPort` |
| `cl_delete_msg_port` | `DeleteMsgPort` (null-safe) |
| `cl_create_message` | `AllocVec(sizeof(cl_message), MEMF_PUBLIC \| MEMF_CLEAR)`, sets `mn_Length`, `mn_ReplyPort`, payload |
| `cl_message_payload` | reads the payload |
| `cl_put_msg` | `PutMsg` |
| `cl_signals` | `1UL << port->mp_SigBit` |
| `cl_wait_signals` | `Wait` |
| `cl_get_msg` | `GetMsg` |
| `cl_reply_msg` | `ReplyMsg` when the reply port is non-null |
| `cl_free_message` | `FreeVec` |
| `cl_release_message` | `FreeVec` for a fetched reply copy |

The shim remains C99 and is only compiled into the hosted-amiga profile.

## `exec-self-test`

A single console program that exercises every wrapper:

1. Create an inbox port and a reply port.
2. Create a message carrying `42` and reply to the reply port.
3. `putMsg(inbox, message)`.
4. `waitSignals(signals(inbox))`, `getMsg(inbox)`, print the payload.
5. `replyMsg(message)`; `getMsg(reply)` returns the reply; print confirmation.
6. `releaseMsg(reply)`, delete both ports.

Fixed transcript:

```text
exec-self-test: start
exec-self-test: signal received
exec-self-test: payload 42
exec-self-test: reply received
exec-self-test: done
```

## Hamurabi

Port of `michaelcmartin/bumbershoot/full/hamurabi.c` (MIT). The port:

- Uses `i32` for the game state and `f32` for the rating comparisons and average percentage.
- Replaces C stdio with `std.io` printing and a deterministic built-in steward policy.
- Uses a fixed 32-bit LCG instead of the C xorshift, so the transcript is deterministic.
- Keeps the ten-year loop, land/grain/plant choices, harvest, rats, births, plague, and rating.
- The steward buys 10 acres in year 1 and sells 10 in year 2 (so both land-trade paths run),
  feeds everyone when the store allows it, and plants what the remaining grain, land, and people
  support.
- Writes a hand-checked transcript to stdout.

The expected output is captured from the port, checked by hand against the game rules, and
stored beside the program.
