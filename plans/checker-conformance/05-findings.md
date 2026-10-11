# Findings: Checker Conformance

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: in progress

## Review Protocol

Each phase receives one review round from the lenses listed in its execution-plan header,
dispatched per the project policy (AGENTS.md). A finding is recorded as
`<lens>-NNN | severity | area | what | reproduction | expected vs observed | ruling`; a
critical or major finding gets an authorized ruling in the active mode and a fix round, then
one scoped re-review; direct verification closes further fixes in the same phase. The phase's
verdict line records `no findings` or the final state of every finding.

## Phase 1: Initializer guard operands and typedef collectors

**Baseline**: `477f27dd36dcbde5f40e6dc267ce1864388dbe11` (the `8170199` tree).
**Lenses**: correctness, memory-safety, emission-integrity

### Deliverables

| # | Shape | Result |
| - | ----- | ------ |
| DEF-136 | The initializer materialization stores an impure dividend (`Div`/`Rem`) and an impure slice index base into an init local before their guard runs; the fixed-array base keeps its spelling (C cannot assign an array to a temporary) and the existing single-call shape | Fixed; spec `initdividend`/`initbase`; impl pins the one-call stores |
| DEF-142 | The body collector records every value's own substituted type, so an inline `slice(...)` builder's view typedef exists | Fixed; spec `inlinearg`; impl pins the typedef and the compound literal |
| DEF-215 | The collector walks the lifted-lambda specializations with their substitutions (mirroring the emission's specialization guard) | Fixed; spec `lambdaspec`; impl pins the typedef and the lifted body's `sizeof` use |
| DEF-216 | The monomorphization records a written type argument's concrete types, so a struct instance referenced only as a slice element is declared — and, in the emitter, before the slice typedef that names it | Fixed; spec `structelement`; impl pins the declaration order |

### Red evidence

At the phase baseline the five row cases failed exactly as filed: `initdividend` and
`initbase` exited 22 (the guard and the operation each ran the impure call), `inlinearg`
failed the strict build with `'ClSlice_String' undeclared`, `lambdaspec` with
`'ClSlice_i32' undeclared` (the specialization body), and `structelement` with the
undeclared `ClWrap__i32` its slice typedef names. The `initownbase` reference passed at
the baseline (the fixed-array base was already single-call and AddressSanitizer-clean),
as intended.

### Implementation notes

- **Shape gating.** The initializer additions fire only for an impure dividend and an
  impure non-fixed-array base; the needs-materialization predicate mirrors both so a
  lazily evaluated position (a conditional arm or a short-circuit right side) structures
  its guard exactly when the store happens. The collector additions only grow the typedef
  set; the mono recording only grows the instance list.
- **The fixed-array guard.** `store_impure_intrinsic_arg` already documents the reason a
  fixed array never stores into a temporary; the index base carries the same guard so the
  verified owning-index-base shape re-emits byte-identically.
- **The specialization walk** mirrors `lifted_emissions`: same enclosing lookup, same
  `lifted_needs_specialization` gate, same `instance_subst` — so collected types exist
  exactly where a specialization is emitted.
- **The struct-instance recording** sits in `walk_value` beside the value-type recording,
  so it applies to plain bodies, instances, and lifted specializations alike.

### Verification

Spec suite 6/6 plain and under `RDC_ASAN=1`; impl 6/6; `make check` 62 suites green (the
two new suites included). The phase review (`correctness` + `memory-safety` +
`emission-integrity`) is dispatched after the phase commit.

### Review and fix round

_(findings recorded after the review round)_

## Phase 2: Generic instance conformance

**Baseline**: _(recorded at phase start)_
**Lenses**: correctness, semantics, emission-integrity

_(findings recorded after the review round)_

## Phase 3: Call, member, and `cstring` surfaces

**Baseline**: _(recorded at phase start)_
**Lenses**: correctness, semantics, emission-integrity
**Gate**: AR-2 (the arity diagnostic code) must be resolved before Step 3.2.

_(findings recorded after the review round)_

## Phase 4: Null-shaped values

**Baseline**: _(recorded at phase start)_
**Lenses**: correctness, semantics

_(findings recorded after the review round)_

## Phase 5: Diagnostic hygiene

**Baseline**: _(recorded at phase start)_
**Lenses**: correctness, semantics

_(findings recorded after the review round)_

## Phase 6: Close-out

**Baseline**: _(recorded at phase start)_
**Lenses**: none new

_(the full-verification evidence recorded at close-out)_
