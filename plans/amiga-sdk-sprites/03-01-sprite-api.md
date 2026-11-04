# Design: Sprite API

> **Document**: 03-01-sprite-api.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1–R6, R9; AR-2–AR-8, AR-11

## Module surface

`std.amiga.sprite` is available on `hosted-amiga` and `hosted-amiga-13`; the
host and freestanding profiles reject it with `E11200`. The module uses the
1.3 `graphics.library` simple-sprite machine (`GetSprite`, `ChangeSprite`,
`MoveSprite`, `FreeSprite`) and attaches sprites to an Intuition `Screen`
viewport.

```text
export handle Sprite freedBy cl_sprite_free;
export handle SpriteAnimation freedBy cl_sprite_animation_free;
```

## Sprite handle

The shim record stores the `SimpleSprite`, channel, height, position,
visibility, ATTACH bit, the chip-memory image and blank buffers, and the
attached `Screen`.

| API | Returns | Rule |
|---|---|---|
| `newSprite(channel: u8, height: u8)` | `Sprite?` | channel 0..7, height 1..255; `null` if the channel is taken or chip memory fails |
| `freeSprite(take sprite)` | `void` | hides the channel, frees the images, and releases the channel |
| `spriteSetPixels(borrow sprite, const pixels: u8[])` | `void` | `16 * height` values 0..3; wrong length is ignored |
| `spriteMove(borrow sprite, x: i32, y: i32)` | `void` | x 0..511, y 0..255; `MoveSprite` relative to the screen viewport |
| `spriteShow(borrow sprite)` | `void` | installs the image and rebuilds the screen display list |
| `spriteHide(borrow sprite)` | `void` | installs the zero image |
| `spriteAttach(borrow even, borrow odd)` | `bool` | same height, same screen, adjacent channels; sets the odd ATTACH bit after `ChangeSprite` |
| `spriteAttachScreen(borrow sprite, borrow screen)` | `bool` | sets the `SPRITES` flag, adds the sprite, and calls `RethinkDisplay` |
| `spriteDetach(borrow sprite)` | `void` | clears the screen back-pointer; call after hiding and before closing the screen |
| `spriteChannel`, `spriteHeight`, `spriteX`, `spriteY`, `spriteVisible` | values | read-only accessors |

## Pixel packing

For line `y` and pixel `x` (0..15), the two words are:

```text
wordA = sum(((pixel & 1) << (15 - x)) for x in 0..15)
wordB = sum((((pixel >> 1) & 1) << (15 - x)) for x in 0..15)
image[0] = 0; image[1] = 0          // posctl header, written by MoveSprite
image[2 + y*2] = wordA
image[3 + y*2] = wordB
image[2 + height*2] = 0
image[3 + height*2] = 0             // reserved
```

The image is `(height * 2 + 4) * 2` bytes in chip memory, word-aligned. The
`SimpleSprite.height` is set before `ChangeSprite`.

## Screen attachment

`GetSprite` reserves a channel and fills the `SimpleSprite` number.
`ChangeSprite(&screen->ViewPort, &sprite, image)` adds the sprite to the
viewport's sprite list; `MoveSprite` writes the live position words.
`RethinkDisplay` rebuilds the screen's display list after an image-pointer
change. The screen and frame-detach path is:

```text
spriteHide -> spriteDetach -> close screen -> freeSprite
```

The shim also tracks channels per screen and clears the back-pointer when the
screen closes, so `freeSprite` after a close is safe even if `spriteDetach` was
omitted. The preferred order remains detach, close, free.

## Attached pair

An attached pair is two `Sprite` handles on adjacent channels with the same
height. The even sprite carries the low two bits and the odd sprite carries the
high two bits; the odd ATTACH bit (`0x80` in `posctl[1]`) makes the hardware
combine them. The pair uses sprite palette registers 16..31. The caller moves
both to the same position.

## Collision

`enableCollisions()` writes `CLXCON = 0xF000`: all four sprite-pair groups
include their odd channel. `collisions()` reads and clears `CLXDAT`.

| Bit | Meaning |
|---|---|
| 9 | sprite 0/1 vs sprite 2/3 |
| 10 | sprite 0/1 vs sprite 4/5 |
| 11 | sprite 0/1 vs sprite 6/7 |
| 12 | sprite 2/3 vs sprite 4/5 |
| 13 | sprite 2/3 vs sprite 6/7 |
| 14 | sprite 4/5 vs sprite 6/7 |
| 1–8 | playfield collisions (raw; the demo masks them out) |

The demo parks the pair on channels 2/3 and the single on channel 4; their
overlap sets bit 12 (`0x1000`).

## Animation

`SpriteAnimation` owns one `Sprite` channel and `frames` chip images:

| API | Rule |
|---|---|
| `newAnimation(channel: u8, height: u8, frames: u8)` | 1..8 frames; `null` on failure |
| `animationSetFrame(borrow anim, frame: u8, const pixels: u8[])` | packs one frame |
| `animationShowFrame(borrow anim, frame: u8)` | installs and rebuilds the frame |
| `animationNext(borrow anim)` | advances modulo the frame count |
| `animationMove`, `animationShow`, `animationHide` | delegate to the sprite |
| `animationAttachScreen` / `animationDetach` | attach/detach the animation's sprite |
| `freeAnimation(take anim)` | frees all frames and the channel |

## Error behavior

- A channel already owned returns `null`.
- Height 0, a pixel count mismatch, an out-of-range position, or a pair with
  different heights/screens is ignored or rejected.
- A failed chip allocation frees the partial record and returns `null`.
- The shim never traps on caller input.
