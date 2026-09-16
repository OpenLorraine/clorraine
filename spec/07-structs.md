# Chapter 07 - Structs

> **Status:** planned
> **Depends on:** Ch 03 (ownership), Ch 02 (types)
> **Derived from:** Blend65 Ch 07 (syntax kept; copy semantics replaced)

## Scope (planned)

* Declaration: `struct Name { field: T; ... }`; nominal types; no inheritance.
* Struct literals: all fields in declaration order; no positional omission.
* `copy struct`: allowed only when every field is `copy`; freely assignable.
* Move-only structs: any struct with an `own` field (or non-`copy` field);
  assignment moves and zeroes the source (Ch 17 SS5).
* Drop order for struct fields: declaration order at scope exit, each field
  dropped with its contract deleter; enforced null-safety per field.
* v1 restriction: no `borrow` fields (E10036); no self-referential structs
  without raw pointers.
* Layout: C-compatible layout guaranteed for hosted FFI; field reordering is
  not performed (predictability over packing).
* Methods: none in v1; module functions take `borrow`/`borrow mut` parameters.

## Open questions

| Question | Recommendation |
|---|---|
| `copy` spelling | Marker before `struct`; naming pass decides |
| Padding and `packed` | No packing in v1; explicit byte arrays at boundaries |
| Opaque FFI structs | Declared as handles; no field access until imported |
