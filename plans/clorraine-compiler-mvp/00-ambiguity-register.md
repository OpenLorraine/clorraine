# Ambiguity Register: Clorraine Compiler MVP

> **Status**: ✅ GATE PASSED — all 71 items resolved (AR-1..AR-62; AR-63/AR-64 raised and resolved during Phase 4 execution; AR-65 raised and resolved during Phase 5 execution on 2026-09-19; AR-66, AR-66a, and AR-67 raised and resolved during Phase 6 execution on 2026-09-20; AR-68 raised and resolved during Phase 7 execution on 2026-09-21; AR-69 and AR-70 raised and resolved during Phase 8 execution on 2026-09-21)
> **Last Updated**: 2026-09-21 20:52
> **CodeOps Artifact Schema**: 1

Scope of this gate: the implementation plan for the Clorraine compiler MVP
(`plans/clorraine-compiler-mvp/`). Planning target: a Rust compiler that turns
the corpus subset of the frozen specification (`spec/`, v0.4, Phase E passed)
into the single translation unit defined by Ch 17, plus its validation corpus,
its Phase 2 verification spikes, and its test suites.

## Resolution key

- Reference docs: `decisions.md` = `spec/decisions.md`, `Ch NN` = `spec/NN-*.md`,
  `preflight` = `spec/preflight-report.md`, `findings1` = `docs/findings-phase1.md`,
  `findings0` = `docs/findings-phase0.md`.
- "User accepted recommendation" means the recommendation as written in this
  register was accepted in bulk; the decision text below is the accepted option.

## Batch 1 — planning parameters (user answers, 2026-09-17)

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| AR-1 | Scope | When to execute this plan | Plan now / plan+execute | Plan now; execution later via `exec-plan clorraine-compiler-mvp --auto-commit` | ✅ Resolved — user chose plan-only |
| AR-2 | Scope | MVP acceptance baseline | Corpus subset / corpus+full compile-fail / full v1 | Corpus-driven subset: 4 programs + 6 run faults + spikes; other features answer E90000 | ✅ Resolved — user chose corpus-driven subset |
| AR-3 | Scope | Corpus role | Executable golden / design-only / +FS-UAE all | Executable golden programs, hand-checked now, compiled and diffed under host and hosted-amiga | ✅ Resolved — user chose executable golden |
| AR-4 | Behavioral | Interrupt-counter execution | FS-UAE run + build fallback / run everywhere / compile-only | FS-UAE run; build-only fallback where interrupts are unsupported | ✅ Resolved — user chose FS-UAE run with fallback |
| AR-5 | Technical | Profiles implemented by the MVP | host+hosted-amiga / hosted-amiga only / host only | Both `host` (fast tests) and `hosted-amiga` (GCC, vbcc, vamos) | ✅ Resolved — user chose both profiles |
| AR-6 | Naming | Compiler location, binary, CLI role | `compiler/` crate, `clorc`, emits C / driver / other name | `compiler/` crate, binary `clorc`, emits C only; scripts drive GCC/vbcc | ✅ Resolved — user chose crate + clorc (binary renamed `clc` by AR-60) |
| AR-7 | Technical | Verify command | Layered make targets / separate commands / other | Layered: fast `cargo` tests in the container; full `make verify` umbrella whose targets the plan defines | ✅ Resolved — user chose layered make targets |
| AR-8 | Naming | Spike code and findings location | `spikes/` + `docs/findings-phase2.md` / scratch / examples | Committed `spikes/` sources; findings in `docs/findings-phase2.md` | ✅ Resolved — user chose spikes/ + findings doc |
| AR-9 | Scope | Roadmap creation | Yes / no | Create `plans/00-roadmap.md` | ✅ Resolved — user chose to create the roadmap |

## Batch 2 — feature scope and architecture (user accepted recommendations, 2026-09-18)

