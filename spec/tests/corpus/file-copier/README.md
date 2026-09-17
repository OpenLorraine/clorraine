# Corpus: file-copier

`program.clor` copies a 30-byte payload from one file to another through a
4096-byte `Buffer` and checks the copy byte for byte. It is the acceptance
target for file IO, buffer views, nullable acquisition, partial IO, and the
cleanup ladder on nested failure paths.

* Module: `tests.corpus.FileCopier`
* Entry: `func main(): i32` (M8, MOD-28)
* Profiles: `host` and `hosted-amiga`
* Expected output: `expected-output.txt` — byte-exact, final newline
* Clean-compile contract: zero diagnostics, errors and warnings alike

## Construct coverage

| Construct | Authorizing rules |
|---|---|
| `module tests.corpus.FileCopier;` first | MOD-1, MOD-2 (Ch 10 §2) |
| Named imports from `std.io`, `std.mem`, `std.text` | MOD-9, MOD-11, MOD-12, MOD-17 (Ch 10 §4) |
| `const PAYLOAD: static str` and path constants | grammar §3.5; T3; constant-context rules in Ch 04 §16 |
| `let` bindings and assignment | V1 (Ch 03 §2); STMT-3 (Ch 05 §3) |
| Helper functions, unmarked and `borrow` parameters | FN-1, FN-2 (Ch 06 §1); V5 modes (Ch 03 §2) |
| `if` / early `return` in a `bool` condition | STMT-1, STMT-2 (Ch 05 §§5-6); F2 (Ch 06 §3) |
| `while` loops over byte counts | Ch 05 §7 (B3) |
| Nullable acquisition `File?` / `Buffer?` and narrowing | TY-5, TY-6, TY-7 (Ch 02 §5); T8; MR-2, MR-3 (Ch 11 §2); E10010 |
| `toCString(stringOf(path))` for `cstring` parameters | T5 (Ch 08 §7); helper table appendix §9 |
| `bytes(payloadString)` byte view of an owned `String` | T2, T5 (Ch 08 §7); appendix §9 |
| `slice(src, lo, hi)` sub-views | appendix §9; SL-1 (Ch 08 §3) |
| `readFile(borrow f, borrow bufferBytes(buf))` reborrow | O12 (Ch 16 §3); EL-5 (Ch 16 §4) |
| `u8[]` views are copies of pointer and length | T9, SL-1 (Ch 08 §3); O9 no local view expressions (Ch 16 §2) |
| Bounds-checked indexing `copy[i]`, `expected[i]` | T12, IX-1 (Ch 08 §4); E11211 |
| `usize(...)` explicit conversion from `i32` | TY-11 (Ch 02 §9); B8 cast form (Ch 04 §14) |
| Template formatting `$"...{len(payload)}..."` | S1, S4, S5, TP-1..TP-4 (Ch 04 §12, Ch 08 §8) |
| Drops: files and buffers released exactly once | O2, O7 (Ch 03 §3, §5); OW-11..OW-13; P5 (Ch 11 §2) |
| Cleanup ladder on every normal exit edge | STMT-12 (Ch 05 §15); Ch 17 §4 |

## Warning audit

| Warning | Why the program is clean |
|---|---|
| W10000 unused local | Every local is read: paths feed the helpers, views feed calls, counters feed output |
| W10001 lend of a duplicable value | Only views and move-only handles are lent; a view lend copies the view, not the storage (Ch 16 §2) |
| W10002 switch fallthrough | No `switch` in this program |
| W10003 unreachable code | Every `return` is the last statement on its path; the trailing `return 0` is reachable |
| W10004 large stack frame | Locals are handles, views, and scalars; the 4096 bytes live on the heap |
| W10005 user contract table | No `ffi` block and no `--contracts` use (application code stays above the boundary, X6) |
| W10006 unused import | Every imported name is used |

## Deliberately not used

No floats, lambdas, `comptime`, `embed`, `unsafe`, `raw`, `volatile`, user `ffi`
blocks, generic declarations, callable bounds, `defer`, partial moves, nested
templates, or library builds. The program avoids `for` loops and user structs so
the exercised surface stays narrow.

## Friction log

Points where writing the program against the frozen specification felt wrong or
ambiguous; the full list is reviewed at the phase checkpoint.

1. **String literal to `cstring`.** The chapter examples pass string literals
   directly (`openFile("RAM:a")` in Ch 02 §5 and Ch 03 §3, `openInto("RAM:data",
   ...)` in Ch 12 §5), but the only implicit conversions are `static str -> str`
   and `String -> str` (Ch 02 §9); a literal is not convertible to `cstring`
   without a named call. The program therefore spells the explicit path
   `toCString(stringOf(path))`. A spec ruling could add a `static str ->
   cstring` widening for literals; until then the explicit form is used.
2. **`toCString` accepted types.** The appendix §9 table says `String` or `str`,
   but a `str` carries no NUL guarantee and the call is documented as
   allocation-free; only the `String` form is sound. The program passes a
   `String`.
3. **`static str` payload cannot be written as bytes.** `writeBytes` takes
   `u8[]`, and no conversion from `str` to `u8[]` exists, so the payload is
   materialized once as a `String` and viewed through `bytes`. A small
   `str` byte-view helper (or allowing `slice` on `str`) would remove the
   allocation.
4. **`bufferBytes` loan wording.** Ch 16 §3 calls the buffer view a writable
   reborrow, while the corpus hand-check notes call it a shared loan; the
   program relies on the writable reading (fill, then `readFile` into it).
