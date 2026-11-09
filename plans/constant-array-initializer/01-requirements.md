# Requirements: Constant Array Initialization (PLAN-25)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Revision**: final — requirements and plan preflight findings resolved

## Feature Overview

This feature continues the measurement-first generated-C optimization programme.
The existing benchmark harness selects one hot emitted shape; the feature then
lands one emitted-code change with a measured win, no scored regression, and no
language, standard-library, ABI, runtime-shim, or toolchain change.

The selected shape is **a module-level `let` fixed array with a complete,
explicit list of guard-free constant scalar elements**. The current emitter
writes the global's empty state and then one store per element in
`cl_init_<module>`. For the `lookup` benchmark's 4,096-entry `i32` sine table,
that is 4,096 m68k store instructions and a 24,464-byte `.text` reduction when
they become one read-only template plus a copy.

A semantics-preserving probe replaces the stores with one block-scope
`static const` template and one `memcpy` at the same initialization point. The
probe was re-run on the committed `benchmarks/lookup/program.clo` with the
current compiler (a block-scope template inside `cl_init_bench_Lookup`, the
exact emitted store expressions, direct `vamos --instr-trace`, and
`m68k-amigaos-size`):

| Metric | Current emission | Template + `memcpy` | Delta |
|---|---:|---:|---:|
| `lookup` raw m68k instructions | 17,025 | 12,842 | -24.6% |
| `lookup` m68k `.text` (`-O2`) | 117,440 | 92,976 | -20.8% |
| Program output | `lookup: 134217728` | `lookup: 134217728` | equal |

The harness subtracts the 3,372-instruction calibration before scoring, so the
net percentage instruction delta is larger (-30.6%); the absolute delta is
unchanged at 4,183 instructions. The probe is exploratory evidence. Phase 1
re-runs the measurement with the committed benchmark and the harness, and the
final requirement is recorded in `05-findings.md`.

## Scope Boundary

In scope: the baseline audit; the `lookup` benchmark; the constant-array data
template emission; behaviour and emission tests; the measurement evidence; the
required phase reviews; and the documentation/roadmap closeout.

Out of scope: language, grammar, diagnostic, ABI, runtime-shim, and standard
library changes; new language features; DX, installer, and LSP work; Amiga
wrapper expansion; v2 language work; v1.1 `Box<T>` and trampoline work;
compiler compile-time speed; host wall time; linker or toolchain flag changes;
whole-program dead-code elimination; safety trade-offs; unchecked bounds; trap
removal; new external crates; new build scripts; and new specialist agents.
The Scope Boundary is authoritative; the candidate table below is illustrative,
not exhaustive.

## Two Comparisons, One Vocabulary

This feature uses two deliberately separate comparisons. “Head” always means
the final feature head.

- **Cumulative comparison:** base `v1.1.0` → final head. The compiler source
  necessarily differs because the nested-fill and volatile-fill work landed
  between the tag and this feature. Those differences are expected and are
  listed in `05-findings.md`; the cumulative table answers the release-level
  question the user asked.
- **Isolated comparison:** pre-change head (the benchmark commit) → final
  head. The only compiler-source difference allowed is this feature's codegen
  change. The isolated documents are `build/perf/isolated-head.json` and
  `build/perf/head.json`; the isolated comparison is the attribution and
  no-regression gate.

The plan-docs and benchmark commits must change no compiler source relative to
their clean parent. A compiler-source difference between the pre-change head
and the final head outside this feature's recorded codegen change is a blocker.
A compiler-source difference between `v1.1.0` and the final head is expected
for the cumulative table and must be enumerated.

## Functional Requirements

### R1 — Measurement audit and committed benchmark

- **R1.1 Clean baseline.** Benchmark only a clean tree. The plan documents and
  the benchmark are committed before the baseline run. The base compiler is
  the release tag `v1.1.0`; the isolated base is the benchmark commit, which
  changes no compiler file; the final head is the feature head.
  `git status --porcelain` is empty for every measured run.
- **R1.2 Pre-change gate.** Run `make check` on the clean baseline before
  changing a compiler file. A failure blocks the feature.