| # | Category | Ambiguity / Gap | Accepted resolution | Status |
|---|----------|-----------------|---------------------|--------|
| AR-10 | Scope | Features deliberately staged out of the MVP and their skipped golden tests | Stage out data inclusion (`embed`), lambda semantics, callable bounds, `stores`/`transfers`, pools, user contract tables, `freestanding`, `comptime`, nested templates. The harness skips `tests/staged/*`, every `// stage:` file (E10306, E10401-E10404, E90002), and E10213/E10214, E10303, E10304 by explicit exclusion list; skipped tests are reported by name and reason | ✅ Resolved — user accepted recommendation |
| AR-11 | Scope | Float surface for the two accepted E11212 run cases | `f32`/`f64` types, literals, arithmetic, comparisons, int↔float casts with the E11212 guard, and `text(f)` at six significant digits (S2a) | ✅ Resolved — user accepted recommendation |
| AR-12 | Scope | Intrinsic set | `sizeof`, `offsetof`, `len`, `panic`, `assert`, the six volatile intrinsics in; `embed` out (E90000) | ✅ Resolved — user accepted recommendation |
| AR-13 | Scope | Generics subset needed by `Array<T>` | Type parameters on functions and structs, monomorphization with dedupe, explicit arguments plus arguments-only inference, export ban, recursion cap (E10300/E10301/E10302/E10305); no callable bounds | ✅ Resolved — user accepted recommendation |
| AR-14 | Behavioral | Lambda reporting (frozen syntax, staged semantics; L4) | Parse the full lambda syntax; a lambda literal reports E90000 `'lambda' is not yet supported`; a capture list reports E90002; stage-annotated tests are skipped | ✅ Resolved — user accepted recommendation |
| AR-15 | Scope | Standard-library slice shipped by the MVP | `std.mem` (Buffer, Array<T>, Span, alloc), `std.text`/`std.mem` String helpers, `std.fmt` (text, toHex, toDecimal, padLeft, padRight), `std.io` (print, println), `std.amiga.interrupt` (dispatcher), `Status` — expanded by AR-47 | ✅ Resolved — user accepted recommendation |
| AR-16 | Interface | `std.io` output signatures (appendix marks output as planned) | `print(s: str): void`, `println(s: str): void` | ✅ Resolved — user accepted recommendation |
| AR-17 | Scope | Per-chip MMIO wrappers (`std.amiga.custom`) | Excluded from the MVP; no corpus program needs them | ✅ Resolved — user accepted recommendation |
| AR-18 | Scope | `defer`, enums, `static str` fields, `///` doc comments | All included; `///` is parsed and ignored | ✅ Resolved — user accepted recommendation |
| AR-19 | Technical | Runtime shim and stdlib architecture | Stdlib in Clorraine under top-level `stdlib/`, embedded in the binary and compiled with each program; per-profile C shim (`runtime/host.c`, `runtime/hosted-amiga.c`) embedded and prepended to the single TU, exposing the profile C API the stdlib binds through `ffi` | ✅ Resolved — user accepted recommendation |
| AR-20 | Technical | Rust project shape and dependency policy | Root `Cargo.toml` workspace + `compiler/` crate (lib + `clorc` bin; renamed `clc` by AR-60), edition 2021, std-only, zero external crates | ✅ Resolved — user accepted recommendation |
| AR-21 | Interface | `clorc` CLI surface | `clorc --profile <host\|hosted-amiga> [--deny-warnings] [--max-errors=N] -o OUT.c FILE...`; profile required; exit 1 on any error; `--version`. Binary renamed `clc` by AR-60 | ✅ Resolved — user accepted recommendation |
| AR-22 | UX | Diagnostic rendering | stderr; first line exactly `error CODE: message` / `warning CODE: message` (N4); following lines `  --> file:line:col` and optional `  help: ...` | ✅ Resolved — user accepted recommendation |
| AR-23 | Technical | Compile-fail message assertion | Assert the code and wildcard-match the registered Ch 14 template (`{...}` matches any text); fall back to code-only for unparseable rows | ✅ Resolved — user accepted recommendation |
| AR-24 | Technical (complexity escalation) | Test-harness support surface | Use the smaller solution: std-only Rust integration tests in `compiler/tests/` spawning the built `clorc` binary (`CARGO_BIN_EXE_clorc`), golden files read from `spec/tests/`; cross-toolchain legs are thin Make targets delegating to `docker/Makefile`; no separate harness crate. Direct user choice after the visible packet; independent challenger verdict: Unnecessary for the larger harness crate. Spawning binary later renamed `clc` (`CARGO_BIN_EXE_clc`), AR-60 | ✅ Resolved — user chose the smaller solution A |
| AR-25 | Technical | Module loading and source discovery (MOD-2) | User files given on the CLI; imports resolve against the loaded set plus the embedded stdlib table; module identity comes from `module` declarations; no directory scanning | ✅ Resolved — user accepted recommendation |
| AR-26 | Naming | Rust test-file naming vs `[feature].spec.test.[ext]` | `compiler/tests/<feature>_spec_test.rs` and `<feature>_impl_test.rs`, documented in AGENTS.md as the Rust spelling of the standard | ✅ Resolved — user accepted recommendation |
| AR-27 | Technical | Profile facts storage | Compiled-in Rust profile tables for `host` and `hosted-amiga` (Ch 15 §6 first position) | ✅ Resolved — user accepted recommendation |
| AR-28 | Naming | Corpus location, layout, program names | `spec/tests/corpus/{file-copier,interrupt-counter,string-parser,buffer-container}/` each with `program.clo`, `expected-output.txt`, `README.md` | ✅ Resolved — user accepted recommendation |
| AR-29 | Behavioral | Corpus program behavior contract | Deterministic stdout, no CLI arguments (M8), relative file paths only, self-contained fixtures | ✅ Resolved — user accepted recommendation |
| AR-30 | Behavioral | Interrupt-counter mechanism and profile behavior | VBlank handler through `std.amiga.interrupt`; `volatile let ticks: u32`; deterministic output shape; `--profile host` reports E11200 | ✅ Resolved — user accepted recommendation |
| AR-31 | Process | Corpus and spike hand-check gate | User reviews each corpus program's rule-coverage table and the spike findings before Phase 2 starts; spec friction pauses the run and needs an explicit user ruling (lineage §10: amend the rule, not the model) | ✅ Resolved — user accepted recommendation |
| AR-32 | Behavioral | Run-fault execution legs | All six cases compile and execute under `host` in `cargo test`; the same cases also execute under `hosted-amiga` via vamos in `make verify` | ✅ Resolved — user accepted recommendation |
| AR-33 | Behavioral | Compile-fail execution profile and annotations | Default `host`; honor `// profile:` and `// cli:`; warnings assert the `warning CODE:` first line | ✅ Resolved — user accepted recommendation |
| AR-34 | Behavioral | Warning escalation case (PF-020) | Add `spec/tests/compile-fail/W10000_deny_warnings.clo` with `// cli: --deny-warnings`, asserting a nonzero exit and the `warning W10000:` first line; document the semantics in the suite README | ✅ Resolved — user accepted recommendation |
| AR-35 | Behavioral | Output determinism proof (P13) | A spec test compiles the same input twice and requires byte-identical C; `#line` carries the logical CLI path | ✅ Resolved — user accepted recommendation |
| AR-36 | Technical | Runtime-fault and panic output streams | Host profile prints the fault first line to stderr then `abort()`; hosted-amiga prints to stdout then exits (Ch 17 §12); `panic` prints `panic: <msg>` | ✅ Resolved — user accepted recommendation |
| AR-37 | Technical | Host-profile C compile command | `cc -std=c99 -O0 -g -Wall -Wextra -Werror` | ✅ Resolved — user accepted recommendation |
| AR-38 | Process | AGENTS.md | Create after the plan via the `analyze-project` skill: verify commands, layout, Rust naming map | ✅ Resolved — user accepted recommendation |
| AR-39 | Naming | Plan folder and identifier | `plans/clorraine-compiler-mvp/` (flat layout; no RD upstream) | ✅ Resolved — user accepted recommendation |
| AR-40 | Process | Roadmap row semantics | Seed one plan row (`clorraine-compiler-mvp`, stage `Plan Created`); no RD rows | ✅ Resolved — user accepted recommendation |
| AR-41 | Scope | Spike acceptance evidence | Source/fixtures under `spikes/<name>/` plus a findings entry with the exact command, observed result, and the codegen rule confirmed or changed; spec changes need AR-31 approval | ✅ Resolved — user accepted recommendation |
| AR-42 | Scope | Standard-library warning purity | The stdlib slice compiles with zero diagnostics under both profiles; a spec test enforces it | ✅ Resolved — user accepted recommendation |
| AR-43 | Scope | Corpus compiles with zero diagnostics | Yes; the corpus harness fails on any diagnostic | ✅ Resolved — user accepted recommendation |
| AR-44 | Technical | Compiler-internal spec-test granularity | Primary spec tests are the golden suites; per-stage unit ST cases only where a stage behavior is not covered by a golden case | ✅ Resolved — user accepted recommendation |
| AR-45 | Process | Post-plan session hygiene | Author the document set, hand off a summary; execution stays a separate `exec-plan` run | ✅ Resolved — user accepted recommendation |
| AR-46 | Technical | How the stdlib materializes views from owned storage (GF16/GF23) | Stdlib declares compiler-known `intrinsic func` helpers (`rawSlice<T>(p: raw T, n: usize): T[]`, `rawStr(p: raw u8, n: usize): str`) lowered by codegen to the view structs; used by Buffer/String/Span helpers | ✅ Resolved — user accepted recommendation |
| AR-47 | Interface | Stdlib additions required by the corpus | Expand the AR-15 slice: `std.io` gains `File`, `openFile`, `createFile`, `readFile`, `writeBytes`, `MODE_OLDFILE`, `MODE_NEWFILE`; `std.amiga.dos` gains `delayTicks(ticks: u32): void` (hosted-amiga only, E11200 under host) | ✅ Resolved — user accepted recommendation |
| AR-48 | Behavioral | Corpus temporary-file cleanup | The corpus runner deletes `corpus-in.txt`/`corpus-out.txt` after each leg; no `deleteFile` API | ✅ Resolved — user accepted recommendation |

