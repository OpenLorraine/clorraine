/* Equivalent C reference for benchmarks/matrix: a 32x32 local array filled
 * with checked nested indexing, then summed row by row through a helper that
 * takes the row by pointer. */
#include "ref_io.h"

#define N 32

static int32_t rowSum(const int32_t row[N])
{
    int32_t total = 0;
    int32_t j = 0;
    while (j < N) {
        REF_CHECK(j, N);
        total = total + row[j];
        j = j + 1;
    }
    return total;
}

int main(void)
{
    int32_t a[N][N] = {{0}};
    int32_t i = 0;
    int32_t total = 0;
    unsigned char *line;
    size_t n = 0;
    while (i < N) {
        int32_t j = 0;
        while (j < N) {
            REF_CHECK(i, N);
            REF_CHECK(j, N);
            a[i][j] = i * N + j;
            j = j + 1;
        }
        i = i + 1;
    }
    i = 0;
    while (i < N) {
        REF_CHECK(i, N);
        total = total + rowSum(a[i]);
        i = i + 1;
    }
    line = (unsigned char *)ref_alloc(8 + 11 + 1);
    n += ref_put(line + n, "matrix: ");
    n += ref_format_i32(line + n, total);
    ref_write_line(line, n);
    free(line);
    return 0;
}
