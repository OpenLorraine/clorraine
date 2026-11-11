/* Host runtime shim for generated Clorraine C.
 *
 * The shim contains no ownership logic; it only provides the small set of
 * helpers the emitted translation unit calls. Everything here is `static
 * inline` (or a macro) so a program that uses only some of the helpers still
 * compiles clean under `-Wall -Wextra -Werror`, which does not warn about an
 * unused inline function.
 */

#include <limits.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Zero one object through its address. A macro so a call site writes the
 * single argument `cl_zero(&value)`; `sizeof` supplies the byte count. */
#define cl_zero(pointer) memset((void *)(pointer), 0, sizeof(*(pointer)))

/* IEEE-754 NaN test on the raw bit pattern.
 *
 * The comparison-based tests are not portable to the soft-float targets the
 * project targets, so the exponent and mantissa are inspected directly. A
 * `double` parameter accepts a `float` argument through the usual promotion. */
static inline int cl_isnan(double value)
{
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof bits);
    if ((bits & UINT64_C(0x7ff0000000000000)) != UINT64_C(0x7ff0000000000000)) {
        return 0;
    }
    return (bits & UINT64_C(0x000fffffffffffff)) != 0;
}

/* Abort with a runtime fault. The first line is exactly `error CODE: message`,
 * which is the fault contract the run tests assert. */
static inline void cl_trap(const char *code, const char *message)
{
    fprintf(stderr, "error %s: %s\n", code, message);
    fflush(stderr);
    abort();
}

/* The byte length of a NUL-terminated C string. The pointer forms of a panic
 * message keep printing their text through the length-based runtime. */
static inline size_t cl_cstring_len(const char *text)
{
    return strlen(text);
}

/* Abort on a user-level panic. The message is passed as pointer and length, so
 * a view or handle needs no NUL terminator; the bytes are written exactly as
 * they are. */
static inline void cl_panic(const unsigned char *message, size_t len)
{
    fwrite("panic: ", 1, 7, stderr);
    fwrite(message, 1, len, stderr);
    fwrite("\n", 1, 1, stderr);
    fflush(stderr);
    abort();
}

/* Heap allocation with the allocator contract: a failure traps. The size is a
 * `size_t`, so a caller that computes a block size in `size_t` never has it
 * truncated back to the 32-bit `unsigned int` an AmigaOS word would hold. */
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

/* Allocate one zeroed box block. The language's empty state is all-zero, so
 * the first store's old-value drop is a no-op. An allocation failure keeps the
 * `cl_alloc` contract: it traps. */
void *cl_box_alloc(size_t size)
{
    void *block = cl_alloc(size == 0 ? 1 : size);
    memset(block, 0, size == 0 ? 1 : size);
    return block;
}

/* Release a box block. Null-safe. */
void cl_box_free(void *block)
{
    cl_free(block);
}

/* Byte copy and compare helpers the library may bind. */
void *cl_memcpy(void *dst, const void *src, unsigned int n)
{
    return memcpy(dst, src, n);
}

int cl_memcmp(const void *left, const void *right, unsigned int n)
{
    return memcmp(left, right, n);
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
    unsigned int len;
    unsigned int cap;
} cl_buffer_block;

/* Allocate a buffer with `size` readable bytes. An allocation failure traps,
 * matching the allocator contract. */
void *cl_buffer_new(unsigned int size)
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
unsigned int cl_buffer_len(void *handle)
{
    return ((cl_buffer_block *)handle)->len;
}

/* Grow a buffer's capacity to at least `cap` bytes. */
void cl_buffer_reserve(void *handle, unsigned int cap)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    if (cap <= buffer->cap) {
        return;
    }
    unsigned int old_cap = buffer->cap;
    unsigned char *grown = (unsigned char *)realloc(buffer->data, cap);
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
void cl_buffer_append(void *handle, void *src, unsigned int n)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    if (n > UINT_MAX - buffer->len) {
        cl_trap("E1", "buffer too large");
    }
    unsigned int needed = buffer->len + n;
    if (needed > buffer->cap) {
        unsigned int cap = buffer->cap == 0 ? 8 : buffer->cap;
        while (cap < needed) {
            if (cap > UINT_MAX / 2) {
                cap = needed;
                break;
            }
            cap *= 2;
        }
        cl_buffer_reserve(handle, cap);
    }
    memcpy(buffer->data + buffer->len, src, n);
    buffer->len = needed;
}

/* Write `n` bytes from `src` at `off`, growing the buffer as needed. The
 * offset and count are both bounded so `off + n` cannot wrap and the copy stays
 * inside the buffer. */
