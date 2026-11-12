# Requirements: Template Placeholder Residuals II

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Issue #28 records seven adjacent residuals found by the PLAN-31 reviews. Each
reproduces on the base `bb3956c` or is a pre-existing codegen/runtime gap.
The feature closes all seven with the frozen rules and adds no language
surface.

## Functional Requirements

### Must Have

- [x] **R1 — Explicit text formattability.** `text(v)` requires a formattable
  argument (its arity is the separate #24 mini-plan). A null-shaped argument
  reports E10500 (`type 'null' is not formattable`); a concrete non-formattable
  argument reports E10500 with its type; an opaque parameter projection such as
  `text(x.n)` reports E10500 at the projection. An unnarrowed nullable
  formattable argument reports E10010. No C is written.
- [x] **R2 — Generic text argument.** A generic `text(x)` defers to each
  instance: a formattable, non-null (or narrowed) instance is accepted; a
  non-formattable instance reports E10500; an unnarrowed nullable instance
  reports E10010.
- [x] **R3 — Bare null binary operand.** A literal `null` operand reports
  E10012 at the null unless the operator is `==`/`!=` and the sibling is
  null-capable or null-shaped. `x == null` and `x != null` stay legal;
  `null == 5`, `null + 1`, `null + null`, and `null < null` are rejected.
  `null && flag` keeps the frozen E10218 condition report.
- [x] **R4 — Generic null-arm parity.** A generic `flag ? null : x` in an
  inferred or placeholder context follows the concrete settlement: at
  `T = str` it is accepted as the empty state; at `T = String` it reports
  E10010 only at the null arm; at a scalar or struct `T` it reports E10012 at
  the null arm. A placeholder over a `raw`/`cstring`/slice/`Box` arm still
  reports E10500 because the whole value is not formattable, and a written
  destination keeps its existing E10012 rule.
- [x] **R5 — Scalar null arm.** `let x = flag ? null : 5;` reports E10012 at
  the null arm. A written non-nullable destination keeps its existing E10012.
  An inferred handle result stays `T?` and is accepted until used.
- [x] **R6 — For-step owning assignment.** A step assignment whose target may
  own releases the old value once per iteration; a loop with three iterations
  is leak-free under ASan on the fall-through and `continue` edges.
- [x] **R7 — For-init effects.** A template or call in a for-init expression
  list runs exactly once and releases its result before the first test.
- [x] **R8 — Aggregate field order contract.** A same-order multi-field
  literal evaluates its fields in source order on the supported toolchains and
  a differing order takes the fieldwise store; both are pinned by a test, and
  the codegen contract states the supported-toolchain assumption.
- [x] **R9 — Panic message safety.** A zero-length panic message (a null view
  or zero-state String) writes the prefix and newline without a zero-length
  `fwrite` of a NULL pointer on host or Amiga.

### Should Have

- [x] **R10 — Diagnostic consistency.** Every new report uses a frozen code,
  message, and help: E10500 for formattability, E10010 for a possible-null
  use, E10012 for a null at a concrete position.
- [x] **R11 — Emission stability.** A non-owning for step keeps the compact
  `for` header; every effectful init (a template, call, or increment) now runs;
  the `text` one-argument shape and the #26/#27 copy shapes are unchanged.

### Won't Have (Out of Scope)

- #24 (`text` arity) — separate mini-plan `plans/text-arity/`.
- A general nullable dataflow analysis, a new null diagnostic, a new text
  framework, or a new loop representation.
- Any `spec/`, `stdlib/`, or ABI edit.
- An emission-wide rewrite of every aggregate literal to fieldwise stores
  (AR-12 documents the supported-toolchain assumption instead).

## Technical Requirements

### Performance

- The changes are static diagnostics, loop shapes, and two runtime guards. No
  benchmark program uses a for-step owning assignment, a for-init expression
  list, or a non-formattable `text` argument.
- `make bench-compare` runs against `bb3956c`; every scored output must be
  equal and no scored metric may exceed a 1% regression.

### Compatibility

- Rejecting `text(null)`, a non-formattable `text` argument, a bare null
  arithmetic operand, and a scalar/struct null arm tightens only unsound
  programs.
- Generic null arms at views and handles now match their concrete twins; the
  #26 ST-37 E10010 stays.
- A for-step owning assignment and a for-init effect now run correctly; valid
  programs keep their behavior.

### Security

- No new input, parser, FFI, or `unsafe` surface. The runtime guard is an
  internal null-length check; no user data or secrets are involved (AR-19).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
|----------|--------------------|--------|-----------|--------|
| `text(null)` | E10500 / E10012 / accept | E10500 | Frozen formattability rule | AR-1 |
| Nullable `text` | E10010 / accept / defer | E10010 unless narrowed | The explicit converter is a concrete use | AR-20 |
| Opaque `text` projection | E10500 / defer / leave | E10500 | Mirrors the placeholder opaque gate | AR-21 |
| Non-formattable `text` | E10500 / E10200 / leave | E10500 | Same rule as a placeholder | AR-2 |
| Generic `text` | defer / reject / leave | deferred duty | A formattable instance must stay legal | AR-3 |
| Bare null operand | E10012 / E10200 / leave | E10012; E10218 for `&&`/`||` | The null rule owns arithmetic, the frozen condition rule owns logical operands | AR-4 |
| Null-arm parity | shared helper / duplicate | shared helper | One rule for concrete and generic | AR-5–AR-9 |
| Bare-null predicate | define / reuse nullable-capable | define `accepts_bare_null` | Handles/`Box` are nullable-capable but not bare-null; the predicate must distinguish them | AR-5, AR-6 |
| Scalar null arm | E10012 / E10011 / leave | E10012 | The null is illegal at the settled position | AR-8 |
| For-step assignment | flat / header drop / leave | flat | The pre-drop is a statement | AR-10 |
| For-init effects | statement path / eager / leave | statement path | Reuses the ordinary release discipline | AR-11 |
| Aggregate order | rewrite / document + test | document + test | Issue authorizes it; no observable supported-toolchain gain | AR-12 |
| `cl_panic` | skip zero write / substitute / leave | skip | Removes the undefined call with no output change | AR-13 |
| #24 | bundle / separate | separate | Small independent task | AR-14 |
| Spec edit | yes / no | no | The frozen rules already require the behavior | AR-15 |

> **Traceability:** every scope decision cites its owning Ambiguity Register
> entry; the register holds the full delegated provenance.

## Acceptance Criteria

1. [x] All requirements R1–R11 are verified by the ST cases in
   [07-testing-strategy.md](07-testing-strategy.md).
2. [x] Every #28 reproduction behaves as specified under the host build, ASan,
   and UBSan.
3. [x] `make check`, `make asan`, `make ubsan`, `make verify`,
   `make bench-shape`, and `make bench` + `make bench-compare` are green.
4. [x] All scored benchmark outputs are equal; no scored metric regresses by
   more than 1%.
5. [x] No critical or major review finding remains open.
6. [x] The findings ledger, the roadmap row, and issue #28 reflect the
   delivered evidence.
