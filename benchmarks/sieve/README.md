# Sieve

Ten passes of a sieve over `u8[8192]`. The fill literal `[; 1]` is one source
statement, the loop bound is a module `const`, and the inner loops exercise
bounds-checked indexing. It is the fill-size and const-fold benchmark.
