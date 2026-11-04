# Explain Mode: Clorraine DX Tooling

> **Document**: 03-01-explain-mode.md
> **Parent**: [Index](00-index.md)

## Overview

`--explain <CODE>` turns a diagnostic code into its normative registry entry. Both CLIs gain the
mode; the text comes from `spec/14-diagnostics.md` embedded into the binary, so an explanation can
never drift from the registry (AR-4). This document also owns the twelve registry help texts
(AR-10) — the batch's only specification edit.

## Architecture

### Current Architecture

- `compiler/src/diag.rs` owns `Diagnostic` / `Diagnostics` but has no registry awareness.
- `clc` (`compiler/src/driver.rs`) parses `--version`, `--profile`, `--max-errors`,
  `--deny-warnings`, `-o`, and files; `--version` short-circuits after parsing (`:253`).
- `clorraine` (`compiler/src/clorraine/cli.rs`) honours `-h`/`-V` anywhere (`:107-118`) and then
  requires a command word; the help text is the `HELP` const (`:59`).

### Proposed Changes

- New module `compiler/src/diag/registry.rs` (declared from `diag.rs` as `mod registry;`, the
  same file-plus-directory pattern as `codegen.rs` + `codegen/`): embeds the registry with
  `include_str!("../../../spec/14-diagnostics.md")`, parses the tables, and renders an entry.
- `clc` and `clorraine` gain the early-exit flag and call one shared helper.
- The twelve registry rows gain their approved help cells.

## Implementation Details

### New Types/Interfaces

```rust
// compiler/src/diag/registry.rs

use super::Severity;

/// One parsed registry row.
pub struct RegistryEntry {
    /// The five-digit code, e.g. `E10034`.
    pub code: &'static str,
    /// Derived from the `E`/`W` code prefix.
    pub severity: Severity,
    /// The enclosing chapter section, e.g. `Ownership, moves, initialization,
    /// nullability, borrows (E100xx)`.
    pub section: &'static str,
    /// The Trigger cell, verbatim.
    pub trigger: &'static str,
    /// The First-line cell without its code-span backticks.
    pub template: &'static str,
    /// The Help cell, when the row carries one.
    pub help: Option<&'static str>,
}

/// The explain text for `code`, or `None` when the registry has no such row.
///
/// Shared by both CLIs: one lookup and one render, so `clc --explain` and
/// `clorraine --explain` can never disagree.
pub fn explain(code: &str) -> Option<String>;

/// Parses a registry document. Private to the module; tests feed synthetic
/// tables through it.
fn parse_registry(text: &str) -> Vec<RegistryEntry>;
```

### New Functions/Methods

- `parse_registry(text)` walks line by line: a `## ` heading becomes the current section with
  its leading `N. ` numbering and all backtick characters removed; a table row (starts and ends
  with `|`) whose second cell is a five-digit `E`/`W` code becomes an entry.
- `explain(code)` parses the embedded registry (per invocation; the document is smaller than
  100 KB) and returns the rendered text for the first exact-match row, else `None`.
- `render_entry(entry)` produces the text below; the `help:` line is omitted when the row has no
  help cell (defensive — no row lacks one after AR-10).

### Render shape (pinned by tests)

```text
error E10034: cannot release or store borrowed parameter '{name}'
  trigger: Borrowed parameter released or stored by the callee
  section: Ownership, moves, initialization, nullability, borrows (E100xx)
  help: declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt
```

The first line mirrors the compiler's own diagnostic style (`error CODE: template`); a warning row
renders `warning W10000: ...`. Trailing newline after the last line, matching `Diagnostics::render`.

### Integration Points

**`clc` (`compiler/src/driver.rs`)**

- `Options` gains `explain: Option<String>`; parsing accepts `--explain <CODE>` (via the existing
  `argument` helper) and `--explain=<CODE>`.
- In `run`, explain is checked after a successful parse and before the `--version` branch; other
  valid arguments (files, `--profile`) are ignored. A malformed command line still errors from the
  parser, exactly as `--version` does today.

```rust
    if let Some(code) = &options.explain {
        return match diag::registry::explain(code) {
            Some(text) => {
                print!("{text}");
                0
            }
            None => {
                eprint!(
                    "{}",
                    format_error(
                        options.diagnostics,
                        &format!("usage error: unknown diagnostic code '{code}'")
                    )
                );
                EXIT_USAGE
            }
        };
    }
```

The success text is always the human registry entry; only a failure follows the selected
diagnostic stream, so a machine consumer that asked for JSON never receives a human error line.

**`clorraine` (`compiler/src/clorraine/cli.rs`, `mod.rs`)**

- `parse` scans for `--explain` / `--explain=` next to the existing `-h`/`-V` scans, in that
  order (help, version, explain — the existing flags keep their precedence; explain joins the
  early-exit family as AR-3 records). New `Action::Explain(String)`.
- A missing value is a parse error: `--explain needs a value`.
- `run_with_marker` handles the action by calling the same shared helper and mapping `None` to
  `usage error: unknown diagnostic code '<code>'` / exit 2.
- `HELP` gains one option line:

```text
      --explain CODE                  Print the registry entry for a diagnostic code
```

### Registry Help Texts (AR-10)

The exact cells appended to the twelve uncovered rows are in `00-ambiguity-register.md`
§Resolution Notes (AR-10). Each row keeps its existing cells; only the Help cell is added. No
other byte of `spec/14-diagnostics.md` changes.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Unknown code | `usage error: unknown diagnostic code '<code>'` on stderr, or the null-code JSON object when JSON was requested; exit 2, no stdout | AR-3, R5 |
| Missing value after `--explain` | `usage error: --explain needs a value`, exit 2 | AR-3 |
| Code with no help cell (defensive) | Entry renders without the `help:` line | AR-14 |
| Malformed registry table | Parse skips non-matching lines; the count/row guard tests fail `make check` | AR-14 |
| `--explain` plus other arguments | Explain wins for valid arguments; malformed input still errors in the parser | AR-3, AR-13 |

## Testing Requirements

- Specification tests: ST-1 – ST-7 (`07-testing-strategy.md`) in
  `compiler/tests/dx_tooling_spec_test.rs` (spawns both binaries like
  `clorraine_driver_spec_test.rs` does).
- Implementation tests: ST-7's registry coverage (114 codes, every row has help, representative
  fields), synthetic-table parse robustness, and the exact render shape in
  `compiler/tests/dx_tooling_impl_test.rs`; unit tests inside `diag/registry.rs` for
  `parse_registry`.
