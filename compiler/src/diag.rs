//! Diagnostic values, collection, and rendering.
//!
//! The first rendered line is exactly `error CODE: message` or
//! `warning CODE: message`, followed by an indented `--> path:line:col` line
//! and an optional `help:` line. Errors reject the program; warnings only fail
//! a build when warnings are denied. `--max-errors` caps how many error
//! diagnostics are kept.

pub mod registry;

use crate::span::{SourceMap, Span};

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// The program is rejected.
    Error,
    /// The program compiles with a noted problem.
    Warning,
}

/// How diagnostics render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticFormat {
    /// The fixed human format (`error CODE: message` plus location and help).
    Human,
    /// One JSON object per diagnostic, newline-delimited on stderr.
    Json,
}

impl DiagnosticFormat {
    /// Parses a `--diagnostics` option value.
    ///
    /// Only `json` is accepted. The error text is the user-facing usage
    /// message both command-line tools print, so the accepted spelling and
    /// its rejection cannot drift apart.
    pub fn parse(value: &str) -> Result<Self, String> {
        if value == "json" {
            Ok(Self::Json)
        } else {
            Err(format!("unknown diagnostics format '{value}' (expected json)"))
        }
    }
}

/// The format an argument vector asks for.
///
/// The scan runs before option parsing so a usage error can still honour a
/// requested JSON stream. It accepts both the joined and the separate
/// spelling; any other value selects the human stream, because no format was
/// successfully chosen.
pub fn format_from_args(args: &[String]) -> DiagnosticFormat {
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--diagnostics" {
            if args.get(index + 1).map(String::as_str) == Some("json") {
                return DiagnosticFormat::Json;
            }
        } else if argument.strip_prefix("--diagnostics=") == Some("json") {
            return DiagnosticFormat::Json;
        }
        index += 1;
    }
    DiagnosticFormat::Human
}

/// Formats one driver-level error for the selected stream.
///
/// Human mode keeps the existing byte-for-byte shape: a message that already
/// ends with a newline is printed as-is, and any other message gains one. JSON
/// mode renders one null-code object line.
pub fn format_error(format: DiagnosticFormat, message: &str) -> String {
    match format {
        DiagnosticFormat::Human => {
            if message.ends_with('\n') {
                message.to_owned()
            } else {
                format!("{message}\n")
            }
        }
        DiagnosticFormat::Json => json_error_line(message),
    }
}

