/*
 * Spike S3 leg: the vbcc variant of the arithmetic table.
 *
 * Compile with `vc +aos68k -c99 ... -DVBCC_IDIOM` so the wrapping +, -, *, and
 * unary - run on the unsigned counterpart, which vbcc defines without needing
 * a flag. Compiling without the macro records what plain signed vbcc does.
 * The shared cases live in arith_cases.h.
 */
#include "arith_cases.h"

int main(void)
{
    run_cases();
    return 0;
}
