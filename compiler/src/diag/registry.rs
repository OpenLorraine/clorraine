//! Explain mode: the diagnostic registry embedded from the specification.
//!
//! The diagnostics chapter is the single source of truth for codes, triggers,
//! message templates, and help text. The chapter is compiled into the binary,
//! parsed on demand, and rendered for `--explain`, so an explanation can never
//! drift from the registry.
//!
//! The parser deliberately ignores every line that is not a registry table
//! row. A malformed edit therefore cannot crash the tool; the coverage tests in
//! the integration suite fail instead, which keeps the registry honest.

use super::Severity;

/// The normative registry document compiled into both binaries.
const REGISTRY: &str = include_str!("../../../spec/14-diagnostics.md");

/// One parsed registry row.
///
/// String fields borrow from the embedded document; `section` is owned because
/// parsing removes the heading number and the code-span backticks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryEntry<'a> {
    /// The five-digit code, for example `E10034`.
    pub code: &'a str,
    /// Derived from the `E`/`W` code prefix.
    pub severity: Severity,
    /// The enclosing section heading without its number or backticks.
    pub section: String,
    /// The Trigger cell, verbatim.
    pub trigger: &'a str,
    /// The First-line cell without its wrapping code-span backticks.
    pub template: &'a str,
    /// The Help cell, when the row carries one.
    pub help: Option<&'a str>,
}

/// Returns the explain text for `code`, or `None` when no live row matches.
///
/// Both command-line tools call this one function, so `clc --explain` and
/// `clorraine --explain` cannot disagree. The registry is parsed once per
/// process invocation; the document is far smaller than any meaningful
/// startup cost.
pub fn explain(code: &str) -> Option<String> {
    parse_registry(REGISTRY)
        .into_iter()
        .find(|entry| entry.code == code)
        .map(|entry| render_entry(&entry))
}

/// Parses the registry tables, tracking the latest section heading.
///
/// A row is a table line whose first cell is a live five-digit `E`/`W` code.
/// Retired rows carry a suffix after the code and are skipped, as are the
/// range rows in the code-block table.
fn parse_registry(text: &str) -> Vec<RegistryEntry<'_>> {
    let mut entries = Vec::new();
    let mut section = String::new();
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            section = section_title(heading).replace('`', "");
            continue;
        }
        let Some(cells) = table_cells(line) else {
            continue;
        };
        let Some(code) = cells.first().and_then(|cell| live_code(cell)) else {
            continue;
        };
        if cells.len() < 3 {
            continue;
        }
        let help = cells
            .get(3)
            .map(|cell| cell.trim())
            .filter(|cell| !cell.is_empty());
        entries.push(RegistryEntry {
            code,
            severity: if code.starts_with('W') {
                Severity::Warning
            } else {
                Severity::Error
            },
            section: section.clone(),
            trigger: cells[1],
            template: cells[2].trim_matches('`'),
            help,
        });
    }
    entries
}

/// Splits a markdown table row into trimmed cells.
fn table_cells(line: &str) -> Option<Vec<&str>> {
    let inner = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(str::trim).collect())
}

/// Returns the cell when it is exactly a live five-digit code.
///
/// A retired row spells its first cell as `` `E10037` retired ``; the extra
/// text makes the cell fail this check, so retired codes stay reserved without
/// appearing in explain mode.
fn live_code(cell: &str) -> Option<&str> {
    let bytes = cell.as_bytes();
    let is_code = bytes.len() == 6
        && matches!(bytes[0], b'E' | b'W')
        && bytes[1..].iter().all(u8::is_ascii_digit);
    is_code.then_some(cell)
}

/// Removes the leading `N. ` numbering from a section heading.
fn section_title(heading: &str) -> &str {
    match heading.split_once(". ") {
        Some((number, title))
            if !number.is_empty() && number.chars().all(|digit| digit.is_ascii_digit()) =>
        {
            title
        }
        _ => heading,
    }
}

/// Renders one entry in the fixed explain shape.
fn render_entry(entry: &RegistryEntry<'_>) -> String {
    let severity = match entry.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    let mut text = format!(
        "{severity} {}: {}\n  trigger: {}\n  section: {}\n",
        entry.code, entry.template, entry.trigger, entry.section
    );
    if let Some(help) = entry.help {
        text.push_str("  help: ");
        text.push_str(help);
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small registry document with an error, a warning, and a retired row.
    const SYNTHETIC: &str = "\
# Registry

## 3. Ownership (`E100xx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| E10010 | Null use | `value '{name}' may be null here` | check `!= null` |
| E10037 retired | Old code | `gone` | — |

## 14. Warnings (`W100xx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| W10000 | Unused local | `unused local '{name}'` | remove it |
";

    #[test]
    fn embedded_registry_parses_every_live_row() {
        let entries = parse_registry(REGISTRY);
        assert_eq!(
            entries.len(),
            114,
            "parsed codes: {:?}",
            entries.iter().map(|entry| entry.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn parses_sections_codes_and_severities() {
        let entries = parse_registry(SYNTHETIC);
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].code, "E10010");
        assert_eq!(entries[0].severity, Severity::Error);
        assert_eq!(entries[0].section, "Ownership (E100xx)");
        assert_eq!(entries[0].trigger, "Null use");
        assert_eq!(entries[0].template, "value '{name}' may be null here");
        assert_eq!(entries[0].help, Some("check `!= null`"));

        assert_eq!(entries[1].code, "W10000");
        assert_eq!(entries[1].severity, Severity::Warning);
        assert_eq!(entries[1].section, "Warnings (W100xx)");
    }

    #[test]
    fn skips_retired_rows_and_non_code_tables() {
        let entries = parse_registry(SYNTHETIC);
        assert!(entries.iter().all(|entry| entry.code != "E10037"));
    }

    #[test]
    fn a_row_without_a_help_cell_renders_without_a_help_line() {
        let text = "\
## 1. Section (`E100xx`)

| Code | Trigger | First line (after the code) |
|---|---|---|
| E10000 | A reason | `a message` |
";
        let entries = parse_registry(text);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].help, None);
        assert_eq!(
            render_entry(&entries[0]),
            "error E10000: a message\n  trigger: A reason\n  section: Section (E100xx)\n"
        );
    }

    #[test]
    fn a_warning_renders_its_severity_word() {
        let entries = parse_registry(SYNTHETIC);
        assert_eq!(
            render_entry(&entries[1]),
            "warning W10000: unused local '{name}'\n  trigger: Unused local\n  section: Warnings (W100xx)\n  help: remove it\n"
        );
    }
}
