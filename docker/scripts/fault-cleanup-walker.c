/* Test-only fixture for the A500 fault-cleanup gate.
 *
 * Prints one line describing the system state a program that ends abnormally
 * could leave damaged:
 *
 *   <tag> vertb N port 0|1 workers N view sys|other screens N windows N
 *         sprites MASK fonts N chip N mem N
 *
 * - vertb: servers on the vertical-blank interrupt chain;
 * - port: 1 while a public port named "ClFaultCleanup" exists;
 * - workers: Clorraine worker tasks still on the ready or wait list;
 * - view: "sys" while graphics shows Intuition's own view;
 * - screens: screens open in Intuition;
 * - windows: windows open on the Workbench screen;
 * - sprites: the hardware sprite channels graphics has handed out;
 * - fonts: users of the diamond.font the probes open, 0 when it is not
 *   loaded. A font a program never closed keeps its user;
 * - chip: free chip memory in bytes;
 * - mem: free memory of every kind in bytes. The emulated A500 also has slow
 *   RAM, where the runtime's own structures (copper lists, layers, device
 *   requests) live, so chip memory alone misses those.
 *
 * Both memory fields return to the same value only when a program gave back
 * everything and loaded nothing that stays resident (a disk font or a disk
 * library does), so the gate compares them only for the probe that loads
 * nothing.
 *
 * No library open count is reported: diskfont.library's count did not move
 * after a program left it open, so it cannot show a leaked open here.
 *
 * The gate runs it before and after each probe; every field must match. The
 * state is read under Forbid (Disable for the interrupt chain) and printed
 * afterwards, so no output call runs with task switching off.
 *
 * Build: m68k-amigaos-gcc -mcrt=nix13 -O2 -Wall -Wextra -Werror. */

#include <exec/types.h>
#include <exec/execbase.h>
#include <exec/interrupts.h>
#include <exec/libraries.h>
#include <exec/memory.h>
#include <exec/lists.h>
#include <exec/ports.h>
#include <hardware/intbits.h>
#include <graphics/gfxbase.h>
#include <graphics/text.h>
#include <intuition/intuitionbase.h>
#include <intuition/screens.h>
#include <proto/exec.h>
#include <stdio.h>
#include <string.h>

struct GfxBase *GfxBase = NULL;
struct IntuitionBase *IntuitionBase = NULL;

/* Nodes on an Exec list. */
static ULONG count_nodes(struct List *list)
{
    ULONG count = 0;
    struct Node *node;

    for (node = list->lh_Head; node->ln_Succ != NULL; node = node->ln_Succ) {
        count++;
    }
    return count;
}

/* Nodes on an Exec list whose name is `name`. */
static ULONG count_named(struct List *list, const char *name)
{
    ULONG count = 0;
    struct Node *node;

    for (node = list->lh_Head; node->ln_Succ != NULL; node = node->ln_Succ) {
        if (node->ln_Name != NULL && strcmp(node->ln_Name, name) == 0) {
            count++;
        }
    }
    return count;
}

int main(int argc, char **argv)
{
    const char *tag = argc > 1 ? argv[1] : "state";
    ULONG vertb;
    ULONG port;
    ULONG workers;
    int system_view;
    ULONG screens = 0;
    ULONG windows = 0;
    ULONG sprites;
    ULONG fonts = 0;
    ULONG chip;
    ULONG mem;
    struct Screen *screen;
    struct Window *window;
    struct Node *node;

    GfxBase = (struct GfxBase *)OpenLibrary((STRPTR)"graphics.library", 0);
    IntuitionBase = (struct IntuitionBase *)OpenLibrary((STRPTR)"intuition.library", 0);
    if (GfxBase == NULL || IntuitionBase == NULL) {
        printf("%s walker: cannot open graphics or intuition\n", tag);
        return 20;
    }

    Disable();
    vertb = count_nodes((struct List *)SysBase->IntVects[INTB_VERTB].iv_Data);
    Enable();

    Forbid();
    port = FindPort((STRPTR)"ClFaultCleanup") != NULL ? 1 : 0;
    workers = count_named(&SysBase->TaskReady, "clorraine.worker") +
              count_named(&SysBase->TaskWait, "clorraine.worker");
    system_view = GfxBase->ActiView == &IntuitionBase->ViewLord;
    for (screen = IntuitionBase->FirstScreen; screen != NULL;
         screen = screen->NextScreen) {
        screens++;
        if ((screen->Flags & SCREENTYPE) == WBENCHSCREEN) {
            for (window = screen->FirstWindow; window != NULL;
                 window = window->NextWindow) {
                windows++;
            }
        }
    }
    sprites = GfxBase->SpriteReserved;
    for (node = GfxBase->TextFonts.lh_Head; node->ln_Succ != NULL;
         node = node->ln_Succ) {
        struct TextFont *font = (struct TextFont *)node;
        if (node->ln_Name != NULL && strcmp(node->ln_Name, "diamond.font") == 0) {
            fonts += font->tf_Accessors;
        }
    }
    Permit();
    chip = AvailMem(MEMF_CHIP);
    mem = AvailMem(MEMF_ANY);

    printf("%s vertb %lu port %lu workers %lu view %s screens %lu windows %lu "
           "sprites %lu fonts %lu chip %lu mem %lu\n",
           tag, (unsigned long)vertb, (unsigned long)port,
           (unsigned long)workers, system_view ? "sys" : "other",
           (unsigned long)screens, (unsigned long)windows,
           (unsigned long)sprites, (unsigned long)fonts, (unsigned long)chip,
           (unsigned long)mem);

    CloseLibrary((struct Library *)IntuitionBase);
    CloseLibrary((struct Library *)GfxBase);
    return 0;
}
