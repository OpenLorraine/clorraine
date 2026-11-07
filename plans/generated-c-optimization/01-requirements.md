# Requirements: Generated-C Optimization (PLAN-14)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

This feature makes the compiler's generated C measurably better without
changing what a Clorraine program means. It starts with a benchmark harness
because PLAN-14 forbids optimizing on feel. The harness measures the emitted C
and the linked program on the host and on m68k, and it compares a base compiler
with a candidate compiler under explicit regression thresholds.

The baseline study selects two measured costs and two dead-condition
simplifications. It also rejects a plausible optimization that measured worse
on the target. The result is a small, evidence-backed programme rather than a
general optimizer.

## Functional Requirements

### R1 — Deterministic benchmark harness

- **R1.1 Commands and location.** Add `tools/perf/bench.py`, a Python 3
  standard-library-only harness with `run`, `compare`, and `test`
  subcommands. Add `make bench`, `make bench-fast`, `make bench-base`,
  `make bench-compare`, and `make bench-test`. `make bench` builds the
  workspace and runs the scored suite with tracing inside the pinned SDK
  container. It never needs a ROM, FS-UAE, an X display, or network access.
- **R1.2 Metrics.** Each run records, per benchmark and per measured build:
  emitted-C bytes and lines; host debug (`-O0 -g`) and release (`-O2`) binary
  bytes; host debug wall time (one run) and host release wall time (best and
  median of a fixed run count); m68k `-O0` and `-O2` `.text`, `.data`, and
  `.bss` bytes; the m68k `-O2` dynamic instruction count from a streamed
  `vamos --instr-trace` run; and the SHA-256 of the normalized program output.
- **R1.3 Calibration.** The suite includes one calibration program that only
  starts the runtime and prints one line. It is built with the same compiler,
  profile, and flags as the scored programs. Its instruction count is
  subtracted from every raw count; the JSON carries both `raw` and
  `net_instructions`. The calibration is not a scored benchmark.
- **R1.4 Deterministic result document.** The JSON result has a versioned
  schema, stable key order, and records the compiler identity (`clc` SHA-256,
  repository commit, `--version` output), the toolchain versions (host `cc`,
  `m68k-amigaos-gcc`, `vamos`, Python), the run count, the profile, the C
  flags, the output SHA-256, and the per-benchmark metrics. Two runs of the
  same compiler produce identical instruction counts, sizes, and output
  hashes. Timings and the time stamp are the only intentionally
  non-deterministic fields.
- **R1.5 Comparison gate.** `compare` loads two result documents, requires the
  same schema, toolchain identity, run count, and benchmark set, and checks
  every benchmark: the output SHA-256 must match; `net_instructions` must not
  grow by more than 1%; m68k `-O2` `.text` must not grow by more than 1%; and
  at least one benchmark must improve by 20% or more in `net_instructions` or
  10% or more in m68k `.text`. Host time is recorded and displayed as
  advisory, not gated, because it is noisy and not the target (AR-2). The
  command prints a table and exits non-zero on a failed hard check.
- **R1.6 Harness self-tests.** `make bench-test` runs `unittest` cases inside
  the pinned container, with no SDK toolchain beyond Python: JSON schema
  validation, delta arithmetic, threshold pass/fail, missing-metric failure,
  malformed-input failure, a fake compiler that fails to emit, a fake compiler
  that produces different output, and a two-run determinism case. `make check`
  runs `bench-test`, so the harness cannot rot silently.
- **R1.7 Fail loudly and bound every step.** A missing compiler, a failed
  emit, a failed C build, a program whose output differs from its expected
  file, a missing trace, an unparsable size line, or a timeout stops the run
  with a non-zero exit and the exact command that failed. Every subprocess has
  a timeout; the trace step streams and counts lines instead of writing a
  multi-hundred-megabyte log.
- **R1.8 No new dependencies.** Python standard library only. No pip package,
  no Node package, no Rust crate, and no new container image.