void cl_buffer_write(void *handle, unsigned int off, void *src, unsigned int n)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    if (n > UINT_MAX - off) {
        cl_trap("E11211", "index out of range");
    }
    unsigned int needed = off + n;
    if (needed > buffer->cap) {
        cl_buffer_reserve(handle, needed);
    }
    memcpy(buffer->data + off, src, n);
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
    unsigned int len;
    unsigned int cap;
} cl_string_block;

/* The first payload byte of a string block. */
static unsigned char *cl_string_bytes(cl_string_block *string)
{
    return (unsigned char *)(string + 1);
}

/* Allocate an empty string with room for `cap` payload bytes.
 *
 * The block size is computed in `size_t`, a wider type than the 32-bit `cap`
 * on the host, so the size never wraps and a request too large to allocate
 * traps through the allocator contract instead of writing past a short block. */
void *cl_string_new(unsigned int cap)
{
    // The modeled `usize` is 32 bits, so a block of `cap` payload bytes must
    // still fit a 32-bit size; the top few capacities do not, and are rejected
    // rather than asking the host allocator for a block the target cannot
    // address.
    if (cap > UINT_MAX - (unsigned int)(sizeof(cl_string_block) + 1)) {
        cl_trap("E1", "string too large");
    }
    size_t total = sizeof(cl_string_block) + (size_t)cap + 1;
    cl_string_block *string = (cl_string_block *)cl_alloc(total);
    string->len = 0;
    string->cap = cap;
    cl_string_bytes(string)[0] = '\0';
    return string;
}

/* Append `n` bytes from `src`. The caller must have reserved the capacity. */
void cl_string_append(void *handle, void *src, unsigned int n)
{
    cl_string_block *string = (cl_string_block *)handle;
    if (n > string->cap - string->len) {
        cl_trap("E1", "string capacity exceeded");
    }
    /* A zero-length append may carry a null view pointer; skip the copy so
     * the call is valid under UndefinedBehaviorSanitizer. */
    if (n != 0) {
        memcpy(cl_string_bytes(string) + string->len, src, n);
    }
    string->len += n;
    cl_string_bytes(string)[string->len] = '\0';
}

/* The first payload byte of a string; the trailing NUL makes it a C string.
 * The all-zero handle is the empty state (O7), so its pointer is NULL. */
void *cl_string_ptr(void *handle)
{
    if (handle == NULL) {
        return NULL;
    }
    return cl_string_bytes((cl_string_block *)handle);
}

