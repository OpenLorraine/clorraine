/* Does vamos execute 68020 instructions? Built with -m68020, the 32-bit
 * multiply and divide below compile to mulu.l and divu.l, which a 68000
 * does not have. Under `vamos -C 68020` the program must print the expected
 * line; under the default 68000 CPU it must fail (contrasting input). */
#include <stdio.h>
#include <stdint.h>

int main(void)
{
    volatile uint32_t a = 123456789u;
    volatile uint32_t b = 987u;
    uint32_t product = a * b;
    uint32_t quotient = a / b;
    printf("cpu68020: %lu %lu\n", (unsigned long)product, (unsigned long)quotient);
    return 0;
}
