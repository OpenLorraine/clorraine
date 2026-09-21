/* Host runtime shim for generated Clorraine C.
 *
 * The shim contains no ownership logic; it only provides the small set of
 * helpers the emitted translation unit calls. Everything here is `static
 * inline` (or a macro) so a program that uses only some of the helpers still
 * compiles clean under `-Wall -Wextra -Werror`, which does not warn about an
 * unused inline function.
 */

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

/* Abort on a user-level panic. */
static inline void cl_panic(const char *message)
{
    fprintf(stderr, "panic: %s\n", message);
    fflush(stderr);
    abort();
}

/* Heap allocation with the allocator contract: a failure traps. */
void *cl_alloc(unsigned int size)
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
    unsigned int needed = buffer->len + n;
    if (needed > buffer->cap) {
        unsigned int cap = buffer->cap == 0 ? 8 : buffer->cap;
        while (cap < needed) {
            cap *= 2;
        }
        cl_buffer_reserve(handle, cap);
    }
    memcpy(buffer->data + buffer->len, src, n);
    buffer->len = needed;
}

/* Write `n` bytes from `src` at `off`, growing the buffer as needed. */
void cl_buffer_write(void *handle, unsigned int off, void *src, unsigned int n)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    unsigned int needed = off + n;
    if (needed > buffer->cap) {
        cl_buffer_reserve(handle, needed);
    }
    memcpy(buffer->data + off, src, n);
    if (needed > buffer->len) {
        buffer->len = needed;
    }
}
