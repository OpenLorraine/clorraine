# Findings: Compiler Throughput (PLAN-38)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-13

## 1. Baseline

- Base commit: `9c445375118796e0ca95262340cda7f6b675e2bb`
  (`main == origin/main`, clean tree).
- Compiler: `clc 1.1.2`, release build in the pinned Docker SDK image.
- Host: 4 vCPU, 15 GiB RAM. The machine is shared, so wall time on small
  inputs is advisory; CPU time is the stable small-input signal.

The base has no throughput harness. The numbers below come from the new
harness with the base binary, both runs inside the pinned container:

```sh
make -C docker exec CMD="cargo build --release --bin clc"
make -C docker exec CMD="python3 tools/perf/compile_time.py run \
  --clc build/tmp/throughput/base-clc --compiler-git 9c44537 \
  --label compile-base --out build/perf/compile-base.json --runs 5"
make bench-compile
python3 tools/perf/compile_time.py compare \
  --metric median_cpu_ns build/perf/compile-base.json build/perf/compile-head.json
```

### 1.1 Absolute baseline

| Workload | Profile | Wall median | CPU median |
|---|---|---:|---:|
| Spec corpus, 130 files | host | 437.8 ms total | 365.9 ms total |
| `calibration` | host | 5.16 ms | 4.20 ms |
| `local-table` | host | 6.04 ms | 5.16 ms |
| `lookup` | host | 11.13 ms | 10.46 ms |
| `matrix` | host | 7.45 ms | 6.12 ms |
| `moves` | host | 6.53 ms | 5.76 ms |
| `sieve` | host | 10.88 ms | 10.05 ms |
| `strings` | host | 6.02 ms | 5.28 ms |
| `boing` | hosted-amiga | 21.44 ms | 20.50 ms |
| `exec-demo` | hosted-amiga | 25.96 ms | 24.93 ms |
| `gui` | hosted-amiga | 26.78 ms | 26.02 ms |
| `imageviewer` | hosted-amiga | 53.20 ms | 51.55 ms |
| `services` | hosted-amiga | 9.10 ms | 8.00 ms |
| `workbench` | hosted-amiga | 20.09 ms | 18.83 ms |
| `functions-1000` | host | 51.66 ms | 50.14 ms |
| `functions-2000` | host | 122.07 ms | 120.09 ms |
| `functions-4000` | host | 376.40 ms | 373.85 ms |
| `functions-8000` | host | 1,326.74 ms | 1,309.52 ms |
| `calls-4000` | host | 122.19 ms | 121.24 ms |
| `statements-4000` | host | 53.55 ms | 51.87 ms |
| `locals-4000` | host | 320.84 ms | 316.45 ms |
| `mixed-4000` | host | 773.36 ms | 761.90 ms |

The corpus splits into 114 compile-fail files (106 expected failures plus 8
warning-only successes), 7 corpus programs (4 host successes plus 3
profile-surface rejections), 6 run fixtures (all success), and 3 staged files
(1 success plus 2 expected failures). That is 19 exit-0 and 111 nonzero runs.

## 2. Hot Passes

A temporary instrumented build (not committed) timed the front-end stages.
The stage table excludes process startup, stdlib file reads, output writing,
and the instrumentation itself, so its sum is below the harness wall time.

| Stage | functions-8,000 | mixed-4,000 |
|---|---:|---:|
| Parse | 26.4 ms | 22.1 ms |
| Load | 11.2 ms | 14.9 ms |
| Resolve | 2.5 ms | 2.6 ms |
| Sema | 19.1 ms | 18.5 ms |
| Ownership | 110.2 ms | 346.1 ms |
| Codegen | **980.1 ms** | **305.6 ms** |
| **Stage sum** | **1,149.5 ms** | **709.8 ms** |

The function-farm wall median is 1,326.7 ms; the remaining ~180 ms is process
startup, file reads, output writing, and instrumentation. Codegen is 74% of
the stage sum and 67% of the wall total. It grows quadratically: the
4,000→8,000 doubling multiplies codegen time by 3.2.

The emitter performed two linear scans of every lowered body, once per
prototype and once per emitted declaration. The ownership pass is the largest
remaining stage on the mixed shape, but it is close to linear and stays below
the fixed codegen cost, so it is recorded rather than changed.

## 3. Implementation

- `compiler/src/codegen.rs` adds `body_index(functions)`, a
  `BTreeMap<module, BTreeMap<name, &ir::Function>>` built once in
  `Emitter::collect`. The first record for a `(module, name)` key wins, which
  preserves the old `Iterator::find` semantics for a speculatively lowered
  duplicate.
- `emit_function` and `param_c_names` now resolve through the index instead of
  scanning `ownership.functions`.
