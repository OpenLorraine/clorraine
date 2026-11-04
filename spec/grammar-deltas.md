# Clorraine - Grammar Deltas from Blend65 v4

> **Status:** draft (Phase B anchor)
> **Scope:** every delta between the Clorraine grammar and the Blend65 v4 grammar, with the reason; the grammar freeze checklist resolutions; corpus validation
> **References:** [grammar.ebnf.md](grammar.ebnf.md) (syntax), [decisions.md](decisions.md) (authority), [lineage.md](lineage.md) (Rust semantics pin)

## 1. Purpose and method

Clorraine reuses the structure and process of the Blend65 specification and
replaces everything that existed only because of the 6502 target. This file
lists every surface delta, classifies it, and names the replacement or the
decision behind it. Classes:

| Class | Meaning |
|---|---|
| Rename | Same construct, new spelling |
| Removed (target) | Existed only for the 6502; replaced by something else |
| Lifted | A Blend65 restriction that the C backend removes |
| Carried | Deliberately kept from Blend65 |
| Added | Not present in Blend65 |
| Freeze | A Phase B decision that closes a gap the ledger left open |

## 2. Renames and surface adaptations

| Blend65 v4 | Clorraine | Reason |
|---|---|---|
| `function` | `func` | Ledger spelling; shorter in signatures |
| `fn` (function type) | `func` | One keyword for both uses (F3) |
| `boolean` | `bool` | Ledger spelling |
| `byte`/`sbyte`/`word`/`sword` | `u8`/`i8`/`u16`/`i16` | Fixed widths, plus `u32`/`i32`/`usize`/`isize`/`f32`/`f64` (T1) |
| `comptime function` | `comptime func` | Consistency; syntax frozen, semantics staged (R1) |
| `enum Name { A }` | `enum Name : u8 { A }` | Explicit backing integer type (draft Ch 09 kept) |
| `case 1, 2:` | `case 1:` / `case 2:` | C-exact labels (B5) |
| `... fallthrough;` | (implicit) | C-exact fallthrough (B5) |
| `$FF` hex | `0xFF` only | `$` introduces templates; C-like surface |
| unary `+` (sign prefix) | (not adopted) | Write the operand directly; the C-ish prefix is redundant |
| `let name: T;` at module level | initializer required | No uninitialized globals (M7, freeze GF20) |
| `type` (reserved keyword) | ordinary identifier | Blend65 reserved it without a production; dropped |

## 3. Removed because of the 6502 target

| Blend65 construct | Replacement |
|---|---|
| `place(at, align, noCross, region)` | None in the grammar; profile placement returns later (appendix) |
| `zeropage { ... }` blocks | None: no zero page on m68k |
| `loadable const` | Module `const` covers the use case |
| `interrupt function` | Standard-library dispatcher with a C-side qualified handler (F7) |
| `peek`/`poke`/`peekw`/`pokew` | `readVolatile*`/`writeVolatile*` intrinsics plus `raw`/`unsafe` (T7, T6) |
| `lo`/`hi` | Shifts and masks |
| `bcd_add`/`bcd_sub` | Standard library later |
| `petscii`/`screen_codes`/`atascii`/`internal_codes` | Standard library encoders |
| `sin8`/`cos8`/`sin16`/`cos16` | Math library (floats now exist, T1) |
| `asm_sei`/`asm_cli`/`asm_php`/`asm_plp`/`asm_nop` | OS/stdlib wrappers or `unsafe` raw calls |
| Static Frame Allocation (SFA) | C activation stack (P1) |

## 4. Blend65 restrictions lifted by the C backend

Blend65 enforced these restrictions because of SFA and the 6502. The C backend
removes the reason, and each row names what Clorraine allows instead.

