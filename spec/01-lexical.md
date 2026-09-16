# Chapter 01 - Lexical Structure

> **Status:** planned
> **Derived from:** Blend65 Ch 01 (kept; token set adapted)

## Scope (planned)

* Source encoding (UTF-8), comments (`//`, `/* */` non-nesting), whitespace.
* Identifiers and case sensitivity; naming conventions (camelCase functions
  and locals, PascalCase types, UPPER_SNAKE constants).
* Keywords: the final list depends on the pending ownership-keyword decision;
  the semantic modes from Ch 03 are fixed, the spellings are not.
* Literals: decimal/hex/binary integers with `_` separators, fixed-width
  suffixes (`u8`..`u32`, `i8`..`i32`), characters, strings, booleans.
* Statement termination: semicolons; no automatic insertion.
* No preprocessor: no `#define`, no `#include`; modules replace includes
  (Ch 10), contracts replace macros at the FFI boundary (Ch 12).

## Open questions

| Question | Recommendation |
|---|---|
| Ownership keyword set | Decide in a dedicated naming pass; DX criteria: shortest possible, no sigils, readable in signatures |
| Escape-block keyword (`unsafe` placeholder) | Same naming pass |
| String literal encoding | Bytes with explicit encoding only at boundaries; UTF-8 as source text |
