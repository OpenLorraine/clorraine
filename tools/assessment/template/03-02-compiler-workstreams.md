# Compiler Workstreams: Independent Assessment of Clorraine

> **Document**: 03-02-compiler-workstreams.md
> **Parent**: [Index](00-index.md)

## Overview

Checklists for WS1–WS4. Each item has an ID; the workstream file reports one outcome per ID
(03-01 §Checklist outcomes, ST-8). Method, evidence, and severity are owned by 03-01.
Partition per AR #18; effort per AR #19.

---

## WS1 — Semantic core

**Components:** `lex`, `parse`, `ast`, `load`, `resolve`, `sema`, `types`, `consts`,
`analysis.rs`, `driver.rs`, `abi.rs`, `ffi.rs`, `profile.rs` (~23k lines).
**Spec:** chapters 01, 02, 04–10, 12–15, `grammar.ebnf.md`, `grammar-deltas.md`, `decisions.md`.
**Lens:** wrong acceptance and wrong rejection, including a wrong diagnostic code (chapter 14);
diagnostic wording is not assessed — an assessment focus, not a scope decision.

| ID | Check |
|----|-------|
| WS1-01 | Lexer: numeric literal ranges and suffixes at every width, escape sequences, string/char edge cases, comment nesting, UTF-8 handling against chapter 01 |
| WS1-02 | Parser: grammar conformance against `grammar.ebnf.md` and `grammar-deltas.md`; operator precedence and associativity; ambiguous constructs; error recovery never accepting an invalid program |
| WS1-03 | Name resolution: shadowing, module imports and visibility, cyclic imports, qualified access, stdlib name collisions with user names; `analysis.rs` stage order never letting a later stage consume a rejected program |
| WS1-04 | Type checking: integer promotion and conversion rules, signed/unsigned mixing, implicit narrowing, nullable types, enum/union exhaustiveness, generic constraint checking |
| WS1-05 | Constant evaluation: `consts/` folding matches runtime semantics at every width (overflow, shifts by ≥ width, negative shifts, division by zero, `i32::MIN / -1`, float rounding); folded vs. runtime differential probes |
| WS1-06 | Type layout: size, alignment, padding, and field order for structs, unions, enums, arrays, and nested aggregates on host and m68k (2-byte alignment on 68000 GCC); agreement between `types/` and the emitted C (`sizeof` probes) |
| WS1-07 | FFI contracts (chapter 12): declared C signatures vs. NDK and libc prototypes (expectation source: the NDK include prototypes), by-value aggregate passing, nullability at the boundary |
| WS1-08 | Data inclusion (chapter 13): size limits, alignment of included data, chip-memory placement where the spec requires it |
| WS1-09 | Profile gating (chapter 15): a profile-restricted API is rejected on every other profile; KS1.3-only restrictions on `hosted-amiga-13` |
| WS1-10 | Robustness: pathological inputs (deep nesting, very long identifiers, huge literals, many generics) never crash or hang `clc`; time and peak RSS within 10× the Phase 0 `bench-compile` value for the same input size (03-01 §Severity) |

---

## WS2 — Ownership, borrowing, and drops

**Components:** `own/` (37,963 lines), `borrow.rs`, `init_graph.rs`.
**Spec:** chapters 03, 05 (`defer`), 06 §6.1 (lambdas), 11, 16, `decisions.md`.
**Lens:** any accepted program that leaks, double-frees, uses after free or move, or reads
uninitialized memory is Critical. Every probe runs under the ASan and UBSan builds of
03-01 §Ground rules.
**Sessions:** WS2a = WS2-01..WS2-06; WS2b = WS2-07..WS2-12.

| ID | Check |
|----|-------|
| WS2-01 | Move semantics: moves in every expression position (argument, return, field init, array element, struct literal, conditional arms, template placeholder); partial moves and drops of the remainder |
| WS2-02 | Drop elaboration: drop order (spec order vs. emitted order), drop flags on conditional paths, drops on early `return`, `break`, `continue`, and runtime faults |
| WS2-03 | Loops: moves inside loops, element loops over owned/borrowed arrays, nested loops, `break` with owned temporaries live, loop-carried reinitialization |
| WS2-04 | `defer`: ordering with drops, captures of moved values, nested defers, defers on fault paths |
| WS2-05 | Temporaries: lifetime of call results, slice temporaries, template temporaries, temporaries in guarded (short-circuit and conditional) operands |
| WS2-06 | NLL v2 (chapter 16): loans across branches, reborrows, two-phase borrows, borrows of fields vs. whole, borrows held across calls that may move |
| WS2-07 | Views and escapes: a borrow view stored into an owned field, returned, captured, or written through an out-slot; global borrows |
| WS2-08 | Generics: ownership of substituted types (droppable vs. copy instantiations of the same generic); monomorphized drop glue |
| WS2-09 | Closures and lambdas: capture by move vs. borrow, release of callables, closures that outlive captures |
| WS2-10 | Projections: conditional projections, projections over call results, nested field/element projections with moves |
| WS2-11 | Initialization: definite-initialization analysis (`init_graph.rs`) on every control shape; reading a moved-from or never-initialized binding is rejected |
| WS2-12 | Structural review: how many independent code paths decide "move / lend / copy" and "drop here"; places where the same decision is re-derived and can disagree (input to WS8-08) |

