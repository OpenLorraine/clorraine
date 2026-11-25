/* Seeded known symbol for the 400-attribute red check: spin() executes a counted
 * loop of 20,000 iterations and nothing else does comparable work, so the attribution
 * must put most executed instructions (well above half) in `_spin`. */
#include <stdio.h>

__attribute__((noinline)) static int spin(int n)
{
    volatile int acc = 0;
    int i;
    for (i = 0; i < n; i++) {
        acc += i;
    }
    return acc;
}

int main(void)
{
    printf("%d\n", spin(20000));
    return 0;
}
