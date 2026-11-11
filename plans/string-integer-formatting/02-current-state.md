# Current State and Measurement Audit

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Repository State

| Item | Value |
|---|---|
| Base commit | `8f07e408975d9ad090392712030d2e2761d19437` (`main`, v1.1.2 closeout) |
| Worktree | clean before the baseline (`git status --porcelain` empty) |
| Compiler | `clc 1.1.2`, built with `cargo build --workspace` |
| Toolchain | pinned SDK container; m68k GCC 16.2.0b, libnix `nix20`, `vamos` 0.8.1 |
| Benchmark flags | shipped release flags recorded in `build/perf/base.json` |

## Current Implementation

### The String block

Both `String` profiles use the same layout: a two-word header `{ len, cap }`
followed by the payload and a trailing NUL. The host block is at
`compiler/runtime/shim_host.c:226`; the Amiga block is at
`compiler/runtime/shim_hosted_amiga.c:847`.

`cl_string_new(cap)` allocates one block, sets `len = 0`, `cap = cap`, and
writes the first NUL (`shim_host.c:242`, `shim_hosted_amiga.c:863`).
`cl_string_append` copies bytes and enforces capacity
(`shim_host.c:262`, `shim_hosted_amiga.c:879`).

### The integer converters

```c
/* compiler/runtime/shim_host.c:352 */
void *cl_text_i32(int value)
{
    char buffer[24];
    int written = snprintf(buffer, sizeof buffer, "%d", value);
    if (written < 0) {
        cl_trap("E1", "text formatting failed");
    }
    return cl_string_from(buffer, (unsigned int)written);
}

/* compiler/runtime/shim_host.c:362 */
void *cl_text_u32(unsigned int value)
{
    char buffer[24];
    int written = snprintf(buffer, sizeof buffer, "%u", value);
    if (written < 0) {
        cl_trap("E1", "text formatting failed");
    }
    return cl_string_from(buffer, (unsigned int)written);
}
```

The Amiga bodies are the same shape with `LONG`/`ULONG` and `%ld`/`%lu`
(`shim_hosted_amiga.c:969` and `:979`). `cl_string_from` allocates an exact
block and copies the stack buffer into it (`shim_host.c:292`), so one integer
conversion performs a format, an allocation, and a copy.

The template lowering is unchanged and out of scope. For the scored loop it
emits:

```c
s = cl_string_concat((ClStrPart[]){
    { (const u8 *)"token-", 6u, NULL },
    cl_str_part_owned(cl_text_u32((u32)(i))),
    { (const u8 *)"-", 1u, NULL },
    cl_str_part_owned(cl_text_u32((u32)((i * (u32)(3)))))
}, 4u);
```

Each `cl_text_u32` result owns a separate block; `cl_string_concat` copies the
two small blocks into the result and frees them. That is three allocations
and three frees per iteration.

## Measurement Audit

### Method

The pre-plan audit emitted `benchmarks/strings/program.clo` with the current
`clc` for `hosted-amiga`, built it with the shipped m68k release flags
(`-std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -ffunction-sections
-fdata-sections -Wl,--gc-sections`), and ran it under
`vamos --instr-trace`. Function attribution mapped each traced PC to the
nearest preceding function label from `m68k-amigaos-objdump -d`; the load base
was confirmed from the startup PC.

Allocation counts came from a temporary host build with counters in
`cl_alloc` and `cl_free`; the probe lived under `build/perf/audit` and is not
committed.

### Baseline

| Metric | Value |
|---|---:|
| `strings` raw instructions | 2,966,979 |
| Calibration | 2,928 |
| `strings` net instructions | 2,964,051 |
| `strings` m68k `.text` | 24,196 bytes |
| Allocations per run | 6,003 |
| Allocations per iteration | 3 (two converters + one builder) |
| Allocations outside the loop | 3 (`toHex`, its copy, the final builder) |

### Attribution (2,964,051 net)

| Bucket | Net instructions | Share |
|---|---:|---:|
| `___vfprintf_total_size` | 1,026,591 | 34.6% |
| `_putc` | 338,412 | 11.4% |
| `___ulldivus` | 304,878 | 10.3% |
| `_malloc` | 174,416 | 5.9% |
| `_free` | 174,261 | 5.9% |
| `___udivsi3` | 130,665 | 4.4% |
| `_vsnprintf` | 112,000 | 3.8% |
| `_memcpy` | 60,029 | 2.0% |
| `_snprintf` | 16,000 | 0.5% |
| `_vfprintf` | 12,000 | 0.4% |
| Program body (`_main`) | 384,063 | 13.0% |
| Startup/constructor region | 87,114 | 2.9% |
| Remainder | 143,622 | 4.8% |
| **`snprintf` family** | **1,940,546** | **65.4%** |
| **`malloc` + `free`** | **348,677** | **11.8%** |

