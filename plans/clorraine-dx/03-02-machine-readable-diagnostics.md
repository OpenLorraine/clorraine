# Machine-Readable Diagnostics: Clorraine DX Tooling

> **Document**: 03-02-machine-readable-diagnostics.md
> **Parent**: [Index](00-index.md)

## Overview

`--diagnostics=json` renders every diagnostic — and every driver-level error — as one JSON object
per line on stderr, so editors and CI can consume the toolchain without parsing the human format.
The default output, stream choice, and exit codes stay byte-identical when the flag is absent
(AR-5). The schema is fixed and additive-only (AR-6, R11).

## Architecture

### Current Architecture

- `Diagnostics::render` (`compiler/src/diag.rs:188`) returns the human text as a `String`; the
  drivers `eprint!` it.
- `clc` renders diagnostics at four sites (`driver.rs:254`, `:283`, `:307`, `:321`) and prints
  plain errors at `:240`, `:250`, `:263`, `:327`.
- `clorraine` prints its own errors at `clorraine/mod.rs:45`, `:69`, `:127`, `:141` and the ICE
  text from `build.rs`; it calls the compiler in process (`build.rs:72`).

### Proposed Changes

- `diag.rs` gains a `DiagnosticFormat` enum, a `render_json` method, a JSON string escaper, a
  null-code error line, and an argument pre-scan helper.
- Both CLIs parse the flag, route every error print through the format, and `clorraine` forwards
  the flag to the in-process compiler.

## Implementation Details

### New Types/Interfaces

```rust
// compiler/src/diag.rs

/// How diagnostics render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticFormat {
    /// The fixed human format (`error CODE: message` plus location and help).
    Human,
    /// One JSON object per diagnostic, newline-delimited on stderr.
    Json,
}

/// The format an argument vector asks for.
///
/// Scans for `--diagnostics=json` (and the separate `--diagnostics json`
/// spelling) so a usage error can still honour the requested stream; the
/// authoritative value after a successful parse is the parsed option.
pub fn format_from_args(args: &[String]) -> DiagnosticFormat;

/// One non-registry error as a JSON object line (see the schema below).
pub fn json_error_line(message: &str) -> String;

impl Diagnostics {
    /// Renders every kept diagnostic as one JSON object per line.
    pub fn render_json(&self, sources: &SourceMap) -> String;
}

/// Appends `text` as a JSON string literal, escaping `"`, `\`, and control
/// characters so one diagnostic can never span two lines (AR-15).
fn push_json_string(out: &mut String, text: &str);
```

### JSON schema (pinned by tests)

One object per diagnostic, keys in this exact order, one trailing `\n`:

```json
{"code":"E10034","severity":"error","message":"cannot release or store borrowed parameter 'x'","help":"declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt","file":"main.clo","line":10,"column":25,"endLine":10,"endColumn":26}
```

| Key | Type | Value |
| --- | ---- | ----- |
| `code` | string \| null | The registry code; `null` in the driver-error line |
| `severity` | string | `"error"` or `"warning"` (unchanged by `--deny-warnings`) |
| `message` | string | The rendered message |
| `help` | string \| null | The help line, when present |
| `file` | string \| null | `SourceMap::path`; `null` without a span |
| `line`, `column` | number \| null | 1-based start from `SourceMap::location` |
| `endLine`, `endColumn` | number \| null | 1-based position of `span.end` (exclusive) |

Driver errors render as `{"code":null,"severity":"error","message":<full text>,"help":null,
"file":null,"line":null,"column":null,"endLine":null,"endColumn":null}` — the message keeps its
existing prefix (`usage error: …`, `clc: …`, `clorraine: …`, or the ICE text with its `note:` and
`first error:` lines) and escaping keeps it on one line (AR-7).

### Integration Points

**`clc` (`compiler/src/driver.rs`)**

- `Options` gains `diagnostics: DiagnosticFormat`; `--diagnostics` accepts the separate and
  joined spellings; any value other than `json` is a usage error
  (`unknown diagnostics format '<value>' (expected json)`).
- `run` pre-scans the raw arguments with `format_from_args` before parsing, so a usage error from
  the parser still prints in the requested stream.
- The four diagnostic renders choose `render` or `render_json`; the four plain error prints route
  to `eprintln!("{message}")` or `eprint!("{}", json_error_line(&message))`.

**`clorraine` (`compiler/src/clorraine/`)**

- `cli.rs`: `Options` gains `diagnostics`; the option is parsed after the command like every other
  option; the same value validation. `HELP` gains one line:

```text
      --diagnostics=json              Print diagnostics as JSON objects on stderr
```

- `mod.rs`: `run_with_marker` pre-scans for the format so parse errors honour it; `invoke` and the
  `emit` / `build` / `run` error prints route through one small helper
  (`report_error(format, message)` → human `eprintln!` or `json_error_line`). The SDK/missing-tool
  messages and the ICE text use the same helper.
- `build.rs`: the compiler argument vector gains `--diagnostics=json` when the format is JSON, so
  the in-process compiler renders NDJSON directly to stderr (the driver prints nothing extra).

### Format evolution

The schema is additive-only (R11): consumers ignore unknown keys; a future incompatible change
introduces a new `--diagnostics` value rather than reshaping an object. No version field is added
— the format name is the version.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Unknown `--diagnostics` value | Usage error verbatim (human — no format was selected) | AR-5 |
| Parser usage error with JSON requested anywhere | Pre-scan selects JSON; one object line | AR-7 |
| Driver/environment/ICE error in JSON mode | `json_error_line` with the full message | AR-7 |
| Message with `"`, `\`, newline, or control chars | Escaped; one object stays one line | AR-15 |
| Diagnostic without a span | Position fields `null` | AR-6 |
| `--deny-warnings` | Severity stays `"warning"`; exit code already nonzero | AR-6 |

## Testing Requirements

- Specification tests: ST-8 – ST-15 (`07-testing-strategy.md`) in
  `compiler/tests/dx_tooling_spec_test.rs`.
- Implementation tests: exact line shapes (including key order), the end-position computation on
  a multi-character token, escaping of quotes/backslashes/newlines/control characters, the empty
  diagnostic set (no stderr), and the pre-scan helper — in
  `compiler/tests/dx_tooling_impl_test.rs`; unit tests inside `diag.rs`.
