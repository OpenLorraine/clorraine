/* Hosted-Amiga runtime shim for generated Clorraine C.
 *
 * The shim has the same surface as the host shim but carries the AmigaOS ABI
 * types (`BPTR`, `APTR`, `LONG`, `ULONG`, and the rest) that `exec/types.h`
 * and `dos/dos.h` declare. It contains no ownership logic: it only provides
 * the helpers the emitted translation unit calls and the C-side VBlank server
 * the standard-library interrupt dispatcher installs.
 *
 * The IO, memory, and text helpers use the libc the toolchain's libnix runtime
 * supplies (`nix20` for `hosted-amiga`, `nix13` for `hosted-amiga-13`), matching
 * the platform norm. The shim also calls `dos.library` (delay), `exec.library`
 * (ports, messages, interrupts), `graphics.library`, and `intuition.library`.
 * The DOS `Delay` primitive is declared here with the standard-argument
 * attribute the library requires rather than including `proto/dos.h`, whose
 * prototypes for `Open`, `Close`, `Read`, and `Write` would conflict with the
 * ABI prototypes the standard library declares and the shim defines.
 */

#include <exec/types.h>
#include <exec/interrupts.h>
#include <exec/memory.h>
#include <exec/ports.h>
#include <hardware/intbits.h>
#include <proto/exec.h>
/* `CreatePort`/`DeletePort` are amiga.lib helpers, not exec.library vectors;
 * the NDK declares them here rather than in proto/exec.h. */
#include <clib/alib_protos.h>
#include <proto/graphics.h>
#include <proto/intuition.h>
#include <intuition/screens.h>
#include <graphics/rastport.h>
#include <graphics/gfx.h>
#include <graphics/gfxbase.h>
#include <graphics/gfxmacros.h>
#include <graphics/sprite.h>
#include <graphics/view.h>
#include <hardware/custom.h>
/* `dos/dos.h` declares the `BPTR` file-handle type the ABI mapping uses; the
 * generated code emits no AmigaOS typedef of its own, so the shim must supply
 * it. `stdbool.h` supplies the C99 `bool` the Clorraine boolean lowers to. */
#include <dos/dos.h>
#include <limits.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The DOS timer primitive. The library calling convention needs the
 * standard-argument attribute, so the prototype is written directly instead
 * of including `proto/dos.h`, which would also declare the file functions with
 * signatures the generated ABI prototypes cannot match. */
extern void __stdargs Delay(LONG timeout);
extern void __stdargs DateStamp(struct DateStamp *stamp);

/* Suspends the calling task for `ticks` vertical-blank intervals. */
void cl_delay_ticks(ULONG ticks)
{
    Delay((LONG)ticks);
}

/* Returns the DOS tick counter (1/50 s on PAL) as one 32-bit value. */
ULONG cl_dos_ticks(void)
{
    struct DateStamp stamp;
    DateStamp(&stamp);
    return (ULONG)stamp.ds_Days * 24UL * 60UL * 3000UL +
           (ULONG)stamp.ds_Minute * 3000UL +
           (ULONG)stamp.ds_Tick;
}

/* Zero one object through its address. A macro so a call site writes the
 * single argument `cl_zero(&value)`; `sizeof` supplies the byte count. */
#define cl_zero(pointer) memset((void *)(pointer), 0, sizeof(*(pointer)))

/* IEEE-754 NaN test on the raw bit pattern.
 *
 * The platform soft-float comparisons are not IEEE for NaN, so the exponent
 * and mantissa are inspected directly. A `double` parameter accepts a `float`
 * argument through the usual promotion. */
static inline int cl_isnan(double value)
{
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof bits);
    if ((bits & UINT64_C(0x7ff0000000000000)) != UINT64_C(0x7ff0000000000000)) {
        return 0;
    }
    return (bits & UINT64_C(0x000fffffffffffff)) != 0;
}

/* Leave the process after a runtime fault. The Amiga fault contract writes the
 * first line `error CODE: message` to stdout and then exits, so the run-fault
 * leg reads it from the program's standard output. */
static inline void cl_trap(const char *code, const char *message)
{
    printf("error %s: %s\n", code, message);
    fflush(stdout);
    exit(20);
}

/* The byte length of a NUL-terminated C string. The pointer forms of a panic
 * message keep printing their text through the length-based runtime. */
static inline unsigned long cl_cstring_len(const char *text)
{
    return (unsigned long)strlen(text);
}

/* Leave the process after a user-level panic. The message is passed as pointer
 * and length, so a view or handle needs no NUL terminator; the bytes are
 * written exactly as they are. */
static inline void cl_panic(const unsigned char *message, unsigned long len)
{
    fwrite("panic: ", 1, 7, stdout);
    fwrite(message, 1, (size_t)len, stdout);
    fwrite("\n", 1, 1, stdout);
    fflush(stdout);
    exit(20);
}

/* The C-side VBlank dispatcher surface.
 *
 * The standard-library interrupt dispatcher registers a capture-less Clorraine
 * function here. A handler is a plain function pointer, and the server
 * trampoline calls it and returns, so the handler stays allocation-free,
 * drop-free, and panic-free as the interrupt contract requires.
 *
 * The dispatcher has one server slot, so registering a handler replaces the
 * current one. Each registration gets a fresh non-zero token, so a token from a
 * replaced registration can never remove the handler installed after it.
 * `cl_remove_handler` clears the live token, which makes a second removal a
 * no-op. */
typedef void (*cl_handler_fn)(void);

static cl_handler_fn cl_vblank_handler;
static struct Interrupt cl_vblank_server;
static ULONG cl_vblank_token;
static ULONG cl_vblank_next_token = 1;

static void cl_vblank_code(void)
{
    if (cl_vblank_handler != NULL) {
        cl_vblank_handler();
    }
}

/* Installs `handler` as the VBlank server, replacing any current one.
 *
 * A null handler is rejected so an accidental unregister cannot leave a
 * dangling callback. The token is advanced past zero so a wrapped counter
 * never names a registration with the token's "no handler" sentinel. The
 * server primitives return no status on this NDK, so no result is checked. */
ULONG cl_add_handler(cl_handler_fn handler)
{
    if (handler == NULL) {
        return 0;
    }
    if (cl_vblank_handler != NULL) {
        RemIntServer(INTB_VERTB, &cl_vblank_server);
    }
    cl_vblank_handler = handler;
    cl_vblank_token = cl_vblank_next_token;
    cl_vblank_next_token++;
    if (cl_vblank_next_token == 0) {
        cl_vblank_next_token = 1;
    }
    cl_vblank_server.is_Node.ln_Type = NT_INTERRUPT;
    cl_vblank_server.is_Node.ln_Pri = 0;
    cl_vblank_server.is_Node.ln_Name = (char *)"clorraine.vblank";
    cl_vblank_server.is_Data = NULL;
    cl_vblank_server.is_Code = (VOID (*)())cl_vblank_code;
    AddIntServer(INTB_VERTB, &cl_vblank_server);
    return cl_vblank_token;
}

/* Removes the VBlank server when `id` names the live registration. Idempotent:
 * a second removal, a zero token, or a stale token does nothing. */
void cl_remove_handler(ULONG id)
{
    if (id != 0 && id == cl_vblank_token && cl_vblank_handler != NULL) {
        RemIntServer(INTB_VERTB, &cl_vblank_server);
        cl_vblank_handler = NULL;
        cl_vblank_token = 0;
    }
}

/* Heap allocation with the allocator contract: a failure traps. The size is a
 * `size_t`, so a caller that computes a block size in `size_t` never has it
 * truncated back to the 32-bit `ULONG` an AmigaOS word would hold. */
void *cl_alloc(size_t size)
{
    void *block = malloc(size == 0 ? 1 : size);
    if (block == NULL) {
        cl_trap("E1", "out of memory");
    }
    return block;
}

/* Release a block from `cl_alloc`. Null-safe. */
void cl_free(void *block)
{
    free(block);
}

/* Byte copy and compare helpers the library may bind. */
void *cl_memcpy(void *dst, const void *src, ULONG n)
{
    return memcpy(dst, src, (size_t)n);
}

int cl_memcmp(const void *left, const void *right, ULONG n)
{
    return memcmp(left, right, (size_t)n);
}

/* A Buffer is a stable descriptor: callers hold the descriptor pointer while
 * its byte block may be reallocated, so borrowing the descriptor stays sound.
 * The generated `ClBuffer` typedef is an opaque pointer, so these definitions
 * spell the underlying C types directly; the prototypes the emitter writes
 * later are compatible with them. They are not `static`, because a program
 * that does not use `std.mem` still sees the definitions and must not fail
 * `-Wunused-function`. */
typedef struct cl_buffer_block {
    unsigned char *data;
    ULONG len;
    ULONG cap;
} cl_buffer_block;

/* Allocate a buffer with `size` readable bytes. An allocation failure traps,
 * matching the allocator contract. */
void *cl_buffer_new(ULONG size)
{
    cl_buffer_block *buffer = (cl_buffer_block *)cl_alloc(sizeof *buffer);
    buffer->data = (unsigned char *)cl_alloc(size);
    buffer->len = size;
    buffer->cap = size;
    return buffer;
}

/* Release a buffer's data and descriptor. Null-safe. */
void FreeBuffer(void *handle)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    if (buffer == NULL) {
        return;
    }
    cl_free(buffer->data);
    cl_free(buffer);
}

/* The first byte of a buffer's storage. */
void *cl_buffer_ptr(void *handle)
{
    return ((cl_buffer_block *)handle)->data;
}

