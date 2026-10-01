# Requirements: Residual Defect Hardening

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The plan makes four accepted programs correct. Each requirement below is a filed defect with a
reproduced failure; the expected behavior is stated as the program-level outcome, not as an
emission detail (the component specs own the mechanism).

The defining constraint is the frozen specification: every expected behavior already follows from
`spec/`, and the plan aligns the implementation with it. No new language rules, no new
diagnostics, and no new test harness.

## Functional Requirements

### Must Have

- [x] **R1 — DEF-46, flat literal types.** A no-context array literal whose elements are
  themselves literals has the same flat fixed-array type a context-typed literal has, so type
  equality, `iterated_element`, and the C declarator agree, and the type is canonical on every
  reachable construction path (the constructor and generic substitution). `let lit = [[1, 2], [3, 4]];`
  declares a `i32[2][2]` value and reads correctly. (C symbol names still fold a fixed-array type
  to its base element; the duplicate-symbol collision for distinct fixed-array type arguments is
  out of scope and filed as DEF-50 — see Won't Have.)
- [x] **R2 — DEF-46, deferred row store.** A deferred array literal whose nested row is not itself
  a literal (a conditional, a place, or a call) compiles to valid C and evaluates the chosen row
  once on the deferred path. `ok && [c ? rows[0] : rows[1], [5, 6]][0][0] == 1` compiles and
  computes the chosen element.
- [x] **R3 — DEF-47, multi-extent return.** Returning a fixed-array place of more than one extent
  copies every extent correctly. `func copy(take a: String[2][2]): String[2][2] { return a; }`
  builds, the caller reads the copied rows, and the source is emptied exactly once.
- [x] **R4 — DEF-48, unguarded loop release.** A loop condition holding an owned call result
  releases it. `while (grow().count > 0 && i < 3)` and its `do`/`for` spellings run the correct
  number of times and leak nothing under AddressSanitizer.
- [x] **R5 — DEF-49, generic ownership.** A recorded generic instance whose substituted types own
  storage releases what the body owns. `func keep<T>(take a: T[2]): i32 { return 0; }` with
  `T = String` and a `String[2]` argument frees the parameter; a body that stores a param-typed
  value moves under an owning substitution (no double free), including a `take` argument place,
  which the caller empties and the callee owns; a `borrow` parameter is never dropped by either
  side; a body that reuses a moved-if-owning value reports the existing moved-value error
  (`E10020`) once per generic function and binding, at the body's use site.
- [x] **R6 — Non-owning instances unchanged.** An instance whose substituted types are duplicable
  or views keeps today's behavior: scalar substitutions add no release and reject nothing that
  compiles today. If the Phase 4 reachability probe finds a view outliving a param-typed
  temporary, the acceptance change it implies needs an explicit user ruling before it ships.
- [x] **R7 — No specification change.** No file under `spec/` changes; the defects are
  implementation gaps against the frozen oracle (AR #8).
- [x] **R8 — No new diagnostics.** No new diagnostic code; only existing codes are reported, and
  only where a substituted type makes an existing rule apply (AR #11).

### Should Have

- [x] **R9 — Close-out probes.** Pre-fix AddressSanitizer evidence for the leak-only defects
  (DEF-48, DEF-49) and clean post-fix probes for all four (AR #6).
- [x] **R10 — Docs synced.** The `AGENTS.md` test map gains the two new suites, and the roadmap
  DEF rows move to Done when the plan completes (AR #7).

### Won't Have (Out of Scope)

- Any specification text, golden, or diagnostic-registry change.
- The C-symbol collision when distinct fixed-array type arguments instantiate the same generic
  struct (`type_key` folds an array to its base element): `Grid<i32[3]>` and `Grid<i32[2]>` emit
  duplicate `ClGrid__i32` definitions. Filed as a separate residual defect (DEF-50); symbol names
  stay base-element only in this plan.
- A generalized ownership-analysis layer, a new IR pass over generic bodies re-checked from the
  source tree, or a new generic-substitution architecture. The obligation-record design is the
  approved mechanism (AR #17).
- Unrelated refactoring of the C spelling, the guard machinery, or the loop shapes beyond what
  the four defects require.
- New dependencies, crates, build scripts, or test harnesses; the workspace stays std-only
  (AR #14).
- New language features or diagnostics.

## Technical Requirements

### Compatibility

- All four fixes preserve the accepted-program set: no program that compiles today may be
  rejected, and no diagnostic that fires today may change, except the DEF-49 copy-conflict case
  that is currently unsound (R5). The flat canonicalization covers generic substitution, so a
  currently accepted `T[N]` instantiation with an array type argument stays accepted and starts
  emitting valid C.
- The emitted C stays valid C99 and builds under the host, hosted-amiga, and freestanding
  profiles; the cross-toolchain legs (`make verify`) stay green.
- Emission stays deterministic: the same source emits byte-identical C (AR #13).

### Performance

No measurable compile-time or runtime regression is required to be proven; the fixes add no
per-iteration work beyond the release DEF-48 already needs, and the per-instance audit runs once
per recorded instance.

### Security

N/A — the change is compiler-internal and adds no input, parsing, or output surface (AR #15).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Plan shape | One plan / split DEF-49 | One plan, four phases + close-out | The four defects share the fixed-array and ownership machinery; one plan keeps the change reviewable together | AR #1 |
| Array literal types | Flat canonical / nested-aware spelling / expression-only | Flat canonical (constructor + substitution) plus expression store | Flat extents are the existing convention; the nested form already breaks conditional typing, and generic substitution builds it too | AR #3 |
| Return copy | Delegate to the store core / hand-extend the loop | Delegate | The core already recurses extents, zeroes the source, and gates on `needs_drop` | AR #12 |
| Loop release | Guarded flag-first reuse / comma-expression drop | Guarded flag-first reuse | The guarded path already drops before re-store and releases after the loop | AR #5 |
| Generic ownership | Targeted pass / AST re-check / obligation records | Obligation records discharged per instance | Fits the checked-once architecture; deposits symbolically and discharges with the substitution | AR #17 |
| Diagnostics | New instantiation code / reuse `E10020` | Reuse `E10020` | No new codes (R8); the substituted copy is exactly a moved-value conflict | AR #11 |
| Regression suites | New files / extend call-value-integrity suites | New `residual_defect_hardening_{spec,impl}_test.rs` | Matches the per-feature suite convention | AR #2 |

## Acceptance Criteria

1. [x] R1 – R8 hold, each pinned by a specification case in `07-testing-strategy.md`.
2. [x] Every phase's spec tests fail before its implementation (or are recorded pre-passing
   regression pins) and pass after it.
3. [x] The full workspace suite, the corpus, and the determinism suite pass; `make verify` passes
   every cross-toolchain leg.
4. [x] The DEF-48 and DEF-49 leak probes are clean under `ASAN_OPTIONS=detect_leaks=1`, with the
   pre-fix leak recorded for each.
5. [x] `AGENTS.md` maps the new suites and `plans/00-roadmap.md` marks DEF-46 – DEF-49 Done with
   this plan linked.