- **R1.9 Base-compiler mode.** `make bench-base COMMIT=<ref>` resolves the
  ref to a canonical 40-hex commit in the make target, forwards it through the
  container's environment passthrough, creates a detached git worktree under
  ignored `build/perf/`, builds `clc` at that commit, runs the same current
  harness and the same committed benchmarks with
  `--clc <base>/target/debug/clc --compiler-git <resolved> --label base`,
  writes `build/perf/base.json`, and removes the worktree. A stale worktree is
  removed with `git worktree remove --force` and `git worktree prune`. `make
  bench` writes `build/perf/head.json` with the head commit. `make
  bench-compare` compares them. This is the only supported base/head path.
- **R1.10 Harness contract.** The harness accepts `--clc`, `--label`,
  `--compiler-git`, `--out`, `--runs`, and `--no-trace`; it validates
  `--label` against a safe path component, refuses an `--out` or run directory
  outside `build/perf/`, validates benchmark names against the suite
  directory, uses argument-list subprocesses only (never a shell), and removes
  only the run directory it created. A user-supplied path never reaches a
  shell or a recursive delete.

### R2 — Representative benchmark suite

- **R2.1 Layout.** Add `benchmarks/<name>/program.clo`,
  `benchmarks/<name>/expected-output.txt`, and `benchmarks/<name>/README.md`
  for every benchmark, plus `benchmarks/README.md` for the suite. Programs use
  module names under `bench.*`.
- **R2.2 Coverage.** The suite has four scored programs plus the calibration:
  `sieve` (large one-extent fixed-array fill, scalar module `const` in loop
  bounds, nested loops, bounds checks), `matrix` (multi-extent fixed arrays,
  arithmetic, helper calls), `strings` (String building and parsing, `str`
  views, index and slice access), and `moves` (struct and fixed-array moves,
  generic instantiation, ownership cleanup).
- **R2.3 Program contract.** Each program prints a deterministic final
  checksum line built from fixed-width integers (`i32`/`u32`, never `usize`)
  and exits 0. The expected output is identical on `host` and `hosted-amiga`.
  The programs use no Amiga hardware, no file system, no clock, no randomness,
  and no command-line input.
- **R2.4 Size and time budget.** Each scored program finishes in at most 2
  seconds on the host release build, at most 15 seconds under `vamos` without
  tracing, and at most 300 seconds under the streamed trace. The suite is
  sized so `make bench` is a minutes-to-tens-of-minutes gate, not an
  hour-scale one. The harness enforces each budget with a timeout and records
  the duration.
- **R2.5 Fixture status.** The benchmark programs are tooling fixtures, not
  frozen-specification goldens. Their expected output is part of this
  feature's contract, and changing a benchmark's workload requires recording a
  new baseline in the evidence document.

### R3 — Baseline evidence and selection rule

- **R3.1 Baseline record.** Record the exploratory study in
  `02-current-state.md` and the committed baseline in `05-findings.md`. The
  exploratory numbers are labelled indicative; the committed baseline is
  recorded after the benchmark suite exists and before the first optimization,
  using `make bench-base` on the recorded commit and the committed programs.
  The exact commands and toolchain identity are part of the record.
  Machine-specific JSON stays under ignored `build/perf/`.
- **R3.2 Selection rule.** An optimization enters the plan only when the
  baseline shows at least one of: a named benchmark spends at least 5% of its
  `net_instructions` in the shape, the emitted C for one source statement
  exceeds 1,000 statements, or the linked `.text` for the shape exceeds 8 KiB.
  A provably dead emitted condition (a constant-false or type-impossible
  comparison that the source semantics already exclude) is also eligible as a
  dead-condition removal, because it is not a performance bet. The record
  states which trigger applies.
- **R3.3 Rejection is a result.** A candidate that measures worse on the
  authoritative metric is recorded as rejected, with its numbers. The plan must
  not ship a "theoretically better" change that regresses the target.
