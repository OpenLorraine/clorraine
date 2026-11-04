# Execution Plan: Amiga SDK Bundle 2 — Sprites and Collision

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-04 18:45
> **Progress**: 8/8 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Three phases: pin the sprite API with specification tests, implement the raw
OCS sprite channels and animation, then build and prove the sprite demo on the
real A500/KS1.3 machine. Specification tests come before implementation.

## Phase 1: Specification tests (red)

> **Reasoning**: high — new public API and hardware register path.

**Tasks**: 2

- [x] 1.1.1 Write `compiler/tests/amiga_sprite_spec_test.rs` for ST-1–ST-3 and ST-6; run it red because the module and shim do not exist
- [x] 1.1.2 Write `compiler/tests/amiga_sprite_impl_test.rs` for ST-4; run it red

**Verify**: `cargo test --test amiga_sprite_spec_test --test amiga_sprite_impl_test`

## Phase 2: Sprite implementation

> **Reasoning**: high — custom-chip registers, DMA pointers, and collision.

**Tasks**: 3

- [x] 2.1.1 Add `stdlib/std.amiga.sprite.clo`; register it in `compiler/src/stdlib.rs` and the `hosted-amiga-13` whitelist; run ST-1–ST-3 green
- [x] 2.1.2 Implement sprite channel allocation, pixel packing, move/show/hide, attached pairs, collision registers, and free in the hosted-Amiga shim; cross-build with `nix13`/`nix20`
- [x] 2.1.3 Implement `SpriteAnimation` frame buffers and pointer switching; run the full `cargo test --workspace`

**Verify**: `cargo test --workspace`; `make check`

## Phase 3: Sprite demo and runtime close-out

> **Reasoning**: high — real KS1.3 emulator acceptance and deterministic collision evidence.

**Tasks**: 3

- [x] 3.1.1 Add `examples/amiga/sprite-demo.clo`; build it for `hosted-amiga-13`; verify the pair, single sprite, and collision on the A500
- [x] 3.1.2 Add `docker/scripts/a500-sprite-smoke.sh` and `make sdk-a500-sprites`; capture the held display and collision line; add the leg to `make verify`
- [x] 3.1.3 Update the appendices, `AGENTS.md`, registry/coverage, issue #3, programme/roadmap; run `make verify` and `make sdk-registry`; close the plan

**Verify**: `make verify`; `make sdk-registry`; `docker/scripts/a500-sprite-smoke.sh build/clorraine/sprite-demo/hosted-amiga-13/sprite-demo`

## Dependencies

- Phase 1 is the red oracle.
- Phase 2.1.2 depends on 2.1.1; 2.1.3 depends on 2.1.2.
- Phase 3 depends on Phase 2.
- The A500 smoke depends on the licensed ROM/`rom.key`, the host X display, and
  the existing Bundle 1 display.

## Success Criteria

1. All 8 tasks complete.
2. The sprite module compiles on both hosted profiles and is rejected by host.
3. The A500 demo shows the pair and single sprite and prints the deterministic
   collision line.
4. The hello and Boing A500 gates stay green.
5. Registry Bundle 2 is 6/6 done; issue #3 closed.
6. No language, grammar, diagnostic, or ABI change.

## Completion

Completed on 2026-11-04. `make verify` exits 0; the hello, Boing, and sprite
A500 gates all pass. The sprite demo holds its display and prints
`sprite-demo: collision 0x1000`. Registry Bundle 2 is 6/6 done; issue #3 is
closed.
