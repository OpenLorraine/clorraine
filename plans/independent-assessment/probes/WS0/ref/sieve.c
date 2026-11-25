/* Equivalent C reference for benchmarks/sieve: an 8,192-slot byte sieve run
 * ten times with every index checked. */
#include "ref_io.h"

#define N 8192

static int32_t sieveOnce(void)
{
    uint8_t flags[N];
    int32_t i = 2;
    int32_t count = 0;
    int32_t k = 0;
    memset(flags, 1, sizeof flags);
    flags[0] = 0;
    flags[1] = 0;
    while (i * i < N) {
        REF_CHECK(i, N);
        if (flags[i] == 1) {
            int32_t j = i * i;
            while (j < N) {
                REF_CHECK(j, N);
                flags[j] = 0;
                j = j + i;
            }
        }
        i = i + 1;
    }
    while (k < N) {
        REF_CHECK(k, N);
        if (flags[k] == 1) {
            count = count + 1;
        }
        k = k + 1;
    }
    return count;
}

int main(void)
{
    int32_t total = 0;
    int32_t r = 0;
    unsigned char *line;
    size_t n = 0;
    while (r < 10) {
        total = total + sieveOnce();
        r = r + 1;
    }
    line = (unsigned char *)ref_alloc(7 + 11 + 1);
    n += ref_put(line + n, "sieve: ");
    n += ref_format_i32(line + n, total);
    ref_write_line(line, n);
    free(line);
    return 0;
}
