//! Command-line specification tests for front-end stage behavior.
//!
//! The golden suites state one expected diagnostic per file; these cases cover
//! the lexical and parser forms the specification calls out explicitly: the
//! literal and template spellings, the statement-level disambiguations, and
//! the diagnostic-only declaration starts. Every case drives the built `clc`
//! binary, so the published command-line behavior is the oracle.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Codes only the front end reports. A snippet that must be accepted is not
/// allowed to produce any of them.
const FRONT_END_CODES: &[&str] = &[
    "E10204", "E10208", "E10501", "E20000", "E20001", "E20002", "E20003", "E20004", "E20005",
    "E20006", "E20007", "E20008", "E90005",
];

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Creates a unique directory for one test process and tag.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-stage-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
        Self { path }
    }

    /// Path of the scratch directory.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Compiles one source snippet and returns the captured process output.
fn run_source(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("cannot write snippet: {err}"));
    let output_path = scratch.path().join("out.c");
    run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ])
}

/// True when the line starts a rendered diagnostic.
fn is_diagnostic_head(line: &str) -> bool {
    line.starts_with("error ") || line.starts_with("warning ")
}

/// Extracts the code from a diagnostic head line, if the line has one.
fn diagnostic_code(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("error ")
        .or_else(|| line.strip_prefix("warning "))?;
    rest.get(..6)
}

/// Asserts that the front end accepts a snippet.
///
/// "Accepts" means every stderr line is part of a rendered diagnostic, no
/// front-end code is reported, and the exit code follows the command-line
/// contract (1 when an error was reported, 0 otherwise). Later stages may add
/// semantic diagnostics for the same snippet; those are allowed because the
/// case targets the front-end stage.
fn assert_front_end_accepts(tag: &str, source: &str) {
    let run = run_source(tag, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    let mut errors = Vec::new();
    for line in stderr.lines().filter(|line| !line.is_empty()) {
        if !is_diagnostic_head(line) && !line.starts_with("  ") {
            panic!("{tag}: unexpected compiler output line {line:?}");
        }
        if let Some(code) = diagnostic_code(line) {
            if FRONT_END_CODES.contains(&code) {
                panic!("{tag}: front end reported {code}: {line}");
            }
        }
        if line.starts_with("error ") {
            errors.push(line.to_owned());
        }
    }
    let exit = run.status.code().unwrap_or(-1);
    let expected_exit = if errors.is_empty() { 0 } else { 1 };
    assert_eq!(
        exit, expected_exit,
        "{tag}: unexpected exit code (errors: {errors:?})"
    );
}

/// Asserts that compiling a snippet reports every expected diagnostic line.
fn assert_reports(tag: &str, source: &str, expected_lines: &[&str]) {
    let run = run_source(tag, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    for expected in expected_lines {
        assert!(
            stderr.lines().any(|line| line == *expected),
            "{tag}: missing diagnostic `{expected}`; stderr:\n{stderr}"
        );
    }
    assert_eq!(run.status.code(), Some(1), "{tag}: failing compiles exit 1");
}

/// Literal and template spellings from the lexical chapter are accepted.
#[test]
fn lexical_literals_and_templates_are_accepted() {
    assert_front_end_accepts(
        "lex-literals",
        "module spec.stage.lex_literals;

func main(): i32 {
    let x = 1;
    let text = \"A\";
    let byte = 'x';
    let hex = 0xFF;
    let binary = 0b1010;
    let grouped = 1_000;
    let scientific = 1.5e3;
    let template = $\"a{x}b\";
    return 0;
}
",
    );
}

/// Malformed lexical forms report their registered codes and messages.
#[test]
fn malformed_literals_report_their_codes() {
    assert_reports(
        "lex-malformed",
        "module spec.stage.lex_malformed;

func main(): i32 {
    let a = 1.;
    let b = .5;
    let c = 0x;
    let d = 'ab';
    let e = \"\\q\";
    return 0;
}
",
        &[
            "error E20004: invalid numeric literal '1.'",
            "error E20004: invalid numeric literal '.5'",
            "error E20004: invalid numeric literal '0x'",
            "error E20007: character literal must encode exactly one byte",
            "error E20003: invalid escape sequence '\\q'",
        ],
    );
}

/// Comparison and generic-call forms survive the bounded speculative parse.
#[test]
fn comparison_and_generic_call_forms_parse() {
    assert_front_end_accepts(
        "parse-call-forms",
        "module spec.stage.parse_call_forms;

func shapes(a: i32, b: i32, c: i32, f: i32): i32 {
    let comparison = (a < b) > (c);
    let explicit_args = f<a>(1);
    let call_shaped = a < b > (c);
    return 0;
}
",
    );
}

/// A statement starting with `identifier {` is a struct literal; a bare block
/// stays a block.
#[test]
fn struct_literal_and_block_forms_disambiguate() {
    assert_front_end_accepts(
        "parse-struct-block",
        "module spec.stage.parse_struct_block;

struct Point {
    x: i32;
    y: i32;
}

func main(): i32 {
    Point { x: 1, y: 2 };
    {
        let y = 1;
    }
    return 0;
}
",
    );
}

/// Both `for` header forms parse: the element form and the three-clause form.
#[test]
fn both_for_forms_parse() {
    assert_front_end_accepts(
        "parse-for-forms",
        "module spec.stage.parse_for_forms;

func main(): i32 {
    let values = [1, 2, 3];
    for (value in values) {
        let copied = value;
    }
    for (let i = 0; i < 3; i = i + 1) {
        let current = i;
    }
    return 0;
}
",
    );
}

/// Declaration starts that exist only to produce a targeted diagnostic report
/// that diagnostic with its registered message.
#[test]
fn diagnostic_only_starts_report_their_codes() {
    assert_reports(
        "parse-union",
        "module spec.stage.parse_union;

union Value {
    as_int: i32;
    as_float: f32;
}
",
        &["error E10204: tagged unions are not part of Clorraine"],
    );

    assert_reports(
        "parse-volatile",
        "module spec.stage.parse_volatile;

func demo(): void {
    volatile let x: u32 = 0;
    x = x + 1;
}
",
        &["error E10208: 'volatile' is not allowed here"],
    );

    assert_reports(
        "parse-format",
        "module spec.stage.parse_format;

func demo(): String {
    let x: i32 = 5;
    return $\"{x:04x}\";
}
",
        &["error E10501: format specifiers are not supported in v1"],
    );
}
