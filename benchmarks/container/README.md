# Container

A growable-array workload adapted from
`spec/tests/corpus/buffer-container/program.clo`. One thousand twenty-four
`Point` structs are pushed through the monomorphized `Array<Point>` instance,
then scanned four times through the `get(i)[0]` one-element view API. Each
element crosses the `sumPoints` call boundary by value.

It is a real container workload, not a microbenchmark for one codegen rule.
The program uses the public `std.mem` API and does not depend on how the
compiler lowers generic instances, views, bounds checks, or struct copies.

| Property | Value |
|---|---|
| Elements | 1,024 `Point { x, y }` values |
| Passes | 4 scans after one fill |
| Hot shapes | generic `push`/`get` instances, one-element view indexing, dynamic-length bounds checks, struct copies |
| Expected output | `expected-output.txt` |