/* The number of live bytes in a buffer. */
ULONG cl_buffer_len(void *handle)
{
    return ((cl_buffer_block *)handle)->len;
}

/* Grow a buffer's capacity to at least `cap` bytes. */
void cl_buffer_reserve(void *handle, ULONG cap)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    if (cap <= buffer->cap) {
        return;
    }
    ULONG old_cap = buffer->cap;
    unsigned char *grown = (unsigned char *)realloc(buffer->data, (size_t)cap);
    if (grown == NULL) {
        cl_trap("E1", "out of memory");
    }
    /* The grown region is fresh storage; zero it so a container element store
     * reads the canonical empty state instead of allocator garbage. */
    memset(grown + old_cap, 0, (size_t)(cap - old_cap));
    buffer->data = grown;
    buffer->cap = cap;
}

/* Append `n` bytes from `src` to a buffer, growing it as needed. */
void cl_buffer_append(void *handle, APTR src, ULONG n)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    if (n > ULONG_MAX - buffer->len) {
        cl_trap("E1", "buffer too large");
    }
    ULONG needed = buffer->len + n;
    if (needed > buffer->cap) {
        ULONG cap = buffer->cap == 0 ? 8 : buffer->cap;
        while (cap < needed) {
            if (cap > ULONG_MAX / 2) {
                cap = needed;
                break;
            }
            cap *= 2;
        }
        cl_buffer_reserve(handle, cap);
    }
    memcpy(buffer->data + buffer->len, src, (size_t)n);
    buffer->len = needed;
}

/* Write `n` bytes from `src` at `off`, growing the buffer as needed. The
 * offset and count are both bounded so `off + n` cannot wrap and the copy stays
 * inside the buffer. */
void cl_buffer_write(void *handle, ULONG off, APTR src, ULONG n)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    if (n > ULONG_MAX - off) {
        cl_trap("E11211", "index out of range");
    }
    ULONG needed = off + n;
    if (needed > buffer->cap) {
        cl_buffer_reserve(handle, needed);
    }
    memcpy(buffer->data + off, src, (size_t)n);
    if (needed > buffer->len) {
        buffer->len = needed;
    }
}

/* A String is one heap block laid out as `[len: u32][cap: u32][bytes][NUL]`.
 * The handle points at the block start, so the payload bytes begin right after
 * the two counts and a trailing NUL keeps them usable as a C string.
 *
 * In-place growth is deferred, and growing the block would move the handle, so
 * a builder reserves the final size in `cl_string_new` and its appends fit.
 * `cl_string_append` therefore refuses to grow rather than leave the caller
 * with a stale handle. */
typedef struct cl_string_block {
    ULONG len;
    ULONG cap;
} cl_string_block;

/* The first payload byte of a string block. */
static unsigned char *cl_string_bytes(cl_string_block *string)
{
    return (unsigned char *)(string + 1);
}

/* Allocate an empty string with room for `cap` payload bytes.
 *
 * The block size is computed in `size_t` so a capacity near the top of the
 * 32-bit range cannot wrap the allocation size down to a short block; a
 * capacity whose block cannot be represented is rejected with the E1 fault. */
void *cl_string_new(ULONG cap)
{
    if ((size_t)cap > SIZE_MAX - sizeof(cl_string_block) - 1) {
        cl_trap("E1", "string too large");
    }
    cl_string_block *string =
        (cl_string_block *)cl_alloc(sizeof *string + (size_t)cap + 1);
    string->len = 0;
    string->cap = cap;
    cl_string_bytes(string)[0] = '\0';
    return string;
}

/* Append `n` bytes from `src`. The caller must have reserved the capacity. */
void cl_string_append(void *handle, APTR src, ULONG n)
{
    cl_string_block *string = (cl_string_block *)handle;
    if (n > string->cap - string->len) {
        cl_trap("E1", "string capacity exceeded");
    }
    memcpy(cl_string_bytes(string) + string->len, src, (size_t)n);
    string->len += n;
    cl_string_bytes(string)[string->len] = '\0';
}

/* The first payload byte of a string; the trailing NUL makes it a C string. */
void *cl_string_ptr(void *handle)
{
    return cl_string_bytes((cl_string_block *)handle);
}

/* The number of live payload bytes in a string. */
ULONG cl_string_len(void *handle)
{
    return ((cl_string_block *)handle)->len;
}

/* Release a string block. Null-safe. */
void FreeString(void *handle)
{
    if (handle != NULL) {
        cl_free(handle);
    }
}

/* Copy `n` bytes into a fresh string of exactly that size. */
static void *cl_string_from(const void *bytes, ULONG n)
{
    void *string = cl_string_new(n);
    cl_string_append(string, (APTR)bytes, n);
    return string;
}

/* Copy a whole string block into a fresh, independent block. The caller owns
 * the copy and releases it through the string deleter. */
void *cl_string_copy(void *handle)
{
    cl_string_block *string = (cl_string_block *)handle;
    return cl_string_from(cl_string_bytes(string), string->len);
}

/* One piece of a template. A literal piece borrows static bytes and has no
 * `owned` handle; a converted placeholder piece owns a freshly built string
 * that the builder releases once the bytes are appended. */
typedef struct cl_str_part {
    const unsigned char *ptr;
    ULONG len;
    void *owned;
} ClStrPart;

/* Wrap an owned string as a template piece. The builder takes the string over
 * and releases it, so the caller must not use the handle afterwards. */
static inline ClStrPart cl_str_part_owned(void *string)
{
    ClStrPart part;
    part.ptr = (const unsigned char *)cl_string_ptr(string);
    part.len = cl_string_len(string);
    part.owned = string;
    return part;
}

/* Concatenate template pieces into one owned string.
 *
 * The total length is the sum of the piece lengths, so the builder reserves
 * exactly that size once and every append fits. Each owned piece is released
 * after its bytes are copied, which keeps a placeholder's converted string
 * from leaking: the compiler does not drop unnamed temporaries. */
void *cl_string_concat(const ClStrPart *parts, ULONG count)
{
    ULONG index;
    ULONG total = 0;
    for (index = 0; index < count; index++) {
        total += parts[index].len;
    }
    void *string = cl_string_new(total);
    for (index = 0; index < count; index++) {
        cl_string_append(string, (APTR)parts[index].ptr, parts[index].len);
        if (parts[index].owned != NULL) {
            cl_free(parts[index].owned);
        }
    }
    return string;
}

/* The text converters build the same String layout the templates desugar to,
 * so a template and an explicit `text` call format a value identically. */
void *cl_text_i32(LONG value)
{
    char buffer[24];
    int written = snprintf(buffer, sizeof buffer, "%ld", (long)value);
    if (written < 0) {
        cl_trap("E1", "text formatting failed");
    }
    return cl_string_from(buffer, (ULONG)written);
}

void *cl_text_u32(ULONG value)
{
    char buffer[24];
    int written = snprintf(buffer, sizeof buffer, "%lu", (unsigned long)value);
    if (written < 0) {
        cl_trap("E1", "text formatting failed");
    }
    return cl_string_from(buffer, (ULONG)written);
}

/* Six significant digits: `%g` is the C default-precision spelling of that. */
void *cl_text_f64(double value)
{
    char buffer[32];
    int written = snprintf(buffer, sizeof buffer, "%g", value);
    if (written < 0) {
        cl_trap("E1", "text formatting failed");
    }
    return cl_string_from(buffer, (ULONG)written);
}

void *cl_text_bool(int value)
{
    static const char truth[] = "true";
    static const char falsehood[] = "false";
    if (value) {
        return cl_string_from(truth, sizeof truth - 1);
    }
    return cl_string_from(falsehood, sizeof falsehood - 1);
}

/* Build a string from a borrowed byte range; the compiler passes a `str` as
 * its pointer and length so no view struct crosses the shim boundary. */
void *cl_text_str(const void *bytes, ULONG n)
{
    return cl_string_from(bytes, n);
}

/* `0x`-prefixed uppercase hex, with no leading zeros beyond the value. */
void *cl_text_hex_u32(ULONG value)
{
    static const char digits[] = "0123456789ABCDEF";
    char buffer[10];
    ULONG count = 1;
    ULONG remaining = value;
    while (remaining >= 16) {
        remaining >>= 4;
        count++;
    }
    buffer[0] = '0';
    buffer[1] = 'x';
    ULONG index = 0;
    while (index < count) {
        ULONG shift = 4 * (count - 1 - index);
        buffer[2 + index] = digits[(value >> shift) & 0x0f];
        index++;
    }
    return cl_string_from(buffer, 2 + count);
}

/* Pad `src` on the left to `width` bytes with `fill`. */
void *cl_text_pad_left(APTR src, ULONG n, ULONG width, UBYTE fill)
{
    ULONG total = width > n ? width : n;
    void *string = cl_string_new(total);
    ULONG pad = total - n;
    while (pad > 0) {
        cl_string_append(string, &fill, 1);
        pad--;
    }
    cl_string_append(string, src, n);
    return string;
}

/* Pad `src` on the right to `width` bytes with `fill`. */
void *cl_text_pad_right(APTR src, ULONG n, ULONG width, UBYTE fill)
{
    ULONG total = width > n ? width : n;
    void *string = cl_string_new(total);
    ULONG pad = total - n;
    cl_string_append(string, src, n);
    while (pad > 0) {
        cl_string_append(string, &fill, 1);
        pad--;
    }
    return string;
}

/* A file handle is a `BPTR`, an integer word on this ABI, while the libc the
 * shim mirrors names its stream with a pointer. The pointer round-trips through
 * `intptr_t`, which is the same width, so no information is lost. The generated
 * code spells the `File` contract handle as `BPTR` and compares it with zero. */
