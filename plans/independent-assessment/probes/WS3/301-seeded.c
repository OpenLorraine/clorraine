/* Seeded defect for the 301-vbcc-scan red check: an incompatible pointer
 * assignment and an implicit pointer-from-integer conversion must make vbcc warn. */
int main(void) { int i = 0; char *p = &i; long *q = 5; return (int)(*p + *q); }
