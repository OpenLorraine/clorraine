# Current State: Direct Template Formatting

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Repository State

| Item | Value |
|---|---|
| Base commit | `00eb6bc` (main, shim-only formatter delivered) |
| Base metrics | `strings` 1,489,187 net instructions, 24,020 `.text`, 6,003 allocations |
| Toolchain | pinned SDK container; m68k GCC 16.2.0b, libnix `nix20`, `vamos` 0.8.1 |
| Worktree | clean before the plan commit |

## Current Lowering

`ExprEmitter::template` (`compiler/src/codegen/expr.rs:1616`) turns each
placeholder into `cl_str_part_owned(<converter>)`, so an integer placeholder
calls `cl_text_i32`/`cl_text_u32`, allocates a `String`, and the builder copies
and frees it. The scored loop emits:

```c
s = cl_string_concat((ClStrPart[]){
    { (const u8 *)"token-", 6u, NULL },
    cl_str_part_owned(cl_text_u32((u32)(i))),
    { (const u8 *)"-", 1u, NULL },
    cl_str_part_owned(cl_text_u32((u32)((i * (u32)(3)))))
}, 4u);
```

The builder (`compiler/runtime/shim_host.c:333`, Amiga `:950`) reserves the sum
of the piece lengths, appends each piece through `cl_string_append`, and frees
each owned piece. A `String` placeholder is copied by `cl_string_copy`; a `str`
placeholder is copied by `cl_text_str`. That is three allocations per
iteration (two converters + the result).

The decimal writer and the converters are the shim-only result:
`cl_text_i32`/`cl_text_u32` write directly into a max-width block, and every
explicit `text`/`toDecimal` path uses them. This plan keeps those converters
for explicit conversions and changes only the template lowering.

## Pre-Plan Probe

The probe built the same emission for `hosted-amiga`, replaced the template
call with typed parts and the builder with a direct writer, and measured the
committed benchmark with the shipped flags and `vamos --instr-trace`.

The probe was measured before the shim-only change landed, so it is compared
with the `8f07e40` emission. The current shim-only base is shown separately
for context; the final Phase 4 measurement is the gate.

| Benchmark | 8f07e40 instructions | Codegen probe | Probe delta | `00eb6bc` shim-only |
|---|---:|---:|---:|---:|
| sieve | 1,040,581 | 1,040,065 | −0.05% | 1,040,152 |
| matrix | 9,908 | 9,343 | −5.7% | 9,431 |
| strings | 2,964,051 | 1,132,051 | −61.8% | 1,489,187 |
| moves | 45,205 | 44,589 | −1.4% | 44,680 |
| lookup | 9,470 | 8,755 | −7.5% | 8,849 |

| Benchmark | 8f07e40 `.text` | Codegen probe | Probe delta | `00eb6bc` shim-only |
|---|---:|---:|---:|---:|
| sieve | 23,832 | 23,908 | +0.3% | 23,720 |
| matrix | 23,608 | 23,684 | +0.3% | 23,496 |
| strings | 24,196 | 23,948 | −1.0% | 24,020 |
| moves | 23,632 | 23,720 | +0.4% | 23,520 |
| lookup | 39,904 | 39,976 | +0.2% | 39,792 |

The probe's `.text` values are from the earlier variant of the shim (the
pre-final decimal loop); they are exploratory. Against the current shim-only
base, the probe is roughly +0.8% on the single-template benchmarks, which is
inside the 1% guard but close. The final builder must be measured and, if
necessary, trimmed. The probe's `strings` allocation count was **2,002** (2,000
results + 2 outside the loop), down from 6,003.

## Gaps Identified

### Gap 1: Integer placeholders allocate a temporary `String`

**Current:** `cl_str_part_owned(cl_text_u32(...))` allocates, copies, frees.
**Required:** carry the value in the part and format it into the destination.
**Fix:** the typed part and the direct builder.

### Gap 2: `str` and `String` placeholders copy

**Current:** `cl_text_str`/`cl_string_copy` allocate a copy.
**Required:** borrow the bytes for the full expression.
**Fix:** `cl_str_part_string` and a borrowed byte part.

### Gap 3: The allocation count is not pinned by a test

**Current:** the PLAN-28 suite pins output and the converter emission, not the
template allocation count.
**Required:** a wrapped-allocator test proves one result allocation.
**Fix:** the new spec suite.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Borrowed `String`/`str` outlives its source | Low | High | Full-expression lifetime; ASan/ownership probes |
| A scalar part is read with the wrong union member | Low | High | Kind code and round-trip tests |
| Designated union initializers fail on vbcc | Low | High | `make bench-shape` |
| `.text` grows above 1% | Medium | Medium | Probe shows ≤0.3% on strings; the final harness guards every benchmark |
| The lowered call changes evaluation order | Low | Medium | Same C initializer list; boundary tests with side effects |
