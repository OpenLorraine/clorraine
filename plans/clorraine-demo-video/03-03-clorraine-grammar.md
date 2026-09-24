# Clorraine Grammar and Theme: Clorraine Demo Video Toolkit

> **Document**: 03-03-clorraine-grammar.md
> **Parent**: [Index](00-index.md)

## Overview

Shiki highlights code by applying a TextMate grammar and a theme. Clorraine has no such
grammar today; the only highlighter is `demo/clorraine.vim`, which reuses Vim's C rules
and adds Clorraine words. This component defines a TextMate grammar and a Shiki theme
that reproduce and improve on the Vim token set.

## Grammar file

`tools/demo/grammars/clorraine.tmLanguage.json` (AR-26), scope name
`source.clorraine`, file type `clo`. The token set is derived from `demo/clorraine.vim`
(AR-9) and checked against `spec/grammar.ebnf.md`.

### Token tables

Keywords (`demo/clorraine.vim:21-26`):

```
module import export func let take borrow handle deleter
ffi intrinsic unsafe raw volatile enum struct const static
comptime defer
if else while for switch case default break continue return
true false null
```

Scalar and view types (`demo/clorraine.vim:29-32`):

```
i8 i16 i32 i64 u8 u16 u32 u64 usize isize f32 f64 bool void str cstring
```

Owned and stdlib handle types — pinned literal list (`demo/clorraine.vim:33-34` plus
`Screen` from `spec/appendix-amiga.md:29`):

```
String File Buffer HandlerToken Screen
```

The grammar reads this as a fixed list; it is extended only when `spec/appendix-stdlib.md`
or `spec/appendix-amiga.md` adds a handle type (PF-007).

### Scope assignments

| Token | TextMate scope |
| ----- | -------------- |
| Line/block comments | `comment.line.double-slash.clorraine`, `comment.block.clorraine` |
| Strings, char literals, `$` template marker (`demo/clorraine.vim:37`) | `string.quoted.double.clorraine`, `constant.character.clorraine`, `punctuation.definition.template.clorraine` |
| Numbers (decimal, `0x`, `0b`) | `constant.numeric.clorraine` |
| Keywords | `keyword.control.clorraine` / `storage.modifier.clorraine` |
| Types | `storage.type.clorraine` |
| Handle types | `support.type.clorraine` |
| A call name (`Ident` before `(`), per `demo/clorraine.vim:40` | `entity.name.function.clorraine` |
| `module demo.Hello;` name | `entity.name.type.module.clorraine` |
| Operators / punctuation | `keyword.operator.clorraine`, `punctuation.*` |

Clorraine strings use `$"…"` templates with `{expr}` interpolation
(`examples/hello/hello.clo:8`); the grammar highlights the interpolation braces and
their contents as `punctuation.section.interpolation` plus the normal expression rules.

## Shiki theme

`assets/theme.json`: a small Shiki theme reusing the brand values (AR-10) — background
`0x1e1e2e`, accent `0x88ccff`, body text light grey, and mappings from the scopes above
to accessible foreground colors. It is applied only to `code` scenes; cards use
`assets/demo.css`.

## Implementation Details

### `util/shiki.ts`

```ts
/** A Shiki highlighter configured with the Clorraine grammar and brand theme. */
export interface Highlighter { highlight(file: string, code: string): string; }

/** Loads shiki, registers grammars/clorraine.tmLanguage.json, returns a highlighter. */
export async function createHighlighter(): Promise<Highlighter>;
```

`highlight` returns the HTML for the code block; the caller injects it into
`assets/code.html`.

### Verification of the token set

A unit test tokenizes a synthetic fixture that contains one token from every row of the
tables above (plus a comment and a `$"…"` template) and asserts each is scoped with the
expected scope (07 ST-3). A second assertion runs the grammar over
`examples/hello/hello.clo` to catch regressions on real code. This is the guard that the
grammar stays aligned with `demo/clorraine.vim` (PF-007).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Grammar file missing/invalid | Fail at startup with the file path; exit 1 | AR-26 |
| Shiki cannot tokenize a file | Fall back to plain `<pre>` text and log a warning | AR-2 |

## Testing Requirements

- Grammar token tests for keywords, types, handles, comments, and templates, driven by a
  synthetic all-token fixture (ST-3).
