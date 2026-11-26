/* Equivalent C reference for benchmarks/text-scan: the fixed command line is
 * split on spaces 64 times; each token is classified through an enum and a
 * switch, numbers are parsed by hand, and the scan returns a stats struct by
 * value. Every byte access is checked. */
#include "ref_io.h"

enum { WORD = 0, NUMBER = 1 };

typedef struct {
    uint32_t tokens;
    uint32_t numbers;
    uint32_t words;
    int32_t sum;
} Stats;

static const char INPUT[] = "MOV 12 34 ADD 5 7 SUB 90 4 MUL 6 7 DIV 100 5 LOAD 101 202 303 STORE 44 55 JMP 16 JEQ 7 8 PUSH 1000 POP 2 CALL 33 RET 0 NOP 1 2 3 4 5 6 MOV 56 78 ADD 9 10 SUB 11 12 MUL 13 14 DIV 15 16";

/* One checked byte read. */
static uint8_t at(const uint8_t *s, size_t n, size_t i)
{
    REF_CHECK(i, n);
    return s[i];
}

static uint8_t classify(const uint8_t *input, size_t n, size_t start, size_t end)
{
    int allDigits = 1;
    size_t j = start;
    while (j < end) {
        if (at(input, n, j) < '0' || at(input, n, j) > '9') {
            allDigits = 0;
        }
        j = j + 1;
    }
    if (allDigits) {
        return NUMBER;
    }
    return WORD;
}

static int32_t parseNumber(const uint8_t *input, size_t n, size_t start, size_t end)
{
    int32_t value = 0;
    size_t j = start;
    while (j < end) {
        value = value * 10 + (int32_t)(uint8_t)(at(input, n, j) - '0');
        j = j + 1;
    }
    return value;
}

static Stats scanOnce(const uint8_t *input, size_t total)
{
    Stats stats = {0, 0, 0, 0};
    size_t i = 0;
    while (i < total) {
        size_t start;
        while (i < total && at(input, total, i) == ' ') {
            i = i + 1;
        }
        start = i;
        while (i < total && at(input, total, i) != ' ') {
            i = i + 1;
        }
        if (i > start) {
            uint8_t kind = classify(input, total, start, i);
            stats.tokens = stats.tokens + 1;
            switch (kind) {
            case NUMBER:
                stats.numbers = stats.numbers + 1;
                stats.sum = stats.sum + parseNumber(input, total, start, i);
                break;
            case WORD:
                stats.words = stats.words + 1;
                break;
            default:
                break;
            }
        }
    }
    return stats;
}

int main(void)
{
    uint32_t tokens = 0;
    uint32_t numbers = 0;
    uint32_t words = 0;
    int32_t sum = 0;
    int32_t pass = 0;
    unsigned char *line;
    size_t n = 0;
    while (pass < 64) {
        Stats one = scanOnce((const uint8_t *)INPUT, sizeof INPUT - 1);
        tokens = tokens + one.tokens;
        numbers = numbers + one.numbers;
        words = words + one.words;
        sum = sum + one.sum;
        pass = pass + 1;
    }
    line = (unsigned char *)ref_alloc(18 + 10 + 9 + 10 + 7 + 10 + 5 + 11 + 1);
    n += ref_put(line + n, "text-scan: tokens=");
    n += ref_format_u32(line + n, tokens);
    n += ref_put(line + n, " numbers=");
    n += ref_format_u32(line + n, numbers);
    n += ref_put(line + n, " words=");
    n += ref_format_u32(line + n, words);
    n += ref_put(line + n, " sum=");
    n += ref_format_i32(line + n, sum);
    ref_write_line(line, n);
    free(line);
    return 0;
}
