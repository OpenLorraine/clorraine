/*
 * Shared arithmetic cases for the wrapping/shift spike.
 *
 * One table of cases drives the host leg, the m68k GCC leg, and the vbcc leg.
 * The signed operations are written so their result is defined on the target:
 *
 *   - GCC and the host require -fwrapv for wrapping signed +, -, *, and unary -;
 *   - vbcc has no such flag, so -DVBCC_IDIOM rewrites those operations to run
 *     on the unsigned counterpart and converts the result back;
 *   - shifts are always emitted through unsigned operands and the count is
 *     masked to the operand width, so both compilers agree;
 *   - signed >> is arithmetic (sign-replicating) and the logical >> is written
 *     on the unsigned value;
 *   - / and % sit behind the emitted guard, so the MIN / -1 and divide-by-zero
 *     cases return a sentinel instead of executing undefined behavior.
 *
 * The printed format is one line per case: "name = 0xXXXXXXXX". expected.txt
 * holds the hand-computed table; every leg is diffed against it.
 */
#ifndef ARITH_CASES_H
#define ARITH_CASES_H

#include <stdio.h>

/*
 * A fixed 32-bit signed type. vbcc on m68k has no stdint.h in this setup, so
 * it uses long (32-bit); the host and m68k GCC use int32_t.
 */
#if defined(__VBCC__)
typedef long i32;
typedef unsigned long u32;
#else
#include <stdint.h>
typedef int32_t i32;
typedef uint32_t u32;
#endif

#define I32_MINV ((i32)(-2147483647 - 1))
#define I32_MAXV ((i32)2147483647)
#define TRAP_SENTINEL ((i32)0x5A5A5A5A)

/* Wrapping +, -, *, and unary -, selected by the target. */
#if defined(VBCC_IDIOM)
#define WRAP_ADD(a, b) ((i32)((u32)(a) + (u32)(b)))
#define WRAP_SUB(a, b) ((i32)((u32)(a) - (u32)(b)))
#define WRAP_MUL(a, b) ((i32)((u32)(a) * (u32)(b)))
#define WRAP_NEG(a)    ((i32)(0u - (u32)(a)))
#else
#define WRAP_ADD(a, b) ((i32)((a) + (b)))
#define WRAP_SUB(a, b) ((i32)((a) - (b)))
#define WRAP_MUL(a, b) ((i32)((a) * (b)))
#define WRAP_NEG(a)    ((i32)(-(a)))
#endif

/* Shifts: the count is masked to the operand width; the operand is unsigned. */
static i32 op_shl(i32 a, i32 count)
{
    return (i32)((u32)a << ((u32)count & 31u));
}

/* Signed right shift stays signed, so it replicates the sign on both compilers. */
static i32 op_shr_arith(i32 a, i32 count)
{
    return (i32)(a >> ((u32)count & 31u));
}

/* Logical right shift is written on the unsigned value. */
static i32 op_shr_logical(i32 a, i32 count)
{
    return (i32)((u32)a >> ((u32)count & 31u));
}

/* The emitted division guard; a trapped case returns the sentinel. */
static i32 guarded_div(i32 a, i32 d)
{
    if (d == 0 || (a == I32_MINV && d == -1)) {
        return TRAP_SENTINEL;
    }
    return (i32)(a / d);
}

static i32 guarded_mod(i32 a, i32 d)
{
    if (d == 0 || (a == I32_MINV && d == -1)) {
        return TRAP_SENTINEL;
    }
    return (i32)(a % d);
}

static void print_case(const char *name, i32 value)
{
    printf("%s = 0x%08lX\n", name, (unsigned long)(u32)value);
}

/*
 * Volatile inputs keep the compiler from folding the cases at compile time, so
 * the no-flag leg records the real runtime behavior rather than a constant.
 */
static void run_cases(void)
{
    volatile i32 v0 = 0;
    volatile i32 v1 = 1;
    volatile i32 v2 = 2;
    volatile i32 v3 = 3;
    volatile i32 v15 = 15;
    volatile i32 v31 = 31;
    volatile i32 v32 = 32;
    volatile i32 v33 = 33;
    volatile i32 vmin = I32_MINV;
    volatile i32 vmax = I32_MAXV;
    volatile i32 vneg1 = -1;
    volatile i32 vneg7 = -7;
    volatile i32 vneg16 = -16;

    print_case("add_max_plus_one", WRAP_ADD(vmax, v1));
    print_case("sub_min_minus_one", WRAP_SUB(vmin, v1));
    print_case("mul_max_times_two", WRAP_MUL(vmax, v2));
    print_case("neg_min", WRAP_NEG(vmin));
    print_case("add_min_plus_min", WRAP_ADD(vmin, vmin));
    print_case("sub_max_minus_min", WRAP_SUB(vmax, vmin));
    print_case("mul_min_times_min", WRAP_MUL(vmin, vmin));
    print_case("neg_max", WRAP_NEG(vmax));

    print_case("div_zero_guard", guarded_div(v1, v0));
    print_case("div_min_neg_one_guard", guarded_div(vmin, vneg1));
    print_case("mod_zero_guard", guarded_mod(v1, v0));
    print_case("mod_min_neg_one_guard", guarded_mod(vmin, vneg1));
    print_case("div_min_by_two", guarded_div(vmin, v2));
    print_case("div_max_by_neg_one", guarded_div(vmax, vneg1));
    print_case("mod_min_by_two", guarded_mod(vmin, v2));
    print_case("mod_neg7_by_three", guarded_mod(vneg7, v3));

    print_case("shl_one_by_zero", op_shl(v1, v0));
    print_case("shl_one_by_fifteen", op_shl(v1, v15));
    print_case("shl_one_by_thirty_one", op_shl(v1, v31));
    print_case("shl_one_by_thirty_two", op_shl(v1, v32));
    print_case("shl_one_by_thirty_three", op_shl(v1, v33));
    print_case("shl_one_by_neg_one", op_shl(v1, vneg1));
    print_case("shl_neg16_by_two", op_shl(vneg16, v2));
    print_case("shr_arith_neg16_by_two", op_shr_arith(vneg16, v2));
    print_case("shr_arith_min_by_one", op_shr_arith(vmin, v1));
    print_case("shr_arith_max_by_thirty_one", op_shr_arith(vmax, v31));
    print_case("shr_arith_neg1_by_thirty_one", op_shr_arith(vneg1, v31));
    print_case("shr_logical_neg16_by_two", op_shr_logical(vneg16, v2));
    print_case("shr_logical_min_by_thirty_one", op_shr_logical(vmin, v31));
    print_case("shr_logical_neg1_by_thirty_two", op_shr_logical(vneg1, v32));
    print_case("shr_logical_neg1_by_neg_one", op_shr_logical(vneg1, vneg1));
}

#endif /* ARITH_CASES_H */
