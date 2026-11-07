# Benchmark Harness: Generated-C Optimization

> **Document**: 03-01-benchmark-harness.md
> **Parent**: [Index](00-index.md)

## Overview

The harness turns the ad hoc measurements in [Current State](02-current-state.md)
into one reproducible command. It emits each benchmark with a chosen `clc`,
builds host and m68k artifacts, runs them, records deterministic metrics, and
compares a base result with a head result under the thresholds in R1.5.

The harness is tooling, not a shipped product surface. It uses Python 3's
standard library only, runs inside the pinned SDK image, and writes all
generated files under ignored `build/perf/`.

## Architecture

### Files

| Path | Purpose |
|---|---|
| `tools/perf/bench.py` | Harness: `run`, `compare`, and `test` entry points |
| `tools/perf/tests/test_bench.py` | Standard-library `unittest` cases for the harness logic |
| `benchmarks/README.md` | Suite contract and invocation |
| `benchmarks/<name>/program.clo` | Benchmark source, module `bench.<Name>` |
| `benchmarks/<name>/expected-output.txt` | Exact normalized stdout |
| `benchmarks/<name>/README.md` | Workload shape and what it exercises |
| `benchmarks/calibration/program.clo` | Startup-only calibration program |
| `benchmarks/calibration/expected-output.txt` | Calibration output |
| `benchmarks/calibration/README.md` | Calibration contract |
| `docker/scripts/bench-base.sh` | Builds the base `clc` from a commit and runs the harness |
| `build/perf/<label>/` | Generated C, binaries, stdout, per-run scratch (ignored) |
| `build/perf/<label>.json` | Result document (ignored) |

### CLI contract

```text
bench.py run [--clc PATH] [--compiler-git SHA] [--label NAME] [--out PATH]
             [--runs N] [--no-trace] [--bench NAME]...
bench.py compare BASE.json HEAD.json
bench.py test
```

- `--clc` defaults to `target/debug/clc`.
- `--compiler-git` is the commit identity of the compiler binary; `make bench`
  passes the head commit and `bench-base.sh` passes the resolved base commit.
- `--label` defaults to the short `--compiler-git`; it must match
  `^[A-Za-z0-9._-]+$`, must not be `.` or `..`, and is used only as a path
  component after that validation. `--out` defaults to
  `build/perf/<label>.json`.
- `--runs` defaults to 30; it is recorded in the result and must match in a
  comparison. The debug host binary runs once; the release binary runs
  `--runs` times.
- `--no-trace` skips the m68k instruction count; `compare` rejects a document
  without it.
- `--bench` limits the run to the named benchmark(s), for iteration.
- `--out` and the run directory must resolve inside `build/perf/`; the harness
  refuses any other path. Benchmark names are validated against
  `benchmarks/`; an unknown name is a usage error.
- Every subprocess has a timeout: emit 120 s, host build 120 s, m68k build
  300 s, host run 30 s, `vamos` run 60 s, trace 300 s. A timeout kills the
  process group and fails the run. Subprocesses are spawned with argument
  lists, never a shell. The harness removes only the run directory it created.

### Run sequence

For each scored benchmark and the calibration:

1. Emit host C with `clc --profile host -o <run>/host.c <program.clo>`.
2. Emit m68k C with `clc --profile hosted-amiga -o <run>/amiga.c <program.clo>`.
3. Build host debug and release:
   - `cc -std=c99 -O0 -g -Wall -Wextra -Werror -o host-debug host.c`
   - `cc -std=c99 -O2 -o host-release host.c`
4. Build m68k debug and release with the compile matrix:
   - `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror -o amiga-debug amiga.c -lamiga -lm`
   - `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -o amiga-release amiga.c -lamiga -lm`
5. Run the host release binary `--runs` times, verify its normalized output
   against `expected-output.txt`, and record the best and median wall time.
   Run the debug binary once for its size and timing.
6. Run the m68k release binary under `vamos`, verify its normalized output, and
   record its duration. When tracing is enabled, run it again with
   `vamos --instr-trace`, consume stderr in a counting thread, and record the
   number of lines containing `instr:`.
7. Record `m68k-amigaos-size` output for both m68k binaries.
8. Record emitted-C bytes and lines.

### Normalization

- stdout bytes are decoded as UTF-8 with replacement; `\r\n` and lone `\r`
  become `\n`; one trailing newline is retained and extra trailing blank lines
  are trimmed.
- The normalized bytes are compared to the expected file and hashed with
  SHA-256.
- Benchmark checksums use fixed-width `i32`/`u32` values, so the same expected
  output is valid on the 64-bit host and the 32-bit target.

### Calibration

