# Ambiguity Register: Constant Array Initialization (PLAN-25)

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Mode**: auto-design (eligible technical decisions delegated by the user's
> `exec-plan --auto-commit --auto-design` invocation)
> **Root invocation ID**: `codegen-perf-2026-11-09`
> **Policy version**: 1

Auto-design active — eligible technical decisions are delegated and recorded.
Product scope, acceptance criteria, and the benchmark base remain user-owned.

## Decision Records

### AR-01 — Which measured shape is in scope

- **Status**: ✅ Resolved
- **Authority**: User — the goal authorizes the measurement audit and one
  measured shape (`codegen-performance-closeout-goal.md`, Scope In 1–3).
- **Objective**: find a real hot emitted shape that the existing harness can
  measure and that the current code does not already optimize.
- **Decision**: a module-level fixed array with an explicit list of constant
  scalar elements. The current emitter writes one store per element in
  `cl_init_<module>`.
- **Evidence**: `lookup` 4,096-entry `i32` table (offset by 32768 to
  1..65535, non-zero checksum): current `.text` 117,440 and raw instructions 17,025;
  block-scope template+`memcpy` probe 92,976 and 12,842; output equal. The
  four existing benchmarks are saturated or out of scope
  (`plans/nested-fill-compaction/02-current-state.md`).
- **Rejected alternatives**: strings interpolation (stdlib/libnix allocation),
  sieve loop work (4–5 instructions per iteration), matrix fill (already
  compacted), moves struct copies (they need an internal ABI/calling-convention
  change),
  linker DCE (build/toolchain change), whole-program DCE (large call-graph
  surface), function-local tables (separate declaration path, no committed
  benchmark).
- **Strongest counterargument**: a 4,096-entry table is large and the benchmark
  is sensitive to its size; a smaller table would show a smaller `.text` win.
  The in-repo `cube` example uses a 256-entry sine table, so 4,096 is a
  scaled but real audio/graphics table.
- **Confidence**: High — the shape is emitted today and the probe reproduces.
- **Hardening**: requirements preflight and the four phase reviews challenge
  the attribution; the final harness measurement is the gate.
- **Reopen triggers**: the base/head harness shows no `lookup` win, or a
  reviewer finds that the benchmark does not represent a real workload.

### AR-02 — Benchmark base commit

- **Status**: ✅ Resolved
- **Authority**: User — the task explicitly says “Use v1.1.0 as the benchmark
  base.”
- **Decision**: `make bench-base COMMIT=v1.1.0`; the cumulative comparison is
  v1.1.0 → head. The isolated optimization comparison uses the clean
  pre-codegen head document (current tree) so the `lookup` delta is not mixed
  with the already-landed nested-fill/volatile-fill work.
- **Evidence**: `v1.1.0` is an annotated tag on `main`; the current tree is
  `v1.1.1` plus cleanup. The harness records compiler identity and dirty state.
- **Rejected alternatives**: base = current HEAD (would hide the cumulative
  release delta the user asked for); base = v1.1.1 (not the requested base).
- **Confidence**: High.
- **Reopen triggers**: the user names a different base, or `v1.1.0` cannot
  compile the committed benchmark.

### AR-03 — Implementation mechanism

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by `--auto-design` (compiler/optimizer
  mechanism; within the confirmed goal and constraints).
- **Objective**: remove the per-element stores while preserving observable
  behavior and the writable global's address/storage.
- **Decision**: emit one block-scope `static const` template with the constant
  values inside the same `cl_init_<module>` function, then one
  `memcpy(<global>, <template>, sizeof(<global>));` at the exact position of
  the replaced stores.
- **Evidence**: the probe preserves output and measures `.text` 117,440 →
  92,976 and raw instructions 17,025 → 12,842 on the committed `lookup`
  source. The target stays `static` and writable; init order is unchanged; the
  template is read-only. Block-scope `static const` with a constant initializer
  is C99 and is accepted by the host, m68k GCC, and vbcc shape legs (to be
  pinned by `bench-shape`).
