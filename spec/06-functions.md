# Chapter 06 - Functions and Parameters

> **Status:** planned
> **Depends on:** Ch 03 (ownership modes), Ch 12 (contracts)
> **Derived from:** Blend65 Ch 06 (syntax kept; SFA/interrupt ABI replaced)

## Scope (planned)

* Declaration: `function name(a: T, b: borrow U): R { ... }`; return type
  required; no overloading.
* Parameter modes: scalars by value; aggregates default to `borrow`;
  `borrow mut` and `own` are explicit (Ch 03 §6).
* Call sites: passing an `own` value moves it; passing an aggregate to a
  `borrow` parameter creates a temporary borrow for the call.
* Return: `own` values move out; `copy` values copy; borrows only from
  parameters with the single elided lifetime (OW-17).
* Function values: exact-signature function values exist for callbacks
  (needed by the Amiga appendix); no closures, no captured environments.
* Entry points per profile: `hosted-amiga` uses `main()`; `freestanding`
  defines its own startup later; exported functions use the C ABI exactly.
* FFI declarations: function signatures plus contracts (Ch 12); no body.
* `comptime function`: deferred; noted here because it affects name resolution
  when it lands.

## Open questions

| Question | Recommendation |
|---|---|
| Default aggregate parameter mode | `borrow` (read-only) for DX; `borrow mut` explicit |
| Inline/extern markers | Not in v1; the emitter decides `static` vs external |
| Error returns | C-like: nullable handles and enum results; no `Result` |