- `tools/perf/compile_time.py` adds the measurement harness, generated
  workloads, JSON documents, and the `compare` command. Records carry wall
  time, CPU time, peak RSS, and exit codes.
- `make bench-compile` builds the release `clc` and writes
  `build/perf/compile-head.json`.
- `AGENTS.md` and `benchmarks/README.md` document the target and its separation
  from the scored runtime gate.

No language, grammar, ABI, stdlib, runtime, driver-flag, or specification file
changed. No new dependency or script framework was added.

## 4. Result

Head stage attribution:

| Stage | functions-8,000 | mixed-4,000 |
|---|---:|---:|
| Parse | 26.4 ms | 21.8 ms |
| Load | 11.2 ms | 14.8 ms |
| Resolve | 2.5 ms | 2.5 ms |
| Sema | 19.1 ms | 20.2 ms |
| Ownership | 110.2 ms | 367.5 ms |
| Codegen | **123.9 ms** | **103.0 ms** |

Codegen falls 87% on the function farm and 66% on the mixed shape. Index
construction is `O(F log F)` and each lookup is `O(log F)`. The measured
shapes have no generic instances or lifted lambdas, so their remaining codegen
time is near-linear in the number of bodies. Residual generic-instance and
lifted-emission lookups still scan the function table; they were not part of
the measured bottleneck.

### 4.1 Stable comparison

CPU time removes most shared-host scheduling noise. Base/head from the two
documents above:

| Workload | Base CPU | Head CPU | Delta |
|---|---:|---:|---:|
| Spec corpus (130 files) | 365.9 ms | 363.8 ms | −0.6% |
| `calibration` | 4.20 ms | 3.99 ms | −5.2% |
| `local-table` | 5.16 ms | 4.97 ms | −3.6% |
| `lookup` | 10.46 ms | 10.55 ms | +0.9% |
| `matrix` | 6.12 ms | 5.42 ms | −11.5% |
| `moves` | 5.76 ms | 5.43 ms | −5.7% |
| `sieve` | 10.05 ms | 10.37 ms | +3.2% |
| `strings` | 5.28 ms | 5.33 ms | +1.1% |
| `boing` | 20.50 ms | 20.90 ms | +1.9% |
| `exec-demo` | 24.93 ms | 23.91 ms | −4.1% |
| `gui` | 26.02 ms | 25.38 ms | −2.5% |
| `imageviewer` | 51.55 ms | 52.31 ms | +1.5% |
| `services` | 8.00 ms | 8.50 ms | +6.2% |
| `workbench` | 18.83 ms | 20.36 ms | +8.1% |
| `functions-1000` | 50.14 ms | 40.16 ms | −19.9% |
| `functions-2000` | 120.09 ms | 79.90 ms | −33.5% |
| `functions-4000` | 373.85 ms | 156.81 ms | −58.1% |
| `functions-8000` | 1,309.52 ms | 317.11 ms | **−75.8%** |
| `calls-4000` | 121.24 ms | 122.21 ms | +0.8% |
| `statements-4000` | 51.87 ms | 53.75 ms | +3.6% |
| `locals-4000` | 316.45 ms | 319.85 ms | +1.1% |
| `mixed-4000` | 761.90 ms | 549.02 ms | −27.9% |

The benchmark and real-example sets are inside the noise band; no small
workload shows a repeatable regression. The corpus also varies with host load
across document runs, so a pinned true-alternation CPU check was used: 423.1 ms
base → 418.8 ms head (−1.0%). The generated function shapes cross the changed
path and improve monotonically with function count.

### 4.2 Wall time

The user-visible metric on the large generated shapes:

| Workload | Base wall | Head wall | Delta |
|---|---:|---:|---:|
| `functions-1000` | 51.66 ms | 41.48 ms | −19.7% |
| `functions-2000` | 122.07 ms | 81.05 ms | −33.6% |
| `functions-4000` | 376.40 ms | 157.63 ms | −58.1% |
| `functions-8000` | 1,326.74 ms | 318.07 ms | **−76.0%** |
| `mixed-4000` | 773.36 ms | 555.22 ms | −28.2% |

A true-alternation check pinned to one CPU confirmed the same picture
(`/usr/bin/taskset -c 0`, base/head order alternating, temporary scripts under
`build/tmp/throughput`): `functions-1000` −22.8%, `functions-2000` −46.3%,
`functions-4000` −56.4%, `functions-8000` −83.1% by minimum; `mixed-4000`
−30.6% by minimum; `calls-4000` −3.5%, `statements-4000` +0.1%, and
`locals-4000` +2.2%, all inside noise.

### 4.3 End-to-end driver

`clorraine build --release` includes the C compiler, so the front-end win is
diluted. Pinned true-alternation pairs:

| Program | Base | Head | Delta |
|---|---:|---:|---:|
| `lookup` benchmark | 266.1 ms | 260.0 ms | −2.3% |
| `functions-4000` | 785.3 ms | 530.7 ms | −32.4% |
| `functions-8000` | 2,043.5 ms | 898.6 ms | **−56.0%** |
| `mixed-4000` | 2,049.1 ms | 1,759.7 ms | −14.1% |

### 4.4 Peak memory

`ru_maxrss` includes a fork-inheritance floor. The head document records
`rss_floor_kb` = 17,936, and the corpus maximum sits near that floor, so small
compiles are not informative. The large generated shapes exceed it:

| Workload | Base RSS | Head RSS | Delta |
|---|---:|---:|---:|
| `functions-4000` | 43,348 KB | 43,808 KB | +1.1% |
| `functions-8000` | 80,760 KB | 81,532 KB | +1.0% |
| `mixed-4000` | 66,112 KB | 66,524 KB | +0.6% |

The index stores references and cloned module/name keys; its memory cost is
about 1% at the largest measured shape, inside run-to-run noise on a shared
host. The 8,000-function compile peaks at roughly 64 MB above the fork floor.

## 5. Emission and Correctness

The independent emission audit built base and head `clc` and compared every
committed source across the host, hosted-amiga, hosted-amiga-13, and
freestanding profiles:

- 5,311 runs compared; 3,205 emitted C files compared byte-for-byte; zero
  differences in exit code, stdout, stderr, C presence, or C bytes.
- 241 emitted units compiled with `-Werror` on the strict legs; zero
  failures. The only link failure was the known freestanding float case with
  no Amiga math bases, identical on base and head.
- The `bench-shape` gcc, nix13, vbcc, and freestanding legs were warning-free;
  `shapes.c` and `shapes-free.c` were byte-identical.
- The scored `make bench-compare` rows are all `+0.0%` with equal output
  hashes.

The audit also reproduced one pre-existing miscompile on the base: a user
function legally named `cl_lambda_<site>` in the same module as a lifted lambda
can reuse the lifted body because both old and new declaration lookups key on
module and name only. The fix is out of scope for a throughput change; it was
recorded as issue #32 with a minimal reproduction and fix direction. It is now
fixed by excluding lifted bodies from every declaration-body lookup (DEF-558).

## 6. Test Counts

| Suite | Result |
|---|---|
| `cargo test -p clc --lib` | 461 passed, 0 failed (one new body-index case) |
| `cargo test -p clc --test clorraine_driver_spec_test` | 22 passed, 0 failed |
| `make bench-test` | 101 passed, 0 failed (24 new harness cases) |
| `cargo test --workspace` | 148 suites, all green |

## 7. Reviews

| Review | Verdict |
|---|---|
| Correctness reviewer | PASS with four minor findings; no critical or major findings. All four were fixed before closeout. |
| Emission-integrity auditor | Zero attributable differences across 5,311 runs and 3,205 C files; no findings attributable to the change. One pre-existing miscompile recorded above. |
| Memory-safety auditor | No leaks, use-after-free, double-free, or invalid free; emitted C byte-identical to base; no findings. |

The four correctness minors were:

1. `compare` could turn a failed-fast head into an apparent speedup. It now
   requires matching toolchains, profiles, every workload's exit-code set, and
   the corpus file/success/failure split before printing a table.
2. The corpus failure description was imprecise. The findings now state the
   exact compile-fail, corpus, run, and staged split.
3. “Linear” overstated the result. The findings and code comment now say
   `O(F log F)` index construction and scope the near-linear claim to the
   measured shapes.
4. The exact measurement commands and the stage-table denominator were
   missing. Both are now recorded.

## 8. Acceptance

| Gate | Result |
|---|---|
| `make check` | exit 0; 148 workspace suites; `cargo test -p clc --lib` 461 passed; driver spec 22 passed; `make bench-test` 101 passed; spec check, setup check, bench-test, fuzz-test, and fuzz-smoke green |
| `make asan` | 38 suites, green |
| `make ubsan` | 12 suites, green |
| `make bench-shape` | host GCC, nix13 GCC, vbcc, and freestanding builds warning-free |
| `make bench-base COMMIT=9c44537` + `make bench` + `make bench-compare` | every benchmark row `+0.0%`; outputs equal; `compare: passed` |
| `make corpus-amiga` | all portable programs match on hosted-amiga; vbcc build-only legs warning-free |
| `make differential` | 3/3 outputs match |
| `make verify` | `verify: all legs passed` |
| Emission audit | 5,311 runs, 3,205 C files, zero differences |
| Default vectors and scored line | unchanged; no `spec/`, runtime, stdlib, or driver-flag change |