#define CL_MODE_NEWFILE 1006

/* Open `path` for reading or writing; a failure returns zero. */
BPTR Open(char *path, LONG mode)
{
    const char *spelling = (mode == CL_MODE_NEWFILE) ? "wb" : "rb";
    FILE *file = fopen(path, spelling);
    return (BPTR)(intptr_t)file;
}

/* Close a file handle. Null-safe, because the generated drop is null-safe but
 * a caller may also close an already-empty handle. */
void Close(BPTR file)
{
    if (file != 0) {
        fclose((FILE *)(intptr_t)file);
    }
}

/* Read up to `length` bytes into `buffer` and return the count moved. */
LONG Read(BPTR file, APTR buffer, LONG length)
{
    return (LONG)fread(buffer, 1, (size_t)length, (FILE *)(intptr_t)file);
}

/* Write `length` bytes from `buffer` and return the count moved. */
LONG Write(BPTR file, APTR buffer, LONG length)
{
    return (LONG)fwrite(buffer, 1, (size_t)length, (FILE *)(intptr_t)file);
}

/* Write `n` bytes to standard output without a trailing newline. */
void cl_write(APTR buffer, ULONG n)
{
    if (n > 0) {
        fwrite(buffer, 1, (size_t)n, stdout);
    }
}

/* Write `n` bytes to standard output followed by a newline. */
void cl_write_line(APTR buffer, ULONG n)
{
    if (n > 0) {
        fwrite(buffer, 1, (size_t)n, stdout);
    }
    fputc('\n', stdout);
}

/* --- Amiga graphics: a custom screen and drawing calls. -------------------
 *
 * The `Screen` contract handle is an opaque pointer; `cl_open_screen` returns
 * the `intuition.library` screen or NULL. The drawing helpers take that handle
 * back and drive the `graphics.library` RastPort calls. The two library bases
 * are opened on the first screen open and stay open for the life of the task. */

struct GfxBase *GfxBase = NULL;
struct IntuitionBase *IntuitionBase = NULL;

/* The sprite module clears attached channels when their screen closes. */
static void cl_sprite_screen_closing(struct Screen *screen);

/* Open a custom screen; NULL on failure. */
void *cl_open_screen(LONG width, LONG height, UBYTE depth, char *title)
{
    if (GfxBase == NULL) {
        GfxBase = (struct GfxBase *)OpenLibrary((STRPTR) "graphics.library", 33);
    }
    if (IntuitionBase == NULL) {
        IntuitionBase =
            (struct IntuitionBase *)OpenLibrary((STRPTR) "intuition.library", 33);
    }
    if (GfxBase == NULL || IntuitionBase == NULL) {
        return NULL;
    }

    struct NewScreen screen;
    memset(&screen, 0, sizeof screen);
    screen.LeftEdge = 0;
    screen.TopEdge = 0;
    screen.Width = (WORD)width;
    screen.Height = (WORD)height;
    screen.Depth = depth;
    screen.DetailPen = 0;
    screen.BlockPen = 1;
    screen.ViewModes = 0;
    screen.Type = CUSTOMSCREEN;
    screen.Font = NULL;
    /* Intuition keeps the `DefaultTitle` pointer in the opened screen's
     * `Screen.Title` field; it does not copy the text. The caller must
     * therefore keep the title string alive for as long as the screen is open. */
    screen.DefaultTitle = (STRPTR)title;
    screen.Gadgets = NULL;
    screen.CustomBitMap = NULL;
    return (void *)OpenScreen(&screen);
}

/* Set screen color `pen` from 4-bit R/G/B components. */
void cl_set_color(void *screen_handle, UBYTE pen, UBYTE r, UBYTE g, UBYTE b)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    if (screen != NULL) {
        SetRGB4(&screen->ViewPort, (LONG)pen, r >> 4, g >> 4, b >> 4);
    }
}

/* Fill the rectangle (`x1`,`y1`)-(`x2`,`y2`) with color `pen`. */
void cl_fill(void *screen_handle, LONG x1, LONG y1, LONG x2, LONG y2, UBYTE pen)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    if (screen != NULL) {
        SetAPen(&screen->RastPort, (ULONG)pen);
        RectFill(&screen->RastPort, x1, y1, x2, y2);
    }
}

/* Draw the line (`x1`,`y1`)-(`x2`,`y2`) with color `pen`: Move then Draw. */
void cl_draw(void *screen_handle, LONG x1, LONG y1, LONG x2, LONG y2, UBYTE pen)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    if (screen != NULL) {
        SetAPen(&screen->RastPort, (ULONG)pen);
        Move(&screen->RastPort, x1, y1);
        Draw(&screen->RastPort, x2, y2);
    }
}

/* Fill the whole screen with color `pen`. */
void cl_clear(void *screen_handle, UBYTE pen)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    if (screen != NULL) {
        SetAPen(&screen->RastPort, (ULONG)pen);
        RectFill(&screen->RastPort, 0, 0, (LONG)screen->Width - 1,
                 (LONG)screen->Height - 1);
    }
}

/* Close a screen. Null-safe. Detaches any sprite channels first. */
void cl_close_screen(void *screen_handle)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    if (screen != NULL) {
        cl_sprite_screen_closing(screen);
        CloseScreen(screen);
    }
}

/* One message with a small integer payload the acceptance programs can read.
 * The embedded `struct Message` must stay first so exec's list nodes and the
 * reply path see an ordinary message. */
typedef struct cl_message {
    struct Message msg;
    LONG payload;
} cl_message;

/* Create a private message port, or NULL when the system is out of signals or
 * memory. A private port is enough for the v1 acceptance programs; public
 * named ports stay outside the wrapper surface. */
void *cl_create_msg_port(void)
{
    /* CreatePort is the 1.3-compatible private-port constructor. Passing a
     * null name keeps the port private, matching CreateMsgPort's semantics. */
    return (void *)CreatePort(NULL, 0);
}

/* Delete a private message port. Null-safe; the caller has already drained it. */
void cl_delete_msg_port(void *port_handle)
{
    struct MsgPort *port = (struct MsgPort *)port_handle;
    if (port != NULL) {
        DeletePort(port);
    }
}

/* Allocate a message whose reply port is `reply_handle`, with `payload` carried
 * alongside the exec message header. Returns NULL on allocation failure. */
void *cl_create_message(void *reply_handle, LONG payload)
{
    cl_message *message =
        (cl_message *)AllocMem(sizeof *message, MEMF_PUBLIC | MEMF_CLEAR);
    if (message == NULL) {
        return NULL;
    }
    message->msg.mn_Node.ln_Type = NT_MESSAGE;
    message->msg.mn_Length = sizeof *message;
    message->msg.mn_ReplyPort = (struct MsgPort *)reply_handle;
    message->payload = payload;
    return message;
}

/* Read the payload of a message created by `cl_create_message`. */
LONG cl_message_payload(void *message_handle)
{
    return ((const cl_message *)message_handle)->payload;
}

/* Queue `message_handle` on `port_handle` and wake the port owner. */
void cl_put_msg(void *port_handle, void *message_handle)
{
    PutMsg((struct MsgPort *)port_handle, (struct Message *)message_handle);
}

/* The signal mask that wakes the owner of `port_handle`. */
ULONG cl_signals(void *port_handle)
{
    const struct MsgPort *port = (const struct MsgPort *)port_handle;
    return 1UL << port->mp_SigBit;
}

/* Wait for any signal in `mask`, returning the bits that arrived. */
ULONG cl_wait_signals(ULONG mask)
{
    return Wait(mask);
}

/* Take the next message from `port_handle`, or NULL when the queue is empty. */
void *cl_get_msg(void *port_handle)
{
    return (void *)GetMsg((struct MsgPort *)port_handle);
}

/* Reply a fetched message to its reply port. A message without a reply port is
 * left untouched; the wrapper's contract requires one. */
void cl_reply_msg(void *message_handle)
{
    struct Message *message = (struct Message *)message_handle;
    if (message != NULL && message->mn_ReplyPort != NULL) {
        ReplyMsg(message);
    }
}

/* Release a message that has returned through its reply port. Do not call this
 * on a message still owned by another task. */
void cl_free_message(void *message_handle)
{
    if (message_handle != NULL) {
        FreeMem(message_handle, sizeof(cl_message));
    }
}

/* Release a fetched message that has returned through its reply port. The
 * typed wrapper uses this only for a message this program created. */
void cl_release_message(void *message_handle)
{
    cl_free_message(message_handle);
}

/* ------------------------------------------------------------------------- */
/* Native video: chip memory, bitmaps, blitter, copper, and custom displays.  */
/*                                                                           */
/* The public handles are opaque pointers to these private records. The      */
/* application never sees a `BitMap`, `ViewPort`, `UCopList`, or `ColorMap`.  */
/* A `Display` borrows two caller-owned bitmaps; it never frees them.         */
/* ------------------------------------------------------------------------- */

/* The custom-chip register block. Copper moves take the address of the       */
/* destination register, exactly like the `CMOVE(c, custom.color[n], value)`  */
/* macro does.                                                                */
#define CL_VIDEO_CUSTOM_BASE 0x00DFF000UL
#define CL_VIDEO_COLOR_BASE 0x00000180UL
#define CL_VIDEO_COPPER_MAX 256

/* One chip-memory allocation and the size needed by FreeMem. */
typedef struct cl_video_chip_buffer {
    void *memory;
    ULONG size;
} cl_video_chip_buffer;

