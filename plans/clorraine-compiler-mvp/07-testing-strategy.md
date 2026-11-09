# Testing Strategy: Clorraine Compiler MVP

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

Clorraine is specification-first: the golden files under `spec/tests/` **are**
the specification tests for compiler behavior, and the harness mechanically
iterates them; the ST cases below add the concrete stage behaviors that the
goldens do not cover (AR-44). Expectations come from the specification only.
Spec-test code lives in `compiler/tests/` as CLI-level drivers that spawn the
built `clc` binary — never the library API — so the immutable oracle is the
published CLI behavior (AR-24). Implementation tests are in-module Rust unit
tests (`#[cfg(test)]`) and are written after the implementation.

### Coverage Goals

| Code type | Target | Rationale |
|---|---|---|
| Lexer / parser / diagnostics | every in-scope golden file passes; every grammar disambiguation has an impl test | Goldens are the oracle |
| Name/type/const/ownership checks | every in-scope diagnostic code has its golden file; stage ST cases cover rules without a golden | Goldens + ST cases |
| Codegen | fragment ST cases plus byte-exact determinism; corpus E2E for integration | Output is the contract |
| Runtime | all six run-fault cases on host and hosted-amiga; corpus E2E | Fault contract |
| Standard library | warning-free compile + per-module behavior programs + corpus | Real-object testing |

No mocks: tests use the real compiler binary, the real C toolchain, and the
real shims. Test names state behavior (`should [expected] when [condition]`).

## Specification Test Cases

> These cases derive from the specification and the Ambiguity Register only.
> They are the immutable oracle; a failing spec test means the implementation
> is wrong. Golden-suite rows name the file family; the files themselves are
> the concrete input→expected pairs (annotation `// expect-error CODE` /
> `// expect-warning CODE` plus the Ch 14 template), and the harness asserts
> the code and wildcard-matches the registered first line (AR-23).

### Spike acceptance (manual, `spikes/`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-SPIKE-1 | `spikes/01-vbcc-line/demo.c` via the vbcc command | `#line` accepted; decoded line table or vbcc diagnostics name `demo.clo` lines, or the refutation is recorded and the phase pauses | 03-02 S1, PF-042 |
| ST-SPIKE-2 | `spikes/02-ffi-abi/ffi_abi.c` via GCC and vbcc, run under vamos | prototypes accepted warning-free; the copy succeeds; acquisitions equal releases | 03-02 S2, PF-002 |
| ST-SPIKE-3 | `spikes/03-arithmetic/arith_{gcc,vbcc}.c` on host, GCC+vamos, vbcc+vamos | every entry matches the Ch 17 §10 wrapping/masking table; the vbcc idiom is recorded | 03-02 S3, PF-008 |
| ST-SPIKE-4 | `spikes/04-run-faults/traps.c` (host and Amiga shims) | first line `error CODE: message`; abort/exit; the cleanup counter stays 0 | 03-02 S4, PF-020 |

### Corpus end to end

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-CORPUS-1 | `file-copier/program.clo` on host and hosted-amiga/vamos | zero diagnostics; stdout equals `expected-output.txt`; exit 0 | 03-01 §1, AR-28 |
| ST-CORPUS-2 | `interrupt-counter/program.clo` on hosted-amiga/FS-UAE; under `--profile host` | FS-UAE: stdout equals expected, exit 0. Host: `error E11200:` first line, exit 1 | 03-01 §2, AR-30 |
| ST-CORPUS-3 | `string-parser/program.clo` on host and hosted-amiga/vamos | zero diagnostics; stdout equals expected; exit 0 | 03-01 §3 |
| ST-CORPUS-4 | `buffer-container/program.clo` on host and hosted-amiga/vamos | zero diagnostics; stdout equals expected; exit 0 | 03-01 §4 |

