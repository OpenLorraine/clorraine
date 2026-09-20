//! Shared pipeline helper for ownership-layer implementation tests.
//!
//! The helper drives one source string through parse, load, resolve, type, and
//! ownership so a test can assert on the diagnostics and on the lowered IR
//! without repeating the front-end wiring.

use crate::diag::{Diagnostics, Severity};
use crate::parse::Parser;
use crate::span::SourceMap;

use super::OwnershipOutput;

/// Runs the front end and ownership stage on one source string.
///
/// Returns `None` for the IR when an earlier stage already reported an error,
/// because the ownership stage is skipped on a program that did not type.
pub(crate) fn compile(source: &str) -> (Diagnostics, Option<OwnershipOutput>) {
    let mut sources = SourceMap::new();
    let mut diagnostics = Diagnostics::new(false, None);
    let id = sources.add_file("test.clo".to_owned(), source.to_owned());
    let program = Parser::new(&sources, id, &mut diagnostics).parse_program();
    let loaded = crate::load::load(vec![(id, program)], &mut diagnostics);
    crate::resolve::resolve(&loaded, &mut diagnostics);
    if diagnostics.error_count() > 0 {
        return (diagnostics, None);
    }
    let sema = crate::sema::check(&loaded, &mut diagnostics);
    if diagnostics.error_count() > 0 {
        return (diagnostics, None);
    }
    let output = super::check(&loaded, &sema, &mut diagnostics);
    (diagnostics, Some(output))
}

/// The error codes a source string reports, in report order.
pub(crate) fn error_codes(source: &str) -> Vec<String> {
    let (diagnostics, _) = compile(source);
    diagnostics
        .items()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.code.to_owned())
        .collect()
}

/// True when the source reports the given error code at least once.
pub(crate) fn reports(source: &str, code: &str) -> bool {
    error_codes(source).iter().any(|reported| reported == code)
}

/// Wraps a body in a module with the view and consumer helpers the tests use.
///
/// `viewOf` returns a view of its parameter, `consume` takes ownership of a
/// `String`, and `observe` borrows a `str`. The wrapper is shared by the tests
/// that need a helper function to move, borrow, or view a value.
pub(crate) fn with_view_helper(body: &str) -> String {
    format!(
        "module t;\n\
         func viewOf(s: str): str {{ return s; }}\n\
         func consume(take s: String): void {{}}\n\
         func observe(s: str): void {{}}\n\
         {body}\n\
         func main(): i32 {{ return 0; }}\n"
    )
}