- **Rejected alternatives**:
  - Static initializer in the global declaration plus skipping the init
    function: removes the runtime copy entirely and saves the most total
    bytes, but moves the write earlier than every module initializer and
    changes the observable result if another initializer writes an
    address-taken global first. A whole-program write/order proof is a larger
    surface and a potential safety trade-off.
  - Linker `-ffunction-sections --gc-sections`: a build/toolchain change, not
    generated-C codegen; it would break the harness's same-flags comparison
    invariant and targets the out-of-scope runtime shim.
  - Whole-program dead-function elimination: a call-graph/FFI feature, not a
    local emitted-shape rule.
- **Strongest counterargument**: the static initializer is simpler and saves
  more total bytes (the template version keeps both the `.bss` target and the
  read-only template). The answer is that the scored `.text` and instruction
  metrics are the same, and the template version has no init-order or
  address-taken semantic risk. The extra total bytes are not a scored
  regression and are still smaller than the current store run (the measured
  `.text` delta is 24,464 bytes).
- **Confidence**: High — the probe is semantics-preserving and the scored
  metrics match the static-initializer probe.
- **Hardening**: correctness and emission-integrity reviews test the init order,
  address-taken mutation, volatile exclusions, and strict builds. The
  performance auditor checks that the win is not a measurement artifact.
- **Reopen triggers**: a review finds an observable order/aliasing difference,
  or a toolchain rejects the block-scope template.

### AR-04 — Eligibility boundary

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by `--auto-design` (optimizer precondition;
  smallest viable design).
- **Decision**: module-level `let`; one known extent > 0; non-float scalar
  leaf including `bool`; a **complete** explicit element list with no fill;
  every element guard-free and effect-free (integer/bool/char literal,
  non-float integer cast, `Neg`/`BitNot`/`Not` only); at least 16 elements
  and 64 bytes under the profile’s type table; not `volatile`; host,
  hosted-amiga, or hosted-amiga-13.
  Module `const` arrays, short/empty lists, freestanding multi-byte tables,
  and guard-bearing elements are excluded.
- **Evidence**: the measured `lookup` shape is exactly this. Fills already
  have their own compaction paths; multi-extent, float, struct, owned, and
  view elements have separate semantics; a zero extent is a front-end
  `E10226`; a module `const` explicit list is `E10224`; a short list leaves
  an omitted tail at the global’s existing state while an extent-sized
  template would zero-fill it, and the tail policy is unmeasured;
  `constant_integer` alone accepts `MIN / -1` and float casts that the
  current emitter guards.
- **Rejected alternatives**: include fill forms or local arrays now (larger
  test and semantics surface without a measurement); include floats (accepted
  float limits); include structs/owned (drop and padding semantics); include
  short lists (the tail policy is unmeasured); include module
  `const` arrays (a checker/language change); include freestanding multi-byte
  tables (byte-wise shim `memcpy`, unmeasured).
- **Strongest counterargument**: users may expect the same win for local
  tables and short lists; the requirements record those as residual
  opportunities, not silent omissions.
- **Confidence**: High.
- **Reopen triggers**: a measured local/fill/short-list shape appears in the
  harness, or the `lookup` shape needs an excluded form.

### AR-05 — Initialization order and target mutability

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by `--auto-design` (semantics-preservation
  mechanism).
- **Decision**: keep the module initializer and the writable global; do the
  copy at the same point. Do not make the global `const`, do not skip the
  init function, and do not rely on C static-initialization timing.
- **Evidence**: `global_init_order` and the module topological order already
  define the init point; a `memcpy` there has the same effects as the stores.
  Address-taken `unsafe` mutation continues to work because the target is
  still writable.
- **Rejected alternatives**: static declaration initializer (AR-03), moving
  the copy to the entry stub (would change module order).
- **Confidence**: High.
- **Reopen triggers**: a reviewer demonstrates an observable order difference.

