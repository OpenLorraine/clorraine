# Benchmark Suite

These are tooling fixtures, not frozen-specification goldens. Each directory
holds `program.clo`, `expected-output.txt`, and `README.md`. `calibration` is a
startup-only program; the other four are scored.

Run the suite inside the pinned SDK container:

```sh
make bench           # full trace, writes build/perf/head.json
make bench-fast      # no trace, writes build/perf/head-fast.json
make bench-base COMMIT=<recorded>
make bench-compare   # base.json vs head.json
```

The harness is `tools/perf/bench.py`. It emits each program with `clc`, builds
host and m68k artifacts, runs them, and records emitted-C size, section sizes,
host time, and the m68k dynamic instruction count. The benchmark checksums use
fixed-width integers so the same expected output is valid on the 64-bit host
and the 32-bit target.

| Program | Shape |
|---|---|
| `calibration` | startup only; its instruction count is subtracted |
| `sieve` | 8,192-slot `u8` fill, module `const` loop bound, nested bounds checks |
| `matrix` | 32x32 fixed array, row parameter, nested indexing, helper calls |
| `strings` | template-built Strings, `str` views, byte access, formatting |
| `moves` | struct with a fixed-array field, generic identity, one String move |

Changing a workload requires a new committed baseline in
`plans/generated-c-optimization/05-findings.md`.
