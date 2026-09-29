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
    unsigned char *grown = (unsigned char *)realloc(buffer->data, cap);
    if (grown == NULL) {
        cl_trap("E1", "out of memory");
    }
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
    memcpy(cl_string_bytes(string) + string->len, src, n);
    string->len += n;
    cl_string_bytes(string)[string->len] = '\0';
}

/* The first payload byte of a string; the trailing NUL makes it a C string. */
void *cl_string_ptr(void *handle)
{
    return cl_string_bytes((cl_string_block *)handle);
}

/* The number of live payload bytes in a string. */
unsigned int cl_string_len(void *handle)
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
static void *cl_string_from(const void *bytes, unsigned int n)
{
    void *string = cl_string_new(n);
    cl_string_append(string, (void *)bytes, n);
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
    unsigned int len;
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
void *cl_string_concat(const ClStrPart *parts, unsigned int count)
{
    unsigned int index;
    unsigned int total = 0;
    for (index = 0; index < count; index++) {
        total += parts[index].len;
    }
    void *string = cl_string_new(total);
    for (index = 0; index < count; index++) {
        cl_string_append(string, (void *)parts[index].ptr, parts[index].len);
        if (parts[index].owned != NULL) {
            cl_free(parts[index].owned);
        }
    }
    return string;
}

/* The text converters build the same String layout the templates desugar to,
 * so a template and an explicit `text` call format a value identically. */
void *cl_text_i32(int value)
{
    char buffer[24];
    int written = snprintf(buffer, sizeof buffer, "%d", value);
    if (written < 0) {
        cl_trap("E1", "text formatting failed");
    }
    return cl_string_from(buffer, (unsigned int)written);
}

void *cl_text_u32(unsigned int value)
{
    char buffer[24];
    int written = snprintf(buffer, sizeof buffer, "%u", value);
    if (written < 0) {
        cl_trap("E1", "text formatting failed");
    }
    return cl_string_from(buffer, (unsigned int)written);
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
