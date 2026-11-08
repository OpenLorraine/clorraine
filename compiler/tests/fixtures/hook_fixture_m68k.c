/* Synchronous C hook fixture for the m68k ABI proof.
 *
 * The callback parameter is the declared `func(i32): i32` signature in the
 * Amiga ABI spelling, so the build proves the trampoline and wrapper agree
 * with the compiler-generated prototype.
 */

#include <exec/types.h>

LONG cl_hook_fixture_apply(LONG (*cb)(LONG), LONG value) {
    return cb(value);
}