## Batch 3 — authoring-time items (user answers, 2026-09-18)

Surfaced while authoring 03-05/03-07; resolved before the dependent documents
were written (surface-during-authoring rule).

| # | Category | Ambiguity / Gap | Accepted resolution | Status |
|---|----------|-----------------|---------------------|--------|
| AR-49 | Behavioral | `Array<T>` element drops: appendix §3 says elements drop first-to-last, but v1 has no drop glue (X2) | `Array<T>` accepts duplicable element types only; a non-duplicable instantiation is E90000. Owning-element drop glue is deferred with the v1.1 Box work | ✅ Resolved — user accepted recommendation |
| AR-50 | Technical | `xs.len()` on `Array<T>` (appendix §3 names the `len` intrinsic, which cannot know stdlib layouts) | The compiler-known `len` intrinsic lowers on `Array<T>` to its documented `count: usize` field; views/fixed arrays keep their specified behavior | ✅ Resolved — user accepted recommendation |
| AR-51 | Behavioral | How `slice` reports out-of-range bounds (appendix §8/IX-1 say E11211) | `slice` is a compiler-known `intrinsic func`; codegen emits the E11211 guard. `rawSlice`/`rawStr` remain unsafe unchecked primitives (AR-46) | ✅ Resolved — user accepted recommendation |
| AR-52 | Naming | The appendix uses both `module Std.Mem;` and `import ... from std.mem;`; identifiers are case-sensitive | Stdlib modules declare lowercase dotted names (`module std.mem;`), matching every import example. The appendix's `Std.Mem` line is a known editorial defect; correcting the spec text needs a separate ruling (AR-31) | ✅ Resolved — user accepted recommendation |
| AR-53 | Interface | Appendix §1 says allocation returns `T?`; appendix §5/§9 list builders as returning plain `String` | Low-level allocators return nullable (`bufferNew: Buffer?`, `bufferFrom: Buffer?`); convenience builders (`text`, `toHex`, `toDecimal`, `padLeft`, `padRight`, `stringOf`, `clone`) return `String` and panic on OOM (E1). Template builders panic on OOM inside the shim | ✅ Resolved — user accepted recommendation |

