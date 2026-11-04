# Requirements: Amiga SDK Bundle 2 — Sprites and Collision

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Functional Requirements

### Must Have

- **R1 — Eight channels.** `newSprite(channel, height)` allocates one of the
  eight OCS channels (0..7), with a chip-memory sprite image of the requested
  height (1..255). A channel already owned returns `null`.
- **R2 — Pixels.** `setPixels(borrow sprite, const pixels: u8[])` accepts
  `16 * height` values in 0..3 and packs the two sprite bitplanes.
- **R3 — Movement.** `move(borrow sprite, x, y)` positions the sprite
  (x 0..511, y 0..255); `show`/`hide` toggle the channel without freeing it.
- **R4 — Attached pairs.** `attach(borrow even, borrow odd)` requires adjacent
  channels with the same height and screen, sets the odd channel's ATTACH bit
  after `ChangeSprite`, and yields the 16-color pair. The caller sets the two
  halves' pixels and moves both to the same position.
- **R5 — Collision.** `enableCollisions()` writes `CLXCON = 0xF000` (all sprite
  pairs, odd channels included); `collisions()` reads and clears `CLXDAT`.
  Sprite-sprite bits are 9..14; the demo parks pair 2/3 and single 4 together
  and checks bit 12 (`0x1000`).
- **R6 — Animation.** `newAnimation(channel, height, frames)` owns one channel
  and a fixed set of frame buffers; `setFrame`, `showFrame`, `next`, `move`,
  `show`, `hide`, and `free` cover a simple animation.
- **R7 — Demo.** `examples/amiga/sprite-demo.clo` opens a 320x256 Intuition
  screen, draws a static background, shows a 16-color attached pair on channels
  2/3, a single sprite on 4, and a four-frame animation on 6, forces a known
  overlap, holds a deterministic frame, and prints
  `sprite-demo: collision 0x1000`.
- **R8 — Runtime proof.** `docker/scripts/a500-sprite-smoke.sh` boots the demo
  on the real KS1.3 ROM, compares the 640x512 held display crop and the
  collision line to committed goldens, and fails if either is missing.
- **R9 — Compatibility.** The module is available on `hosted-amiga` and
  `hosted-amiga-13`, rejected by host, and leaves the 3.1 and A500 gates green.
- **R10 — Documentation.** Update the appendices, `AGENTS.md`, registry,
  coverage, issue #3, and the programme/roadmap.

### Won't Have (Out of Scope)

- More than 16 pixels per sprite; AGA wide sprites.
- Playfield collision match-value authoring beyond the raw `CLXCON` default.
- Sprite scaling, genlock, or hardware playfield animation.
- Any language, grammar, diagnostic, or ABI change.

## Technical Requirements

- The sprite image is chip memory; the pointer is word-aligned.
- `DMACON` sprite DMA is enabled by every show/move path, so a display
  `LoadView` cannot leave the channels disabled.
- The image has a two-word zero terminator after the last line.
- The attached pair uses the sprite palette range 16..31; the demo sets those
  colors through `std.amiga.custom.setColor`.

## Acceptance Criteria

1. The sprite module compiles on both hosted profiles and cross-builds with
   `nix13`/`nix20` under strict warnings; host rejects it.
2. The spec tests cover channel allocation, pixel packing, movement,
   show/hide, attachment, collision, and animation.
3. The A500 demo shows the sprites over the Intuition screen and prints the
   deterministic `sprite-demo: collision 0x1000` line.
4. `make check`, `make verify`, `make sdk-registry`, and the sprite smoke pass.
5. The hello and Boing A500 gates stay green.
6. Registry Bundle 2 is 6/6 done; issue #3 closed.
