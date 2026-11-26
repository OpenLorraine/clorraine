extern void use(void *p);
extern long callee(void *b, unsigned long n);
long f(void) { short x[3]; use(x); return callee(x, 2); }
long g(void) { char x[6]; use(x); return 0; }
