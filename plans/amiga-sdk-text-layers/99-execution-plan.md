# Execution Plan: Amiga SDK Bundle 4 — Text, Fonts, Layers

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-04 20:45
> **Progress**: 9/9 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Three phases: pin the text/layers surface with specification tests, implement
fonts/text and layers/clipping, then build and prove the scrolling demo on the
real A500/KS1.3 machine. Specification tests come before implementation.

## Phase 1: Specification tests (red)

> **Reasoning**: high — two new public modules and font/layer lifetimes.

**Tasks**: 2

- [x] 1.1.1 Write `compiler/tests/amiga_text_spec_test.rs` for ST-1–ST-3 and ST-6; run it red
- [x] 1.1.2 Write `compiler/tests/amiga_text_impl_test.rs` for ST-4; run it red

**Verify**: `cargo test --test amiga_text_spec_test --test amiga_text_impl_test`

## Phase 2: Fonts, text, and layers implementation

> **Reasoning**: high — library ownership and layer clipping.

**Tasks**: 4

- [x] 2.1.1 Add `stdlib/std.amiga.text.clo` and `std.amiga.layers.clo`; register both and add them to the 1.3 whitelist; run ST-1–ST-3 green
- [x] 2.1.2 Implement `OpenFont`/`OpenDiskFont`/`DefaultFont`, text measure/draw in the shim; cross-build with `nix13`/`nix20`
- [x] 2.1.3 Implement layers create/free/move/fill/text/update and the bitmap bounds checks; cross-build
- [x] 2.1.4 Run the full `cargo test --workspace` and `make check`

**Verify**: `cargo test --workspace`; `make check`

## Phase 3: Scrolling demo and runtime close-out

> **Reasoning**: high — real KS1.3 emulator acceptance with font and layer clipping.

**Tasks**: 3

- [x] 3.1.1 Add `examples/amiga/scroller.clo`; build it for `hosted-amiga-13`; verify the font, measure, layer, clipping, and move on the A500
- [x] 3.1.2 Add `docker/scripts/a500-text-smoke.sh` and `make sdk-a500-text`; capture the held screen and text line; add the leg to `make verify`
- [x] 3.1.3 Update the appendices, `AGENTS.md`, registry/coverage, issue #5, programme/roadmap; run `make verify` and `make sdk-registry`; close the plan

**Verify**: `make verify`; `make sdk-registry`; `docker/scripts/a500-text-smoke.sh build/clorraine/scroller/hosted-amiga-13/scroller`

## Dependencies

- Phase 1 is the red oracle.
- Phase 2.1.2/2.1.3 depend on 2.1.1; 2.1.4 depends on both.
- Phase 3 depends on Phase 2.
- The A500 smoke depends on the licensed ROM/`rom.key`, the host X display, and
  the Bundle 1 screen path.

## Success Criteria

1. All 9 tasks complete.
2. The two modules compile on both hosted profiles and are rejected by host.
3. The A500 scroller shows the clipped text and prints the deterministic line.
4. The four earlier A500 gates stay green.
5. Registry Bundle 4 is at least 4/6 done with regions deferred; issue #5
   closed.
6. No language, grammar, diagnostic, or ABI change.

## Completion

Completed on 2026-11-04. `make verify` exits 0; the hello, Boing, sprite,
paddle, and text A500 gates all pass. The text run holds the clipped layer frame
and prints `text: ok width 128 height 8 layer 160x64 checks 510`. The diskfont
and 2.0+ regions rows are deferred. Registry Bundle 4 is 4/6 done; issue #5 is
closed.