### AR-06 — Test strategy

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by `--auto-design` (testing strategy).
- **Decision**: a behaviour suite (`*_spec_test.rs`) and an emission suite
  (`*_impl_test.rs`); red-first positive pins run against the pre-change
  compiler through the existing `RDC_CLC` seam; exclusion guards; sanitizer
  markers; a shape-fixture case; harness tests. The behaviour cases record
  expected output that the base compiler also produces. The suites cover
  every eligible type, boundary, guard-free exclusion, front-end-invalid
  diagnostic, address-taken mutation, cross-module order, short/empty lists,
  and the freestanding profile gate.
- **Evidence**: the repository’s existing feature suites follow this pattern;
  the change has no ownership/drop change but still runs under ASan/UBSan
  because the suite covers arrays, guards, and address-taken mutation.
- **Rejected alternatives**: only impl pins (would not prove behavior);
  adding cases to the existing nested-fill suite (different feature and
  lifecycle); depending on the gitignored base binary for the normal green
  run (the `RDC_CLC` seam is optional evidence only).
- **Confidence**: High.
- **Reopen triggers**: a review finds an untested eligible/excluded shape.

### AR-07 — Benchmark requirement values

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by `--auto-design` (performance engineering;
  the user set the no-regression/acceptance frame).
- **Decision**: initial `lookup:instructions:10` and `lookup:text:10`;
  re-derive in Phase 1 from the committed harness. The no-regression rule
  remains the generic 1% guard.
- **Evidence**: probe deltas -24.6% raw instructions and -20.8% `.text` on the
  committed `lookup` source; the net percentage instruction improvement is
  larger after calibration subtraction (-30.6%). 10% leaves margin for the
  block-scope template and line wrapping.
- **Rejected alternatives**: requiring the probe’s exact 24.6%/20.8% (brittle
  across the template spelling); a code-size-only requirement (the measured
  runtime also improves).
- **Confidence**: High.
- **Reopen triggers**: the formal baseline measures a materially different
  delta.

### AR-08 — No harness, linker, or shim change

- **Status**: ✅ Resolved
- **Authority**: User — the goal’s scope excludes runtime-shim, toolchain, and
  scoring changes; the measurement contract is unchanged.
- **Decision**: use the existing harness, flags, `--require` option, and
  JSON schema. Do not add linker flags, do not change the shim, and do not
  change the scored programs.
- **Evidence**: `tools/perf/bench.py` already records compiler identity, C
  size, section sizes, host time, and streamed instructions; the comparator
  already supports `--require`.
- **Rejected alternatives**: `-ffunction-sections`/`--gc-sections` (AR-03),
  new benchmark harness code (not needed).
- **Confidence**: High.
- **Reopen triggers**: the harness cannot measure the new benchmark
  deterministically.

### AR-09 — Documentation and roadmap

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by `--auto-design` (documentation of an
  approved change).
- **Decision**: update `benchmarks/README.md`, the `AGENTS.md` command table
  only if the boundary changes, `docs/known-limitations.md` only if a new
  limit exists, the findings ledger, and one `PLAN-25` roadmap row. No plan
  identifiers in source or doc comments.
- **Evidence**: the repository’s closeout pattern.
- **Confidence**: High.

### AR-10 — Specialist coverage

- **Status**: ✅ Resolved
- **Authority**: AI — delegated by `--auto-design` (review routing within the
  repository’s existing quality profile).
- **Decision**: reuse the project’s existing correctness reviewer,
  memory-safety auditor, emission-integrity auditor, and performance auditor.
  No new specialist is proposed.
- **Evidence**: `AGENTS.md` phase-review dispatch and
  `codeops/specialist-check.json` cover this compiler/codegen surface. The
  change does not add a new domain, dependency, protocol, or infrastructure.
- **Confidence**: High.

## Final Verification

- [x] Every entry is `✅ Resolved` with an explicit authority and decision.
- [x] Every delegated entry is within the confirmed goal, scope, constraints,
      and acceptance criteria.
- [x] No reserved authority (product scope, acceptance criteria, publication,
      spending, credentials) was exercised by auto-design.
- [x] Deferred machinery is absent from executable requirements.
- [x] The register is referenced by the requirements document.
