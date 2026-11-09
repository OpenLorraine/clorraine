# Requirements: Volatile Nested Fill Fix (PLAN-23)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

A nested fixed-array fill into volatile storage must emit valid C. The row copy
passes the volatile destination to `memcpy`, which discards the qualifier and
fails a strict build. The fix keeps the existing evaluate-once row temporary
and copies each row leaf by leaf when, and only when, the destination storage
is volatile.

The fix covers three copy sites the nested fill path uses:

- the fill-row copies inside `emit_array_assign_to`;
- the explicit prefix-row copies that `emit_array_assign_to` delegates when a
  prefix element is a place or another value;
- the whole-element copies of a run-time indexed target, which the statement
  store lowers through `Statement::StorePointer`.

## Scope Boundary

In scope: those copy sites, the volatility predicate that selects them, the
tests, the strict-shape fixture, the reviews, and the closeout records. The
v1.1.1 release is a separate roadmap row (PLAN-24) executed after this plan's
closeout; it follows `RELEASE.md` and is not part of PLAN-23's requirements.

Out of scope: whole-array assignment or move copies into a volatile array
(`g = other`, a call result, a self-reading literal), a volatile array used as
a copy *source* into a row temporary, struct-literal prefix elements with a
fixed-array field, a nested fill through a `for borrow` alias over volatile
storage, a `borrow` parameter bound to a volatile field, owning explicit
elements, and any language, ABI, runtime, stdlib, driver, LSP, or benchmark
change. These are recorded as separate limitations or follow-ups. The
whole-array place copy is pinned so this fix cannot silently claim it.

## Functional Requirements

### R1 - Volatile-safe nested fill copy

- **R1.1 Volatile detection.** A fill target is volatile when either:
  - its place base is a module-level `volatile let` global, or
  - a field projection on the path names a struct field declared `volatile`.
  - A field reached through one or more `raw` pointer layers counts; the field
    declaration is resolved through the pointee struct. A `volatile raw T`
    field is emitted as `volatile T *`, so a member behind it is volatile too.
  - A `len` projection is a field only when it names a struct field; on a
    fixed array it is the structural length constant and has no storage. The
    predicate mirrors `place()` here.
  - A run-time indexed whole-element target (`g[i]`, `regs.a[i]`) carries its
    element expression, not a `Place`. Its volatility is derived from the
    addressed value tree: a volatile global or field, a member or element of
    one, or an address-of/cast of one.
  - Any other target is non-volatile.
- **R1.2 Copy spelling.** For a volatile target, the fill-row copy and the
  run-time indexed element copy emit one C store per deepest element through
  the destination lvalue: `dest[i][j] = row[j];` (one index step per array
  extent).
- **R1.3 Prefix spelling.** A volatile prefix row whose value names existing
  storage is copied leaf by leaf directly. A value that could re-run an effect
  (a call, a conditional, a call-returning member, a call-valued index, or a
  materialization of one) is evaluated once into a row temporary first, then
  copied leaf by leaf. No prefix copy emits `memcpy` into a volatile
  destination.
- **R1.4 Unchanged non-volatile path.** A non-volatile target keeps the current
  spelling: one `memset` for a compacted uniform zero fill, otherwise a row
  temporary plus one `memcpy` per covered row. The prefix-row copy keeps its
  `memcpy`. A non-volatile run-time indexed element keeps its byte copy.
- **R1.5 Evaluate once.** The fill row temporary and the prefix value are
  evaluated once. An impure fill or prefix runs once; the copy reads the
  temporary.
- **R1.6 Full prefix.** A uniform zero fill with `prefix >= extent` still emits
  nothing. A non-zero or impure full prefix keeps the temporary and its unused
  marker.
- **R1.7 C validity.** The emitted C for the in-scope shapes is valid C99 for
  host, m68k GCC, vbcc, and freestanding. No
  `-Wdiscarded-array-qualifiers` or equivalent diagnostic.