### Run-fault suite (`spec/tests/run/`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-RUN-1 | `E11210_division_by_zero.clo` | compiles clean; first line `error E11210: integer division by zero`; aborts without the ladder | Ch 14 §11, Ch 17 §10 |
| ST-RUN-2 | `E11210_min_neg_one.clo` | compiles clean; first line `error E11210: integer division by zero`; aborts | Ch 14 §11, PF-006 |
| ST-RUN-3 | `E11211_index_out_of_range.clo` | compiles clean; first line `error E11211: index out of range`; aborts | Ch 14 §11, IX-1 |
| ST-RUN-4 | `E11212_float_to_int_range.clo` | compiles clean; first line `error E11212: float-to-integer conversion out of range`; aborts | Ch 14 §11, PF-007 |
| ST-RUN-5 | `E11212_nan_float_to_int.clo` | same as ST-RUN-4 (NaN fails `cl_isnan(f) || !(f >= lo && f <= hi)`) | Ch 14 §11, PF-007, AR-58 |
| ST-RUN-6 | `E11213_assertion_failed.clo` | compiles clean; first line `error E11213: assertion failed`; aborts | Ch 14 §11, EX-15 |
| ST-RUN-7 | all six above under hosted-amiga via vamos (S4 patterns) | each prints its first line on stdout and exits nonzero; ladder skipped | AR-32, AR-36 |

### Compile-fail suites (`spec/tests/compile-fail/`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-CF-OWN | all `E100*.clo` files (23 cases incl. three E10031) | exit 1; annotated code; first line matches the Ch 14 template | Ch 14 §3, Ch 03, Ch 16 |
| ST-CF-FFI | `E101*.clo` (5 files) | exit 1; annotated code; template match | Ch 14 §4, Ch 12 |
| ST-CF-TYPE | all `E102*.clo` except E10213/E10214 (excluded, AR-10) | exit 1; annotated code; template match | Ch 14 §5, Ch 02/04/07/09 |
| ST-CF-GEN | `E10300`, `E10301`, `E10302`, `E10305` (E10303/E10304/E10306 excluded) | exit 1; annotated code; template match | Ch 14 §6, AR-13 |
| ST-CF-STR | `E10500`, `E10501`, `E10503` | exit 1; annotated code; template match | Ch 14 §8, Ch 04 §11 |
| ST-CF-UNSAFE | `E10600`-`E10604` (5 files) | exit 1; annotated code; template match | Ch 14 §9, T6/T7 |
| ST-CF-MOD | `E11100`-`E11106` plus `E10219` | exit 1; annotated code; template match | Ch 14 §10, Ch 10 |
| ST-CF-PROFILE | `E11200`, `E11201` | exit 1; annotated code; template match | Ch 14 §11, Ch 15 |
| ST-CF-LEX | `E20000`-`E20008` (9 files) | exit 1; annotated code; template match | Ch 14 §12, Ch 01 |
| ST-CF-WARN | `W10000`-`W10004`, `W10006` (6 files) | exit 0; first line `warning CODE: ...`; default build keeps warnings deniable | Ch 14 §14, N6 |
| ST-CF-STAGED | `E90001_comptime_staged.clo`, `E90005_nested_template.clo` | exit 1; annotated code | Ch 14 §13 |
| ST-CF-DENY | `W10000_deny_warnings.clo` (`// cli: --deny-warnings`) | exit 1; first line stays `warning W10000: unused local '...'` | AR-34, N6 |
| ST-CF-SKIP | `tests/staged/*` (4 files) and all `// stage:` files (E10306, E10401-E10404, E90002) plus the AR-10 exclusion list | not executed; each reported by name and reason; aggregate skip count shown | AR-10, AR-23 |

