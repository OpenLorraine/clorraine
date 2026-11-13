# Requirements: Compiler Throughput (PLAN-38)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Status**: Approved for execution
> **Base**: `9c44537` (post-PLAN-37; `clc 1.1.2`)

## Feature Overview

Issue #13 asks for a compiler-throughput baseline and a measured improvement or
a documented no-op. The compiler front end is fast on hand-written sources (a
few milliseconds), but generated programs with thousands of functions expose a
quadratic scan in the C emitter. The feature makes that path linear, records
the result, and keeps every generated-code guarantee intact.

The work is measurement plus one internal emitter lookup change. It does not
change the language, the emitted C, the ABI, the standard library, the runtime,
the driver flags, or the scored benchmark harness.

## Functional Requirements

### Must Have

- [ ] **R1 — Repeatable harness.** `tools/perf/compile_time.py` measures
  `clc` wall time and peak RSS over four workload classes: the full
  specification corpus, the committed benchmarks, the largest Amiga examples,
  and deterministic generated inputs.
- [ ] **R2 — Per-workload record.** Every workload records median, minimum,
  and maximum wall time, peak RSS, and the compiler exit codes.
- [ ] **R3 — Comparable documents.** A `run` writes one JSON document with the
  compiler identity, toolchain, run count, and workload set. A `compare`
  command prints a base/head table and can enforce a regression limit.
- [ ] **R4 — Make target.** `make bench-compile` builds the release `clc` in
  the pinned container and writes `build/perf/compile-head.json`.
- [ ] **R5 — Generated shapes.** The generated inputs cover function count
  (1,000–8,000), call count (4,000), statement count (4,000), local count
  (4,000), and a mixed shape with 4,000 functions each called once.
- [ ] **R6 — Hot-pass attribution.** Record per-stage wall time for parse,
  load, resolve, sema, ownership, and codegen on representative shapes, using
  a temporary instrumented build that is not committed.
- [ ] **R7 — Measured optimization.** A quadratic path that the measurement
  identifies is replaced by a bounded lookup, with no change to emitted C,
  diagnostics, exit codes, or runtime behavior.
- [ ] **R8 — No generated-code regression.** The scored `-O2` benchmark rows
  are unchanged, and an independent base/head emission diff over the corpus,
  benchmarks, examples, and generated inputs reports no difference.
- [ ] **R9 — Documentation.** `AGENTS.md` lists `make bench-compile` and
  `benchmarks/README.md` explains the throughput harness and its separation
  from the scored runtime gate.
- [ ] **R10 — Tests.** The harness has unit tests for generation, summary,
  measurement, comparison, and document validation. The new emitter index has
  a unit test for module keying and first-record semantics.
- [ ] **R11 — Acceptance.** `make check`, `make asan`, `make ubsan`,
  `make bench-shape`, `make bench-compare`, `make corpus-amiga`,
  `make differential`, and `make verify` pass.
- [ ] **R12 — Issue closeout.** Close issue #13 with the baseline table, the
  measured improvement, and the remaining hot-pass note; mark the plan and
  roadmap Done.

### Should Have

- [ ] **R13 — No new surface.** No new dependency, compiler pass, profile,
  environment variable, build flag, or runtime helper. One harness, one make
  target, one internal index.

## Scope Boundary

**In scope:** the throughput harness and its tests; the `make bench-compile`
target; the emitter body index; per-stage measurement; the baseline and
improvement tables; the issue, plan, and roadmap closeout.

**Out of scope:** parallel or incremental compilation; a compiler daemon;
LTO/PGO; allocator or arena rewrites; optimizing the ownership pass without a
measured bottleneck; changing the scored `-O2` line; changing the frozen
specification, the stdlib, the ABI, the runtime shims, or generated C.

## Measurement Requirements

- [ ] **M1 — Same emission.** The optimization must leave the emitted C
  byte-identical for every workload that compiles successfully.
- [ ] **M2 — Representative classes.** Cover the corpus (including
  compile-fail fixtures), the seven benchmarks, six large Amiga examples, and
  the generated shapes.
- [ ] **M3 — Machine noise.** Use enough runs for a median and record the
  minimum; corroborate small-workload changes with an interleaved base/head
  run so ordering and frequency drift cannot masquerade as a win.
- [ ] **M4 — Recorded evidence.** Put the exact commands, compiler identity,
  stage table, workload table, and memory numbers in
  [05-findings.md](05-findings.md); keep scratch artifacts under `build/tmp`.

## Acceptance Criteria

- [ ] Every R1–R13 item has a test, a recorded measurement, or a documented
  decision.
- [ ] The new harness tests fail before the harness exists and pass after it.
- [ ] The body-index unit test fails to compile on the base and passes on the
  head.
- [ ] The emitted-C audit reports zero differences.
- [ ] `make bench-compare` reports zero scored deltas.
- [ ] The full `make verify` gate is green before the plan is marked Done.
