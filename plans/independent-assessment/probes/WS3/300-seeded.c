/* Seeded defects for the 300-strict-build red check: each line must raise a
 * warning under the strict flags (-Wconversion, -Wsign-conversion, -Wshadow,
 * -Wstrict-prototypes, -pedantic). */
int f();
int f() { return 0; }
unsigned g(int x) { int y = 1; { int y = 2; (void)y; } short s = x; (void)s; return x + (unsigned)y; }
