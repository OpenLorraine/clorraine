# Requirements: Call Value Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Fix the sixteen defects filed against the Clorraine compiler as DEF-30 – DEF-45. Each defect is an
accepted program (or a spec-mandated rejection) whose lowering, emission, or resolution does not
follow the value semantics of the frozen specification under `spec/`. The feature delivers correct
owned conditionals (handles and owned aggregates) with every adopting site and deferred-take
ownership, valid C for fixed-array values in stores, arguments, projections, and struct-literal
fields, correct function-pointer declarators and callable resolution, and correct guard/cleanup
emission. It adds no language surface and changes no
specification text.

## Functional Requirements

### Must Have

- [x] **R1 — Owned conditionals (DEF-34):** a move-only value selected by `?:` in a `let`,
  assignment, owned argument, `take` position, `return`, or struct-literal field moves the chosen
  arm into the destination; each arm counts as maybe-moved afterwards; the destination owns exactly
  one value. The destination rule keys on `needs_drop` and not-fixed-array, so owned aggregates
  (structs with owned fields) are covered. An assignment drops or saves the destination's old value,
  and a self-naming arm restores it, mirroring the fixed-array store.
- [x] **R2 — Deferred take results (DEF-45):** a take call deferred into a guard releases its
  transfer temporary exactly once when the value is discarded, and transfers ownership without a
  double release when a binding, an assignment, a `take` argument, a `return`, or a
  struct-literal field adopts it. A `take` argument materializes into a non-releasing argument
  temporary (the callee's parameter cleanup owns the value).
- [x] **R3 — Element arms (AR #3):** a conditional over move-only element places reports `E10026`
  (spec/03-ownership.md:126-128); a conditional over duplicable element places copies and runs.
- [x] **R4 — Indexed fixed-array stores (DEF-31):** runtime and constant index element stores on a
  fixed array emit valid C with byte-copy semantics; a nested array literal from variables stores
  element-wise.
- [x] **R5 — Conditional fixed-array arguments (DEF-32, DEF-33):** a conditional argument whose
  arms are call results, literals, or duplicable values materializes the chosen value per path.
- [x] **R6 — Nested fixed-array call results (DEF-32):** a by-value fixed-array call result passed
  as an argument without a destination gets one.
- [x] **R7 — Array-literal projections (DEF-35):** `[a, b][0]` in value and call-argument
  positions materializes the literal and reads the element through a valid place.
- [x] **R8 — Struct-literal array fields (DEF-38):** a conditional (or otherwise place-less) array
  value in a struct-literal field emits a store-based field initialization.
- [x] **R9 — Func-pointer declarators (DEF-30, DEF-43):** return-position function types, closure
  struct `fn` fields, and lifted-lambda signatures use the shared func-pointer declarator,
  including the hidden destination for fixed-array returns.
- [x] **R10 — Callable resolution (DEF-40, DEF-41):** a call by name through a lambda capture
  resolves to the environment field, and through a module-level func-typed global to the global
  symbol, in both lowering and emission.
- [x] **R11 — Func-value widening (DEF-42):** an implicit `String` → `str` argument to a
  func-value callee is widened from the callee's function type, not only from declared signatures.
- [x] **R12 — Borrow func-type parameters (DEF-44):** `FuncTy` carries each parameter's mode; the
  declarator and the call site pass the pointer for a `borrow` parameter.
- [x] **R13 — Array-parameter zeroing (DEF-36):** moving out of a by-reference fixed-array
  parameter zeroes its elements (recursively), never the parameter pointer; true locals keep the
  byte-zero `cl_zero` form.
- [x] **R14 — Do-while guarded conditions (DEF-37):** the condition block of a `do`/`while`
  evaluates after every body iteration, including conditions with guards.
- [x] **R15 — Growable drops in embedded guards (DEF-39):** the embedded guard spelling drops a
  previously materialized growable-array value before re-materializing it, for element types that
  need no drop; the statement path's element loop has no C99 expression form, so an `Array<T>`
  whose `T` needs a drop keeps today's skip in that spelling (recorded in Won't Have).
- [x] **R16 — Regression safety:** the frozen specification suites are unchanged; the closed
  plans' spec oracles keep passing; implementation/unit pins that describe replaced behavior are
  updated in place (AR #15).
- [x] **R17 — Verification:** every task runs `make check` before its checkbox flips, every phase
  re-runs it, and close-out runs the full acceptance and the AddressSanitizer probes (AR #16,
  AR #17).
- [x] **R18 — Documentation and tracking:** the project guidance test map and the roadmap are
  updated; the plan declares the sixteen defects (AR #18).

### Won't Have (Out of Scope)

- New language features or specification changes; move-only element arms stay `E10026`.
- A generalized ownership/verification layer or new IR pass; fixes are position-specific.
- Patterns beyond each filed defect's shapes and the named variants confirmed in AR #13.
- Changes to the closed plans' specification suites (see R16).
- `take` in a func-type parameter: after the `FuncParam` change the mode is representable but has
  no ownership semantics; the shape stays accepted-but-unhandled (do not rely on it).
- A growable `Array<T>` whose `T` needs a drop in an embedded guard keeps today's skip; only the
  statement spelling drops it correctly. The DEF-39 fix covers element types that need no drop.

## Technical Requirements

### Compatibility

- The frozen `spec/` corpus and golden data must keep passing unchanged
  (`./spec/check.sh`, `compiler/tests/*_spec_test.rs`).
- The emitted C stays C99, `-Wall -Wextra -Werror` clean, for the `host`, `hosted-amiga`, and
  `freestanding` profiles.
- The m68k freestanding profile stays linkable; the m68k ABI spellings must not regress.

### Security

- The compiler is off-line and reads only repository files; the defects are memory-safety and
  code-validity issues in emitted code. The mandatory security check for this feature is the
  ownership/leak verification (ASan probes, AR #16), which tests for double-free and leak classes.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Plan shape | one plan / two plans / per-defect | One plan, phased by cluster | Shared machinery and one verification story | AR #1 |
| Owned conditional mechanism | branch store + transfer-aware release / branch store only / defer | Branch store + transfer-aware release | Fixes the double-free and the leak together | AR #2 |
| Element arms | accept / keep `E10026` | Keep `E10026` | Spec limits element moves in v1 | AR #3 |
| `borrow` func types | implement / reject / defer | Implement | Grammar sanctions the mode | AR #4 |
| Test placement | new suites / extend existing | New suites | The closed plan's oracles stay frozen | AR #5 |
| Struct-literal array field | zero-init + field store / temp + memcpy | Zero-init + field store | Reuses the fixed-array assign machinery; keeps evaluation order | AR #7 |
| Do-while guarded condition | goto / restructure | Goto-free flag-first loop | The goto conflicts with the frozen codegen contract; the restructure handles fall-through, `continue`, and embedded-only guards (preflight PF-001/PF-002) | AR #8 |
| Growable drop in guards | add expression form / statement fallback | Limited expression form (no-drop elements) | The statement path is an inline loop, not a runtime call; the limit is stated in Won't Have (preflight PF-010) | AR #9 |

> **Traceability:** Every scope decision references the Ambiguity Register entry that resolved it.
> See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] Every specification test case (ST-1 – ST-41) passes; at least one case per defect fails
   before its fix (red) and passes after (green). Cases that already pass before the fix are
   recorded as regression pins in `07-testing-strategy.md` and are not counted as red-phase
   evidence.
2. [x] `make check` passes with no warnings; `make verify` reports all legs passed.
3. [x] The AddressSanitizer probes for the memory-safety defects (DEF-34, 36, 37, 39, 45) run
   host-side with `clang -std=c99 -O0 -g -fsanitize=address` and `ASAN_OPTIONS=detect_leaks=1`,
   using the probe programs named in `07-testing-strategy.md`, and report clean. The pinned
   container has no ASan runtime, so the probes run on the host.
4. [x] `DEF-30` – `DEF-45` are marked Done in `plans/00-roadmap.md` with a link to this plan.
5. [x] Documentation updated: `AGENTS.md` test map and roadmap rows; the closed plans' spec suites
   unchanged.
