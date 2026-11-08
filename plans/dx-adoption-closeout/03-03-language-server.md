# Language Server: DX and Adoption Closeout

> **Document**: 03-03-language-server.md
> **Parent**: [Index](00-index.md)

## Overview

A stdio JSON-RPC server that gives an editor compiler diagnostics and the
minimum navigation surface. It reuses the shared analysis entry point, owns no
third-party code, and never invokes the C toolchain. Requirements: R4-R8,
AR-3 to AR-16.

## Architecture

### Current Architecture

No protocol layer exists. `compiler/src/lib.rs` exposes every stage. The CLI is
the only consumer of the full pipeline.

### Proposed Changes

```
compiler/src/lsp/
├── mod.rs          # run_stdio(), public server entry
├── json.rs         # Json value, parser, writer
├── protocol.rs     # Content-Length frames
├── uri.rs          # file/untitled URI conversion
├── text.rs         # line index and UTF-16 positions
├── workspace.rs    # module index, overlays, import closure
├── symbols.rs      # declaration table and lookup
├── features.rs     # hover, definition, completion, document symbols
├── server.rs       # lifecycle, dispatch, cancellation, publish
└── tests.rs        # unit tests for the modules above
compiler/src/lsp_main.rs  # thin binary
```

`compiler/Cargo.toml` gains:

```toml
[[bin]]
name = "clorraine-lsp"
path = "src/lsp_main.rs"
```

No dependency is added.

## Implementation Details

### JSON (`json.rs`)

```rust
pub enum Json {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}
```

- Parse UTF-8 `&str`; reject trailing content.
- Support `\" \\ \/ \b \f \n \r \t` and `\uXXXX`, including surrogate pairs.
- Reject a lone surrogate, an unescaped control byte, a number that is not an
  i64, and nesting deeper than 128.
- `get`, `as_str`, `as_i64`, `as_array`, `as_object` accessors return `Option`.
- The writer escapes `"`, `\`, control bytes below `0x20`, and emits `\u00XX`
  for them. Object keys are written in `BTreeMap` order for deterministic
  tests; protocol fields are written in the order the spec requires.

### Framing (`protocol.rs`)

```rust
pub struct FrameReader<R: BufRead> { /* ... */ }
pub struct FrameWriter<W: Write> { /* ... */ }

pub enum FrameError {
    Io(std::io::Error),
    Eof,
    MissingLength,
    BadLength,
    TooLarge,
}
```

- Read header lines until an empty line; accept `Content-Length` with any
  case; ignore other headers.
- Reject a body over 8 MiB.
- Read exactly the declared byte count; require valid UTF-8.
- A bad frame is fatal for the connection: the reader sends
  `Incoming::Invalid(reason)` to the handler, which writes one JSON-RPC error
  with `id: null` when possible and exits 1.

### URIs (`uri.rs`)

- `file:///path` -> `PathBuf`; decode `%XX`; reject a non-UTF-8 path.
- `untitled:` is kept as an opaque document key and never mapped to a file.
- Path -> URI percent-encodes everything outside the unreserved set plus `/`.
- A path outside the root is allowed only for an explicitly opened document,
  never for a workspace scan.

### Text (`text.rs`)

```rust
pub struct LineIndex { /* line start byte offsets */ }

impl LineIndex {
    pub fn new(text: &str) -> Self;
    pub fn offset(&self, text: &str, position: Position) -> Option<usize>;
    pub fn position(&self, text: &str, offset: usize) -> Position;
    pub fn range(&self, text: &str, span: Span) -> Range;
}
```

Positions are zero-based LSP positions. Character columns count UTF-16 code
units. A position past the end clamps to the end of its line; a line past the
end clamps to the last line. A character inside a surrogate pair moves to the
next code point boundary.

### Workspace (`workspace.rs`)

```rust
pub struct DocumentStore {
    overlays: BTreeMap<String, String>, // key: URI
}

pub struct ModuleIndex {
    modules: BTreeMap<String, Vec<IndexEntry>>,
    paths: BTreeMap<String, IndexEntry>,
}

pub struct IndexEntry {
    pub uri: String,
    pub path: Option<PathBuf>,
    pub module: Option<String>,
    pub imports: Vec<String>,
    pub references: Vec<String>,
}
```

- `index_root(root)`: walk `*.clo` lexicographically, skipping `.git`,
  `build`, `target`, `node_modules`, hidden directories, and symlinked
  directories. Stop after 2000 files. Parse each file with
  `analysis::parse`; keep `module`, `imports`, and
  `load::referenced_modules` names. A parse error does not abort the walk.