- **R1.3 Committed benchmark.** Add `benchmarks/lookup/` with a 4,096-entry
  `i32` sine table, a deterministic `main` that scans the table once, an
  `expected-output.txt`, and a README that explains the real workload. The
  sine samples are offset by 32768 to 1..65535 so the checksum is non-zero; a
  skipped, zeroed, or partially copied template changes the output. The program uses
  only `std.io`, runs on host and `hosted-amiga`, and does not change or tune
  the four existing scored programs. Update the `benchmarks/README.md` table,
  the “scored” count, and the baseline-location note.
- **R1.4 Baseline documents.** Remove stale `build/perf/base.json`,
  `build/perf/head.json`, and `build/perf/base-clc` before the run; never
  reuse a previous baseline. Run `make bench-base COMMIT=v1.1.0` and
  `make bench` on the benchmark commit, then copy the pre-change `head.json`
  to `build/perf/isolated-head.json`. The base document must record
  `compiler.git == v1.1.0^{commit}` and a `compiler.version` line containing
  `1.1.0` (the harness records the full `clc --version` line); the isolated
  and final head documents must record the benchmark and feature commits
  respectively. Build and retain the benchmark-commit compiler as
  `build/perf/isolated-base-clc`; the v1.1.0 `base-clc` is not the isolated
  base. Record the compiler path, SHA-256, commit, version, and `dirty` flag
  for each document. A path-only binary hash difference is recorded; an
  unexpected compiler-source difference is a blocker (see “Two Comparisons”).
- **R1.5 Attribution.** Record a per-benchmark table with base `v1.1.0`,
  isolated pre-change, final head, and cumulative values for
  `instructions_net`, m68k `-O2` `.text`, and emitted C bytes. Attribute the
  `lookup` delta to the initializer template; attribute any other cumulative
  row to the already-landed nested-fill or volatile-fill work. Run and record
  the isolated comparison exactly:
  `make -C docker exec CMD="python3 tools/perf/bench.py compare --require lookup:instructions:10 --require lookup:text:10 build/perf/isolated-head.json build/perf/head.json"`.
  The isolated comparison is green only when every per-run output matches and
  no scored benchmark exceeds the 1% no-regression guard.
- **R1.6 No benchmark tuning.** The existing scored programs' sources,
  expected outputs, run count, trace flags, and harness schema are unchanged.
  The new benchmark is added before the codegen change and is measured by the
  existing harness. Record `git diff <benchmark-commit>..<feature-head> -- benchmarks/`
  and the SHA-256 of each benchmark source and expected file in
  `05-findings.md`; the only benchmark change is the `lookup` addition.

**Acceptance:** `05-findings.md` holds the identity table, the baseline metrics,
the isolated and cumulative comparison tables and exact outputs, the `lookup`
attribution, and the benchmark-source diff/hash record.

### R2 — Constant-array data template

- **R2.1 Eligibility.** Apply the data-template form only when all of the
  following hold:
  - the binding is a module-level **`let`** (a module-level `const` array with
    an explicit list is rejected by the front end with `E10224` today and is
    out of scope);
  - its declared type is a fixed array with exactly one extent, the extent is
    known and greater than zero, and the array has at least 16 elements and at
    least 64 bytes under the checked type table (`usize`/`isize` are 32-bit
    typedefs on every profile; the threshold is not target-width-dependent).
    Phase 1 measures the small-table boundary and finalizes the lower bound
    before Phase 2 sizes the tests; the 4,096-entry `lookup` table qualifies;
  - the leaf element type is a 2- or 4-byte non-float scalar: `i16`, `i32`,
    `u16`, `u32`, `usize`, or `isize` (an explicit list; 1-byte `i8`/`u8`/
    `bool` elements are excluded because the measured u8 template cost is
    +55/+107/+211 instructions at 64/128/256 bytes, above the 1% guard);
  - the initializer is an array literal with an explicit element list, no
    fill form, and **`elements.len() == extent`**. A short or empty list is
    excluded because the current stores leave an omitted tail at the global’s
    existing state, while an extent-sized template would zero-fill it; the
    tail policy is unmeasured. The extent-sized declarator prevents a
    short-template over-read, but it does not preserve a prior tail mutation;
  - every element passes a **guard-free, effect-free structural predicate**:
    an integer/`bool`/`char` literal; a cast between non-float scalar types
    (including a `bool` target) whose source is also non-float; or `Neg`,
    `BitNot`, or `Not` over an eligible operand. Binary operations (including `/`, `%`, and shifts),
    float-to-integer casts, conditionals, `Materialize`/deferred forms, calls,
    places, intrinsics, and logical short-circuit forms are excluded, because
    the current emitter may attach a runtime trap or an unevaluated operand to
    them. The value is then checked with `constant_integer` (width-correct
    wrap/mask rules);
  - the binding is not `volatile`;
  - the selected profile is `host`, `hosted-amiga`, or `hosted-amiga-13`.
    `freestanding` keeps the current stores because its runtime `memcpy` is a
    byte loop and its performance is not measured; this is a recorded profile
    limit, not a safety trade-off.
