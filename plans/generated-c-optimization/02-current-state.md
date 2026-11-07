# Current State and Baseline Study

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Environment

| Item | Value |
|---|---|
| Baseline commit | `0ac87d3e03ece01a0df474f1cd43826df90c0f41` |
| Host | x86_64 Linux, `cc` in the pinned image (Debian GCC 12.2.0) |
| Target C compiler | `m68k-amigaos-gcc` (GCC) 16.2.0b, `-std=c99 -mcrt=nix20 -m68000 -fwrapv` |
| Emulator | `vamos` from the pinned SDK image, `--instr-trace` |
| Harness runtime | Python 3.11.2 (stdlib only) |

The exploratory program is a sieve over `u8[8192]`, repeated 20 times for the
instruction trace and 2,000 times for host timing. It uses a module-level
`const N: i32 = 8192`, a fill literal `[; 1]`, nested `while` loops with
signed `i32` indices, and a sum loop. The committed `benchmarks/sieve` program
keeps the same shape; its baseline is measured by `make bench-base` before the
first optimization and recorded in `05-findings.md`. The numbers below are the
exploratory selection evidence, not the committed acceptance baseline.

## Baseline Measurements

### Emitted C and linked size

| Variant | C bytes | m68k `.text` `-O2` | m68k `.text` `-O0` | Notes |
|---|---:|---:|---:|---|
| Baseline | 489,491 | 101,396 | 152,116 | 8,192 direct fill stores |
| Fill compacted only | 310,413 | 76,768 | 102,976 | `memset(flags, 1, sizeof flags)` replacing the 8,192 stores |
| Const folded only | 489,455 | 101,344 | — | `8192` literal in the loop tests; storage unchanged |
| Const folded + fill compacted | 310,377 | 76,712 | — | projected head shape |

The fill alone accounts for 179,078 emitted-C bytes (36.6%) and 24,628 bytes
of `-O2` `.text` (24.3%), or 49,140 bytes of `-O0` `.text` (32.3%). The
8,192 stores are one source statement (`let flags: u8[N] = [; 1];`), which
clears the 1,000-statement trigger in R3.2.

### m68k dynamic instruction count

`vamos --instr-trace <binary> 2> trace.log; grep -c 'instr:' trace.log`,
20 sieve repetitions, m68k `-O2`, repeated twice for each variant. Counts were
identical across repetitions.

| Variant | Instructions | Delta vs baseline |
|---|---:|---:|
| Baseline | 3,044,035 | — |
| Const folded, guards retained | 2,165,514 | −28.9% |
| Const folded, three upper guards removed by hand | 2,436,214 | −20.0% |
| Const folded + fill compacted | 2,083,614 | −31.6% |

The third row is the important reject. Removing the three hot bounds checks
made the host build faster but made the m68k build slower than constant
folding alone. The likely cause is a register-allocation and code-layout
change on the 68000. The plan therefore keeps the guards and lets the C
compiler eliminate them where the bound is literal.

### Host wall time

Best and median of 30 process runs inside the pinned image, 2,000 sieve
repetitions. Host time is recorded for context; it is advisory and not a hard
gate (AR-2). The earlier development-host GCC 15.2 values are superseded by
these container values.

| Variant | `-O0` best / median | `-O2` best / median |
|---|---:|---:|
| Baseline | 127.3 / 132.1 ms | 29.6 / 31.0 ms |
| Const folded | 124.3 / 130.8 ms | 21.0 / 22.0 ms |
| Checks removed by hand | 121.1 / 128.8 ms | 31.7 / 35.0 ms |

Constant folding is a clear `-O2` win (about 29% on the median). The manual
check removal is slower than the baseline on this compiler, so it is rejected
outright. The debug profile is not gated.

## What the Baseline Selects

