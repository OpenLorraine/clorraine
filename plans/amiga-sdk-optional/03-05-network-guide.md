# Network and AmigaGuide: Bundle 9

> **Document**: 03-05-network-guide.md
> **Parent**: [Index](00-index.md)

## Overview

This component adds `SDK-OPT-002` TCP sockets and `SDK-OPT-003` AmigaGuide. Both are
`hosted-amiga` only. The socket gate uses FS-UAE’s emulated `bsdsocket.library` with a host
`127.0.0.1` echo server; the AmigaGuide gate uses the Workbench 3.1 library and a public Workbench
screen. [AR-6, AR-28, AR-29]

## Implementation Details

### `std.amiga.bsdsocket`

| Export | Signature | Ownership / behavior |
|---|---|---|
| `Socket` | `handle freedBy cl_close_socket` | Owns the OS descriptor |
| `AF_INET` | `const i32 = 2` | IPv4 |
| `SOCK_STREAM` | `const i32 = 1` | TCP |
| `openSocket` | `(kind: i32, protocol: i32): Socket?` | Opens `bsdsocket.library` v4; `socket()`; null on error |
| `connectSocket` | `(borrow socket, host: cstring, port: u16): bool` | `inet_addr` + `sockaddr_in` + `connect()` |
| `sendSocket` | `(borrow socket, borrow data: u8[]): i32` | `send()`; bytes sent or -1 |
| `tcpEcho` | `(host, port, request, response): i32` | one-shot send/reply; -1 on failure |
| `recvSocket` | `(borrow socket, borrow data: u8[]): i32` | `recv()`; bytes read, 0 EOF, -1 error |
| `closeSocket` | `(take socket): void` | `CloseSocket`; free record |

The guest `connect("127.0.0.1", port)` is proxied by FS-UAE to the host `connect()`. The network
gate runs with `--network host` and a host echo server. `sendSocket`/`recvSocket` may return short
counts. The typed split socket path is implemented and compile/impl-tested; the runtime gate uses
the one-shot `tcpEcho` helper because the emulated library's split descriptor path did not reach the
host peer in probes. [AR-6, AR-28]

### `std.amiga.amigaguide`

| Export | Signature | Ownership / behavior |
|---|---|---|
| `AmigaGuide` | `handle freedBy cl_close_amigaguide` | Owns `AMIGAGUIDECONTEXT`, `NewAmigaGuide`, copies |
| `GuideMessage` | `handle freedBy cl_reply_guide_message` | Owns one pending `struct AmigaGuideMsg` |
| `openAmigaGuide` | `(path: cstring, publicScreen: cstring): AmigaGuide?` | `OpenAmigaGuideAsync`; `HTF_NOACTIVATE` |
| `closeAmigaGuide` | `(take guide): void` | `CloseAmigaGuide`; frees struct/copies |
| `amigaGuideSignal` | `(borrow guide): u32` | Signal mask |
| `waitAmigaGuideMessage` | `(borrow guide): GuideMessage?` | `Wait` + `GetAmigaGuideMsg` |
| `guideMessageType` | `(borrow message): u32` | `agm_Type` |
| `guideMessagePrimary` | `(borrow message): u32` | `agm_Pri_Ret` |
| `guideMessageSecondary` | `(borrow message): u32` | `agm_Sec_Ret` |
| `replyGuideMessage` | `(take message): void` | `ReplyAmigaGuideMsg` |
| `sendAmigaGuideCommand` | `(borrow guide, command: cstring): bool` | `SendAmigaGuideCmd` |

The `NewAmigaGuide` struct and its `nag_Name`/`nag_PubScreen` copies stay alive until
`CloseAmigaGuide`; the autodoc requires them for async messages. A missing file is reported as an
async message, not a null context; the gate checks the primary/secondary result. The gate starts
Workbench with the existing fixture, asserts `startwb 1 screen <nonzero>`, opens a committed
`.guide`, waits for one message, replies, closes. [AR-29]

## Error Handling

- `openSocket` returns null when the library/socket is unavailable; connect false; send/recv -1.
- `openAmigaGuide` returns null only for an immediate allocation/context failure; file errors arrive
  as messages.
- `GuideMessage` must be replied exactly once; its drop replies.
- Socket record closes exactly once; null-safe.

## Tests

- Compile both surfaces on `hosted-amiga`; reject host/1.3/freestanding with `E11200`.
- Cross-build `nix20`; pin `OpenLibrary("bsdsocket.library", 4)`, `socket`, `connect`, `send`,
  `recv`, `CloseSocket`, `OpenAmigaGuideAsync`, `AmigaGuideSignal`, `GetAmigaGuideMsg`,
  `ReplyAmigaGuideMsg`, `CloseAmigaGuide`.
- Runtime: A1200/KS3.1 with `--bsdsocket_library=1 --network host`; host echo server. Gate line
  `net: ok send 5 recv 5` after the loop.
- Runtime: A1200/KS3.1 with Workbench started; gate line `guide: ok msg 1 result 0`.