/* One displayable bitmap. The `BitMap` is the first member so a handle can   */
/* be used directly where `struct BitMap *` is expected.                     */
typedef struct cl_video_bitmap {
    struct BitMap bitmap;
    LONG width;
    LONG height;
} cl_video_bitmap;

/* One copper builder with one instruction stream per display view. */
typedef struct cl_video_copper {
    struct UCopList *lists[2];
    int count;
    int finished;
} cl_video_copper;

/* One double-buffered custom display. */
typedef struct cl_video_display {
    struct View views[2];
    struct ViewPort view_ports[2];
    struct RasInfo ras_infos[2];
    struct BitMap *bitmaps[2];
    struct ColorMap *color_map;
    struct View *old_view;
    struct UCopList *user_copper[2];
    int width;
    int height;
    int depth;
    int front;
    int built;
    int failed;
} cl_video_display;

/* Open graphics.library once; the screen helpers share the same base. */
static int cl_video_ensure_graphics(void)
{
    if (GfxBase == NULL) {
        GfxBase = (struct GfxBase *)OpenLibrary((STRPTR) "graphics.library", 33);
    }
    return GfxBase != NULL;
}

/* Allocate a chip buffer, or NULL. The record itself lives in fast memory. */
void *cl_video_chip_alloc(ULONG size)
{
    cl_video_chip_buffer *buffer;

    if (size == 0) {
        return NULL;
    }
    buffer = (cl_video_chip_buffer *)AllocMem(
        (ULONG)sizeof *buffer, MEMF_ANY | MEMF_CLEAR);
    if (buffer == NULL) {
        return NULL;
    }
    buffer->memory = AllocMem(size, MEMF_CHIP | MEMF_CLEAR);
    if (buffer->memory == NULL) {
        FreeMem(buffer, (ULONG)sizeof *buffer);
        return NULL;
    }
    buffer->size = size;
    return buffer;
}

/* Release a chip buffer. Null-safe. */
void cl_video_chip_free(void *handle)
{
    cl_video_chip_buffer *buffer = (cl_video_chip_buffer *)handle;

    if (buffer == NULL) {
        return;
    }
    if (buffer->memory != NULL) {
        FreeMem(buffer->memory, buffer->size);
    }
    FreeMem(buffer, (ULONG)sizeof *buffer);
}

/* Return the payload size, or zero for a null handle. */
ULONG cl_video_chip_size(void *handle)
{
    const cl_video_chip_buffer *buffer =
        (const cl_video_chip_buffer *)handle;

    return buffer == NULL ? 0 : buffer->size;
}

/* Bounds-checked byte reads and writes. */
UBYTE cl_video_chip_read8(void *handle, ULONG offset)
{
    const cl_video_chip_buffer *buffer =
        (const cl_video_chip_buffer *)handle;

    if (buffer == NULL || offset >= buffer->size) {
        return 0;
    }
    return ((const UBYTE *)buffer->memory)[offset];
}

void cl_video_chip_write8(void *handle, ULONG offset, UBYTE value)
{
    cl_video_chip_buffer *buffer = (cl_video_chip_buffer *)handle;

    if (buffer == NULL || offset >= buffer->size) {
        return;
    }
    ((UBYTE *)buffer->memory)[offset] = value;
}

/* The 16- and 32-bit helpers require alignment so the 68000 never sees an
 * address trap. `memcpy` keeps the value independent of the compiler's
 * alignment assumptions. */
UWORD cl_video_chip_read16(void *handle, ULONG offset)
{
    const cl_video_chip_buffer *buffer =
        (const cl_video_chip_buffer *)handle;
    UWORD value = 0;

    if (buffer == NULL || (offset & 1UL) != 0 ||
        offset > buffer->size || buffer->size - offset < 2UL) {
        return 0;
    }
    memcpy(&value, (const UBYTE *)buffer->memory + offset, sizeof value);
    return value;
}

void cl_video_chip_write16(void *handle, ULONG offset, UWORD value)
{
    cl_video_chip_buffer *buffer = (cl_video_chip_buffer *)handle;

    if (buffer == NULL || (offset & 1UL) != 0 ||
        offset > buffer->size || buffer->size - offset < 2UL) {
        return;
    }
    memcpy((UBYTE *)buffer->memory + offset, &value, sizeof value);
}

ULONG cl_video_chip_read32(void *handle, ULONG offset)
{
    const cl_video_chip_buffer *buffer =
        (const cl_video_chip_buffer *)handle;
    ULONG value = 0;

    if (buffer == NULL || (offset & 3UL) != 0 ||
        offset > buffer->size || buffer->size - offset < 4UL) {
        return 0;
    }
    memcpy(&value, (const UBYTE *)buffer->memory + offset, sizeof value);
    return value;
}

void cl_video_chip_write32(void *handle, ULONG offset, ULONG value)
{
    cl_video_chip_buffer *buffer = (cl_video_chip_buffer *)handle;

    if (buffer == NULL || (offset & 3UL) != 0 ||
        offset > buffer->size || buffer->size - offset < 4UL) {
        return;
    }
    memcpy((UBYTE *)buffer->memory + offset, &value, sizeof value);
}

/* Create a bitmap with one AllocRaster plane per bit. */
void *cl_video_bitmap_new(LONG width, LONG height, UBYTE depth)
{
    cl_video_bitmap *bitmap;
    int plane;

    if (width < 1 || width > 1024 || height < 1 || height > 1024 ||
        depth < 1 || depth > 8) {
        return NULL;
    }
    if (!cl_video_ensure_graphics()) {
        return NULL;
    }
    bitmap = (cl_video_bitmap *)AllocMem(
        (ULONG)sizeof *bitmap, MEMF_ANY | MEMF_CLEAR);
    if (bitmap == NULL) {
        return NULL;
    }
    InitBitMap(&bitmap->bitmap, (LONG)depth, width, height);
    bitmap->width = width;
    bitmap->height = height;
    for (plane = 0; plane < (int)depth; plane++) {
        bitmap->bitmap.Planes[plane] =
            AllocRaster((ULONG)width, (ULONG)height);
        if (bitmap->bitmap.Planes[plane] == NULL) {
            int free_plane;
            for (free_plane = 0; free_plane < plane; free_plane++) {
                FreeRaster(bitmap->bitmap.Planes[free_plane],
                           (ULONG)width, (ULONG)height);
            }
            FreeMem(bitmap, (ULONG)sizeof *bitmap);
            return NULL;
        }
        memset(bitmap->bitmap.Planes[plane], 0,
               (size_t)bitmap->bitmap.BytesPerRow * (size_t)bitmap->bitmap.Rows);
    }
    return bitmap;
}

/* Release every plane and the bitmap record. Null-safe. */
void cl_video_bitmap_free(void *handle)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;
    int plane;

    if (bitmap == NULL) {
        return;
    }
    for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
        if (bitmap->bitmap.Planes[plane] != NULL) {
            FreeRaster(bitmap->bitmap.Planes[plane],
                       (ULONG)bitmap->width, (ULONG)bitmap->height);
        }
    }
    FreeMem(bitmap, (ULONG)sizeof *bitmap);
}

/* Geometry accessors. Null handles return zero. */
LONG cl_video_bitmap_width(void *handle)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;

    return bitmap == NULL ? 0 : bitmap->width;
}

LONG cl_video_bitmap_height(void *handle)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;

    return bitmap == NULL ? 0 : bitmap->height;
}

UBYTE cl_video_bitmap_depth(void *handle)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;

    return bitmap == NULL ? 0 : bitmap->bitmap.Depth;
}

ULONG cl_video_bitmap_bytes_per_row(void *handle)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;

    return bitmap == NULL ? 0 : (ULONG)bitmap->bitmap.BytesPerRow;
}

/* True when (`x`,`y`) is inside the bitmap and `plane` exists. */
static int cl_video_bitmap_has_pixel(const cl_video_bitmap *bitmap,
                                     int plane, int x, int y)
{
    if (bitmap == NULL || x < 0 || y < 0) {
        return 0;
    }
    if (plane < 0 || plane >= (int)bitmap->bitmap.Depth) {
        return 0;
    }
    if (x >= (int)bitmap->width) {
        return 0;
    }
    return y < (int)bitmap->height;
}

/* Plot one pixel into one plane. */
static void cl_video_plot_plane(cl_video_bitmap *bitmap, int plane,
                                int x, int y, int on)
{
    UBYTE *row;
    UBYTE mask;

    if (!cl_video_bitmap_has_pixel(bitmap, plane, x, y)) {
        return;
    }
    row = bitmap->bitmap.Planes[plane] +
          (size_t)y * bitmap->bitmap.BytesPerRow;
    mask = (UBYTE)(0x80U >> (unsigned)(x & 7));
    if (on) {
        row[x >> 3] |= mask;
    } else {
        row[x >> 3] &= (UBYTE)~mask;
    }
}