| # | Blend65 restriction | Why it existed | Clorraine now | Ref |
|---|---|---|---|---|
| 1 | No recursion (`E10180`/`E10181`) | Static frames cannot nest | Direct, indirect, mutual, and generic recursion | F7 |
| 2 | Static Frame Allocation for every function | 6502 addressing and memory budget | Ordinary C frames; large locals allowed with a size warning | P1 |
| 3 | No heap or dynamic allocation in the language | No allocator in a 6502 image | Allocator contracts; `String`, `Buffer`, `Array<T>` | F3, T2 |
| 4 | Fixed extents only; any-size parameters special-cased | Compile-time memory budget | Slices `T[]` (pointer + length views); growable `Array<T>` | T9, F3 |
| 5 | No general typed pointers; address-of only as a bounded borrow | Banked 16-bit addressing; proto-provenance model | `raw T`, `&place`, pointer arithmetic, all inside `unsafe` | T6 |
| 6 | Local-address escape rejected (`E10260`) | Static frames made escapes unsound | Safe subset kept (views cannot escape); raw escapes are audited | O11, T6 |
| 7 | 16-bit address space assumptions | 6502 | 32-bit m68k now; `usize`/`isize` keep ports honest | T1 |
| 8 | Byte and word scalars only | 6502 ALU | `i8`-`u32`, `usize`/`isize`, and `f32`/`f64` | T1, F17 |
| 9 | Explicit `fallthrough;` statement required | Readability policy | C-exact implicit fallthrough plus an optional warning | B5 |
| 10 | Special `interrupt function` kind and ABI qualifiers | 6502 IRQ vectors and register saves | Ordinary functions behind a stdlib dispatcher | F7 |
| 11 | `place()` and zero-page placement clauses | ROM/RAM layout proof | No language placement; profile regions later | draft Ch 11 |
| 12 | No function values; call targets were static | SFA | Exact-signature function pointers; lambdas (implemented, with C-hook trampolines staged) | F3, L1-L5 |
| 13 | No floating point | 6502 cost and cycles | `f32`/`f64` with soft-float verification and an FPU leg | T1, F17, P15 |

## 5. Carried deliberately from Blend65

| Construct | Why it stays |
|---|---|
| C-style syntax, semicolons, mandatory braces | Surface identity; junior familiarity |
| Module system: `module A.B`, `export`, named imports, qualified access, merging, cycles | Worked unchanged; only initialization is replaced (M5) |
| 5-digit stable diagnostic blocks and the registry chapter | Tooling and stability value |
| C operator precedence table and `T(expr)` casts | C exactness (B7, B8) |
| Struct literals with named fields; `Point {}` zero value | Ergonomics (F4) |
| Array fill literals `[a, b; fill]` and `[; fill]` | Compact fixed-array initialization |
| Scoped enums with explicit values | Plus a backing type now |
| Static-origin string literals; byte-oriented text | Matches T2/T3 |
| `embed` data inclusion | Asset workflow (Ch 13) |
| `sizeof`/`offsetof`/`len` query intrinsics | `usize` results; `len` compile-time for fixed arrays |
| `comptime` declaration syntax | Compile-time evaluation path; staged, not dropped (R1) |
| `//` and `/* */` comments | C familiarity; `///` added for documentation (B9) |

## 6. Added in Clorraine (not in Blend65)

| Feature | Ref |
|---|---|
| Ownership modes and markers (`const`/`borrow`/`take`; optional call-site assertions) | V1-V11 |
| Moves, zero = empty, definite assignment | O1-O7, F9 |
| Views (slices, `str`), elision, reborrow | T9, F2, O12 |
| Nullability `T?` with `null` as the only empty state | T8, F9 |
| `raw` and `unsafe` (block statement and prefix operator) | T6, GF4 |
| `volatile` fields, volatile globals, six hardware intrinsics | T7 |
| FFI blocks, `handle T freedBy F`, uncontracted calls inside `unsafe` | X1-X6 |
| Generics with monomorphization and callable bounds | G1-G8 |
| Lambdas (implemented; C-hook trampolines staged) | L1-L5 |
| Templates `$"..."` | S1-S5 |
| `defer` | F5 |
| Floats | T1, F17 |
| Documentation comments `///` | B9 |
| Dot-call syntax | F8 |
| Always-on bounds checks | T12 |
| Defined arithmetic (wrap, masked shifts, division trap) | B7, P12 |
| Generated module initialization | M5 |
| `intrinsic func` declarations | T7 |