## Batch 4 — runtime items (execution checkpoint, 2026-09-18)

Surfaced by the Phase 0 corpus hand-check and the independent phase review;
resolved at the 0.2.2 checkpoint before any compiler work.

| # | Category | Ambiguity / Gap | Accepted resolution | Status |
|---|----------|-----------------|---------------------|--------|
| AR-54 | Technical (runtime) | The corpus contract asked `string-parser` to print `hex=0x00003A` via `padLeft(toHex(u32(sum)), 8, '0')`, but the frozen `toHex` already returns the `0x` prefix, so the expression yields `00000x3A` | The program prints `hex=0x3A` via `toHex(u32(sum))`; `padLeft` stays in the stdlib surface without a dedicated corpus behavior. Plan 03-01 and the expected output were amended; no specification change | ✅ Resolved — user chose option A |
| AR-55 | Behavioral (runtime, specification amendment) | `removeHandler(t: HandlerToken)` passes a duplicable struct to an unmarked read-only lend, which the original W10001 rule (golden `W10001_needless_lend.clo`) made a warning, contradicting the zero-diagnostic corpus contract (AR-43) | W10001 is narrowed to **writable** lends of duplicable values (writes affect only the copy); read-only lends copy silently. Amended: Ch 03, Ch 06, Ch 14 registry, Ch 16 §2/§5/§7, ledger D9, the lineage D9 row, and the `W10001_needless_lend.clo` golden. The corpus compiles clean | ✅ Resolved — user chose option A (spec amendment approved) |

## Batch 5 — runtime items from the Phase 1 spikes (2026-09-18)

Surfaced by the four accepted spikes and ruled on at the 1.5.2 checkpoint.
All four are specification-text changes approved by the user (AR-31).