- **R3.4 Reopen trigger.** If a later benchmark shows the shape is no longer
  hot, or the C compiler already removes the cost, the owning optimization is
  reopened and may be removed.

### R4 — Scalar module `const` folding in function bodies

- **R4.1 Read folding.** A read of a module-level scalar `const` whose
  initializer folds to a constant value lowers to a typed literal inside a
  function body, including a local initializer and a lambda body. The scalar
  types are the integer types (`i8`, `i16`, `i32`, `u8`, `u16`, `u32`, `usize`,
  `isize`) and `bool`. Character literals have type `u8` and are covered by the
  integer rule. An integer or `bool` initializer folds as a constant expression
  at the const's declared integer width. A `f32`/`f64` const stays a place in
  this plan: the existing constant folder evaluates floats in `f64` and does
  not round an `f32` cast at each operation, so a folded float read can differ
  from its stored value. An enum-typed, `raw`, view, aggregate, or handle const
  is not folded either.
- **R4.2 Scope-gated read.** Folding applies only when the resolved place is
  a bare module-level binding whose `GlobalInfo` is a `const`. A local or
  parameter that shadows the name and a qualified read with a field or element
  projection stay places. A bare module-qualified const (`m.N`) is a global
  with no projection and may fold. The lookup is the same `place_target`
  resolution the checker uses.
- **R4.3 Address-taken exclusion.** A whole-program pre-pass records every
  module const whose address is taken by `&name` or `&qualified.name`. The
  walker visits every body-bearing item (`Func` and `Comptime`), every module
  initializer (`GlobalLet`, `GlobalConst`), and every lambda body, and every
  statement and expression child within them. A recorded const is never folded
  anywhere. This preserves the accepted `unsafe &N` plus raw-write behavior,
  which can change the storage between reads.
- **R4.4 Imported and expression initializers.** A const from another module,
  or a const whose initializer is an expression over other constants, folds by
  evaluating the initializer with a `ConstFolder` constructed for the
  **declaring module**, not the using module. A chain such as `const A = 5;
  const B = A;` in module `m` and a read of `B` in module `n` folds to `5`.
- **R4.5 Typed literal and typed fold.** An integer or `bool` initializer is
  folded with the const's declared integer width (the folder's typed
  `integer_in`/`integer_at` entry point), so every arithmetic operation wraps
  and every shift count masks exactly as the runtime does. A final wrap alone
  is insufficient because `1 << 32` must mask the count, not wrap the result.
  The folded value carries the declared type:
  - a non-negative integer is `ValueKind::Int` with the const's type;
  - a negative integer is a typed unary negation of its magnitude, so it
    renders through the existing signed-wrap spelling rather than a huge
    unsigned literal;
  - `bool` is `ValueKind::Bool`;
  - a `f32`/`f64` const stays a place; float folding waits for a folder that
    rounds at every `f32` operation;
  - an integer const whose initializer contains an `f32`/`f64` subexpression
    stays a place, because the folder does not round that cast;
  - a narrow arithmetic initializer (an operation whose operand type is below
    32 bits) stays a place, because C promotes the operation to `int` while
    the typed folder wraps at the operand width; a direct narrow literal or
    cast is still folded;
  - an integer const used where a narrower type is declared is emitted with
    that declared type's suffix and cast, exactly as a source literal.
- **R4.6 Storage unchanged.** The module binding keeps its current storage and
  run-time initializer. This plan does not make it `static const`, does not
  remove the initializer, and does not change initialization order. The
  emitted symbol remains available for any accepted address form.
- **R4.7 Module initializers excluded.** The separate module-initializer
  lowering is not changed; a read inside a module-level `let`/`const`
  initializer stays a place. This is an explicit boundary, not an accidental
  gap.
- **R4.8 Acceptance.** On the committed `sieve` benchmark, folding `N`
  reduces `net_instructions` by at least 20% at m68k `-O2` and does not
  regress any other benchmark by more than the R1.5 threshold.

### R5 — Compact large one-extent scalar fills

