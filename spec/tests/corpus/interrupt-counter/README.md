# Corpus: interrupt-counter

`program.clor` registers a VBlank handler that counts ticks into a module-level
`volatile` value, waits for fifty ticks with a bounded delay loop, then removes
the handler and reports the result. It is the acceptance target for the
interrupt dispatcher boundary and shared interrupt state.

* Module: `tests.corpus.InterruptCounter`
* Entry: `func main(): i32` (M8, MOD-28)
* Profile: `hosted-amiga` only; under `--profile host` the program must produce
  E11200 because the dispatcher and `std.amiga.dos` are outside the host
  profile surface
* Expected output: `expected-output.txt` — byte-exact, final newline
* Clean-compile contract: zero diagnostics, errors and warnings alike

## Construct coverage

| Construct | Authorizing rules |
|---|---|
| `module tests.corpus.InterruptCounter;` first | MOD-1, MOD-2 (Ch 10 §2) |
| Imports from `std.amiga.interrupt`, `std.amiga.dos`, `std.io` | MOD-9, MOD-11, MOD-12, MOD-17 (Ch 10 §4) |
| `volatile let ticks: u32 = 0;` at module level | grammar §3.5; T7 (module-level volatile, Ch 05 §3 / Ch 11) |
| Capture-less handler `func onTick(): void` | F3, FN-1 (Ch 06 §1); exact-signature function pointers (Ch 06 §6) |
| Handler writes shared state only | IH-2 handler rules (appendix §7): allocation-free, drop-free, panic-free |
| `addHandler(onTick)` bare identifier | F3, GF13 (Ch 04 §10); appendix §7 |
| `HandlerToken` returned and handed back | EN/struct duplicability (Ch 02 §8); appendix §7 |
| `while (ticks < 50 && i < 1000)` bounded wait | B3 (Ch 05 §7); `&&` short-circuit (Ch 04 §5) |
| `u32` arithmetic and literals | Ch 02 §2, §10; literal typing Ch 04 §13 |
| `if` / `else` result split, `rc` assignment | STMT-1, STMT-2 (Ch 05 §5-6) |
| `delayTicks(1)` FFI-backed wrapper call | X6 (Ch 12 §1); appendix §7 |
| `removeHandler(token)` | appendix §7 |
| Drops: token and scalars, no owned resources | O2 (Ch 03 §5); OW-11 |

## Warning audit

| Warning | Why the program is clean |
|---|---|
| W10000 unused local | `token`, `i`, and `rc` are all read after assignment |
| W10001 lend of a duplicable value | **Open question:** `removeHandler(token)` passes a duplicable struct to an unmarked lend parameter. The golden `W10001_needless_lend.clor` expects W10001 for exactly that shape (`show(p: Point)`), so this call is expected to warn unless the rule is narrowed or the signature changes. See the friction log. `addHandler(onTick)` passes a function value, not a place |
| W10002 switch fallthrough | No `switch` in this program |
| W10003 unreachable code | Every statement is reachable; the failure branch returns a nonzero code |
| W10004 large stack frame | Scalars only |
| W10005 user contract table | No `ffi` block and no `--contracts` use |
| W10006 unused import | Every imported name is used |

## Deliberately not used

No floats, lambdas, `comptime`, `embed`, `unsafe`, `raw`, user `ffi` blocks,
generic declarations, callable bounds, `defer`, partial moves, nested
templates, library builds, `std.amiga.custom`, or `std.amiga.exec`. The handler
is deliberately minimal: no allocation, no drop, no panic, no loan.

## Friction log

1. **W10001 versus `removeHandler(t: HandlerToken)`.** The dispatcher token is a
   duplicable struct and the standard-library signature takes it as an unmarked
   read-only lend. The warning rule as demonstrated by the golden test fires on
   that shape, so a zero-diagnostic corpus and the fixed signature cannot both
   hold. Options: narrow W10001 to explicit `borrow`/`const` markers, change the
   signature to `take`, or accept one warning for this call. Needs a ruling at
   the phase checkpoint.
2. **Function values and W10001.** A function type is duplicable (Ch 02 §8); if
   the warning is implemented literally over duplicable types it could also
   cover `addHandler(onTick)`. Functions are not places, so the program assumes
   the warning targets places only.
3. **Execution leg.** The accepted run leg is FS-UAE with `RUN_CAPTURE=1`; if
   the handler cannot execute there, the documented fallback is a build-only
   leg plus a recorded finding.

## Hand-check record

Line-by-line review against the frozen specification completed on 2026-09-18.
Every line maps to a rule in the coverage table. The W10001 item is the open
question for this program's zero-diagnostic claim and needs a phase checkpoint
ruling; the handler itself is clean (no allocation, no drop, no panic, no
loan). No program text changed during the pass.