- **R2.2 Spelling.** In place of the per-element stores, emit exactly one
  block-scope `static const` template and one `memcpy` into the global:
  - the template is declared inside the same `cl_init_<module>` function,
    with an extent-sized declarator (`<element> <template>[<extent>]`), a
    `static const` qualifier, and an internal name using the reserved
    `__cl_ctmpl_` prefix and a per-function counter; the name must not equal
    the target global, any init local (`__cl_itmpN`), or any other symbol the
    function names;
  - the initializer is the exact per-element `store_text` rendering, eight
    values per line, so a large table stays well under C99’s 4095-character
    logical-line limit;
  - the copy is `memcpy(<global>, <template>, sizeof(<global>));`;
  - no per-element store for that binding is emitted.
- **R2.3 Target storage.** The global declaration keeps its existing `static`
  writable empty state, type, size, address, and qualifiers. The template is a
  separate `const` object. The change does not make the global `const`, does
  not move it to read-only storage, and does not change any exported or
  internal ABI.
- **R2.4 Exclusions.** Retain the current spelling (which may already be a
  fill compaction) for every reachable excluded shape, including: a volatile
  global; a multi-extent array; a fill form; a short or empty list; a
  non-constant element; a place copy; a call or conditional initializer; a
  struct, view, or float element; a `MIN / -1` or `MIN % -1` element; a
  float-to-integer cast; a logical short-circuit form; and a profile other
  than `host`, `hosted-amiga`, or `hosted-amiga-13` (that is, only
  `freestanding` is excluded). A handle/owned element is rejected by the
  front end (`E11106`) and is a compile-fail case, not a reachable store.
  Other front-end-invalid shapes — zero extent (`E10226`), a slice
  (`E10200`), and a module `const` array with an explicit list (`E10224`) —
  are compile-fail or eligibility-predicate cases.
- **R2.5 Order and effects.** The eligible list is guard-free and
  effect-free. The `memcpy` runs at the same position and in the same
  module/global order as the replaced stores. Initialization order,
  dependency order, later mutation, address-taking, and `unsafe` access remain
  unchanged. No runtime trap is removed: a shape that would emit a division,
  float-cast, or conditional guard is not eligible.
- **R2.6 C validity.** The emitted C is valid C99 for host `cc`, m68k GCC for
  `hosted-amiga` and `hosted-amiga-13`, vbcc, and the freestanding profile. A
  block-scope `static const` array with a constant initializer and a
  `memcpy` call are supported by every leg; `host`, `hosted-amiga`, and
  `hosted-amiga-13` emit the template, and the freestanding leg sees the
  unchanged stores. No new header, crate, or build script is introduced.
- **R2.7 Locality.** The change stays in the module-level global initializer
  path in `compiler/src/codegen/program.rs` and reuses the existing constant
  evaluator, type spelling, and `memcpy` spelling. The volatile check is made
  in `emit_global_init`, where the `GlobalInfo` flag is available, or is
  passed explicitly to the helper; the existing `emit_global_array_fill`
  signature alone does not carry it. Function-local array literals, array
  fills, and the one-extent/nested fill compaction are unchanged.