/// Renders one non-registry error as a JSON object line.
///
/// The text keeps its existing prefix (`usage error:`, `clc:`, `clorraine:`,
/// or the internal-compiler-error note). A trailing newline is a human line
/// terminator rather than message content, so it is trimmed before encoding;
/// any internal newline is escaped and the object stays on one line.
pub fn json_error_line(message: &str) -> String {
    let message = message.trim_end_matches('\n');
    let mut line = String::from("{\"code\":null,\"severity\":\"error\",\"message\":");
    push_json_string(&mut line, message);
    line.push_str(",\"help\":null,\"file\":null,\"line\":null,\"column\":null,\"endLine\":null,\"endColumn\":null}\n");
    line
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

    /// Moves every kept diagnostic of another collector into this one,
    /// honoring this collector's error cap.
    pub fn absorb(&mut self, other: Diagnostics) {
        for diagnostic in other.items {
            self.push(diagnostic);
        }
    }

    /// Replaces one kept diagnostic in place.
    ///
    /// A later, more specific report about the same site supersedes an earlier
    /// one. The severity is unchanged, so the error and warning counts stay as
    /// they are.
    pub fn replace_at(&mut self, index: usize, diagnostic: Diagnostic) {
        if let Some(slot) = self.items.get_mut(index) {
            *slot = diagnostic;
        }
    }

    /// Drops diagnostics added at or after `from` that repeat an earlier one
    /// (same code, message, and span).
    ///
    /// A deferred block is checked on every exit edge and through nested
    /// continuations, so a check can re-report the same source occurrence; the
    /// occurrence is kept once while distinct diagnostics stay.
    pub fn dedupe_from(&mut self, from: usize) {
        let mut unique: Vec<Diagnostic> = Vec::new();
        for (index, diagnostic) in std::mem::take(&mut self.items).into_iter().enumerate() {
            let repeated = index >= from
                && unique.iter().any(|known| {
                    known.code == diagnostic.code
                        && known.message == diagnostic.message
                        && known.span == diagnostic.span
                });
            if repeated {
                match diagnostic.severity {
                    Severity::Error => self.error_count -= 1,
                    Severity::Warning => self.warning_count -= 1,
                }
            } else {
                unique.push(diagnostic);
            }
        }
        self.items = unique;
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
            rendered.push_str(&format!(
                "{} {}: {}\n",
                severity_word(diagnostic.severity),
                diagnostic.code,
                diagnostic.message
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

    /// Renders every kept diagnostic as one JSON object per line.
    ///
    /// Keys stay in the frozen order; a field with no value is `null`. The
    /// end position is the exclusive end of the diagnostic's span, so an
    /// editor can underline the same range the compiler reported.
    pub fn render_json(&self, sources: &SourceMap) -> String {
        let mut rendered = String::new();
        for diagnostic in &self.items {
            rendered.push_str("{\"code\":");
            push_json_string(&mut rendered, diagnostic.code);
            rendered.push_str(",\"severity\":");
            push_json_string(&mut rendered, severity_word(diagnostic.severity));
            rendered.push_str(",\"message\":");
            push_json_string(&mut rendered, &diagnostic.message);
            rendered.push_str(",\"help\":");
            match &diagnostic.help {
                Some(help) => push_json_string(&mut rendered, help),
                None => rendered.push_str("null"),
            }
            rendered.push_str(",\"file\":");
            match diagnostic.span {
                Some(span) => {
                    let start = sources.location(span.file, span.start as usize);
                    let end = sources.location(span.file, span.end as usize);
                    push_json_string(&mut rendered, sources.path(span.file));
                    rendered.push_str(&format!(
                        ",\"line\":{},\"column\":{},\"endLine\":{},\"endColumn\":{}",
                        start.line, start.column, end.line, end.column
                    ));
                }
                None => rendered.push_str(
                    "null,\"line\":null,\"column\":null,\"endLine\":null,\"endColumn\":null",
                ),
            }
            rendered.push_str("}\n");
        }
        rendered
    }

    /// Renders every kept diagnostic in the selected format.
    ///
    /// Both drivers go through this one method, so a render site added later
    /// cannot silently keep the human shape in JSON mode.
    pub fn render_with(&self, sources: &SourceMap, format: DiagnosticFormat) -> String {
        match format {
            DiagnosticFormat::Human => self.render(sources),
            DiagnosticFormat::Json => self.render_json(sources),
        }
    }
}

/// The lowercase word used for a severity in both formats.
fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

/// Appends `text` as a JSON string literal.
///
/// Quotes, backslashes, and control characters are escaped, so one diagnostic
/// can never span two lines and the output cannot carry terminal control
/// bytes. Other characters pass through unchanged; the writer never emits
/// invalid UTF-8 because it iterates `char`s.
fn push_json_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control.is_control() => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
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

    #[test]
    fn diagnostics_format_parse_accepts_only_json() {
        assert_eq!(
            DiagnosticFormat::parse("json"),
            Ok(DiagnosticFormat::Json)
        );
        assert_eq!(
            DiagnosticFormat::parse("xml"),
            Err("unknown diagnostics format 'xml' (expected json)".to_owned())
        );
    }

    #[test]
    fn format_from_args_accepts_both_spellings() {
        assert_eq!(
            format_from_args(&["--diagnostics=json".to_owned()]),
            DiagnosticFormat::Json
        );
        assert_eq!(
            format_from_args(&["--diagnostics".to_owned(), "json".to_owned()]),
            DiagnosticFormat::Json
        );
        assert_eq!(
            format_from_args(&["--diagnostics=xml".to_owned()]),
            DiagnosticFormat::Human
        );
        assert_eq!(format_from_args(&[]), DiagnosticFormat::Human);
    }

    #[test]
    fn json_error_line_escapes_and_keeps_one_line() {
        let line = json_error_line("usage error: a \"q\"\nand\tmore\\");
        assert!(line.contains("\\\"q\\\""), "quote: {line}");
        assert!(line.contains("\\nand\\tmore\\\\"), "controls: {line}");
        assert_eq!(line.lines().count(), 1);
        assert!(line.ends_with("}\n"));
    }

    #[test]
    fn json_escapes_del_and_c1_controls() {
        let line = json_error_line("a\u{1}b\u{7f}c\u{9b}d");
        assert!(
            line.contains("a\\u0001b\\u007fc\\u009bd"),
            "DEL and C1 controls: {line}"
        );
    }

    #[test]
    fn render_json_uses_the_frozen_key_order_and_positions() {
        let (sources, id) = single_file_sources();
        let mut diagnostics = Diagnostics::new(false, None);
        diagnostics.push(
            Diagnostic::error("E20005", "bad 'x'", Span::new(id, 4, 6)).with_help("rename `x`"),
        );
        assert_eq!(
            diagnostics.render_json(&sources),
            "{\"code\":\"E20005\",\"severity\":\"error\",\"message\":\"bad 'x'\",\"help\":\"rename `x`\",\"file\":\"main.clo\",\"line\":1,\"column\":5,\"endLine\":1,\"endColumn\":7}\n"
        );
    }

    #[test]
    fn render_json_nulls_every_absent_field() {
        let (sources, _) = single_file_sources();
        let mut diagnostics = Diagnostics::new(false, None);
        diagnostics.push(Diagnostic::cli_error("E11201", "unknown profile 'nope'"));
        assert_eq!(
            diagnostics.render_json(&sources),
            "{\"code\":\"E11201\",\"severity\":\"error\",\"message\":\"unknown profile 'nope'\",\"help\":null,\"file\":null,\"line\":null,\"column\":null,\"endLine\":null,\"endColumn\":null}\n"
        );
        assert_eq!(Diagnostics::new(false, None).render_json(&sources), "");
    }

    #[test]
    fn render_json_counts_columns_in_characters() {
        let mut sources = SourceMap::new();
        let id = sources.add_file("main.clo".to_owned(), "é x\n".to_owned());
        let mut diagnostics = Diagnostics::new(false, None);
        diagnostics.push(Diagnostic::error("E10000", "m", Span::new(id, 3, 4)));
        let line = diagnostics.render_json(&sources);
        assert!(
            line.contains("\"line\":1,\"column\":3,\"endLine\":1,\"endColumn\":4"),
            "multi-byte column: {line}"
        );
    }

    #[test]
    fn format_error_preserves_human_bytes_and_wraps_json() {
        assert_eq!(
            format_error(DiagnosticFormat::Human, "usage error: x"),
            "usage error: x\n"
        );
        assert_eq!(
            format_error(DiagnosticFormat::Human, "clorraine: x\n"),
            "clorraine: x\n"
        );
        assert!(format_error(DiagnosticFormat::Json, "clc: x\n")
            .starts_with("{\"code\":null,\"severity\":\"error\",\"message\":\"clc: x\""));
    }
}
