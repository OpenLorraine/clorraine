# Requirements: Direct Template Formatting

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Revision**: final — all ambiguity-register items resolved

## Feature Overview

Issue #19's shim-only formatter removed the C library formatting cost but left
the template allocator untouched: each integer placeholder still built a
temporary `String`. Issue #25 records the measured next shape. This plan
implements it.

A template with `n` integer placeholders must allocate one result `String`,
not `n + 1`. The lowering carries the placeholder value in a 12-byte part; the
builder formats it into the destination. `str` and `String` placeholders
borrow their bytes instead of copying them. Float placeholders keep the
documented `%g` output and its one temporary.

## Scope Boundary

**In scope:** `compiler/src/codegen/expr.rs::template`; the `ClStrPart`
builder in `compiler/runtime/shim_host.c` and
`shim_hosted_amiga.c`; behavior, allocation, and emission tests; the full
benchmark, sanitizer, emission, and review gate; the #25/#19 closure and
roadmap update.

**Out of scope:** language syntax, semantics, grammar, diagnostics, ABI, and
`spec/`; `stdlib/*.clo`; the explicit `text`/`toDecimal`/`toHex` converters;
float formatting beyond the existing `%g`; buffer/array allocation; a new
builder API or allocator; the freestanding profile; versioning or a release.

## Functional Requirements

### R1 — One result allocation per non-float template

- A template with integer, bool, or string placeholders allocates exactly one
  `String` (the result).
- A template with a float placeholder may allocate the existing `%g`
  temporary plus the result; the builder frees the temporary.
- No converter `String` is created for an integer, bool, `str`, or `String`
  placeholder.

### R2 — Output is byte-identical

- Every template output is unchanged: decimal boundaries, adjacent
  placeholders, literal text, empty `str`, `String`, bool, and float.
- The existing `string_integer_formatting` behavior suite and the frozen
  spec/corpus goldens stay green.

### R3 — Ownership and lifetime

- A `String` placeholder is borrowed for the duration of the full expression
  and the result copies its bytes; the source is not consumed, freed, or
  invalidated.
- An owned temporary from a call (for example `text(7)`) remains alive until
  the end of the statement, after the builder copies it.
- The builder frees every owned piece exactly once.
- ASan with leak detection and UBSan with recovery disabled are clean.

### R4 — Emission and ABI

- The template still produces an owned `String` and still lowers to one
  `cl_string_concat` call.
- The program body, the explicit converters, and every shim API signature are
  unchanged.
- The `ClStrPart` shape is shim-internal; no user-visible ABI changes.
- No `spec/` or `stdlib/` file changes.

### R5 — Profiles and strict C

- Host, `hosted-amiga`, `hosted-amiga-13`, vbcc, and freestanding build legs
  are warning-free.
- The freestanding shim has no `String` and is untouched.
- The new designated union initializers compile under the pinned C99
  toolchains.

### R6 — Measured win and no scored regression

- `strings` net instructions must drop below the shim-only head (1,489,187)
  and allocations must drop from 6,003 toward 2,002.
- No scored benchmark may exceed the 1% instruction or `.text` guard; every
  output hash matches.
- The base is the current `main` head `00eb6bc` built clean.

### R7 — Closeout

- `make check`, `make asan` (new suite), `make ubsan` (new suite),
  `make bench-shape`, `make bench-compare`, and `make verify` are green.
- `05-findings.md` records the allocation split, the final table, the emission
  diff, and the review verdicts.
- Issues #25 and #19 are closed with the final evidence; the roadmap row is
  Done.

## Scope Decisions

| Decision | Chosen | Rationale | AR Ref |
|---|---|---|---|
| Implement #25 | Yes | The active directive says #19's allocation component is unresolved | AR-01 |
| Part shape | 12-byte union | Measured probe; preserves the compound-literal footprint | AR-02 |
| Scalar kinds | `len` code 0/1 | One array, no second field | AR-02 |
| String/str | Borrow | Removes the copy and is lifetime-safe | AR-03 |
| Float | Existing `%g` temp | Preserves the documented output | AR-03, AR-04 |
| Capacity | Max reserve, exact logical cap | One pass, append behavior preserved | AR-05 |

## Acceptance Criteria

1. [ ] A two-integer-placeholder template makes one allocation; the red probe
       on the base makes three.
2. [ ] Every `07-testing-strategy.md` case passes.
3. [ ] `strings` allocations are near 2,002 and net instructions are below
       1,489,187; every other benchmark is inside the 1% guard and every
       output hash is equal.
4. [ ] ASan and UBSan are clean.
5. [ ] The emission-integrity audit finds no program-body change beyond the
       template call and the shim builder.
6. [ ] `make check`, `make bench-shape`, `make bench-compare`, and
       `make verify` pass.
7. [ ] #25 and #19 are closed; the roadmap is Done.
