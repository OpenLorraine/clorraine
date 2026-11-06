/* Picasso96 RTG screen, bitmap, pixel, blit, and readback shim.
 *
 * The minimal API definitions are embedded immediately before this file by
 * Profile::shim_suffix; the generated C has no external P96 include path.
 * The library base stays open for the process lifetime. Every entry point
 * validates its handles and bounds before touching P96.
 */

#include <exec/types.h>
#include <exec/memory.h>
#include <string.h>
#include <graphics/gfx.h>
#include <graphics/rastport.h>
#include <intuition/screens.h>
#include <proto/exec.h>
#include <proto/graphics.h>

struct Library *P96Base;

typedef struct cl_rtg_screen {
    struct Screen *screen;
    struct Rectangle dclip;
    char *title;
    ULONG mode;
} cl_rtg_screen;

typedef struct cl_rtg_bitmap {
    struct BitMap *bitmap;
    struct RastPort rp;
} cl_rtg_bitmap;

static int cl_rtg_open(void)
{
    if (P96Base == NULL) {
        P96Base = OpenLibrary((STRPTR)"Picasso96API.library", 2);
    }
    if (GfxBase == NULL) {
        GfxBase = (struct GfxBase *)OpenLibrary((STRPTR)"graphics.library", 33);
    }
    return P96Base != NULL && GfxBase != NULL;
}

static int cl_rtg_valid_size(LONG width, LONG height, LONG depth)
{
    return width >= 1 && width <= 4096 && height >= 1 && height <= 4096 &&
           depth == 32;
}

LONG cl_rtg_boards(void)
{
    struct TagItem tags[2];
    ULONG boards = 0;

    if (!cl_rtg_open()) {
        return 0;
    }
    tags[0].ti_Tag = CL_P96_RD_NumberOfBoards;
    tags[0].ti_Data = (ULONG)&boards;
    tags[1].ti_Tag = TAG_DONE;
    tags[1].ti_Data = 0;
    (void)p96GetRTGDataTagList(tags);
    return (LONG)boards;
}

ULONG cl_rtg_mode(LONG width, LONG height, LONG depth)
{
    struct TagItem tags[9];
    ULONG mode;

    if (!cl_rtg_valid_size(width, height, depth) || !cl_rtg_open()) {
        return 0;
    }
    tags[0].ti_Tag = CL_P96_BIDTAG_NominalWidth;
    tags[0].ti_Data = (ULONG)width;
    tags[1].ti_Tag = CL_P96_BIDTAG_NominalHeight;
    tags[1].ti_Data = (ULONG)height;
    tags[2].ti_Tag = CL_P96_BIDTAG_Depth;
    tags[2].ti_Data = (ULONG)depth;
    tags[3].ti_Tag = CL_P96_BIDTAG_FormatsAllowed;
    tags[3].ti_Data = CL_P96_RGBFF_TRUEALPHA;
    tags[4].ti_Tag = TAG_DONE;
    tags[4].ti_Data = 0;
    mode = p96BestModeIDTagList(tags);
    if (mode == 0xFFFFFFFFUL) {
        return 0;
    }
    return mode;
}

LONG cl_rtg_mode_width(ULONG mode)
{
    if (mode == 0 || !cl_rtg_open()) {
        return 0;
    }
    return (LONG)p96GetModeIDAttr(mode, CL_P96_IDA_WIDTH);
}

LONG cl_rtg_mode_height(ULONG mode)
{
    if (mode == 0 || !cl_rtg_open()) {
        return 0;
    }
    return (LONG)p96GetModeIDAttr(mode, CL_P96_IDA_HEIGHT);
}

LONG cl_rtg_mode_depth(ULONG mode)
{
    if (mode == 0 || !cl_rtg_open()) {
        return 0;
    }
    return (LONG)p96GetModeIDAttr(mode, CL_P96_IDA_DEPTH);
}

LONG cl_rtg_mode_format(ULONG mode)
{
    if (mode == 0 || !cl_rtg_open()) {
        return 0;
    }
    return (LONG)p96GetModeIDAttr(mode, CL_P96_IDA_RGBFORMAT);
}