- **R5.1 Threshold and scope.** A one-extent scalar fill in statement position
  is emitted compactly when its covered slot count (`extent - prefix`) is at
  least 16 for a `u8`/`i8` element (the `memset` form) or at least 64 for any
  other scalar (the loop form; the loop saves code size but is slower than
  unrolled stores). A smaller fill keeps the per-element spelling. Nested-row
  fills, struct-literal fills, array copies, the deferred expression form, a
  field or volatile target, a `DerefTemp` loop alias, and a fill whose
  expression reads a named binding through the shared self-read path are
  excluded; a field target may name a volatile member, and a volatile array
  cannot be passed to `memset`.
- **R5.2 Forms.** A `u8` or `i8` fill with a constant integer value is emitted
  as one `memset` over the covered slots. Any other scalar element type,
  including `bool`, uses one `for` loop over the covered indices. `bool` is
  not byte-sized on the target, so it must not use `memset`.
- **R5.3 Prefix and extent.** `[a, b; fill]` keeps the two direct prefix
  stores and compacts only the remaining `extent - 2` slots. `[; fill]`
  compacts all slots. An empty covered range emits no loop and no `memset`.
- **R5.4 Evaluation and ownership.** The fill expression still evaluates
  exactly once when it is impure or store-needing; the existing
  once-evaluated temporary is copied to each covered element in the compact
  loop. A pure fill repeats its expression in the loop, which is semantically
  identical to the existing per-element spelling. The prefix stores,
  move-outs, and cleanup order are unchanged.
- **R5.5 Targets.** Compaction works for an array object, an adjusted array
  parameter, and a hidden fixed-array return slot. The byte count for an
  adjusted parameter uses the element count times the element size, never
  `sizeof` of the pointer.
- **R5.6 Valid C99.** The compact form uses only `memset`, a block, a `for`
  loop, `sizeof`, and existing typedefs. No compiler extension is used and no
  new runtime helper is added.
- **R5.7 Acceptance.** On the committed `sieve` benchmark, m68k `-O2` `.text`
  shrinks by at least 10% and emitted-C bytes shrink by at least 25%, with no
  benchmark's `net_instructions` growing by more than the R1.5 threshold.

### R6 — Non-negative constant index-guard simplification

- **R6.1 Dead half removed.** For a non-negative constant index into a
  slice or `str` view (a dynamic extent), the emitted bounds condition omits
  its `< 0` half. The upper-bound comparison remains. "Constant" means a
  literal, a `bool`/character value, a cast, a unary form, or a binary
  expression evaluated with the C semantics (wrap at the result type and mask
  shift counts); a non-constant expression keeps the full guard.
  This is the only new case: an unsigned index already omits the lower half,
  and a constant index into a fixed array in range is already unguarded.
- **R6.2 Signed and negative retained.** A non-constant signed index keeps the
  negative check. A negative literal into a fixed array is rejected by the
  existing semantic check; a negative literal into a slice or `str` is accepted
  and must remain a runtime `E11211` trap. The emitter does not add a new
  diagnostic.
- **R6.3 All guarded views.** The rule applies to a fixed array, a slice view,
  and a `str`/`static str` view through the shared guard builder. The
  fixed-array and unsigned forms are regression assertions, not new behavior.
- **R6.4 No manual upper-bound elimination.** This plan does not add a
  loop-bound analysis, a range analysis, or a fact database. The baseline
  showed that manually removing the upper checks made the m68k code slower
  after constant folding. The C compiler performs the elimination once the
  bound is a literal.
- **R6.5 Acceptance.** The emitted condition for `s[1]` no longer contains
  `< 0`; the fixed-array and unsigned forms are unchanged; slice/string
  negative and upper-out-of-range cases still trap with `E11211`; and no
  benchmark regresses beyond the R1.5 threshold.

### R7 — Constant division and remainder guard folding

