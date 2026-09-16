# Chapter 09 - Enums and Tagged Unions

> **Status:** planned
> **Derived from:** Blend65 Ch 09 (scoped enums kept; payloads added)

## Scope (planned)

* Plain enums: scoped names (`Direction.UP`), explicit backing integer type,
  no implicit conversions except explicit casts.
* Tagged unions: `union Name { VariantA(T), VariantB { x: u32; y: u32; } }`;
  exhaustive `switch` required by the checker (no missing-case errors at run
  time).
* Ownership inside variants: moving a payload out of a union requires the
  matching case and is checked; dropping a union drops the active variant.
* Nullable optional values are expressed as tagged unions (`Maybe<T>` in the
  prelude, spelling later); `own` handles use the zero-handle convention at
  the ABI boundary instead.
* Layout: deterministic tag plus payload; no C union punning in safe code;
  raw access only in escape blocks.
* `switch` exhaustiveness diagnostics live in Ch 14 (E110xx block).

## Open questions

| Question | Recommendation |
|---|---|
| `Maybe`/`Optional` spelling | Prelude type, generic later; avoid `null` in the language |
| Payload syntax | Struct-like payloads preferred (readable, no tuple types) |
| JSON-like literals | Not in v1 |
