# Design: A500/Kickstart 1.3 Profile

> **Document**: 03-01-a500-profile.md
> **Parent**: [Index](00-index.md)

## Profile model

Add one variant, `Profile::HostedAmiga13`, for the A500 core tier.

| Fact | `hosted-amiga-13` | `hosted-amiga` (unchanged) |
|---|---|---|
| Hardware | A500: 68000, OCS, 512 KB chip | A1200: 68020, AGA, 2 MB chip |
| OS | Kickstart 1.3 | Kickstart 3.1 |
| C runtime | `-mcrt=nix13` | `-mcrt=nix20` |
| CPU | `-m68000` | `-m68000` (opt-in `-m68020`) |
| Link | `-lamiga -lm` | `-lamiga -lm` |
| Stack warning | 1024 bytes | 1024 bytes |
| Default runner | `vamos` | `vamos` |
| FS-UAE | `--emulator` allowed | `--emulator` allowed |

`Profile::parse("hosted-amiga-13")`, `name()`, `implemented()`, `shim()`,
`stack_frame_limit()`, the ABI matches in `codegen.rs` and `types/layout.rs`, the
toolchain `Tool`, the runner, and the `--emulator` validation all gain the new
variant. The shim is the existing hosted-amiga shim.

## Module gate

`Profile::supports_module` for the 1.3 profile is a whitelist:

```text
std.io
std.mem
std.text
std.fmt
std.amiga.custom
std.amiga.dos
std.amiga.graphics
std.amiga.intuition
std.amiga.interrupt
```

`std.amiga.exec` is excluded. Importing it reports `E11200`; no translation unit
is written. The 3.1 profile keeps every module.

The 1.3 graphics/intuition modules are allowed because the calls they lower to
(`OpenScreen`, `SetRGB4`, `RectFill`, `Draw`, `Delay`, `AddIntServer`) exist in
Kickstart 1.3. `std.amiga.exec` is excluded because its current `CreateMsgPort`/
`AllocVec` path is 2.0+ even though the shim fallback in this bundle is 1.3-safe.

## Shared shim changes

The shim is one file; the changes work on both 1.3 and 3.1:

| Current | Replacement | Reason |
|---|---|---|
| `CreateMsgPort()` | `CreatePort(NULL, 0)` | 1.3-safe; same private-port semantics |
| `DeleteMsgPort(port)` | `DeletePort(port)` | 1.3-safe |
| `AllocVec(size, flags)` | `AllocMem(size, flags)` | 1.3-safe; `FreeMem` needs the size |
| `FreeVec(ptr)` | `FreeMem(ptr, sizeof(cl_message))` | 1.3-safe |
| `OpenLibrary("graphics.library", 37)` | version 33 | 1.3 library version |
| `OpenLibrary("intuition.library", 37)` | version 33 | 1.3 library version |

The message allocation size is `sizeof(cl_message)`; the free path uses the same
constant. The 3.1 `std.amiga.exec` tests must stay green.

## A500 runtime

The driver builds `build/clorraine/<stem>/hosted-amiga-13/<stem>`. The smoke script:

1. Builds `examples/hello/hello.clo` with `--profile hosted-amiga-13`.
2. Runs `docker/scripts/run.sh` with `AMIGA_MODEL=A500`,
   `KICKSTART=/amiga/roms/amiga-os-130.rom`, `RUN_CAPTURE=1`, and a bounded timeout.
3. Polls `build/run/out.txt` for the hello line; stops the emulator; diffs the
   normalized capture against the expected line.

`run.sh` gains explicit environment overrides: it captures `AMIGA_MODEL` and
`KICKSTART` before sourcing `docker/.env`, then reapplies them, so a caller can
select A500/1.3 without editing the local `.env`.

If the directory drive does not mount on KS1.3, the smoke uses the existing
`RUNNER=adf` path; the script records which path was used.

## Compatibility

- The 3.1 profile emits the same C and uses the same shim; only the shim's
  1.3-safe replacements change, and those are behavior-equivalent.
- No grammar, checker, diagnostic, or ABI change.
- No new stdlib module.

## Error behavior

- Unknown profile name: `E11201`.
- Known but unimplemented profile: `E11200` (none in this bundle).
- 1.3 program importing a gated module: `E11200` with the module and profile named.
- Missing C toolchain/ROM in the smoke: skip with a recorded note, not a silent pass.
