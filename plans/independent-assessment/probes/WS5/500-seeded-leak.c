/* Seeded defect for the WS5 vamos leak detector: an AllocMem block and an opened
 * library are never released; vamos must report an orphan and a library that
 * cannot be expunged. */
#include <exec/types.h>
#include <exec/memory.h>
#include <proto/exec.h>
#include <stdio.h>
int main(void)
{
    APTR p = AllocMem(1234, MEMF_PUBLIC);
    struct Library *u = OpenLibrary((STRPTR)"utility.library", 0);
    printf("leak %p %p\n", p, (void *)u);
    return 0;
}
