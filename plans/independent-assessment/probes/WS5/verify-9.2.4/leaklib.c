/* Seeded: opens graphics, intuition, and layers and never closes them. */
#include <stdio.h>
#include <proto/exec.h>
int main(void)
{
    void *a = OpenLibrary((STRPTR)"graphics.library", 33);
    void *b = OpenLibrary((STRPTR)"intuition.library", 33);
    void *c = OpenLibrary((STRPTR)"layers.library", 33);
    printf("leaklib %d %d %d\n", a != 0, b != 0, c != 0);
    return 0;
}