/* Fill a horizontal span of one plane, inclusive. */
static void cl_video_fill_plane(cl_video_bitmap *bitmap, int plane,
                                int x1, int x2, int y, int on)
{
    UBYTE *row;
    int first;
    int last;
    int bit_first;
    int bit_last;
    UBYTE first_mask;
    UBYTE last_mask;
    int index;

    if (!cl_video_bitmap_has_pixel(bitmap, plane, x1, y) ||
        !cl_video_bitmap_has_pixel(bitmap, plane, x2, y)) {
        return;
    }
    row = bitmap->bitmap.Planes[plane] +
          (size_t)y * bitmap->bitmap.BytesPerRow;
    first = x1 >> 3;
    last = x2 >> 3;
    bit_first = x1 & 7;
    bit_last = x2 & 7;
    first_mask = (UBYTE)(0xFFU >> (unsigned)bit_first);
    last_mask = (UBYTE)(0xFFU << (unsigned)(7 - bit_last));
    if (first == last) {
        UBYTE mask = (UBYTE)(first_mask & last_mask);
        if (on) {
            row[first] |= mask;
        } else {
            row[first] &= (UBYTE)~mask;
        }
        return;
    }
    if (on) {
        row[first] |= first_mask;
    } else {
        row[first] &= (UBYTE)~first_mask;
    }
    for (index = first + 1; index < last; index++) {
        row[index] = on ? 0xFFU : 0x00U;
    }
    if (on) {
        row[last] |= last_mask;
    } else {
        row[last] &= (UBYTE)~last_mask;
    }
}

/* Clear the bitmap to `pen`. */
void cl_video_bitmap_clear(void *handle, UBYTE pen)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;
    int plane;

    if (bitmap == NULL) {
        return;
    }
    for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
        memset(bitmap->bitmap.Planes[plane],
               (pen & (UBYTE)(1U << (unsigned)plane)) != 0 ? 0xFF : 0x00,
               (size_t)bitmap->bitmap.BytesPerRow * (size_t)bitmap->bitmap.Rows);
    }
}

/* Plot one pixel with the full pen value. */
void cl_video_bitmap_plot(void *handle, LONG x, LONG y, UBYTE pen)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;
    int plane;

    if (bitmap == NULL) {
        return;
    }
    for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
        cl_video_plot_plane(bitmap, plane, (int)x, (int)y,
                            (pen & (UBYTE)(1U << (unsigned)plane)) != 0);
    }
}

/* Clip a line endpoint pair to the bitmap before the Bresenham loop. */
static int cl_video_clip_line(cl_video_bitmap *bitmap, int *x1, int *y1,
                              int *x2, int *y2)
{
    int width;
    int height;

    if (bitmap == NULL) {
        return 0;
    }
    width = (int)bitmap->width;
    height = (int)bitmap->height;
    if (width <= 0 || height <= 0) {
        return 0;
    }
    if (*x1 < -32768) {
        *x1 = -32768;
    }
    if (*x1 > 32767) {
        *x1 = 32767;
    }
    if (*x2 < -32768) {
        *x2 = -32768;
    }
    if (*x2 > 32767) {
        *x2 = 32767;
    }
    if (*y1 < -32768) {
        *y1 = -32768;
    }
    if (*y1 > 32767) {
        *y1 = 32767;
    }
    if (*y2 < -32768) {
        *y2 = -32768;
    }
    if (*y2 > 32767) {
        *y2 = 32767;
    }
    return 1;
}

/* Draw a line with integer Bresenham stepping. */
void cl_video_bitmap_line(void *handle, LONG x1, LONG y1, LONG x2, LONG y2,
                          UBYTE pen)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;
    int ix1 = (int)x1;
    int iy1 = (int)y1;
    int ix2 = (int)x2;
    int iy2 = (int)y2;
    int dx;
    int dy;
    int sx;
    int sy;
    int error;

    if (!cl_video_clip_line(bitmap, &ix1, &iy1, &ix2, &iy2)) {
        return;
    }
    dx = ix2 > ix1 ? ix2 - ix1 : ix1 - ix2;
    dy = iy2 > iy1 ? iy2 - iy1 : iy1 - iy2;
    sx = ix1 < ix2 ? 1 : -1;
    sy = iy1 < iy2 ? 1 : -1;
    error = dx - dy;
    for (;;) {
        cl_video_bitmap_plot(bitmap, ix1, iy1, pen);
        if (ix1 == ix2 && iy1 == iy2) {
            break;
        }
        if (error * 2 > -dy) {
            error -= dy;
            ix1 += sx;
        }
        if (error * 2 < dx) {
            error += dx;
            iy1 += sy;
        }
    }
}

/* Fill an inclusive rectangle, clipping to the bitmap. */
void cl_video_bitmap_fill(void *handle, LONG x1, LONG y1, LONG x2, LONG y2,
                          UBYTE pen)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;
    int ix1 = (int)x1;
    int iy1 = (int)y1;
    int ix2 = (int)x2;
    int iy2 = (int)y2;
    int swap;
    int width;
    int height;
    int y;
    int plane;

    if (bitmap == NULL) {
        return;
    }
    if (ix1 > ix2) {
        swap = ix1;
        ix1 = ix2;
        ix2 = swap;
    }
    if (iy1 > iy2) {
        swap = iy1;
        iy1 = iy2;
        iy2 = swap;
    }
    width = (int)bitmap->width;
    height = (int)bitmap->height;
    if (ix2 < 0 || iy2 < 0 || ix1 >= width || iy1 >= height) {
        return;
    }
    if (ix1 < 0) {
        ix1 = 0;
    }
    if (iy1 < 0) {
        iy1 = 0;
    }
    if (ix2 >= width) {
        ix2 = width - 1;
    }
    if (iy2 >= height) {
        iy2 = height - 1;
    }
    for (y = iy1; y <= iy2; y++) {
        for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
            cl_video_fill_plane(bitmap, plane, ix1, ix2, y,
                                (pen & (UBYTE)(1U << (unsigned)plane)) != 0);
        }
    }
}

/* Read one plane bit. */
BOOL cl_video_bitmap_plane_bit(void *handle, UBYTE plane, LONG x, LONG y)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;
    const UBYTE *row;
    UBYTE mask;

    if (!cl_video_bitmap_has_pixel(bitmap, (int)plane, (int)x, (int)y)) {
        return FALSE;
    }
    row = bitmap->bitmap.Planes[plane] +
          (size_t)y * bitmap->bitmap.BytesPerRow;
    mask = (UBYTE)(0x80U >> (unsigned)(x & 7));
    return (row[x >> 3] & mask) != 0 ? TRUE : FALSE;
}

/* Write one plane bit. */
void cl_video_bitmap_set_plane_bit(void *handle, UBYTE plane, LONG x, LONG y,
                                   BOOL on)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;

    cl_video_plot_plane(bitmap, (int)plane, (int)x, (int)y, on ? 1 : 0);
}

/* A rectangle is valid when it lies completely inside the bitmap. */
static int cl_video_rect_inside(const cl_video_bitmap *bitmap,
                                int x, int y, int width, int height)
{
    int limit_x;
    int limit_y;

    if (bitmap == NULL || width <= 0 || height <= 0 || x < 0 || y < 0) {
        return 0;
    }
    limit_x = (int)bitmap->width;
    limit_y = (int)bitmap->height;
    if (x > limit_x || y > limit_y) {
        return 0;
    }
    return width <= limit_x - x && height <= limit_y - y;
}

/* Plain blitter copy (minterm C0). */
void cl_video_bitmap_blit(void *dest_handle, void *src_handle,
                          LONG sx, LONG sy, LONG dx, LONG dy,
                          LONG width, LONG height)
{
    cl_video_bitmap *dest = (cl_video_bitmap *)dest_handle;
    const cl_video_bitmap *src = (const cl_video_bitmap *)src_handle;

    if (width > 992 || height > 1024) {
        return;
    }
    if (!cl_video_rect_inside(src, (int)sx, (int)sy, (int)width, (int)height) ||
        !cl_video_rect_inside(dest, (int)dx, (int)dy, (int)width, (int)height)) {
        return;
    }
    (void)BltBitMap(&src->bitmap, sx, sy, &dest->bitmap, dx, dy,
                    width, height, 0xC0, 0xFF, NULL);
    WaitBlit();
}

/* Masked blitter copy (minterm E0, single-plane mask). */
void cl_video_bitmap_blit_masked(void *dest_handle, void *src_handle,
                                 void *mask_handle,
                                 LONG sx, LONG sy, LONG dx, LONG dy,
                                 LONG width, LONG height)
{
    cl_video_bitmap *dest = (cl_video_bitmap *)dest_handle;
    const cl_video_bitmap *src = (const cl_video_bitmap *)src_handle;
    const cl_video_bitmap *mask = (const cl_video_bitmap *)mask_handle;
    struct RastPort rast_port;

    if (dest == NULL || src == NULL || mask == NULL ||
        mask->bitmap.Depth != 1 ||
        mask->width != src->width || mask->height != src->height ||
        mask->bitmap.BytesPerRow != src->bitmap.BytesPerRow) {
        return;
    }
    if (width > 992 || height > 1024) {
        return;
    }
    if (!cl_video_rect_inside(src, (int)sx, (int)sy, (int)width, (int)height) ||
        !cl_video_rect_inside(dest, (int)dx, (int)dy, (int)width, (int)height)) {
        return;
    }
    InitRastPort(&rast_port);
    rast_port.BitMap = &dest->bitmap;
    rast_port.Mask = 0xFF;
    BltMaskBitMapRastPort(&src->bitmap, sx, sy, &rast_port, dx, dy,
                          width, height, 0xE0, mask->bitmap.Planes[0]);
    WaitBlit();
}

