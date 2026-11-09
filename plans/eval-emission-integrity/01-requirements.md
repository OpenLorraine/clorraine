# Requirements: Evaluation and Emission Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The batch closes 24 defects across five layers: module initializers that emit broken or unsound
code (dangling hidden destinations, double-evaluated guards, backing-less slice literals), emitted
C shapes that fail the strict build in ordinary programs (settings from the earlier batches
onward, including the bounded-field call), checker judgments the frozen rules already decide (view
equality, `str` loops, member calls on bare parameters, null inference, generic projections, `len`
folds), two write-mutability rulings (value-position updates, `cstring` element writes), and five
ownership/release holes (a view-bound temporary leak, a defer-preempted return leak, a value-call
temporary leak, the block-bodied capture-return code, and conditional capture moves that
double-free). One further row (DEF-103) closes as the documented, fail-closed key-marker
limitation.

## Functional Requirements

### Must Have — Phase 1: module-initializer integrity

- [x] R1 (DEF-117): a fixed-array-returning call in any module-initializer position (whole value,
      element/member projection, user-fn argument, intrinsic argument, conditional arm) binds
      through a hidden destination and the unit builds; the function-body path is unchanged.
- [x] R2 (DEF-121): a module initializer evaluates every impure guard operand exactly once — the
      guard and the store share the materialized operand (`let v: u8 = s[idx()];` with a counting
      `idx` builds, bounds-checks, and runs once).
- [x] R3 (DEF-122): a slice-typed binding initialized from an array literal is rejected with
      `E10200` at the checker (module and function-body spellings); `slice(array, lo, hi)`
      initialization stays accepted.

### Must Have — Phase 2: strict-build emission shapes

- [x] R4 (DEF-76): a generic call whose fixed-array result feeds `len` emits no set-but-unused
      local; the unit builds under `-Wall -Wextra -Werror`, matching the concrete path.
- [x] R5 (DEF-97): the same class in the `codegen_run_spec_test` recording (reproduced first) is
      fixed together with R4 and the affected suite builds green.
- [x] R6 (DEF-80): an owned struct's empty return emits a `cl_zero_<ClType>` helper call, never a
      compound literal (spec/17-codegen.md:19-23); the view zero-helpers stay.
- [x] R7 (DEF-90): every freestanding unit with a `bool` (including module-initializer locals)
      builds under `-ffreestanding -Werror`; the freestanding preamble (or shim) provides the type.
- [x] R8 (DEF-98): a view passed to an uncontracted extern parameter coerces to its pointer — the
      prototype stays `APTR` and the call passes `.ptr` (or the equivalent); the strict build
      passes and the ABI table is unchanged.
- [x] R9 (DEF-112): a source binding named `__env`, `__cl_ret`, `__cl_tmpN`, `__cl_ownedN`, or
      `__cl_itmpN` de-duplicates against the generated spellings and the unit builds; no new
      rejections are introduced.
- [x] R10 (DEF-120): a multi-extent row slice of a user struct emits its typedef after the struct
      definitions (pointer-element slices stay before), so `-Werror` builds pass; the emission
      order record from the store-emission plan is revised with the new evidence.
- [x] R11 (DEF-128): a call of a callable-bound parameter's struct field builds and runs (the
      emission resolves the field through the instance field/closure call path, not an unqualified
      free-call spelling).

### Must Have — Phase 3: value typing and checker conformance

- [x] R12 (DEF-99): the growable-slice `len` fold-to-zero is reproduced first, then fixed (the
      same class as R13 where they agree).
- [x] R13 (DEF-111): `len(b.s)` inside a generic body with `b: T` substituted to a struct reads
      the instance field's real length, not the fallback `0`.
- [x] R14 (DEF-108): `let x = null;` keeps the nullable inference and the unit builds (the
      declaration and `null` spelling agree); the accepted pairing with a view (`if (x == xs)`)
      keeps its behavior.
- [x] R15 (DEF-107): `str`/`static str` pair equality reports `E10222` (the null-pair exemption
      and the `cstring` pointer comparison stay); the untyped-operand spelling reports too.
- [x] R16 (DEF-118): a `str` element loop reports the new `E10043` at the loop (reads and
      write-through spellings); fixed-array and slice loops are unchanged.
- [x] R17 (DEF-129): a member call on a bare type-parameter receiver reports `E10212` at the
      callee (one report per site); struct/enum receivers and callable-bound members are
      unchanged.

