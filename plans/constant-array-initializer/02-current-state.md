# Current State: Constant Array Initialization (PLAN-25)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Repository State

| Item | Value |
|---|---|
| Base tag | `v1.1.0` commit `c0bd6d05fe2a985874fddcce9cd07d5100b89e2f` (annotated tag object `c0e34568f423a1ff2994dddd6f2f654e50f338d6`) |
| Current head at audit | `a589353` |
| Worktree | clean before the audit |
| Branch | `main`, equal to `origin/main` |
| Prior compiler work between base and head | nested-fill compaction and volatile nested-fill fixes; expected in the cumulative comparison |
| Roadmap before this plan | 568/570 done, 0 open, 2 parked (PLAN-06, PLAN-07) |

## Harness Audit

`tools/perf/bench.py` already records every metric the acceptance needs. No
harness change is required.

| Metric | Location | Field |
|---|---|---|
| Compiler identity | `compiler_identity` (`bench.py:125-143`) | `path`, `sha256`, `git`, `version`, `dirty` |
| Toolchain identity | `toolchain` (`bench.py:115-122`) | `host_cc`, `m68k_gcc`, `vamos`, `python` |
| Emitted C size | `measure` (`bench.py:354-437`) | `c_bytes`, `c_lines`, `host_c_bytes`, `host_c_lines` |
| m68k section sizes | `size_sections` (`bench.py:217-220`) | `m68k_debug` / `m68k_release` `text`, `data`, `bss` |
| Host time | `time_host` (`bench.py:343-350`) | `host_release_ns`, `host_release_median_ns` |
| Guest instructions | `trace_vamos` (`bench.py:284-339`) and calibration subtraction in `command_run` (`:443-478`) | `instructions_raw`, `instructions_net` |
| Output equality | per-run host/m68k equality (`bench.py:387-397`) | not the `output_sha256` field, which hashes the expected file |
| Comparison requirements | `--require NAME:METRIC:PERCENT` (`bench.py` `command_compare`) | `instructions` or `text` |

## Measured Attribution

The current four scored benchmarks are saturated or out of scope after the
nested-fill and volatile-fill work:

| Benchmark | Current finding | Decision |
|---|---|---|
| sieve | 4–5 instructions per loop iteration; the C compiler removes the guards | Saturated |
| matrix | the nested zero fill is one `memset` | Saturated |
| strings | dominated by standard-library and libnix allocation | Out of scope |
| moves | by-value struct copies need an internal ABI/calling-convention change | Out of scope (internal ABI) |

The audit finds one shape with clear headroom: a module-level `let` fixed
array with a complete explicit list of guard-free constant scalar elements.
The current emitter writes the empty state and then one store per element in
`cl_init_<module>`.

### Current emission

`benchmarks/lookup/program.clo` declares:

```clo
let table: i32[4096] = [32768, 32818, 32869, ...];
```

The current C is:

```c
static i32 cl_bench_Lookup_table[4096] = {0};

static void cl_init_bench_Lookup(void)
{
    cl_bench_Lookup_table[0] = 32768;
    cl_bench_Lookup_table[1] = 32818;
    /* ... 4,094 more stores ... */
}
```

On m68k `-O2`, the store run accounts for 24,464 bytes of `.text` between the
current emission and the template form. The current `lookup` measurement is:

| Metric | Current |
|---|---:|
| raw m68k instructions | 17,025 |
| m68k `-O2` `.text` | 117,440 |
| output | `lookup: 134217728` |

### Probe

The probe emits the same C, then replaces the store run with one block-scope
`static const` template and one `memcpy` at the same position in
`cl_init_bench_Lookup`:

```c
static void cl_init_bench_Lookup(void)
{
    static const i32 __cl_ctmpl_0[4096] = {
        32768, 32818, 32869, 32919, 32969, 33019, 33070, 33120,
        /* ... eight values per line ... */
    };
    memcpy(cl_bench_Lookup_table, __cl_ctmpl_0, sizeof(cl_bench_Lookup_table));
}
```

