# Current State: Amiga Library Surface

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing implementation

- `stdlib/` has `std.amiga.{custom,dos,graphics,interrupt,intuition}` but **no**
  `std.amiga.exec.clo`.
- `compiler/runtime/shim_hosted_amiga.c` provides the DOS delay, VBlank dispatcher, memory,
  buffer, text, and screen helpers. It has **no** `exec.library` port, signal, or message calls.
- `compiler/src/stdlib.rs` registers every embedded module and its exports. `std.amiga.exec` is
  absent from both `SOURCES` and `MODULES`.
- `Profile::supports_module` accepts every `std.*` module on hosted-amiga; the host profile
  rejects `std.amiga.*`, and freestanding accepts only `std.io`.
- `spec/appendix-amiga.md` §6 already sketches the intended event loop and imports
  `MsgPort, signals, waitSignals, getMsg, replyMsg` from `std.amiga.exec`. §2 sketches the
  `GetMsg`/`ReplyMsg` FFI shape.
- `spec/appendix-stdlib.md` has sections for memory, formatting, interrupts, screens, and
  drawing, but **no** `std.amiga.exec` section.
- `examples/` has small host programs; there are no hosted-amiga acceptance programs.

## Gaps

| Gap | Required change |
|---|---|
| No module | Add `stdlib/std.amiga.exec.clo` with the wrapper surface |
| No shim calls | Add the port, message, signal, put/get/reply helpers to `shim_hosted_amiga.c` |
| No registration | Add the module to `stdlib.rs` `SOURCES` and `MODULES` |
| No acceptance programs | Add `examples/amiga/exec-self-test.clo` and `examples/amiga/hamurabi.clo` |
| No oracle | Add hand-checked expected output files and a hosted-amiga runner test |
| No appendix section | Add the `std.amiga.exec` section to `spec/appendix-stdlib.md`; re-review appendix A |

## Risks

| Risk | Mitigation |
|---|---|
| AmigaOS message ownership is easy to get wrong | The wrapper makes `putMsg`/`replyMsg`/`freeMessage` consume the message; the self-test covers the full cycle |
| `Wait` can return for signals outside the mask | The programs wait for their own port signal and ignore unrelated bits |
| Hamurabi's C uses `float`, `printf`, `scanf`, and `time` | Port with `f32` comparisons, template printing, a deterministic steward policy, and a fixed RNG seed |
| The cross toolchain is unavailable on a bare host | The acceptance suite skips with a note, matching the existing hosted-amiga suites |