### Stage behavior (not covered by a golden alone)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-LEX-1 | `"A" 'x' 0xFF 0b1010 1_000 1.5e3 $"a{x}b"` | tokens with expected kinds/values; template parts split correctly | LEX-13..LEX-26, grammar §10 |
| ST-LEX-2 | `1.`, `.5`, `0x`, `'ab'`, `"\q"` | E20004, E20004, E20004, E20007, E20003 respectively | Ch 01 |
| ST-PARSE-1 | `a < b > (c)` and `f<i32>(1)` | comparison versus generic call per the bounded speculative rule | grammar §11 |
| ST-PARSE-2 | `Point { x: 1 };` versus `{ let y = 1; }` | struct literal versus block in statement position | grammar §11 |
| ST-PARSE-3 | `for (x in xs)` versus `for (let i = 0; i < n; i = i + 1)` | element form versus three-clause form | grammar §5.2 |
| ST-PARSE-4 | `union U { a: i32; }`, misplaced `volatile`, `$"{x:04x}"` | E10204, E10208, E10501 (parser detection) | PF-031, grammar §11 |
| ST-NAME-1 | Ch 10 §5 shadowing example; use before declaration in a block | inner binding wins; later reference is E10219 | MOD-15/16 |
| ST-NAME-2 | merged duplicate `func f`; missing import; duplicate import | E11101; E11102; E11103 | MOD-7..MOD-12 |
| ST-TYPE-1 | `u8 + u16`, `i32 + u32`, `let x: u8 = 300`, `let y: i32 = 1.5` | unify to `u16`; E10200; E10202; E10200 | T1a, TY-11..13, PF-005 |
| ST-TYPE-2 | `u16(u8val)`; `u8(u16val)`; `i32(f64val)`; `f32(f64val)` | explicit widening/narrowing/casts allowed; `f32→f64` implicit | T1/T1a, TY-12 |
| ST-TYPE-3 | `if (1) { }`; `1 && true`; `E.A > E.B` | E10218; E10218 (non-`bool` logical operand); E10222 (enum ordering, T10a) | EX-6, EX-7, EN-6 |
| ST-CONST-1 | `const N: i32 = 2 + 3 * 4;`; `const Z: i32 = 1 / 0;`; module `let n = 5; const M: i32 = n;` | 14; E10223; E10224 | EX-16 |
| ST-ENUM-1 | `enum E : u8 { A, B = 5, C }` | values 0, 5, 6; `E(x)` cast | EN-1/EN-3, T10a |
| ST-OWN-1 | move then use; move while borrowed; assign to `const` | E10020; E10022; E10023 | OW-4/OW-5, STMT-3 |
| ST-OWN-2 | view return with two params; view of a local; `take`-only candidate | E10036; E10035; E10036 | EL-2/EL-3, PF-013 |
| ST-OWN-3 | nullable use before `== null`; `null` to non-null | E10010; E10012 | TY-6/TY-7 |
| ST-OWN-4 | nested partial move; whole use after partial move | E10026; E10025 | STR-10..STR-12 |
| ST-OWN-5 | `borrow bufferBytes(buf)` where `buf` is a `borrow` parameter | compiles (exclusive reborrow limited to the call, O12) | BR-10, O12 |
| ST-OWN-6 | scope drops: locals reverse order; fields declaration order; temp at statement end | emitted ladder order matches O2/D5 | O2, D5, CL-3 |
| ST-FFI-1 | uncontracted call outside `unsafe`; owned value to uncontracted param; bad deleter | E10100; E10101; E10102 | CN-10/CN-11/CN-6 |
| ST-FFI-2 | `Write(borrow file: File, const buf: u8[], len: i32)` emitted prototype | `LONG Write(BPTR, APTR, LONG)` | Ch 17 §3, S2 |
| ST-CG-1 | a function owning two resources with an early return | one `__cl_c<N>` label per scope; null-safe drops on the fall-through path | Ch 17 §4 |
| ST-CG-2 | handle move; aggregate move | `dst = src; src = NULL;`; `dst = src; cl_zero(&src);` | Ch 17 §5 |
| ST-CG-3 | any compiled statement | `#line N "logical/path.clo"` precedes it; the path is the CLI path, never absolute | P13, AR-35 |
| ST-CG-4 | `a / b`, `xs[i]`, `i32(f)` | guards for E11210, E11211, E11212 exactly as Ch 17 §10 | Ch 17 §10, S4 |
| ST-CG-5 | two modules with initializers and a dependency | `cl_init_*` functions in topological order, lexicographic tie-break; stub runs them before user `main` | MOD-20/21, PF-009 |
| ST-CG-6 | `identity<i32>` and `identity<t2>` where both key `i32`; a distinct key | identical instantiations dedupe to one symbol; distinct use `__<typekey>` | P11, G3 |
| ST-CG-7 | compile `string-parser` twice | byte-identical C output | P13, AR-35 |
| ST-CG-8 | `f32`/`f64` literals, arithmetic, comparison, casts | emitted C float/double code; `text(f)` six significant digits | AR-11, S2a |
| ST-CLI-1 | missing `--profile`; `--profile nope`; `--profile freestanding` | usage error; E11201; E11200 | AR-21, PR-5/PR-6 |
| ST-CLI-2 | a file with two independent errors; `--max-errors=1` | both reported (N5); exactly one reported with the cap | N5 |
| ST-CLI-3 | first-line rendering | `error CODE: message` exact first line, then `  --> path:line:col` | AR-22, N4 |
| ST-STDLIB-1 | compile the embedded stdlib slice alone under both profiles | zero diagnostics | AR-42 |
| ST-STDLIB-2 | program using `bufferNew`/`bufferBytes`/`slice`/`println` on host | expected stdout; E11211 if sliced out of range | appendix §2/§8, AR-51 |
| ST-STDLIB-3 | file round-trip through `createFile`/`writeBytes`/`openFile`/`readFile` on host | bytes read equal bytes written | appendix §4, Ch 12 §6 |
| ST-STDLIB-4 | `arrayNew<i32>` push/get/set/len; `len` lowers to `.count` | expected values; duplicate keys dedupe | appendix §3, AR-50 |
| ST-STDLIB-5 | `arrayNew<Buffer>()` instantiation | E90000 (non-duplicable element restriction) | AR-49 |
| ST-STDLIB-6 | `spanOf(src, 10, 5)` with `len(src) == 8` | E11211 first line at run time | appendix §8, AR-51 |
| ST-WARN-1 | each warning trigger | `warning W10000/W10001/W10002/W10003/W10004/W10006` first lines; W10004 uses profile thresholds (host 16384, hosted-amiga 1024) | Ch 14 §14, W10004a |

