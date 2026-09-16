# Chapter 08 - Arrays, Slices, and Strings

> **Status:** planned
> **Depends on:** Ch 03 (ownership), Ch 07 (structs)
> **Derived from:** Blend65 Ch 08 (literal syntax kept; pointer decay removed)

## Scope (planned)

* Fixed arrays: `T[N]`, size a compile-time constant; no decay to pointers.
* Fill literals: `[value; count]` and `[; zero]` (Blend65 sugar kept).
* Slices: `borrow T[]` / `borrow mut T[]`, a (pointer, length) pair; bounds
  available via `len`; indexing is bounds-checked in debug builds and
  unchecked in release (documented; opt-in always-check later).
* Strings: `string` is a byte sequence with a length; `cstring` is the
  NUL-terminated form for the C boundary; conversions are explicit.
* Owned buffers: `own Buffer` with an allocator contract; arrays of owned
  elements drop element-wise in index order.
* String literals are `copy` static data (profile places them in read-only
  storage).
* No pointer arithmetic on slices; slicing by range creates a sub-borrow with
  the same owner.

## Open questions

| Question | Recommendation |
|---|---|
| `string` vs `cstring` default | `string` (length-carrying) in language code; `cstring` at FFI |
| Bounds-check policy | Debug on, release off in v1; profile flag later |
| `embed` assets | Ch 13; arrays of bytes with known length |
