/* Companion for 048-layout-sizeof.clo: the same layouts in plain C99, printed
 * in the same format, so the probe's sizeof/offsetof can be compared with the
 * target C compiler's under identical flags. */
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

struct A { uint8_t a; int32_t b; };
struct B { uint8_t a; uint16_t b; uint8_t c; };
struct C { uint8_t a; };
struct D { int32_t a; uint8_t b; };
struct E { uint8_t a; struct A x; };
struct F { uint8_t a[3]; uint16_t b; };
struct G { uint8_t d; uint32_t v; };
struct H { bool f; uint8_t g; };
struct I { uint8_t a; double d; };
struct J { uint16_t a; uint8_t b; };
struct K { uint8_t a; uint8_t b; uint32_t c; uint8_t d; };

#define Z(x) ((unsigned long)(x))

int main(void)
{
    printf("A %lu %lu\n", Z(sizeof(struct A)), Z(offsetof(struct A, b)));
    printf("B %lu %lu %lu\n", Z(sizeof(struct B)), Z(offsetof(struct B, b)), Z(offsetof(struct B, c)));
    printf("C %lu\n", Z(sizeof(struct C)));
    printf("D %lu %lu\n", Z(sizeof(struct D)), Z(offsetof(struct D, b)));
    printf("E %lu %lu\n", Z(sizeof(struct E)), Z(offsetof(struct E, x)));
    printf("F %lu %lu\n", Z(sizeof(struct F)), Z(offsetof(struct F, b)));
    printf("G %lu %lu\n", Z(sizeof(struct G)), Z(offsetof(struct G, v)));
    printf("H %lu %lu\n", Z(sizeof(struct H)), Z(offsetof(struct H, g)));
    printf("I %lu %lu\n", Z(sizeof(struct I)), Z(offsetof(struct I, d)));
    printf("J %lu %lu\n", Z(sizeof(struct J)), Z(offsetof(struct J, b)));
    printf("K %lu %lu %lu\n", Z(sizeof(struct K)), Z(offsetof(struct K, c)), Z(offsetof(struct K, d)));
    printf("const %lu %lu\n", Z(sizeof(struct A)), Z(sizeof(struct K) + offsetof(struct K, d)));
    return 0;
}
