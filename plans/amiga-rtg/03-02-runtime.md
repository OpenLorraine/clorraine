# Runtime Shim: Picasso96

> **Document**: 03-02-runtime.md
> **Parent**: [Index](00-index.md)

## File and Placement

`compiler/runtime/shim_hosted_amiga_optional_rtg.c` is appended to
`HOSTED_AMIGA_SUFFIX` after the network/AmigaGuide suffix. It is emitted only
for `hosted-amiga`; `hosted-amiga-13` keeps an empty suffix, so the A500/KS1.3
translation unit never sees P96 headers or symbols.

`profile.rs` embeds the vendored `compiler/runtime/picasso96.h` directly
before the RTG shim with `include_str!`, so the generated C has no external
include path. The header provides:

- `CL_P96_RGBFTYPE` values and the `CL_P96_RGBFB_*`/`CL_P96_RGBFF_*` masks used by mode discovery.
- `CL_P96_BIDTAG_*`, `CL_P96_SA_*`, `CL_P96_BMA_*`, `CL_P96_IDA_*`, and `CL_P96_RD_*` constants.
- `struct CL_P96_RenderInfo` and `CL_P96_BMF_USERPRIVATE`.
- The fixed-argument `LP*` call macros for GCC, using the documented library
  offsets from the P96 developer package. For vbcc, the header provides plain
  prototypes; the SDK `amiga.lib` supplies the P96 stubs used by the build-only
  leg. The full P96 varargs macro forms are omitted because vbcc rejects them;
  the shim calls only fixed-argument functions. The runtime gate is GCC.

The header is a minimal, attributed definition file. It is not a replacement
for the full P96 SDK and does not include PIP/overlay or palette APIs.

## Library and Board

```c
struct Library *P96Base;

static int cl_rtg_open(void)
{
    if (P96Base == NULL) {
        P96Base = OpenLibrary((STRPTR)"Picasso96API.library", 2);
    }
    return P96Base != NULL;
}
```

`cl_rtg_boards` fills `CL_P96_RD_NumberOfBoards` through
`p96GetRTGDataTagList`. A missing library returns 0. The base stays open for
the process lifetime; there is no per-call open/close race.

## Mode Selection

`cl_rtg_mode(width, height, depth)`:

1. Rejects dimensions outside 1..4096 and depths other than 24 or 32.
2. Maps depth 24 to `CL_P96_RGBFF_TRUECOLOR` and depth 32 to
   `CL_P96_RGBFF_TRUEALPHA`.
3. Calls `p96BestModeIDTagList` with `CL_P96_BIDTAG_NominalWidth`,
   `NominalHeight`, `Depth`, and `FormatsAllowed`.
4. Returns 0 for `0xFFFFFFFF` (P96's `INVALID_ID`).

`cl_rtg_mode_width/height/depth/format` use `p96GetModeIDAttr` with the
corresponding `CL_P96_IDA_*` attribute.

## Screen

`cl_rtg_open_screen` validates the arguments, calls `cl_rtg_mode`, then:

```c
tags[0] = CL_P96_SA_DisplayID, mode;
tags[1] = CL_P96_SA_Title, title;
tags[2] = CL_P96_SA_DClip, &handle->dclip;
tags[3] = CL_P96_SA_ShowTitle, FALSE;
tags[4] = CL_P96_SA_Quiet, TRUE;
tags[5] = TAG_DONE;
screen = p96OpenScreenTagList(tags);
```

The handle stores the full-screen `dclip` from the actual P96 mode dimensions
(`0,0` to `mode_width-1,mode_height-1`), a copy of the title, and the mode.
The title is freed after a successful `p96CloseScreen`; a failed close leaks
the record rather than freeing memory the live screen may still reference. `cl_rtg_close_screen` calls
`p96CloseScreen` once and frees the handle. Screen attributes come from
`p96GetBitMapAttr(screen->RastPort.BitMap, CL_P96_BMA_*)`; the shim never reads
the `BitMap` plane pointers.

## Offscreen Bitmap

`cl_rtg_alloc_bitmap` accepts depth 24 or 32 and calls:

```c
bitmap = p96AllocBitMap(width, height, depth, CL_P96_BMF_USERPRIVATE, NULL,
                        depth == 32 ? CL_P96_RGBFB_A8R8G8B8
                                    : CL_P96_RGBFB_B8G8R8);
```

The handle owns a `struct RastPort`, initialized with `InitRastPort` and
`rp.BitMap = bitmap`. `cl_rtg_free_bitmap` calls `p96FreeBitMap` and frees the
handle. `CL_P96_BMA_BYTESPERROW` is exposed through `rtgBitmapBytesPerRow`.

## Drawing

- Screen writes use `p96WritePixel(&screen->RastPort, x, y, color)`.
- Screen and bitmap reads use a 1x1 `p96ReadPixelArray` read and convert RGB
  back to ARGB. `p96ReadPixel` returns 0 inside the screen's top-left clip even
  when the pixel is present; the array path reads the stored pixel.
- Screen rectangle fills use `p96RectFill` on a bare RastPort over the screen
  bitmap, bypassing the screen's layer clip.
- Bitmap writes/reads use the same calls on the owned RastPort.
- Bitmap rectangle fills are a bounded `p96WritePixel` loop. The spike showed
  `p96RectFill` did not round-trip `0x000000FF` on a user-private
  A8R8G8B8 bitmap; the loop does. The loop rejects rectangles larger than
  1,048,576 pixels.
- `cl_rtg_screen_blit_bitmap` calls
  `BltBitMapRastPort(bitmap->bitmap, sx, sy, &target, dx, dy, width, height,
  0xC0)` on a bare RastPort over the screen bitmap, and validates both handles
  and the source/destination rectangles.
- `CL_P96_BMA_BYTESPERROW` is queried for information only; a `CL_P96_BMF_USERPRIVATE`
  bitmap never moves and needs no lock for the readback path.

## RGB Readback

`cl_rtg_screen_read_pixels` and `cl_rtg_bitmap_read_pixels`:

1. Validate the handle, rectangle, `width/height <= 4096`, the actual target
   dimensions `<= 4096`, and `length >= width * height * 3`.
2. Fill a local `struct CL_P96_RenderInfo` with `Memory = out`, `BytesPerRow = width *
   3`, and `RGBFormat = CL_P96_RGBFB_R8G8B8`. Width 4096 gives a 12,288-byte row,
   inside the signed `WORD` field.
3. Call `p96ReadPixelArray(&ri, 0, 0, rp, x, y, width, height)`.
4. Return `width * height * 3`, or -1 on validation failure.

The output is top-down RGB, exactly the payload of a P6 PPM.

## Ownership and Error Rules

| Situation | Result |
|---|---|
| P96 library or board absent | `rtgBoards` 0; screen/bitmap acquisition null |
| no matching mode | `rtgMode` 0; `openRtgScreen` null |
| null handle | 0/false; no dereference |
| coordinate or rectangle outside the target | false/0; no draw |
| readback buffer too small or row too wide | -1; no read |
| allocation failure | null; partial native resource released |
| close/free twice | the handle is consumed by the generated drop; no second native call |

The shim uses `AllocMem`/`FreeMem` for handles, exactly like the existing
optional modules. It does not allocate unbounded buffers and does not retain
caller pointers after a call returns.