- **R7.1 Non-zero constant divisor.** When the divisor is a constant integer
  other than `0` and `-1` (including a negative representation such as
  `-(1)`, a cast, a folded const, or a binary expression), the emitted
  zero-and-overflow guard is omitted. The constant value is evaluated with the
  C semantics (wrap at the result type and mask shift counts); a non-constant
  expression keeps the full guard. The operation still runs behind the
  language's wrapping rules.
- **R7.2 Minus one.** When the divisor is constant `-1`, only the
  `MIN / -1` overflow check is emitted; the zero check is omitted.
- **R7.3 Zero.** When the divisor is constant `0` — a literal, a binary
  expression, or a finite float cast that evaluates to zero — the existing
  behavior is unchanged: the value renderer emits the trap expression and
  `division_guard_condition` returns no separate `if` guard. The constant
  folder's `E10223` remains the compile-time diagnostic in a constant context.
- **R7.4 Non-literal divisor.** A run-time divisor keeps the full
  zero-and-overflow guard unchanged.
- **R7.5 Acceptance.** Every existing division-fault program still traps with
  the same code and exit status; the emitted guard for `x % 2` no longer
  contains `2 == 0`; a `MIN / -1` case still traps; and no benchmark regresses
  beyond the R1.5 threshold.

### R8 — Verification and acceptance

- **R8.1 Specification tests.** Add
  `compiler/tests/generated_c_optimization_spec_test.rs`. The suite is a
  green-before/green-after regression oracle for behavior, because R4–R7
  preserve program meaning: folded integer constants (including negative,
  imported, expression-initialized, and typed shift/wrap constants such as
  `1 << 31` and `1 << 32`); a shadowing local; an address-taken const with a
  raw write; a non-address-taken const; a float const that stays a place; a
  narrow arithmetic const that stays a place; a nested `&N` in a call
  argument, lambda, and `defer`; byte, integer, `bool`, prefixed, impure,
  adjusted parameter, and hidden-return-slot fills; a pure volatile place read
  per element and an impure fill evaluated once into a temporary; unsigned,
  signed, slice, and string index boundaries; constant zero, `-1`, and
  non-zero division; a wrapping binary divisor that keeps the full guard; and
  the trap cases. None of these edit a frozen `spec/tests` golden.
- **R8.2 Implementation tests.** Add
  `compiler/tests/generated_c_optimization_impl_test.rs` as the red-first
  driver for the emitted shapes: a folded const leaves the storage
  declaration and initializer but has no read; a 4,096-slot `u8` fill emits
  `memset`; a 4,096-slot `i32` fill emits one loop and at most the threshold
  number of direct stores; a 15-slot fill stays unrolled; a 16-slot fill
  compacts; the adjusted-parameter fill uses the element-count byte size and
  not `sizeof` the pointer; a small fill stays unrolled; `s[1]` has no `< 0`;
  a `% 2` guard has no zero check; determinism across two emits.
- **R8.3 Existing gates.** `make check`, `make asan`, `make corpus-amiga`,
  `make differential`, and `make verify` all pass. The test count grows by the
  two new suites; no existing suite is weakened or skipped. The new spec suite
  opts into `RDC_ASAN` for the fill and const cases, and `make bench-test` runs
  in the pinned container as part of `make check`.
- **R8.4 Benchmark gate.** `make bench-base COMMIT=<recorded>` then
  `make bench` then `make bench-compare` passes R1.5. The evidence table
  records the base and head values for every benchmark and every metric.
- **R8.5 Emission integrity.** Base-versus-head emitted C over the corpus is
  reviewed against a retained base `clc` binary. Every difference is
  attributable to one of R4–R7. Every accepted unit builds warning-free with
  the host strict flags and m68k GCC; a focused shape fixture (byte fill,
  integer fill, prefixed fill, hidden-return fill, slice guard) builds
  warning-free with vbcc and for the freestanding profile.
- **R8.6 Independent review.** The correctness reviewer, memory-safety
  auditor, emission-integrity auditor, performance auditor, and
  semantics-reviewer review the executed phases under the repository's strict
  quality profile. No critical or major finding remains open.
