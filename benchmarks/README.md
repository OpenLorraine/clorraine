# Benchmark Suite

These are tooling fixtures, not frozen-specification goldens. Each directory
holds `program.clo`, `expected-output.txt`, and `README.md`. `calibration` is a
startup-only program; the other five are scored.

Run the suite inside the pinned SDK container:

```sh
make bench           # full trace, writes build/perf/head.json
make bench-fast      # no trace, writes build/perf/head-fast.json
make bench-base COMMIT=<recorded>
make bench-compare   # base.json vs head.json
make bench-flags     # same-compiler old-vs-shipped size/runtime audit
```

The harness is `tools/perf/bench.py`. It emits each program with `clc`, builds
host and m68k artifacts with the shipped release flags, runs them, and records
emitted-C size, section sizes, the linked file size (`m68k_release_bytes`),
host time, and the m68k dynamic instruction count. The benchmark checksums use
fixed-width integers so the same expected output is valid on the 64-bit host
and the 32-bit target.

The scored comparison is the regression guard: both the base and head compiler
are measured with the current shipped flags, and the compare gate enforces
output equality plus the 1% net-instruction and `.text` limits. Emitted-C and
file-size changes are recorded but are advisory in that gate; the independent
base/head emitted-C diff is the emission proof. The size attribution comes from
`make bench-flags`, which builds the same `clc` emission with the old, GC-only,
and shipped flag sets and asserts the size/runtime floors.

| Program | Shape |
|---|---|
| `calibration` | startup only; its instruction count is subtracted |
| `sieve` | 8,192-slot `u8` fill, module `const` loop bound, nested bounds checks |
| `matrix` | 32x32 fixed array, row parameter, nested indexing, helper calls |
| `strings` | template-built Strings, `str` views, byte access, formatting |
| `moves` | struct with a fixed-array field, generic identity, one String move |
| `lookup` | 4,096-entry `i32` sine table, one scan, module-level constant table startup |

Changing a workload requires a new committed baseline in the active
optimization plan’s `05-findings.md`. The `lookup` baseline is in
`plans/constant-array-initializer/05-findings.md`; the earlier suite baseline
is in `plans/generated-c-optimization/05-findings.md`.