## 7. Guarantees lost and substitutes

Every guarantee Blend65 could prove statically came from SFA or the fixed
target. The C backend trades those proofs for reach; each row names the
substitute.

| Lost guarantee | Substitute |
|---|---|
| Static RAM and frame budget per function | Profile size warnings (the W10004 size-warning pattern); codegen report planned |
| Static stack-peak proof | Documented stack-depth guidance (F7); debug instrumentation in the emulator |
| Zero-page budget proof | No zero page; A4/A5 usage is codegen territory |
| Interrupt overlap proof (compiler-generated save/restore) | Dispatcher contract: handlers are allocation-free, drop-free, panic-free (F7) |
| Escape proof for local addresses (`E10260`) | Safe subset keeps the no-escape rule for views (O11); raw escapes are audited `unsafe` |
| Defined (unspecified but safe) uninitialized locals | Definite-assignment error (F9) |
| Deterministic ROM layout budget | Deterministic emitted C (P13); placement is a profile feature later |
| 6502 cycle predictability | Emulator timing; documented `-O` policy; no cycle guarantee |

## 8. Grammar freeze checklist resolutions

The pre-Phase-B review left surface gaps. Each is resolved here; user-ruled
freezes (GF1-GF4) are recorded in the ledger.

| Ref | Item | Resolution | Where |
|---|---|---|---|
| GF1 | Generic explicit type arguments in expressions | Angle brackets with a bounded speculative parse (`<` is comparison unless a type list closes before `(` or `{`) | grammar 6.5, 11; ledger 19 |
| GF2 | Assignment as an expression | C precedence, **void result**; chaining and value use are errors | grammar 6.1; ledger 19 |
| GF3 | Literal typing | No suffixes; context-typed (default `i32`; hex/binary may use the full unsigned width) | grammar 10.5; ledger 19 |
| GF4 | Raw operations in expressions | `unsafe` prefix operator (`unsafe p.next`) plus the `unsafe { ... }` statement | grammar 6.3, 11; ledger 19 |
| GF5 | Member access through `raw` (E1-F2) | `.` for all member access; the dereference is implied and requires an unsafe context | grammar 6.4, 11 |
| GF6 | `const raw` register base (E1-F3) | Constant `raw` casts of integer addresses are constant expressions | grammar 6.8; T7 |
| GF7 | Writable lend of a view of a borrow (E2-F4) | Call-site markers plus the O12 reborrow rules | grammar 6.4; O12 |
| GF8 | Character literal type (E3-F2) | `'x'` is a `u8` byte literal | grammar 10.6; F14 |
| GF9 | Element loop binding (E4-F2) | Copy by default; `for (borrow e in xs)` binds exclusive element views | grammar 5.2; F1 |
| GF10 | Bounds checking (E4-F3) | Always checked; unchecked access only through raw/`unsafe` or stdlib helpers | T12 |
| GF11 | Field punctuation and enum member access (E4-F5) | `.` for fields, enum members (`Direction.UP`), and module qualification | grammar 6.4 |
| GF12 | Volatile globals (E5-F1) | `volatile` qualifier on module-level `let` | grammar 3.5; T7 |
| GF13 | Function values (E5-F3) | Bare identifier; no `&`; `&` is address-of only | grammar 6.5 |
| GF14 | `sizeof` (E6-F3) | Reserved built-in `sizeof(type)`, with `offsetof` and `len`; all return `usize` | grammar 7 |
| GF15 | Narrowing inside `defer` | The block is checked at the exit context with the exit-site flow state | grammar 5.4; F5 |
| GF16 | Slice from a raw pointer | No grammar form; a stdlib `unsafe` helper takes `raw T` plus length and returns `T[]` | stdlib scope; O9 |
| GF17 | Zero forms | `Point {}` zero struct; `[]` and `[; fill]` arrays; `null` only for handles | grammar 6.6, 6.7; F9 |
| GF18 | Uninitialized locals | `let x: T;` allowed; definite assignment enforces use-after-init; `const` needs an initializer | grammar 5; F9 |
| GF19 | Float literals | `float_literal` production; `1.5` is `f64`; digits on both sides of the dot | grammar 10.5; T1 |
| GF20 | Module-level initializer | Mandatory at module level | grammar 3.5; M7 |
| GF21 | `str`/`cstring` surface | Keywords used in type positions only | grammar 10.2; T2 |
| GF22 | `comptime` syntax | Frozen as `comptime func`; semantics staged behind staged diagnostics | grammar 3.1; R1 |
| GF23 | Intrinsic declarations | `intrinsic func name(...): T;` accepted only for compiler-known names | grammar 3.6; T7 |