## Test Categories

### Specification Tests (from ST cases above)

| Test File | ST Cases Covered | Component |
|---|---|---|
| `compiler/tests/compile_fail_spec_test.rs` | ST-CF-*, ST-CF-DENY, ST-CF-SKIP | Diagnostics |
| `compiler/tests/stage_spec_test.rs` | ST-LEX-*, ST-PARSE-*, ST-NAME-*, ST-TYPE-*, ST-CONST-1, ST-ENUM-1, ST-OWN-*, ST-FFI-*, ST-CG-1..6, ST-CG-8, ST-CLI-1..3, ST-STDLIB-1, ST-STDLIB-5, ST-WARN-1 | Stage behavior |
| `compiler/tests/codegen_run_spec_test.rs` | ST-STDLIB-2, ST-STDLIB-3, ST-STDLIB-4, ST-STDLIB-6 | Host codegen compile-and-run |
| `compiler/tests/run_fault_spec_test.rs` | ST-RUN-1..7 (host leg; Amiga leg driven by `make verify`) | Runtime |
| `compiler/tests/corpus_spec_test.rs` | ST-CORPUS-1,3,4, ST-CORPUS-2 (host E11200) | End to end |
| `compiler/tests/determinism_spec_test.rs` | ST-CG-7 | Codegen |
| `compiler/tests/cli_spec_test.rs` | ST-CLI-1..3 | Driver |
| `spikes/*/Makefile` (manual) | ST-SPIKE-1..4 | Toolchain |
| `make verify` legs | ST-RUN-7, ST-CORPUS-2 (FS-UAE) | Target |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
|---|---|---|
| `src/lex.rs` unit tests | tokenization, templates, lexical diagnostics | High |
| `src/parse.rs` unit tests | disambiguation table, recovery, error spans | High |
| `src/types.rs` unit tests | conversion matrix, unification, casts | High |
| `src/consts.rs` unit tests | folding, overflow, faults, extents | High |
| `src/ir.rs` / `src/own.rs` / `src/borrow.rs` unit tests | IR snapshots, joins, loan conflicts, reborrow | High |
| `src/codegen.rs` / `src/abi.rs` unit tests | fragments, naming, ladder, guards | High |
| `src/resolve.rs`, `src/load.rs` unit tests | scopes, imports, module merge, init order | Medium |

## Test Data

### Fixtures Needed

- The existing `spec/tests/compile-fail/`, `spec/tests/run/`, and the new
  `spec/tests/corpus/` goldens (the owning data; the harness iterates them).
- `spikes/` C fixtures and their Makefiles.
- The Docker SDK image; no ROM files (vamos and the AROS ROM suffice).
- Scratch directories for host compiles and corpus IO; the corpus runner
  removes `corpus-in.txt`/`corpus-out.txt` (AR-48).

### Mock Requirements

None. Tests use the real compiler binary, the real host C compiler, and the
real cross toolchain. A stub is never substituted for the compiler or shim.

## Verification Checklist

- [x] All specification test cases (ST-*) defined with concrete input/output pairs
- [x] Every ST case traces to a requirement, spec doc, or AR entry
- [x] Specification tests written BEFORE implementation
- [x] Specification tests verified to FAIL before implementation (red phase)
- [x] All specification tests pass after implementation (green phase)
- [x] Implementation tests written for edge cases and internals
- [x] All unit / integration / E2E tests pass
- [x] No regressions in existing tests (`spec/check.sh` stays green)
- [x] Security-relevant checks covered (untrusted source, no shell, no path traversal)
