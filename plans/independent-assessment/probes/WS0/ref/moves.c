/* Equivalent C reference for benchmarks/moves: a struct with a fixed-array
 * field built, passed through an identity function, and summed by value
 * 1,000 times, then one heap string of "moves" whose length is added. */
#include "ref_io.h"

typedef struct {
    int32_t values[8];
    int32_t tag;
} Box;

static Box makeBox(int32_t n)
{
    Box b;
    b.values[0] = n;
    b.values[1] = n + 1;
    b.values[2] = n + 2;
    b.values[3] = n + 3;
    b.values[4] = n + 4;
    b.values[5] = n + 5;
    b.values[6] = n + 6;
    b.values[7] = n + 7;
    b.tag = n;
    return b;
}

static Box id(Box x)
{
    return x;
}

static int32_t sumBox(Box b)
{
    int32_t total = 0;
    size_t i = 0;
    while (i < 8) {
        REF_CHECK(i, 8);
        total = total + b.values[i];
        i = i + 1;
    }
    return total + b.tag;
}

int main(void)
{
    static const char text[] = "moves";
    int32_t total = 0;
    int32_t i = 0;
    char *name;
    size_t name_len = sizeof text - 1;
    unsigned char *line;
    size_t n = 0;
    while (i < 1000) {
        Box b = makeBox(i);
        Box c = id(b);
        total = total + sumBox(c);
        i = i + 1;
    }
    name = (char *)ref_alloc(name_len + 1);
    memcpy(name, text, name_len + 1);
    total = total + (int32_t)name_len;
    line = (unsigned char *)ref_alloc(7 + 11 + 1);
    n += ref_put(line + n, "moves: ");
    n += ref_format_i32(line + n, total);
    ref_write_line(line, n);
    free(line);
    free(name);
    return 0;
}