/* FNV-1a 32-bit over the geometry and every plane byte. */
ULONG cl_video_bitmap_checksum(void *handle)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;
    uint32_t hash = UINT32_C(2166136261);
    int plane;

    if (bitmap == NULL) {
        return 0;
    }
    hash = (hash ^ (uint32_t)bitmap->bitmap.Depth) * UINT32_C(16777619);
    hash = (hash ^ (uint32_t)bitmap->width) * UINT32_C(16777619);
    hash = (hash ^ (uint32_t)bitmap->height) * UINT32_C(16777619);
    hash = (hash ^ (uint32_t)bitmap->bitmap.BytesPerRow) * UINT32_C(16777619);
    hash = (hash ^ (uint32_t)bitmap->bitmap.Rows) * UINT32_C(16777619);
    for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
        const uint8_t *bytes = (const uint8_t *)bitmap->bitmap.Planes[plane];
        size_t length =
            (size_t)bitmap->bitmap.BytesPerRow * (size_t)bitmap->bitmap.Rows;
        size_t index;
        for (index = 0; index < length; index++) {
            hash = (hash ^ (uint32_t)bytes[index]) * UINT32_C(16777619);
        }
    }
    return (ULONG)hash;
}

/* Free one user copper list through a temporary viewport. */
static void cl_video_free_user_copper(struct UCopList *list)
{
    struct ViewPort view_port;

    if (list == NULL) {
        return;
    }
    InitVPort(&view_port);
    view_port.UCopIns = list;
    FreeVPortCopLists(&view_port);
}

/* Free the two instruction streams of a copper that never reached a display. */
static void cl_video_copper_destroy(cl_video_copper *copper)
{
    int index;

    if (copper == NULL) {
        return;
    }
    for (index = 0; index < 2; index++) {
        cl_video_free_user_copper(copper->lists[index]);
        copper->lists[index] = NULL;
    }
    FreeMem(copper, (ULONG)sizeof *copper);
}

/* Create a copper builder, or NULL. */
void *cl_video_copper_new(void)
{
    cl_video_copper *copper;
    int index;

    if (!cl_video_ensure_graphics()) {
        return NULL;
    }
    copper = (cl_video_copper *)AllocMem(
        (ULONG)sizeof *copper, MEMF_ANY | MEMF_CLEAR);
    if (copper == NULL) {
        return NULL;
    }
    for (index = 0; index < 2; index++) {
        copper->lists[index] = (struct UCopList *)AllocMem(
            (ULONG)sizeof(struct UCopList), MEMF_PUBLIC | MEMF_CLEAR);
        if (copper->lists[index] == NULL ||
            UCopperListInit(copper->lists[index], CL_VIDEO_COPPER_MAX) == NULL) {
            cl_video_copper_destroy(copper);
            return NULL;
        }
    }
    return copper;
}

/* Release a copper that was not moved into a display. */
void cl_video_copper_free(void *handle)
{
    cl_video_copper_destroy((cl_video_copper *)handle);
}

/* Append the same wait to both streams. */
void cl_video_copper_wait(void *handle, LONG line)
{
    cl_video_copper *copper = (cl_video_copper *)handle;
    int index;

    if (copper == NULL || copper->finished ||
        copper->count >= CL_VIDEO_COPPER_MAX) {
        return;
    }
    for (index = 0; index < 2; index++) {
        (void)CWait(copper->lists[index], line, 0);
        CBump(copper->lists[index]);
    }
    copper->count++;
}

/* Append the same register move to both streams. */
void cl_video_copper_move(void *handle, UWORD reg, UWORD value)
{
    cl_video_copper *copper = (cl_video_copper *)handle;
    APTR destination;
    int index;

    if (copper == NULL || copper->finished ||
        copper->count >= CL_VIDEO_COPPER_MAX) {
        return;
    }
    destination = (APTR)(CL_VIDEO_CUSTOM_BASE + (ULONG)reg);
    for (index = 0; index < 2; index++) {
        (void)CMove(copper->lists[index], destination, (LONG)value);
        CBump(copper->lists[index]);
    }
    copper->count++;
}

/* Append a 12-bit color write to both streams. */
void cl_video_copper_color(void *handle, UBYTE index, UBYTE r, UBYTE g,
                           UBYTE b)
{
    UWORD value;

    if (index > 31) {
        return;
    }
    value = (UWORD)((((UWORD)r >> 4) << 8) |
                    (((UWORD)g >> 4) << 4) |
                    ((UWORD)b >> 4));
    cl_video_copper_move(handle,
                         (UWORD)(CL_VIDEO_COLOR_BASE + (ULONG)index * 2UL),
                         value);
}

/* Append the documented wait-forever instruction. */
void cl_video_copper_end(void *handle)
{
    cl_video_copper *copper = (cl_video_copper *)handle;
    int index;

    if (copper == NULL || copper->finished) {
        return;
    }
    for (index = 0; index < 2; index++) {
        (void)CWait(copper->lists[index], 10000, 255);
        CBump(copper->lists[index]);
    }
    copper->finished = 1;
}

/* Set one palette entry from 8-bit components. */
void cl_video_display_set_color(void *handle, UBYTE index, UBYTE r, UBYTE g,
                                UBYTE b)
{
    cl_video_display *display = (cl_video_display *)handle;

    if (display == NULL || (int)index >= (1 << display->depth)) {
        return;
    }
    SetRGB4CM(display->color_map, (LONG)index,
              (ULONG)(r >> 4), (ULONG)(g >> 4), (ULONG)(b >> 4));
}

/* Default palette: black, white, RGB, CMY, then a gray ramp. */
static void cl_video_display_default_palette(cl_video_display *display)
{
    int count = 1 << display->depth;
    int index;

    for (index = 0; index < count; index++) {
        ULONG r = 0;
        ULONG g = 0;
        ULONG b = 0;
        switch (index) {
        case 0:
            break;
        case 1:
            r = 15;
            g = 15;
            b = 15;
            break;
        case 2:
            r = 15;
            break;
        case 3:
            g = 15;
            break;
        case 4:
            b = 15;
            break;
        case 5:
            r = 15;
            g = 15;
            break;
        case 6:
            g = 15;
            b = 15;
            break;
        case 7:
            r = 15;
            b = 15;
            break;
        default:
            r = (ULONG)((index - 8) * 2);
            if (r > 15) {
                r = 15;
            }
            g = r;
            b = r;
            break;
        }
        SetRGB4CM(display->color_map, (LONG)index, r, g, b);
    }
}

/* Open a display over two caller-owned bitmaps, or NULL. */
void *cl_video_display_open(void *front_handle, void *back_handle)
{
    cl_video_bitmap *front = (cl_video_bitmap *)front_handle;
    cl_video_bitmap *back = (cl_video_bitmap *)back_handle;
    cl_video_display *display;
    int index;

    if (front == NULL || back == NULL) {
        return NULL;
    }
    if (front->bitmap.Depth != back->bitmap.Depth ||
        front->width != back->width || front->height != back->height ||
        front->bitmap.BytesPerRow != back->bitmap.BytesPerRow ||
        front->width < 1 || front->width > 320 ||
        front->height < 1 || front->height > 256 ||
        front->bitmap.Depth < 1 || front->bitmap.Depth > 5) {
        return NULL;
    }
    if (!cl_video_ensure_graphics()) {
        return NULL;
    }
    display = (cl_video_display *)AllocMem(
        (ULONG)sizeof *display, MEMF_ANY | MEMF_CLEAR);
    if (display == NULL) {
        return NULL;
    }
    display->bitmaps[0] = &front->bitmap;
    display->bitmaps[1] = &back->bitmap;
    display->width = (int)front->width;
    display->height = (int)front->height;
    display->depth = (int)front->bitmap.Depth;
    display->color_map = GetColorMap(1L << display->depth);
    if (display->color_map == NULL) {
        FreeMem(display, (ULONG)sizeof *display);
        return NULL;
    }
    cl_video_display_default_palette(display);
    /* Initialize both viewports once. Sprite and copper attachments land in
     * their lists after this point; the first flip only compiles them. */
    for (index = 0; index < 2; index++) {
        InitView(&display->views[index]);
        InitVPort(&display->view_ports[index]);
        display->views[index].ViewPort = &display->view_ports[index];
        display->view_ports[index].DWidth = (WORD)display->width;
        display->view_ports[index].DHeight = (WORD)display->height;
        display->view_ports[index].DxOffset = 0x81;
        display->view_ports[index].DyOffset = 0x2c;
        display->view_ports[index].Modes = 0;
        display->views[index].Modes = 0;
        display->view_ports[index].RasInfo = &display->ras_infos[index];
        display->view_ports[index].ColorMap = display->color_map;
        display->ras_infos[index].BitMap = display->bitmaps[index];
        display->ras_infos[index].RxOffset = 0;
        display->ras_infos[index].RyOffset = 0;
        display->ras_infos[index].Next = NULL;
    }
    return display;
}

/* Queue a copper for both viewports before the first flip. */
BOOL cl_video_display_set_copper(void *handle, void *copper_handle)
{
    cl_video_display *display = (cl_video_display *)handle;
    cl_video_copper *copper = (cl_video_copper *)copper_handle;
    int index;

    if (display == NULL || copper == NULL) {
        cl_video_copper_destroy(copper);
        return FALSE;
    }
    if (display->built) {
        cl_video_copper_destroy(copper);
        return FALSE;
    }
    for (index = 0; index < 2; index++) {
        cl_video_free_user_copper(display->user_copper[index]);
        display->user_copper[index] = copper->lists[index];
        copper->lists[index] = NULL;
    }
    FreeMem(copper, (ULONG)sizeof *copper);
    return TRUE;
}

