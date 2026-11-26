/* Equivalent C reference for benchmarks/strings: 2,000 heap strings
 * "token-<i>-<3i>" built with the template sizing rule, measured, first byte
 * read with a check, and freed; the total is printed as 0x-prefixed hex
 * through an owned hex string. */
#include "ref_io.h"

/* `0x`-prefixed uppercase hex with no leading zeros, as an owned block. */
static unsigned char *toHex(uint32_t value, size_t *len)
{
    static const char digits[] = "0123456789ABCDEF";
    unsigned char *out = (unsigned char *)ref_alloc(10 + 1);
    size_t count = 1;
    uint32_t remaining = value;
    size_t index = 0;
    while (remaining >= 16) {
        remaining >>= 4;
        count++;
    }
    out[0] = '0';
    out[1] = 'x';
    while (index < count) {
        size_t shift = 4 * (count - 1 - index);
        out[2 + index] = (unsigned char)digits[(value >> shift) & 0x0f];
        index++;
    }
    *len = 2 + count;
    return out;
}

int main(void)
{
    uint32_t total = 0;
    uint32_t i = 0;
    unsigned char *hex;
    size_t hex_len;
    unsigned char *line;
    size_t n = 0;
    while (i < 2000) {
        unsigned char *s = (unsigned char *)ref_alloc(6 + 10 + 1 + 10 + 1);
        size_t len = 0;
        len += ref_put(s + len, "token-");
        len += ref_format_u32(s + len, i);
        len += ref_put(s + len, "-");
        len += ref_format_u32(s + len, i * 3u);
        REF_CHECK(0, len);
        total = total + (uint32_t)len + (uint32_t)s[0];
        free(s);
        i = i + 1;
    }
    hex = toHex(total, &hex_len);
    line = (unsigned char *)ref_alloc(9 + hex_len + 1);
    n += ref_put(line + n, "strings: ");
    memcpy(line + n, hex, hex_len);
    n += hex_len;
    ref_write_line(line, n);
    free(line);
    free(hex);
    return 0;
}