- **R2.8 No trap or safety change.** No runtime bounds check, division guard,
  float-cast guard, ownership action, or drop is removed or weakened. The
  template contains only values the compiler already proved constant and
  guard-free.

**Acceptance:** an eligible global emits one extent-sized template and one
`memcpy` and no per-element store; every reachable excluded shape keeps its old
spelling; every front-end-invalid shape is recorded as such; the program output
is unchanged on host and m68k.

### R3 — Tests

- **R3.1 Behaviour suite.** `compiler/tests/constant_array_initializer_spec_test.rs`
  covers at least:
  - `i32[32]` positive and negative tables, including `i32(2147483647)` and
    `i32(-2147483647)`;
  - `u16[32]`, `u32[16]` including `u32(4294967295)`, `i16[32]`,
    `usize[32]`, and `isize[32]` (2/4-byte template cases);
  - `u8[64]`, `i8[64]` including `i8(-128)` and `i8(127)`, and `bool[64]`
    tables, plus a `bool(1)` scalar-to-bool cast (1-byte old-store cases with
    the correct output);
  - typed wrapping casts such as `u8(300)` → 44 and `i8(200)` → -56 (1-byte
    old-store cases);
  - an address-taken `i32[32]` table mutated after initialization;
  - a cross-module reader and a later initializer that reads the table;
  - a table initialized from a place (old stores, correct output);
  - a fill form, a volatile table, a multi-extent table, a short list, and an
    empty list, each with a qualifying extent (old stores, correct output);
  - a struct, view, or float element with a qualifying table (old stores,
    correct output); a handle/owned element is an `E11106` compile-fail case;
  - a below-threshold `i32[8]` control (old stores, correct output);
  - call and conditional elements (`f()` and `c ? 1 : 2`) with a qualifying
    table (old stores, correct output, one evaluation);
  - the guard-bearing exclusions `min / neg` and `min % neg` (module-level
    `let min: i32 = i32(u32(0x80000000)); let neg: i32 = -1;`, so the strict
    build has no constant-overflow warning), `i32(f32(1.5))` (in-range, output
    1), `i32(f32(3.0e9))` (`E11212`), `false && f()`, and `false && (x > 0)`
    (the same guard/fault as the base compiler);
  - compile-fail/eligibility cases for zero extent (`E10226`), a slice
    (`E10200`), a handle/owned array (`E11106`), a module `const` explicit
    list (`E10224`), and an out-of-range untyped literal (`E10202`).
  Run cases compile, build, run, and match an expected output that the base
  compiler also produces. The division guard/trap cases (ST-24, ST-26, ST-31)
  assert the same emitted guard and the same fault as the base compiler rather
  than a normal output; the short-circuit cases (ST-27, ST-28) are
  normal-output exclusions that prove the call/read is not evaluated.
  Front-end-invalid cases assert the diagnostic and do not run. The suite supports the existing `RDC_CLC` seam so
  the same cases can run against the pre-change compiler for the red/green
  record; a relative `RDC_CLC` value is resolved against the workspace root.
- **R3.2 Emission suite.**
  `compiler/tests/constant_array_initializer_impl_test.rs` pins:
  - the extent-sized `static const` template inside `cl_init_<module>`, its
    `__cl_ctmpl_<n>` name (first `__cl_ctmpl_0`, then `__cl_ctmpl_1`), and its
    eight-values-per-line wrapping;
  - the exact typed values for `-1`, `i32(2147483647)`, `i32(-2147483647)`,
    `u16(255)`, `u32(4294967295)`, `usize(7)`, `isize(-7)`, and a positive
    value;
  - the `memcpy` with `sizeof(<global>)`;
  - the absence of per-element stores;
  - the target declaration remains `static`, writable, and the same type/size;
  - the `memcpy` position in `cl_init` relative to the replaced stores;
  - two globals in one module use distinct templates; a template name does not
    collide with a global or an earlier `__cl_itmp` init local;
  - every reachable exclusion keeps the old stores and emits no template,
    including the 1-byte `i8`/`u8`/`bool` tables;
  - `host`, `hosted-amiga`, and `hosted-amiga-13` emit the template for a
    2/4-byte table, while `freestanding` emits no template.
