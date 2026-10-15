# Requirements: Residual Major Closeout

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Complete the 20 open 🟠 major defect rows named by AR-1, severity-first
(the four batch-J follow-on majors and the two ruling-parked majors stay out, AR-2):
six phases grouped by origin machinery, 19 red rows implemented with red → green cases and
`DEF-209` verified fixed at the planning head and pinned as a control. Every row was re-probed
against the compiler built from head `80433a6` (AR-3). Each requirement below is conformance to
the frozen specification: the fix makes an accepted program build and run with the language's
semantics, stops an invalid-C acceptance, or makes a report match the frozen codes. No new
language surface, no new diagnostic code, no specification or registry edit (AR-5, AR-6, AR-7,
AR-12). The two roadmap-open behavior decisions are resolved inside the frozen rules: `DEF-233`
rejects the value-form enum member read with the registered `E10212` (AR-6), and `DEF-211`
measures the whole fixed-array type (AR-7).

## Functional Requirements

### Must Have

- [ ] **R1 — Module-initializer call resolution, guards, and nullable view conditionals**
  (`DEF-196`, `DEF-197`, `DEF-207`, `DEF-219`, `DEF-220`, plus the `DEF-209` control): an
  initializer call whose callee is an expression resolves the callee like the body path and
  stores the call's result, not its argument (`h = (5)` stops); an inferred generic call at an
  initializer top level (including the view-builder spelling) records its instance and emits the
  keyed callee; a conditional at a nullable view destination keeps the null arm's nullness
  (each arm builds its own view; the null arm is the zero view) so the length read never
  dereferences a null handle; an owned bind in a short-circuit right side releases inside the
  block that declares it. `DEF-209`'s projection-root shapes stay build-clean and ASan-clean.
