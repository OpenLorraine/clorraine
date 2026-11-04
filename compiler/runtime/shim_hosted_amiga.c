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

/* Suspends the calling task for `ticks` vertical-blank intervals. */
void cl_delay_ticks(ULONG ticks)
{
    Delay((LONG)ticks);
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

/* Close a screen. Null-safe. */
void cl_close_screen(void *screen_handle)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    if (screen != NULL) {
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
