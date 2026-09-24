# Drawing: Clorraine Amiga Graphics

> **Document**: 03-02-drawing.md
> **Parent**: [Index](00-index.md)

## Overview

`std.amiga.graphics` owns the drawing calls: color setup, rectangle fill, line
draw, and clear. Every call takes `borrow screen: Screen`, so the caller keeps
ownership and the `RastPort` stays hidden inside the shim. (AR-6, AR-7, AR-18)

## Architecture

### Current Architecture

The spike has `setColor` and `fill` in one provisional module that also owns the
`Screen` handle; there is no `draw` or `clear`, and no separate drawing module.
The plan rewrites that module into the drawing half of the split (PF-001).

### Proposed Changes

Rewrite the spike module `stdlib/std.amiga.graphics.clo` into the drawing half:
it imports `Screen` from `std.amiga.intuition` ([03-01](03-01-screen-lifecycle.md)),
drops the `Screen`/`openScreen` declarations, and adds `draw`/`clear`. The
cross-module import is handled per AR-35. Update the registration in
`compiler/src/stdlib.rs` (already present from the spike).

## Implementation Details

### New Types/Interfaces

```clorraine
module std.amiga.graphics;

import { Screen } from std.amiga.intuition;

ffi stdgraphics {
    func cl_set_color(borrow screen: Screen, pen: u8, r: u8, g: u8, b: u8): void;
    func cl_fill(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8): void;
    func cl_draw(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8): void;
    func cl_clear(borrow screen: Screen, pen: u8): void;
}

/// Sets screen color `pen` from `r`, `g`, `b` (0-255 each).
export func setColor(borrow screen: Screen, pen: u8, r: u8, g: u8, b: u8): void {
    cl_set_color(screen, pen, r, g, b);
}

/// Fills the rectangle (`x1`,`y1`)-(`x2`,`y2`) with `pen`.
export func fill(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8): void {
    cl_fill(screen, x1, y1, x2, y2, pen);
}

/// Draws the line (`x1`,`y1`)-(`x2`,`y2`) with `pen`.
export func draw(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8): void {
    cl_draw(screen, x1, y1, x2, y2, pen);
}

/// Fills the whole screen with `pen`.
export func clear(borrow screen: Screen, pen: u8): void {
    cl_clear(screen, pen);
}
```

### New Functions/Methods

| Name | Signature | Contract |
|---|---|---|
| `setColor` | `(borrow screen, pen: u8, r: u8, g: u8, b: u8): void` | Maps 8-bit components to the Amiga 4-bit palette (`SetRGB4`). (AR-12, AR-14) |
| `fill` | `(borrow screen, x1..y2: i32, pen: u8): void` | Inclusive rectangle; the `RectFill` primitive. (AR-7) |
| `draw` | `(borrow screen, x1..y2: i32, pen: u8): void` | Arbitrary line; the `Draw` primitive. (AR-7, AR-13) |
| `clear` | `(borrow screen, pen: u8): void` | Fills the whole screen rectangle. (AR-15) |

### Integration Points

- `compiler/src/stdlib.rs` — `MODULES` gains
  `["setColor", "fill", "draw", "clear"]` for `std.amiga.graphics`.
- [03-03-runtime-shim.md](03-03-runtime-shim.md) — `cl_set_color`, `cl_fill`,
  `cl_draw`, `cl_clear`. (AR-9)
- `std.amiga.intuition` — the `Screen` type the calls borrow. (AR-5, AR-35)

## Code Examples

### Example 1: draw two bars and a diagonal

```clorraine
import { Screen, openScreen } from std.amiga.intuition;
import { setColor, fill, draw, clear } from std.amiga.graphics;

func main(): i32 {
    let screen: Screen? = openScreen(320, 256, 4, "Bars");
    if (screen == null) { return 1; }
    setColor(screen, 1, 255, 0, 0);
    setColor(screen, 2, 0, 255, 0);
    clear(screen, 0);
    fill(screen, 10, 10, 150, 60, 1);
    draw(screen, 0, 0, 319, 255, 2);
    return 0;
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| Out-of-range pen or color index | Caller contract; out-of-range indices are undefined by this wrapper. Clipping applies to coordinates, not pens (`SetAPen`/`SetRGB4` take an index the screen depth or color map bounds) | AR-14 |
| Drawing with a null screen | The nullable rule (Ch 02 §5) rejects using a `Screen?` before a `!= null` check (`E10010`); the shim is null-safe as well | AR-16, AR-17 |
| Coordinates outside the screen | `graphics.library` clips coordinates; the wrapper does not fault | AR-13 |

> **Traceability:** Every error-handling strategy and design choice references
> the Ambiguity Register entry that resolved it; see
> `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: a program importing both modules compiles clean for
  hosted-amiga; the corpus program prints the sentinel and runs in FS-UAE.
  ([07](07-testing-strategy.md) ST-2–ST-5)
- Implementation tests: the drawing `ffi` symbols are defined by the
  hosted-amiga shim (the existing stdlib unit test already asserts this for
  every module); a focused test pins the exports. (AR-11, AR-22)