| Change | Trigger | Evidence |
|---|---|---|
| R4 scalar `const` folding | `sieve` spends 28.9% of its m68k instructions on the run-time constant read and the missing constant propagation | 3,044,035 → 2,165,514; host `-O2` 31.0 → 22.0 ms median |
| R5 fill compaction | one source statement emits 8,192 statements; 24.3% of `.text` | 489,491 → 310,413 C bytes; 101,396 → 76,768 `.text` `-O2` |
| R6 non-negative slice/string guard | provably dead `< 0` half in a dynamic-extent guard | `s[1]` emits `if ((1 < 0) \|\| ...)`; `usize` and fixed-array constant forms already omit it |
| R7 constant division guard folding | provably dead condition for a constant divisor | emitted `if (2 == 0 \|\| (x == MIN && 2 == -1))`; dead-condition removal under R3.2 |

## What the Baseline Rejects

| Candidate | Why rejected |
|---|---|
| Manual upper-bound elimination or loop-range analysis | measured m68k regression after constant folding (2,165,514 → 2,436,214); the C compiler already removes the check when the bound is a literal |
| `static const` / read-only storage for module constants | the measured win is the read, not the storage; changing storage touches the module-init contract and is a separate capability |
| Removing the `if (sizeof(...)) { }` warning markers | compile-time no-op; no measured runtime or `.text` cost |
| Removing the trailing `continue` in loop bodies | removed by the C compiler at `-O2`; no measured benchmark delta |
| Removing the zero-initialization of locals that are immediately assigned | one store per local; below the 5% threshold and would need a definite-assignment proof |
| Hoisting slice or array lengths into locals | not measured as a cost in the selected benchmarks; the C compiler already keeps a loop-invariant length where the alias rules allow |

## Codebase Facts

- The statement-form fill is owned by
  `compiler/src/codegen/function.rs::emit_array_assign_to` (the
  `for index in prefix..extent` loops). A fill is already classified as pure,
  impure, conditional, struct-literal, and nested-row. A second emitter,
  `compiler/src/codegen/guards.rs::array_assign_expr`, handles the deferred
  expression form with comma-joined stores; R5 explicitly excludes it.
- The bounds condition is built in
  `compiler/src/codegen/guards.rs::bounds_guard_condition` and rendered by
  `compiler/src/codegen/render.rs::bounds_condition`. `bounds_condition`
  already omits the `< 0` half when the index type is unsigned; the remaining
  dead case is a non-negative constant index into a slice or `str`.
- The division guard is emitted by
  `compiler/src/codegen/guards.rs::emit_division_guard`; a literal zero
  divisor is rendered as a trap expression by the value renderer, so no
  separate `if` guard is emitted for it.
- Function-body module `const` reads lower to `ValueKind::Place` in
  `compiler/src/own/lower.rs::lower_value`; module-level initializers use a
  separate path, `compiler/src/codegen/program.rs::lower_global`. The
  constant folder is `compiler/src/consts/mod.rs::ConstFolder`; it folds an
  initializer in the folder's module context, so folding an imported const
  must construct it for the declaring module.
- The module `const` storage and initializer are emitted by
  `compiler/src/codegen/program.rs`; `compiler/src/codegen/init.rs` computes
  the initialization order. The storage is a mutable file-scope `static`
  assigned in `cl_init_<module>`.
- `unsafe &N` on a module `const` is accepted and compiles to
  `&(cl_<module>_N)`; a raw write through the result changes the storage.
  This is why R4 needs the address-taken exclusion.

## Reproduction Commands

```sh
# Emit and build the exploratory sieve on the host.
target/debug/clc --profile host -o build/perf/sieve.c build/perf/sieve.clo
cc -std=c99 -O2 -o build/perf/sieve build/perf/sieve.c

# Emit and build for m68k inside the pinned image.
make -C docker exec CMD="target/debug/clc --profile hosted-amiga -o build/perf/sieve.c build/perf/sieve.clo"
make -C docker exec CMD="m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -o build/perf/sieve build/perf/sieve.c -lamiga -lm"

# Size and instruction count.
make -C docker exec CMD="m68k-amigaos-size build/perf/sieve"
make -C docker exec CMD="vamos --instr-trace build/perf/sieve >/dev/null 2>build/perf/trace.log; grep -c 'instr:' build/perf/trace.log"
```

The committed harness replaces these ad hoc commands with one reproducible
`make bench` run and a JSON result.
