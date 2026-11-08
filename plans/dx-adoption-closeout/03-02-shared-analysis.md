# Shared Analysis Entry Point: DX and Adoption Closeout

> **Document**: 03-02-shared-analysis.md
> **Parent**: [Index](00-index.md)

## Overview

The driver, the language server, and the documentation generator need the same
front end. This component extracts the driver's pipeline into a documented
library call that accepts in-memory sources and an entry-point mode. It does
not change any stage. Requirements: R8, AR-8, AR-16.

## Architecture

### Current Architecture

`compiler/src/driver.rs:330-410` performs the sequence inline. The embedded
stdlib injection helpers (`inject_stdlib`, `requested_modules`,
`report_unavailable_profile_imports`, `is_unavailable_stdlib`) are private
functions in `driver.rs`. `load::load` always checks the entry point.

### Proposed Changes

A new `compiler/src/analysis.rs` owns:

```rust
/// One input file: its logical path and full text.
pub struct SourceFile {
    pub path: String,
    pub text: String,
}

/// How much of the front end to run.
pub enum Mode {
    /// Stop after parsing (the existing stage-select behavior).
    ParseOnly,
    /// Run the complete front end.
    Full,
}

/// The result of one analysis.
pub struct Analysis {
    /// All inputs, including injected standard-library modules.
    pub sources: SourceMap,
    /// File identity by logical path.
    pub files: BTreeMap<String, FileId>,
    /// Parsed programs, in input order.
    pub programs: Vec<(FileId, Program)>,
    /// The loaded program, when loading ran.
    pub loaded: Option<Loaded>,
    /// Diagnostics from every stage that ran.
    pub diagnostics: Diagnostics,
    /// Type and ownership output when the program checked cleanly.
    pub typed: Option<(SemaOutput, OwnershipOutput)>,
}

/// The parse-only result.
pub struct Parsed {
    pub sources: SourceMap,
    pub files: BTreeMap<String, FileId>,
    pub programs: Vec<(FileId, Program)>,
    pub diagnostics: Diagnostics,
}

/// Runs the front end over in-memory sources.
pub fn analyze(
    profile: Profile,
    files: Vec<SourceFile>,
    mode: Mode,
    check_entry_point: bool,
) -> Analysis;

/// Parses sources only; the documentation generator uses this.
pub fn parse(files: Vec<SourceFile>) -> Parsed;
```

`Analysis` carries the two stage outputs so `driver::run` can pass them to
`codegen::emit` exactly as today; the server ignores them. The parse-only
entry point takes no profile.

`load::load` keeps its signature and behavior. A new
`load::load_with_entry_check(programs, check_entry_point, diagnostics)`
implements both; `load` delegates with `true`.

`driver.rs` keeps the CLI, file-read error message, profile name validation,
`--parse-only` result, and output writing. It builds `SourceFile`s, calls
`analysis::analyze`, emits `analysis.diagnostics` against `analysis.sources`,
and emits C from `analysis.typed` when it is present and there are no errors.

## Implementation Details

### Exact sequence

`analyze` with `Mode::Full` performs, in order:

1. Register every input in a fresh `SourceMap`; record `files`.
2. Parse each file; keep a program only when that file added no parse error.
3. Report an embedded stdlib module the profile does not provide; stop before
   loading when one was reported.
4. Inject embedded stdlib modules to a fixpoint, exactly as the driver does.
5. `load::load_with_entry_check`.
6. `resolve::resolve`.
7. If no errors: `sema::check`; if still no errors: `own::check`.
8. Return everything.

`Mode::ParseOnly` stops after step 2. The driver still validates the CLI
profile before calling `analyze`, as it does today; the mode simply does not
use the value. The standalone `parse` entry point takes no profile at all.

### Integration points

- `driver::run` becomes the only caller of `Mode::ParseOnly` and `Mode::Full`
  in the CLI.
- `lsp` calls `Mode::Full` with `check_entry_point = false`.
- `docgen` calls `analysis::parse` only.

### Behavior preservation

The refactor changes no stage. The existing `make check` suites, the corpus,
and `make differential` are the oracle. A base/head emitted-C diff over
`spec/tests/corpus` is part of the phase verification even though no emitter
file changes.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Unreadable file | Driver reports its existing message; `analysis` never reads disk | AR-16 |
| Parse error in one file | Drop that program; keep other files | AR-16 |
| Profile module unavailable | Same `E11200` as today | AR-16 |
| Missing `main` | Only when `check_entry_point` is true | AR-8 |
| Stage error cascade | Sema/ownership stay gated on zero errors | AR-16 |

## Testing Requirements

- Driver CLI suites must remain byte-identical.
- A unit test proves `Mode::ParseOnly` never loads or checks.
- A unit test proves `check_entry_point = false` accepts a library module and
  `true` still rejects it.
- A corpus emission diff proves no C changed.
