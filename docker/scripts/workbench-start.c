/* Test-only Workbench start fixture.
 *
 * The environment has licensed Kickstart ROMs but no Workbench disk, so
 * `C:LoadWB` is unavailable. This fixture calls the same private
 * `StartWorkbench` vector (LVO -42) that `C:LoadWB` uses, waits for the
 * `Workbench` public screen, and reports the result. It is not part of the
 * public SDK.
 */
#include <exec/types.h>
#include <dos/dos.h>
#include <workbench/workbench.h>
#include <clib/wb_protos.h>
#include <clib/alib_protos.h>
#include <proto/exec.h>
#include <proto/dos.h>
#include <proto/intuition.h>
#include <stdio.h>

extern struct Library *WorkbenchBase;

static BOOL call_start_workbench(ULONG flags, ULONG ptr)
{
    register struct Library *base __asm("a6") = WorkbenchBase;
    register ULONG d0 __asm("d0") = flags;
    register ULONG d1 __asm("d1") = ptr;

    __asm volatile ("jsr %/a6@(-42:W)"
                    : "+d"(d0), "+d"(d1)
                    : "a"(base)
                    : "a0", "a1", "cc", "memory");
    return (BOOL)d0;
}

int main(void)
{
    struct Screen *screen;
    BOOL started;

    WorkbenchBase = OpenLibrary((STRPTR)"workbench.library", 37);
    if (WorkbenchBase == NULL) {
        printf("startwb 0 screen 0\n");
        fflush(stdout);
        return 1;
    }
    started = call_start_workbench(0, 0);
    Delay(100);
    screen = LockPubScreen((STRPTR)"Workbench");
    printf("startwb %ld screen %lx\n", (long)started, (unsigned long)screen);
    fflush(stdout);
    if (screen != NULL) {
        UnlockPubScreen(NULL, screen);
    }
    CloseLibrary(WorkbenchBase);
    return (started && screen != NULL) ? 0 : 1;
}
