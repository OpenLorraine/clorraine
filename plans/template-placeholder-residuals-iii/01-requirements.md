# Requirements: Template Placeholder Residuals III

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Issue #29 records six adjacent residuals found by the #28/#24 reviews. Each
reproduces on the integration head `541ba27` and is a pre-existing gap. The
feature closes all six with the frozen rules and one delegated decision the
spec leaves open.

## Functional Requirements

### Must Have

- [ ] **R1 — Aliased `text`.** `import { text as t } from std.fmt;` makes
  `t(v)` the intrinsic: the arity, formattability, nullable-use, generic-duty,
  ownership, and `cl_text_*` conversion judgments all apply exactly as for the
  bare spelling. A lexical or module-level value named `t` still shadows the
  binding.
- [ ] **R2 — Qualified `text`.** `std.fmt.text(v)` applies the same intrinsic
  judgments only when no real declaration is reached through the path. A user
  function, `ffi` declaration, or module global named `std.fmt.text` wins, and
  a value binding of the head `std` makes it an ordinary dot/member path.
- [ ] **R3 — Generic condition context.** A generic conditional used directly
  as a condition reports E10218 at the conditional span when its substituted
  arms agree on a non-bool type, and does not also report E10012 at the null
  arm. A `&&`/`||` operand also keeps the operand judge's E10012, matching the
  concrete twin. A bool settlement keeps the ordinary null-arm rule; an
  arm-agreement failure keeps E10200.
- [ ] **R4 — Both-null conditional.** A conditional with two null arms settles
  to the null value: it is accepted where a bare `let z = null;` is accepted,
  E10012 at each written arm at a concrete destination, E10500 in a
  placeholder, and accepted at a nullable destination. The behavior is
  documented and pinned; the `Deferred` settlement name has no consumer for
  this shape, so no instance duty or settlement variant is added.
- [ ] **R5 — Null comparison with an open parameter.** A **left-literal**
  `null == x` / `null != x` with `x` still open defers to the instance; a
  substituted sibling that is not null-capable reports E10012 at the null. A
  null-capable substitution stays accepted. The right-literal `x == null` /
  `x != null` form stays the frozen generic narrowing idiom and is unchanged.
- [ ] **R6 — Null cstring panic.** `panic(v)` with a null `cstring` writes the
  panic prefix and newline without calling `strlen(NULL)` or passing a null
  buffer to `fwrite`.
- [ ] **R7 — For-step fixed-array assignment.** A for-step assignment whose
  target is a fixed array uses the flat loop and the ordinary store, so a call
  result or member source copies through the hidden destination/element path
  and the emitted C is valid.
- [ ] **R8 — Diagnostic consistency.** Every new report uses a frozen code,
  message, and help: E10500 for formattability, E10010 for a possible-null use,
  E10012 for a null at a concrete position, E10218 for a non-bool condition,
  E10308 for arity, and E10200 for an arm-agreement failure.

### Should Have

- [ ] **R9 — Emission stability.** Bare `text`, the `$text` sentinel, the
  template placeholder path, non-owning for steps, and existing null-arm
  emissions stay unchanged.
- [ ] **R10 — Shadow preservation.** A module-level or lexical value binding
  named `text` (or the alias) keeps the ordinary value-call path, as shipped in
  #24/#28, and a user `std.fmt.text` declaration is never treated as the
  intrinsic.

### Won't Have (Out of Scope)

- A general nullable dataflow analysis, a new null diagnostic, a new text
  framework, or a new loop representation.
- Any `spec/`, `stdlib/`, or ABI edit.
- Making `let z = null;` illegal (the both-null decision deliberately matches
  that existing rule).
- Changing the `len(cstring)` rule: a possibly-null cstring operand already
  reports E10010, so the shim guard is observable through the panic path only.
- Any other residual not named in issue #29.

## Technical Requirements

### Performance

- The changes are static name/diagnostic judgments, one deferred duty, three
  small shim guards, and one loop-shape predicate. No benchmark program uses an
  aliased/qualified `text`, a generic conditional condition, a null/comparison
  against an open parameter, a null cstring panic, or a fixed-array step
  assignment.
- `make bench-compare` runs against `541ba27`; every scored output must be
  equal and no scored metric may exceed a 1% regression.

### Compatibility

- Supporting aliases and the qualified path only widens valid programs; every
  existing bare-`text` behavior is unchanged.
- Rejecting a substituted non-null-capable null comparison tightens only
  programs that the concrete rule already rejects (a null against a
  non-null-capable type); the generic spelling now matches that rule.
- The both-null decision preserves every existing accepted/rejected case,
  including the typed-destination E10012-per-arm behavior.
- The cstring length change makes a previously crashing panic path return the
  empty length; a runtime-null cstring from an FFI boundary also gets the O7
  empty length. No valid non-null behavior changes.
- The fixed-array step form changes only a shape that previously failed the C
  build.

### Security

- No new input, parser, FFI, or `unsafe` surface. The shim null checks are
  internal length guards; no user data or secrets are involved (AR-13).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
|----------|--------------------|--------|-----------|--------|
| Aliased `text` | support / reject / leave | support | The import already resolves; the alias is the same helper | AR-1 |
| Qualified `text` | support / reject / leave | support, real declaration wins | The path names the same module helper unless a declaration shadows it | AR-2 |
| Lowered spelling | `text` / `$text` / duplicate codegen | `$text` sentinel | Handled before user-symbol resolution | AR-3 |
| Generic condition | carry flag / special-case / leave | carry flag | Matches the concrete one-shot rule | AR-4 |
| Both-null conditional | reject / accept as null / new variant | accept as null, documented + pinned | No concrete type exists; the `Deferred` return has no consumer | AR-5 |
| Concrete both-null | one / per arm / E10500 | per-arm E10012 | Existing destination rule | AR-7 |
| Null vs open parameter | defer / reject / leave | deferred E10012 (left literal) | Fixes the issue's `null == x` form and keeps the right-literal narrowing idiom | AR-9 |
| Null cstring | guard codegen / null-safe helper / leave | null-safe helper | One empty-length contract across shims | AR-11 |
| Array step | flat every fixed array / special-case | flat every fixed array | One predicate, no invalid C | AR-12 |
| Spec edit | yes / no | no | The frozen rules already decide every other shape | AR-13 |

> **Traceability:** every scope decision cites its owning Ambiguity Register
> entry; the register holds the full delegated provenance.

## Acceptance Criteria

1. [ ] All requirements R1–R10 are verified by the ST/IT cases in
   [07-testing-strategy.md](07-testing-strategy.md).
2. [ ] Every #29 reproduction behaves as specified under the host build, ASan,
   and UBSan.
3. [ ] `make check`, `make asan`, `make ubsan`, `make verify`,
   `make bench-shape`, and `make bench` + `make bench-compare` are green.
4. [ ] All scored benchmark outputs are equal; no scored metric regresses by
   more than 1%.
5. [ ] No critical or major review finding remains open.
6. [ ] The findings ledger, the roadmap row, and issue #29 reflect the
   delivered evidence.