| # | Category | Ambiguity / Gap | Accepted resolution | Status |
|---|----------|-----------------|---------------------|--------|
| AR-56 | Technical (runtime) | S1: vbcc embeds the `#line` filename in its Hunk Debug data, but the SDK's `objdump`, `readelf`, and `gdb` cannot decode it, so the Ch 17 §11 debugging claim is unusable for vbcc | Record the limitation in Ch 17 §11: GCC legs are debuggable and mapped; vbcc emits the mapping but SDK tooling cannot decode it in v1. `#line` is still emitted and diagnostics name the `.clo` source | ✅ Resolved — user chose "record the limitation" |
| AR-57 | Technical (runtime) | S2: Ch 17 §3 maps a handle binding to the opaque `Cl<Name>` typedef, while the ABI table says the FFI prototype uses the contract-table C type (`File` -> `BPTR`) | Emit the contract-table ABI type directly for an FFI-mapped handle local (`BPTR`, `struct Library *`); reserve `Cl<Name>` for handles with no fixed C type. Align Ch 17 §3, §4, §7 and PF-047 | ✅ Resolved — user chose the ABI type |
| AR-58 | Technical (runtime) | S4: under `vamos` the `mathieeedoubbas` comparisons are not IEEE, so `!(f >= lo && f <= hi)` does not trap NaN; the vbcc leg makes `vamos` raise in `IEEEDPFix` | Emit `cl_isnan(f) || !(f >= lo && f <= hi)`, where `cl_isnan` is a shim IEEE bit-pattern test. Amend Ch 17 §10, §12, §14 | ✅ Resolved — user chose the `cl_isnan` bit test |
| AR-59 | Technical (runtime) | S4: GCC float programs need `-lm` and vbcc float programs need `-lmieee`; the Ch 17 §13 matrix lists neither | Always append `-lm` (GCC) and `-lmieee` (vbcc) to the Amiga leg commands. Amend Ch 17 §13 | ✅ Resolved — user chose always-on flags |

## Batch 6 — naming amendments during execution (2026-09-18)

| # | Category | Ambiguity / Gap | Accepted resolution | Status |
|---|----------|-----------------|---------------------|--------|
| AR-60 | Naming (execution amendment) | The compiler binary was named `clorc`; the user requested the shorter, DX-friendly `clc` ("Clorraine Compiler") | Rename the binary to `clc`: Cargo `[[bin]] name = "clc"`, `CARGO_BIN_EXE_clc`, and every CLI sample updated. Supersedes the binary-name fields of AR-6, AR-20, AR-21, and AR-24 | ✅ Resolved — user directed the rename |
| AR-61 | Naming (execution amendment) | The source extension was `.clor`; the user requested a short, DX-friendly extension. `.cl` (Common Lisp/OpenCL) and `.cr` (Crystal) were rejected as misclassified by host tooling, and `.clo` was found to have no verified file-type collision | Rename the extension to `.clo`: all 115 `spec/tests/**` sources renamed, `spec/check.sh` globs, spec examples, plan docs, spike fixtures and evidence, and the living docs updated. Supersedes the extension references in AR-23, AR-44, and the suite READMEs | ✅ Resolved — user chose `.clo` |
| AR-62 | Technical (runtime) | The module stage needs the stdlib export surface, and the W10006 golden imports `Buffer` from `std.mem`, but the stdlib sources are Phase 7 work and `stdlib/` does not exist yet; the design documents define the embedded table as `include_str!` of the six sources | `compiler/src/stdlib.rs` holds a static table of the six stdlib module names and their exported names (from 03-07 and the appendix), used by loading, import resolution, and the unused-import warning. Phase 7 replaces the table body with `include_str!` of the real sources behind the same interface | ✅ Resolved — user chose the static export-name table |

## Batch 7 — runtime items during Phase 4 execution (2026-09-19)

Surfaced while authoring the Phase 4 specification tests (task 4.1.1).
Both were ruled on by the user on 2026-09-19 (option A in each case).