- **R1.8 Locality.** The change stays in `compiler/src/codegen/function.rs`
  and its tests/fixture. It adds no language feature and no new module.

### R2 - Tests

- **R2.1 Behaviour suite.** `compiler/tests/nested_fill_compaction_spec_test.rs`
  gains:
  - ST-V1: a function-body zero fill into a `volatile let` 2D `i32` array;
  - ST-V2: a function-body non-zero fill into a `volatile let` 3D `i32`
    array;
  - ST-V3: a function-body fill of a `volatile let` array of plain structs;
  - ST-V4: a `volatile let` 2D array with two explicit prefix rows and a zero
    fill;
  - ST-V5: a place-valued prefix row followed by a fill;
  - ST-V6: the volatile overlay field reached through a `raw` pointer, emitted
    and built with a strict host C compiler but not run (the probe address is
    not a real host object);
  - ST-V7: a run-time indexed whole-element target on a volatile 3D global;
  - ST-V8: an impure prefix row that must evaluate its call once;
  - ST-V9: a run-time indexed whole-element target on a volatile overlay
    field, built only;
  - ST-V10: a `volatile raw` pointer field, built only;
  - ST-V11: a volatile destination filled from constant-indexed volatile rows
    (`h[0]`, `h[1]`);
  - ST-V12: a volatile destination filled from an arithmetic-indexed volatile
    row (`h[k - 1]`).
  Each running case emits host C, builds with
  `cc -std=c99 -O0 -g -Wall -Wextra -Werror` (plus the sanitizer flags when
  `RDC_ASAN`/`RDC_UBSAN` is set), runs it, and compares stdout. The suite
  already carries the sanitizer markers.
- **R2.2 Emission suite.**
  `compiler/tests/nested_fill_compaction_impl_test.rs` pins, using the full
  mangled global symbol where a case uses a global:
  - IP-V1: a volatile global 2D zero fill: no destination `memcpy`, leaf
    stores;
  - IP-V2: a volatile global 3D fill: no destination `memcpy`, leaf stores;
  - IP-V3: a volatile overlay field: no destination `memcpy`, leaf stores;
  - IP-V4: a volatile global struct-row fill: no destination `memcpy`, struct
    leaf stores;
  - IP-V5: a place-valued prefix row: no destination `memcpy`, leaf stores;
  - IP-V6: a struct field named `len` with a volatile array type: leaf stores;
  - IP-V7: a non-volatile field target: the existing `memcpy` spelling is
    unchanged;
  - IP-V8: a non-volatile local: the existing `memset` compaction is
    unchanged;
  - IP-V9: a volatile one-extent scalar fill: unchanged element stores;
  - IP-V10: a volatile full-prefix zero fill: no temporary and no marker;
  - IP-V11: volatile explicit rows: direct stores, no row temporary;
  - IP-V12: a whole-array place copy `g = other` into a volatile global still
    emits `memcpy`, documenting the separate limitation;
  - IP-V13: a run-time indexed volatile global element: no destination
    `memcpy`, leaf stores;
  - IP-V14: an impure prefix row: one call, a row temporary, leaf stores, no
    destination `memcpy`;
  - IP-V15: a run-time indexed volatile overlay field: no destination
    `memcpy`, leaf stores;
  - IP-V16: a `volatile raw` pointer field: leaf stores, no destination
    `memcpy`;
  - IP-V17: a constant-indexed volatile source row: direct leaf stores, no
    row temporary;
  - IP-V18: an arithmetic-indexed volatile source row: direct leaf stores, no
    row temporary.
- **R2.3 Red first.** The twelve new behaviour cases fail on the base compiler
  because of the discarded qualifier (ST-V8 also shows the base's
  re-evaluation); the twelve new and changed emission pins fail on the base. The
  guards pass on the base.
