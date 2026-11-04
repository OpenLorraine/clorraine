# Requirements: Amiga Library Surface

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Functional requirements

### Must have

- **R1** — `std.amiga.exec` exports an opaque `MsgPort` handle, an opaque `NewMessage` handle for created messages, and an opaque `Message` handle for fetched messages.
- **R2** — The module creates and deletes a private message port:
  `createMsgPort(): MsgPort?` and `deleteMsgPort(take port: MsgPort): void`.
- **R3** — The module creates and frees a message with a reply port and a small payload:
  `createMessage(borrow reply: MsgPort, payload: i32): NewMessage?`,
  `messagePayload(borrow msg: Message): i32`, `freeMessage(take msg: NewMessage): void`,
  and `releaseMsg(take msg: Message): void`.
- **R4** — The module sends, waits, receives, and replies:
  `putMsg(borrow port: MsgPort, take msg: NewMessage): void`,
  `signals(borrow port: MsgPort): u32`,
  `waitSignals(mask: u32): u32`,
  `getMsg(borrow port: MsgPort): Message?`,
  `replyMsg(take msg: Message): void`.
- **R5** — The module is available only on `hosted-amiga`; the host and freestanding profiles
  reject it with `E11200`.
- **R6** — `exec-self-test` creates a port, sends a `NewMessage` to it, waits for the signal,
  receives a `Message`, replies it to a second port, receives the reply, releases it, prints a
  fixed transcript, and drains on every failure path before deleting a port.
- **R7** — Hamurabi is ported from the MIT source and plays a deterministic ten-year game with a
  built-in steward policy, printing a hand-checked transcript.
- **R8** — No language, grammar, or diagnostic change. The only specification edits are the
  `std.amiga.exec` section in `spec/appendix-stdlib.md` and the re-review notes in
  `spec/appendix-amiga.md`; both keep `stable (v1.0)`.

### Won't have

- The full Amiga library surface, windows, graphics beyond the existing modules, audio, or
  network access.
- Public (named) message ports beyond the private port needed by the acceptance programs.
- General device I/O, multi-tasking, or task creation wrappers.
- A language-level async or concurrency feature.

## Technical requirements

- The wrapper holds every FFI declaration and `unsafe` block; applications import only the
  exported wrappers.
- `NewMessage` and `Message` are move-only. `putMsg` consumes the sent `NewMessage`; `getMsg`
  returns an owned `Message`; `replyMsg` consumes it; `releaseMsg` releases a fetched message that
  will not be replied, and `freeMessage` releases an unsent `NewMessage`.
- A failed port or message allocation returns `null`; callers check it before use.
- The shim is valid C99 and builds with the hosted-amiga and freestanding legs; the freestanding
  shim is untouched.
- The programs build and run on hosted-amiga against an independent, hand-checked output file.

## Acceptance criteria

1. `std.amiga.exec` compiles on hosted-amiga and is rejected on the host profile with `E11200`.
2. `exec-self-test` and Hamurabi build and run under the hosted-amiga toolchain and match their
   expected output.
3. `make check`, `make verify`, and `./spec/check.sh` pass.
4. `spec/appendix-amiga.md` and `spec/appendix-stdlib.md` are re-reviewed against the delivered
   API and remain `stable (v1.0)`.
5. No language or ABI change.