| # | Category | Ambiguity / Gap | Options Presented | Status |
|---|----------|-----------------|-------------------|--------|
| AR-63 | Technical (runtime) | `String` scoping. The compile-fail goldens use bare `String` with no import (16 files; the first to be asserted is `E10500_nonformattable_placeholder.clo`), and Ch 11 says "`String` is built in (T2)". But Ch 02 §4 lists `String` as a named type, 03-07 declares `export handle String freedBy FreeString;` in `std.text`, the AR-62 export table lists `String` under `std.text`, and every corpus program imports it. The name layer reports `E10219` for bare `String`, so a later-stage golden that uses it bare can never reach its intended diagnostic | **A**: treat `String` as a compiler-predeclared handle type visible in every module (with its `FreeString` deleter as the T2a declaration), keep the `std.text` export, and accept the now-redundant import; no oracle change. **B**: keep `String` a `std.text` export only and add `import { String } from std.text;` to the affected goldens (oracle change, AR-31). **C**: defer the bare-`String` later-stage goldens | ✅ Resolved — user chose A (built-in `String`) |
| AR-64 | Technical (runtime) | ST-TYPE-3 in `07-testing-strategy.md` assigns `1 && true` to `E10222`, but Ch 04 §6 lists `&&` among the constructs that require a `bool` condition, otherwise `E10218`; the plan artifact and the frozen specification disagree | **A**: follow the specification — a non-`bool` `&&` operand is `E10218` — and correct the ST-TYPE-3 row in `07-testing-strategy.md`. **B**: follow the plan row — `1 && true` is `E10222` | ✅ Resolved — user chose A (spec is authoritative; ST row corrected) |

## Batch 8 — runtime items during Phase 5 execution (2026-09-19)

Surfaced while authoring the Phase 5 specification tests (task 5.1.1).
Ruled on by the user on 2026-09-19.

| # | Category | Ambiguity / Gap | Options Presented | Status |
|---|----------|-----------------|-------------------|--------|
| AR-65 | Specification amendment (runtime, golden correction) | The Phase 5 red phase shows four compile-fail goldens reporting an earlier-stage code instead of their annotated ownership/duplicability code, so the red state is not clean. `E10025_use_partially_moved.clo` declares `observe(s: String)` but calls `observe(p)` with `p: Pair`, so `E10200` masks the intended partial-move `E10025` and the golden's own trigger ("whole-value use of a partially moved struct") is unreachable. `E10031_tocstring_mutation.clo` and `E10031_view_widening_mutation.clo` call `stringOf`/`toCString` with no `import … from std.text;`, so `E10219` masks `E10031` (MOD-4). `E11106_nonduplicable_global.clo` has the same missing import and was unasserted | **A**: correct the goldens — change `E10025`'s parameter type to `Pair`, add the missing `std.text` imports to the two `E10031` files and `E11106`, and assert `E11106`. **B**: keep the goldens frozen and record them as excluded/deferred. **C**: change the resolver to predeclare the stdlib names (rejected: MOD-4 requires imports) | ✅ Resolved — user chose A |
| AR-65a | Implementation correction (runtime, same batch) | `E10602_pointer_arithmetic_outside_unsafe.clo` reports `E10222` ("operator '+' is not defined for 'raw u32'") because the type layer rejects raw pointer arithmetic. Ch 02 §7 and Ch 04 §7 make `p + n`, `p - n`, and `p[i]` legal operations that require `unsafe`; the diagnostic is `E10602`, so the type layer must admit them and the unsafe check must report outside `unsafe` | **A**: fix the type layer to type `raw T` arithmetic as `raw T` and let the unsafe-context check report `E10602`. **B**: defer the golden. This is not a golden defect | ✅ Resolved — user chose A |

## Batch 9 — runtime item during Phase 6 execution (2026-09-20)

Surfaced while implementing the code generator (task 6.2.1). Ruled on by the user on 2026-09-20.

| # | Category | Ambiguity / Gap | Options Presented | Status |
|---|----------|-----------------|-------------------|--------|
| AR-66 | Technical (runtime) | The code-generation contract (Ch 17 §2) and `03-06-codegen.md` describe "the typed control-flow IR", but the Phase 5 IR is untyped: `ir::Value` and `ir::Place` carry no type, and `ir::Function` carries no return type, parameter types, or local declarations. The type facts live only in `sema.expr_types`, keyed by AST-node address, which IR values do not reference, so the emitter cannot declare locals or choose guards and casts from the IR alone | **A**: make the IR genuinely typed — thread each value's and place's `Ty` (and the function signature facts) through `lower_value` and the function builder, so codegen translates the typed IR and ownership and emission cannot disagree. **B**: keep the IR untyped and have the emitter re-walk the AST with `sema` in lockstep with IR control flow | ✅ Resolved — user chose A (typed IR) |
| AR-66a | Implementation correction (runtime, same batch) | The Phase 5 IR stores `Drop`/`Defer` inline with no scope boundary, so an emitter translating the IR cannot build the Ch 17 §4 cleanup ladder (one label per owning scope, innermost first) | Add cleanup-scope structure to the IR: `ScopeId`, `ScopeInfo { parent, in_loop }`, scoped `Drop`/`Defer`, and a `ScopeEnd` marker at normal fall-through scope ends; the checker assigns scope ids and tags every cleanup. Keeps the ownership semantics | ✅ Resolved — implemented (IR scope structure) |
| AR-67 | Technical (runtime) | Implementing AR-66 exposed a conflict: the Phase 5 IR is a flat control-flow graph, so translating it directly emits every block as a label plus `goto`, which Ch 17 §1 forbids ("`goto` for cleanup ladders only"). Ch 17 §2's statement list (`branch`, `switch`) describes structured statements, not a block graph | **A**: add structured-control metadata to the IR (if/loop/switch descriptors over the existing blocks) and emit structured C `if`/`while`/`switch`, keeping the CFG for the ownership fixpoint and `goto` for the ladder only. **B**: recover structure in the emitter from the reducible CFG. **C**: accept the `goto` control flow and record a deviation | ✅ Resolved — user chose A (structured IR metadata) |

