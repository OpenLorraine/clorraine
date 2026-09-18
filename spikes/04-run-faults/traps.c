/*
 * Spike S4 fixture: the emitted runtime guards and the fault shim.
 *
 * The program runs one fault case per invocation. Each case is written the way
 * the emitter will write it: a guard sits before the operation, and the guard's
 * failure path calls cl_trap, which prints exactly one line and aborts without
 * running the cleanup ladder.
 *
 *   division guard  d == 0 || (a == MIN && d == -1)      E11210
 *   bounds guard    i >= n                               E11211
 *   float guard     !(f >= lo && f <= hi)                E11212
 *   assert guard    !cond                                E11213
 *
 * Two shim variants are selected at compile time:
 *   - host:  fprintf(stderr, ...) then abort()
 *   - Amiga: printf(...) then exit(20), selected with -DAMIGA_SHIM
 *
 * A cleanup stand-in runs only after a successful operation. When a trap fires,
 * cl_trap never returns, so the cleanup output never appears; the harness uses
 * that to prove the ladder was skipped.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(__VBCC__)
typedef long i32;
typedef unsigned long u32;
#else
#include <stdint.h>
typedef int32_t i32;
typedef uint32_t u32;
#endif
typedef double f64;

#define I32_MINV ((i32)(-2147483647 - 1))

#ifdef AMIGA_SHIM
static void cl_trap(const char *code, const char *message)
{
    printf("error %s: %s\n", code, message);
    exit(20);
}
#else
static void cl_trap(const char *code, const char *message)
{
    fprintf(stderr, "error %s: %s\n", code, message);
    abort();
}
#endif

/*
 * The cleanup stand-in. If it ever runs, it prints, so a harness can tell that
 * the ladder was not skipped when it should have been.
 */
static unsigned cleanup_runs;

static void cleanup(void)
{
    cleanup_runs++;
    printf("cleanup ran (%u)\n", cleanup_runs);
}

/* Integer division: the guard covers a zero divisor and MIN / -1. */
static i32 guarded_div(i32 a, i32 d)
{
    if (d == 0 || (a == I32_MINV && d == -1)) {
        cl_trap("E11210", "integer division by zero");
    }
    return a / d;
}

/* Bounds: the emitted guard checks the index against the length. */
static i32 guarded_index(const i32 *xs, u32 n, u32 i)
{
    if (i >= n) {
        cl_trap("E11211", "index out of range");
    }
    return xs[i];
}

/*
 * Float-to-integer. This is the guard as it stood when the run-fault evidence
 * was captured; the checkpoint ruling adds a cl_isnan bit test in front of the
 * comparison, but the file keeps the original form so the recorded results
 * still match.
 */
static i32 guarded_float_to_int(f64 f)
{
    f64 lo = -2147483648.0;
    f64 hi = 2147483647.0;
    if (!(f >= lo && f <= hi)) {
        cl_trap("E11212", "float-to-integer conversion out of range");
    }
    return (i32)f;
}

/* assert: a false condition faults. */
static void guarded_assert(int cond)
{
    if (!cond) {
        cl_trap("E11213", "assertion failed");
    }
}

int main(int argc, char **argv)
{
    if (argc < 2) {
        fprintf(stderr, "usage: %s <case>\n", argv[0]);
        return 2;
    }

    if (strcmp(argv[1], "div_by_zero") == 0) {
        volatile i32 zero = 0;
        printf("result %ld\n", (long)guarded_div(10, zero));
    } else if (strcmp(argv[1], "min_neg_one") == 0) {
        volatile i32 neg_one = -1;
        printf("result %ld\n", (long)guarded_div(I32_MINV, neg_one));
    } else if (strcmp(argv[1], "index_out_of_range") == 0) {
        const i32 xs[3] = { 1, 2, 3 };
        volatile u32 i = 5;
        printf("result %ld\n", (long)guarded_index(xs, 3, i));
    } else if (strcmp(argv[1], "float_to_int_range") == 0) {
        volatile f64 big = 3.0e9;
        printf("result %ld\n", (long)guarded_float_to_int(big));
    } else if (strcmp(argv[1], "nan_float_to_int") == 0) {
        /*
         * Two distinct volatile zeros keep a compiler from folding 0.0/0.0 to
         * 1.0; IEEE division then yields a quiet NaN on every leg.
         */
        volatile f64 zero_a = 0.0;
        volatile f64 zero_b = 0.0;
        volatile f64 nan = zero_a / zero_b;
        printf("result %ld\n", (long)guarded_float_to_int(nan));
    } else if (strcmp(argv[1], "assertion_failed") == 0) {
        volatile int cond = 0;
        guarded_assert(cond);
        printf("result ok\n");
    } else {
        fprintf(stderr, "unknown case: %s\n", argv[1]);
        return 2;
    }

    cleanup();
    return 0;
}
