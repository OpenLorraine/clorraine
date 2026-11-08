//! Specification tests for the shared front-end entry point.
//!
//! The analyzer is the one path the compiler driver, the language server, and
//! future tools use. These cases pin the two behaviors those callers depend on:
//! a parse-only mode that never runs a later stage, and an entry-point check
//! that can be switched off for a library module.

use clc::analysis::{analyze, Mode, SourceFile};
use clc::profile::Profile;

/// One module with no `main`, used as the library case.
const LIBRARY: &str = "\
module app.core;

export func add(a: i32, b: i32): i32 {
    return a + b;
}
";

/// Builds one in-memory source file.
fn source(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

/// True when any diagnostic message mentions the executable entry point.
fn mentions_entry_point(analysis: &clc::analysis::Analysis) -> bool {
    analysis
        .diagnostics
        .items()
        .iter()
        .any(|diagnostic| diagnostic.message.contains("func main(): i32"))
}

#[test]
fn should_stop_after_parsing_in_parse_only_mode() {
    let analysis = analyze(
        Profile::Host,
        vec![source("app/core.clo", LIBRARY)],
        Mode::ParseOnly,
        true,
    );
    assert!(analysis.diagnostics.items().is_empty());
    assert!(analysis.loaded.is_none());
    assert!(analysis.typed.is_none());
    assert_eq!(analysis.programs.len(), 1);
}

#[test]
fn should_accept_a_library_module_without_main() {
    let analysis = analyze(
        Profile::Host,
        vec![source("app/core.clo", LIBRARY)],
        Mode::Full,
        false,
    );
    assert!(
        !mentions_entry_point(&analysis),
        "a library module must not report the executable entry point"
    );
    assert!(
        analysis.diagnostics.items().is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics.items()
    );
    assert!(analysis.typed.is_some());
}

#[test]
fn should_reject_a_missing_main_when_checked_as_an_executable() {
    let analysis = analyze(
        Profile::Host,
        vec![source("app/core.clo", LIBRARY)],
        Mode::Full,
        true,
    );
    assert!(mentions_entry_point(&analysis));
    assert!(analysis.typed.is_none());
}
