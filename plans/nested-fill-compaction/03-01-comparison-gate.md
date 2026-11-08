# Design: Parameterized Comparison Gate

> **Document**: 03-01-comparison-gate.md
> **Parent**: [Index](00-index.md)

## Problem

`bench.py compare` currently encodes one plan's targets:

```python
if name == "sieve":
    if head_i > base_i * 0.80: ...
    if head_t > base_t * 0.90: ...
    if head_c > base_c * 0.75: ...
...
if not improved:
    failures.append("no benchmark improved by 20% instructions or 10% .text")
```

Two consequences:

1. A flat baseline (`base == head`) fails the sieve targets and the
   "improved" rule even though nothing regressed.
2. A different optimization with a smaller, real win fails the same rules.

The gate must separate two concerns:

- **Invariants**, always enforced: output equality, metric presence,
  structural compatibility, and the >1% no-regression rule.
- **Expectations**, declared per closeout: named benchmarks and metrics that
  must improve by a stated percentage.

## Interface

```
python3 tools/perf/bench.py compare \
    [--require NAME:instructions:PERCENT]... \
    [--require NAME:text:PERCENT]... \
    BASE.json HEAD.json
```

- `NAME` must be a benchmark present in both documents. Parse from the right
  (`rsplitn(3, ':')`) so a benchmark directory name may contain a colon; the
  last field is `PERCENT`, the middle is `METRIC`, and the rest is `NAME`.
- `METRIC` is `instructions` (maps to `instructions_net`) or `text` (maps to
  `m68k_release.text`).
- `PERCENT` is a finite number greater than zero. It is the minimum
  improvement: the requirement passes when
  `head <= base * (1 - PERCENT / 100)`.
- A repeatable option; order does not matter. A duplicate `NAME:METRIC` is
  allowed and each entry is checked independently.
- Without `--require`, only the invariants run.

Invalid input fails before the table is printed:

| Input | Diagnostic |
|---|---|
| Unknown name | `unknown benchmark in --require: NAME` |
| Unknown metric | `unknown metric in --require: METRIC (use instructions or text)` |
| Non-numeric or non-finite percent | `invalid percentage in --require: VALUE` |
| Zero or negative percent | `percentage must be positive in --require: VALUE` |
| Wrong field count | `expected NAME:METRIC:PERCENT in --require: VALUE` |

## Comparison Behaviour

The existing invariant block stays as it is:

- output SHA-256 equality per benchmark,
- `instructions_net` present,
- `m68k_release.text` present,
- head `instructions_net` and `.text` not greater than base times 1.01,
- structural fields (`toolchain`, `runs`, `flags`, `profiles`) equal,
- benchmark sets equal.

The hardcoded sieve block and the generic `improved` block are removed.

For each parsed requirement, the comparison records a row:

```
require: matrix instructions  base 10959  head 9908  delta -9.6%  need <= -5.0%  PASS
```

A missed requirement appends one failure:

```
matrix: instructions -9.6% misses the required -5.0%? -> no, -9.6 <= -5.0 passes
matrix: text -1.2% misses the required -3.0%
```

The exact wording is not part of the contract; the exit status, the named
benchmark, the metric, and the percentages are.

## Make Target

```make
bench-compare:
	$(DOCKER_MAKE) exec CMD="python3 tools/perf/bench.py compare $(addprefix --require ,$(REQUIRE)) build/perf/base.json build/perf/head.json"
```

- `REQUIRE` is empty by default, so the target performs the invariants only.
- The initial invocation is
  `make bench-compare REQUIRE="matrix:instructions:5 matrix:text:3"`; Phase 1
  records the final values.
- `$(addprefix --require ,...)` expands the space-separated list into repeated
  flags without nested quotes.

## Unit Tests

`tools/perf/tests/test_bench.py` gains:

| Test | Expected |
|---|---|
| met instructions requirement | exit 0 |
| met text requirement | exit 0 |
| missed instructions requirement | exit 1, named failure |
| missed text requirement | exit 1, named failure |
| flat metrics, no requirement | exit 0 |
| flat metrics, requirement | exit 1 |
| unknown benchmark | `HarnessError`, no table |
| unknown metric | `HarnessError`, no table |
| zero, negative, NaN, infinity | `HarnessError`, no table |
| malformed field count | `HarnessError`, no table |
| duplicate requirement | both checked |
| retired C-byte target case | replaced by an invariant case; a C-byte miss no longer fails |
| existing output/regression/missing/structural cases | unchanged behaviour |
| `parse_args` with two `--require` values | both strings collected in order |
| `parse_args` with a colon in a name | the raw string is preserved; the parser does not split |
| `command_compare` shared validator: unknown name, metric, percent, field count | `HarnessError`, no table |
| captured stdout on a missed requirement | contains the benchmark, metric, observed delta, and required percent |
| `make -n bench-compare REQUIRE="a:instructions:5 b:text:3"` | expands to two `--require` flags |
| `make -n bench-compare` with empty `REQUIRE` | no `--require` flag |

`parse_args` only collects the raw strings; semantic validation lives in one
shared validator called by `command_compare` before the table. The met,
missed, flat, and invalid cases are red on the old comparator: the old code
ignores `require` and fails the generic sieve rule, so each new assertion also
checks the named benchmark, metric, and percentages rather than only the exit
status.

The `compare` helper in the test class passes `require=[]` by default so the
existing cases keep testing the invariants. A second helper passes a
requirement list and returns the captured stdout so the named failure is
asserted, not just the exit code. The Makefile expansion is checked with
`make -n` and a text assertion, not by running the container.

## Compatibility

- The JSON schema is unchanged.
- The emit, build, run, trace, and measurement path is unchanged; this commit
  changes only compare interpretation, tests, the Makefile, and the AGENTS
  row.
- The no-regression thresholds stay at 1%.
- Host time and C bytes stay advisory and are still printed.
- A missing `output_sha256` on both documents is a pre-existing harness
  behaviour; this feature does not change it. Normal runs always write the
  hash.

## Verification

1. `make bench-test` green.
2. A flat comparison with no `REQUIRE` exits 0.
3. A synthetic missed requirement exits 1 with the named metric.
4. Base/head emission over the corpus is byte-identical for this commit.