- [ ] **R2 — Generic element placeholder, inference, and bounds** (`DEF-229`, `DEF-230`,
  `DEF-231`, `DEF-232`): the type stage binds the loop element of an opaque iterable as
  `#element of T`, the substitution/key derivation/audit map it to the substituted argument's
  one-step element, so a member read of a loop element passed to a generic callee instantiates
  (no `cl_id__unknown`), a closure capture's environment field carries the element type (no
  `i32` field for a struct element), an unresolved inferred type argument (the element
  placeholder, the null shape) resolves before the key is derived, and an index of a row
  element gets the frozen judgment: `E10216` for a constant out-of-range index at the instance
  and the `E11211` bounds trap for a run-time index (the concrete twin's parity).
- [ ] **R3 — Member typing: enum value form and view members** (`DEF-233`, `DEF-238`): the
  value-form enum member read (`e.A` on a value) reports the registered `E10212` on the concrete
  and the instance paths (Ch 04 §8's member form is `Type.MEMBER`; AR-6), while the scoped
  `E.A` and qualified `A.B.f` forms stay accepted; the view members `.ptr` and `.len` carry
  their checked types in the member dispatch (the view's pointer; `usize`), so an integer
  destination reports the frozen `E10200` where the C build used to fail, and a `cstring`
  destination keeps the existing pointer-member conversion.
- [ ] **R4 — Call conversions and `ffi` result typing** (`DEF-242`, `DEF-243`): a
  parameter-typed argument, return value, or declaration value whose instance resolves to a
  concrete destination that cannot receive it reports the frozen `E10200` once per site (the
  per-instance conversion duty, AR-8), while the legally converting instances stay accepted; an
  `ffi` call's result carries the declaration's resolved return type, so a `Count(): i32`
  result at a `cstring` parameter reports or converts per the frozen table instead of failing
  the strict build.
- [ ] **R5 — Null-shaped destinations and capture slot stores** (`DEF-245`, `DEF-251`): a
  null-shaped value at a nullable view or handle destination (store, argument, return) lowers
  to the destination's null value (the zero view / `NULL`) per Ch 17's `T?` lowering, so the
  accepted spellings build on every profile (the stop condition in AR-13 applies); a writable
  `[borrow x]` capture stored into a nullable owning out-slot is judged as a capture move (the
  carve-out no longer exempts a capture source), so the accepted double-free spelling reports
  the frozen capture rule instead of aborting at run time.
- [ ] **R6 — Standalone emission shapes** (`DEF-170`, `DEF-189`, `DEF-211`, `DEF-255`): a call
  through an imported or qualified func-typed global (and a field call on an imported or
  qualified struct global) emits the global's C symbol for every spelling; the freestanding
  shim's `_start` aligns the stack before calling `main`, so the host no-libc leg runs the
  16-byte-aligned shapes and the m68k link stays clean; `sizeof` of a fixed-array type is the
  size of the whole type (AR-7); a void expression-bodied lambda whose expression is an abort
  intrinsic statement (`assert`/`panic`) emits the abort, not a value return, so the strict
  build passes and the run aborts.

### Won't Have (Out of Scope)

- `DEF-167`, `DEF-169`, `DEF-171`, `DEF-190`, `DEF-191`, `DEF-263` — the six older rows parked
  for user rulings (AR-2): a non-callable-callee diagnostic code, an escaping type-key scheme,
  staged `embed`, a nameless nullable source diagnostic, the `unsafe` exemption, and the folded
  fixed-array `.len` receiver evaluation.
- The batch-J follow-on rows (`DEF-311`–`DEF-334` minus the closed `DEF-331`/`332`/`333`):
  they form the batch after this one (AR-2).
- Any new diagnostic code, registry row, specification chapter, or golden (AR-12); a row that
  turns out to need one is parked mid-phase with the evidence (AR-13).
- New language surface: the value-form enum member read is rejected, never given an invented
  constant interpretation (AR-6); no conversion is added to the frozen table (AR-12).

## Technical Requirements

### Compatibility

- The host, hosted-amiga, and freestanding profiles stay conformant; every accepted program
  keeps building under `-std=c99 -Wall -Wextra -Werror` on the pinned container, and the
  `DEF-245` handle spelling also under the hosted-amiga driver flags.
- Byte identity wherever a phase does not change the shape (the emission differential classifies
  every delta); the pinned origin suites and the `callable_release_integrity` /
  `checker_conformance` oracles stay green.

### Security

- Not applicable beyond the compiler's own input validation (unchanged).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Batch scope | 20 open majors / parked rows / batch-J follow-ons | 20 open majors | Severity-first; fresh evidence; the origin machineries group cleanly | AR-1, AR-2 |
| Phase grouping | six machinery phases / one per row / origin-batch phases | six machinery phases | Coherent review surfaces; dependency order P1 → P6 | AR-4 |
| `DEF-233` | reject / emit the constant | reject with `E10212` | Ch 04 §8's member form; no new surface; both paths | AR-6 |
| `DEF-211` | whole-type size / park | whole-type size | `sizeof(type)` is the size of the type; no spec edit | AR-7 |
| `DEF-242` | per-instance duty / declaration rejection | per-instance duty, stop-gated | Keeps legal instances accepted; reports the frozen `E10200` | AR-8 |
| `DEF-245` | typed-null lowering / `E10012` rejection | typed-null lowering, stop-gated | Ch 17's `T?` lowering; keeps the accepted spellings | AR-13 |
| Test placement | origin suites / new pair | origin suites, additions only | The machinery is the origin batches' (AR-12); a new pair only if a case has no natural home | AR-9 |

> **Traceability:** Every scope decision references the Ambiguity Register entry that resolved
> it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] Every one of the 19 red rows is closed with a reproduction that flips (red before, green
   after) and a suite pin that fails without the fix; `DEF-209` stays green as a pinned control
   (AR-3, AR-5).
2. [ ] Every ownership/handle fix is ASan-clean on the host (plain and `RDC_ASAN=1`), with loop
   and early-exit variants where the shape is re-executed.
3. [ ] The strict build passes for every accepted shape in scope on the pinned container; the
   freestanding host no-libc run and the hosted-amiga build are green for their rows (AR-11).
4. [ ] No behavior change outside the in-scope rows; the origin suites, the batch-H/I/J suites,
   and the `callable_release_integrity` oracle stay green.
5. [ ] `make check` green on every phase; `make verify` green at close-out.
6. [ ] The roadmap's 20 rows are accounted for (Done with per-row evidence; `DEF-209` recorded
   as incidentally fixed); the six older parked rows stay open with their rulings listed
   (AR-2); the batch-J follow-ons stay untouched.
7. [ ] No spec, registry, or golden changes; the new cases append to the origin suites only
   (AR-9); the `AGENTS.md` test map updates at close-out only if its wording needs the batch
   mention.
