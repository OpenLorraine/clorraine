# Validation Corpus: Clorraine Compiler MVP

> **Document**: 03-01-corpus.md
> **Parent**: [Index](00-index.md)
> **Status**: Phase 0 source — created before any compiler code (AR-3, AR-44)

## Overview

The corpus is the MVP's acceptance target and its positive golden suite. It is
the specification's own v1-seam validator (`spec/lineage.md:169-175`): the
programs exercise the conservative pre-NLL approximation, the copy-on-lend
rule for duplicable values (V9/V9a), and view elision without annotations. The
programs are written by hand first, reviewed against the frozen specification,
and only then used to drive the compiler.

This document owns the corpus layout, behavior contracts, expected outputs,
and the hand-check procedure. Exact standard-library signatures live in
[03-07-stdlib.md](03-07-stdlib.md); rule citations are to the specification.

## Location and Layout

```
spec/tests/corpus/
├── file-copier/
│   ├── program.clo          # module tests.corpus.FileCopier
│   ├── expected-output.txt   # byte-exact stdout
│   └── README.md             # rule-coverage + hand-check record
├── interrupt-counter/
│   ├── program.clo          # module tests.corpus.InterruptCounter
│   ├── expected-output.txt
│   └── README.md
├── string-parser/
│   ├── program.clo          # module tests.corpus.StringParser
│   ├── expected-output.txt
│   └── README.md
└── buffer-container/
    ├── program.clo          # module tests.corpus.BufferContainer
    ├── expected-output.txt
    └── README.md
```

Every program is a single file plus the stdlib. It declares its module first
(MOD-1), has no CLI arguments (M8 fixes `func main(): i32`), prints to stdout
with `println`/templates, and returns `0` on success, `1` on failure. A program
must compile with **zero diagnostics** (AR-43; Ch 14 §15 positive rule) and
produce the byte-exact expected output plus a final newline.

## Hand-Check Procedure (AR-31)

Before Phase 2 starts, each program gets a line-by-line review against the
specification. Each `README.md` records:

1. A construct table: Clorraine construct → rule IDs (`Ch NN §`, rule ID) that
   authorize it, with special attention to the corpus seams: copy-on-lend
   (V9/V9a, `W10001`), the v1 lexical loan checker (`lineage.md:99-104`),
   elision (EL-1..EL-6), reborrow (O12), drop order (O2, P2), and the cleanup
   ladder (Ch 17 §4).
2. A warning audit: why every construct is warning-free (`W10000`-`W10006`).
3. A deliberate-avoidance list: features the program does **not** use, so the
   surface stays inside AR-10/AR-11/AR-13.
4. A friction log: any point where writing the program against the frozen
   specification felt wrong or ambiguous. Repeated V9 or pre-NLL rejections are
   grounds to amend the rule, never to reopen the model; any specification
   change pauses the phase for an explicit user ruling (AR-31).

The user reviews all four programs and their records at the Phase 0 checkpoint
(`99-execution-plan.md` step 0.2) before front-end work begins. The review is
a plan gate: a rejected program is revised and re-reviewed.

## The Four Programs

### 1. File Copier — `file-copier`
module `tests.corpus.FileCopier`

**Purpose:** exercises FFI contracts and IO wrappers, `Buffer`, nullable
acquisition, partial-IO loops, the cleanup ladder on nested failure paths, and
a writable lend of a view of a borrow (`borrow bufferBytes(buf)`, O12).

**Required language surface:** modules/imports; `const` with a `static str`
initializer; `let`; `helper` functions; `if`/`while`/`return`; `i32`/`u32`/
`usize`/`u8`/`bool` conversions; templates; `String` widening; `Buffer`;
`u8[]` views; `slice`; indexing; null narrowing; drop ordering; `deref`-free
`Buffer` access through stdlib wrappers only.

**Required stdlib APIs (03-07):** `std.io`: `File`, `openFile`, `createFile`,
`readFile`, `writeBytes`, `MODE_OLDFILE`, `MODE_NEWFILE`, `println`;
`std.mem`: `Buffer`, `bufferNew`, `bufferBytes`, `bufferLen`, `slice`.

**Behavior:**
1. `const PAYLOAD: static str = "Clorraine corpus: file-copier\n";` (30 bytes).
2. Create `corpus-in.txt`; on `null`, print `file-copier: FAILED` and return 1.
3. Write the payload, looping on partial writes; print
   `file-copier: wrote 30 bytes`.