/* Build the views, or switch to the other one. */
void cl_video_display_flip(void *handle)
{
    cl_video_display *display = (cl_video_display *)handle;
    int index;

    if (display == NULL || display->failed) {
        return;
    }
    if (!display->built) {
        display->old_view = GfxBase->ActiView;
        for (index = 0; index < 2; index++) {
            display->view_ports[index].UCopIns = display->user_copper[index];
            display->user_copper[index] = NULL;
            display->view_ports[index].Modes = 0;
            display->views[index].Modes = 0;
            (void)MakeVPort(&display->views[index],
                            &display->view_ports[index]);
            (void)MrgCop(&display->views[index]);
            if (display->views[index].LOFCprList == NULL) {
                display->failed = 1;
                display->built = 1;
                return;
            }
        }
        LoadView(&display->views[0]);
        WaitTOF();
        display->front = 0;
        display->built = 1;
        return;
    }
    LoadView(&display->views[1 - display->front]);
    WaitTOF();
    display->front = 1 - display->front;
}

/* True when the display could not build its views. */
BOOL cl_video_display_failed(void *handle)
{
    const cl_video_display *display = (const cl_video_display *)handle;

    return display == NULL || display->failed ? TRUE : FALSE;
}

/* Return 0 for the front bitmap, 1 for the back bitmap. */
UBYTE cl_video_display_index(void *handle)
{
    const cl_video_display *display = (const cl_video_display *)handle;

    return display == NULL ? 0 : (UBYTE)display->front;
}

/* Restore the system view and release the display. */
void cl_video_display_free(void *handle)
{
    cl_video_display *display = (cl_video_display *)handle;
    int index;

    if (display == NULL) {
        return;
    }
    if (display->built) {
        LoadView(display->old_view);
        WaitTOF();
        WaitTOF();
        for (index = 0; index < 2; index++) {
            if (display->views[index].LOFCprList != NULL) {
                FreeCprList(display->views[index].LOFCprList);
            }
        }
    }
    for (index = 0; index < 2; index++) {
        cl_video_free_user_copper(display->user_copper[index]);
        display->user_copper[index] = NULL;
        FreeVPortCopLists(&display->view_ports[index]);
    }
    if (display->color_map != NULL) {
        FreeColorMap(display->color_map);
    }
    FreeMem(display, (ULONG)sizeof *display);
}

/* Wait for the next vertical blank. */
void cl_video_wait_vblank(void)
{
    if (cl_video_ensure_graphics()) {
        WaitTOF();
    }
}

/* ------------------------------------------------------------------------- */
/* OCS hardware sprites: eight DMA channels, pairs, collision, animation.     */
/*                                                                           */
/* The graphics.library simple-sprite machine owns the sprite copper          */
/* instructions. Sprites attach to an Intuition screen's viewport; the image  */
/* is the documented posctl[2] + data[height][2] + reserved[2] block in chip  */
/* memory. The module tracks channels so a closing screen can detach them.    */
/* ------------------------------------------------------------------------- */

#define CL_SPRITE_COUNT 8
#define CL_SPRITE_ATTACHED 0x80
#define CL_SPRITE_CUSTOM ((volatile struct Custom *)0x00DFF000UL)

/* One hardware sprite channel. */
typedef struct cl_sprite {
    struct SimpleSprite simple;
    int channel;
    int height;
    int x;
    int y;
    int visible;
    int attach;
    UWORD *image;
    UWORD *blank;
    struct Screen *screen;
} cl_sprite;

/* One animation over a fixed set of frame images. */
typedef struct cl_sprite_animation {
    cl_sprite *sprite;
    int frame_count;
    int frame;
    UWORD **frames;
} cl_sprite_animation;

/* Channel ownership is process-wide because the hardware channels are. */
static cl_sprite *cl_sprite_channels[CL_SPRITE_COUNT];

/* Bytes in one sprite image: posctl[2] + data[height][2] + reserved[2]. */
static ULONG cl_sprite_image_bytes(int height)
{
    return (ULONG)(height * 2 + 4) * (ULONG)sizeof(UWORD);
}

/* Pack a 16-pixel-per-line image after the two-word posctl header. */
static void cl_sprite_pack(UWORD *image, int height, const UBYTE *pixels)
{
    int y;

    image[0] = 0;
    image[1] = 0;
    for (y = 0; y < height; y++) {
        UWORD plane_a = 0;
        UWORD plane_b = 0;
        int x;
        for (x = 0; x < 16; x++) {
            UBYTE value = (UBYTE)(pixels[y * 16 + x] & 0x03U);
            UWORD bit = (UWORD)(0x8000U >> (unsigned)x);
            if ((value & 0x01U) != 0) {
                plane_a |= bit;
            }
            if ((value & 0x02U) != 0) {
                plane_b |= bit;
            }
        }
        image[2 + y * 2] = plane_a;
        image[3 + y * 2] = plane_b;
    }
    image[2 + height * 2] = 0;
    image[3 + height * 2] = 0;
}

/* Change the sprite image and rebuild the screen's display list. */
static void cl_sprite_change_image(cl_sprite *sprite, UWORD *image)
{
    if (sprite == NULL || sprite->screen == NULL) {
        return;
    }
    ChangeSprite(&sprite->screen->ViewPort, &sprite->simple, image);
    if (sprite->attach && sprite->simple.posctldata != NULL) {
        sprite->simple.posctldata[1] |= CL_SPRITE_ATTACHED;
    }
    RethinkDisplay();
}

/* Move the live position; the 1.3 MoveSprite preserves the ATTACH bit. */
static void cl_sprite_move_apply(cl_sprite *sprite)
{
    if (sprite == NULL || sprite->screen == NULL) {
        return;
    }
    MoveSprite(&sprite->screen->ViewPort, &sprite->simple,
               (WORD)sprite->x, (WORD)sprite->y);
    if (sprite->attach && sprite->simple.posctldata != NULL) {
        sprite->simple.posctldata[1] |= CL_SPRITE_ATTACHED;
    }
}

/* Detach the channel and free the image, blank, and record. */
static void cl_sprite_destroy(cl_sprite *sprite, int free_images)
{
    if (sprite == NULL) {
        return;
    }
    if (sprite->screen != NULL) {
        sprite->visible = 0;
        cl_sprite_change_image(sprite, sprite->blank);
        if (cl_video_ensure_graphics()) {
            WaitTOF();
        }
    }
    (void)FreeSprite((WORD)sprite->simple.num);
    if (sprite->screen != NULL) {
        RethinkDisplay();
        sprite->screen = NULL;
    }
    if (free_images) {
        if (sprite->image != NULL) {
            FreeMem(sprite->image, cl_sprite_image_bytes(sprite->height));
        }
        if (sprite->blank != NULL) {
            FreeMem(sprite->blank, cl_sprite_image_bytes(sprite->height));
        }
    }
    if (sprite->channel >= 0 && sprite->channel < CL_SPRITE_COUNT) {
        cl_sprite_channels[sprite->channel] = NULL;
    }
    FreeMem(sprite, (ULONG)sizeof *sprite);
}

/* Allocate a channel and its image/blank buffers, or NULL. */
void *cl_sprite_new(UBYTE channel, UBYTE height)
{
    cl_sprite *sprite;
    UWORD *image;
    UWORD *blank;

    if (channel >= CL_SPRITE_COUNT || height < 1) {
        return NULL;
    }
    if (!cl_video_ensure_graphics()) {
        return NULL;
    }
    sprite = (cl_sprite *)AllocMem(
        (ULONG)sizeof *sprite, MEMF_ANY | MEMF_CLEAR);
    if (sprite == NULL) {
        return NULL;
    }
    sprite->simple.num = (UWORD)-1;
    if (GetSprite(&sprite->simple, (LONG)channel) < 0) {
        FreeMem(sprite, (ULONG)sizeof *sprite);
        return NULL;
    }
    sprite->simple.height = (UWORD)height;
    image = (UWORD *)AllocMem(
        cl_sprite_image_bytes((int)height), MEMF_CHIP | MEMF_CLEAR);
    blank = (UWORD *)AllocMem(
        cl_sprite_image_bytes((int)height), MEMF_CHIP | MEMF_CLEAR);
    if (image == NULL || blank == NULL) {
        if (image != NULL) {
            FreeMem(image, cl_sprite_image_bytes((int)height));
        }
        if (blank != NULL) {
            FreeMem(blank, cl_sprite_image_bytes((int)height));
        }
        (void)FreeSprite((WORD)sprite->simple.num);
        FreeMem(sprite, (ULONG)sizeof *sprite);
        return NULL;
    }
    sprite->channel = (int)channel;
    sprite->height = (int)height;
    sprite->x = 0;
    sprite->y = 0;
    sprite->visible = 0;
    sprite->attach = 0;
    sprite->image = image;
    sprite->blank = blank;
    sprite->screen = NULL;
    cl_sprite_channels[channel] = sprite;
    return sprite;
}

/* Hide the channel, wait one frame, and release the images and channel. */
void cl_sprite_free(void *handle)
{
    cl_sprite_destroy((cl_sprite *)handle, 1);
}

/* Attach a sprite to an Intuition screen's viewport. */
BOOL cl_sprite_attach_screen(void *sprite_handle, void *screen_handle)
{
    cl_sprite *sprite = (cl_sprite *)sprite_handle;
    struct Screen *screen = (struct Screen *)screen_handle;

    if (sprite == NULL || screen == NULL || sprite->screen != NULL) {
        return FALSE;
    }
    sprite->screen = screen;
    screen->ViewPort.Modes |= SPRITES;
    ChangeSprite(&screen->ViewPort, &sprite->simple,
                 sprite->visible ? sprite->image : sprite->blank);
    if (sprite->attach && sprite->simple.posctldata != NULL) {
        sprite->simple.posctldata[1] |= CL_SPRITE_ATTACHED;
    }
    MoveSprite(&screen->ViewPort, &sprite->simple,
               (WORD)sprite->x, (WORD)sprite->y);
    RethinkDisplay();
    return TRUE;
}

