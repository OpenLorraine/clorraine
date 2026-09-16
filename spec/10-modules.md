# Chapter 10 - Modules

> **Status:** planned
> **Derived from:** Blend65 Ch 10 (kept nearly verbatim)

## Scope (planned)

* One `module A.B;` declaration per file, first statement; dotted namespace
  path; filenames have no semantic meaning.
* Declarations only at module level; executable code lives in functions.
* Visibility: `export` makes a declaration public; otherwise module-private.
* Imports: named lists only, `import { X, Y as Z } from A.B;`; no wildcard.
* Qualified access without import always available (`A.B.f()`).
* Circular imports allowed; declaration merging across files of the same
  module name.
* Exactly one `main()` for executable builds; `--library` builds need none.
* Deterministic module initialization: `let` initializers are scheduled by
  dependency and effect order, with a total tie-break by fully qualified name
  (Blend65 decision kept); initializer cycles are E11100.
* FFI declarations (Ch 12) are module-level declarations and exportable.

## Open questions

| Question | Recommendation |
|---|---|
| Module init vs globals restriction (Ch 03 §8) | Initializers may not create `own` values that outlive startup |
| One type per file? | No; modules hold multiple declarations |
| Namespacing of generated C symbols | Module-qualified (Ch 17 §7) |