void *cl_rtg_open_screen(LONG width, LONG height, LONG depth, char *title)
{
    struct TagItem tags[6];
    struct Screen *screen;
    cl_rtg_screen *handle;
    ULONG mode;

    if (!cl_rtg_valid_size(width, height, depth)) {
        return NULL;
    }
    mode = cl_rtg_mode(width, height, depth);
    if (mode == 0) {
        return NULL;
    }
    handle = (cl_rtg_screen *)AllocMem(
        (ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        return NULL;
    }
    {
        LONG actual_width = cl_rtg_mode_width(mode);
        LONG actual_height = cl_rtg_mode_height(mode);
        if (actual_width < 1 || actual_width > 4096 ||
            actual_height < 1 || actual_height > 4096) {
            FreeMem(handle, (ULONG)sizeof *handle);
            return NULL;
        }
        handle->dclip.MinX = 0;
        handle->dclip.MinY = 0;
        handle->dclip.MaxX = (WORD)(actual_width - 1);
        handle->dclip.MaxY = (WORD)(actual_height - 1);
    }
    {
        const char *spelling = title == NULL ? "" : title;
        ULONG title_size = (ULONG)strlen(spelling) + 1UL;
        handle->title = (char *)AllocMem(title_size, MEMF_ANY | MEMF_CLEAR);
        if (handle->title == NULL) {
            FreeMem(handle, (ULONG)sizeof *handle);
            return NULL;
        }
        memcpy(handle->title, spelling, (size_t)title_size);
    }
    tags[0].ti_Tag = CL_P96_SA_DisplayID;
    tags[0].ti_Data = mode;
    tags[1].ti_Tag = CL_P96_SA_Title;
    tags[1].ti_Data = (ULONG)handle->title;
    tags[2].ti_Tag = CL_P96_SA_DClip;
    tags[2].ti_Data = (ULONG)&handle->dclip;
    tags[3].ti_Tag = CL_P96_SA_ShowTitle;
    tags[3].ti_Data = FALSE;
    tags[4].ti_Tag = CL_P96_SA_Quiet;
    tags[4].ti_Data = TRUE;
    tags[5].ti_Tag = TAG_DONE;
    tags[5].ti_Data = 0;
    screen = p96OpenScreenTagList(tags);
    if (screen == NULL) {
        FreeMem(handle->title, (ULONG)strlen(handle->title) + 1UL);
        FreeMem(handle, (ULONG)sizeof *handle);
        return NULL;
    }
    handle->screen = screen;
    handle->mode = mode;
    return handle;
}

void cl_rtg_close_screen(void *handle_void)
{
    cl_rtg_screen *handle = (cl_rtg_screen *)handle_void;

    if (handle == NULL) {
        return;
    }
    if (handle->screen != NULL) {
        if (!p96CloseScreen(handle->screen)) {
            /* Closing failed; the screen may still reference the title and
             * DClip, so leak the record rather than free live memory. */
            return;
        }
    }
    if (handle->title != NULL) {
        FreeMem(handle->title, (ULONG)strlen(handle->title) + 1UL);
    }
    FreeMem(handle, (ULONG)sizeof *handle);
}

LONG cl_rtg_screen_width(void *handle_void)
{
    const cl_rtg_screen *handle = (const cl_rtg_screen *)handle_void;

    if (handle == NULL || handle->screen == NULL) {
        return 0;
    }
    return (LONG)p96GetBitMapAttr(handle->screen->RastPort.BitMap,
                                  CL_P96_BMA_WIDTH);
}

LONG cl_rtg_screen_height(void *handle_void)
{
    const cl_rtg_screen *handle = (const cl_rtg_screen *)handle_void;

    if (handle == NULL || handle->screen == NULL) {
        return 0;
    }
    return (LONG)p96GetBitMapAttr(handle->screen->RastPort.BitMap,
                                  CL_P96_BMA_HEIGHT);
}

LONG cl_rtg_screen_depth(void *handle_void)
{
    const cl_rtg_screen *handle = (const cl_rtg_screen *)handle_void;

    if (handle == NULL || handle->screen == NULL) {
        return 0;
    }
    /* P96 reports a 32-bit truecolor bitmap as 24 color bits plus an
     * unused alpha byte; the public depth is bits per pixel. */
    return (LONG)p96GetBitMapAttr(handle->screen->RastPort.BitMap,
                                  CL_P96_BMA_BITSPERPIXEL);
}

LONG cl_rtg_screen_format(void *handle_void)
{
    const cl_rtg_screen *handle = (const cl_rtg_screen *)handle_void;

    if (handle == NULL || handle->screen == NULL) {
        return 0;
    }
    return (LONG)p96GetBitMapAttr(handle->screen->RastPort.BitMap,
                                  CL_P96_BMA_RGBFORMAT);
}

ULONG cl_rtg_screen_mode(void *handle_void)
{
    const cl_rtg_screen *handle = (const cl_rtg_screen *)handle_void;

    return handle == NULL ? 0 : handle->mode;
}

void *cl_rtg_alloc_bitmap(LONG width, LONG height, LONG depth)
{
    struct BitMap *bitmap;
    cl_rtg_bitmap *handle;

    if (!cl_rtg_valid_size(width, height, depth) || !cl_rtg_open()) {
            return NULL;
    }
    bitmap = p96AllocBitMap((ULONG)width, (ULONG)height, 32,
                            CL_P96_BMF_USERPRIVATE, NULL, CL_P96_RGBFB_A8R8G8B8);
    if (bitmap == NULL) {
        return NULL;
    }
    handle = (cl_rtg_bitmap *)AllocMem(
        (ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        p96FreeBitMap(bitmap);
        return NULL;
    }
    handle->bitmap = bitmap;
    InitRastPort(&handle->rp);
    handle->rp.BitMap = bitmap;
    return handle;
}

void cl_rtg_free_bitmap(void *handle_void)
{
    cl_rtg_bitmap *handle = (cl_rtg_bitmap *)handle_void;

    if (handle == NULL) {
        return;
    }
    if (handle->bitmap != NULL) {
        p96FreeBitMap(handle->bitmap);
    }
    FreeMem(handle, (ULONG)sizeof *handle);
}

LONG cl_rtg_bitmap_width(void *handle_void)
{
    cl_rtg_bitmap *handle = (cl_rtg_bitmap *)handle_void;

    if (handle == NULL || handle->bitmap == NULL) {
        return 0;
    }
    return (LONG)p96GetBitMapAttr(handle->bitmap, CL_P96_BMA_WIDTH);
}

LONG cl_rtg_bitmap_height(void *handle_void)
{
    cl_rtg_bitmap *handle = (cl_rtg_bitmap *)handle_void;

    if (handle == NULL || handle->bitmap == NULL) {
        return 0;
    }
    return (LONG)p96GetBitMapAttr(handle->bitmap, CL_P96_BMA_HEIGHT);
}

LONG cl_rtg_bitmap_depth(void *handle_void)
{
    cl_rtg_bitmap *handle = (cl_rtg_bitmap *)handle_void;

    if (handle == NULL || handle->bitmap == NULL) {
        return 0;
    }
    return (LONG)p96GetBitMapAttr(handle->bitmap, CL_P96_BMA_BITSPERPIXEL);
}

LONG cl_rtg_bitmap_format(void *handle_void)
{
    cl_rtg_bitmap *handle = (cl_rtg_bitmap *)handle_void;

    if (handle == NULL || handle->bitmap == NULL) {
        return 0;
    }
    return (LONG)p96GetBitMapAttr(handle->bitmap, CL_P96_BMA_RGBFORMAT);
}

LONG cl_rtg_bitmap_bytes_per_row(void *handle_void)
{
    cl_rtg_bitmap *handle = (cl_rtg_bitmap *)handle_void;

    if (handle == NULL || handle->bitmap == NULL) {
        return 0;
    }
    return (LONG)p96GetBitMapAttr(handle->bitmap, CL_P96_BMA_BYTESPERROW);
}

static int cl_rtg_in_rect(LONG x, LONG y, LONG width, LONG height)
{
    return x >= 0 && y >= 0 && x < width && y < height;
}

/* Read one ARGB pixel through p96ReadPixelArray. p96ReadPixel returns 0 for
 * pixels inside the screen's top-left clip even when the pixel is present;
 * the array path reads the stored pixel and converts RGB back to ARGB. */
static ULONG cl_rtg_read_pixel_rp(struct RastPort *rp, LONG x, LONG y)
{
    UBYTE rgb[3];
    struct CL_P96_RenderInfo info;

    info.Memory = rgb;
    info.BytesPerRow = 3;
    info.pad = 0;
    info.RGBFormat = CL_P96_RGBFB_R8G8B8;
    p96ReadPixelArray(&info, 0, 0, rp, (UWORD)x, (UWORD)y, 1, 1);
    return ((ULONG)rgb[0] << 16) | ((ULONG)rgb[1] << 8) | (ULONG)rgb[2];
}

static int cl_rtg_valid_rect(LONG x0, LONG y0, LONG x1, LONG y1,
                             LONG width, LONG height)
{
    return x0 >= 0 && y0 >= 0 && x0 <= x1 && y0 <= y1 &&
           x1 < width && y1 < height;
}

/* True when an area of `width` x `height` fits at (x,y). The subtraction
 * form avoids signed overflow when a caller passes a large coordinate. */
static int cl_rtg_valid_area(LONG x, LONG y, LONG width, LONG height,
                             LONG target_width, LONG target_height)
{
    return x >= 0 && y >= 0 && width >= 1 && height >= 1 &&
           width <= target_width && height <= target_height &&
           x <= target_width - width && y <= target_height - height;
}

BOOL cl_rtg_screen_write_pixel(void *handle_void, LONG x, LONG y, ULONG color)
{
    const cl_rtg_screen *handle = (const cl_rtg_screen *)handle_void;
    LONG width;
    LONG height;

    if (handle == NULL || handle->screen == NULL) {
        return FALSE;
    }
    width = cl_rtg_screen_width(handle_void);
    height = cl_rtg_screen_height(handle_void);
    if (!cl_rtg_in_rect(x, y, width, height)) {
        return FALSE;
    }
    (void)p96WritePixel(&handle->screen->RastPort, (UWORD)x, (UWORD)y, color);
    return TRUE;
}

ULONG cl_rtg_screen_read_pixel(void *handle_void, LONG x, LONG y)
{
    const cl_rtg_screen *handle = (const cl_rtg_screen *)handle_void;
    LONG width;
    LONG height;

    if (handle == NULL || handle->screen == NULL) {
        return 0;
    }
    width = cl_rtg_screen_width(handle_void);
    height = cl_rtg_screen_height(handle_void);
    if (!cl_rtg_in_rect(x, y, width, height)) {
        return 0;
    }
    return cl_rtg_read_pixel_rp((struct RastPort *)&handle->screen->RastPort,
                                x, y);
}

BOOL cl_rtg_screen_fill_rect(void *handle_void, LONG x0, LONG y0, LONG x1,
                             LONG y1, ULONG color)
{
    const cl_rtg_screen *handle = (const cl_rtg_screen *)handle_void;
    LONG width;
    LONG height;

    if (handle == NULL || handle->screen == NULL) {
        return FALSE;
    }
    width = cl_rtg_screen_width(handle_void);
    height = cl_rtg_screen_height(handle_void);
    if (!cl_rtg_valid_rect(x0, y0, x1, y1, width, height)) {
        return FALSE;
    }
    {
        struct RastPort target;
        InitRastPort(&target);
        target.BitMap = handle->screen->RastPort.BitMap;
        p96RectFill(&target, (UWORD)x0, (UWORD)y0, (UWORD)x1, (UWORD)y1,
                    color);
    }
    return TRUE;
}

BOOL cl_rtg_bitmap_write_pixel(void *handle_void, LONG x, LONG y, ULONG color)
{
    cl_rtg_bitmap *handle = (cl_rtg_bitmap *)handle_void;
    LONG width;
    LONG height;

    if (handle == NULL || handle->bitmap == NULL) {
        return FALSE;
    }
    width = cl_rtg_bitmap_width(handle_void);
    height = cl_rtg_bitmap_height(handle_void);
    if (!cl_rtg_in_rect(x, y, width, height)) {
        return FALSE;
    }
    (void)p96WritePixel(&handle->rp, (UWORD)x, (UWORD)y, color);
    return TRUE;
}

ULONG cl_rtg_bitmap_read_pixel(void *handle_void, LONG x, LONG y)
{
    cl_rtg_bitmap *handle = (cl_rtg_bitmap *)handle_void;
    LONG width;
    LONG height;

    if (handle == NULL || handle->bitmap == NULL) {
        return 0;
    }
    width = cl_rtg_bitmap_width(handle_void);
    height = cl_rtg_bitmap_height(handle_void);
    if (!cl_rtg_in_rect(x, y, width, height)) {
        return 0;
    }
    return cl_rtg_read_pixel_rp((struct RastPort *)&handle->rp, x, y);
}

BOOL cl_rtg_bitmap_fill_rect(void *handle_void, LONG x0, LONG y0, LONG x1,
                             LONG y1, ULONG color)
{
    cl_rtg_bitmap *handle = (cl_rtg_bitmap *)handle_void;
    LONG width;
    LONG height;
    LONG x;
    LONG y;
    ULONG area;

    if (handle == NULL || handle->bitmap == NULL) {
        return FALSE;
    }
    width = cl_rtg_bitmap_width(handle_void);
    height = cl_rtg_bitmap_height(handle_void);
    if (!cl_rtg_valid_rect(x0, y0, x1, y1, width, height)) {
        return FALSE;
    }
    area = (ULONG)(x1 - x0 + 1) * (ULONG)(y1 - y0 + 1);
    if (area > 1048576UL) {
        return FALSE;
    }
    for (y = y0; y <= y1; y++) {
        for (x = x0; x <= x1; x++) {
            (void)p96WritePixel(&handle->rp, (UWORD)x, (UWORD)y, color);
        }
    }
    return TRUE;
}

BOOL cl_rtg_screen_blit_bitmap(void *screen_void, void *bitmap_void,
                               LONG dx, LONG dy, LONG sx, LONG sy,
                               LONG width, LONG height)
{
    cl_rtg_screen *screen = (cl_rtg_screen *)screen_void;
    cl_rtg_bitmap *bitmap = (cl_rtg_bitmap *)bitmap_void;

    if (screen == NULL || screen->screen == NULL || bitmap == NULL ||
        bitmap->bitmap == NULL || width < 1 || height < 1 ||
        width > 4096 || height > 4096) {
        return FALSE;
    }
    if (!cl_rtg_valid_area(sx, sy, width, height,
                           cl_rtg_bitmap_width(bitmap_void),
                           cl_rtg_bitmap_height(bitmap_void)) ||
        !cl_rtg_valid_area(dx, dy, width, height,
                           cl_rtg_screen_width(screen_void),
                           cl_rtg_screen_height(screen_void))) {
        return FALSE;
    }
    {
        struct RastPort target;
        InitRastPort(&target);
        target.BitMap = screen->screen->RastPort.BitMap;
        BltBitMapRastPort(bitmap->bitmap, (WORD)sx, (WORD)sy, &target,
                          (WORD)dx, (WORD)dy, (WORD)width, (WORD)height, 0xC0);
    }
    return TRUE;
}

static LONG cl_rtg_read_pixels(struct RastPort *rp, LONG x, LONG y,
                               LONG width, LONG height, LONG target_width,
                               LONG target_height, APTR out, ULONG length)
{
    struct CL_P96_RenderInfo info;
    ULONG need;

    if (rp == NULL || out == NULL || width > 4096 || height > 4096 ||
        target_width > 4096 || target_height > 4096 ||
        !cl_rtg_valid_area(x, y, width, height, target_width,
                           target_height)) {
        return -1;
    }
    need = (ULONG)width * (ULONG)height * 3UL;
    if (length < need) {
        return -1;
    }
    info.Memory = (APTR)out;
    info.BytesPerRow = (WORD)(width * 3);
    info.pad = 0;
    info.RGBFormat = CL_P96_RGBFB_R8G8B8;
    p96ReadPixelArray(&info, 0, 0, rp, (UWORD)x, (UWORD)y, (UWORD)width,
                      (UWORD)height);
    return (LONG)need;
}

LONG cl_rtg_screen_read_pixels(void *handle_void, LONG x, LONG y, LONG width,
                               LONG height, APTR out, ULONG length)
{
    cl_rtg_screen *handle = (cl_rtg_screen *)handle_void;

    if (handle == NULL || handle->screen == NULL) {
        return -1;
    }
    return cl_rtg_read_pixels(&handle->screen->RastPort, x, y, width, height,
                              cl_rtg_screen_width(handle_void),
                              cl_rtg_screen_height(handle_void), out, length);
}

LONG cl_rtg_bitmap_read_pixels(void *handle_void, LONG x, LONG y, LONG width,
                               LONG height, APTR out, ULONG length)
{
    cl_rtg_bitmap *handle = (cl_rtg_bitmap *)handle_void;

    if (handle == NULL || handle->bitmap == NULL) {
        return -1;
    }
    return cl_rtg_read_pixels((struct RastPort *)&handle->rp, x, y, width,
                              height, cl_rtg_bitmap_width(handle_void),
                              cl_rtg_bitmap_height(handle_void), out, length);
}
