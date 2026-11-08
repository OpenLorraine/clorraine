# Current State and Measurement Audit (PLAN-22)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Repository State

| Item | Value |
|---|---|
| Base commit | `b68dae5` (`docs(plans): record the DX re-review outcomes`) |
| Worktree | clean before the baseline; `git status --porcelain` empty |
| Tags | `v1.0.0`, `v1.1.0` |
| `origin/main..HEAD` | 0 |
| Roadmap before this plan | 565/567 done, 0 open, 2 parked (PLAN-06, PLAN-07) |

The main checkout contains two user artifacts (`examples/hello/hello.clo`
without a trailing newline and an untracked `.vscode/`). They are not part of
this feature. The benchmark work runs in a separate worktree at `b68dae5`.

## Harness Audit

`tools/perf/bench.py` already records every metric the acceptance needs. No
harness measurement change is required for this feature.

| Metric | Location | Field |
|---|---|---|
| Compiler identity | `compiler_identity` (`bench.py:124-144`) | `path`, `sha256`, `git`, `version`, `dirty` |
| Toolchain identity | `toolchain` (`bench.py:114-121`) | `host_cc`, `m68k_gcc`, `vamos`, `python` |
| Emitted C size | `measure` (`bench.py:412-416`) | `c_bytes`, `c_lines`, `host_c_bytes`, `host_c_lines` |
| m68k section sizes | `size_sections` (`bench.py:216-220`, used at `:422-423`) | `m68k_debug` / `m68k_release` `text`, `data`, `bss` |
| Host time | `time_host` (`bench.py:342-350`) | `host_release_ns` (best), `host_release_median_ns` |
| Guest instructions | `trace_vamos` (`bench.py:283-339`) and calibration subtraction (`:462-475`) | `instructions_raw`, `instructions_net` |
| Output equality | `output_sha256` (`bench.py:429`) | normalized expected-output hash |

The comparison requires `instructions_net` and `m68k_release.text`; a missing
value fails. `c_bytes` and host time are printed as advisory, not gated. The
output check compares the two `output_sha256` values; it expects both to be
present in a normal run. The harness compares only matching toolchains, runs,
flags, profiles, and benchmark sets, and enforces the >1% no-regression rule.

## Measured Attribution

The numbers below come from the current compiler at `b68dae5`, emitted for
`hosted-amiga`, linked with `-O2`, and traced with
`vamos --instr-trace`. The attribution used a temporary probe under
`build/perf/audit` (since removed) and did not change any benchmark source.

| Benchmark | Net instructions | Dominant cost | Decision |
|---|---:|---|---|
| sieve | 1,040,581 | Two loops hold 99% of the count. m68k GCC already removes every emitted bounds guard and emits 5 instructions per store iteration and 4 per count iteration. | Saturated. No in-scope change. |
| matrix | 10,959 | `let a: i32[32][32] = [; [; 0]];` emits a 32-slot row temporary, 32 scalar stores, and 32 `memcpy` calls. The inlined fill is about 4 KB of `.text`. | Selected. |
| strings | 2,972,051 | Each iteration calls `cl_string_concat` and two `cl_text_u32` allocations. The cost is in stdlib and libnix allocation and copy. | Out of scope (stdlib/runtime). |
| moves | 45,205 | By-value struct copies through `makeBox`, `id<Box>`, and `sumBox`. | Residual. Elision needs an internal ABI change. |

### Sieve evidence

The emitted inner store loop at `-O2` is:

```asm
L226:
    clr.b (a2)
    add.l d0,d1
    add.l d0,a2
    cmp.l #8191,d1
    jle L226
```

No bounds guard and no trap test remain: GCC proved the loop range. The count
loop is strength-reduced to a `dbra` loop. The trace buckets confirm the shape:
of 1,043,953 raw instructions, the inner store loop holds 137,340 iterations
across PCs `002f50`, `002f52`, `002f54`, `002f56`, and `002f5c`, and the count
loop holds 81,920 iterations across `002f74`, `002f76`, and `002f7a` plus
71,640 branch-taken hits at `002f7c`. Net of the 3,372-instruction calibration
this is 1,040,581. There is no emitted-shape win left in this benchmark.

