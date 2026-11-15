# Text Scan

A tokenizer and number parser adapted from
`spec/tests/corpus/string-parser/program.clo`. The same classification and
digit-parsing algorithms run over a longer fixed command line for sixty-four
passes. The scan returns a `Stats` struct by value, so the workload also pays
an ordinary struct return at a non-inlined call boundary.

It is a real text-processing workload, not a microbenchmark for one codegen
rule. The scan does not know how the compiler lowers views, bounds checks, or
struct returns, and the expected output is the parser's own count and sum.

| Property | Value |
|---|---|
| Input | One fixed 183-byte command line |
| Passes | 64 |
| Hot shapes | `str` indexing with dynamic length, bounds checks, enum dispatch, digit parsing, struct return |
| Expected output | `expected-output.txt` |
