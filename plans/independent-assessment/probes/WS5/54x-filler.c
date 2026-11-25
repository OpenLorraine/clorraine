/* WS5 memory filler for the FS-UAE probes (54x-fs-uae.sh cases ending in "f").
 *
 * Allocates the free memory block by block, fills every block with the 68000
 * ILLEGAL opcode (0x4AFC), waits half a second so VBlank interrupts run while
 * the memory holds the pattern, frees everything, and prints "filler survived".
 * A VBlank server left installed in an unloaded program's memory then executes
 * or follows the pattern, so the machine crashes before the line is printed.
 */
#include <stdio.h>
#include <exec/memory.h>
#include <proto/exec.h>
#include <proto/dos.h>

int main(void)
{
    void *head = NULL;
    int blocks = 0;

    while (blocks < 256) {
        ULONG size = AvailMem(MEMF_ANY | MEMF_LARGEST);
        UWORD *block;
        ULONG i;

        if (size < 4096) {
            break;
        }
        size -= 1024;
        block = (UWORD *)AllocMem(size, MEMF_ANY);
        if (block == NULL) {
            break;
        }
        for (i = 0; i < size / 2; i++) {
            block[i] = 0x4AFC;
        }
        *(void **)block = head;
        ((ULONG *)block)[1] = size;
        head = block;
        blocks++;
    }
    Delay(25);
    while (head != NULL) {
        void *next = *(void **)head;
        FreeMem(head, ((ULONG *)head)[1]);
        head = next;
    }
    printf("filler survived %d\n", blocks);
    return 0;
}
