# Requirements: Template Placeholder Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Clorraine templates are specified as strictly left-to-right expressions
(`spec/04-expressions.md` §3) whose placeholders hold formattable values.
Issue #26 shows that the implementation does not meet that contract: a
nullable `String?` placeholder dereferences NULL, a `str` call is evaluated
twice, a `String`/`str` placeholder is read after a later `take` sibling
released its source, and a view can be read after its owner was moved.

The feature makes every placeholder either well-defined or rejected by an
existing rule, with the same guarantee in function bodies, module
initializers, generic instantiations, and guarded positions. It adds no
language surface and changes no runtime helper.

## Functional Requirements

### Must Have

- [ ] **R1 — Nullable placeholder rule.** A `String?` or `str?` placeholder
  that is not narrowed reports E10010 (`value '{name}' may be null here`) and
  no translation unit is written. A narrowed placeholder formats the value.
  The rule also applies to a nullable instantiation of a generic placeholder,
  judged per instance.
- [ ] **R2 — Once, in source order.** Every placeholder expression is
  evaluated at its own source position, left to right, and evaluated once:
  a direct call, intrinsic, projection over a fresh root, view conditional,
  and owning conditional are covered. This includes function bodies, module
  initializers, `?:` arms, `&&`/`||` right operands, and loop conditions.
- [ ] **R3 — Copy before a later move.** A `String` or `str` placeholder's
  bytes are copied before a later sibling can move or release the source.
  `$"{v}{consume(s)}"` is accepted and prints the pre-move bytes. The reverse
  order (`$"{consume(s)}{v}"`) stays E10022.
- [ ] **R4 — Generic placeholders.** A type-parameter placeholder follows the
  same rules at each instantiation; `show<String>(s, s)` with a sibling `take`
  copies before the move, and a nullable instance is rejected unless the body
  narrowed the placeholder.
- [ ] **R5 — No language or ABI change.** No `spec/` edit; placeholder-free
  templates remain `str` literals; zero-length strings keep the #25 behavior;
  no new shim entry point.
- [ ] **R6 — Gate and budget.** The full project gate is green, every scored
  benchmark output is equal, and no scored metric regresses by more than 1%
  against `8b4bff2`.

### Should Have

- [ ] **R7 — Diagnostic consistency.** The nullable placeholder uses the
  frozen E10010 message and help, at the placeholder expression's span.
- [ ] **R8 — Emission clarity.** The generated C keeps one copy per
  string-like placeholder and one part per placeholder; the internal copy
  call never appears in the output.

### Won't Have (Out of Scope)

- #24 (`text` arity), #23, #22, #21, #20, #18, #17, #16, #15, #14, #13 — each
  stays a separate issue (AR-14).
- A new `ValueKind` variant, a new shim function, or a new formatting
  framework (AR-8).
- An instance-specialized generic fast path; generic scalar placeholders may
  pay one extra `String` conversion (AR-18).
- Any edit to `spec/` or to the emitted C ABI.

## Technical Requirements

### Performance

- Concrete scalar-only templates keep their current shape: no temporary is
  added when no placeholder has effects and none precedes an effect (AR-9).
- The `strings` benchmark's hot loop keeps its two direct scalar parts.
- `make bench-compare` enforces the 1% guard and equal outputs.

### Compatibility

- The `host`, `hosted-amiga`, `hosted-amiga-13`, and `freestanding` profiles
  keep their current behavior. Freestanding already rejects placeholders with
  E11200, so the new path is unreachable there.
- No public syntax, type, or ABI change; the nullable rejection is the
  existing TY-6 rule applied to a path that missed it (AR-13).

### Security

- No new input, parser, FFI, or `unsafe` surface. The copy lowers an
  already-checked expression and uses the existing runtime converter
  (AR-19).
- The copy temporary is adopted exactly once by the template part; no
  double free, use-after-free, or leak (R1–R4, AR-12).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
|----------|--------------------|--------|-----------|--------|
| Nullable placeholder | reject / empty string / crash | reject E10010 | Existing frozen rule and module-path precedent | AR-1, AR-2 |
| View before move | order / reject / crash | order | The frozen left-to-right and last-use rules permit it | AR-5 |
| View after move | reject / accept | reject E10022 | Already enforced; accepting would read freed storage | AR-6 |
| Generic placeholders | include / defer | include | Same crash is reachable through a generic wrapper | AR-7 |
| Copy mechanism | new IR kind / hidden call / new shim | hidden `$text` call | Reuses `text_call`; smallest emission surface | AR-8 |
| Scalar temporaries | always / conditional / never | conditional | Keeps the scored loop unchanged | AR-9 |
| Module initializers | include / defer | include once-only | The same rule applies to every evaluation context | AR-10 |
| Guarded positions | include / defer | include | Same root cause; existing fill machinery supports it | AR-11 |
| Spec edit | yes / no | no | The spec already states the required behavior | AR-13 |
| #24 and other issues | bundle / separate | separate | Strict scope; independent fixes | AR-14 |

> **Traceability:** every scope decision cites its owning Ambiguity Register
> entry; the register holds the full delegated provenance.

## Acceptance Criteria

1. [ ] All must-have requirements R1–R8 are verified by the ST cases in
   [07-testing-strategy.md](07-testing-strategy.md).
2. [ ] The four #26 reproductions plus the generic, guarded, loop-condition,
   and module-initializer variants behave as specified under the host build
   and under AddressSanitizer.
3. [ ] `make check`, `make asan`, `make ubsan`, `make verify`,
   `make bench-shape`, and `make bench` + `make bench-compare` are green.
4. [ ] All scored benchmark outputs are equal; no scored metric regresses by
   more than 1%.
5. [ ] No critical or major review finding remains open.
6. [ ] The plan's findings ledger, the roadmap row, and issue #26 reflect the
   delivered evidence.
