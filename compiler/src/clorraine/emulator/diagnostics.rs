//! User-facing diagnostics and result output for the verified runner.

use std::io::{self, Write};
use std::path::Path;

use super::storage::{self, Storage};
use crate::diag::{format_error, DiagnosticFormat};
use crate::clorraine::run::RunResult;

/// Ends the run as a retained failure and prints the readable guest output.
pub(super) fn fail_retained(
    storage: &Storage,
    format: DiagnosticFormat,
    reason: &str,
    result: Option<&[u8]>,
) -> RunResult {
    if let Some(result) = result {
        print_result(result);
    }
    report(format, &retained_message(&storage.root, reason));
    failed()
}

/// The one-line retained-run diagnostic.
pub(super) fn retained_message(root: &Path, reason: &str) -> String {
    format!(
        "clorraine: {reason}; private run retained at '{}'",
        storage::shown(root)
    )
}

/// Writes guest result bytes to stdout.
pub(super) fn print_result(result: &[u8]) {
    let mut stdout = io::stdout();
    let _ = stdout.write_all(result);
    let _ = stdout.flush();
}

/// Writes one driver diagnostic in the selected format.
pub(super) fn report(format: DiagnosticFormat, message: &str) {
    eprint!("{}", format_error(format, message));
}

/// The failed result shape used by every runtime error.
pub(super) fn failed() -> RunResult {
    RunResult { code: 1, captured: false }
}
