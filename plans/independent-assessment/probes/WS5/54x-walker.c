/* WS5 FS-UAE system-state walker (probes 540, 541).
 *
 * Prints one line describing the system state a Clorraine program must leave
 * unchanged when it exits: the number of servers on the VERTB interrupt chain,
 * whether the public port "ClProbe541" exists, and whether card.resource is in
 * the system resource list. The runner prints it before and after each probe
 * program; the expectation is that both lines are equal.
 *
 * Build: m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -O2 -o walker 54x-walker.c
 */
#include <stdio.h>
#include <exec/execbase.h>
#include <exec/interrupts.h>
#include <exec/lists.h>
#include <hardware/intbits.h>
#include <proto/exec.h>

extern struct ExecBase *SysBase;

int main(int argc, char **argv)
{
    const char *tag = argc > 1 ? argv[1] : "state";
    struct List *servers;
    struct Node *node;
    int vertb = 0;
    int port;
    int card;

    Forbid();
    servers = (struct List *)SysBase->IntVects[INTB_VERTB].iv_Data;
    if (servers != NULL) {
        for (node = servers->lh_Head; node->ln_Succ != NULL; node = node->ln_Succ) {
            vertb++;
        }
    }
    port = FindPort((STRPTR)"ClProbe541") != NULL;
    card = OpenResource((STRPTR)"card.resource") != NULL;
    Permit();
    printf("%s vertb %d port %d card %d\n", tag, vertb, port, card);
    return 0;
}