### Must Have — Phase 4: write-mutability rules

- [x] R18 (DEF-119, user-ruled): a value-position update goes through the statement-form checks —
      `E10023` on `const`, `E10024` on read-only parameters, `E10042` on writable-lend `str` —
      with exactly one report per site; statement forms are unchanged.
- [x] R19 (DEF-124, user-ruled): a `cstring` element write reports `E10042`; the registry row
      gains the `cstring` wording; whole-view rebinds and writable slice writes stay accepted.

### Must Have — Phase 5: ownership lifetimes and releases

- [x] R20 (DEF-95, user-ruled): a view bound from an owned projection temporary takes a
      binding-scoped owner that releases at the binding's scope end — the per-iteration leak is
      gone (ASan-clean) and the shape stays accepted.
- [x] R21 (DEF-96): when a deferred `break`/`continue` preempts a pending ordinary owned return,
      the abandoned value releases on the preemption edge; ordinary returns keep byte-identical
      emission where no preemption is reachable.
- [x] R22 (DEF-125): a func-value call applies its own func type's parameter modes, so an owned
      temporary argument binds and releases once (ASan-clean); the module-callee identity gate
      stays.
- [x] R23 (DEF-126, ruled consequence): a block-bodied lambda return or take of a by-value
      generic capture reports the ruled `E10401` (not `E10034`); the closure batch's block-body
      pin is updated deliberately.
- [x] R24 (DEF-127): a conditional capture return (`c ? x : x`) reports `E10401` in the
      expression-body, concrete, take-argument, and `[borrow x]` spellings, one report per site;
      the double-free is gone.

### Must Have — closure

- [x] R25 (DEF-103, user-ruled): the `Q`-marker collision stays as the documented fail-closed
      limitation; the row closes with its evidence (marker site, recursion, top-level analogue,
      fail-closed proof) and no code change.

### Won't Have (Out of Scope)

- The batch-A/closure rows already done; PLAN-05/11/13/14; any behavior the frozen rules decide
  differently from the recorded directions (none known — the register's conformance rows carry
  the readings).
- Static backing storage for slice literals (declined, AR-B5); an escaping key scheme (declined,
  AR-B3); `str`-loop support via typed `u8` views (declined, AR-B8); environment-owned captures,
  relay support, and the other batch-A declined alternatives (unchanged).

## Technical Requirements

### Compatibility

- The frozen chapters change only in the approved diagnostics (E10043 row, E10042 wording,
  E10212 trigger; AR-B6, AR-B11). Every other fix conforms the implementation to existing rules;
  the spec-delta whitelist is exact.
- Emission changes keep byte-identity where the plan pins it (DEF-96 ordinary returns, DEF-120's
  unaffected shapes, DEF-112's unaffected spellings); the corpus legs are the outer check.

### Security

- Memory safety is the surface: DEF-95, DEF-96, DEF-121, DEF-125, and DEF-127 carry host ASan
  probes as their oracle; the strict-build fixes must leave no new run-time paths.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Batch membership | 18 rows / 23 + fresh / trim | 24 fix rows + 1 closure | agreed scope; fresh rows are the same family | AR-B1 |
| DEF-95 owner | scope-owned / reject | scope-owned | memory-correct, shape stays valid | AR-B2 |
| DEF-103 | documented / diagnostic / escaping | documented | fail-closed; escaping is an ABI trigger | AR-B3 |
| DEF-108 | keep inference / reject | keep inference | the type system supports it | AR-B4 |
| DEF-122 | reject / static backing | reject | no new machinery; mirrors the rules | AR-B5 |
| DEF-118/129 codes | E10043+E10212 / variants | both approved | the only fitting landings | AR-B6 |
| Phase structure | per layer (5) / per defect | per layer | shared review surfaces | AR-B16 |

> **Traceability:** every scope decision back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Acceptance Criteria

1. [x] All 25 requirements verified by the ST cases in [07](07-testing-strategy.md); R2 and
       R20–R22 and R24 additionally by host ASan probes.
2. [x] The E10043 golden and the E10042/E10212 wording changes are live; `./spec/check.sh` and
       the compile-fail driver green.
3. [x] No regression: `make check` green per phase; the batch's suites plus the existing closure,
       store-emission, borrow-generic, defer, slice-temporary, and freestanding suites stay green.
4. [x] `make verify` legs passed at close-out; DEF-103's closure recorded with its evidence.