The calibration program imports `std.io`, prints one fixed line, and exits. It
is built and run with the same compiler, profile, flags, and `--runs` value as
the scored programs. Its m68k instruction count is the fixed startup cost.
`net_instructions = raw_instructions - calibration.instructions`; a negative
result is a failure. Host net time is recorded the same way but is advisory.

### Trace streaming

`vamos --instr-trace` writes one line per executed instruction to stderr. The
harness starts the process with stderr as a pipe, reads it in a background
thread, and counts lines containing `instr:`. It never stores the trace. The
stdout is captured separately and checked. The trace step has its own timeout
and duration record; a timeout kills the process group and fails the run.

## Result Schema

```json
{
  "schema": 1,
  "label": "head",
  "created": "2026-11-07T12:00:00Z",
  "compiler": {"path": "...", "sha256": "...", "git": "...", "version": "..."},
  "harness_git": "...",
  "toolchain": {
    "host_cc": "cc (Debian 12.2.0) ...",
    "m68k_gcc": "m68k-amigaos-gcc (GCC) 16.2.0b ...",
    "vamos": "amitools 0.8.1 ...",
    "python": "3.11.2"
  },
  "runs": 30,
  "profiles": {"host": "host", "m68k": "hosted-amiga"},
  "flags": {
    "host_debug": "-std=c99 -O0 -g -Wall -Wextra -Werror",
    "host_release": "-std=c99 -O2",
    "m68k_debug": "-std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror",
    "m68k_release": "-std=c99 -mcrt=nix20 -m68000 -fwrapv -O2"
  },
  "calibration": {
    "instructions": 0,
    "host_release_net_ns": 0
  },
  "benchmarks": {
    "sieve": {
      "c_bytes": 0,
      "c_lines": 0,
      "host_debug_bytes": 0,
      "host_release_bytes": 0,
      "host_debug_ns": 0,
      "host_release_ns": 0,
      "host_release_median_ns": 0,
      "host_release_net_ns": 0,
      "m68k_debug": {"text": 0, "data": 0, "bss": 0},
      "m68k_release": {"text": 0, "data": 0, "bss": 0},
      "instructions_raw": 0,
      "instructions_net": 0,
      "output_sha256": "...",
      "duration_ms": 0,
      "trace_duration_ms": 0
    }
  }
}
```

Field rules:

- `schema` is `1`; a mismatch fails a comparison.
- `compiler.sha256` is the SHA-256 of the `clc` binary; `compiler.git` is the
  commit passed by `--compiler-git` (the base commit for `bench-base`, the
  head commit for `make bench`); `harness_git` is `git rev-parse HEAD` of the
  running harness.
- `toolchain` strings are the first line of each tool's version output, except
  `vamos`, which has no `--version`: the harness reads
  `importlib.metadata.version("amitools")`. A comparison requires them to
  match, so a toolchain change is never mistaken for a codegen change.
- `flags` are the exact arrays used; a comparison requires them to match.
- `instructions_net` is absent (or `null`) when `--no-trace` is used.
- JSON is written with sorted keys and a trailing newline.

## Compare Rules

The comparator loads both documents and fails on a structural mismatch
(schema, toolchain, runs, flags, benchmark set, missing trace values). For
each benchmark:

| Metric | Rule | Severity |
|---|---|---|
| `output_sha256` | equal | hard fail |
| `instructions_net` | `head <= base * 1.01` | hard fail |
| `m68k_release.text` | `head <= base * 1.01` | hard fail |
| `sieve.instructions_net` | `head <= base * 0.80` | hard fail (R4.8) |
| `sieve.m68k_release.text` | `head <= base * 0.90` | hard fail (R5.7) |
| `sieve.c_bytes` | `head <= base * 0.75` | hard fail (R5.7) |
| `host_release_median_ns` | displayed; no hard threshold | advisory |

The table shows every benchmark and both hard metrics with the signed delta. A
failed row names the threshold and the R requirement it enforces. The
sieve-specific rows are the headline acceptance; the generic 1% rows catch a
regression anywhere else. `c_bytes` is hard-gated only for `sieve`, the
measured fill benchmark; every other benchmark displays it as advisory.

The 1% allowance is a guard against a real regression in a deterministic
metric, not noise padding: repeated instruction counts and sizes are exact.
Host time is advisory because its 30-run spread is 6–10% (AR-2, AR-11).

## Base/Head Workflow

```sh
make bench-base COMMIT=0ac87d3   # base compiler -> build/perf/base.json
make bench                       # head compiler -> build/perf/head.json
make bench-compare               # enforce R1.5
```

`make bench-base` validates the ref in the make target before anything reaches
a shell:

