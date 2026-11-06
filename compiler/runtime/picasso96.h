/*
 * Minimal Picasso96 API definitions for the Clorraine RTG shim.
 *
 * Derived from the P96 developer package (P96Develop 3.6.0, CC-BY 4.0);
 * see THIRD_PARTY_NOTICES.md. Only the constants, structures, and calls used
 * by the shim are present. This is not a replacement for the full P96 SDK.
 */
#ifndef CLORRAINE_PICASSO96_H
#define CLORRAINE_PICASSO96_H

#include <exec/types.h>
#include <exec/libraries.h>
#include <utility/tagitem.h>
#include <intuition/screens.h>
#include <graphics/rastport.h>

typedef enum {
    CL_P96_RGBFB_NONE = 0,
    CL_P96_RGBFB_CLUT,
    CL_P96_RGBFB_R8G8B8,
    CL_P96_RGBFB_B8G8R8,
    CL_P96_RGBFB_R5G6B5PC,
    CL_P96_RGBFB_R5G5B5PC,
    CL_P96_RGBFB_A8R8G8B8,
    CL_P96_RGBFB_A8B8G8R8,
    CL_P96_RGBFB_R8G8B8A8,
    CL_P96_RGBFB_B8G8R8A8,
    CL_P96_RGBFB_R5G6B5,
    CL_P96_RGBFB_R5G5B5,
    CL_P96_RGBFB_B5G6R5PC,
    CL_P96_RGBFB_B5G5R5PC
} CL_P96_RGBFTYPE;

#define CL_P96_RGBFF_CLUT        (1L << CL_P96_RGBFB_CLUT)
#define CL_P96_RGBFF_R8G8B8      (1L << CL_P96_RGBFB_R8G8B8)
#define CL_P96_RGBFF_B8G8R8      (1L << CL_P96_RGBFB_B8G8R8)
#define CL_P96_RGBFF_R5G6B5PC    (1L << CL_P96_RGBFB_R5G6B5PC)
#define CL_P96_RGBFF_R5G5B5PC    (1L << CL_P96_RGBFB_R5G5B5PC)
#define CL_P96_RGBFF_A8R8G8B8    (1L << CL_P96_RGBFB_A8R8G8B8)
#define CL_P96_RGBFF_A8B8G8R8    (1L << CL_P96_RGBFB_A8B8G8R8)
#define CL_P96_RGBFF_R8G8B8A8    (1L << CL_P96_RGBFB_R8G8B8A8)
#define CL_P96_RGBFF_B8G8R8A8    (1L << CL_P96_RGBFB_B8G8R8A8)
#define CL_P96_RGBFF_R5G6B5      (1L << CL_P96_RGBFB_R5G6B5)
#define CL_P96_RGBFF_R5G5B5      (1L << CL_P96_RGBFB_R5G5B5)
#define CL_P96_RGBFF_B5G6R5PC    (1L << CL_P96_RGBFB_B5G6R5PC)
#define CL_P96_RGBFF_B5G5R5PC    (1L << CL_P96_RGBFB_B5G5R5PC)
#define CL_P96_RGBFF_TRUECOLOR   (CL_P96_RGBFF_R8G8B8 | CL_P96_RGBFF_B8G8R8)
#define CL_P96_RGBFF_TRUEALPHA   (CL_P96_RGBFF_A8R8G8B8 | CL_P96_RGBFF_A8B8G8R8 | \
                           CL_P96_RGBFF_R8G8B8A8 | CL_P96_RGBFF_B8G8R8A8)

#define CL_P96_BMF_USERPRIVATE 0x8000

enum {
    CL_P96_BMA_WIDTH = 0,
    CL_P96_BMA_HEIGHT,
    CL_P96_BMA_DEPTH,
    CL_P96_BMA_MEMORY,
    CL_P96_BMA_BYTESPERROW,
    CL_P96_BMA_BYTESPERPIXEL,
    CL_P96_BMA_BITSPERPIXEL,
    CL_P96_BMA_RGBFORMAT,
    CL_P96_BMA_ISP96,
    CL_P96_BMA_ISONBOARD,
    CL_P96_BMA_BOARDMEMBASE,
    CL_P96_BMA_BOARDIOBASE,
    CL_P96_BMA_BOARDMEMIOBASE
};

enum {
    CL_P96_IDA_WIDTH = 0,
    CL_P96_IDA_HEIGHT,
    CL_P96_IDA_DEPTH,
    CL_P96_IDA_BYTESPERPIXEL,
    CL_P96_IDA_BITSPERPIXEL,
    CL_P96_IDA_RGBFORMAT,
    CL_P96_IDA_ISP96,
    CL_P96_IDA_BOARDNUMBER,
    CL_P96_IDA_STDBYTESPERROW,
    CL_P96_IDA_BOARDNAME,
    CL_P96_IDA_COMPATIBLEFORMATS,
    CL_P96_IDA_VIDEOCOMPATIBLE,
    CL_P96_IDA_PABLOIVCOMPATIBLE,
    CL_P96_IDA_PALOMAIVCOMPATIBLE
};

#define CL_P96_BIDTAG_Dummy            (TAG_USER + 96)
#define CL_P96_BIDTAG_FormatsAllowed   (CL_P96_BIDTAG_Dummy + 0x0001)
#define CL_P96_BIDTAG_FormatsForbidden (CL_P96_BIDTAG_Dummy + 0x0002)
#define CL_P96_BIDTAG_NominalWidth     (CL_P96_BIDTAG_Dummy + 0x0003)
#define CL_P96_BIDTAG_NominalHeight    (CL_P96_BIDTAG_Dummy + 0x0004)
#define CL_P96_BIDTAG_Depth            (CL_P96_BIDTAG_Dummy + 0x0005)