- `analysis_set(uri)`: start with the requested document; add every indexed
  file whose module is named by the current set's imports or references;
  repeat to a fixpoint. Add every file that declares a module already in the
  set (the loader's merge rule). Cap the set at 256 files.
- Text for a file is the overlay when present, else the on-disk text. A read
  failure drops the file and is reported as a normal missing-module path.

### Server lifecycle (`server.rs`)

- A reader thread owns stdin and parses frames. It recognizes
  `$/cancelRequest` itself, records the id in a shared
  `Mutex<HashSet<i64>>`, and forwards every other message to the handler
  channel.
- The handler loop owns stdout. It dispatches one message at a time.
- A request checks `cancelled(id)` before running and between analysis files.
  A cancelled request answers `-32800`.
- `initialize` stores the root, profile, and client capabilities, builds the
  index, and returns:

```json
{
  "capabilities": {
    "positionEncoding": "utf-16",
    "textDocumentSync": 1,
    "hoverProvider": true,
    "definitionProvider": true,
    "completionProvider": {"triggerCharacters": [".", ":"]},
    "documentSymbolProvider": true
  },
  "serverInfo": {"name": "clorraine-lsp", "version": "1.1.0"}
}
```

The version comes from `env!("CARGO_PKG_VERSION")`.

- `shutdown` answers `null` and sets the flag; `exit` exits 0 when the flag is
  set and 1 otherwise.
- A request before `initialize` other than `initialize` gets `-32002`
  (server not initialized). A request after `shutdown` gets `-32600`.
- Notifications before `initialize` are ignored.

### Diagnostics (`server.rs`)

On `didOpen`, `didChange`, `didSave`, and `didClose`:

1. Update the overlay store (`didClose` removes it).
2. If the document is closed, publish an empty array and stop.
3. Build the analysis set and run `analysis::analyze(profile, ..., Full,
   false)`.
4. Filter diagnostics to the file id whose path equals the document path.
5. Convert each span to a range with `text.rs`; severity 1 for errors, 2 for
   warnings; `source: "clorraine"`; `code` as a string; help appended as
   `\n\n` plus the help line.
6. Publish `textDocument/publishDiagnostics` with `uri`, `version` when known,
   and the array.

A missing file with no overlay is an empty array. A parse error still publishes
and still allows symbols.

### Symbols (`symbols.rs`)

Build a table from the analysis set:

| Symbol | Fields |
| ------ | ------ |
| `DeclSymbol` | name, kind, name_span, full_span, module, file_id, signature |
| `LocalSymbol` | name, kind, name_span, full_span, function span, file_id |
| `ImportSymbol` | local, original, module, name_span, file_id |

- Top-level: functions, comptime functions, structs, enums, handles, ffi
  blocks, constants, globals.
- Locals: parameters and `let`/`const` statements, including nested blocks and
  `for` bindings.
- Imports: one entry per item with its local alias.
- Module segments: `A.B.C` matches the module `A.B.C` when the whole dotted
  name is indexed; a single segment matches a module whose first segment
  equals it.

Lookup at a byte offset prefers a name span that contains the offset, then the
innermost local whose name matches the token and whose function span contains
the offset and whose name span starts before the offset, then the enclosing
module's declaration, then an import, then a module.

### Features (`features.rs`)

- **Hover** returns `{"contents":{"kind":"markdown","value":"```clorraine\n<text>\n```\n\n<kind> in module `<module>`"},"range":...}`.
  `<text>` is the source slice from the declaration start to the body `{` for
  functions, to the first `{` for structs/enums, or the whole span for
  handles/constants/globals/locals. It is trimmed; internal whitespace is
  preserved. Locals show their `let`/`const` statement head.
- **Definition** returns `{"uri":..., "range":...}` for the declaration name
  span. A module maps to the first file that declares it, at the module
  declaration's name span.
- **Completion** returns a sorted unique array of
  `{"label","kind","detail"}`. Sources: a fixed keyword list, a fixed built-in
  type list, the current module's declarations, the current file's imports,
  and indexed module names. `sort_text` is the lowercase label with a numeric
  prefix by category (`0` keyword, `1` type, `2` module, `3` local/decl) so the
  client groups predictably.
- **Document symbols** returns `{"name","kind","range","selectionRange"}`
  entries in source order for the requested file. Nested struct fields are not
  symbols.

If the file has syntax errors, completion and document symbols still work on
the partial parse when the parser produced items; hover and definition return
`null` for a position that has no parsed symbol. A missing module declaration
does not disable completion.

### Error handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Malformed JSON | `-32700`, id null, continue | AR-5 |
| Invalid request shape | `-32600`, id when recoverable | AR-5 |
| Missing params | `-32602`, id | AR-5 |
| Unknown method | `-32601` for request; ignore notification | AR-5 |
| Before initialize | `-32002` | AR-5 |
| After shutdown | `-32600` | AR-5 |
| Missing document | Empty diagnostics; `null` features | AR-11 |
| Non-file URI request | `-32602` unless an `untitled:` overlay exists | AR-12 |
| Cancelled request | `-32800` | AR-10 |
| Frame too large | Write one error when possible, then exit 1 | AR-5 |
| I/O failure | Exit 1 with one stderr line | AR-9 |

## Testing Requirements

- Unit: JSON escapes/surrogates/depth/truncation; frame split across reads;
  UTF-16 positions with a non-BMP character; URI round-trip with spaces;
  workspace closure with an unrelated program; cancellation set.
- Integration (`lsp_spec_test.rs`): spawn the binary, initialize a fixture
  workspace, assert diagnostics, hover, definition, completion, and document
  symbols; assert malformed input, missing file, unsaved overlay, cancellation,
  and clean shutdown.
- No test writes outside its temporary root.
