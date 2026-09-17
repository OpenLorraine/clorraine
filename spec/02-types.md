# Chapter 02 - Type System

> **Status:** planned
> **Depends on:** Ch 03 (ownership), Ch 12 (contracts), Ch 15 (profiles)
> **Derived from:** Blend65 Ch 02 (mechanism kept, width model replaced)

## Scope (planned)

* Primitive types: `i8`, `i16`, `i32`, `u8`, `u16`, `u32`, `f32`, `f64`,
  `bool`, `void`; pointer-sized `usize`/`isize` for profile portability
  (m68k 32-bit today, ARM64 64-bit later); `'x'` is a `u8` byte literal.
* No implicit mixed-sign or narrowing conversions; explicit casts only
  (`T(expr)` syntax kept from Blend65; no C-style casts).
* Parameter modes: unmarked/`const` read-only lend, `borrow` writable lend,
  `take` ownership transfer; `raw T` unchecked escape hatch; no pointer
  arithmetic outside `unsafe`.
* Handles: opaque owned types produced by contracts (`handle File`); their
  representation is profile-defined; release comes from the contract.
* Aggregates: structs (Ch 07), arrays and slices (Ch 08), C-style tagged
  structs (Ch 09); no tagged unions.
* Duplicability derived from fields; move-only types (Ch 03).
* Volatile access for hardware registers: a type qualifier (`volatile`) that
  lowers to C `volatile`; required by the Amiga appendix and later by
  LorraineOS drivers.
* Deterministic arithmetic: wrapping semantics for fixed-width integers
  (A3); no implementation-defined overflow.

## Open questions

| Question | Recommendation |
|---|---|
| Integer literal suffix spelling | `1u8`, `2i32` style; final list with Ch 01 |
| `usize` on m68k | 32-bit; never assume it fits a pointer on ARM64 |
| Volatile spelling | keyword or type constructor; decide with naming pass |
| Tagged-union syntax | Ch 09; needed before `Option`-style nullable types |
