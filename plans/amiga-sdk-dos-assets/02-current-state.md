# Current State: Amiga SDK Bundle 5

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing surface

| Area | Current state | Gap for Bundle 5 |
|---|---|---|
| `std.io` | `File` handle (`BPTR` contract), `openFile`, `createFile`, `readFile`, `writeBytes`; the shim backs `Open`/`Read`/`Write`/`Close` with the toolchain libc stream | No seek, position, or length |
| `std.amiga.dos` | Only `delayTicks` and `ticks` | No file, lock, path, directory, or volume surface |
| `std.amiga.video` | `Bitmap` create/free, plane bit get/set, fill/plot/line, blit, checksum, 320x256 `Display` | No bulk chunky-to-planar copy |
| `std.amiga.text` / `layers` | ROM/disk fonts, measure/draw, custom-display layers | Reused for the viewer caption |
| IFF/ILBM | None | No parser, decoder, or loader |
| Demos | hello, Boing, sprites, paddle, scroller | No file-backed asset demo |
| C shim | `dos.library` only for `Delay`/`DateStamp`; `dos/dos.h` types already included | No `Lock`, `Examine`, `Info`, `CreateDir`, `DeleteFile`, `Rename`, `CurrentDir`, `ParentDir` calls |

## Relevant code

| File | Role |
|---|---|
| `stdlib/std.io.clo` | `File` contract and libc-backed byte transfers |
| `stdlib/std.amiga.dos.clo` | The module to extend |
| `stdlib/std.amiga.video.clo` | `Bitmap`, `Display`, and the new index-copy primitive |
| `compiler/runtime/shim_hosted_amiga.c` | File, lock, path, volume, and bitmap helpers |
| `compiler/src/stdlib.rs` | Module registration and export tables |
| `compiler/src/profile.rs` | 1.3 module whitelist |
| `compiler/src/abi.rs` | `File` and `Lock` contract handles map to `BPTR` |
| `compiler/tests/codegen_run_spec_test.rs` | Host compile-and-run harness pattern |
| `compiler/tests/amiga_text_spec_test.rs` | Hosted-profile spec-test pattern |
| `docker/scripts/a500-text-smoke.sh` | A500 smoke template |
| `Makefile` | Build and verify targets |

## Constraints found in the code

- The shim deliberately avoids `proto/dos.h` because its `Open`/`Close`/`Read`/
  `Write` prototypes conflict with the libc-backed ABI functions. New DOS calls
  must be declared with `__stdargs` directly, as `Delay` already is.
- `File` and `Lock` are the only contract handles that map to `BPTR`
  (`compiler/src/abi.rs:16`); a new `Lock` handle therefore matches the DOS lock
  type without a cast in the generated prototype.
- `newBitmap` accepts 1..1024 pixels and 1..8 planes; `openDisplay` accepts at
  most 320x256 and 1..5 planes. The fixture is 320x200x4.
- `displaySetColor` takes 8-bit components and shifts them to the 4-bit OCS
  palette, so `CMAP` bytes pass through unchanged.
- The host profile has no Amiga modules; `std.iff` must therefore be free of
  `std.amiga.*` imports to stay host-runnable.
- The host `std.mem` shim provides `cl_buffer_new`/`cl_buffer_*`; the pure
  parser can use `Buffer` views without an Amiga dependency.