/* The number of live payload bytes in a string. The zero state is empty. */
unsigned int cl_string_len(void *handle)
{
    if (handle == NULL) {
        return 0;
    }
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
static void *cl_string_from(const void *bytes, unsigned int n)
{
    void *string = cl_string_new(n);
    cl_string_append(string, (void *)bytes, n);
    return string;
}

/* Copy a whole string block into a fresh, independent block. The caller owns
 * the copy and releases it through the string deleter. The all-zero handle is
 * the empty state (O7), so its copy is an empty string. */
void *cl_string_copy(void *handle)
{
    cl_string_block *string = (cl_string_block *)handle;
    if (string == NULL) {
        return cl_string_new(0);
    }
    return cl_string_from(cl_string_bytes(string), string->len);
}

/* The decimal writer is defined below the builder; these prototypes keep the
 * builder's write path above its callers. */
static unsigned int cl_format_u32(unsigned char *dst, unsigned int value);
static unsigned int cl_format_i32(unsigned char *dst, int value);
void *cl_text_f64(double value);

/* Kind codes for a scalar piece: `len` holds the code and `ptr` is NULL.
 * The codes start above the byte lengths a zero view can collide with, so a
 * `ptr == NULL, len == 0` piece is the empty byte piece, not a scalar. */
#define CL_STR_KIND_I32 2u
#define CL_STR_KIND_U32 3u

typedef struct cl_str_part {
    const unsigned char *ptr;
    unsigned int len;
    union {
        void *owned;
        int i;
        unsigned int u;
    } value;
} ClStrPart;

/* Wrap an owned string as a template piece. The builder takes the string over
 * and releases it, so the caller must not use the handle afterwards. */
static inline ClStrPart cl_str_part_owned(void *string)
{
    ClStrPart part;
    part.ptr = (const unsigned char *)cl_string_ptr(string);
    part.len = cl_string_len(string);
    part.value.owned = string;
    return part;
}

/* A borrowed byte piece. A zero-length view is valid here; the scalar kinds
 * start at 2 so it is never read as a scalar. */
static inline ClStrPart cl_str_part_bytes(const unsigned char *ptr, unsigned int len)
{
    ClStrPart part;
    part.ptr = ptr;
    part.len = len;
    part.value.owned = NULL;
    return part;
}

static inline ClStrPart cl_str_part_i32(int value)
{
    ClStrPart part;
    part.ptr = NULL;
    part.len = CL_STR_KIND_I32;
    part.value.i = value;
    return part;
}

static inline ClStrPart cl_str_part_u32(unsigned int value)
{
    ClStrPart part;
    part.ptr = NULL;
    part.len = CL_STR_KIND_U32;
    part.value.u = value;
    return part;
}

/* A bool piece points at the static text; no allocation is needed. */
static inline ClStrPart cl_str_part_bool(int value)
{
    static const unsigned char truth[5] = "true";
    static const unsigned char falsehood[6] = "false";
    ClStrPart part;
    part.ptr = value ? truth : falsehood;
    part.len = value ? 4u : 5u;
    part.value.owned = NULL;
    return part;
}

/* A float piece keeps the documented `%g` conversion and its one temporary;
 * the builder releases the temporary after copying the bytes. */
static inline ClStrPart cl_str_part_f64(double value)
{
    return cl_str_part_owned(cl_text_f64(value));
}

/* Concatenate template pieces into one owned string.
 *
 * The first pass reserves the maximum width of every scalar piece (11 bytes
 * for an i32, 10 for a u32) and the exact length of every byte piece. The
 * second pass writes the digits or copies the bytes, then sets `len` and `cap`
 * to the written count so a later append still traps. The bytes a `String` or
 * `str` placeholder needs are copied at part construction, before a sibling
 * placeholder can release the source. */
void *cl_string_concat(const ClStrPart *parts, unsigned int count)
{
    unsigned int index;
    unsigned int total = 0;
    for (index = 0; index < count; index++) {
        unsigned int extra = parts[index].len;
        if (parts[index].ptr == NULL && parts[index].len >= CL_STR_KIND_I32) {
            extra = parts[index].len == CL_STR_KIND_I32 ? 11u : 10u;
        }
        unsigned int next = total + extra;
        if (next < total) {
            cl_trap("E1", "string too large");
        }
        total = next;
    }
    cl_string_block *string = (cl_string_block *)cl_string_new(total);
    unsigned char *dst = cl_string_bytes(string);
    unsigned int len = 0;
    for (index = 0; index < count; index++) {
        const ClStrPart *part = &parts[index];
        if (part->ptr == NULL && part->len >= CL_STR_KIND_I32) {
            if (part->len == CL_STR_KIND_I32) {
                len += cl_format_i32(dst + len, part->value.i);
                continue;
            }
            len += cl_format_u32(dst + len, part->value.u);
            continue;
        }
        if (part->len != 0) {
            memcpy(dst + len, part->ptr, (size_t)part->len);
        }
        len += part->len;
        if (part->value.owned != NULL) {
            cl_free(part->value.owned);
        }
    }
    string->len = len;
    string->cap = len;
    dst[len] = '\0';
    return string;
}

/* The decimal writer assumes the modeled 32-bit `int`; a target with a wider
 * or narrower int must fail the build instead of silently truncating text. */
#if INT_MAX != 2147483647 || UINT_MAX != 4294967295u
#error "the decimal formatter assumes a 32-bit int and unsigned int"
#endif

/* Write `value` as unsigned decimal digits at `dst` and return the count.
 *
 * The digits are produced least-significant first, then copied into reading
 * order. The buffer holds the ten digits of `UINT_MAX`. */
static unsigned int cl_format_u32(unsigned char *dst, unsigned int value)
{
    unsigned char digits[10];
    unsigned int count = 0;
    /* `count == 0` makes the first pass unconditional, so zero prints "0". */
    while (value != 0 || count == 0) {
        digits[count++] = (unsigned char)('0' + (value % 10u));
        value /= 10u;
    }
    unsigned int index;
    for (index = 0; index < count; index++) {
        dst[index] = digits[count - 1 - index];
    }
    return count;
}

/* Write a signed decimal at `dst` and return the count.
 *
 * `0u - (unsigned int)value` produces the magnitude of `INT_MIN` without
 * signed overflow; negating in signed arithmetic would be undefined. */
static unsigned int cl_format_i32(unsigned char *dst, int value)
{
    unsigned int written = 0;
    unsigned int magnitude = (unsigned int)value;
    if (value < 0) {
        dst[written++] = '-';
        magnitude = 0u - magnitude;
    }
    return written + cl_format_u32(dst + written, magnitude);
}

/* The text converters build the same String layout the templates desugar to,
 * so a template and an explicit `text` call format a value identically.
 *
 * The integer converters allocate the maximum decimal width and write the
 * digits in place: `len` and `cap` are then set to the written count, so the
 * block's logical capacity is exact and the trailing NUL closes the payload. */
void *cl_text_i32(int value)
{
    cl_string_block *string = (cl_string_block *)cl_string_new(11);
    unsigned char *dst = cl_string_bytes(string);
    unsigned int written = cl_format_i32(dst, value);
    string->len = written;
    string->cap = written;
    dst[written] = '\0';
    return string;
}

void *cl_text_u32(unsigned int value)
{
    cl_string_block *string = (cl_string_block *)cl_string_new(10);
    unsigned char *dst = cl_string_bytes(string);
    unsigned int written = cl_format_u32(dst, value);
    string->len = written;
    string->cap = written;
    dst[written] = '\0';
    return string;
}

/* Six significant digits: `%g` is the C default-precision spelling of that. */
void *cl_text_f64(double value)
{
    char buffer[32];
    int written = snprintf(buffer, sizeof buffer, "%g", value);
    if (written < 0) {
        cl_trap("E1", "text formatting failed");
    }
    return cl_string_from(buffer, (unsigned int)written);
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
void *cl_text_str(const void *bytes, unsigned int n)
{
    return cl_string_from(bytes, n);
}

/* `0x`-prefixed uppercase hex, with no leading zeros beyond the value. */
void *cl_text_hex_u32(unsigned int value)
{
    static const char digits[] = "0123456789ABCDEF";
    char buffer[10];
    unsigned int count = 1;
    unsigned int remaining = value;
    while (remaining >= 16) {
        remaining >>= 4;
        count++;
    }
    buffer[0] = '0';
    buffer[1] = 'x';
    unsigned int index = 0;
    while (index < count) {
        unsigned int shift = 4 * (count - 1 - index);
        buffer[2 + index] = digits[(value >> shift) & 0x0f];
        index++;
    }
    return cl_string_from(buffer, 2 + count);
}

/* Pad `src` on the left to `width` bytes with `fill`. */
void *cl_text_pad_left(void *src, unsigned int n, unsigned int width, unsigned char fill)
{
    unsigned int total = width > n ? width : n;
    void *string = cl_string_new(total);
    unsigned int pad = total - n;
    while (pad > 0) {
        cl_string_append(string, &fill, 1);
        pad--;
    }
    cl_string_append(string, (void *)src, n);
    return string;
}

/* Pad `src` on the right to `width` bytes with `fill`. */
void *cl_text_pad_right(void *src, unsigned int n, unsigned int width, unsigned char fill)
{
    unsigned int total = width > n ? width : n;
    void *string = cl_string_new(total);
    unsigned int pad = total - n;
    cl_string_append(string, (void *)src, n);
    while (pad > 0) {
        cl_string_append(string, &fill, 1);
        pad--;
    }
    return string;
}

/* A file handle on the host is a libc stream. The generated code spells the
 * `File` contract handle as its ABI type `BPTR` (an opaque pointer), so the
 * definitions below take and return that primitive pointer and cast to the
 * libc stream type internally. The Clorraine wrapper adds the mode constants
 * and the `File?` result. */
typedef FILE *ClFile;

/* The two Clorraine file modes, mapped to the libc open spelling. */
#define CL_MODE_OLDFILE 1005
#define CL_MODE_NEWFILE 1006

/* Open `path` for reading or writing; a failure returns `NULL`. */
void *Open(char *path, int mode)
{
    const char *spelling = (mode == CL_MODE_NEWFILE) ? "wb" : "rb";
    return (void *)fopen(path, spelling);
}

/* Close a file handle. Null-safe, because the generated drop is null-safe but
 * a caller may also close an already-empty handle. */
void Close(void *file)
{
    if (file != NULL) {
        fclose((ClFile)file);
    }
}

/* Read up to `len` bytes into `buf` and return the count moved. */
int Read(void *file, void *buf, int len)
{
    return (int)fread(buf, 1, (size_t)len, (ClFile)file);
}

/* Write `len` bytes from `buf` and return the count moved. */
int Write(void *file, void *buf, int len)
{
    return (int)fwrite(buf, 1, (size_t)len, (ClFile)file);
}

/* Write `n` bytes to standard output without a trailing newline. */
void cl_write(void *buf, unsigned int n)
{
    if (n > 0) {
        fwrite(buf, 1, n, stdout);
    }
}

/* Write `n` bytes to standard output followed by a newline. */
void cl_write_line(void *buf, unsigned int n)
{
    if (n > 0) {
        fwrite(buf, 1, n, stdout);
    }
    fputc('\n', stdout);
}
