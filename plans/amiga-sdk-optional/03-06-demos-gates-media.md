# Demos, Gates, and Media: Bundle 9

> **Document**: 03-06-demos-gates-media.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns the licensed-media staging helper, the acceptance demos, the runtime gates,
and the `Makefile` wiring. It reuses the existing A500 ADF and A600/A1200 directory-HD patterns; no
new framework is introduced. [AR-7, AR-33, AR-36]

## Media Installation

### Helper: `docker/scripts/workbench-media.sh`

| Function | Behavior |
|---|---|
| `wb_stage <adf> <dest>` | `xdftool -r <adf> unpack <dest>` into a staging tree; removes `.xdfmeta` |
| `wb_copy <stage> <hd> [trees...]` | Copies `Libs`, `Devs`, `Classes`, `Fonts`, `Locale`, `C` with source case |
| `wb_dirs <hd>` | Creates `Libs`, `Devs`, `Classes`, `Fonts`, `Locale`, `C`, `env`, `envarc`, `rexx` |
| `wb_assigns <hd>` | Writes `CNEXT:C/Assign` lines; `LIBS: CNEXT:Libs ADD CNEXT:Classes` |
| `wb_datatypes <hd>` | Runs `CNEXT:C/AddDataTypes REFRESH QUIET` before datatype/BOOPSI demos |
| `wb_cleanup <work>` | Removes staging, unpacked media, and decrypted ROMs |

Never copy the staged `S/` tree; write the test `s/startup-sequence` after the copy. The media
stays read-only under `/amiga`; the helper copies only named trees into `build/`. [AR-7, AR-36]

### Gate media matrix

| Gate | Base media | Installed |
|---|---|---|
| `sdk-a500-diskfont` | Workbench 1.3.4 | `DF1:C/Assign`; `LIBS:`/`FONTS:` from DF1; `diamond.font` size 12 |
| `sdk-a600-addons` | Workbench 2.04 + 2.1 | `asl`, `commodities`, `rexxsyslib`, `locale`; clipboard/serial/parallel/printer devices; `LOCALE:` |
| `sdk-a600-ks31-addons` | Workbench 3.1 | `lowlevel`, `nonvolatile`; `ENV:`/`ENVARC:` |
| `sdk-a1200-ks30-addons` | Workbench 3.0 | pool module; `colorwheel.gadget`; `Classes` in `LIBS:` |
| `sdk-a1200-addons` | Workbench 3.1 | `datatypes`, `amigaguide`; `Classes/DataTypes`; `AddDataTypes`; Workbench start |

## Demos

| Demo | Capability | Report line |
|---|---|---|
| `examples/amiga/optional-exec20.clo` | semaphore, hook, tag list, date | `exec: ok sem 1 hook 1 tag 1 date 1` |
| `examples/amiga/optional-pool.clo` | utility pool | `pool: ok 1` |
| `examples/amiga/optional-args.clo` | ReadArgs, EClock, regions | `args: ok n 3 file 1 num 7 sw 1`; `regions: ok bounds 10 10 20 20`; `eclock: ok 709379` |
| `examples/amiga/optional-intuition.clo` | gadtools + ASL | `gt: ok gad 4`; `asl: ok path 1` |
| `examples/amiga/optional-services.clo` | commodities, locale, ARexx, clipboard, devices | `cmd: ok broker 1 dup 1 reopen 1`; `locale: ok name 1 cc 1 gmt 1 fmt 1`; `rexx: ok action 1`; `clip: ok n 4`; `ser: ok n 4`; `par: ok n 4`; `prt: ok open 1` |
| `examples/amiga/optional-boopsi.clo` | BOOPSI colorwheel | `boopsi: ok hue 42` |
| `examples/amiga/optional-datatypes.clo` | datatypes ILBM | `dt: ok w 320 h 200 depth 4 colors 16` |
| `examples/amiga/optional-lowlevel.clo` | ReadJoyPort | `ll: ok v 40` |
| `examples/amiga/optional-nonvolatile.clo` | nonvolatile | `nv: ok n 23` |
| `examples/amiga/optional-network.clo` | bsdsocket | `net: ok send 5 recv 5` |
| `examples/amiga/optional-guide.clo` | AmigaGuide | `guide: ok msg 1 result 0` |
| `examples/amiga/diskfont.clo` | diskfont | `diskfont: ok h 12` |

