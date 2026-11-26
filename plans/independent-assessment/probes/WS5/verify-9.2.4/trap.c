/* Verifier crash attempt for a dangling VBlank server: a program whose code
 * hunk starts with 96 KB of ILLEGAL (0x4AFC) words. When the shell loads it
 * into the memory a faulted program occupied, a stale server's is_Code lands
 * in the pad. It waits two seconds of VBlanks and prints a line if it survives.
 * Build: m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -O2 -o trap trap.c
 */
#include <stdio.h>
#include <proto/exec.h>

extern void __stdargs Delay(long ticks);

__asm__(".text\n.globl _cl_trap_pad\n_cl_trap_pad:\n.fill 49152,2,0x4afc\n");

int main(void)
{
    Delay(100);
    printf("trap survived\n");
    return 0;
}
