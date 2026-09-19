//! Diagnostic values, collection, and rendering.
//!
//! The first rendered line is exactly `error CODE: message` or
//! `warning CODE: message`, followed by an indented `--> path:line:col` line
//! and an optional `help:` line. Errors reject the program; warnings only fail
//! a build when warnings are denied. `--max-errors` caps how many error
//! diagnostics are kept.

use crate::span::{SourceMap, Span};

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// The program is rejected.
    Error,
    /// The program compiles with a noted problem.
    Warning,
}

/// One diagnostic: code, message, optional source location, and optional
/// help.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable registry code, for example `E20000`.
    pub code: &'static str,
    /// Whether the diagnostic rejects the program.
    pub severity: Severity,
    /// Rendered message, the text after `CODE: `.
    pub message: String,
    /// Primary source location; command-line diagnostics have none.
    pub span: Option<Span>,
    /// Optional fix guidance rendered on its own line.
    pub help: Option<String>,
}

impl Diagnostic {
    /// Builds an error diagnostic at a source location.
    pub fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            span: Some(span),
            help: None,
        }
    }

    /// Builds a warning diagnostic at a source location.
    pub fn warning(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            message: message.into(),
            span: Some(span),
            help: None,
        }
    }

    /// Builds an error diagnostic with no source location, used for problems
    /// the command line itself carries.
    pub fn cli_error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            span: None,
            help: None,
        }
    }

    /// Adds fix guidance to the diagnostic.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Collects diagnostics and applies the warning policy and error cap.
pub struct Diagnostics {
    items: Vec<Diagnostic>,
    deny_warnings: bool,
    max_errors: Option<usize>,
    error_count: usize,
    warning_count: usize,
}

impl Diagnostics {
    /// Creates a collector.
    ///
    /// `max_errors` caps how many error diagnostics are kept; warnings are
    /// always kept. When `deny_warnings` is set, any warning makes the run
    /// fail, but the warning still renders as a warning.
    pub fn new(deny_warnings: bool, max_errors: Option<usize>) -> Self {
        Self {
            items: Vec::new(),
            deny_warnings,
            max_errors,
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Records a diagnostic unless the error cap is already reached.
    pub fn push(&mut self, diagnostic: Diagnostic) {
        match diagnostic.severity {
            Severity::Error => {
                if self.max_errors.is_some_and(|max| self.error_count >= max) {
                    return;
                }
                self.error_count += 1;
            }
            Severity::Warning => self.warning_count += 1,
        }
        self.items.push(diagnostic);
    }

    /// Kept diagnostics in report order.
    pub fn items(&self) -> &[Diagnostic] {
        &self.items
    }

    /// Number of kept error diagnostics.
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    /// Number of kept warning diagnostics.
    pub fn warning_count(&self) -> usize {
        self.warning_count
    }

    /// True when nothing was reported.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// True when the run must fail: any error, or a warning while warnings are
    /// denied.
    pub fn failed(&self) -> bool {
        self.error_count > 0 || (self.deny_warnings && self.warning_count > 0)
    }

    /// Process exit code for the collected diagnostics.
    pub fn exit_code(&self) -> i32 {
        if self.failed() {
            1
        } else {
            0
        }
    }

    /// Renders every kept diagnostic as a block terminated by a newline.
    pub fn render(&self, sources: &SourceMap) -> String {
        let mut rendered = String::new();
        for diagnostic in &self.items {
            let kind = match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            rendered.push_str(&format!(
                "{kind} {}: {}\n",
                diagnostic.code, diagnostic.message
            ));
            if let Some(span) = diagnostic.span {
                let location = sources.location(span.file, span.start as usize);
                rendered.push_str(&format!(
                    "  --> {}:{}:{}\n",
                    sources.path(span.file),
                    location.line,
                    location.column
                ));
            }
            if let Some(help) = &diagnostic.help {
                rendered.push_str(&format!("  help: {help}\n"));
            }
        }
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    /// Builds a map with one one-line file for rendering tests.
    fn single_file_sources() -> (SourceMap, crate::span::FileId) {
        let mut sources = SourceMap::new();
        let id = sources.add_file("main.clo".to_owned(), "let x = 1;\n".to_owned());
        (sources, id)
    }

    #[test]
    fn renders_the_fixed_first_line_and_location() {
        let (sources, id) = single_file_sources();
        let mut diagnostics = Diagnostics::new(false, None);
        diagnostics.push(Diagnostic::error(
            "E20005",
            "'if' is a keyword and cannot be used as a name",
            Span::new(id, 4, 6),
        ));
        let rendered = diagnostics.render(&sources);
        assert_eq!(
            rendered,
            "error E20005: 'if' is a keyword and cannot be used as a name\n  --> main.clo:1:5\n"
        );
    }

    #[test]
    fn renders_help_lines_when_present() {
        let (sources, id) = single_file_sources();
        let mut diagnostics = Diagnostics::new(false, None);
        diagnostics.push(
            Diagnostic::error("E10204", "tagged unions are not part of Clorraine", Span::new(id, 0, 5))
                .with_help("model the data as a C-style tagged struct and use `switch`"),
        );
        let rendered = diagnostics.render(&sources);
        assert!(rendered.ends_with("  help: model the data as a C-style tagged struct and use `switch`\n"));
    }

    #[test]
    fn max_errors_caps_kept_errors_but_not_warnings() {
        let (_, id) = single_file_sources();
        let mut diagnostics = Diagnostics::new(false, Some(1));
        diagnostics.push(Diagnostic::error("E20000", "first", Span::new(id, 0, 1)));
        diagnostics.push(Diagnostic::error("E20001", "second", Span::new(id, 0, 1)));
        diagnostics.push(Diagnostic::warning("W10000", "warned", Span::new(id, 0, 1)));
        assert_eq!(diagnostics.error_count(), 1);
        assert_eq!(diagnostics.warning_count(), 1);
        assert_eq!(diagnostics.items().len(), 2);
        assert_eq!(diagnostics.items()[0].message, "first");
        assert_eq!(diagnostics.items()[1].code, "W10000");
    }

    #[test]
    fn denied_warnings_fail_the_run_without_changing_the_severity() {
        let (sources, id) = single_file_sources();
        let mut diagnostics = Diagnostics::new(true, None);
        diagnostics.push(Diagnostic::warning("W10000", "unused local 'x'", Span::new(id, 4, 5)));
        assert!(diagnostics.failed());
        assert_eq!(diagnostics.exit_code(), 1);
        assert!(diagnostics.render(&sources).starts_with("warning W10000: "));
    }

    #[test]
    fn cli_diagnostics_render_without_a_location() {
        let (sources, _) = single_file_sources();
        let mut diagnostics = Diagnostics::new(false, None);
        diagnostics.push(Diagnostic::cli_error("E11201", "unknown profile 'nope'"));
        assert_eq!(
            diagnostics.render(&sources),
            "error E11201: unknown profile 'nope'\n"
        );
    }

    #[test]
    fn clean_collection_succeeds() {
        let diagnostics = Diagnostics::new(false, None);
        assert!(diagnostics.is_empty());
        assert!(!diagnostics.failed());
        assert_eq!(diagnostics.exit_code(), 0);
    }
}
