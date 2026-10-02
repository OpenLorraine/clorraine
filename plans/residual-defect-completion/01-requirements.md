# Requirements: Residual Defect Completion

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The plan makes six accepted-but-broken shapes correct: three emit invalid or wrong C (DEF-50,
DEF-52, DEF-54), two leak or double free across a call boundary (DEF-53, DEF-55), and one leaks
a guarded re-store (DEF-51). Each requirement below is a filed defect with a reproducible failure;
the expected behavior is stated as the program-level outcome, not as an emission detail (the
component specifications own the mechanism).

The defining constraint is the frozen specification: every expected behavior already follows from
`spec/`, and the plan aligns the implementation with it. No new language rules, no new
diagnostics, and no new test harness.

## Functional Requirements

### Must Have

- [ ] **R1 — DEF-50, array-extent symbol keys.** Distinct fixed-array type arguments instantiate
  distinct C symbols: `Grid<i32[2]>` and `Grid<i32[3]>` coexist in one program, the emitted C
  builds, and the program reads the values it stored. Multi-extent arguments (`Grid<i32[2][2]>`)
  and owning element arguments (`Grid<String[2]>`) keep the same property, and slice arguments
  keep a key distinct from any fixed extent.
- [ ] **R2 — DEF-54, substituted borrow argument.** A `borrow` argument whose declared type is a
  projection of a generic-struct parameter (`sink<T>(borrow p.v)` with `p: Outer<T>`) is passed
  as the pointer the callee's C parameter expects, builds, and returns the callee's value; the
  argument stays a view (the caller's value is usable after the call and is not released).
- [ ] **R3 — DEF-52, lifted-lambda instances.** A lambda inside a generic function body is
  specialized for each enclosing instance: a generic call inside the lambda body is instantiated
  with the enclosing substitution, the emitted C builds, and each instance computes its own
  result. Two instances of the same enclosing function coexist; a capturing lambda whose captured
  value's type mentions the enclosing parameter is specialized too.
- [ ] **R4 — DEF-55, ownerless projection binding.** An owning read of a field projection from a
  fresh call result (`return mkv().a`, `let s: String = mkv().a`, `sink(mkv().a)`) binds the
  result to an owned temporary, moves the projection out, and releases the remaining fields
  exactly once. The value handed to the caller or callee is the projection; no sibling leaks and
  nothing is freed twice.
- [ ] **R5 — DEF-53, ownerless element take under an owning substitution.** `sink<T>(mk()[i])`
  instantiated with an owning `T` (for example `String`) releases the taken element exactly once:
  the callee owns the element it receives, the bound temporary releases the remaining elements,
  and the program is clean under AddressSanitizer. The concrete non-generic element take keeps
  its existing `E10026` behavior (no accepted program changes).
- [ ] **R6 — DEF-51, guarded drop-needing element.** A conditionally evaluated materialized
  operand whose type needs an element loop to drop (a growable `Array<T>` with a drop-needing
  `T`, or a struct containing one) releases its previous value before the re-store. The program
  computes the correct value and is clean under LeakSanitizer for the `while`, `do`, and `for`
  spellings. Types whose drop already has an expression form keep their emitted shape.
- [ ] **R7 — Compatibility.** No program that compiles today gains a new error, and no diagnostic
  that fires today changes, except that each of the six reproductions becomes an accepted,
  correct program. Concrete `E10026` element-take rejections are unchanged.
- [ ] **R8 — No specification change.** No file under `spec/` changes; no new diagnostic code is
  added (the defects are implementation gaps against the frozen oracle).

### Should Have

- [ ] **R9 — Determinism.** The same source emits byte-identical C across runs, including the new
  symbol keys and lifted specializations.
- [ ] **R10 — Close-out probes.** Pre-fix reproduction evidence is recorded and the post-fix
  AddressSanitizer/LeakSanitizer close-out probes for all six defects are clean.
- [ ] **R11 — Docs synced.** The `AGENTS.md` test map gains the two new suites, and the roadmap
  DEF-50 – DEF-55 rows move to Done with this plan linked.

### Won't Have (Out of Scope)

- Any specification text, golden, or diagnostic-registry change.
- The per-type drop-helper facility for DEF-51; the approved mechanism is the smaller statement
  hoist (AR-8).
- Per-instance obligation duties for the call-result cases; the approved mechanism is the
  bind-temporary design (AR-4).
- DEF-45-class transfer temporaries, the deferred element-move limits, and every other reviewed
  residual not named as DEF-50 – DEF-55 (strict scope, AR-5).
- New dependencies, crates, build scripts, or test harnesses; the workspace stays std-only.
- Unrelated refactoring of codegen naming, the guard machinery, or the instance walk beyond what
  the six defects require.

## Technical Requirements

### Compatibility

- The accepted-program set is preserved: no currently accepted program may be rejected, and no
  diagnostic that fires today may change (R7).
- The emitted C stays valid C99 and builds under the host, hosted-amiga, and freestanding
  profiles; the cross-toolchain legs (`make verify`) stay green.
- Emission stays deterministic (R9).

### Performance

No measurable compile-time or runtime regression is required to be proven. The fixes add no
per-iteration runtime work; the lifted specialization emits one function per generic instance
that uses a lambda, not one per lambda site in the whole program.

### Security

N/A — the change is compiler-internal and adds no input, parsing, or output surface.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Plan shape | One plan / two cluster plans / per-defect plans | One plan, two clusters | Shared machinery; DEF-53 and DEF-55 must be designed together | AR-1 |
| DEF-52 policy | Specialize per instance / reject with a diagnostic | Specialize per instance | Keeps the program accepted; no spec change | AR-2 |
| Array key encoding | `arr2_i32` extents / suffixed `i32_2` / struct-only fold | `arr2_i32` / `arr2x3_i32` | Deterministic, collision-free, readable | AR-3 |
| Call-result ownership | Bind temp and move projection / per-instance duty | Bind temp and move projection | Smaller; reuses the move and drop machinery | AR-4 |
| Scope baseline | Fix all six / allow a spec change | Fix all six under the frozen spec | The expected behavior follows from `spec/` | AR-5 |
| DEF-51 mechanism | Helper facility / statement hoist | Statement hoist | Existing statement path; no new machinery | AR-8 |

> **Traceability:** Every scope decision references the Ambiguity Register entry. See
> `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] R1 – R8 hold, each pinned by a specification case in `07-testing-strategy.md`.
2. [ ] Every phase's spec tests fail before its implementation (or are recorded pre-passing
   regression pins) and pass after it.
3. [ ] The full workspace suite, the corpus, and the determinism suite pass; `make verify` passes
   every cross-toolchain leg.
4. [ ] The six defects' close-out probes are clean under AddressSanitizer with leak detection on,
   with the pre-fix failure recorded for each.
5. [ ] `AGENTS.md` maps the new suites and `plans/00-roadmap.md` marks DEF-50 – DEF-55 Done with
   this plan linked.