4. Open `corpus-in.txt`; allocate a 4096-byte `Buffer`; open/create
   `corpus-out.txt`.
5. Loop: read into `borrow bufferBytes(buf)`; on `n > 0` write exactly `n`
   bytes (partial-write loop); accumulate `copied`. Stop at `n <= 0`.
6. Print `file-copier: copied 30 bytes`.
7. Reopen `corpus-out.txt`, read it back into the buffer, and compare every
   byte with the payload bytes; print `file-copier: content matches` or
   `file-copier: FAILED`.
8. Print `file-copier: ok` and return 0.

**Expected output (`expected-output.txt`):**
```
file-copier: wrote 30 bytes
file-copier: copied 30 bytes
file-copier: content matches
file-copier: ok
```

**Hand-check focus:** every exit path drops `File` and `Buffer` exactly once
(O1/O2, P5); the writable reborrow compiles under O12; `String` widening does
not outlive the statement (D5); no `unsafe`, no `raw` in application code
(X6).

### 2. Interrupt Counter — `interrupt-counter`
module `tests.corpus.InterruptCounter`

**Purpose:** exercises the interrupt/hook dispatcher boundary (F7, IH-1..IH-5),
a `volatile` module-level `let` shared with a handler (T7), a capture-less
function value (`func(): void`, F3/L3), and a bounded wait loop.

**Required language surface:** module-level `volatile let`; function value as
a bare identifier; `while`; `u32` arithmetic; templates; `if`/`return`.

**Required stdlib APIs (03-07):** `std.amiga.interrupt`: `HandlerToken`,
`addHandler`, `removeHandler`; `std.amiga.dos`: `delayTicks`; `std.io`:
`println`.

**Profile rule (AR-30):** this program is `hosted-amiga` only. Under
`--profile host` the compiler reports E11200 because `std.amiga.dos` and the
dispatcher are not in the host profile surface (PR-2); the corpus harness
asserts that diagnostic rather than compiling the program for host.

**Behavior:**
1. `volatile let ticks: u32 = 0;`
2. `func onTick(): void { ticks = ticks + 1; }`
3. In `main`: register the handler, print `interrupt-counter: registered`.
4. Bounded wait: `while (ticks < 50 && i < 1000) { delayTicks(1); i = i + 1; }`.
5. If `ticks >= 50`, print `interrupt-counter: ticks>=50 ok`; otherwise print
   `interrupt-counter: ticks>=50 failed` and set `rc = 1`.
6. Unregister, print `interrupt-counter: removed`; print
   `interrupt-counter: ok`; return `rc`.

**Expected output (`expected-output.txt`):**
```
interrupt-counter: registered
interrupt-counter: ticks>=50 ok
interrupt-counter: removed
interrupt-counter: ok
```

**Hand-check focus:** the handler is allocation-free, drop-free, panic-free
(IH-2); `volatile` prevents stale reads (IH-3); the handler creates no
capture and no loan (L3/L5); the token is a plain duplicable struct.

**Execution:** the accepted home leg is FS-UAE with `RUN_CAPTURE=1`
(`docker/scripts/run.sh`, AR-4). If FS-UAE cannot execute the handler in the
tested environment, the fallback is a build-only leg plus a recorded finding;
the user rules on the fallback at the Phase 1/8 checkpoint.

### 3. String/Parser — `string-parser`
module `tests.corpus.StringParser`

**Purpose:** exercises `str` indexing and element access, an `enum` + `switch`
classifier, template formatting, `String` temporaries at statement end, and
digit parsing without library help.

**Required language surface:** `enum TokenKind : u8`; `switch`/`case`/`break`;
`static str` const; `str` indexing and `len`; char literals (`u8`); `while`;
`if`/`else`; templates; `String`; `static str` widening; explicit conversions
(`i32`, `u32`, `usize`).

**Required stdlib APIs (03-07):** `std.fmt`: `toHex`; `std.text`:
`String`; `std.io`: `println`.

**Behavior:**
1. `const INPUT: static str = "MOV 12 34 ADD 5 7";`
2. Scan bytes; split on `' '`; for each token classify with a `switch` on
   `TokenKind`: all-digit ⇒ `Number` (accumulate `sum`), otherwise `Word`.
