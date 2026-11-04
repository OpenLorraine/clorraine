//! Implementation tests for the developer-experience tooling internals.
//!
//! These cases pin data shapes and helper behaviour that the end-to-end
//! specification suite cannot reach: full registry coverage, the exact render
//! contract, and later the JSON encoder and grammar scanner.

use std::path::Path;

use clc::diag::{registry, Diagnostic, DiagnosticFormat, Diagnostics};
use clc::span::{SourceMap, Span};

mod dx_support;

/// The exact explain text for `E10034`, used as the render oracle.
const EXPLAIN_E10034: &str = "error E10034: cannot release or store borrowed parameter '{name}'\n  trigger: Borrowed parameter released or stored by the callee\n  section: Ownership, moves, initialization, nullability, borrows (E100xx)\n  help: declare the parameter `take` or by value; a `borrow T?` slot may hold its own incoming value, a static view, or a newly owned value\n";

/// Reads the normative registry document.
fn registry_text() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec/14-diagnostics.md");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// Collects the live five-digit codes from the registry tables.
///
/// The scanner mirrors the production parser's shape on purpose: it reads the
/// specification independently, so the coverage test fails when a row is
/// malformed rather than silently shrinking the checked set.
fn live_codes(text: &str) -> Vec<String> {
    let mut codes = Vec::new();
    for line in text.lines() {
        let Some(inner) = line.trim().strip_prefix('|').and_then(|line| line.strip_suffix('|'))
        else {
            continue;
        };
        let Some(cell) = inner.split('|').next().map(str::trim) else {
            continue;
        };
        let bytes = cell.as_bytes();
        let is_code = bytes.len() == 6
            && matches!(bytes[0], b'E' | b'W')
            && bytes[1..].iter().all(u8::is_ascii_digit);
        if is_code {
            codes.push(cell.to_owned());
        }
    }
    codes
}

#[test]
fn every_live_registry_row_is_explainable_and_has_help() {
    let codes = live_codes(&registry_text());
    assert_eq!(codes.len(), 114, "the registry has 114 live rows");

    for code in &codes {
        let text = registry::explain(code)
            .unwrap_or_else(|| panic!("{code} has no explain entry"));
        assert!(
            text.contains("\n  help: "),
            "{code} renders without a help line:\n{text}"
        );
        assert!(
            text.ends_with('\n'),
            "{code} does not end with a newline:\n{text:?}"
        );
    }
}

#[test]
fn the_render_shape_is_pinned() {
    assert_eq!(
        registry::explain("E10034").expect("E10034 is registered"),
        EXPLAIN_E10034
    );
}

#[test]
fn unknown_codes_are_absent() {
    assert_eq!(registry::explain("NOPE"), None);
    assert_eq!(registry::explain("e10034"), None);
    assert_eq!(registry::explain("E10037"), None, "retired codes stay absent");
}

#[test]
fn render_with_dispatches_and_counts_character_columns() {
    let mut sources = SourceMap::new();
    let id = sources.add_file("main.clo".to_owned(), "é x\n".to_owned());
    let mut diagnostics = Diagnostics::new(false, None);
    diagnostics.push(Diagnostic::error("E10000", "m", Span::new(id, 3, 4)));

    let json = diagnostics.render_with(&sources, DiagnosticFormat::Json);
    assert!(
        json.contains("\"line\":1,\"column\":3,\"endLine\":1,\"endColumn\":4"),
        "character columns: {json}"
    );
    assert!(diagnostics
        .render_with(&sources, DiagnosticFormat::Human)
        .starts_with("error E10000: m\n"));
}

#[test]
fn explain_errors_honour_json_when_requested() {
    for binary in [
        env!("CARGO_BIN_EXE_clc"),
        env!("CARGO_BIN_EXE_clorraine"),
    ] {
        let output = std::process::Command::new(binary)
            .args(["--diagnostics=json", "--explain", "NOPE"])
            .output()
            .unwrap_or_else(|error| panic!("failed to spawn {binary}: {error}"));
        assert_eq!(output.status.code(), Some(2), "{binary}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "{\"code\":null,\"severity\":\"error\",\"message\":\"usage error: unknown diagnostic code 'NOPE'\",\"help\":null,\"file\":null,\"line\":null,\"column\":null,\"endLine\":null,\"endColumn\":null}\n",
            "{binary}"
        );
    }
}

#[test]
fn the_pattern_scanner_handles_escaped_backslashes() {
    let json = r#"{"match":"\\b(a|i8)\\b","begin":"\\$","end":"\""}"#;
    assert_eq!(
        dx_support::pattern_strings(json),
        ["\\b(a|i8)\\b", "\\$", "\""]
    );
}

#[test]
fn word_boundaries_do_not_cross_alternation_terms() {
    assert!(dx_support::word_bounded("\\b(i8|i16)\\b", "i8"));
    assert!(dx_support::word_bounded("\\b(i8|i16)\\b", "i16"));
    assert!(!dx_support::word_bounded("\\b(i8|i16)\\b", "i"));
    assert!(!dx_support::word_bounded("case", "as"));
}

#[test]
fn the_canonical_grammar_scopes_the_new_groups() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../editors/clorraine.tmLanguage.json");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let strings = dx_support::json_strings(&json);
    assert!(strings
        .iter()
        .any(|value| value == "support.function.builtin.clorraine"));
    assert!(strings
        .iter()
        .any(|value| value == "keyword.operator.arrow.clorraine"));

    let patterns = dx_support::pattern_strings(&json);
    assert!(patterns
        .iter()
        .any(|pattern| dx_support::word_bounded(pattern, "writeVolatile32")));
    assert!(patterns
        .iter()
        .any(|pattern| dx_support::word_bounded(pattern, "sizeof")));
}

#[test]
fn the_driver_sanitizes_an_explain_code_before_echoing_it() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_clorraine"))
        .args(["--explain", "A\u{1b}[31mB"])
        .output()
        .expect("failed to spawn the clorraine driver");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains('\u{1b}'),
        "the error must not echo a raw escape byte: {stderr:?}"
    );
    assert_eq!(stderr.lines().count(), 1, "the error stays on one line");
}
