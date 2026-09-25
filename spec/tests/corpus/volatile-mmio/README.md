# Corpus: volatile-mmio

`program.clo` imports the per-chip MMIO wrapper `std.amiga.custom`, sets a color
through the offset-based volatile intrinsic, toggles DMA through the
overlay-struct form, and reports success. It is the acceptance target for the
volatile intrinsics and the per-chip wrapper on hosted-amiga: the wrapper holds
the raw register bases and the `unsafe` blocks, so the program itself stays on
the safe surface.

* Module: `tests.corpus.VolatileMmio`
* Entry: `func main(): i32` (M8, MOD-28)
* Profile: `hosted-amiga` only; under `--profile host` the program must produce
  E11200 because `std.amiga.custom` is outside the host profile surface
* Expected output: `expected-output.txt` — byte-exact, final newline
* Clean-compile contract: zero diagnostics, errors and warnings alike

## Construct coverage

| Construct | Authorizing rules |
|---|---|
| `module tests.corpus.VolatileMmio;` first | MOD-1, MOD-2 (Ch 10 §2) |
| Imports from `std.amiga.custom`, `std.io` | MOD-9, MOD-11, MOD-12, MOD-17 (Ch 10 §4) |
| `setColor(0, 0x0F00)` offset-based write through `writeVolatile16` | T7 (Ch 04 §11); appendix-amiga.md §4 |
| `setDma(true)` / `setDma(false)` read-modify-write through the overlay struct | T7, T11 (Ch 04 §11; Ch 02 §7); appendix-amiga.md §4 |
| `const raw` register base inside the wrapper | T7, appendix-amiga.md §4 |
| Safe wrapper call from application code | X6 (Ch 12 §1); appendix-amiga.md §4 |
| `println` for the final report | appendix-stdlib.md §4 |

## Warning audit

| Warning | Why the program is clean |
|---|---|
| W10000 unused local | There are no locals |
| W10001 lend of a duplicable value | No lend of a duplicable value |
| W10002 switch fallthrough | No `switch` in this program |
| W10003 unreachable code | Every statement is reachable; `return 0` is the last |
| W10004 large stack frame | No locals and no aggregates |
| W10005 user contract table | No `ffi` block and no `--contracts` use (X6) |
| W10006 unused import | `setColor`, `setDma`, and `println` are all used |

## Deliberately not used

No floats, lambdas, `comptime`, `embed`, `unsafe`, `raw`, `volatile`, user `ffi`
blocks, generic declarations, callable bounds, `defer`, partial moves, nested
templates, or library builds: the program stays above the wrapper and touches no
raw address or `unsafe` block itself.

## Friction log

1. **The wrapper writes real custom registers.** `setDma` and `setColor` write
   to the custom-chip region at `0x00DFF000`/`0x00DFF180`. Under `vamos` the
   custom-chip region is plain memory, so the writes complete and the program
   reports success; the FS-UAE leg would drive the same registers on real
   hardware. The regression risk is deliberately limited to the register
   write completing, not to any visible hardware effect.
2. **The host rejection is the profile gate.** The only host-side contract is
   E11200 from the profile module gate; there is no host run, because
   `std.amiga.custom` has no host surface.

## Hand-check record

Line-by-line review against the frozen specification completed on 2026-09-25.
Every line maps to a rule in the coverage table. No program text changed during
the pass.