## Batch 10 — runtime item during Phase 7 execution (2026-09-21)

Surfaced while starting the standard-library slice (task 7.2.1). Ruled on by the user on 2026-09-21.

| # | Category | Ambiguity / Gap | Options Presented | Status |
|---|----------|-----------------|-------------------|--------|
| AR-68 | Technical (runtime) | The plan's standard-library surface cannot be written in the frozen language. `03-07-stdlib.md` declares `export struct Array<T>` and `export func arrayNew<T>`/`push<T>`/`get<T>`/`set<T>`/`reserve<T>`, but the grammar forbids exported generics (`grammar.ebnf.md:532`, G4; E10302) and its `intrinsic_decl` has no type parameters (`:158`), so AR-46's `rawSlice<T>`/`rawStr` and AR-51's generic `slice` are unparseable and undeclarable. `ffi_decl` also has no link-name (`:156`), so a stdlib `ffi` function can only bind a C symbol of the same name. Meanwhile AR-62 requires real `stdlib/*.clo` sources and the appendix counts the library as ordinary Clorraine | **A**: treat the embedded standard library as a compiler-known surface — real `stdlib/*.clo` bodies whose generic declarations need no `export` (visibility comes from the AR-62 export table), `intrinsic func` gains optional type parameters for the four compiler-known helpers, and stdlib `ffi` declarations name the shim C symbols directly, retiring the hard-coded `cl_<name>` call map for stdlib-owned names; no frozen-spec text or golden changes and E10302/G4 still apply to user code. **B**: amend the frozen grammar and chapters for exported generics, generic intrinsic declarations, and an `ffi` link-name, updating E10302/G4 and their goldens. **C**: keep a declarations-only stdlib and define the emitted `cl_*` symbols in the shim, without compiling bodies | ✅ Resolved — user chose A (compiler-known stdlib surface) |

## Batch 11 — runtime item during Phase 8 execution (2026-09-21)

Surfaced while adding the hosted-amiga vbcc leg (task 8.2.2). Ruled on by the user on 2026-09-21.

| # | Category | Ambiguity / Gap | Options Presented | Status |
|---|----------|-----------------|-------------------|--------|
| AR-69 | Technical (runtime) | The emitted C marks unused parameters, unread hoisted locals and temporaries, and private-function references with `(void)x;`. That idiom is silent under GCC `-Wall -Wextra -Werror` and host `cc`, but vbcc 0.9 reports `warning 153: statement has no effect` for every occurrence, so the vbcc leg is not warning-free as `03-01-corpus.md` requires (R17). The normative vbcc command line is frozen (`spec/17-codegen.md:360`) and carries no `-Werror`, so the fix must be in the emitted C | **A**: replace every marker with `if (sizeof(x)) { }`, and private functions with `if (sizeof(&fn)) { }` — uniform, type-agnostic, silent under m68k GCC 16.2, host cc 12.2, and vbcc 0.9, with no casts and no self-assignment. **B**: use `x = x;` for variables and a same-typed local function pointer self-assigned in the entry stub for functions — portable but more emitter code and less uniform. **C**: keep `(void)x;` and accept the vbcc warnings, ruling that the `warning-free` phrasing is not binding for the vbcc leg | ✅ Resolved — user chose A (`if (sizeof(x)) { }`) |

## Batch 12 — runtime item during Phase 8 execution (2026-09-21)

Surfaced while running the full compile-fail scope (task 8.3.1). Ruled on by the user on 2026-09-21.

