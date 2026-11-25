/* Equivalent C reference for benchmarks/container: 1,024 point structs pushed
 * onto a growable array whose capacity doubles from eight bytes (the same
 * realloc sequence as the runtime buffer), then four checked scans. */
#include "ref_io.h"

typedef struct {
    int32_t x;
    int32_t y;
} Point;

typedef struct {
    Point *data;
    size_t count;
    size_t cap; /* in elements */
} PointArray;

static void push(PointArray *xs, Point v)
{
    if (xs->count == xs->cap) {
        size_t cap = xs->cap == 0 ? 1 : xs->cap * 2;
        Point *grown = (Point *)realloc(xs->data, cap * sizeof(Point));
        if (grown == NULL) {
            ref_trap("E1", "out of memory");
        }
        xs->data = grown;
        xs->cap = cap;
    }
    xs->data[xs->count++] = v;
}

static int32_t sumPoints(const PointArray *pts)
{
    int32_t total = 0;
    size_t i = 0;
    while (i < pts->count) {
        Point p;
        REF_CHECK(i, pts->count);
        p = pts->data[i];
        total = total + p.x * 2 + p.y;
        i = i + 1;
    }
    return total;
}

int main(void)
{
    PointArray pts = {NULL, 0, 0};
    int32_t i = 0;
    int32_t total = 0;
    int32_t pass = 0;
    unsigned char *line;
    size_t n = 0;
    while (i < 1024) {
        Point p;
        p.x = i;
        p.y = i * 3;
        push(&pts, p);
        i = i + 1;
    }
    while (pass < 4) {
        total = total + sumPoints(&pts);
        pass = pass + 1;
    }
    line = (unsigned char *)ref_alloc(11 + 11 + 1 + 10 + 1);
    n += ref_put(line + n, "container: ");
    n += ref_format_i32(line + n, total);
    n += ref_put(line + n, " ");
    n += ref_format_u32(line + n, (uint32_t)pts.count);
    ref_write_line(line, n);
    free(pts.data);
    free(line);
    return 0;
}
