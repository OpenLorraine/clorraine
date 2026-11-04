# Requirements: Amiga SDK Bundle 1 — Native Video

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Give Clorraine programs on the A500 a native-video API: chip memory, bitmaps,
drawing, blitter copies, a copper list builder, a double-buffered custom
display, a 4-bit palette, fixed-point math, and a deterministic frame checksum.
The acceptance is a Boing demo running on the real Kickstart 1.3 machine.

## Functional Requirements

### Must Have

- **R1 — Chip memory.** `allocChip(size)` returns an owned `ChipBuffer` in
  `MEMF_CHIP`; `freeChip` releases it; byte/word/long read and write helpers are
  available for plane-adjacent work. A failed allocation returns `null`.
- **R2 — Bitmaps.** `newBitmap(width, height, depth)` allocates a displayable
  bitmap in chip memory and returns `null` on failure. Width, height, depth,
  bytes-per-row, plane-bit read/write, and free are exposed.
- **R3 — Software drawing.** `clear`, `plot`, `line`, and `fill` write the
  selected pen into every plane of a bitmap. Coordinates outside the bitmap are
  clipped; pen bits above the depth are ignored.
- **R4 — Blitter.** `blit` copies a rectangle with `BltBitMap` and minterm
  `0xC0`; `blitMasked` copies through a single-plane mask with
  `BltMaskBitMapRastPort` and minterm `0xE0`. Both end with `WaitBlit()` before
  returning, and both reject requests wider than 992 pixels or taller than 1024
  rows (the OCS blitter limit).
- **R5 — Copper.** `newCopper` builds a 1.3 user copper list; `copperWait`,
  `copperMove`, `copperColor`, and `copperEnd` append instructions. A display
  accepts the list before its first flip and restores the previous system view
  when freed.
- **R6 — Display.** `openDisplay(front, back)` borrows two caller-owned bitmaps
  and creates two `View`/`ViewPort` pairs. Bundle 1 is a low-resolution PAL
  view: width 1..=320, height 1..=256, depth 1..=5. The first `flip` attaches
  a queued copper list, builds the copper lists, and loads the first view;
  later flips wait for VBlank and alternate. `index` reports which bitmap is
  visible. `closeDisplay` restores the saved `GfxBase->ActiView`.
- **R7 — Palette.** `setColor(display, index, r, g, b)` updates the 4-bit
  `ColorMap` before the first flip. `blit`-visible output uses the palette
  baked into the views.
- **R8 — Math.** `std.math.sin(angle)` and `cos(angle)` cover a 256-step turn
  and return -256..256; `sqrt(value)` returns the integer floor root. They are
  pure Clorraine, deterministic, and available on `host` and both hosted-Amiga
  profiles.
- **R9 — Frame checksum.** `checksum(bitmap)` returns FNV-1a 32-bit over the
  plane bytes, seeded with depth/width/height, so a fixed frame has a stable
  value across runs.
- **R10 — Boing.** `examples/amiga/boing.clo` draws a static background, bounces
  a masked ball with the blitter, uses the copper for a per-line background
  gradient, flips two bitmaps, parks the ball at a fixed position for the held
  acceptance frame, and prints `boing: checksum <8 hex digits>` after restoring
  the system view.
- **R11 — Runtime proof.** `docker/scripts/a500-boing-smoke.sh` decrypts the
  licensed KS1.3 ROM, boots the demo as an A500, compares the held display crop
  to a committed golden image, and compares the final checksum line to a
  committed golden crop.
- **R12 — Documentation and tracking.** Update `spec/appendix-amiga.md`,
  `spec/appendix-stdlib.md`, `AGENTS.md`, the registry/coverage, issue #2, and
  the programme/roadmap rows.

### Should Have

- **R13 — Timing.** The demo reports the rendered frame count; the A500 run is
  expected to sustain near 50 frames per second with the small masked blit.
- **R14 — Failure diagnostics.** Constructor failures are distinguishable in the
  demo output (`boing: allocation failed`, `boing: display failed`).

### Won't Have (Out of Scope)

- Sprites, collision, audio, input, text/fonts/layers, DOS assets, Intuition
  UI, Exec, Workbench, AGA, HAM, and 24-bit palette.
- Runtime palette changes after the first flip; Bundle 1 uses a static palette
  plus copper instructions.
- A public raw `COP1LC` takeover API; the copper is a user list owned by the
  display.
- Any language, grammar, diagnostic, or ABI change.

## Technical Requirements

### Compatibility

- `hosted-amiga` (3.1) keeps every existing API and test. The new modules are
  additive.
- `hosted-amiga-13` accepts `std.amiga.video` and `std.math`; `std.amiga.exec`
  stays rejected.
- The emitted C remains C99 and builds with `-Wall -Wextra -Werror` on both
  `-mcrt=nix13` and `-mcrt=nix20`.

### Performance

- No full-frame copy in the Boing loop. Each frame restores the previous ball
  rectangle from a static background tile and blits the masked ball once.
- The display uses one `LoadView` per frame plus `WaitTOF`; copper lists are
  compiled once before the first flip.

### Security

- No network, credential, file, or user-input surface. The shim only touches
  chip memory and the graphics/copper hardware through documented 1.3 calls.
- All lengths and coordinates are bounds-checked before touching memory.

## Scope Decisions

| Decision | Options | Chosen | Rationale | AR Ref |
|---|---|---|---|---|
| Module split | One video module; video + math | `std.amiga.video` + `std.math` | Math is platform-independent and host-testable | AR-2 |
| Display base | Intuition screen; custom view | Custom `LoadView` view | Full control, double buffer, copper; spike-verified | AR-4 |
| Blitter | Raw registers; graphics.library | graphics.library | 1.3-safe, waits for the blitter, less fragile | AR-6 |
| Palette | Runtime rebuild; static + copper | Static + copper | Avoids per-frame copper rebuild; matches the demo | AR-7 |
| Math | libm doubles; fixed point | Fixed point | No soft-float cost; deterministic | AR-8 |
| Checksum | CRC32; FNV-1a | FNV-1a 32 | Small, dependency-free, stable | AR-9 |

## Acceptance Criteria

1. A `hosted-amiga-13` program importing `std.amiga.video` and `std.math`
   compiles, emits C, and cross-builds with `nix13` under strict warnings.
2. The same program is rejected by the `host` profile with `E11200`.
3. A host program using `std.math` prints the expected sine/cosine/sqrt values.
4. `boing.clo` builds for `hosted-amiga-13` and boots on FS-UAE A500 with the
   real KS1.3 ROM; the live display is captured.
5. The final `boing: checksum ...` line matches the committed golden crop.
6. The 3.1 graphics/Exec suites stay green.
7. `make check`, `make verify`, `make sdk-registry`, and the Boing smoke pass.
8. Registry Bundle 1 is 11/11 done; issue #2 closed; roadmap/programme updated.
