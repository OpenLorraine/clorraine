# Execution Plan: Amiga SDK Bundle 5 — DOS and Assets

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-04
> **Progress**: 13/13 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Four phases: pin and implement the pure ILBM parser, add the DOS file/lock/
path/volume surface, build the loader and viewer, then prove it on the real
A500/KS1.3 machine. Specification tests come before implementation in every
phase.

## Phase 1: Pure IFF/ILBM parser

> **Reasoning**: high — a new host-runnable parser with hostile-input bounds.

**Tasks**: 3

- [x] 1.1.1 Write `compiler/tests/amiga_ilbm_spec_test.rs` for ST-1–ST-6 and ST-15, including 0xFF-filled destinations and wrapped lengths; run it red
- [x] 1.1.2 Add `stdlib/std.iff.clo` with `IlbmInfo`, `ilbmInfo`, `ilbmDecode`, `ilbmPalette`, and `ilbmPixelHash`; register the module and run ST-1–ST-5 green
- [x] 1.1.3 Add the parser bounds/ASan cases (ST-16, IT-3) to `amiga_assets_impl_test.rs`; run `cargo test --test amiga_ilbm_spec_test --test amiga_assets_impl_test`

**Verify**: `cargo test --test amiga_ilbm_spec_test --test amiga_assets_impl_test`

## Phase 2: DOS file, lock, path, and volume surface

> **Reasoning**: high — DOS resource ownership and 1.3-only library calls.

**Tasks**: 3

- [x] 2.1.1 Write `compiler/tests/amiga_dos_spec_test.rs` for ST-7 and ST-8 with the full surface, host rejection, and short name buffers; run it red
- [x] 2.1.2 Extend `stdlib/std.amiga.dos.clo` and `compiler/runtime/shim_hosted_amiga.c` with seek/position/length, locks, path info, directory/file mutation, current/parent dir, and volume info; run ST-7 green
- [x] 2.1.3 Add the DOS routing/drop/short-name/shim-type cases (IT-1, IT-2, IT-5, IT-6) to `amiga_assets_impl_test.rs`; run them green

**Verify**: `cargo test --test amiga_dos_spec_test --test amiga_assets_impl_test`

## Phase 3: ILBM loader, fixture, and viewer

> **Reasoning**: high — file decoding, chip-RAM limits, and the A500 proof.

**Tasks**: 3

- [x] 3.1.1 Add loader/viewer spec cases (ST-6, ST-9, ST-15) to the ILBM and DOS suites; run the full surface red against `std.amiga.ilbm`
- [x] 3.1.2 Add `stdlib/std.amiga.ilbm.clo`, the `bitmapFromIndices` wrapper and shim, `examples/amiga/imageviewer.clo`, `tools/amiga-sdk/make_ilbm_fixture.py`, and `examples/amiga/assets/clorraine.ilbm`; run ST-9 and the cross-builds green
- [x] 3.1.3 Add the loader routing/drop/fixture cases (IT-1, IT-2, IT-4) to `amiga_assets_impl_test.rs`; run all three feature suites green

**Verify**: `cargo test --test amiga_ilbm_spec_test --test amiga_dos_spec_test --test amiga_assets_impl_test`; build the demo for `hosted-amiga-13`

## Phase 4: A500 acceptance and closeout

> **Reasoning**: high — real KS1.3 emulator acceptance and programme closeout.

**Tasks**: 4

- [x] 4.1.1 Add `docker/scripts/a500-assets-smoke.sh` and `make sdk-a500-assets`; add the leg to `make verify`
- [x] 4.1.2 Run the demo on FS-UAE A500/KS1.3, fix target-only defects, capture `docs/amiga-sdk/a500-assets.png` and `docs/amiga-sdk/a500-assets-line.png`, and pass the smoke
- [x] 4.1.3 Update the appendices, `AGENTS.md`, registry rows (`SDK-DOS-001/002/003/005/008` done, `SDK-DOS-004/006` deferred), `bundles.tsv`, coverage, issue #6, programme/roadmap, and `codeops/specialist-check.json`
- [x] 4.1.4 Run `make check`, then `make verify`; regenerate and update the issue body; close #6; commit through the active exec-plan commit mode and confirm a clean tree with no decrypted ROM

**Verify**: `make check`; `make verify`; `make sdk-registry`; `make sdk-a500-assets`

## Dependencies

- Phase 1 has no dependency beyond the existing stdlib host path.
- Phase 2.1.2 depends on 2.1.1; 2.1.3 depends on 2.1.2.
- Phase 3 depends on the parser (Phase 1) and DOS (Phase 2).
- Phase 4 depends on Phase 3 and the licensed ROM/`rom.key`, the host X
  display, and the Bundle 1 display path.
- The A500 smoke depends on the committed fixture and goldens.

## Success Criteria

1. All 13 tasks complete.
2. `std.iff` parses and decodes the fixture on the host; the parser hash is
   pinned independently.
3. `std.amiga.dos` compiles on both hosted profiles and is rejected by host.
4. The viewer sets `dos 255` and `vol 1`, loads the fixture, and holds the
   deterministic frame.
5. `make check`, `make verify`, and `make sdk-registry` pass; the five earlier
   A500 gates stay green.
6. Registry Bundle 5 is 5/7 done with `SDK-DOS-004`/`006` deferred; issue #6 is
   closed; the roadmap/programme show Bundle 5 done.
7. No language, grammar, diagnostic, or ABI change.

## Completion

Completed on 2026-11-05. `make check`, `make asan`, and `make verify` pass.
The A500 run holds the image frame and prints
`img: ok 320x200x4 hash 603248125 dos 255 vol 1`. Registry Bundle 5 is 5/7
with `SDK-DOS-004`/`006` deferred; issue #6 is closed.
