/* Sanitizer self-test for the WS2 red check. Each mode commits one seeded defect;
 * the ASan/UBSan lines of 03-01 §Ground rules must report it:
 *   leak         - a heap block is never freed (LeakSanitizer, detect_leaks=1)
 *   double-free  - one block is freed twice (AddressSanitizer)
 *   use-after-free - a freed block is read (AddressSanitizer)
 *   overflow     - signed int overflow (UndefinedBehaviorSanitizer)
 *   clean        - no defect; every line must pass */
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv)
{
    const char *mode = argc > 1 ? argv[1] : "clean";
    volatile int big = INT_MAX;
    char *p = malloc(16);
    if (p == NULL) {
        return 2;
    }
    strcpy(p, "x");
    if (strcmp(mode, "leak") == 0) {
        p = NULL;
    } else if (strcmp(mode, "double-free") == 0) {
        free(p);
    } else if (strcmp(mode, "use-after-free") == 0) {
        free(p);
        printf("%c\n", p[0]);
        p = NULL;
        return 0;
    } else if (strcmp(mode, "overflow") == 0) {
        printf("%d\n", big + 1);
    }
    free(p);
    printf("%s ok\n", mode);
    return 0;
}
