/* WS5 seeded defect for the FS-UAE walker (54x-fs-uae.sh case "seeded").
 *
 * Deliberately leaves the system in the three states the walker reports:
 * a VERTB server still installed after exit (its node and code live in
 * AllocMem memory that is never freed, so the server stays harmless), the
 * public port "ClProbe541" still in the port list, and card.resource removed.
 */
#include <stdio.h>
#include <exec/interrupts.h>
#include <exec/memory.h>
#include <hardware/intbits.h>
#include <proto/exec.h>
#include <clib/alib_protos.h>

int main(void)
{
    struct Interrupt *server;
    UWORD *code;
    APTR card;

    server = (struct Interrupt *)AllocMem(sizeof *server, MEMF_PUBLIC | MEMF_CLEAR);
    code = (UWORD *)AllocMem(4, MEMF_PUBLIC);
    if (server == NULL || code == NULL) {
        return 20;
    }
    code[0] = 0x7000; /* moveq #0,d0 */
    code[1] = 0x4E75; /* rts */
    CacheClearU();
    server->is_Node.ln_Type = NT_INTERRUPT;
    server->is_Code = (void (*)(void))code;
    AddIntServer(INTB_VERTB, server);
    (void)CreatePort((STRPTR)"ClProbe541", 0);
    card = OpenResource((STRPTR)"card.resource");
    if (card != NULL) {
        RemResource(card);
    }
    printf("seeded\n");
    return 0;
}
