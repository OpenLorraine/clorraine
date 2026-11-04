# TextMate Grammar: Clorraine DX Tooling

> **Document**: 03-03-textmate-grammar.md
> **Parent**: [Index](00-index.md)

## Overview

The TextMate grammar moves to `editors/clorraine.tmLanguage.json` as the single canonical copy and
gains every keyword and reserved built-in name the frozen EBNF declares (AR-8, AR-9). The demo
toolkit consumes the canonical file; a workspace test enforces completeness so the grammar cannot
silently fall behind the language.

## Architecture

### Current Architecture

- `tools/demo/grammars/clorraine.tmLanguage.json` (134 lines, `scopeName: source.clorraine`,
  `fileTypes: ["clo"]`) is owned by the demo toolkit and loaded by
  `tools/demo/src/util/shiki.ts:9`.
- Its keyword pattern misses real terms: `as`, `do`, `in`, `from`, `export`, `freedBy`, `void`,
  `f32`, `f64`, `i16`, `u16`, `usize`, `isize`, `cstring`, and every reserved built-in; it lists
  the stale `deleter` (the EBNF keyword is `freedBy`).
- `tools/demo/test/grammar.spec.test.ts` asserts one scope per category over a fixed fixture.

### Proposed Changes

- The grammar file moves to `editors/clorraine.tmLanguage.json` and is removed from its old path
  in the same change, so only the canonical copy exists.
- `shiki.ts` points at the new location: `join(toolDir, "..", "..", "editors",
  "clorraine.tmLanguage.json")`.
- The grammar gains the missing keywords, the contextual `as`, an arrow operator, and a reserved
  built-in group; existing scope names and groupings stay untouched so the demo's colors do not
  churn.
- The demo fixture extends with the newly covered tokens.

## Implementation Details

### Completeness contract (test-enforced)

`spec/grammar.ebnf.md` is the oracle: every term of the `keyword` production (`§10.2`, 50 terms),
the `contextual_keyword` (`§10.3`, `as`), and every `reserved_builtin` (`§10.4`, 12 terms) must
appear word-bounded inside one of the grammar's pattern strings (`match`, `begin`, `end`). Scope
naming and grouping for the new terms are implementation choices; the existing ones stay.

### Scope map (new terms)

| Terms | Pattern group | Scope |
| ----- | ------------- | ----- |
| `as` | keywords | `keyword.control.clorraine` |
| `do`, `in`, `from`, `export`, `freedBy`, `comptime`, `defer`, `static` | existing groups they belong to by role (control for `do`/`in`, storage for the rest) | unchanged names |
| Types already listed (`i16`, `u16`, `f32`, `f64`, `bool`, `void`, `usize`, `isize`, `str`, `cstring`) | `types` | `storage.type.clorraine` |
| `sizeof`, `offsetof`, `len`, `embed`, `panic`, `assert`, `readVolatile8/16/32`, `writeVolatile8/16/32` | new `builtins` group | `support.function.builtin.clorraine` |
| `=>` | `operators` | `keyword.operator.arrow.clorraine` |

Best-effort, not test-enforced: capture lists (`[x, y]`) and generic argument lists stay
unscoped — TextMate cannot resolve them reliably, and the batch's completeness contract is the
keyword/built-in set (AR-9). The `deleter` spelling is replaced by `freedBy` (the EBNF is
normative).

### Consumers

| Consumer | Change |
| -------- | ------ |
| `tools/demo/src/util/shiki.ts` | Grammar path points at `editors/` |
| `tools/demo/test/grammar.spec.test.ts` | Fixture gains `as`, `do … while`, `for (x in xs)`, `embed`, `sizeof(T)`, `=>` rows with their scope assertions |
| Any other reference to the old path | `grep -rn "grammars/"` at execution; updated or confirmed absent |

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Grammar misses an EBNF term | The completeness spec test fails `make check` | AR-9 |
| Demo cannot load the moved file | `make demo-check` fails; the path update lands in the same task | AR-8, AR-11 |
| Grammar emits an invalid JSON document | The file keeps the existing hand-maintained JSON shape; `make demo-check` loads it through Shiki | AR-9 |
| A future EBNF keyword is added | The spec test fails until the grammar covers it | AR-9 |

## Testing Requirements

- Specification tests: ST-16 – ST-19 (`07-testing-strategy.md`) in
  `compiler/tests/dx_tooling_spec_test.rs` — the completeness scan reads
  `spec/grammar.ebnf.md` and `editors/clorraine.tmLanguage.json`; the path assertions read
  `tools/demo/src/util/shiki.ts`.
- Implementation tests: the pattern-extraction scanner's edge cases (escaped backslashes,
  alternation boundaries) and a representative scope spot-check in
  `compiler/tests/dx_tooling_impl_test.rs`.
- Integration: `make demo-check` renders the extended fixture through Shiki.