3. Count `tokens`, `numbers`, `words`.
4. Print `string-parser: tokens=6 numbers=4 words=2`, then
   `string-parser: sum=58`, then
   `string-parser: hex=0x3A` using `toHex(u32(sum))`, then
   `string-parser: ok`; return 0.

**Expected output (`expected-output.txt`):**
```
string-parser: tokens=6 numbers=4 words=2
string-parser: sum=58
string-parser: hex=0x3A
string-parser: ok
```

**Hand-check focus:** `toHex` produces an owned `String` that widens to `str`
for `println` and drops at statement end (D5, T5); `switch` cases end in
`break` so the program is `W10002`-clean; indexing is bounds-checked (IX-1);
the `enum` has an integer backing type (EN-1) and is compared/selected without
ordering (T10a).

### 4. Buffer/Container — `buffer-container`
module `tests.corpus.BufferContainer`

**Purpose:** exercises monomorphized generics with `Array<i32>`, move-only
containers, element storage through views, buffer/slice arithmetic, and drop
order (elements first-to-last, then storage; O2).

**Required language surface:** explicit generic instantiation (`arrayNew<i32>()`,
G2); `Array<i32>` methods as dot-calls (F8); `while`; `%`; `let`/`const`;
null narrowing; `Buffer`; `u8[]`; `slice`; indexing; conversions.

**Required stdlib APIs (03-07):** `std.mem`: `Array`, `arrayNew`, `Buffer`,
`bufferNew`, `bufferBytes`, `bufferLen`, `slice`; `std.io`: `println`.

**Behavior:**
1. `let xs: Array<i32> = arrayNew<i32>();` push `1..8`.
2. Print `buffer-container: len=8`.
3. Sum all elements (dedicated function; `while` loop over `xs.get(i)[0]`) and
   print `buffer-container: sum=36`.
4. Count even elements and print `buffer-container: evens=4`.
5. Allocate a 16-byte `Buffer`; write a byte pattern through
   `bufferBytes(buf)` (bounds-checked indexing); print
   `buffer-container: bytes=16` using `bufferLen`.
6. Print `buffer-container: ok`; return 0.

**Expected output (`expected-output.txt`):**
```
buffer-container: len=8
buffer-container: sum=36
buffer-container: evens=4
buffer-container: bytes=16
buffer-container: ok
```

**Hand-check focus:** `Array<i32>` is move-only (V8, appendix stdlib §3); the
one-element view from `xs.get(i)` is indexed immediately and does not outlive
the statement (O12, E10022/E10031); `bufferBytes` creates a shared loan of the
buffer (appendix stdlib §9); `slice` never allocates (T5).

## Surface Pinned by the Corpus

The corpus deliberately does **not** use: floats; lambdas or closures;
`comptime`; `embed`; `unsafe`/`raw`/volatile intrinsics in application code
(they live inside the stdlib); user `ffi` blocks; generic functions beyond the
stdlib's instantiations; generic callable bounds; `defer`; partial moves;
nested templates; `std.amiga.custom`/`std.amiga.exec`; library builds
(AR-10, AR-14, AR-17).

The compiler still implements those surfaces needed by the golden suites even
when the corpus avoids them (for example floats for the E11212 run cases,
`defer` for the cleanup contract, `unsafe`/`raw` for the stdlib itself).

## Corpus Runner Interface

| Leg | Command surface | Programs | Assertion |
|---|---|---|---|
| Host | `cargo test` (`corpus_spec_test.rs`) | file-copier, string-parser, buffer-container; interrupt-counter asserts E11200 under host | stdout equals `expected-output.txt`; zero diagnostics; exit 0 |
| Hosted-amiga build | `make corpus-amiga` in the SDK | all four (`m68k-amigaos-gcc`, Ch 17 §13 flags) | warning-free build; `.prg` produced |
| Hosted-amiga run | `vamos` through `docker/Makefile` | file-copier, string-parser, buffer-container | stdout equals expected |
| Hosted-amiga vbcc build | `vc +aos68k`, Ch 17 §13 | file-copier | warning-free build (R17) |
| FS-UAE | `docker/scripts/run.sh` with `RUN_CAPTURE=1` | interrupt-counter | captured stdout equals expected (AR-4) |

Each leg runs in a scratch working directory and removes `corpus-in.txt` /
`corpus-out.txt` afterwards (AR-48).