---

## WS3 — IR and C codegen correctness

**Components:** `ir/` (2,355 lines), `codegen/` (20,308 lines), the emitted C.
**Spec:** chapter 07 §7 (volatile/MMIO), chapter 11, chapter 15, chapter 17 (codegen ABI).
**Method:** use the Phase 0 emission (task 0.2.2) for the corpus, the in-scope examples, and
the benchmarks on every profile that accepts each program, plus WS3 probes; build with
stricter flags than the gates use.

| ID | Check |
|----|-------|
| WS3-01 | C99 validity beyond the gates: build emitted C with `-std=c99 -pedantic -Wall -Wextra -Wconversion -Wsign-conversion -Wcast-align -Wshadow -Wstrict-prototypes` on host GCC and m68k GCC; classify each warning class as real defect or noise |
| WS3-02 | vbcc: the emitted C and the hosted-amiga shims compile with vbcc without errors or type-mismatch warnings (C-validity leg only, AR #9) |
| WS3-03 | Undefined behavior: evaluation order of side effects in calls and operators; signed overflow on paths `-fwrapv` does not cover (vbcc, host profile without `-fwrapv`); shift widths; out-of-range conversions; `INT_MIN` division |
| WS3-04 | Aliasing: strict-aliasing violations in emitted casts and shim pointer punning, given that no profile passes `-fno-strict-aliasing` |
| WS3-05 | 68000 specifics: word/long access at odd addresses (address error on 68000), packed or byte-offset struct fields, `int` width assumptions, big-endian layout of unions and included data; a `limit` if neither `vamos` nor FS-UAE raises the address error on a seeded violation |
| WS3-06 | `volatile` and MMIO: every MMIO access is a single access of the declared width, never split, merged, or reordered; per-chip wrappers lower as specified |
| WS3-07 | Name mangling: collisions between user names, generic instances, stdlib symbols, C keywords, NDK macros, and shim symbols; identifier length limits of the m68k toolchains |
| WS3-08 | Guards and faults: bounds, null, and overflow guards emitted exactly where the spec requires; fault paths release owned values (cross-check WS2-02) |
| WS3-09 | Determinism: identical emitted bytes across the two Phase 0 emissions, and between the debug and release `clc` builds in the container |
| WS3-10 | ABI (chapter 17): struct return, by-value aggregates, out-slots, callback signatures, and varargs at the C boundary match the spec on host and m68k |
| WS3-11 | Generated-C shape robustness: deeply nested expressions, large switch/match, long functions, many locals: the C compilers accept them within 10× the Phase 0 C compile time of the nearest-size emitted program (03-01 §Severity) |

---

## WS4 — Generated-code quality and the optimizer question

**Components:** `codegen/`, `ir/`, `consts/`, the release flags, the emitted C of the benchmarks.
**Method:** cite the Phase 0 measurement matrix (03-01 §Measurement rules); attribute
instruction counts per function with `vamos` traces and symbol maps (the method of
`real-workload-audit`); estimate each optimization with a patched-C probe (AR #17).

| ID | Check |
|----|-------|
| WS4-01 | Overhead table: from the Phase 0 matrix, per benchmark, Clorraine vs. the equivalent C reference, each netted against its own calibration, on every lens |
| WS4-02 | Overhead attribution: for every benchmark ≥10% above its C reference on a shipped 68000 lens, attribute the gap to concrete emitted shapes with disassembly evidence |
| WS4-03 | Guard cost: bounds/null/overflow guards GCC cannot remove; which a simple IR range fact would remove; patched-C estimate |
| WS4-04 | Drop-flag and cleanup cost: drop flags and cleanup ladders that survive `-O2`; patched-C estimate of static drop elaboration |
| WS4-05 | Temporaries and copies: address-taken temporaries that block register allocation; by-value aggregate copies; missing return-value elision; patched-C estimate |
| WS4-06 | Unnecessary `volatile`, `static` state, or function-pointer indirection that blocks inlining |
| WS4-07 | Code size on the A500: the fixed per-program cost (calibration `.text` on each side, shim functions surviving `--gc-sections`, string tables), reported once; per-feature cost |
| WS4-08 | Flags: `-O2` vs. `-Os` vs. `-O2 -fomit-frame-pointer` vs. LTO on the A500 lens and the 68020 probe lens; whether `-fwrapv` costs measurable code (flag findings state both flag sets) |
| WS4-09 | Pass candidates: for every shape above, the IR-level pass that would remove it, its expected gain (from the patched-C probe), its implementation size, and its correctness risk |
| WS4-10 | The answer to R5: "no optimizer needed", "targeted passes", or "an optimizer stage", with the measured evidence |

## Testing Requirements

Acceptance per the `07-testing-strategy.md` check-point row of each phase.