- **R3.3 Red first.** The positive emission pins fail against the pre-change
  head compiler and pass on the feature head. Run them with the clean
  benchmark-commit compiler retained as `build/perf/isolated-base-clc` (or,
  for an independent check, the v1.1.0 `build/perf/base-clc`) through the
  existing `RDC_CLC` seam, and record the exact command, compiler identity,
  and failures. The behaviour suite’s expected outputs are recorded from the
  pre-change head compiler; the v1.1.0 compiler produces the same outputs.
  The suite must not depend on a gitignored baseline binary for its normal
  green run. The red tests are not committed as a standalone red commit; they
  stay uncommitted until the implementation makes them green, and the red
  evidence is recorded in `05-findings.md`.
- **R3.4 Test integrity.** Each positive pin must fail when the fast path is
  disabled; each exclusion guard must fail when its exclusion is removed while
  the shape otherwise qualifies. Spot-check the short-list, empty-list, fill,
  guard-free, volatile, profile, type, and threshold exclusions. Each
  assertion must fail without the fix.
- **R3.5 Sanitizers.** The behaviour suite carries the `RDC_ASAN` and
  `RDC_UBSAN` discovery markers and builds its C with the same sanitizer flags
  as the other behaviour suites. `make asan` and `make ubsan` include it.
  The exclusion cases exercise the division/float guards and the
  address-taken mutation path.
- **R3.6 Shape fixture.** Add a **module-level** eligible one-extent
  explicit-list non-volatile `let` scalar array (for example, `i32[64]`) to
  `tools/perf/fixtures/shapes.clo`, and have `main` read it. `make bench-shape`
  must build it with hosted-amiga (m68k GCC), hosted-amiga-13 (nix13), vbcc,
  and freestanding without warnings. The `hosted-amiga` and
  `hosted-amiga-13` legs (including the nix13 compile) exercise the template;
  the freestanding leg exercises the unchanged stores. The fixture’s table is
  large enough to exercise the eight-values-per-line wrapping. The behaviour
  suite (not `bench-shape`) covers host execution.
- **R3.7 Harness tests.** `make bench-test` passes. A targeted `make bench`
  (or `bench.py run --bench lookup --out build/perf/lookup-smoke.json
  --label lookup-smoke`) checks that the `lookup` expected output matches host
  and m68k and that the harness records a nonzero instruction count and linked
  `.text`. The targeted run must not overwrite `build/perf/head.json`; the full
  suite remains the comparison document. The targeted run is part of the
  acceptance for this requirement; `make bench-test` alone does not run the
  benchmark.

**Acceptance:** both new suites pass, the red/green record exists, the shape
fixture builds on every toolchain, the harness test suite is green, and the
targeted `lookup` run records the expected output and metrics.

### R4 — Measurement and acceptance

- **R4.1 Declared win.** The initial expectation is
  `lookup:instructions:10` and `lookup:text:10`. Phase 1 measures the small
  table boundary and the committed `lookup` deltas, and records the final
  threshold and requirement values in `05-findings.md`; a change from the
  probe is a ruling, not a silent edit. After the head commit, run `make bench`
  and
  `make bench-compare REQUIRE="lookup:instructions:10 lookup:text:10"`. Run
  the isolated comparison from R1.5 with the same
  `--require lookup:instructions:10 --require lookup:text:10` so the feature’s
  own win is gated, not only the cumulative release delta. If a review or
  sanitizer fix changes codegen after that measurement, rebuild `clc`, rerun
  `make bench`, rerun both comparisons and the emission audit, then rerun the
  affected gates on the clean fix commit; a stale head document blocks
  closeout.