The `snprintf` family is 5.5× the allocation cost. The converter's
`%u`/`%d` path drives `vfprintf`, `putc`, and two libgcc division loops; the
allocation path is secondary.

## Probes

All probes used the committed benchmark source, the shipped flags, equal
output, and the same 2,928-instruction calibration subtraction.

### Shim-only: manual digits, stack buffer + copy

Replacing only the two converter bodies with a manual decimal writer and
keeping `cl_string_from`:

| Metric | Base | Probe | Delta |
|---|---:|---:|---:|
| `strings` net instructions | 2,964,051 | 1,512,668 | −49.0% |
| `strings` `.text` | 24,196 | 24,092 | −0.4% |
| Allocations | 6,003 | 6,003 | 0 |

### Shim-only: manual digits, direct into the block (selected)

Allocating the maximum width and writing the digits into the block, then
setting `len` and the NUL:

| Metric | Base | Probe | Delta |
|---|---:|---:|---:|
| `strings` net instructions | 2,964,051 | 1,464,668 | −50.6% |
| `strings` `.text` | 24,196 | 24,080 | −0.5% |
| Allocations | 6,003 | 6,003 | 0 |

All scored benchmarks, selected probe:

| Benchmark | Base instructions | Probe | Delta | Base `.text` | Probe | Delta |
|---|---:|---:|---:|---:|---:|---:|
| sieve | 1,040,581 | 1,040,135 | −0.04% | 23,832 | 23,684 | −0.6% |
| matrix | 9,908 | 9,412 | −5.0% | 23,608 | 23,460 | −0.6% |
| strings | 2,964,051 | 1,464,668 | −50.6% | 24,196 | 24,080 | −0.5% |
| moves | 45,205 | 44,659 | −1.2% | 23,632 | 23,484 | −0.6% |
| lookup | 13,653 | 8,824 | −35.4% | 64,328 | 39,756 | −38.2% |

Every output was equal to `expected-output.txt`. The `lookup` win is the
startup formatting of its final line: the base pays the `vfprintf` setup even
for one integer.

### Codegen probe (out of scope, follow-up evidence)

A probe of the larger design that formats template placeholders directly into
the builder destination measured:

| Metric | Base | Codegen probe | Delta |
|---|---:|---:|---:|
| `strings` net instructions | 2,964,051 | 1,132,051 | −61.8% |
| `strings` `.text` | 24,196 | 23,948 | −1.0% |
| Allocations | 6,003 | 2,002 | −66.6% |

It changes the emitted C for every template, so it exceeds issue #19's
"standard-library and runtime-shim work only" boundary. It is retained as
follow-up evidence, not planned work (AR-01, AR-05, AR-16).

## Gaps Identified

### Gap 1: Integer conversion routes through `snprintf`

**Current behavior:** each `cl_text_i32`/`cl_text_u32` call formats through
the C library and then copies a stack buffer into a fresh block.
**Required behavior:** write the decimal digits directly into the block, with
no formatter call and no copy.
**Fix required:** the shared `cl_format_u32`/`cl_format_i32` helpers and the
two converter bodies in both `String` shims.

### Gap 2: The formatter's boundary behavior is not pinned

**Current behavior:** the existing suite covers `usize` and a small integer,
not `INT_MIN`, `UINT_MAX`, adjacent placeholders, or a formatter-call count.
**Required behavior:** every boundary and template shape is a spec case.
**Fix required:** the new spec suite (`07-testing-strategy.md`).

### Gap 3: The allocation residual is not tracked

**Current behavior:** three allocations per template iteration remain.
**Required behavior:** the residual is measured and recorded, and the codegen
option is a follow-up issue.
**Fix required:** `05-findings.md` and the closeout task.

## Dependencies

### Internal

- `compiler/runtime/shim_host.c` and `shim_hosted_amiga.c` are embedded by
  `compiler/src/profile.rs`; no build-system change is needed.
- `stdlib/std.fmt.clo` forwards to the shims; no `.clo` change.
- `tools/perf/bench.py` already records every metric the acceptance uses.

### External

- The pinned SDK toolchain (m68k GCC, libnix, vbcc) and `vamos`.
- The licensed Amiga media are not needed: the benchmark runs under `vamos`.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A manual formatter diverges on a boundary | Low | High | ST cases for `0`, ±1, ±max, `INT_MIN`, `UINT_MAX`; output hashes |
| Direct write exceeds the block | Low | High | Fixed maximum width; memory-safety review; ASan |
| Host and Amiga shims drift | Low | Medium | Same helper names and bodies; impl parity check |
| The benchmark base drifts from the compiler | Low | Medium | Build base from `8f07e40`; harness records identity |
| `.text` regresses above 1% | Low | Medium | Probe shows −0.5% to −38.2%; harness guards |
| The change is judged out of the issue's scope | Low | High | AR-01 cites the issue boundary; codegen is a follow-up |