- **R8.7 Documentation.** Update `AGENTS.md` (commands and layout),
  `benchmarks/README.md`, `03-01-benchmark-harness.md`,
  `03-02-generated-c-changes.md`, `05-findings.md`, and
  `plans/00-roadmap.md`. Add the harness measurement boundary to
  `docs/known-limitations.md` and its required production-readiness register
  entry. Record the dropped struct-size metric under AR-22 (the linked
  `.text`/`.data`/`.bss` sizes are the accepted code-size metric; the ABI and
  struct layout are frozen) and update the roadmap row accordingly.

## Non-Functional Requirements

- **N1 — No behavior change.** For every accepted program, the emitted program
  computes the same values, traps at the same points, and releases the same
  storage. The benchmark outputs, the new spec suite, and the existing
  run-fault suite are the executable proof.
- **N2 — No language or ABI change.** No grammar, diagnostic, golden, profile,
  export, symbol, struct layout, or handle change. No new stdlib surface.
- **N3 — Determinism.** Same source, profile, and flags produce byte-identical
  C. Generated names keep their stable order; compaction adds no
  source-order-dependent numbering that changes between runs.
- **N4 — Portability.** All emitted C remains valid C99 for the host, m68k GCC
  (nix20 and nix13), vbcc, and the freestanding build where the shape can
  appear.
- **N5 — Operability.** The harness reports a single readable table and writes
  one JSON file per run. A failed gate names the benchmark, metric, base value,
  head value, and threshold.
- **N6 — Workspace hygiene.** All generated C, binaries, traces, and JSON live
  under ignored `build/` paths. The harness removes the run directory it
  created and never deletes a user path.
- **N7 — Frozen guard wording is behavioral.** The specification's guard rows
  describe the trap condition, not a requirement to spell every constant-false
  disjunct. Omitting a disjunct that the source semantics already exclude is
  not a specification change; the trap code, message, and exit status stay
  identical. No file under `spec/` is edited.
- **N8 — No shell or path exposure.** The harness uses argument-list
  subprocesses, validates names, and confines output to `build/perf/`.

## Won't Have

- A general optimizer, an SSA form, a dataflow framework, or a C-level
  post-pass.
- Profile-guided optimization, link-time optimization flags, or changes to the
  compile matrix's `-O0`/`-O2` choice.
- FS-UAE cycle or instruction instrumentation; the deterministic `vamos`
  instruction trace is the runtime metric.
- Read-only storage or removal of module-`const` initializers.
- Folding inside module-level initializers.
- Compaction of nested-row fills, struct-literal fills, array-literal prefixes,
  array copies, or the deferred expression form.
- Manual elimination of the upper index bound, loop-range analysis, or
  induction-variable analysis.
- Optimizations of the standard library or the runtime shims.
- A hard host-time gate; host time is advisory.
- Performance claims on physical Amiga hardware; the gate is the emulated
  m68k instruction count and code size.
- A new diagnostic or a change to any frozen `spec/tests` golden.

## Acceptance Summary

| Area | Acceptance |
|---|---|
| Harness | `make bench` produces a schema-valid result; `make bench-base`/`bench-compare` enforce R1.5; `make bench-test` passes in the container |
| Benchmarks | Four scored programs plus calibration, deterministic fixed-width output on host and hosted-amiga, within R2.4 budgets |
| Const folding | R4 behavior suite green; `sieve` net instructions at least 20% lower; address-taken and shadowed cases unchanged |
| Fill compaction | R5 behavior suite green; `sieve` `.text` at least 10% smaller and C at least 25% smaller; parameter byte count pinned |
| Guards | R6/R7 behavior suites green; slice/string negative and literal-division traps preserved; no regression beyond R1.5 |
| Gates | `make check`, `make asan`, `make corpus-amiga`, `make differential`, `make verify` green; freestanding compile leg green |
| Review | Correctness, memory-safety, emission-integrity, performance, and semantics reviews with no open critical/major finding |
