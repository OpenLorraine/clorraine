# Corpus: buffer-container

`program.clor` builds an `Array<i32>` of eight elements, walks it through
one-element views to sum and count the even elements, then fills a 16-byte
`Buffer` through its byte view. It is the acceptance target for monomorphized
generics, dot-calls, view arithmetic, and structural drop order.

* Module: `tests.corpus.BufferContainer`
* Entry: `func main(): i32` (M8, MOD-28)
* Profiles: `host` and `hosted-amiga`
* Expected output: `expected-output.txt` — byte-exact, final newline
* Clean-compile contract: zero diagnostics, errors and warnings alike

## Construct coverage

| Construct | Authorizing rules |
|---|---|
| `module tests.corpus.BufferContainer;` first | MOD-1, MOD-2 (Ch 10 §2) |
| Imports from `std.mem`, `std.io` | MOD-9, MOD-11, MOD-12, MOD-17 (Ch 10 §4) |
| `let xs: Array<i32> = arrayNew<i32>();` explicit type argument | G2 (Ch 06 §8); appendix §3 |
| Dot-calls `xs.push(v)`, `xs.len()`, `xs.get(i)` | F8 (Ch 04 §8); appendix §3 |
| `push` takes `T`; `i32` elements | V5 mode `take`; V8 duplicability (Ch 02 §8) |
| `get(i)` one-element view indexed immediately | appendix §3; O11/O12 (Ch 16 §3); E10022/E10031 guard |
| `while` loops over `xs.len()` and `len(bytes)` | B3 (Ch 05 §7); SL-2 (Ch 08 §3) |
| `%` and `==` in the even test | Ch 04 §5 precedence and integer semantics |
| `borrow xs: Array<i32>` helper parameters | V5 (Ch 03 §2); get requires a writable lend, so reads borrow too |
| Nullable `bufferNew(16)` and narrowing | TY-5..TY-7 (Ch 02 §5); MR-2, MR-3 (Ch 11 §2) |
| `bufferBytes(buf)` writable byte view | appendix §2; O12 exclusive lend (Ch 16 §3) |
| Bounds-checked `bytes[i] = u8(i)` writes | T12, IX-1 (Ch 08 §4); E11211 |
| `usize -> u8` explicit narrowing cast | TY-11, TY-14 (Ch 02 §9); B8 `T(expr)` only |
| `bufferLen(buf)` for the final byte count | appendix §2 |
| Templates for every reported value | S1, S5, TP-1..TP-4 (Ch 04 §12, Ch 08 §8) |
| Drop order: `Array` elements first-to-last, then storage | O2 (Ch 03 §5); OW-11 (Ch 11 §2) |
| `Buffer` drops through `FreeBuffer`, null-safely | P5, O7 (Ch 11 §2; Ch 03 §5) |

## Warning audit

| Warning | Why the program is clean |
|---|---|
| W10000 unused local | Every local is consumed by a loop bound, a sum, a count, or the pattern fill |
| W10001 lend of a duplicable value | `Array` and `Buffer` are move-only, and `v` is a scalar copied at the call (Ch 06 §2) |
| W10002 switch fallthrough | No `switch` in this program |
| W10003 unreachable code | All loops terminate through their counters; the trailing `return 0` is reachable |
| W10004 large stack frame | Two handles, a view, and scalars only |
| W10005 user contract table | No `ffi` block and no `--contracts` use |
| W10006 unused import | `Array`, `arrayNew`, `Buffer`, `bufferNew`, `bufferBytes`, `bufferLen`, `println` are used; `slice` is not imported |

## Deliberately not used

No floats, lambdas, `comptime`, `embed`, `unsafe`, `raw`, `volatile`, user `ffi`
blocks, generic declarations of its own, callable bounds, `defer`, partial
moves, nested templates, or library builds. The program uses only the stdlib's
pre-instantiated `Array<i32>` surface.

## Friction log

1. **`get` forces a writable array lend.** `get<T>(borrow xs: Array<T>, i)` is
   declared with a writable lend because it derives a view, so a read-only
   helper cannot call it: `sumOf(xs: Array<i32>)` would be E10041 (a writable
   view from a read-only loan). The corpus therefore takes `borrow xs` even
   though it only reads. A read-only view accessor would remove the surprise.
2. **`bufferBytes` loan wording.** The corpus contract calls the view a shared
   loan while appendix §2 and the codegen spec call the lend exclusive; the
   writable fill relies on the exclusive reading (Ch 16 §3).
3. **Unused required API.** The corpus contract lists `slice` as required for
   this program, but no behavior step slices; importing it would trip W10006.
   It is not imported.
4. **`u8(i)` pattern cast.** The fill uses an explicit narrowing cast from the
   index; this is inside the allowed cast set but is the kind of place a
   reviewer may ask for a documented pattern instead of an index truncation.
