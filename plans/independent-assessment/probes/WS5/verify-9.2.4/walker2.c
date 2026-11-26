/* Verifier walker variant: counts the VERTB server chain and the nodes of
 * SysBase->PortList (no name lookup), plus FindPort("ClProbe541"), and appends
 * one line to argv[2] (or stdout).
 * Build: m68k-amigaos-gcc -std=c99 -mcrt=nix13|nix20 -m68000 -O2 -o walker2 walker2.c
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
    int ports = 0;
    int named;
    FILE *out = stdout;

    Forbid();
    servers = (struct List *)SysBase->IntVects[INTB_VERTB].iv_Data;
    if (servers != NULL) {
        for (node = servers->lh_Head; node->ln_Succ != NULL && vertb < 100; node = node->ln_Succ) {
            vertb++;
        }
    }
    for (node = SysBase->PortList.lh_Head; node->ln_Succ != NULL && ports < 1000; node = node->ln_Succ) {
        ports++;
    }
    named = FindPort((STRPTR)"ClProbe541") != NULL;
    Permit();
    if (argc > 2) {
        out = fopen(argv[2], "a");
        if (out == NULL) {
            return 5;
        }
    }
    fprintf(out, "%s vertb %d portlist %d named %d\n", tag, vertb, ports, named);
    if (out != stdout) {
        fclose(out);
    }
    return 0;
}
