/*
 * The guest-side success oracle for a verified FS-UAE run.
 *
 * A directory-drive startup script cannot prove success on its own: a program
 * that returns a failure code still lets the next script line run, and FailAt
 * is unavailable on a bare directory drive. This wrapper runs the program
 * through SystemTagList, records the return code in clorraine-rc.txt, and writes
 * clorraine-status.txt only when the return code is zero. The host therefore
 * sees an unambiguous pair: rc 0 plus status "ok" on success, and no status on
 * any failure.
 *
 * The wrapper always returns zero, so the startup script never depends on an
 * AmigaDOS failure level. The program's own output is redirected to
 * clorraine-result.txt and is read by the host before the private run is
 * removed.
 */
#include <proto/dos.h>
#include <stdio.h>

int main(void) {
    LONG rc = SystemTagList("CNEXT:program >CNEXT:clorraine-result.txt", NULL);
    FILE *f = fopen("CNEXT:clorraine-rc.txt", "w");
    if (f) {
        fprintf(f, "%ld\n", (long)rc);
        fclose(f);
    }
    if (rc == 0) {
        f = fopen("CNEXT:clorraine-status.txt", "w");
        if (f) {
            fputs("ok\n", f);
            fclose(f);
        }
    }
    return 0;
}
