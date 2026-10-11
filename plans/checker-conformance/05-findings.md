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

**Baseline**: _(recorded at phase start)_
**Lenses**: correctness, memory-safety, emission-integrity

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
