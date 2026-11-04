# Execution Plan: Amiga SDK Bundle 1 — Native Video

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-04 17:45
> **Progress**: 9/9 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Three phases: pin the new module surface with specification tests, implement the
chip/bitmap/blitter/copper/display shim and fixed-point math, then build and
prove the Boing demo on the real A500/KS1.3 machine. Specification tests come
before implementation in every phase.

## Phase 1: Specification tests (red)

> **Reasoning**: high — new public API and a runtime display path; the tests pin
> the contract before the shim exists.

**Tasks**: 2

- [x] 1.1.1 Write `compiler/tests/amiga_video_spec_test.rs` for ST-1–ST-6 and ST-8 (module gate, full-surface compile, cross-builds, host math values); run it red because the modules and shim helpers do not exist
- [x] 1.1.2 Write `compiler/tests/amiga_video_impl_test.rs` for ST-7 (wrapper routing and null-guarded drops); run it red

**Verify**: `cargo test --test amiga_video_spec_test --test amiga_video_impl_test` (red recorded in `05-findings.md`)

## Phase 2: Module surface and shim

> **Reasoning**: high — C hardware setup, double-buffer lifecycle, and a new
> public standard-library surface.

**Tasks**: 4

- [x] 2.1.1 Add `stdlib/std.amiga.video.clo` and `stdlib/std.math.clo`; register both in `compiler/src/stdlib.rs`; add both to the `hosted-amiga-13` whitelist; run ST-1–ST-3 and ST-8 green
- [x] 2.1.2 Implement `ChipBuffer`, `Bitmap`, software drawing, blitter copies, plane-bit access, and the FNV-1a checksum in `compiler/runtime/shim_hosted_amiga.c`; cross-build the ST-4 program with `nix13`
- [x] 2.1.3 Implement the `Copper` builder and the `Display` view/copper/flip/restore lifecycle in `compiler/runtime/shim_hosted_amiga.c`; cross-build ST-4 and ST-5 with strict warnings
- [x] 2.1.4 Implement `std.math` fixed-point `sin`/`cos`/`sqrt`; run ST-6 green on the host and the full `cargo test --workspace`

**Verify**: `cargo test --workspace`; `make check`; cross-build the native-video fixture for both hosted profiles

## Phase 3: Boing and runtime close-out

> **Reasoning**: high — real Kickstart 1.3 emulator acceptance, screenshot and
> deterministic checksum evidence.

**Tasks**: 3

- [x] 3.1.1 Add `examples/amiga/boing.clo` using every Bundle 1 capability; build it for `hosted-amiga-13`; fix compile/runtime issues
- [x] 3.1.2 Add `docker/scripts/a500-boing-smoke.sh` and `make sdk-a500-boing`; capture the live display and the checksum-line crop; add the leg to `make verify`
- [x] 3.1.3 Update `spec/appendix-amiga.md`, `spec/appendix-stdlib.md`, `AGENTS.md`, the registry/coverage, issue #2, the programme index, and the roadmap; run `make verify` and `make sdk-registry`; close the plan

**Verify**: `make verify`; `make sdk-registry`; `docker/scripts/a500-boing-smoke.sh build/clorraine/boing/hosted-amiga-13/boing`

## Dependencies

- Phase 1 has no dependency on implementation; it is the red oracle.
- Phase 2.1.2 depends on 2.1.1 (module declarations and shim symbol names).
- Phase 2.1.3 depends on 2.1.2 (bitmap and record types).
- Phase 3 depends on Phase 2; the Boing demo uses the full surface.
- The A500 smoke depends on the licensed ROM and `rom.key` in
  `$AMIGA_DATA`, plus the host X display, as the existing hello smoke does.

## Success Criteria

1. All 9 tasks complete.
2. The new modules compile on both hosted profiles and are rejected by host.
3. `std.math` host values match ST-6.
4. `boing.clo` runs on FS-UAE A500/KS1.3, the live display is captured, and the
   final checksum line matches the golden crop.
5. The existing hello smoke, 3.1 graphics/Exec suites, differential, corpus,
   and fault legs stay green.
6. Registry Bundle 1 is 11/11 done; issue #2 closed; roadmap/programme updated.
7. No language, grammar, diagnostic, or ABI change.

## Completion

Completed on 2026-11-04. `make verify` exit 0; both A500 gates passed. The Boing
run produced `boing: checksum 0xF58C7129` and `boing: frames 400 ticks 400`
(50 Hz). Registry Bundle 1 is 11/11 done; issue #2 is closed.
