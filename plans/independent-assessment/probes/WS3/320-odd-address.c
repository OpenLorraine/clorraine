/* WS3-05 detector check: does the emulator raise the 68000 address error for a
 * word access at an odd address? (Seeded violation; expectation in
 * 320-odd-address.expected.) The 68000 faults on a word or long access at an odd
 * address; the 68020 does not. */
#include <stdio.h>

int main(void)
{
    static unsigned char buf[8] = {1, 2, 3, 4, 5, 6, 7, 8};
    volatile unsigned short *p = (volatile unsigned short *)(void *)(buf + 1);
    unsigned short v = *p;
    printf("odd read %u\n", (unsigned)v);
    return 0;
}
