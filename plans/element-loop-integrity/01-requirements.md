# Requirements: Element Loop Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The plan fixes two critical defects in accepted-program code generation. Neither
introduces new language surface, and neither changes the specification: both
restore behavior the specification already states.

**DEF-16 — owned-element reads must lend.** `spec/03-ownership.md` §2 defines the
unmarked and `const` parameter modes as read-only lends and `borrow` as a writable
lend; a lend "carries access, not responsibility, so it never releases." Reading a
container element (an owned array element, a field of one, or a field of a borrow
struct reached through an element) therefore must not give the caller a second
owner. Today the ownership stage binds a scope temporary (`__cl_ownedN`) for such
arguments and drops it, so the container and the temporary both free the same
handle.

**DEF-17 — every `for` runs its own step.** `spec/05-statements.md` defines the
three-clause `for` step and `for ... in`'s per-iteration index advance. Today a
`for` nested inside any other loop prints its own pre-header (the index
initialization) as the C `for` step and leaves the real step unreachable, so the
loop test never becomes false.

## Functional Requirements

### Must Have

- [x] R1 — A sub-object read of an owned container passed to any non-`take`
  parameter is a lend or view: no scope temporary owns it, and no drop of the
  argument happens. This covers element reads at constant and runtime indices,
  fields of elements, elements reached through a `borrow` parameter, fields of a
  `borrow` struct, and template placeholders. (AR #6, AR #13)
- [x] R2 — An ownerless temporary passed to a non-`take` parameter (a call result,
  such as `bytes(stringOf("hi"))`) keeps today's materialization and is released
  exactly once by the scope. (AR #6)
- [x] R3 — Move forms stay rejected unchanged: `take(arr[0])` and
  `let a: String = arr[0]` report `E10026` (partial moves are limited to top-level
  fields). (AR #2)
- [x] R4 — Every `for` nested inside any loop executes its own step: the C header
  carries the true increment, `continue` resumes at the step, and no dead step
  block or duplicated loop is emitted. This covers element loops and three-clause
  loops inside `while`, `for`, and element loops. (AR #3, AR #7, AR #8)
- [x] R5 — Non-nested loops, sequential loops, and every existing accepted
  program keep their current behavior and emitted C. (AR #7)

### Should Have

- [x] R6 — The emitted C stays deterministic: two emissions of the same program
  are byte-identical. (AR #10)
- [x] R7 — Every regression test that could hang fails with a bounded timeout
  instead of blocking the suite. (AR #10)

### Won't Have (Out of Scope)

- DEF-15, DEF-18, DEF-19, DEF-20 — stay filed in the roadmap.
- DEF-21 — a projection over an array-returning call (`mk()[0]`) emits invalid C;
  detected during planning, filed separately, untouched by this plan. (AR #4)
- Specification changes, new diagnostics, or new language surface. (AR #15)
- Corpus or demo changes.

## Technical Requirements

### Compatibility

- Both fixes are profile-independent host-side code generation changes. The host,
  `hosted-amiga`, and `freestanding` profiles are covered by `make check` per task
  and `make verify` before completion. (AR #11)

### Performance

- No runtime performance requirement; the fixes remove emitted work (a temporary
  assignment/drop, a dead block). Compile-time cost is unchanged in kind.

### Security

- Memory safety is the subject: DEF-16 is a double free. The close-out runs host
  AddressSanitizer probes over the fixed programs; no user input, secret, network,
  or storage surface exists. (AR #14)

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Plan structure | one plan / two plans | One plan with two independent components | Two small, sequential fixes sharing one test surface; the DEF-13 + DEF-14 precedent | AR #1, AR #2 |
| DEF-16 line of attack | narrow materialization guard / also lower constant-index reads to places | Narrow guard | Only `Index` values mis-own; the read path lowers, renders, and guards correctly today; runtime indices cannot become places | AR #6, AR #12 |
| DEF-17 line of attack | store the step in the IR shape / fix the search heuristic | Store the step | The builder already knows the step; the search has failed once and would remain a heuristic | AR #7, AR #8 |
| Moving elements | lend / reject | Lend (spec-derived) | `spec/03-ownership.md` §2: unmarked and `const` are read-only lends with "caller keeps" | AR #9 |
| Hang oracle | bounded run in both suites / impl suite only | Bounded run in both suites | The nested-loop ST cases themselves hang in the red phase; `cargo test` must not block | AR #10 |
| New defects found while planning | include / file separately | File separately (DEF-21) | Different root cause (array-returning call projection), not authorized scope | AR #4 |

> **Traceability:** Every scope decision must reference the Ambiguity Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] The DEF-16 reproducers (constant index, runtime index, borrow-param base,
   struct-element field, borrow-struct field, unmarked/`const`/`borrow` owned
   params, template placeholder) build and run without a double free and return
   the specified values.
2. [x] The DEF-17 reproducers (element and three-clause `for` inside element
   loops, `while`, `for`, and `do`/`while`, plus `continue` and `break`) run
   to completion and return the specified sums.
3. [x] Ownerless temporaries still materialize and release exactly once; move
   forms still report `E10026`.
4. [x] Emitted C is deterministic and unchanged for non-nested programs.
5. [x] `make check` is green after every GREEN task; `make verify` reports all legs
   passed before completion; host AddressSanitizer probes are clean at close-out.
6. [x] Documentation is updated: the `AGENTS.md` test naming map gains both suites,
   the roadmap rows for `DEF-16`/`DEF-17` are Done, and `DEF-21` is filed.
