# Runtime Shim: Clorraine Amiga Graphics

> **Document**: 03-03-runtime-shim.md
> **Parent**: [Index](00-index.md)

## Overview

The hosted-amiga C shim (`compiler/runtime/shim_hosted_amiga.c`) implements the
`ffi` symbols the two graphics modules bind. It opens `graphics.library` and
`intuition.library`, owns the `RastPort` calls, and stays warning-clean under
the profile's `-Wall -Wextra -Werror`. (AR-9, AR-23, AR-24)

## Architecture

### Current Architecture

The shim already defines the IO, memory, text, DOS, and VBlank-interrupt
symbols. The spike added a provisional graphics section; this plan promotes it
and adds `cl_draw` and `cl_clear`. (AR-2)

### Proposed Changes

Add four NDK includes and six functions. The two library bases are file-scope
globals opened on the first screen open. No new dependency, no build change.
(AR-23, AR-24)

## Implementation Details

### New Includes

```c
#include <proto/graphics.h>
#include <proto/intuition.h>
#include <intuition/screens.h>
#include <graphics/rastport.h>
```

Added next to the existing `<proto/exec.h>` include. `<exec/types.h>` is already
included, so `LONG`, `UBYTE`, `STRPTR`, and the rest are in scope. (AR-24)

### New Globals and Functions

```c
struct GfxBase *GfxBase = NULL;
struct IntuitionBase *IntuitionBase = NULL;

/* Open a custom screen; NULL on failure. */
void *cl_open_screen(LONG width, LONG height, UBYTE depth, char *title);

/* Set screen color `pen` from 4-bit R/G/B components. */
void cl_set_color(void *screen, UBYTE pen, UBYTE r, UBYTE g, UBYTE b);

/* Fill the rectangle (x1,y1)-(x2,y2) with `pen`. */
void cl_fill(void *screen, LONG x1, LONG y1, LONG x2, LONG y2, UBYTE pen);

/* Draw the line (x1,y1)-(x2,y2): Move then Draw. */
void cl_draw(void *screen, LONG x1, LONG y1, LONG x2, LONG y2, UBYTE pen);

/* Fill the whole screen: RectFill over (0,0)-(Width-1, Height-1). */
void cl_clear(void *screen, UBYTE pen);

/* Close a screen. Null-safe. */
void cl_close_screen(void *screen);
```

Implementation notes:

- `cl_open_screen` opens each library once if its base is still `NULL`, then
  fills a `struct NewScreen` (`CUSTOMSCREEN`, `Depth = depth`, `Width`/`Height`
  cast to `WORD`, `DefaultTitle` cast to `STRPTR`) and returns `OpenScreen`.
  (AR-8, AR-23)
- `title` is a caller-owned `cstring`: the module documents that it must outlive
  the screen, because the shim passes the pointer straight to `OpenScreen`
  without copying it. Task 1.3.4 verifies `OpenScreen`'s copy-vs-store behavior
  and records the exact contract. (PF-004)
- `cl_set_color` calls `SetRGB4(&screen->ViewPort, pen, r >> 4, g >> 4, b >> 4)`.
  (AR-12)
- `cl_fill` calls `SetAPen` then `RectFill`; `cl_draw` calls `SetAPen`,
  `Move`, then `Draw`; `cl_clear` calls `SetAPen` then `RectFill` over the
  screen's own `Width`/`Height`. (AR-13, AR-15)
- `cl_close_screen` calls `CloseScreen` only when the handle is non-null.
  (AR-17)
- Every drawing function null-checks its handle before touching the `RastPort`,
  so a `NULL` from a failed open is safe even outside the compiler's narrowing.
  (AR-16)

### ABI Mapping

| Clorraine | C shim | Why |
|---|---|---|
| `Screen` handle | `void *` (`ClScreen`) | A contract-less handle lowers to the opaque `Cl<Name>` pointer typedef (AR-18) |
| `i32` | `LONG` | The hosted-amiga ABI width |
| `u8` | `UBYTE` | One byte |
| `cstring` | `char *` | The FFI string spelling |

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| `OpenLibrary` fails | The base stays `NULL`; `cl_open_screen` returns `NULL` | AR-16, AR-23 |
| `OpenScreen` fails | Returns `NULL` | AR-16 |
| Null screen passed to a drawing call | Null-checked; no-op | AR-16 |
| A warning under `-Werror` | Explicit casts on every narrowing/sign conversion; additive const only | AR-24 |

> **Traceability:** Every error-handling strategy and design choice references
> the Ambiguity Register entry that resolved it; see
> `00-ambiguity-register.md`.

## Testing Requirements

- The existing stdlib unit test `each_profile_shim_defines_every_ffi_symbol_its_modules_bind`
  (`compiler/src/stdlib.rs:397`) asserts the hosted-amiga shim defines every
  `ffi` symbol the graphics modules bind. (AR-22)
- The hosted-amiga corpus build compiles the shim under the profile's warning
  flags, proving `-Werror` cleanliness. (AR-24)
