# Corpus: string-parser

`program.clor` splits a fixed input line on spaces, classifies each token as a
number or a word through an enum and a `switch`, sums the numbers with
hand-rolled digit parsing, and formats decimal and padded hex output. It is the
acceptance target for `str` scanning, enum dispatch, template formatting, and
`String` temporaries at statement end.

* Module: `tests.corpus.StringParser`
* Entry: `func main(): i32` (M8, MOD-28)
* Profiles: `host` and `hosted-amiga`
* Expected output: `expected-output.txt` — byte-exact, final newline
* Clean-compile contract: zero diagnostics, errors and warnings alike

## Construct coverage

| Construct | Authorizing rules |
|---|---|
| `module tests.corpus.StringParser;` first | MOD-1, MOD-2 (Ch 10 §2) |
| Imports from `std.fmt`, `std.text`, `std.io` | MOD-9, MOD-11, MOD-12, MOD-17 (Ch 10 §4) |
| `enum TokenKind : u8` with the C value sequence | EN-1, EN-3, EN-5 (Ch 09 §1) |
| Scoped member labels `TokenKind.Number` | EN-2 (Ch 09 §1, §6) |
| `switch` with `case`, `default`, and `break` per body | B5, STMT-9, W10002 (Ch 05 §10) |
| `const INPUT: static str` and `let input: str = INPUT;` | T3, T5 (Ch 08 §7); Ch 02 §9 |
| `str` indexing and `len(input)` | IX-1, SL-2 (Ch 08 §§3-4); T12 |
| Character literals `'0'`, `'9'`, `' '` as `u8` | F14 (Ch 04 §13); grammar §10.6 |
| `while` scans, nested token loops | B3 (Ch 05 §7) |
| Digit parsing `value * 10 + i32(input[j] - '0')` | wrapping arithmetic Ch 02 §10; explicit cast Ch 04 §14 |
| `if` / early return classification | STMT-1, STMT-2 (Ch 05 §§5-6); F2 |
| Template placeholders `{tokens}`, `{sum}`, `{hex}` | S1, S4, S5, TP-1..TP-4 (Ch 04 §12, Ch 08 §8) |
| `toHex(u32(sum))` and `padLeft(..., 8, '0')` | appendix §5; T5 named conversions |
| `let hex: String` from a helper; `String` widens to `str` | T2, T5 (Ch 08 §7) |
| Temporaries drop at the end of their statement | D5, OW-13 (Ch 03 §5); Ch 04 §3 |
| Drops: the `String` result and the second `String` temporary | O2: locals in reverse declaration order (Ch 03 §5) |

## Warning audit

| Warning | Why the program is clean |
|---|---|
| W10000 unused local | Every local is read in the scan, the classifier, or the output |
| W10001 lend of a duplicable value | Only `str` is lent; a view lend copies the view, not the storage (Ch 16 §2) |
| W10002 switch fallthrough | All three case bodies end in `break` |
| W10003 unreachable code | The `default` case is reachable; every `return` is last on its path |
| W10004 large stack frame | Scalars, views, and one `String` handle |
| W10005 user contract table | No `ffi` block and no `--contracts` use |
| W10006 unused import | `toHex`, `padLeft`, `String`, and `println` are all used; `text` and `toDecimal` are not imported |

## Deliberately not used

No floats, lambdas, `comptime`, `embed`, `unsafe`, `raw`, `volatile`, user `ffi`
blocks, generic declarations, callable bounds, `defer`, partial moves, nested
templates, or library builds. The program uses `str` indexing rather than
`u8[]` views and does not slice substrings, so it needs no view helper beyond
`len`.

## Friction log

1. **No `str` slicing.** The tokenizer knows each token's bounds but cannot
   materialize a `str` token view: the appendix §9 `slice` table says "view ->
   same view type", while the plan-local `slice<T>(v: T[], lo, hi)` accepts only
   `T[]`. The program sidesteps this by passing `(start, end)` indices to the
   classifier and parser. A `slice` form for `str` would make the natural
   tokenizer possible.
2. **Unused required APIs.** The corpus contract lists `text` and `toDecimal`
   among the required helpers, but the behavior is fully covered by templates
   and `toHex`; importing them would trip W10006. They are not imported.
3. **Enum ordering.** Classification uses only `switch` and equality-compatible
   labels; ordering an enum is outside the program (E10222, EN-6).
