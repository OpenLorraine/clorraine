# Current State: Constant-Array Codegen Residuals (PLAN-36)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## 1. Current Rule

PLAN-25 added `emit_constant_array_template` in
`compiler/src/codegen/program.rs`. The only caller is the module initializer
path (`emit_global_init`), so the template is emitted only for a module-level
`let`. The rule requires:

| Condition | Value |
|---|---|
| Extents | one fixed extent |
| Element | `i16`, `i32`, `u16`, `u32`, `usize`, or `isize` |
| Size | at least 16 elements and 64 bytes |
| Value | `ArrayLit` with no `fill`, `elements.len() == extent` |
| Elements | guard-free constant integers |
| Target | non-volatile |
| Profile | not `freestanding` |

A function-local `let t: i32[N] = [...]` is lowered to a hoisted declaration
zeroed at function entry and a `Statement::Assign` whose value is an
`ArrayLit`. The top-level assignment path in
`compiler/src/codegen/function.rs` calls `emit_array_assign` (or the
`emit_fixed_array_assign_to` core when the literal may read the target), which
emits one store per element. No template is emitted.

## 2. Probe Set

All probes use the same shipped release flags as `tools/perf/bench.py` and the
same `i32` sine values. The probe sources are temporary fixtures under
`build/tmp/probes`; the committed `local-table` benchmark uses the same values.
The calibration program is the committed `benchmarks/calibration` source
(2,928 raw instructions).

| Probe | Shape |
|---|---|
| `local-table` | function-local `i32[256]` full list, scanned once |
| `local-1024` | function-local `i32[1024]` full list, scanned once |
| `local-1280` | function-local `i32[1280]` full list, scanned once |
| `module-1024` | module-level `i32[1024]` full list, scanned once |
| `module-1280` | module-level `i32[1280]` full list, scanned once |
| `module-byte` | module-level `u8[256]` full list, scanned once |
| `module-short` | module-level `i32[64]` with a 16-value prefix, scanned once |

Two temporary compiler builds isolate the decision metrics without changing
the head:

- `build/tmp/clc-band` returns the store form for a 2–5 KB 4-byte array, so
  `module-1024`/`module-1280` compare the two forms with the same target
  placement.
- `build/tmp/clc-byte` admits `i8`/`u8`/`bool` to the template rule, so
  `module-byte` compares the two forms at 256 bytes.

## 3. Baseline Measurements

Metrics are m68k `.text` bytes and net dynamic instructions after calibration.

### 3.1 Function-local table (head vs base)

| Probe | Base `.text` | Head `.text` | Δ `.text` | Base net | Head net | Δ net |
|---|---:|---:|---:|---:|---:|---:|
| `local-table` (`i32[256]`) | 25,644 | 24,640 | **−1,004** | 1,074 | 814 | **−260** |
| `local-1024` (`i32[1024]`) | 31,748 | 27,712 | **−4,036** | 4,210 | 3,165 | **−1,045** |
| `local-1280` (`i32[1280]`) | 33,964 | 28,708 | **−5,256** | 4,273 | 2,601 | **−1,672** |

Emitted C also shrinks: `local-table` 321,168 → 317,283 bytes; `local-1024`
339,337 → 323,141; `local-1280` 345,643 → 325,095.

### 3.2 2–5 KB 4-byte band (template vs store, same module target)

| Probe | Store `.text` | Template `.text` | Δ `.text` | Store net | Template net | Δ net |
|---|---:|---:|---:|---:|---:|---:|
| `module-1024` (`i32[1024]`) | 33,744 | 27,700 | **−6,044** | 3,118 | 3,159 | **+41** |
| `module-1280` (`i32[1280]`) | 36,320 | 28,700 | **−7,620** | 3,893 | 2,600 | **−1,293** |

At 1024 the template is a large size win for a +0.4% instruction cost in this
scan workload. The PLAN-25 init-dominated measurement was +63 instructions
(+1.18%); the direction matches. At 1280 the template wins both metrics.

### 3.3 1-byte table (template vs store, `u8[256]`)

| Form | `.text` | Net instructions | C bytes |
|---|---:|---:|---:|
| Stores | 24,200 | 1,111 | 325,694 |
| Template | 23,864 | 1,322 | 316,725 |
| Δ | **−336** | **+211** | −8,969 |

The template saves only 336 bytes of `.text` and costs 211 instructions, so the
store form stays.

### 3.4 Short list (`i32[64]`, 16-value prefix)

The store form measures 23,688 `.text` / 175 net. The eligibility rule rejects
it (`elements.len() != extent`), and no template is emitted.

A semantic probe with the short local list inside a loop shows:

```
iter 0 tail 0
iter 1 tail 99
```

The declaration is hoisted and zeroed once at function entry; the stores cover
only the 16-element prefix, so a loop re-entry leaves the previous iteration's
tail value. The frozen spec does not state a tail policy for a short list
without a fill, so this performance bundle records the observation and keeps
the stores; it does not change the semantics.

## 4. Why the Local Path Is Separate

The module path writes into a file-scope global that the C declaration already
zeroes. A local path must write into a function-local object at the assignment
position, which may be inside a loop or a branch. A block-scope
`static const` template is valid C99 and has static storage, so the template is
initialized once and the `memcpy` runs on every execution of the assignment.
This preserves the current init position and any later mutation.

The existing predicate and data emission are reusable; the missing piece is a
call site that renders the destination as an `ArrayTarget` and copies with its
address and byte count instead of a global symbol.
