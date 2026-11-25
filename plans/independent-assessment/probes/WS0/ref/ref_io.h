/* Shared helpers for the equivalent C references.
 *
 * Fairness rules (03-01 §Measurement rules): every reference uses the same
 * algorithm, integer widths, data layout, and allocation pattern as its
 * Clorraine benchmark, writes its output through the same stdio path the
 * runtime uses (fwrite of the bytes, then fputc of the newline), and performs
 * by hand the checks the specification requires: every index is
 * bounds-checked (fault E11211) and every allocation failure faults (E1).
 *
 * The output line is built the way the Clorraine template builder builds it:
 * one heap block sized for the literal bytes plus 11 bytes per signed and 10
 * per unsigned 32-bit placeholder, filled by a least-significant-first
 * decimal writer, written, then freed.
 */
#ifndef REF_IO_H
#define REF_IO_H

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Leave with the runtime fault contract: one line on stdout, exit 20. */
static void ref_trap(const char *code, const char *message)
{
    printf("error %s: %s\n", code, message);
    fflush(stdout);
    exit(20);
}

/* The bounds check the specification requires on every index. */
#define REF_CHECK(index, length)                                        \
    do {                                                                \
        if ((size_t)(index) >= (size_t)(length)) {                      \
            ref_trap("E11211", "index out of range");                   \
        }                                                               \
    } while (0)

/* Heap allocation that faults on failure, as the runtime allocator does. */
static void *ref_alloc(size_t size)
{
    void *block = malloc(size == 0 ? 1 : size);
    if (block == NULL) {
        ref_trap("E1", "out of memory");
    }
    return block;
}

/* Write `value` as decimal digits at `dst`; return the digit count. */
static size_t ref_format_u32(unsigned char *dst, uint32_t value)
{
    unsigned char digits[10];
    size_t count = 0;
    size_t index;
    while (value != 0 || count == 0) {
        digits[count++] = (unsigned char)('0' + (value % 10u));
        value /= 10u;
    }
    for (index = 0; index < count; index++) {
        dst[index] = digits[count - 1 - index];
    }
    return count;
}

/* Signed decimal; the magnitude is taken in unsigned arithmetic. */
static size_t ref_format_i32(unsigned char *dst, int32_t value)
{
    size_t written = 0;
    uint32_t magnitude = (uint32_t)value;
    if (value < 0) {
        dst[written++] = '-';
        magnitude = 0u - magnitude;
    }
    return written + ref_format_u32(dst + written, magnitude);
}

/* Append literal bytes at `dst`; return the count. */
static size_t ref_put(unsigned char *dst, const char *text)
{
    size_t n = strlen(text);
    memcpy(dst, text, n);
    return n;
}

/* Write `n` bytes and a newline to standard output, as the runtime does. */
static void ref_write_line(const void *bytes, size_t n)
{
    if (n > 0) {
        fwrite(bytes, 1, n, stdout);
    }
    fputc('\n', stdout);
}

#endif
