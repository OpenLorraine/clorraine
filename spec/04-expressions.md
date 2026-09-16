# Chapter 04 - Expressions and Operators

> **Status:** planned
> **Depends on:** Ch 02 (types), Ch 03 (ownership)
> **Derived from:** Blend65 Ch 04 (kept; address-of replaced by borrows)

## Scope (planned)

* Precedence and associativity: the C table, with no surprises; assignment is
  a statement, not a value-producing expression (avoids ordering questions in
  an ownership language).
* Evaluation order: strictly left-to-right, deterministic (A3).
* Arithmetic, comparison, bitwise, logical operators; `bool` is not an
  integer; no truthiness.
* Borrow creation: `&place`-style borrowing is expressed with the ownership
  mode syntax (Ch 03); no address-of operator on owned values outside the raw
  escape hatch.
* Dereference of `borrow`/`borrow mut`: implicit at member access, explicit in
  general.
* Casts: `T(expr)` only; no C-style casts; no implicit pointer casts.
* `sizeof`, `offsetof`, and array length accessors return `usize`; no
  pointer-difference arithmetic in safe code.
* Escape-block-only operations: raw dereference, raw pointer arithmetic,
  casts to and from `raw`.

## Open questions

| Question | Recommendation |
|---|---|
| Borrow expression spelling | Part of the naming pass; must read naturally in call sites |
| Method-style call sugar | None in v1; module functions only |
| Comparison of handles | Identity comparison allowed; ordering not |
