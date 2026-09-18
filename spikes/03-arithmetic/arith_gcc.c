/*
 * Spike S3 leg: the GCC and host variant of the arithmetic table.
 *
 * Compile the host leg with `cc -std=c99 -fwrapv` and the m68k leg with
 * `m68k-amigaos-gcc ... -fwrapv`. The shared cases live in arith_cases.h; this
 * file only provides the entry point. The vbcc variant is arith_vbcc.c.
 */
#include "arith_cases.h"

int main(void)
{
    run_cases();
    return 0;
}