- **R2.4 Test integrity.** Every new positive assertion fails when the volatile
  copy branches are reverted. Forcing the predicate false fails the volatile
  pins; forcing it true fails the non-volatile field guard. Making an impure
  prefix value "stable" fails ST-V8/IP-V14. Disabling the run-time index
  volatility fails ST-V7/ST-V9/IP-V13/IP-V15. Removing the prefix temporary
  fails ST-V5/IP-V5.
- **R2.5 Strict shapes.** `tools/perf/fixtures/shapes.clo` gains a volatile
  global nested fill and a volatile overlay field fill, with the overlay
  helper called from `main` so no `-Wunused-function` appears.
  `make bench-shape` builds the fixture with m68k GCC, GCC nix13, vbcc, and
  the freestanding profile without a warning.
- **R2.6 Sanitizers.** The behaviour suite runs in `make asan` and
  `make ubsan`; the new cases are included automatically.

### R3 - Verification and reviews

- **R3.1 Focused green.** Both focused suites pass after the fix.
- **R3.2 Fast gate.** `make check` passes.
- **R3.3 Sanitizers.** `make asan` and `make ubsan` pass with the suite counts
  recorded.
- **R3.4 Target gates.** `make corpus-amiga` and `make differential` pass.
- **R3.5 Strict shapes.** `make bench-shape` passes.
- **R3.6 No benchmark change.** Build a fresh base compiler at `7cdbb71` with
  `make bench-base COMMIT=7cdbb71`, run `make bench` on the final fix commit,
  and run `make bench-compare` with no `REQUIRE` (the 1% no-regression guard).
  Record both compiler SHA-256 values and `dirty=false`. The fix only targets
  volatile storage, which no scored benchmark uses; a moving scored benchmark
  is a regression and blocks the closeout.
- **R3.7 Full acceptance.** `make verify` prints `verify: all legs passed`.
- **R3.8 Emission audit.** A base/head diff with the `7cdbb71` base compiler
  over the corpus, the benchmarks, and the Amiga examples shows no change
  outside the volatile nested-fill probes. A non-volatile unit that changes is
  a blocker.
- **R3.9 Reviews.** `correctness-reviewer`, read-only `memory-safety-auditor`,
  read-only `emission-integrity-auditor`, and read-only
  `performance-auditor` review the codegen phase. Every packet includes a
  blast-radius step and a test-integrity step. Critical and major findings
  pause the phase for a ruling.

### R4 - Documentation and closeout

- **R4.1 Findings ledger.** Record the reproduction, the red/green record, the
  integrity mutations, the emission audit, the review verdicts, and the gate
  results in [Findings](05-findings.md).
- **R4.2 Roadmap.** Add the PLAN-23 row as `Executing` with the plan commit,
  mark it `Done` after the gates, and update the progress header.
- **R4.3 Known limitations.** Add the fixed statement and record the remaining
  whole-array copy, volatile-source copy, struct-literal prefix, and
  alias/borrow indirection limitations.
- **R4.4 No identifiers in code.** No plan, requirement, or task identifier
  appears in source or doc comments.

## Non-Functional Requirements

- **N1 Determinism.** Two emits of the same source produce identical C.
- **N2 Safety.** No ownership, drop, or trap change. ASan and UBSan clean.
- **N3 Compatibility.** Accepted non-volatile source emits byte-identical C.
- **N4 Hygiene.** Scratch output stays outside the repository or under
  `build/`; nothing generated is committed.
- **N5 Reviewability.** The fix, tests, and fixture land in one green commit;
  docs land separately. Any codegen change after the measurement forces a
  fresh benchmark and emission audit on the new clean commit.

## Traceability

| Requirement | Evidence |
|---|---|
| R1 | [Design](03-design.md), emission audit |
| R2 | [Testing Strategy](07-testing-strategy.md), red/green record |
| R3 | [Findings](05-findings.md) gate table |
| R4 | roadmap, `docs/known-limitations.md`, closeout commit |
