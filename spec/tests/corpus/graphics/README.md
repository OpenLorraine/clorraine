# Corpus: graphics

`program.clo` opens a 320x256x4 custom screen, sets four colors, fills eight
color bars, draws a diagonal line, reports success, and holds the screen briefly
before it drops. It is the acceptance target for the screen lifecycle, the
drawing calls, and the owned-handle cleanup on real AmigaOS.

* Module: `tests.corpus.Graphics`
* Entry: `func main(): i32` (M8, MOD-28)
* Profile: `hosted-amiga` only; under `--profile host` the program must produce
  E11200 because `std.amiga.intuition`, `std.amiga.graphics`, and
  `std.amiga.dos` are outside the host profile surface
* Expected output: `expected-output.txt` — byte-exact, final newline
* Clean-compile contract: zero diagnostics, errors and warnings alike

## Construct coverage

| Construct | Authorizing rules |
|---|---|
| `module tests.corpus.Graphics;` first | MOD-1, MOD-2 (Ch 10 §2) |
| Imports from `std.amiga.intuition`, `std.amiga.graphics`, `std.text`, `std.io`, `std.amiga.dos` | MOD-9, MOD-11, MOD-12, MOD-17 (Ch 10 §4) |
| `let` bindings and assignment | V1 (Ch 03 §2); STMT-3 (Ch 05 §3) |
| Nullable acquisition `Screen?` and narrowing | TY-5, TY-6, TY-7 (Ch 02 §5); T8; MR-2, MR-3 (Ch 11 §2); E10010 |
| `toCString(stringOf(title))` for the `cstring` title | T5 (Ch 08 §7); helper table appendix §9 |
| `while` loop over the bar count | Ch 05 §7 (B3) |
| `u8(i % 3 + 1)` explicit conversion from `i32` | TY-11 (Ch 02 §9); B8 cast form (Ch 04 §14) |
| Borrowing the owned screen into the drawing calls (`borrow screen`) | V5 modes (Ch 03 §2); O12 (Ch 16 §3) |
| Owned `Screen` handle released exactly once by the generated drop | O2, O7 (Ch 03 §3, §5); OW-11..OW-13 |
| Screen lifecycle and drawing wrappers | appendix-amiga.md §1; the `std.amiga.intuition` and `std.amiga.graphics` sections of appendix-stdlib.md |
| `delayTicks(1500)` FFI-backed wrapper call | X6 (Ch 12 §1); appendix §7 (Amiga) |

## Warning audit

| Warning | Why the program is clean |
|---|---|
| W10000 unused local | `title`, `screen`, and `i` are all read after assignment |
| W10001 lend of a duplicable value | The only lent value is the move-only `Screen` handle, read-only into the drawing calls; no writable lend of a duplicable value |
| W10002 switch fallthrough | No `switch` in this program |
| W10003 unreachable code | Every statement is reachable; the failure branch returns a nonzero code |
| W10004 large stack frame | Scalars and one handle only |
| W10005 user contract table | No `ffi` block and no `--contracts` use (application code stays above the boundary, X6) |
| W10006 unused import | Every imported name is used |

## Deliberately not used

No floats, lambdas, `comptime`, `embed`, `unsafe`, `raw`, `volatile`, user `ffi`
blocks, generic declarations, callable bounds, `defer`, partial moves, nested
templates, or library builds. The program stays on the custom-screen surface:
no windows, text, bitmaps, sprites, or multiple screens.

## Friction log

Points where writing the program against the frozen specification felt wrong or
ambiguous.

1. **`cstring` from a string literal.** Chapter examples pass a literal directly
   to `openScreen`, but the only implicit string conversions are `static str ->
   str` and `String -> str` (Ch 02 §9); a literal is not convertible to `cstring`
   without a named call. The program therefore spells the explicit path
   `toCString(stringOf(title))`.
2. **The run leg needs a real display.** A `vamos` run cannot initialize the
   screen (it needs an SDL/display backend), so the accepted run leg is FS-UAE
   with `RUN_CAPTURE=1`, matching the interrupt-counter.
3. **Pen range is depth-dependent.** The program uses pens 0-3 on a depth-4
   screen; the module documents the valid range as `0..(1 << depth) - 1`.

## Hand-check record

Line-by-line review against the frozen specification completed on 2026-09-24.
Every line maps to a rule in the coverage table; the friction items above are
recorded for the phase checkpoint. No authorization was found missing and no
program text changed during the pass.
