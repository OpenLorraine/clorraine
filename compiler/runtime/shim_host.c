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