| # | Category | Ambiguity / Gap | Options Presented | Status |
|---|----------|-----------------|-------------------|--------|
| AR-70 | Technical (runtime) | Running the full compile-fail scope shows five goldens are silently "pending" (skipped with the vague reason "not covered by the implemented front end yet") rather than asserted or honestly excluded. Two are explicitly in scope in the ST-CF table but unimplemented: `E90001_comptime_staged.clo` (ST-CF-STAGED; a `comptime` body is accepted, so E90001 is never reported) and `E11100_initializer_cycle.clo` (ST-CF-MOD; no program-wide global-initializer dependency graph exists, so a same-module `let a = b + 1; let b = a + 1;` cycle is accepted). Two more are in scope and already work but are missing from the asserted list (`E11200_unimplemented_profile`, `E11201_unknown_profile`; ST-CF-PROFILE). The fifth, `E10400_capturing_lambda_as_func.clo`, is unreachable until lambda captures land (AR-10 stages lambda semantics) but lacks a `// stage:` annotation | **A**: implement E90001 (reject a `comptime` body) and E11100 (build a program-wide global-initializer dependency graph and report a cycle); add E11200/E11201 to the asserted list; exclude E10400 with an explicit staged reason. Pending becomes zero and the full ST-CF scope is asserted. **B**: implement E90001, assert E11200/E11201, and reclassify E11100 and E10400 as explicitly skipped with reasons (honest report, but ST-CF-MOD stays unmet). **C**: assert only E11200/E11201 and reclassify E90001, E11100, and E10400 as skipped with reasons (fastest; two in-scope diagnostics remain unimplemented) | ✅ Resolved — user chose A (implement E90001 and E11100) |

## Complexity Escalation packet (AR-24)

```text
Original goal: run the four golden test suites (compile-fail, run-fault, corpus,
  cross-toolchain) for the compiler MVP on every verify, reproducibly and offline.
Extra system or support code: a dedicated harness/xtask crate or binary with
  subcommands, golden management, configurability, and CI-driven orchestration.
Why it may be needed: centralized orchestration and reporting for four suites across two
  profiles and three runners (host cc, vamos, FS-UAE) could be more maintainable.
Evidence: no CI and no Rust test infrastructure exist yet; the repo precedent is
  spec/check.sh (bash) plus Makefiles; every golden file declares its expectation on line 1.
Smallest solution that still works: std-only Rust integration tests in compiler/tests/
  spawning the built clorc binary (now `clc`, AR-60), discovering golden files from ../spec/tests/, with thin
  Make targets delegating cross-toolchain legs to docker/Makefile and docker/scripts/run.sh.
Extra cost: a second build/orchestration surface, duplicated orchestration, and
  configuration no current requirement uses.
Independent verdict: Unnecessary (challenger), with the refinement that suite tests must
  spawn the CLI, not call the library in-process, so the spec-defined CLI output stays the
  oracle.
Direct user decision: chose the smaller solution A (2026-09-18).
```

## Resolution notes

- AR-1..AR-9: resolved by the user's answers on 2026-09-17.
- AR-10..AR-23, AR-25..AR-45: bulk-accepted by the user on 2026-09-18.
- AR-24: direct user choice on the visible complexity packet (smaller solution).
- AR-46..AR-48: resolved by the user's answers on 2026-09-18 (authoring-time
  items, surfaced before any plan document was written).
- AR-49..AR-53: resolved by the user's answers on 2026-09-18 (authoring-time
  items, surfaced while writing 03-05/03-07, before the dependent text).
- AR-54..AR-55: runtime items surfaced by the Phase 0 corpus hand-check and
  the independent phase review, adjudicated by the user on 2026-09-18 at the
  0.2.2 checkpoint. AR-55 approved a specification amendment; the amended
  sentence set and the fix batch are recorded in `08-phase0-review.md`.
- AR-20 clarification (2026-09-18, post-gate user scope statement): the
  compiler is host-only; a compiler running on AmigaOS/LorraineOS and a
  self-hosted compiler are explicitly out of scope. Rust remains the
  implementation language; the clarification closes the only caveat raised in
  the language review. No new unresolved item.
- Grounding highlights: corpus defined in `spec/lineage.md:169-175`; spikes in
  `spec/preflight-report.md:121-126` and `decisions.md:330,336,370`; Rust is the
  already-pinned implementation language (`docker/Dockerfile:136-141`,
  `docker/README.md`); run-fault cases in `spec/tests/run/`; compile-fail
  conventions in `spec/tests/compile-fail/README.md`; stage annotations in
  E10306, E10401-E10404, E90002; view construction and intrinsic declarations in
  `grammar-deltas.md:156,163` and `08-arrays-strings.md:92-96`; codegen contract
  in Ch 17; profile facts in Ch 15.
