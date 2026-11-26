/* Prints lib_OpenCnt of graphics, intuition, and layers (0 when absent). */
#include <stdio.h>
#include <exec/execbase.h>
#include <proto/exec.h>

extern struct ExecBase *SysBase;

static int count(const char *name)
{
    struct Library *lib;
    int n;

    Forbid();
    lib = (struct Library *)FindName(&SysBase->LibList, (STRPTR)name);
    n = lib == NULL ? -1 : (int)lib->lib_OpenCnt;
    Permit();
    return n;
}

int main(int argc, char **argv)
{
    printf("%s gfx %d intuition %d layers %d\n", argc > 1 ? argv[1] : "libs",
           count("graphics.library"), count("intuition.library"), count("layers.library"));
    return 0;
}