/* Detach a sprite from its screen before the screen is released. */
void cl_sprite_detach(void *handle)
{
    cl_sprite *sprite = (cl_sprite *)handle;

    if (sprite != NULL) {
        sprite->screen = NULL;
    }
}

/* Detach every channel that belongs to the closing screen. */
static void cl_sprite_screen_closing(struct Screen *screen)
{
    int index;

    for (index = 0; index < CL_SPRITE_COUNT; index++) {
        cl_sprite *sprite = cl_sprite_channels[index];
        if (sprite != NULL && sprite->screen == screen) {
            sprite->screen = NULL;
        }
    }
}

/* Pack `16 * height` pixels into the image; a wrong count is ignored. */
void cl_sprite_set_pixels(void *handle, APTR pixels, ULONG count)
{
    cl_sprite *sprite = (cl_sprite *)handle;

    if (sprite == NULL || pixels == NULL ||
        count != (ULONG)(sprite->height * 16)) {
        return;
    }
    cl_sprite_pack(sprite->image, sprite->height, (const UBYTE *)pixels);
    if (sprite->visible) {
        cl_sprite_change_image(sprite, sprite->image);
    }
}

/* Move the channel; the caller keeps the sprite inside the visible screen. */
void cl_sprite_move(void *handle, LONG x, LONG y)
{
    cl_sprite *sprite = (cl_sprite *)handle;

    if (sprite == NULL || x < 0 || x > 511 || y < 0 || y > 255) {
        return;
    }
    sprite->x = (int)x;
    sprite->y = (int)y;
    cl_sprite_move_apply(sprite);
}

/* Show the channel. */
void cl_sprite_show(void *handle)
{
    cl_sprite *sprite = (cl_sprite *)handle;

    if (sprite == NULL) {
        return;
    }
    sprite->visible = 1;
    cl_sprite_change_image(sprite, sprite->image);
}

/* Hide the channel with the zero image. */
void cl_sprite_hide(void *handle)
{
    cl_sprite *sprite = (cl_sprite *)handle;

    if (sprite == NULL) {
        return;
    }
    sprite->visible = 0;
    cl_sprite_change_image(sprite, sprite->blank);
}

/* Attach `odd` to `even`, making the 16-color pair. */
BOOL cl_sprite_attach(void *even_handle, void *odd_handle)
{
    cl_sprite *even = (cl_sprite *)even_handle;
    cl_sprite *odd = (cl_sprite *)odd_handle;

    if (even == NULL || odd == NULL || even->height != odd->height ||
        even->screen != odd->screen ||
        (even->channel & 1) != 0 || odd->channel != even->channel + 1) {
        return FALSE;
    }
    odd->attach = 1;
    if (odd->screen != NULL) {
        cl_sprite_change_image(odd, odd->visible ? odd->image : odd->blank);
    }
    return TRUE;
}

/* Geometry and state accessors. */
UBYTE cl_sprite_channel(void *handle)
{
    const cl_sprite *sprite = (const cl_sprite *)handle;

    return sprite == NULL ? (UBYTE)0xFF : (UBYTE)sprite->channel;
}

UBYTE cl_sprite_height(void *handle)
{
    const cl_sprite *sprite = (const cl_sprite *)handle;

    return sprite == NULL ? 0 : (UBYTE)sprite->height;
}

LONG cl_sprite_x(void *handle)
{
    const cl_sprite *sprite = (const cl_sprite *)handle;

    return sprite == NULL ? 0 : (LONG)sprite->x;
}

LONG cl_sprite_y(void *handle)
{
    const cl_sprite *sprite = (const cl_sprite *)handle;

    return sprite == NULL ? 0 : (LONG)sprite->y;
}

BOOL cl_sprite_visible(void *handle)
{
    const cl_sprite *sprite = (const cl_sprite *)handle;

    return sprite != NULL && sprite->visible ? TRUE : FALSE;
}

/* Enable the four sprite-pair collision groups. */
void cl_sprite_enable_collisions(void)
{
    CL_SPRITE_CUSTOM->clxcon = 0xF000;
}

/* Read and clear the collision data register. */
ULONG cl_sprite_collisions(void)
{
    return (ULONG)CL_SPRITE_CUSTOM->clxdat;
}

/* Create an animation over a fresh sprite channel and `frames` images. */
void *cl_sprite_animation_new(UBYTE channel, UBYTE height, UBYTE frames)
{
    cl_sprite_animation *animation;
    cl_sprite *sprite;
    int index;

    if (frames < 1 || frames > 8) {
        return NULL;
    }
    sprite = (cl_sprite *)cl_sprite_new(channel, height);
    if (sprite == NULL) {
        return NULL;
    }
    animation = (cl_sprite_animation *)AllocMem(
        (ULONG)sizeof *animation, MEMF_ANY | MEMF_CLEAR);
    if (animation == NULL) {
        cl_sprite_destroy(sprite, 1);
        return NULL;
    }
    animation->frames = (UWORD **)AllocMem(
        (ULONG)frames * (ULONG)sizeof(UWORD *), MEMF_ANY | MEMF_CLEAR);
    if (animation->frames == NULL) {
        FreeMem(animation, (ULONG)sizeof *animation);
        cl_sprite_destroy(sprite, 1);
        return NULL;
    }
    animation->sprite = sprite;
    animation->frame_count = (int)frames;
    animation->frame = 0;
    animation->frames[0] = sprite->image;
    for (index = 1; index < (int)frames; index++) {
        animation->frames[index] = (UWORD *)AllocMem(
            cl_sprite_image_bytes((int)height), MEMF_CHIP | MEMF_CLEAR);
        if (animation->frames[index] == NULL) {
            int free_index;
            for (free_index = 0; free_index < index; free_index++) {
                FreeMem(animation->frames[free_index],
                        cl_sprite_image_bytes((int)height));
            }
            FreeMem(animation->frames, (ULONG)frames * (ULONG)sizeof(UWORD *));
            FreeMem(animation, (ULONG)sizeof *animation);
            sprite->image = NULL;
            cl_sprite_destroy(sprite, 1);
            return NULL;
        }
    }
    return animation;
}

/* Release every frame, the frame table, the sprite, and the animation. */
void cl_sprite_animation_free(void *handle)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;
    cl_sprite *sprite;
    int height;
    int index;

    if (animation == NULL) {
        return;
    }
    sprite = animation->sprite;
    if (sprite != NULL) {
        height = sprite->height;
        /* The sprite record owns the channel and the blank image; the
         * animation owns frame 0 and the other frames. */
        sprite->image = NULL;
        cl_sprite_destroy(sprite, 1);
        for (index = 0; index < animation->frame_count; index++) {
            if (animation->frames[index] != NULL) {
                FreeMem(animation->frames[index],
                        cl_sprite_image_bytes(height));
            }
        }
    }
    if (animation->frames != NULL) {
        FreeMem(animation->frames,
                (ULONG)animation->frame_count * (ULONG)sizeof(UWORD *));
    }
    FreeMem(animation, (ULONG)sizeof *animation);
}

/* Pack one animation frame. */
void cl_sprite_animation_set_frame(void *handle, UBYTE frame, APTR pixels,
                                   ULONG count)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation == NULL || pixels == NULL ||
        frame >= (UBYTE)animation->frame_count ||
        count != (ULONG)(animation->sprite->height * 16)) {
        return;
    }
    cl_sprite_pack(animation->frames[frame], animation->sprite->height,
                   (const UBYTE *)pixels);
    if (animation->frame == (int)frame && animation->sprite->visible) {
        cl_sprite_change_image(animation->sprite, animation->frames[frame]);
    }
}

/* Show one animation frame. */
void cl_sprite_animation_show_frame(void *handle, UBYTE frame)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation == NULL || frame >= (UBYTE)animation->frame_count) {
        return;
    }
    animation->frame = (int)frame;
    animation->sprite->image = animation->frames[frame];
    animation->sprite->visible = 1;
    cl_sprite_change_image(animation->sprite, animation->frames[frame]);
}

/* Advance to the next frame and show it. */
void cl_sprite_animation_next(void *handle)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation == NULL) {
        return;
    }
    animation->frame = (animation->frame + 1) % animation->frame_count;
    cl_sprite_animation_show_frame(handle, (UBYTE)animation->frame);
}

/* Move, show, or hide the animation's sprite. */
void cl_sprite_animation_move(void *handle, LONG x, LONG y)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation != NULL) {
        cl_sprite_move(animation->sprite, x, y);
    }
}

void cl_sprite_animation_show(void *handle)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation != NULL) {
        cl_sprite_show(animation->sprite);
    }
}

void cl_sprite_animation_hide(void *handle)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation != NULL) {
        cl_sprite_hide(animation->sprite);
    }
}

/* Attach the animation's sprite to an Intuition screen. */
BOOL cl_sprite_animation_attach_screen(void *handle, void *screen)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation == NULL) {
        return FALSE;
    }
    return cl_sprite_attach_screen(animation->sprite, screen);
}

/* Detach the animation's sprite from its screen. */
void cl_sprite_animation_detach(void *handle)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation != NULL) {
        cl_sprite_detach(animation->sprite);
    }
}

/* Return the current frame index. */
UBYTE cl_sprite_animation_frame(void *handle)
{
    const cl_sprite_animation *animation =
        (const cl_sprite_animation *)handle;

    return animation == NULL ? 0 : (UBYTE)animation->frame;
}