- **R4.2 No regression.** No scored benchmark may regress by more than 1% in
  `instructions_net` or m68k `-O2` `.text` in either the isolated or
  cumulative comparison; every per-run output must match. The harness’s
  per-run host/m68k equality is the output check; the compare document’s
  `output_sha256` is a hash of the expected file, not of the actual run.
- **R4.3 Emission audit.** Diff emitted C from `build/perf/isolated-base-clc`
  and the head `clc` over the following explicit matrix:
  - every corpus program with the profile its run uses, including the
    `freestanding` corpus program with `--profile freestanding` and the
    portable programs with both `host` and `hosted-amiga`;
  - every benchmark program (five scored plus calibration) with both `host`
    and `hosted-amiga` (the harness emits both);
  - every `examples/amiga/*.clo` program with `hosted-amiga`; the A500/KS1.3
    compatible programs additionally with `hosted-amiga-13`.
  In the isolated diff every changed unit is the `lookup` initializer
  template; an unexplained diff blocks closeout. Record
  `git diff v1.1.0..<feature-head> -- benchmarks/` to show the cumulative
  benchmark addition, `git diff <benchmark-commit>..<feature-head> --
  benchmarks/` to show no tuning after the baseline, and the benchmark
  source/expected hashes (R1.6).
- **R4.4 Strict gates.** `make check`, `make asan`, `make ubsan`,
  `make corpus-amiga`, `make differential`, `make bench-shape`,
  `make bench-compare`, and `make verify` are green. The host-only GitHub
  workflow is not the acceptance gate.
- **R4.5 Clean delivery.** Commit the plan documents (including the
  requirements and plan preflight reports), the benchmark/tooling change, the
  baseline evidence, the implementation with its tests and fixture, the fix
  pass, and the closeout separately. The baseline evidence commit (the baseline
  table, threshold, and identity in `05-findings.md`) lands after the baseline
  run and before the implementation commit. The spec tests are written and
  run red before the implementation, but they are not committed as a
  standalone red commit: they land with the implementation commit, which is
  green. Stage explicit paths and review the staged diff before every commit.
  Push only when the full gate is green; after the push the tree is clean and
  `origin/main..HEAD` is 0. Never commit `build/`, `target/`, generated C,
  benchmark JSON, captures, media, secrets, or scratch files.

**Acceptance:** the final benchmark table shows the declared `lookup` win and
no scored regression in either comparison; every named gate is green; the tree
is clean and the branch is pushed.

### R5 — Documentation and roadmap

- **R5.1 Plan records.** `05-findings.md` owns the baseline/head identity, the
  isolated and cumulative tables, the red/green record, the emission audit, the
  accepted limits, the rejected candidates, the review findings, and the final
  gate evidence. The accepted limits include the read-only template data
  beside the writable global and the freestanding profile exclusion;
  `hosted-amiga-13` is included in the optimization.
- **R5.2 Command/docs.** Update `AGENTS.md` if the measurement boundary or
  commands change; update `benchmarks/README.md` for the new program, the
  scored count, and the baseline-location note; update
  `docs/known-limitations.md` for the freestanding multi-byte limitation and
  any other new limit.
- **R5.3 Roadmap.** Add one `PLAN-25` row to `plans/00-roadmap.md` as
  Executing when implementation starts (Phase 3) and Done with the measured
  evidence at closeout; update the roadmap progress count and the plan index
  `Status`/`Last Updated` consistently. The plan index moves from Planning
  Complete to Executing at Phase 3 and Done at Phase 6.
- **R5.4 Comment hygiene.** Do not reference this plan, its requirements, or
  any task/requirement identifier in source code or doc comments. Restate
  durable rationale in plain language.

**Acceptance:** the documents are current, the roadmap row and progress count
are correct, and the source contains no plan references.

### R6 — Independent phase reviews

- **R6.1 Review set.** Each codegen-changing phase gets a correctness reviewer,
  a read-only memory-safety auditor, a read-only emission-integrity auditor,
  and a performance auditor. The plan defines the codegen phase set; a fix
  pass that changes codegen is its own phase with the same four reviewers.
  The semantics reviewer is not required because the language rules and ABI
  are unchanged; add it if a review finds a semantic question.