The intuition demo writes `gt-ready.txt` after the capture, then `asl-ready.txt` before
`AslRequest`, and `asl-done.txt` after it returns. The gate uses those names. The network demo
loops send/recv until 5 bytes. [AR-15, AR-28]

## Gates

### `sdk-a500-diskfont`

1. Build a boot ADF with `CNEXT:diskfont` and `DF1:C/Assign LIBS: DF1:Libs` /
   `DF1:C/Assign FONTS: DF1:Fonts`; boot with `--writable-floppy-images=1`.
2. Boot A500/KS1.3 with `amiga-os-134-workbench.adf` as DF1.
3. Run `CNEXT:diskfont`; it opens `diamond.font` size 12 and writes `diskfont: ok h 12`.
4. Stop FS-UAE, then `xdftool read` the report; assert the height.

### `sdk-a600-addons`

1. Stage Workbench 2.04/2.1; copy libs/devs; write assigns; boot A600/KS2.05.
2. Run `optional-exec20`, `optional-args`, `optional-services`.
3. Poll `exec.txt`, `args.txt`, `regions.txt`, `eclock.txt`, `services.txt`; assert full lines.
4. Serial: FS-UAE `serial_port=tcp://127.0.0.1:PORT/wait`; host echo peer. Parallel: output sink;
   assert the host received the bytes. Printer: open/close/`PRD_RAWWRITE` result.

### `sdk-a600-intuition` (part of `sdk-a600-addons` script with a mode)

1. Same media plus `asl.library`; boot A600/KS2.05 on Xvfb.
2. Run `optional-intuition`; poll `gt-ready.txt`; capture `a600-intuition.cscr`; compare
   `docs/amiga-sdk/a600-intuition.png`.
3. On `asl-ready.txt`, focus the FS-UAE window and type `asl.txt` + Return with retries; poll
   `asl-done.txt`; assert `gt: ok gad 4`, `asl: ok path 1`, and `asl-path.txt` contains `asl.txt`.

### `sdk-a600-ks31-addons`

1. Stage Workbench 3.1; boot A600/KS3.1 (ECS/68000/1MB).
2. Run `optional-lowlevel` and `optional-nonvolatile`; create `ENV:`/`ENVARC:`.
3. Assert `ll: ok v 40`, `nv: ok n 23`.

### `sdk-a1200-ks30-addons`

1. Stage Workbench 3.0; `LIBS: ... ADD Classes`; boot A1200/KS3.0.
2. Run `optional-pool` and `optional-boopsi`; capture `a1200-ks30-boopsi.cscr`; compare
   `docs/amiga-sdk/a1200-ks30-boopsi.png`.
3. Assert `pool: ok 1`, `boopsi: ok hue 42`.

### `sdk-a1200-addons`

1. Stage Workbench 3.1; `AddDataTypes`; boot A1200/KS3.1 with `--bsdsocket_library=1
   --network host`.
2. Start Workbench with `workbench-start.c`; assert `startwb 1 screen <nonzero>`.
3. Run `optional-datatypes`, `optional-network`, `optional-guide`.
4. Host TCP echo server on `127.0.0.1`; guest connects to `127.0.0.1:<port>`.
5. Assert `dt: ok w 320 h 200 depth 4 colors 16`, `net: ok send 5 recv 5`,
   `guide: ok msg 1 result 0`; optionally capture the ILBM screen as `a1200-datatypes.png`.

### Makefile

Add `sdk-a500-diskfont`, `sdk-a600-addons`, `sdk-a600-ks31-addons`, `sdk-a1200-ks30-addons`, and
`sdk-a1200-addons`, each with a build prerequisite and Xvfb. Add all five to `verify` after the
existing A1200 Workbench leg. `make check` runs `spec/check.sh`, the registry check, and the Rust
suites.

## Error Handling

- Every gate clears its `work/` directory and HD before boot.
- Every gate removes decrypted ROMs, staging trees, and unpacked media on exit.
- Polls are bounded; a timeout dumps handshake files and the FS-UAE log.
- A missing media file fails before boot with the exact path.
- A blank capture or line mismatch fails the gate; no stale file can satisfy the handshake.
- Goldens are captured only after the report line passes; `CAPTURE_GOLDEN=1` is a one-time path.

## Tests

- Pin the staging/case/`S` removal, `LIBS: ... ADD Classes`, `AddDataTypes`, media paths, and
  `xdftool -r`.
- Pin each demo’s report line and handshake filenames.
- Pin the gate ROM SHA-1, model/memory, Xvfb, capture conversion, and compare commands.
