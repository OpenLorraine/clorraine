# Requirements: Conditional and Projection Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Overview

Batch H closes 14 defect rows: the 13 filed rows of the conditional/projection ownership cluster
(`DEF-176` – `DEF-188`) and one row the batch's scoping probes found (`DEF-256`). Every requirement
below is a conformance statement against the frozen specification; none adds language surface, and
no diagnostic row is added or changed.

## Functional Requirements

### Conditional arm ownership

| # | Defect | Requirement | Frozen reference |
|---|--------|-------------|------------------|
| FR-1 | DEF-176 | A projection of a conditional in a consuming position (a store, a return, a `take` argument, a defer body) **moves the selected member out of the chosen arm** and leaves the place arm's remaining fields readable; the fresh arm's root releases exactly once. No leak, double free, or read of freed storage at any spelling. | Ch 03 §3 (moves), §4 (partial moves), §5 (drops); O7 zero = empty; Ch 04 §5 (operator semantics) |
| FR-2 | DEF-178 | A guarded intrinsic argument of an owned conditional projection (`c && len((c ? b : mk()).v) > 0`) binds the argument's root through the per-arm guard so the chosen arm's owner releases exactly once and the place arm is only read. | Ch 03 §3/§5; Ch 04 §11 (intrinsics), §6 (bool conditions) |
| FR-3 | DEF-179 | A nested conditional arm's owned store (`d ? (c ? b : mk()).v : s`) moves the member out of the chosen root; the store adopts one owner and the other root releases its un-moved fields once. | Ch 03 §3/§5; Ch 04 §5 |

### Guarded receivers and loans

| # | Defect | Requirement | Frozen reference |
|---|--------|-------------|------------------|
| FR-4 | DEF-180 | The conditional place arm registers the same shared loan as the named spelling: mutating the place while the conditional view is live reports `E10031` once at the mutation site. | Ch 03 §7 (E10031); Ch 16 §3 (loans) |
| FR-5 | DEF-181 | A guarded struct-literal receiver root (`true && H { call, s }.call(5) > 0`, and the `\|\|`/`?:` taken paths) routes through the guarded projection binder: the receiver's storage releases once at the statement end. | Ch 03 §5; Ch 04 §6 (guarded operands) |
| FR-6 | DEF-182 | A dot-`len` receiver of a conditional view projection (`(c ? mk(1) : mk(2)).v.len()`) binds the conditional root so the root's unselected fields release once; the `len(cond.v)` spelling stays clean. | Ch 04 §8 (member access and dot-call), §11; Ch 03 §5 |

### View provenance and elision

| # | Defect | Requirement | Frozen reference |
|---|--------|-------------|------------------|
| FR-7 | DEF-177 | A `str[N]` binding of view elements records the same loan as the single-view twin: moving the backing while the binding is live reports `E10022`, and a binding of a temporary-derived element (`[mk().v]`) is judged in the elision postlude (`E10030`), never left dangling or leaking. | Ch 03 §7 (E10022/E10030); Ch 06 §3.1; Ch 16 §4 (EL-1..EL-6) |
| FR-8 | DEF-187 | A returned view's elision judgment follows the value's **actual derivation**: a derivation that does not name the single view-like lend candidate (e.g. from a non-view-like `Box` lend) reports `E10036` at the declaration; a derivation from the candidate stays accepted; no caller can obtain a view of a dying temporary. | Ch 06 §3.1 (EL-2/EL-3); Ch 16 §4 (BR-6/BR-7, E10036) |

### Statement-path discards

| # | Defect | Requirement | Frozen reference |
|---|--------|-------------|------------------|
| FR-9 | DEF-185 | A discarded closure call with a maybe-owning result uses the named-call sibling's owning predicate, so an owning instance's result releases exactly once. | Ch 03 §5; Ch 06 §6.1 (lambdas) |
| FR-10 | DEF-186 | A discarded call inside a conditional expression statement (`c ? make1() : make2();`) binds the selected arm's result so the statement's release frees it exactly once. | Ch 03 §5; Ch 04 §5 |
| FR-11 | DEF-256 | A discarded `.len()` statement lowers through the value-position intrinsic path, so the emitted C calls the defined helper (or folds), never a bare `len(...)`. | Ch 04 §11; Ch 17 (typed emission) |

### Emission spellings

| # | Defect | Requirement | Frozen reference |
|---|--------|-------------|------------------|
| FR-12 | DEF-183 | A projection of a conditional in return position renders both arms in one spelling; the accepted program builds strictly with `-Wall -Wextra -Werror`. | Ch 04 §5; Ch 17 |
| FR-13 | DEF-184 | A postfix update sharing an expression with a read pins the update into a temporary (`i++ + i`), so the emitted C has one sequence point per spec/04 §3 and builds strictly. | Ch 04 §3 (left-to-right evaluation), §5 (postfix yields the old value) |
| FR-14 | DEF-188 | An owned fixed-array projection of a call result (`func f(): u8[4] { return mk().xs; }`) binds the call root before taking the fixed-array address; the accepted program builds strictly. | Ch 03 §3; Ch 17 |

## Acceptance Criteria

1. Every FR above is covered by a specification test that is red at its phase baseline and green at
   the phase head, plus an implementation pin for the emitted shape or the report.
2. No accepted shape regresses: every case the batch does not target keeps its emitted C byte-identical
   except the classified deltas (the new bindings and spellings).
3. `make check` is green at every phase; `make verify` is green at close-out; the focused suites pass
   plain and under `RDC_ASAN=1` where the phase adds or moves a release.
4. No chapter or registry text changes; no new diagnostic code.
5. The batch's 14 roadmap rows end `Done ✅` with per-row evidence, and `AGENTS.md` lists both suites.
