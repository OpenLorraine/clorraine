# Current State: Amiga RTG

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Registry and Programme

`docs/amiga-sdk/registry.tsv` currently contains the placeholder:

| ID | Bundle | Capability | Priority | Min OS | Status |
|---|---|---|---|---|---|
| `SDK-OPT-004` | `9-optional` | RTG/Picasso96/CyberGraphix | `wont` | 3.1 | `out-of-scope` |

Bundle 9 is complete at 20/20 in-scope rows. The placeholder is the 21st row and
the only reason the old coverage line says 20/21. Issue #11 tracks the future
RTG work and lists the prerequisites that are now satisfied by user approval.

## What Exists in the Repository

| Area | Existing pattern | RTG fit |
|---|---|---|
| stdlib modules | `stdlib/std.amiga.*.clo`, one module per library/domain | Add `std.amiga.picasso96` |
| Optional C shims | `compiler/runtime/shim_hosted_amiga_optional*.c`, concatenated in `Profile::shim_suffix` | Add `shim_hosted_amiga_optional_rtg.c` |
| Handles | `export handle X freedBy cl_close_x`, generated exactly-once drop | `RtgScreen`, `RtgBitmap` |
| Profile gate | `E11200` for modules outside a profile | Host/1.3/freestanding rejection |
| Spec tests | `compiler/tests/amiga_*_spec_test.rs` | New `amiga_rtg_spec_test.rs` |
| Impl pins | `compiler/tests/amiga_*_impl_test.rs` | New `amiga_rtg_impl_test.rs` |
| Gates | `docker/scripts/*-smoke.sh`, `Makefile` target, `make verify` | New `a1200-rtg-smoke.sh` |
| Media staging | `docker/scripts/workbench-media.sh` (`wb_stage`, `wb_copy`, `wb_assigns`) | Reuse for WB3.1 and P96 files |
| Capture conversion | `docs/amiga-sdk/*.png`, ImageMagick in the gate | Convert the PPM to a small PNG for docs |

## P96 Media

The user approved the Aminet package. The gate expects:

```
${AMIGA_DATA}/picasso96/Picasso96.lha
```

The archive is extracted inside the SDK container with `/opt/amiga/bin/lha` to
`build/amiga-sdk/a1200-rtg/p96`. The needed runtime files are:

```
Picasso96Install/Libs/Picasso96API.library   (2.300)
Picasso96Install/Libs/Picasso96/rtg.library
Picasso96Install/Libs/Picasso96/uaegfx.card
Picasso96Install/Libs/Picasso96/emulation.library
Picasso96Install/Devs/Monitors/Picasso96   (renamed to uaegfx)
Picasso96Install/Devs/Picasso96Settings.31  (default settings; uaegfx may ignore)
```

The approved archive has SHA-256
`0146d4a2aaadb82ec454dde6ca4fb33a3f646ca4c0daa46e569c025f4d9728af`. The
surface supports 32-bit truecolor only, matching the proven spike.

The P96 developer archive (`P96Develop.lha`) supplied the API facts used to
write the vendored header. It is not needed at gate time. Neither archive is
committed.

## FS-UAE

FS-UAE 3.1.66 supports `--graphics-card=uaegfx` (also `uaegfx-z2`/`uaegfx-z3`).
The spike used `--graphics-card=uaegfx` on an A1200/KS3.1 with WB3.1 Libs,
P96 libraries, and the `uaegfx` monitor driver. It reported:

```
probe: ok boards 1 mode 50031303 w 640 h 480 depth 32 pixel ff00 ...
```

That proves the library, board, mode database, screen open, pixel write/read,
bitmap allocation, blit, and RGB readback path.

## Gaps to Close

| Gap | Resolution |
|---|---|
| No P96 stdlib module | `std.amiga.picasso96` with opaque handles |
| No P96 C shim | `shim_hosted_amiga_optional_rtg.c` plus vendored `picasso96.h` |
| No profile rejection | `Profile::supports_module` and `E11200` tests |
| No runtime gate | `sdk-a1200-rtg` script and Makefile target |
| No registry row | new `10-rtg` bundle and `SDK-RTG-001` |
| No docs | appendix §14.23, appendix-amiga table, limitations, AGENTS |
| P96 header not in NDK | vendor a minimal attributed definition header |