- **R6.2 Packet contents.** Every packet includes the phase diff, the goal and
  smallest viable design, the active lenses, the blast-radius step (every call
  site, exhaustive match, and parallel path of the changed construct), the
  test-integrity step (an assertion must fail without the fix), the verify
  command and its last result, and the scope mode.
- **R6.3 Rulings.** Any critical or major finding stops execution for a ruling
  in every commit mode. Minor findings are fixed or recorded as accepted
  limits. The final focused re-review must be clean of critical/major findings.
  The acceptance count matches the plan’s codegen phases, not a fixed number.

**Acceptance:** every codegen-changing phase has a four-reviewer record; all
critical/major findings are resolved; the final focused review is clean.

## Non-Functional Requirements

- **N1 Determinism.** The benchmark, emitted-C diff, and measurement must be
  deterministic. No wall-clock threshold is used for acceptance; host time
  remains advisory.
- **N2 Compatibility.** The change preserves the frozen specification, the
  language, the emitted ABI, the profiles, the runtime shim, and the standard
  library.
- **N3 Performance discipline.** Only eligible constant-array initializers
  change; the existing scored programs’ emitted C remains unchanged apart from
  the already-landed prior changes.
- **N4 Maintainability.** The change is one local rule in the existing global
  initializer path, reuses existing helpers, and adds no abstraction, crate,
  build script, or external dependency.
- **N5 Safety.** No ownership, drop, bounds, trap, volatile, or guard behavior
  is weakened. ASan/UBSan cover the behaviour suite.

## Out-of-Scope Candidates

| Candidate | Reason |
|---|---|
| Static declaration initializer that skips the init function | Changes initialization timing and `unsafe` mutation of address-taken globals; a whole-program write/order proof is a larger surface |
| Linker `-ffunction-sections`/`--gc-sections` or other toolchain flags | A build/toolchain change, not generated-C codegen; breaks the harness’s same-flags comparison invariant and targets the out-of-scope runtime shim |
| Whole-program dead-function elimination | A call-graph/FFI feature, not a local emitted-shape rule |
| Function-local constant array literals | A separate declaration path and measurement; not needed for the committed benchmark |
| Fill forms, prefixes, multi-extent arrays, floats, structs, handle/view elements, owned elements | No measured need in this cut; separate semantics and tests; handle/owned arrays are `E11106` |
| Short or empty explicit lists | The current stores leave an omitted tail at the global’s existing state; an extent-sized template would zero-fill it. The tail policy is unmeasured, so the complete-list exclusion remains a scope choice, not an over-read guard |
| Module-level `const` arrays with explicit lists | Rejected today with `E10224`; supporting them is a checker/language change |
| Freestanding multi-byte template performance | The freestanding shim’s `memcpy` is byte-wise and not measured; the profile keeps the old stores |
| New specialist agents, new harness code/schema, runtime-shim changes, new crates/build scripts | Outside the confirmed goal and scope |
| Changing the 1% no-regression guard or the harness flags | The guard and flags are the measurement contract |

## Acceptance Summary

The feature is done when:

1. `lookup` is committed, deterministic, and measured with the existing
   harness; its non-zero checksum detects a skipped, zeroed, or partially
   copied template, while the behaviour/emission suites cover the exact
   values.
2. An eligible module-level `let` constant scalar array initializer emits one
   extent-sized `static const` template and one `memcpy`, with no per-element
   stores.
3. Every reachable excluded shape keeps its old spelling and behavior; every
   front-end-invalid shape is recorded as such.
4. The isolated and cumulative benchmark tables show the `lookup` win and no
   scored regression beyond 1%.
5. `make check`, `make asan`, `make ubsan`, `make corpus-amiga`,
   `make differential`, `make bench-shape`, `make bench-compare`, and
   `make verify` are green.
6. The emission diff and benchmark-source diff are attributed, every
   codegen-changing phase has the four reviews, and no critical/major finding
   remains open.
7. The roadmap row is Done, the plan index is current, the tree is clean, and
   the branch is pushed.
