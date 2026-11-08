# Documentation Generator: DX and Adoption Closeout

> **Document**: 03-04-doc-generator.md
> **Parent**: [Index](00-index.md)

## Overview

`clorraine --doc` turns `///` comments and parsed declarations into a linked
Markdown tree. It is a parse-only tool: no type checking, no C compiler, no
SDK. Requirements: R11-R12, AR-17 to AR-22.

## Architecture

### Current Architecture

`///` is lexer trivia (`compiler/src/lex/mod.rs:140`); the parser discards it.
The driver CLI has early-exit modes for `--explain`, `--help`, and `--version`
(`compiler/src/clorraine/cli.rs:112-150`).

### Proposed Changes

- `compiler/src/clorraine/cli.rs` gains `Action::Doc(DocOptions)` and a
  dedicated scan for `--doc`, `-o`, and files.
- `compiler/src/clorraine/docgen.rs` owns extraction and rendering.
- `compiler/src/analysis.rs::parse` supplies parsed programs.

`DocOptions`:

```rust
pub struct DocOptions {
    pub output: Option<String>,
    pub files: Vec<String>,
}
```

No profile is stored or required.

## Implementation Details

### CLI

- `--doc` may appear anywhere before the files. It selects the mode.
- The only option accepted in this mode is `-o DIR` or `--output DIR`;
  `-o=DIR` is accepted too. Any other option is a usage error.
- At least one file is required.
- `--help` and `--version` keep their precedence over `--doc`.
- The help text lists `--doc` under "Documentation".
- Default output directory: `build/doc` relative to the current working
  directory. The absolute path is printed once.
- Exit codes: `0` success, `1` source read or parse error, `2` usage or output
  path rejection.

### Doc comment extraction

Given a declaration start byte and the source text:

1. Take the text from the start of the line containing the declaration, back
   to the previous newline.
2. Walk upward line by line. A line whose trimmed text starts with `///` is
   part of the run; strip the three slashes and at most one following space.
3. Stop at the first line that is not a doc line, including a blank line.
4. Reverse the collected lines and join with `\n`.

A module documentation run is extracted from the `module` declaration's span
start with the same function. An orphan run with no declaration below it is
never collected because extraction is driven by declarations.

### Signature extraction

`signature(sources, item)`:

| Item | Slice |
| ---- | ----- |
| Function, comptime | declaration start to body `{` (`body.span.start`) |
| Struct, enum | declaration start to first `{` in the span |
| Handle | whole declaration span, trailing `;` trimmed |
| `ffi` block | declaration start to first `{` |
| Constant, global | whole declaration span, trailing `;` trimmed |

The slice is trimmed of leading/trailing whitespace and of trailing `{`.
Internal line breaks and indentation are preserved exactly. A signature is
rendered in a fenced `clorraine` block.

### Rendering

`index.md`:

```markdown
# Clorraine API

- [app.core](app.core.md) — Core application types.
- [app.io](app.io.md) — File helpers.
```

Modules are sorted by dotted name. The summary is the first non-empty
documentation line, or `No module documentation.`.

A module page:

````markdown
# app.core

Core application types.

## Buffer

```clorraine
export struct Buffer {
```

Allocates an owned byte buffer.

**Related:** [Error](app.core.md#error), [Buffer](app.core.md#buffer)
````

- The H1 names the module. The module documentation follows.
- Each declaration is an H2 whose text is the exact identifier, in source
  order. Anchor: the lowercased identifier with non-alphanumeric characters
  removed; collisions get `-2`, `-3`.
- The signature block follows the heading, then the documentation paragraphs.
- `**Related:**` lists unique named types from the declaration's parameter,
  field, return, backing, or freedBy positions that are declared in an input
  module. Links are `[Name](<module>.md#<anchor>)` with the module page's
  relative file name. A type not declared in an input module is omitted.
- A declaration with no documentation still gets a heading and signature;
  no placeholder prose is emitted.

### Determinism

- Module order: `BTreeMap` order.
- Declaration order: source order within a module, files in command-line
  order.
- LF line endings; one trailing newline per file.
- No timestamps, absolute paths, host names, or environment values.
- Rendering is pure; all files are rendered to strings before the output
  directory is created.

### Output safety

- A module name must match `^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)*$`.
  A missing or invalid name is an error and writes nothing.
- The output directory is created if absent. An existing output file is
  overwritten only when its name is in the generated set.
- The final path is canonicalized; it must stay under the canonical output
  root. A symlinked output root is resolved once.
- Only the listed files are read. No module imports are followed.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Unreadable source | stderr names the path; exit 1 | AR-17 |
| Parse error | normal diagnostics; write nothing; exit 1 | AR-17 |
| Missing module name | error; write nothing; exit 1 | AR-21 |
| Invalid module name | error; write nothing; exit 2 | AR-21 |
| Output path escapes root | error; write nothing; exit 2 | AR-21 |
| No doc comments | headings and signatures only | AR-18 |
| Two modules with the same name | merge declarations in file order | AR-20 |

## Testing Requirements

- Golden output for a two-module fixture with generics, a struct, an enum, a
  handle, a constant, a global, and a multi-line function signature.
- Attachment rule: blank line separates; orphan ignored; indented `///`
  works.
- Determinism: two runs byte-identical.
- Path safety: a module named `..` or `a/b` writes nothing.
- No SDK: the test runs with an empty `PATH` except the binary itself.
