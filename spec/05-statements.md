# Chapter 05 - Statements and Control Flow

> **Status:** planned
> **Derived from:** Blend65 Ch 05 (kept)

## Scope (planned)

* Blocks with mandatory braces; no dangling-else.
* Conditions must be `bool`; no truthiness.
* `if`/`else`, `while`, `do`/`while`, C-style `for`, `switch`.
* `switch` is C-exact: fallthrough by default, `break` to exit; no
  `fallthrough` keyword; an implicit-fallthrough warning is planned (B5).
* `break`/`continue` may leave scopes; the drop ladder (Ch 17 §4) guarantees
  releases on every exit edge.
* `goto` exists only in emitted C; the language has no source-level `goto`.
* `return` moves owned values or view types (F2); no `const`/`borrow` return
  markers.
* Escape blocks for raw operations (placeholder `unsafe`); blocks are greppable
  and may not contain `return` of a borrow created inside.

## Open questions

| Question | Recommendation |
|---|---|
| `defer`-style statement | Adopted in v1 (F5); joins the drop ladder |
| Labeled break | Deferred (Blend65 future item) |
| `comptime` blocks | Later phase; `comptime function` planned (Ch 13 context) |
