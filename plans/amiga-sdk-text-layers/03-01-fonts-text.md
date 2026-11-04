# Design: Fonts and Text

> **Document**: 03-01-fonts-text.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1, R2, R9–R10; AR-2–AR-4

## Module surface

`std.amiga.text` is available on `hosted-amiga` and `hosted-amiga-13`; the host
and freestanding profiles reject it with `E11200`.

```text
export handle Font freedBy cl_font_free;

export func openFont(name: cstring, size: u16, style: u8): Font?
export func openDiskFont(name: cstring, size: u16, style: u8): Font?
export func defaultFont(): Font?
export func freeFont(take font: Font): void
export func textWidth(borrow font: Font, const text: u8[]): i32
export func textHeight(borrow font: Font): i32
export func drawText(borrow screen: Screen, borrow font: Font,
                     x: i32, y: i32, pen: u8, const text: u8[]): void
```

The module imports `Screen` from `std.amiga.intuition`.

## Font handle

`openFont` builds a `TextAttr` with the name, Y size, style, and zero flags and
calls `graphics.library/OpenFont`. On the A500 the ROM `topaz.font` opens
successfully; the measured size for `Clorraine layers` is 128x8. `openDiskFont`
opens `diskfont.library` at version 33 and calls `OpenDiskFont`. A bare ADF has
no `FONTS:` volume, so the demo's disk-font attempt returns `null` and the row
is deferred; the ROM font remains the visible path.

`defaultFont` wraps `GfxBase->DefaultFont` with `owns = 0`; its deleter does not
call `CloseFont`. Owned fonts close with `CloseFont`. The acceptance ADF also
has no `LIBS:diskfont.library`/`FONTS:` pair, so the demo's `openDiskFont` call
returns `null`; the demo reports it as `checks 126` (the disk bit stays 0).

## Measurement

`textWidth` uses a temporary `RastPort`:

```text
InitRastPort(&rp)
SetFont(&rp, font)
TextLength(&rp, text, len)
```

`textHeight` returns the font's `tf_YSize`. The text is a byte view; the Amiga
font glyphs are 8-bit, so the bytes are passed through without encoding.

## Drawing

`drawText` uses the screen's `RastPort`:

```text
SetFont(&screen->RastPort, font)
SetAPen(&screen->RastPort, pen)
Move(&screen->RastPort, x, y)
Text(&screen->RastPort, text, len)
```

The screen keeps ownership; the caller must keep the font alive while drawing.
The shim saves and restores the screen RastPort's font around the draw, so a
later `freeFont` cannot leave a dangling font pointer in the RastPort. The
screen's depth bounds `pen`. The layer text path does the same for the layer's
RastPort.

## Errors

- A missing font returns `null`; the caller may fall back to `defaultFont`.
- A text view is passed as bytes; a null/empty view draws nothing.
- The font and screen are never freed by the text functions.
- `freeFont` on the borrowed default font only releases the wrapper; it never
  closes the ROM font.