### Matrix probe

The probe changed only the emitted C of `matrix`: it replaced the row
temporary, the 32 row stores, and the 32 `memcpy` calls with one
`memset(&(a[0]), 0, sizeof(a));`. The benchmark source, expected output, flags,
and trace method were unchanged.

| Metric | Current | One `memset` | Delta |
|---|---:|---:|---:|
| Trace instructions (raw) | 14,331 | 13,280 | -1,051 |
| Net instructions (raw - 3,372 calibration) | 10,959 | 9,908 | -9.6% |
| m68k `-O2` `.text` | 80,712 | 76,676 | -5.0% |
| Emitted C bytes (exploratory) | 315,167 | 312,510 | -2,657 (-0.8%) |
| Program output | `matrix: 523776` | `matrix: 523776` | equal |

The instruction and `.text` deltas reproduce. The **absolute C-byte values are
exploratory**: a later reproduction with the worktree's `clc` measured 313,637
bytes for the current emission and 310,980 bytes for the probe, a difference of
1,530 bytes in both. The delta is the same. Phase 1 re-runs the probe with
`build/perf/base-clc` and records the committed numbers; the requirements gate
uses instructions and `.text`, not C bytes.

The `.text` drop is the expected effect: the current emitter inlines 32 row
copies of 128 bytes each, while one `memset` call replaces them.

### Strings and moves evidence

This part is **code inspection, not a trace bucket**. The strings loop emits,
per iteration, a four-part `cl_string_concat` compound literal, two
`cl_text_u32` allocations, and a `FreeString` of the previous value. The
emitted loop body is small; the 2.97M count is consistent with the stdlib and
libnix allocation and copy work it calls, which this feature may not change.
A per-function bucket is not recorded and is not needed to reject this shape.

The moves loop emits `b = makeBox(i)`, `c = id__Box(b)`, and `sumBox(c)`. The
struct is 36 bytes. The copies are part of the by-value calling convention.
Changing that convention is an ABI change, so the shape is recorded as a
residual opportunity, not implemented here.

## Comparison-Gate Defect

`tools/perf/bench.py:610-626` hardcodes:

- a sieve instruction target of 20%,
- a sieve `.text` target of 10%,
- a sieve C-byte target of 25%,
- a generic rule requiring some benchmark to improve by 20% instructions or
  10% `.text`.

This gate cannot pass a flat baseline (base and head are the same compiler) and
cannot pass a matrix-only 9.6%/5% win. It must be parameterized before the
optimization can be evaluated. The parameterization keeps the no-regression
rule and the output check unchanged.

## Shape in the Emitter

The nested fill path is `emit_array_assign_to` in
`compiler/src/codegen/function.rs` (nested branch at the lines currently
numbered 4138-4173). It always:

1. declares a `__cl_fillrow` temporary,
2. fills the temporary recursively,
3. emits one `memcpy` per covered row.

The one-extent scalar path already compacts byte constant fills to `memset` and
other scalar fills to a loop (`emit_compact_scalar_fill`,
`compiler/src/codegen/function.rs:4284-4321`). The nested path materializes a
row and then calls that path on the row target, so a row of at least 64 slots
can already compact its own fill. Matrix's row is 32 slots, and the outer 32
row copies remain regardless. The optimization is to recognize a uniform
nested zero fill and use one `memset` over all covered rows, which also
supersedes the row-local compaction.

## Residual Opportunities

| Opportunity | Why it is not in this feature |
|---|---|
| Strings interpolation allocation churn | Requires a stdlib or runtime-shim change; out of scope |
| By-value struct copy elision | Requires an internal calling-convention change; ABI risk |
| Sieve loop work | Already at 4-5 instructions per iteration; no emitted-shape slack |
| One-extent non-byte fills below 64 slots | PLAN-14 measured the loop slower than unrolled stores; unchanged |
| Compiler compile-time speed | Separate axis, explicitly out of scope |