## 9. Example validation

The grammar is validated against the pre-Phase-B corpus
(`docs/ergonomics-corpus.md`): every construct in the hand-written sketches
must have a production. The sketches used pre-ruling forms in three places;
those are corrections for Phase C chapters, not grammar gaps.

| Sketch | Constructs exercised | Grammar coverage | Corrections |
|---|---|---|---|
| E1 chips | `volatile` overlay fields, `raw` cast, `.` access in `unsafe`, `const` base, bitwise or, assignment | 3.2, 6.3, 6.4, 6.5, 3.5 | `const raw Regs = raw Regs(...)` now constant-evaluable (GF6) |
| E2 file load | `File?`, `null` narrowing, moves, `borrow bufferBytes(buf)`, owned return, drops | 3.1, 4, 5.1, 6.4 | `borrow <call>` is a marked argument plus reborrow (GF7, O12) |
| E3 text dump | templates, interpolation, `'0'`, imports, inferred locals | 7, 6.5, 10.6, 2.3 | `'0'` types as `u8` (GF8) |
| E4 records | read-only lend default, `borrow` mutation, copies, element loop, view return, `slice`, `len` | 3.1, 5.2, 6.4, 6.5 | `moveAll` uses `for (borrow e in xs)` (GF9); `half` elides to `buf` (EL-2) |
| E5 timer | volatile global, function value, nullable parameter, `!= null` narrowing | 3.5, 6.5, 4, 5.1 | `ticks` gains `volatile` (GF12); `addHandler(onTick)` is a bare function value (GF13) |
| E6 list | `raw Node?` field, `sizeof`, `raw` cast, raw member access, out-parameter, `null` narrowing | 3.2, 4, 6.3, 6.4, 7 | raw reads use `unsafe` (block or prefix, GF4); `sizeof` needs no import (GF14) |

The Phase 0 hosted skeleton is C, not Clorraine, so it validates the codegen
target rather than this grammar (see `docs/findings-phase0.md`).

## 10. Resolved open questions from the draft chapters

| Draft chapter | Stale assumption | Now |
|---|---|---|
| 01 lexical | "ownership keyword set undecided" | `let`/`const`, `const`/`borrow`/`take` (V1-V5) |
| 04 expressions | "assignment is a statement"; "method-style call sugar: none" | GF2 (void expression); dot-call frozen (F8) |
| 06 functions | `borrow mut` parameters; `function` spelling | `borrow` parameter; `func` (V5, D8) |
| 08 arrays | "bounds checked in debug, unchecked in release" | Always checked (T12) |
| 09 enums | tagged unions and `Maybe<T>` | Cancelled; status structs and `switch` (T10) |
| 12 FFI | contract clause keywords (`owns`/`consumes`/...) | `handle T freedBy F` and ordinary signatures (X1-X3) |
| 13 data | "`comptime function` deferred" | Syntax frozen, semantics staged (GF22) |
