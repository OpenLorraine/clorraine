# Requirements: Non-Lexical Lifetimes (NLL v2)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The version-1 borrow checker uses the conservative lexical approximation: a
loan is live from its creation until the end of the scope that owns the
derived view (`spec/16-borrow-rules.md:22-23,237-241`). Non-lexical lifetimes
(NLL, RFC 2094) replace that with last-use regions: a loan is live at a point
only if a use of it is still reachable. The spec stages this as the v2 target
(`spec/16-borrow-rules.md:306-310`) and pins the source contract as unchanged
(`spec/16-borrow-rules.md:312-313`).

This plan makes the implementation match the v2 model. It changes liveness
only: the loan set, the diagnostic codes, and every non-liveness rule stay as
they are. Because the frozen chapters describe v1 as the current model, the
plan also amends them to describe v2, and rewrites the single compile-fail
golden whose program NLL accepts.

## Functional Requirements

### Must Have

- [x] **R1 — Last-use regions (AR-3, AR-4):** a loan is live at a point iff the
  loan's creation reaches the point and one of the loan's uses is reachable
  from it. `E10022` (move), `E10031` (write), and `E10032` (loan request) are
  decided against the regions, not against a lexical scope.
- [x] **R2 — Loan identity and alias fidelity (AR-5):** every view value
  carries a loan id; copies and derived views share the id, and a use through
  any alias extends the same region.
- [x] **R3 — Use and access coverage (AR-6, AR-7):** every access through a
  view value counts as a use — reads, writable-view writes, derivations,
  arguments, returns, and `defer` uses — and events are ordered by evaluation
  order, including inside one statement.
- [x] **R4 — Origin containment (AR-8):** `E10030` fires iff a use of a loan is
  reachable from an owner-release point (scope exit, return cleanup,
  `break`/`continue` cleanup), and is reported at the loan's creation span with
  the existing message and help. A view whose last use precedes its owner's
  scope end is accepted.
- [x] **R5 — Precision coverage (AR-15):** branches, loops, early returns,
  `defer` blocks, element loops, standard-library view helpers, implicit
  `String` widening, and generic instances all follow the same region rules.
- [x] **R6 — Monotonicity and regressions (AR-16):** every program v1 accepts
  stays accepted; the corpus and every existing test suite stay green.
- [x] **R7 — Spec and ledger coherence (AR-2):** `spec/16-borrow-rules.md`,
  `spec/lineage.md`, `spec/decisions.md` (O11), and
  `spec/03-ownership.md:229` describe the model v2 implements; the flipping
  golden is rewritten to a shape that still rejects; no other golden changes.
- [x] **R8 — No new surface (AR-1, AR-22):** no syntax, no new diagnostic code,
  no message/help/span change; every non-liveness ownership rule is unchanged.
- [x] **R9 — Clean implementation (AR-10):** the lexical `Loans` state, its
  duplicate conflict rules, and its lexical `E10030` check are removed with
  their tests migrated; no dead code remains.

### Should Have

- None — the requirements above are the whole feature.

### Won't Have (Out of Scope)

- Per-field loan precision (O8), aggregate view returns (F2), `Box<T>` (X1),
  and nested partial moves (O4): each stays deferred (AR-1).
- A Polonius-style alias-based checker (v3) and a detached checker over a
  separate IR pass (AR-4).
- New syntax or diagnostics, and cleanup of the dormant `Statement::Borrow` /
  `ir::LoanKind` scaffolding (AR-1, AR-17).
- Changed diagnostic text, help, or spans; changed emitted C for accepted
  programs (AR-17, AR-22).

## Technical Requirements

### Performance

No target (AR-19). The analysis is bounded by the function's event count and
block count, with small constants; compilation order of growth is unchanged.

### Compatibility

Internal compiler change only (AR-17). No CLI, ABI, profile, standard-library,
or emitted-artifact contract changes. Diagnostics keep their codes, messages,
helps, and spans; only which programs they fire on narrows (AR-22).

### Security

Not applicable as an input surface (AR-18). The change only removes rejections
the v1 checker produced for programs NLL proves safe, so accepted programs
remain sound under the same memory model.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Work covered | NLL only / also a deferred ledger item | NLL only | the ledger defers O8/F2/X1/O4 | AR-1 |
| Frozen spec | amend + rewrite golden / amend + drop golden / no edits | amend + rewrite golden | keeps a rejection case for every code and never weakens the oracle | AR-2 |
| Model | reachability regions / straight-line with v1 fallback | reachability regions | RFC 2094 and the spec's v2 target | AR-3 |
| Implementation | post-walk CFG pass / silent pre-walk / detached IR | post-walk CFG pass | reuses the graph the walk already builds; no second walk | AR-4 |
| Loan identity | shared loan id / union per owner | shared loan id | RFC behavior for copied references; no over-rejection | AR-5 |
| Uses | every access through a view / direct reads | every access | follows from the confirmed model | AR-6 |
| Same statement | evaluation order / statement granularity | evaluation order | RFC 2094 precision | AR-7 |
| E10030 | liveness + creation span / liveness + use span | liveness + creation span | keeps `spec/14-diagnostics.md:58` output stable | AR-8 |
| Diagnostics | deterministic post-pass / inline via pre-walk | deterministic post-pass | no second walk; determinism preserved | AR-9 |
| Dead state | remove and migrate tests / keep | remove and migrate | no-dead-code rule | AR-10 |
| Flipping tests | rewrite to still reject / reclassify as acceptance | rewrite to still reject | preserves each code's coverage | AR-11 |
| Test placement | new dedicated files / extend suites / new goldens | new dedicated files, no goldens | matches the repo's per-feature test convention | AR-12 |
| Verification | `make check` per task, `make verify` at completion / check only | both | the repo's accepted protocol | AR-13 |
| Complexity | ordinary feature code / escalation packet | ordinary feature code | the analysis is the feature, not support machinery | AR-23 |

> **Traceability:** every scope decision above cites the Ambiguity Register
> entry that resolved it — see `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] R1–R9 implemented, each traced to a passing `ST-` case in
   `07-testing-strategy.md`, with the close-out evidence recorded in
   `99-execution-plan.md`.
2. [x] `make check` is green after every task; `make verify` is green before
   completion (AR-13).
3. [x] The spec amendments of `03-03-spec-amendment.md` are present, reviewed,
   and consistent with the code; the rewritten golden still fails with
   `E10031`.
4. [x] No dead code: the lexical `Loans` set and its rules are gone, and no
   unused parameter, function, or module remains.
5. [x] Every pre-existing test that the model no longer rejects was rewritten
   to still pin its rule, and each rewrite is recorded in
   `99-execution-plan.md` (AR-11).
6. [x] Documentation updated: the plan state, the roadmap row, and
   `AGENTS.md` only if a durable fact changed.