#define CL_P96_SA_Dummy      (TAG_USER + 0x20000 + 96)
#define CL_P96_SA_Width      (CL_P96_SA_Dummy + 0x0003)
#define CL_P96_SA_Height     (CL_P96_SA_Dummy + 0x0004)
#define CL_P96_SA_Depth      (CL_P96_SA_Dummy + 0x0005)
#define CL_P96_SA_Title      (CL_P96_SA_Dummy + 0x0008)
#define CL_P96_SA_DisplayID  (CL_P96_SA_Dummy + 0x0012)
#define CL_P96_SA_DClip       (CL_P96_SA_Dummy + 0x0013)
#define CL_P96_SA_ShowTitle  (CL_P96_SA_Dummy + 0x0014)
#define CL_P96_SA_Quiet      (CL_P96_SA_Dummy + 0x0016)

#define CL_P96_RD_Dummy            (TAG_USER + 0x40000 + 96)
#define CL_P96_RD_NumberOfBoards   (CL_P96_RD_Dummy + 0x0001)

struct CL_P96_RenderInfo {
    APTR Memory;
    WORD BytesPerRow;
    WORD pad;
    CL_P96_RGBFTYPE RGBFormat;
};

extern struct Library *P96Base;

#ifdef __VBCC__
struct BitMap *p96AllocBitMap(ULONG, ULONG, ULONG, ULONG, struct BitMap *,
                              CL_P96_RGBFTYPE);
void p96FreeBitMap(struct BitMap *);
ULONG p96GetBitMapAttr(struct BitMap *, ULONG);
ULONG p96BestModeIDTagList(struct TagItem *);
ULONG p96GetModeIDAttr(ULONG, ULONG);
struct Screen *p96OpenScreenTagList(struct TagItem *);
BOOL p96CloseScreen(struct Screen *);
void p96ReadPixelArray(struct CL_P96_RenderInfo *, UWORD, UWORD, struct RastPort *,
                       UWORD, UWORD, UWORD, UWORD);
ULONG p96WritePixel(struct RastPort *, UWORD, UWORD, ULONG);
ULONG p96ReadPixel(struct RastPort *, UWORD, UWORD);
void p96RectFill(struct RastPort *, UWORD, UWORD, UWORD, UWORD, ULONG);
LONG p96GetRTGDataTagList(struct TagItem *);
#else
#include <inline/macros.h>

#define p96AllocBitMap(SizeX, SizeY, Depth, Flags, Friend, RGBFormat) \
    LP6(0x1e, struct BitMap *, p96AllocBitMap, \
        ULONG, SizeX, d0, ULONG, SizeY, d1, ULONG, Depth, d2, \
        ULONG, Flags, d3, struct BitMap *, Friend, a0, CL_P96_RGBFTYPE, RGBFormat, d7, \
        , P96Base)
#define p96FreeBitMap(BitMap_) \
    LP1NR(0x24, p96FreeBitMap, struct BitMap *, BitMap_, a0, , P96Base)
#define p96GetBitMapAttr(BitMap_, Attribute) \
    LP2(0x2a, ULONG, p96GetBitMapAttr, \
        struct BitMap *, BitMap_, a0, ULONG, Attribute, d0, , P96Base)
#define p96BestModeIDTagList(Tags) \
    LP1(0x3c, ULONG, p96BestModeIDTagList, struct TagItem *, Tags, a0, , P96Base)
#define p96GetModeIDAttr(Mode, Attribute) \
    LP2(0x54, ULONG, p96GetModeIDAttr, \
        ULONG, Mode, d0, ULONG, Attribute, d1, , P96Base)
#define p96OpenScreenTagList(Tags) \
    LP1(0x5a, struct Screen *, p96OpenScreenTagList, \
        struct TagItem *, Tags, a0, , P96Base)
#define p96CloseScreen(Screen_) \
    LP1(0x60, BOOL, p96CloseScreen, struct Screen *, Screen_, a0, , P96Base)
#define p96ReadPixelArray(ri, DestX, DestY, rp, SrcX, SrcY, SizeX, SizeY) \
    LP8NR(0x6c, p96ReadPixelArray, \
          struct CL_P96_RenderInfo *, ri, a0, UWORD, DestX, d0, UWORD, DestY, d1, \
          struct RastPort *, rp, a1, UWORD, SrcX, d2, UWORD, SrcY, d3, \
          UWORD, SizeX, d4, UWORD, SizeY, d5, , P96Base)
#define p96WritePixel(rp, x, y, color) \
    LP4(0x72, ULONG, p96WritePixel, \
        struct RastPort *, rp, a1, UWORD, x, d0, UWORD, y, d1, \
        ULONG, color, d2, , P96Base)
#define p96ReadPixel(rp, x, y) \
    LP3(0x78, ULONG, p96ReadPixel, \
        struct RastPort *, rp, a1, UWORD, x, d0, UWORD, y, d1, , P96Base)
#define p96RectFill(rp, MinX, MinY, MaxX, MaxY, color) \
    LP6NR(0x7e, p96RectFill, \
          struct RastPort *, rp, a1, UWORD, MinX, d0, UWORD, MinY, d1, \
          UWORD, MaxX, d2, UWORD, MaxY, d3, ULONG, color, d4, , P96Base)
#define p96GetRTGDataTagList(Tags) \
    LP1(0xb4, LONG, p96GetRTGDataTagList, struct TagItem *, Tags, a0, , P96Base)
#endif

#endif
