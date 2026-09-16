# Chapter 05 - Statements and Control Flow

> **Status:** planned
> **Derived from:** Blend65 Ch 05 (kept)

## Scope (planned)

* Blocks with mandatory braces; no dangling-else.
* Conditions must be `bool`; no truthiness.
* `if`/`else`, `while`, `do`/`while`, C-style `for`, `switch`.
* `switch` is auto-break with an explicit `fallthrough` statement (Blend65
  decision); `switch` on tagged unions is exhaustive (Ch 09).
* `break`/`continue` may leave scopes; the drop ladder (Ch 17 §4) guarantees
  releases on every exit edge.
* `goto` exists only in emitted C; the language has no source-level `goto`.
* `return` moves owned values; `return borrow` follows OW-17.
* Escape blocks for raw operations (placeholder `unsafe`); blocks are greppable
  and may not contain `return` of a borrow created inside.

## Open questions

| Question | Recommendation |
|---|---|
| `defer`-style statement | Not in v1; the drop model covers it |
| Labeled break | Deferred (Blend65 future item) |
| `comptime` blocks | Later phase; `comptime function` planned (Ch 13 context) |
