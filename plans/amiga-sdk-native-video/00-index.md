# Amiga SDK Bundle 1: Native Video

> **Status**: Done (9/9 tasks)
> **Created**: 2026-11-04
> **Bundle**: `1-native-video` (core; min OS 1.3; OCS/68000/512K)
> **Tracking**: [issue #2](https://github.com/OpenLorraine/clorraine/issues/2)
> **Registry**: [coverage.md](../../docs/amiga-sdk/coverage.md)
> **CodeOps Artifact Schema**: 1

## Purpose

Add the native-video surface the A500 demos need: chip memory, bitmaps,
software drawing, blitter copies, a copper list builder, a double-buffered
`LoadView` display, a 4-bit palette, fixed-point math, and a deterministic frame
checksum. The bundle acceptance is a Boing demo on the real A500/KS1.3 machine.

The bundle deliberately stays out of the Intuition screen API: `Display` is a
custom `graphics.library` view, so the demo owns the whole display and restores
the system view when it closes.

## Documents

| Document | Contents |
|---|---|
| [00-ambiguity-register.md](00-ambiguity-register.md) | Gate register; all items resolved |
| [01-requirements.md](01-requirements.md) | Scope, requirements, acceptance |
| [02-current-state.md](02-current-state.md) | Existing surface, gaps, and the verified spike |
| [03-01-native-video-api.md](03-01-native-video-api.md) | `std.amiga.video` and `std.math` design |
| [03-02-display-copper-boing.md](03-02-display-copper-boing.md) | Display, copper, blitter, checksum, and Boing design |
| [05-findings.md](05-findings.md) | Review and runtime evidence |
| [07-testing-strategy.md](07-testing-strategy.md) | Specification cases and verification |
| [99-execution-plan.md](99-execution-plan.md) | Phases and task checklist |

## Baseline

| Fact | Value |
|---|---|
| Profile | `hosted-amiga-13` (`-mcrt=nix13`, `-m68000`, `-fwrapv`) |
| Machine | A500: 68000, OCS, 512 KB chip, Kickstart 1.3 |
| Runtime proof | FS-UAE A500 with the decrypted licensed KS1.3 ROM |
| Regression gate | `make check`, `make verify`, `make sdk-registry`, A500 smoke |

## Module surface

`std.amiga.video` exports four opaque handles (`ChipBuffer`, `Bitmap`,
`Copper`, `Display`) and their builders. `std.math` exports `sin`, `cos`, and
`sqrt`. No raw OS struct is public.

## Definition of done

1. The eleven registry rows move to `done`, with evidence in this plan.
2. `examples/amiga/boing.clo` runs on FS-UAE A500/KS1.3 and its final checksum
   line matches the committed golden crop.
3. The live Boing display is captured in `docs/amiga-sdk/a500-boing.png`.
4. `make check`, `make verify`, `make sdk-registry`, and
   `docker/scripts/a500-boing-smoke.sh` pass.
5. The 3.1 profile stays green and no language/grammar/diagnostic/ABI text
   changes.
6. Issue #2 is closed and the roadmap/programme index show Bundle 1 done.

## Specialist Agents

None as a project-local specialist — the bundle adds embedded standard-library
sources and a C runtime shim; it does not change compiler semantics, ownership
lowering, IR, or emission. The new shim does have allocation/free paths, so the
phase review included an independent correctness and memory-safety pass over
those paths. The host AddressSanitizer auditor cannot execute the m68k shim;
the A500 runtime and the held-frame/checksum gates cover the target path.