| Metric | Current | Template + `memcpy` | Delta |
|---|---:|---:|---:|
| raw m68k instructions | 17,025 | 12,842 | -24.6% |
| net m68k instructions (minus 3,372 calibration) | 13,653 | 9,470 | -30.6% |
| m68k `-O2` `.text` | 117,440 | 92,976 | -20.8% |
| output | `lookup: 134217728` | `lookup: 134217728` | equal |

The probe writes the template inside the init function, uses the exact emitted
store expressions, keeps the global writable and the `memcpy` at the same
position, and runs both binaries under `vamos --instr-trace`. The absolute
numbers are exploratory; Phase 1 re-runs them with the committed harness.

## Gaps Identified

### Gap 1: Constant array data is emitted as executable stores

**Current behavior:** a module-level array with an explicit constant list is
declared empty and filled by one statement per element.

**Required behavior:** emit one read-only data template and one `memcpy` at the
same initialization point, with no per-element stores.

**Fix required:** a guard-free, effect-free eligibility predicate and a local
data-template emission branch in the module-level initializer path.

### Gap 2: No measured benchmark for explicit constant tables

**Current behavior:** the four scored benchmarks exercise fills, nested arrays,
strings, and struct copies, but not a large explicit constant table.

**Required behavior:** a deterministic `lookup` benchmark that loads a real
sine table once and scans it once with a value-sensitive checksum.

**Fix required:** the committed benchmark and its expected output; no harness
change.

## Dependencies

### Internal dependencies

- `constant_integer` (`compiler/src/codegen/expr.rs:2036`) for width-correct
  constant values.
- `store_text` (`compiler/src/codegen/function.rs:3984`) for the exact typed
  element expressions.
- `declarator`/`TypeTable` for the C array declarator and element sizes.
- `global_init_order` (`compiler/src/codegen/init.rs:69`) for the init position.
- `GlobalInfo.volatile` (`compiler/src/codegen.rs:49`) for the volatile
  exclusion.
- `Profile` (`compiler/src/profile.rs:35`) for the freestanding exclusion.
- Existing `memcpy` use in the emitted translation unit.

### External dependencies

None. The change adds no crate, build script, external tool, or harness
feature.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A short list is eligible and the template changes the tail policy | Medium | High | Require `elements.len() == extent`; exclude short/empty lists; add behavior and emission cases; record the extent-sized template zero-fill versus the current tail state |
| A guarded expression is folded and a trap is removed | Medium | High | Guard-free structural predicate; no Binary/float/conditional/materialize/call/place; exclusion and trap cases |
| A template name shadows a global or init local | Low | High | Per-init-function `__cl_ctmpl_<n>` counter, reserved prefix, debug assertion; collision tests |
| The global becomes read-only or changes address/order | Low | High | Keep the writable target and the same `memcpy` position; target/position pins |
| The 4,096-value line exceeds C99’s line limit | Medium | Medium | Eight values per line; shape fixture exercises the wrap |
| Freestanding multi-byte tables use the byte-wise shim `memcpy` | Medium | Medium | Profile-gate the template to host, hosted-amiga, and hosted-amiga-13; record the freestanding limit |
| The baseline documents are stale | Medium | High | Remove and rebuild; check the `v1.1.0` identity; retain the isolated-base compiler |
| The benchmark is mistaken for a tuned microbenchmark | Low | Medium | Real sine-table workload; committed before the codegen change; benchmark-source diff and hashes |
| The template increases total data size | High | Low | The scored `.text` and instructions improve; record the read-only template beside the writable global as an accepted limit |

## Residual Opportunities

| Opportunity | Evidence | Disposition |
|---|---|---|
| Function-local constant array literals | A separate declaration path; no committed benchmark | Residual |
| Short/empty explicit lists | Needs an extent-sized tail policy; current emitter leaves the zero tail | Residual |
| Fill forms and prefixes | Existing fill compaction covers the common forms | Residual |
| Module-level `const` arrays with explicit lists | Rejected by `E10224`; a checker/language change | Out of scope |
| Freestanding multi-byte template performance | Byte-wise shim `memcpy`; not measured | Out of scope for this cut |
| Linker section garbage collection | A toolchain change; dead code is largely the runtime shim | Out of scope |
| Whole-program dead-function elimination | A call-graph/FFI feature | Out of scope |
