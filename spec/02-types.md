# Chapter 02 - Type System

> **Status:** planned
> **Depends on:** Ch 03 (ownership), Ch 12 (contracts), Ch 15 (profiles)
> **Derived from:** Blend65 Ch 02 (mechanism kept, width model replaced)

## Scope (planned)

* Primitive types: `i8`, `i16`, `i32`, `u8`, `u16`, `u32`, `bool`, `void`;
  pointer-sized `usize`/`isize` for profile portability (m68k 32-bit today,
  ARM64 64-bit later).
* No implicit mixed-sign or narrowing conversions; explicit casts only
  (`T(expr)` syntax kept from Blend65; no C-style casts).
* Typed references: `borrow T`, `borrow mut T`, `raw T`; no pointer
  arithmetic outside the raw escape hatch.
* Handles: opaque owned types produced by contracts (`handle File`); their
  representation is profile-defined; release comes from the contract.
* Aggregates: structs (Ch 07), arrays and slices (Ch 08), tagged unions
  (Ch 09).
* `copy` type property and move-only types (Ch 03).
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