```make
bench-base:
	@test -n "$(COMMIT)" || { echo 'usage: make bench-base COMMIT=<ref>'; exit 2; }
	@sha=$$(git rev-parse --verify --quiet "$(COMMIT)^{commit}") || { \
		echo "bench-base: unknown commit: $(COMMIT)"; exit 2; }
	@COMMIT=$$sha $(DOCKER_MAKE) exec PASSTHROUGH="COMMIT" CMD="docker/scripts/bench-base.sh"
```

`PASSTHROUGH` forwards the canonical 40-hex SHA as an environment variable, so
a ref containing shell metacharacters never becomes shell text. The script:

1. verifies `COMMIT` matches `^[0-9a-f]{40}$`;
2. refuses a symlinked `build/perf/base-worktree`, then runs
   `git worktree remove --force` (ignoring "not a working tree") and
   `git worktree prune`;
3. creates a detached worktree at that commit;
4. builds `clc` with the pinned toolchain;
5. copies the base `clc` to `build/perf/base-clc` for the later emission
   audit;
6. runs `bench.py run --clc <worktree>/target/debug/clc
   --compiler-git $$COMMIT --label base --out build/perf/base.json`;
7. removes the worktree and its target directory on success or failure.

The base worktree uses the current harness and the current committed
benchmarks; only `clc` comes from the base commit. That is the correct
comparison: the source programs and the measurement code are fixed, and the
compiler is the variable. The retained `build/perf/base-clc` is the input to
the R8.5 corpus emission diff.

## Error Handling

| Case | Handling |
|---|---|
| Missing `clc`, `cc`, `m68k-amigaos-gcc`, `vamos`, `m68k-amigaos-size` | Fail with the missing tool and the PATH searched |
| Emit or build non-zero | Fail with the exact argv and captured stderr |
| Output mismatch | Fail with the benchmark, expected hash, observed hash, and a short diff |
| Missing `instr:` lines | Fail; do not record a zero count |
| Trace timeout | Kill the process group and fail with the elapsed limit |
| Size line unparsable | Fail with the raw line |
| `--out` outside `build/perf/` | Usage error |
| Unknown benchmark name | Usage error |
| Base/head structural mismatch | Fail before comparing any metric |
| Host time regression | Display only; never fail |

## Harness Self-Tests

`tools/perf/tests/test_bench.py` uses `unittest` and temporary directories. It
covers:

1. schema validation accepts a complete document and rejects a missing
   required field;
2. delta arithmetic and the 1%/20%/10% rules;
3. each hard-fail case (output, instructions, `.text`);
4. the improvement requirement;
5. a structural mismatch (toolchain, runs, flags, benchmark set);
6. an unparsable size line;
7. a fake compiler that exits non-zero;
8. a fake compiler that emits C for a program whose output differs;
9. two runs of the same fixture produce the same instruction count and output
   hash (using a stub trace file, not the toolchain).

The tests feed recorded or synthetic data into the parsing and comparison
functions. The fake-compiler cases use a stub tool directory on `PATH` whose
`clc` exits non-zero (or emits a program with a different fixed output); a
stub `cc`/`vamos` is not needed because the run stops at the emit failure. The
integration proof is `make bench` itself. Additional cases cover an `--out`
outside `build/perf/`, an unknown `--bench`, a `--label` traversal, a timeout
through a stub process, and `\r\n` normalization. The test module imports
`bench` through `-t tools/perf` (or inserts the parent path explicitly).

## Makefile Integration

```make
bench:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && python3 tools/perf/bench.py run --compiler-git $$(git rev-parse HEAD) --label head --out build/perf/head.json"

bench-fast:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && python3 tools/perf/bench.py run --compiler-git $$(git rev-parse HEAD) --label head-fast --out build/perf/head-fast.json --no-trace"

bench-base:
	@test -n "$(COMMIT)" || { echo 'usage: make bench-base COMMIT=<ref>'; exit 2; }
	@sha=$$(git rev-parse --verify --quiet "$(COMMIT)^{commit}") || { \
		echo "bench-base: unknown commit: $(COMMIT)"; exit 2; }
	@COMMIT=$$sha $(DOCKER_MAKE) exec PASSTHROUGH="COMMIT" CMD="docker/scripts/bench-base.sh"

bench-compare:
	$(DOCKER_MAKE) exec CMD="python3 tools/perf/bench.py compare build/perf/base.json build/perf/head.json"

bench-test:
	$(DOCKER_MAKE) exec CMD="python3 -m unittest discover -s tools/perf/tests -t tools/perf -v"
```

`bench-fast` writes a separate `head-fast.json`; it never overwrites the full
`head.json`. A per-phase acceptance uses the full `make bench` so the trace
metrics exist. `make check` appends `bench-test` after the workspace tests.
The full `make bench` is not part of `make check`: it emits, builds, runs, and
traces the suite, so it belongs to the optimization and release gates, not the
per-commit fast path.
