//! Command-line specification tests for front-end stage behavior.
//!
//! The golden suites state one expected diagnostic per file; these cases cover
//! the lexical and parser forms the specification calls out explicitly (the
//! literal and template spellings, the statement-level disambiguations, and
//! the diagnostic-only declaration starts) plus the name and module rules that
//! no single golden states on its own: shadowing, declaration order, merged
//! modules, and import bindings. The ownership, borrow, and FFI cases cover the
//! same gap for the ownership layer: move state, nullability narrowing, partial
//! moves, lifetime elision, exclusive reborrow, drop planning, and the
//! uncontracted-extern boundary. Every case drives the built `clc` binary, so
//! the published command-line behavior is the oracle.
//!
//! The codegen cases read the emitted translation unit and pin its contract:
//! the cleanup ladder, move lowering, `#line` mapping, the runtime guards, the
//! module initialization order, generic instantiation dedupe, and the
//! float/double surface. The driver cases pin the profile errors, the error
//! cap, and the fixed diagnostic head and location lines.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Codes that a snippet the in-scope stages accept must never produce.
///
/// The lexical and parser diagnoses come first; the name and module diagnoses
/// follow, because an accepted program must also pass resolution. A case that
/// pins acceptance fails when any of these appears, so a stage regression is
/// visible as an unexpected diagnostic instead of a silent pass.
const UNACCEPTABLE_CODES: &[&str] = &[
    // Lexical and parser diagnoses.
    "E10204", "E10208", "E10501", "E20000", "E20001", "E20002", "E20003", "E20004", "E20005",
    "E20006", "E20007", "E20008", "E90005",
    // Name and module diagnoses.
    "E10219", "E11101", "E11102", "E11103", "E11104", "E11105",
    // Type, constant, enum, and template diagnoses. A snippet the in-scope
    // stages accept must not produce any of these either, so a false type,
    // fold, or format error fails an acceptance case instead of passing it.
    "E10200", "E10201", "E10202", "E10203", "E10205", "E10206", "E10207", "E10209", "E10210",
    "E10211", "E10212", "E10215", "E10216", "E10217", "E10218", "E10220", "E10221", "E10222",
    "E10223", "E10224", "E10226", "E10300", "E10301", "E10302", "E10305", "E10500", "E10503",
    // Ownership, borrow, FFI, and unsafe diagnoses. An accepted program must
    // not false-reject through these either, so the positive reborrow and drop
    // cases stay meaningful once the ownership layer lands.
    "E10010", "E10011", "E10012", "E10020", "E10021", "E10022", "E10023", "E10024", "E10025",
    "E10026", "E10030", "E10031", "E10032", "E10033", "E10034", "E10035", "E10036", "E10038",
    "E10039", "E10040", "E10041", "E10100", "E10101", "E10102", "E10103", "E10104", "E10600",
    "E10601", "E10602", "E10603", "E10604",
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

/// Compiles several source files in one invocation and returns its output.
///
/// A merged-module case needs more than one file in one compilation, because
/// the merge happens only when the driver loads the files together. Each entry
/// is a file name and its contents; the file name carries no semantic meaning
/// and only keeps the paths in diagnostics readable.
fn run_sources(tag: &str, files: &[(&str, &str)]) -> Output {
    let scratch = ScratchDir::new(tag);
    let output_path = scratch.path().join("out.c");
    let mut args = vec![
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
    ];
    for (name, source) in files {
        let source_path = scratch.path().join(name);
        std::fs::write(&source_path, source)
            .unwrap_or_else(|err| panic!("cannot write snippet {name}: {err}"));
        args.push(source_path.display().to_string());
    }
    run_clc(&args)
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

/// Asserts that the in-scope stages accept a snippet.
///
/// "Accepts" means every stderr line is part of a rendered diagnostic, no code
/// from `UNACCEPTABLE_CODES` is reported, and the exit code follows the
/// command-line contract (1 when an error was reported, 0 otherwise). Later
/// stages may add their own diagnostics for the same snippet; those are
/// tolerated because the case pins the stage that owns it, not the whole
/// pipeline.
fn assert_front_end_accepts(tag: &str, source: &str) {
    let run = run_source(tag, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    let mut errors = Vec::new();
    for line in stderr.lines().filter(|line| !line.is_empty()) {
        if !is_diagnostic_head(line) && !line.starts_with("  ") {
            panic!("{tag}: unexpected compiler output line {line:?}");
        }
        if let Some(code) = diagnostic_code(line) {
            if UNACCEPTABLE_CODES.contains(&code) {
                panic!("{tag}: the in-scope stages reported {code}: {line}");
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

/// Asserts that a captured run reported every expected diagnostic line.
///
/// Shared by the single-file and multi-file drivers so both exercise the same
/// rendering and exit-code contract.
fn assert_run_reports(tag: &str, run: Output, expected_lines: &[&str]) {
    let stderr = String::from_utf8_lossy(&run.stderr);
    for expected in expected_lines {
        assert!(
            stderr.lines().any(|line| line == *expected),
            "{tag}: missing diagnostic `{expected}`; stderr:\n{stderr}"
        );
    }
    assert_eq!(run.status.code(), Some(1), "{tag}: failing compiles exit 1");
}

/// Asserts that compiling one snippet reports every expected diagnostic line.
fn assert_reports(tag: &str, source: &str, expected_lines: &[&str]) {
    assert_run_reports(tag, run_source(tag, source), expected_lines);
}

/// Asserts that compiling one snippet reports a diagnostic with the given code.
///
/// A full-line assertion is right when the registry fully determines the
/// message. A mixed-sign binary operator fixes the code but not which operand
/// the rendered message names, so the case pins the code alone and leaves the
/// operand direction to the implementation.
fn assert_reports_code(tag: &str, source: &str, code: &str) {
    let run = run_source(tag, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    let head = format!("error {code}: ");
    assert!(
        stderr.lines().any(|line| line.starts_with(&head)),
        "{tag}: missing `{head}...` diagnostic; stderr:\n{stderr}"
    );
    assert_eq!(run.status.code(), Some(1), "{tag}: failing compiles exit 1");
}

/// Asserts that compiling several files together reports the expected lines.
fn assert_reports_across_files(tag: &str, files: &[(&str, &str)], expected_lines: &[&str]) {
    assert_run_reports(tag, run_sources(tag, files), expected_lines);
}

/// Asserts that a compile produced no diagnostics at all and exited zero.
///
/// A warning is a diagnostic, so this rejects warnings too. It is the stricter
/// sibling of [`assert_front_end_accepts`], which tolerates the diagnostics a
/// later stage may add but accepts a program the in-scope stages understand.
fn assert_no_diagnostics(tag: &str, run: Output) {
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.trim().is_empty(),
        "{tag}: expected no diagnostics; stderr:\n{stderr}"
    );
    assert_eq!(run.status.code(), Some(0), "{tag}: a clean compile exits 0");
}

/// Asserts that one compile reported the exact warning first line and exited 0.
///
/// Warnings are deniable by default, so the default build still exits 0; the
/// case pins both the first line and that exit code.
fn assert_warning_first_line(tag: &str, run: Output, expected: &str) {
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.lines().any(|line| line == expected),
        "{tag}: missing warning `{expected}`; stderr:\n{stderr}"
    );
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: warnings are deniable by default"
    );
}

/// Asserts that compiling one snippet reports the exact warning first line.
fn assert_reports_warning(tag: &str, source: &str, expected: &str) {
    assert_warning_first_line(tag, run_source(tag, source), expected);
}

/// Asserts that a compile did not report a warning with the given code and
/// exited zero.
///
/// The warning-free half of a profile-threshold case uses this: a frame below
/// the profile's limit must not draw the large-frame warning.
fn assert_omits_warning(tag: &str, run: Output, code: &str) {
    let stderr = String::from_utf8_lossy(&run.stderr);
    let head = format!("warning {code}: ");
    assert!(
        !stderr.lines().any(|line| line.starts_with(&head)),
        "{tag}: unexpected `{head}...` warning; stderr:\n{stderr}"
    );
    assert_eq!(run.status.code(), Some(0), "{tag}: a clean compile exits 0");
}

/// Compiles one source snippet and returns the process output and emitted C.
///
/// The plain `run_source` leaves the emitted file inside a scratch directory
/// that is removed when the call returns. A codegen case needs the text, so
/// this variant reads it back first. A missing or unreadable output becomes an
/// empty string; the caller treats that as a failed emission.
fn run_source_emitting(tag: &str, source: &str) -> (Output, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("cannot write snippet: {err}"));
    let output_path = scratch.path().join("out.c");
    let run = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    (run, read_emitted(&output_path))
}

/// Compiles one source snippet with the hosted-amiga profile and returns its C.
///
/// The hosted-amiga translation unit cannot be compiled on the host, but its
/// declaration surface (the ABI typedefs and the shim includes) can be asserted
/// as text.
fn run_source_emitting_hosted_amiga(tag: &str, source: &str) -> (Output, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("cannot write snippet: {err}"));
    let output_path = scratch.path().join("out.c");
    let run = run_clc(&[
        "--profile".to_owned(),
        "hosted-amiga".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    (run, read_emitted(&output_path))
}

/// Compiles several source files and returns the process output and emitted C.
fn run_sources_emitting(tag: &str, files: &[(&str, &str)]) -> (Output, String) {
    let scratch = ScratchDir::new(tag);
    let output_path = scratch.path().join("out.c");
    let mut args = vec![
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
    ];
    for (name, source) in files {
        let source_path = scratch.path().join(name);
        std::fs::write(&source_path, source)
            .unwrap_or_else(|err| panic!("cannot write snippet {name}: {err}"));
        args.push(source_path.display().to_string());
    }
    (run_clc(&args), read_emitted(&output_path))
}

/// Compiles one snippet whose path on the command line is relative.
///
/// The command runs with the scratch directory as its working directory, so
/// the path the driver sees is exactly `relative_path`. That is the logical
/// path that reaches the emitted `#line` directives and the diagnostic
/// locations, and it must stay relative rather than becoming an absolute path.
fn run_source_relative(tag: &str, relative_path: &str, source: &str) -> (Output, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join(relative_path);
    if let Some(parent) = source_path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", parent.display()));
    }
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("cannot write snippet: {err}"));
    let output_path = scratch.path().join("out.c");
    let run = Command::new(env!("CARGO_BIN_EXE_clc"))
        .current_dir(scratch.path())
        .args([
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            output_path.display().to_string(),
            relative_path.to_owned(),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
    (run, read_emitted(&output_path))
}

/// Compiles one snippet with extra flags inserted between `--profile` and `-o`.
fn run_source_with_flags(tag: &str, source: &str, flags: &[&str]) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("cannot write snippet: {err}"));
    let output_path = scratch.path().join("out.c");
    let mut args = vec!["--profile".to_owned(), "host".to_owned()];
    args.extend(flags.iter().map(|flag| (*flag).to_owned()));
    args.push("-o".to_owned());
    args.push(output_path.display().to_string());
    args.push(source_path.display().to_string());
    run_clc(&args)
}

/// Reads the emitted C, treating a missing file as empty output.
fn read_emitted(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Asserts that a codegen case compiled cleanly and produced C text.
///
/// The emitter writes an empty file until it lands, and an empty translation
/// unit is never a passing codegen result. Failing here names the missing
/// emitter instead of letting a later substring check report the same gap.
fn assert_emitted(tag: &str, run: &Output, emitted: String) -> String {
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: the front end rejected a valid snippet; stderr:\n{stderr}"
    );
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
    );
    emitted
}

/// Counts the cleanup labels defined in the emitted C.
///
/// A definition is a line whose first token is `__cl_c` followed by digits and
/// a colon; a trailing comment is allowed. A `goto` reference never starts a
/// line this way, so the count is the number of scopes that own a label.
fn cleanup_label_count(emitted: &str) -> usize {
    emitted
        .lines()
        .filter(|line| {
            let Some(rest) = line.trim_start().strip_prefix("__cl_c") else {
                return false;
            };
            let digits = rest.chars().take_while(|ch| ch.is_ascii_digit()).count();
            digits > 0 && rest[digits..].starts_with(':')
        })
        .count()
}

/// Counts the C function bodies in `emitted` whose signature names `symbol`.
///
/// The emitted C puts the opening brace on its own line, so a body is a
/// signature line ending in `)` whose next line opens a brace. A forward
/// prototype ends in `;` and a call is not followed by a brace, so the count
/// is the number of emitted definitions rather than references.
fn definition_count(emitted: &str, symbol: &str) -> usize {
    let lines: Vec<&str> = emitted.lines().collect();
    lines
        .windows(2)
        .filter(|pair| {
            pair[0].contains(symbol)
                && pair[0].trim_end().ends_with(')')
                && pair[1].trim_start().starts_with('{')
        })
        .count()
}

/// Asserts that every needle appears in `text` in the given order.
fn assert_in_order(tag: &str, text: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        match text[cursor..].find(needle) {
            Some(offset) => cursor += offset + needle.len(),
            None => panic!("{tag}: `{needle}` is missing or out of order in:\n{text}"),
        }
    }
}

/// Counts rendered error head lines in captured stderr.
fn error_head_count(stderr: &[u8]) -> usize {
    String::from_utf8_lossy(stderr)
        .lines()
        .filter(|line| line.starts_with("error "))
        .count()
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

/// A comparison chain and a generic call both parse under the bounded
/// speculative rule.
///
/// A parenthesized comparison is unambiguous, and `f<i32>(1)` is the explicit
/// generic-call form. The bare `a < b > (c)` shape is the ambiguous one: the
/// speculative parse binds it as a generic call `a<b>(c)`, so it must not
/// produce a comparison diagnostic.
#[test]
fn comparison_and_generic_call_forms_parse() {
    assert_front_end_accepts(
        "parse-call-forms",
        "module spec.stage.parse_call_forms;

func identity<T>(x: T): T {
    return x;
}

func shapes(a: i32, b: i32, c: i32): i32 {
    let comparison = a < b;
    let explicit_args = identity<i32>(1);
    let call_shaped = a < b > (c);
    return 0;
}

func main(): i32 {
    return shapes(1, 2, 3);
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

/// Names resolve in lexical scopes with the innermost binding winning.
///
/// The specification's shadowing example must be accepted: a parameter shadows
/// a module value and a local shadows the parameter, so the same spelling
/// legally binds to three different declarations. A reference before the
/// name's declaration in its own block is instead E10219, proving that a block
/// local is not visible at an earlier point even though the spelling exists
/// elsewhere.
#[test]
fn name_resolution_honors_scopes_and_declaration_order() {
    assert_front_end_accepts(
        "name-shadowing",
        "module spec.stage.name_shadowing;

let LIMIT: i32 = 100;

func pick(LIMIT: i32): i32 {       // the parameter shadows the module value
    let doubled: i32 = LIMIT * 2;  // LIMIT here is the parameter
    let LIMIT: i32 = doubled;      // the local shadows the parameter
    return LIMIT;
}

func main(): i32 {
    return pick(5);
}
",
    );

    assert_reports(
        "name-use-before-declaration",
        "module spec.stage.name_use_before_declaration;

func demo(): i32 {
    let first: i32 = second;   // E10219: declared later in this block
    let second: i32 = 2;
    return first + second;
}
",
        &["error E10219: 'second' is not declared"],
    );
}

/// Files that declare the same module merge, and imports bind module names.
///
/// Two files with the same `module` declaration share one namespace, so one
/// function declared in both is E11101. An import naming a missing export is
/// E11102, and binding one imported name twice is E11103; the rename form with
/// `as` is the documented escape hatch and is not exercised here.
#[test]
fn merged_modules_and_import_bindings_report_their_codes() {
    assert_reports_across_files(
        "module-merged-duplicate",
        &[
            (
                "alpha.clo",
                "module spec.stage.merged;

func f(): i32 {
    return 1;
}
",
            ),
            (
                "beta.clo",
                "module spec.stage.merged;

func f(): i32 {
    return 2;
}

func main(): i32 {
    return f();
}
",
            ),
        ],
        &["error E11101: duplicate declaration of 'f' in module 'spec.stage.merged'"],
    );

    assert_reports(
        "import-not-exported",
        "module spec.stage.import_not_exported;

import { clamp } from Demo.MinMax;

func main(): i32 {
    return clamp(0);
}
",
        &["error E11102: 'clamp' is not exported from module 'Demo.MinMax'"],
    );

    assert_reports(
        "import-bound-twice",
        "module spec.stage.import_bound_twice;

import { clamp } from Demo.MinMax;
import { clamp } from Demo.More;

func main(): i32 {
    return clamp(0);
}
",
        &["error E11103: duplicate import of 'clamp'"],
    );
}

/// Same-sign integer operands unify to a common type, and conversions the
/// language does not allow are reported: a mixed-sign operator, a literal that
/// overflows its context type, and an implicit float-to-integer assignment.
///
/// The `u8 + u16` result type is only observable once code is emitted, a later
/// stage, so that part pins acceptance instead of the unified type. The
/// mixed-sign message names one operand and the specification does not fix
/// which one, so it pins the code alone.
#[test]
fn integer_operands_unify_and_bad_conversions_report_their_codes() {
    assert_front_end_accepts(
        "type-widening",
        "module spec.stage.type_widening;

func sum(a: u8, b: u16): u16 {
    return a + b;
}

func main(): i32 {
    return 0;
}
",
    );

    assert_reports_code(
        "type-mixed-sign",
        "module spec.stage.type_mixed_sign;

func demo(): i32 {
    let a: i32 = 1;
    let b: u32 = 2;
    let c = a + b;
    return c;
}

func main(): i32 {
    return 0;
}
",
        "E10200",
    );

    assert_reports(
        "type-literal-overflow",
        "module spec.stage.type_literal_overflow;

func demo(): u8 {
    let x: u8 = 300;
    return x;
}

func main(): i32 {
    return 0;
}
",
        &["error E10202: literal does not fit 'u8'"],
    );

    assert_reports(
        "type-float-to-integer",
        "module spec.stage.type_float_to_integer;

func demo(): i32 {
    let y: i32 = 1.5;
    return y;
}

func main(): i32 {
    return 0;
}
",
        &["error E10200: cannot convert 'f64' to 'i32' implicitly"],
    );
}

/// Explicit casts cover widening, narrowing, and float conversions, and `f32`
/// widens to `f64` implicitly.
///
/// The specification allows every form here, so the case pins acceptance: any
/// type diagnostic would be a false rejection.
#[test]
fn explicit_casts_and_float_widening_are_accepted() {
    assert_front_end_accepts(
        "type-casts",
        "module spec.stage.type_casts;

func demo(): i32 {
    let small: u8 = 1;
    let wide: u16 = u16(small);
    let narrow: u8 = u8(wide);
    let exact: f64 = 1.5;
    let as_int: i32 = i32(exact);
    let as_float: f32 = f32(exact);
    let promoted: f64 = as_float;
    return as_int;
}

func main(): i32 {
    return 0;
}
",
    );
}

/// A non-`bool` condition, a non-`bool` logical operand, and an operator
/// applied to the wrong operand type are reported, including ordering an enum.
#[test]
fn non_bool_conditions_and_undefined_operators_report_their_codes() {
    assert_reports(
        "type-non-bool-condition",
        "module spec.stage.type_non_bool_condition;

func demo(): i32 {
    if (1) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
",
        &["error E10218: condition must be 'bool'"],
    );

    assert_reports(
        "type-logical-operand",
        "module spec.stage.type_logical_operand;

func demo(): bool {
    let x = 1 && true;
    return x;
}

func main(): i32 {
    return 0;
}
",
        &["error E10218: condition must be 'bool'"],
    );

    assert_reports(
        "type-enum-ordering",
        "module spec.stage.type_enum_ordering;

enum E : u8 {
    A,
    B,
}

func demo(): bool {
    let ordered = E.A > E.B;
    return ordered;
}

func main(): i32 {
    return 0;
}
",
        &["error E10222: operator '>' is not defined for 'E'"],
    );
}

/// A module `const` initializer folds with runtime semantics, and the two
/// constant faults are reported: division by zero and a runtime value where a
/// constant is required.
///
/// The folded value `14` is only observable once code is emitted, so that part
/// pins acceptance instead of the folded value.
#[test]
fn constant_initializers_fold_and_constant_faults_report_their_codes() {
    assert_front_end_accepts(
        "const-fold",
        "module spec.stage.const_fold;

const N: i32 = 2 + 3 * 4;

func main(): i32 {
    return N;
}
",
    );

    assert_reports(
        "const-division-by-zero",
        "module spec.stage.const_division_by_zero;

const Z: i32 = 1 / 0;

func main(): i32 {
    return Z;
}
",
        &["error E10223: constant expression divides by zero"],
    );

    assert_reports(
        "const-non-constant",
        "module spec.stage.const_non_constant;

let n: i32 = 5;
const M: i32 = n;

func main(): i32 {
    return M;
}
",
        &["error E10224: expression is not constant"],
    );
}

/// Enum members without an explicit value number sequentially from zero, and
/// the enum converts to and from its backing integer type only through
/// explicit casts.
///
/// The member values `0`, `5`, and `6` are only observable once code is
/// emitted, so the case pins acceptance instead of the member values.
#[test]
fn enum_members_number_sequentially_and_cast_explicitly() {
    assert_front_end_accepts(
        "enum-values",
        "module spec.stage.enum_values;

enum E : u8 {
    A,
    B = 5,
    C,
}

func demo(): u8 {
    let x: u8 = 0;
    let e: E = E(x);
    return u8(e);
}

func main(): i32 {
    return 0;
}
",
    );
}

/// A template placeholder that holds a non-formattable type and a template
/// with an unbalanced brace each report their registered code.
///
/// `String` is a standard-library type, so it is imported: the placeholder
/// check runs after name resolution, and a program that does not resolve could
/// not reach that check.
#[test]
fn template_placeholder_diagnostics_report_their_codes() {
    assert_reports(
        "template-non-formattable",
        "module spec.stage.template_non_formattable;

import { String } from std.text;

struct Point {
    x: i32;
    y: i32;
}

func sink(s: String): void {
}

func demo(): void {
    let p = Point { x: 1, y: 2 };
    let s: String = $\"at {p}\";
    sink(s);
}

func main(): i32 {
    return 0;
}
",
        &["error E10500: type 'Point' is not formattable"],
    );

    assert_reports(
        "template-unbalanced",
        "module spec.stage.template_unbalanced;

import { String } from std.text;

func demo(): String {
    let n: u32 = 1;
    return $\"n={n\";
}

func main(): i32 {
    return 0;
}
",
        &["error E10503: unbalanced braces in template"],
    );
}

/// A move invalidates its source, an owner may not move while a loan of it is
/// live, and a `const` binding is not assignable.
///
/// Each fault is independent, so it gets its own snippet: the move in the
/// first, the live shared loan in the second, and the immutable binding in the
/// third.
#[test]
fn moves_loans_and_const_bindings_report_their_codes() {
    assert_reports(
        "own-use-after-move",
        "module spec.stage.own_move_then_use;

func observe(s: String): void {
}

func demo(): void {
    let s: String = $\"x{1}\";
    let t: String = s;      // move: s is invalid after this
    observe(s);             // E10020
    observe(t);
}

func main(): i32 {
    return 0;
}
",
        &["error E10020: 's' was moved and cannot be used"],
    );

    assert_reports(
        "own-move-while-borrowed",
        "module spec.stage.own_move_while_borrowed;

func viewOf(s: str): str {
    return s;
}

func observe(s: str): void {
}

func consume(take s: String): void {
}

func demo(): void {
    let s: String = $\"x{1}\";
    let v: str = viewOf(s); // a live shared loan of s
    consume(s);             // E10022
    observe(v);
}

func main(): i32 {
    return 0;
}
",
        &["error E10022: cannot move 's' while it is borrowed"],
    );

    assert_reports(
        "own-assign-to-const",
        "module spec.stage.own_assign_to_const;

func demo(): void {
    const limit: i32 = 10;
    limit = 20;             // E10023
}

func main(): i32 {
    return 0;
}
",
        &["error E10023: cannot assign to immutable 'limit'"],
    );
}

/// A view return must derive from exactly one view-like lend parameter, and a
/// view of a local owner cannot be returned.
///
/// The first snippet has two view-like lends, the second returns a view of a
/// local, and the third has only a `take` parameter. `take` creates no loan, so
/// it is not an elision candidate and the return still has zero lend sources.
#[test]
fn view_returns_elide_from_exactly_one_lend_parameter() {
    assert_reports(
        "own-view-two-params",
        "module spec.stage.own_view_two_params;

func pick(a: u8[], b: u8[]): u8[] {
    return a;               // E10036: two candidates
}

func main(): i32 {
    return 0;
}
",
        &["error E10036: view return does not derive from exactly one parameter"],
    );

    assert_reports(
        "own-view-of-local",
        "module spec.stage.own_view_of_local;

func viewOf(s: str): str {
    return s;
}

func bad(): str {
    let s: String = $\"x{1}\";
    return viewOf(s);       // E10035
}

func main(): i32 {
    return 0;
}
",
        &["error E10035: cannot return a view of local 's'"],
    );

    assert_reports(
        "own-view-take-only",
        "module spec.stage.own_view_take_only;

func takeView(take s: str): str {
    return s;               // E10036: 'take' is not an elision candidate
}

func main(): i32 {
    return 0;
}
",
        &["error E10036: view return does not derive from exactly one parameter"],
    );
}

/// A nullable value is usable only after an `== null` / `!= null` narrowing on
/// the dominating path, and `null` is not allowed where a non-nullable type is
/// required.
#[test]
fn nullability_narrowing_and_null_assignment_report_their_codes() {
    assert_reports(
        "own-nullable-use",
        "module spec.stage.own_nullable_use;

handle File freedBy CloseFile;

ffi lib {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
}

func use(f: File): void {
}

func demo(): void {
    let f: File? = OpenFile();
    use(f);                 // E10010: f may be null here
}

func main(): i32 {
    return 0;
}
",
        &["error E10010: value 'f' may be null here"],
    );

    assert_reports(
        "own-null-where-nonnull",
        "module spec.stage.own_null_where_nonnull;

func demo(): i32 {
    let n: i32 = null;      // E10012
    return n;
}

func main(): i32 {
    return 0;
}
",
        &["error E10012: 'null' is not allowed here"],
    );
}

/// Partial moves are limited to a top-level field of an owned local or `take`
/// parameter; a nested path is E10026, and a whole-value use of a partially
/// moved value is E10025.
///
/// The `observePair` parameter is the same struct type, so the second snippet
/// isolates the partial-move fault and does not also carry a type mismatch.
#[test]
fn partial_moves_report_their_codes() {
    assert_reports(
        "own-nested-partial-move",
        "module spec.stage.own_nested_partial_move;

struct Inner {
    s: String;
}

struct Outer {
    inner: Inner;
}

func observe(s: String): void {
}

func demo(): void {
    let o = Outer { inner: Inner { s: $\"x{1}\" } };
    let s: String = o.inner.s;  // E10026: nested-path move
    observe(s);
}

func main(): i32 {
    return 0;
}
",
        &["error E10026: partial moves are limited to top-level fields in v1"],
    );

    assert_reports(
        "own-partial-move-whole-use",
        "module spec.stage.own_partial_move_whole_use;

struct Pair {
    left: String;
    right: String;
}

func observePair(p: Pair): void {
}

func observe(s: String): void {
}

func demo(): void {
    let p = Pair { left: $\"a{1}\", right: $\"b{2}\" };
    let l: String = p.left; // partial move of a top-level field
    observePair(p);         // E10025: p is partially moved
    observe(l);
}

func main(): i32 {
    return 0;
}
",
        &["error E10025: 'p' is partially moved and cannot be used as a whole"],
    );
}

/// A writable view derived from a `borrow` parameter is an exclusive reborrow
/// limited to the expression that creates it.
///
/// `bufferBytes(buf)` derives a writable view from the writable loan `buf`, so
/// `borrow bufferBytes(buf)` is legal and `buf` is usable again right after the
/// call. The case pins acceptance: any ownership diagnostic here is a false
/// rejection.
#[test]
fn exclusive_reborrow_of_a_borrow_parameter_is_accepted() {
    assert_front_end_accepts(
        "own-exclusive-reborrow",
        "module spec.stage.own_exclusive_reborrow;

import { Buffer, bufferBytes } from std.mem;

func writeInto(borrow bytes: u8[]): void {
    bytes[0] = 1;
}

func loadFrom(borrow buf: Buffer): void {
    writeInto(borrow bufferBytes(buf));
}

func main(): i32 {
    return 0;
}
",
    );
}

/// A well-ordered program with locals, owned struct fields, a statement
/// temporary, and a `defer` compiles clean.
///
/// Drop order itself is only observable in the emitted ladder, which is a
/// codegen assertion; at this stage the case pins that the planned scope
/// cleanup is accepted. The locals are declared `a` then `r`, `r` holds owned
/// fields, `observe($"temp{4}")` creates a statement temporary, and the `defer`
/// is registered last.
#[test]
fn well_ordered_drops_compile_clean() {
    assert_front_end_accepts(
        "own-drop-order",
        "module spec.stage.own_drop_order;

struct Resources {
    first: String;
    second: String;
}

func observe(s: String): void {
}

func demo(): void {
    let a: String = $\"a{1}\";
    let r = Resources { first: $\"b{2}\", second: $\"c{3}\" };
    defer {
        observe(a);
    }
    observe($\"temp{4}\");
    observe(r.first);
    observe(r.second);
}

func main(): i32 {
    return 0;
}
",
    );
}

/// The FFI boundary enforces its contracts: an uncontracted extern may be
/// called only inside `unsafe`, its uncontracted parameters accept only
/// values with no ownership, and a `freedBy` deleter must take `take T` and
/// return `void`.
///
/// The three faults are independent, so each gets its own snippet. The
/// `E10010` nullable check lives with the other nullability cases.
#[test]
fn ffi_contract_violations_report_their_codes() {
    assert_reports(
        "ffi-uncontracted-call",
        "module spec.stage.ffi_uncontracted_call;

handle File freedBy CloseFile;

ffi lib {
    func RawRead(f: File?, n: i32): i32;   // no mode on 'f': uncontracted
    func CloseFile(take f: File): void;
}

func demo(): i32 {
    let f: File? = null;
    return RawRead(f, 0);   // E10100
}

func main(): i32 {
    return 0;
}
",
        &["error E10100: uncontracted call to 'RawRead' requires 'unsafe'"],
    );

    assert_reports(
        "ffi-owned-to-uncontracted",
        "module spec.stage.ffi_owned_to_uncontracted;

handle File freedBy CloseFile;

ffi lib {
    func RawStat(f: File): i32;    // uncontracted extern
    func CloseFile(take f: File): void;
}

func demo(f: File): void {
    unsafe {
        let n = RawStat(f);        // E10101: no contract for parameter 'f'
        n = n + 1;
    }
}

func main(): i32 {
    return 0;
}
",
        &["error E10101: 'RawStat' has no contract for parameter 'f'"],
    );

    assert_reports(
        "ffi-bad-deleter",
        "module spec.stage.ffi_bad_deleter;

handle File freedBy BadDeleter;

ffi lib {
    func BadDeleter(f: File): i32;  // E10102: needs 'take File' and 'void'
}

func main(): i32 {
    return 0;
}
",
        &["error E10102: 'BadDeleter' is not a valid deleter for 'File'"],
    );
}

/// A function that owns resources in nested scopes gets one cleanup label per
/// owning scope, and every fall-through drop is null-safe.
///
/// The outer scope owns `first` and the inner block owns `second`, so the
/// ladder has exactly two labels and neither drop can run on an empty value.
/// The drop shape and the label naming are fixed, so the case asserts them
/// exactly.
#[test]
fn cleanup_ladder_has_one_label_per_owning_scope() {
    let source = r#"module spec.stage.cg_ladder;

handle File freedBy CloseFile;

ffi lib {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
}

func acquireBoth(): bool {
    let first: File? = OpenFile();
    if (first == null) {
        return false;
    }
    {
        let second: File? = OpenFile();
        if (second == null) {
            return false;
        }
    }
    return true;
}

func main(): i32 {
    if (acquireBoth()) {
        return 0;
    }
    return 1;
}
"#;
    let (run, emitted) = run_source_emitting("cg-ladder", source);
    let c = assert_emitted("cg-ladder", &run, emitted);
    assert_eq!(
        cleanup_label_count(&c),
        2,
        "one label per owning scope:\n{c}"
    );
    assert!(c.contains("__cl_c1:"), "the inner label is numbered first:\n{c}");
    assert!(c.contains("__cl_c2:"), "the outer label follows it:\n{c}");
    assert!(
        c.contains("if (first) { CloseFile(first); }"),
        "the outer drop is null-safe:\n{c}"
    );
    assert!(
        c.contains("if (second) { CloseFile(second); }"),
        "the inner drop is null-safe:\n{c}"
    );
}

/// A handle move empties the source with a null write, and an aggregate move
/// zeroes the source so a later drop is a no-op.
///
/// The move spellings are fixed by the ownership-to-code contract, so the case
/// asserts the exact assignments for one handle and one struct with owned
/// fields.
#[test]
fn handle_and_aggregate_moves_zero_the_source() {
    let source = r#"module spec.stage.cg_moves;

import { String } from std.text;

handle File freedBy CloseFile;

ffi lib {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
}

struct Pair {
    left: String;
    right: String;
}

func observeHandle(f: File): void {
}

func observePair(p: Pair): void {
}

func demo(): void {
    let a: File? = OpenFile();
    if (a == null) {
        return;
    }
    let b: File = a;
    observeHandle(b);

    let p = Pair { left: $"a{1}", right: $"b{2}" };
    let q: Pair = p;
    observePair(q);
}

func main(): i32 {
    demo();
    return 0;
}
"#;
    let (run, emitted) = run_source_emitting("cg-moves", source);
    let c = assert_emitted("cg-moves", &run, emitted);
    assert!(c.contains("b = a;"), "a handle move copies first:\n{c}");
    assert!(c.contains("a = NULL;"), "a handle move empties the source:\n{c}");
    assert!(c.contains("q = p;"), "an aggregate move copies first:\n{c}");
    assert!(c.contains("cl_zero(&p);"), "an aggregate move zeroes the source:\n{c}");
}

/// Every emitted statement is mapped to its logical source path by a `#line`
/// directive, and the path is the command-line path, never an absolute one.
///
/// The command passes `sub/main.clo` from the scratch directory, and `return 0`
/// sits on line 4 of that file, so the mapping is checked for both the exact
/// line number and the relative path spelling.
#[test]
fn line_directives_name_the_logical_cli_path() {
    let source = r#"module spec.stage.cg_line;

func main(): i32 {
    return 0;
}
"#;
    let (run, emitted) = run_source_relative("cg-line", "sub/main.clo", source);
    let c = assert_emitted("cg-line", &run, emitted);
    assert!(c.contains("#line "), "the emitted C carries line mappings:\n{c}");
    assert!(
        c.contains("#line 4 \"sub/main.clo\""),
        "the return maps to line 4 of the relative path:\n{c}"
    );
}

/// The three arithmetic hazards lower to their fixed runtime guards.
///
/// Division uses the zero-and-minimum guard, the bounds check runs before an
/// index, and the float-to-integer guard leads with the NaN bit test. The
/// guard expressions are fixed; the bounds comparison spelling is not fixed, so
/// that part asserts that the check reaches the fault code instead.
#[test]
fn division_index_and_float_guards_are_emitted() {
    let source = r#"module spec.stage.cg_guards;

func divide(a: i32, d: i32): i32 {
    return a / d;
}

func index(xs: u32[2], i: usize): u32 {
    return xs[i];
}

func convert(f: f64): i32 {
    return i32(f);
}

func main(): i32 {
    let xs: u32[2] = [1, 2];
    let i: usize = 0;
    let q: i32 = divide(10, 3);
    let v: u32 = index(xs, i);
    let n: i32 = convert(1.5);
    return q + n + i32(v);
}
"#;
    let (run, emitted) = run_source_emitting("cg-guards", source);
    let c = assert_emitted("cg-guards", &run, emitted);
    assert!(
        c.contains("d == 0 || (a == "),
        "the division guard tests a zero divisor first:\n{c}"
    );
    assert!(
        c.contains(" && d == -1)"),
        "the division guard also catches the minimum over -1 case:\n{c}"
    );
    assert!(c.contains("E11210"), "the division guard reaches E11210:\n{c}");
    assert!(c.contains("E11211"), "every index reaches the E11211 check:\n{c}");
    assert!(
        c.contains("cl_isnan(f)"),
        "the float guard leads with the NaN bit test:\n{c}"
    );
    assert!(
        c.contains("!(f >= ") && c.contains("f <= "),
        "the float guard range-checks the value:\n{c}"
    );
    assert!(c.contains("E11212"), "the float guard reaches E11212:\n{c}");
}

/// An `ffi` declaration emits a prototype built from the ABI mapping.
///
/// A parameter mode never shapes the prototype: the declared type alone fixes
/// the C type. `File` maps to `BPTR`, a view to `APTR`, and `i32` to `LONG`, so
/// the `Write` prototype is the fixed ABI spelling the specification pins.
#[test]
fn ffi_prototypes_use_the_abi_mapping() {
    let source = r#"module spec.stage.cg_ffi_abi;

handle File freedBy CloseFile;

ffi dos {
    func Write(borrow file: File, const buf: u8[], n: i32): i32;
    func CloseFile(take f: File): void;
}

func main(): i32 {
    return 0;
}
"#;
    let (run, emitted) = run_source_emitting("cg-ffi-abi", source);
    let c = assert_emitted("cg-ffi-abi", &run, emitted);
    assert!(
        c.contains("LONG Write(BPTR, APTR, LONG);"),
        "the Write prototype is the ABI spelling:\n{c}"
    );
    assert!(
        c.contains("void CloseFile(BPTR);"),
        "the deleter prototype is the ABI spelling:\n{c}"
    );
}

/// A slice view uses a mutable element pointer so a write through it compiles.
///
/// A `borrow T[]` parameter is writable, and the same slice typedef serves a
/// read-only lend, so the element pointer cannot be `const` without rejecting
/// the writable case.
#[test]
fn a_slice_view_uses_a_mutable_element_pointer() {
    let source = r#"module spec.stage.cg_slice_mut;

func fill(borrow xs: u8[]): void {
    xs[0] = 1;
}

func main(): i32 {
    return 0;
}
"#;
    let (run, emitted) = run_source_emitting("cg-slice-mut", source);
    let c = assert_emitted("cg-slice-mut", &run, emitted);
    assert!(
        c.contains("typedef struct { u8 *ptr; usize len; } ClSlice_u8;"),
        "the slice element pointer is mutable:\n{c}"
    );
}

/// The hosted-amiga unit relies on the NDK headers for the AmigaOS ABI types.
///
/// The shim includes `exec/types.h` and `dos/dos.h`, which already declare
/// `BPTR`, `LONG`, `ULONG`, `STRPTR`, and `BOOL`; redeclaring them under
/// `-std=c99` is an error. The Clorraine fixed-width scalars are still emitted
/// because no target header provides them.
#[test]
fn hosted_amiga_emits_no_clashing_abi_typedefs() {
    let source = r#"module spec.stage.cg_hosted_typedefs;

handle File freedBy CloseFile;

ffi dos {
    func OpenFile(): File?;
    func CloseFile(take f: File): void;
}

func demo(): void {
    let f: File? = OpenFile();
    if (f == null) {
        return;
    }
    CloseFile(f);
}

func main(): i32 {
    demo();
    return 0;
}
"#;
    let (run, emitted) = run_source_emitting_hosted_amiga("cg-hosted-typedefs", source);
    let c = assert_emitted("cg-hosted-typedefs", &run, emitted);
    for clash in [
        "typedef int LONG;",
        "typedef unsigned int ULONG;",
        "typedef char *STRPTR;",
        "typedef int BOOL;",
    ] {
        assert!(
            !c.contains(clash),
            "the hosted-amiga unit does not redeclare `{clash}`:\n{c}"
        );
    }
    assert!(
        c.contains("typedef int i32;"),
        "the fixed-width scalars are still emitted:\n{c}"
    );
    assert!(
        c.contains("#include <dos/dos.h>"),
        "the shim includes the header that declares BPTR:\n{c}"
    );
    // `BPTR` is an integer on the target, so the file handle's empty state and
    // null comparison use the integer constant rather than `NULL`.
    assert!(
        c.contains("BPTR f = 0;"),
        "an integer handle is initialized with the integer null:\n{c}"
    );
    assert!(
        c.contains("(f == 0)"),
        "an integer handle compares against the integer null:\n{c}"
    );
}

/// Module initializers are generated as `cl_init_*` functions and the entry
/// stub runs them in topological order before the user's `main`.
///
/// Two modules are independent and one depends on another. The independent
/// pair meets the lexicographic tie break, and the dependent module must wait
/// for the module its initializer names, so the stub's call order is
/// `Test.Bravo`, `Test.Zeta`, `Test.Alpha`, and only then the user's `main`.
#[test]
fn module_initializers_run_in_topological_order_before_main() {
    let (run, emitted) = run_sources_emitting(
        "cg-init",
        &[
            (
                "z.clo",
                "module Test.Zeta;

export let z: i32 = 1;
",
            ),
            (
                "b.clo",
                "module Test.Bravo;

export let b: i32 = 2;
",
            ),
            (
                "a.clo",
                "module Test.Alpha;

import { z } from Test.Zeta;

export let a: i32 = z + 1;

func main(): i32 {
    return a;
}
",
            ),
        ],
    );
    let c = assert_emitted("cg-init", &run, emitted);
    assert_in_order(
        "cg-init",
        &c,
        &[
            "cl_init_Test_Bravo()",
            "cl_init_Test_Zeta()",
            "cl_init_Test_Alpha()",
            "cl_Test_Alpha_main()",
        ],
    );
}

/// One generic body per distinct type key: two call sites that key `i32` share
/// one symbol, and a distinct key gets its own type-keyed symbol.
///
/// The `i32` key is reached both by an explicit `identity<i32>` call and by a
/// type parameter instantiated at `i32`, so a missed dedupe would emit two
/// bodies under one name. The distinct `u8` key proves the `__<typekey>`
/// suffix.
#[test]
fn identical_generic_instantiations_share_one_symbol() {
    let source = r#"module spec.stage.cg_generic;

func identity<T>(x: T): T {
    return x;
}

func forward<t2>(x: t2): t2 {
    return identity<t2>(x);
}

func other(y: u8): u8 {
    return identity<u8>(y);
}

func demo(x: i32): i32 {
    let a: i32 = identity<i32>(x);
    let b: i32 = forward<i32>(x);
    let c: u8 = other(u8(0));
    return a + b + i32(c);
}

func main(): i32 {
    return demo(1);
}
"#;
    let (run, emitted) = run_source_emitting("cg-generic", source);
    let c = assert_emitted("cg-generic", &run, emitted);
    assert_eq!(
        definition_count(&c, "cl_identity__i32"),
        1,
        "the two i32-keyed call sites dedupe to one body:\n{c}"
    );
    assert_eq!(
        definition_count(&c, "cl_identity__u8"),
        1,
        "the distinct key emits one body under its type key:\n{c}"
    );
}

/// Two modules with a same-named generic function report a duplicate
/// declaration instead of silently sharing one emitted symbol.
///
/// A generic instance is named `cl_<name>__<typekey>` without a module
/// qualifier, so the two declarations cannot both be represented. A
/// single-module program is unaffected and still dedupes its instances.
#[test]
fn same_named_generics_in_two_modules_report_a_duplicate_declaration() {
    let files = [
        (
            "a.clo",
            "module Dup.A;

func identity<T>(x: T): T { return x; }

export func useA(): i32 { return identity<i32>(1); }
",
        ),
        (
            "b.clo",
            "module Dup.B;

func identity<T>(x: T): T { return x + 100; }

export func useB(): i32 { return identity<i32>(2); }
",
        ),
        (
            "main.clo",
            "module Dup.Main;

import { useA } from Dup.A;
import { useB } from Dup.B;

func main(): i32 { return useA() + useB(); }
",
        ),
    ];
    assert_reports_across_files(
        "rv434",
        &files,
        &["error E11101: duplicate declaration of 'identity' in module 'Dup.B'"],
    );
}

/// Float types and casts lower to C `float` and `double`, and `text` lowers to
/// the generated float-to-text helper.
///
/// The `f32`/`f64` mapping and the C-cast lowering are fixed. The fixed
/// six-significant-digit precision that `text` promises is a property of the
/// emitted library helper rather than a fixed spelling of the call, so this
/// case checks the helper call reaches the output.
#[test]
fn float_and_double_operations_lower_to_c_float_code() {
    let source = r#"module spec.stage.cg_float;

import { text } from std.fmt;
import { String } from std.text;

func observe(s: String): void {
}

func demo(): f64 {
    let x: f32 = 1.5;
    let y: f64 = 2.5;
    let sum: f64 = y + f64(x);
    let wide: f64 = x;
    let back: f32 = f32(y);
    let cmp: bool = sum > wide;
    let s: String = text(sum);
    observe(s);
    if (cmp) {
        return sum;
    }
    return f64(back);
}

func main(): i32 {
    let v: f64 = demo();
    return i32(v);
}
"#;
    let (run, emitted) = run_source_emitting("cg-float", source);
    let c = assert_emitted("cg-float", &run, emitted);
    assert!(c.contains("float"), "f32 lowers to C float:\n{c}");
    assert!(c.contains("double"), "f64 lowers to C double:\n{c}");
    assert!(c.contains("(float)"), "the f32 cast is a C float cast:\n{c}");
    assert!(c.contains("(double)"), "the f64 cast is a C double cast:\n{c}");
    assert!(
        c.contains("cl_text_"),
        "text lowers to the generated float-to-text helper:\n{c}"
    );
}

/// The profile selection has three fixed rejections: a missing flag is a usage
/// error, an unknown name is E11201, and the designed but unimplemented
/// `freestanding` profile is E11200.
#[test]
fn profile_flag_errors_report_usage_and_codes() {
    let scratch = ScratchDir::new("cli-profile");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(
        &source_path,
        "module spec.cli.profile;

func main(): i32 {
    return 0;
}
",
    )
    .expect("write snippet");
    let source = source_path.display().to_string();
    let output = scratch.path().join("out.c").display().to_string();

    let missing = run_clc(&["-o".to_owned(), output.clone(), source.clone()]);
    assert_eq!(
        missing.status.code(),
        Some(2),
        "a missing --profile is a usage error"
    );
    assert!(
        String::from_utf8_lossy(&missing.stderr).starts_with("usage error:"),
        "a missing --profile prints a usage error"
    );

    let unknown = run_clc(&[
        "--profile".to_owned(),
        "nope".to_owned(),
        "-o".to_owned(),
        output.clone(),
        source.clone(),
    ]);
    assert_eq!(unknown.status.code(), Some(1), "an unknown profile exits 1");
    assert_eq!(
        String::from_utf8_lossy(&unknown.stderr).lines().next(),
        Some("error E11201: unknown profile 'nope'"),
        "an unknown profile is E11201 on the first line"
    );

    let freestanding = run_clc(&[
        "--profile".to_owned(),
        "freestanding".to_owned(),
        "-o".to_owned(),
        output,
        source,
    ]);
    assert_eq!(
        freestanding.status.code(),
        Some(1),
        "an unimplemented profile exits 1"
    );
    assert_eq!(
        String::from_utf8_lossy(&freestanding.stderr).lines().next(),
        Some("error E11200: 'freestanding' is not available on profile 'freestanding'"),
        "freestanding is E11200 on the first line"
    );
}

/// A qualified `std.amiga.dos` reference is a profile-surface reference.
///
/// The reference loads the module like an import does, so the host profile
/// must reject it with E11200 rather than emit a call to a symbol the host
/// shim lacks; the hosted-amiga profile still accepts and emits it.
#[test]
fn a_qualified_amiga_reference_is_rejected_on_host_and_allowed_on_hosted_amiga() {
    let source = r#"module spec.stage.qualified_amiga;

func main(): i32 {
    std.amiga.dos.delayTicks(1);
    return 0;
}
"#;
    let host = run_source("qualified-amiga", source);
    assert_eq!(
        host.status.code(),
        Some(1),
        "the host profile must reject a qualified std.amiga.dos reference; stderr:\n{}",
        String::from_utf8_lossy(&host.stderr)
    );
    let stderr = String::from_utf8_lossy(&host.stderr);
    assert!(
        stderr.lines().any(|line| line.starts_with("error E11200: ")
            && line.contains("std.amiga.dos")
            && line.contains("profile 'host'")),
        "expected an E11200 naming std.amiga.dos, got:\n{stderr}"
    );

    let (amiga, emitted) = run_source_emitting_hosted_amiga("qualified-amiga-hosted", source);
    assert_eq!(
        amiga.status.code(),
        Some(0),
        "the hosted-amiga profile must accept the reference; stderr:\n{}",
        String::from_utf8_lossy(&amiga.stderr)
    );
    assert!(
        !emitted.trim().is_empty(),
        "the hosted-amiga program must emit C"
    );
}

/// A user module whose name starts with `std.amiga.` is not a profile error.
///
/// The profile check keys on the module actually being an embedded library
/// module, so a user module that borrows the prefix resolves like any other.
#[test]
fn a_user_module_with_the_amiga_prefix_is_not_a_profile_error() {
    let files = [
        (
            "main.clo",
            "module spec.stage.user_amiga;\n\nimport { helper } from std.amiga.custom;\n\nfunc main(): i32 {\n    return helper();\n}\n",
        ),
        (
            "custom.clo",
            "module std.amiga.custom;\n\nexport func helper(): i32 {\n    return 7;\n}\n",
        ),
    ];
    let (run, emitted) = run_sources_emitting("user-amiga", &files);
    assert_eq!(
        run.status.code(),
        Some(0),
        "a user module named std.amiga.custom must not be a profile error; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!emitted.trim().is_empty(), "the program must emit C");
}

/// Independent errors are all reported, and the error cap trims the report to
/// the requested count.
#[test]
fn independent_errors_are_reported_up_to_the_cap() {
    let source = r#"module spec.cli.two_errors;

func demo(): i32 {
    let a: i32 = missingOne;
    let b: i32 = missingTwo;
    return a + b;
}

func main(): i32 {
    return 0;
}
"#;
    let all = run_source_with_flags("cli-errors", source, &[]);
    assert_eq!(all.status.code(), Some(1), "a failing compile exits 1");
    assert_eq!(
        error_head_count(&all.stderr),
        2,
        "both independent errors are reported"
    );

    let capped = run_source_with_flags("cli-errors-cap", source, &["--max-errors=1"]);
    assert_eq!(capped.status.code(), Some(1), "a capped failing compile exits 1");
    assert_eq!(
        error_head_count(&capped.stderr),
        1,
        "--max-errors=1 keeps exactly one error"
    );
}

/// A diagnostic renders its fixed head line first and its `  --> path:line:col`
/// location on the next line.
#[test]
fn diagnostic_head_and_location_render_on_fixed_lines() {
    let source = r#"module spec.cli.render;

func demo(): i32 {
    let a: i32 = missingOne;
    return a;
}

func main(): i32 {
    return 0;
}
"#;
    let run = run_source_with_flags("cli-render", source, &[]);
    let stderr = String::from_utf8_lossy(&run.stderr);
    let mut lines = stderr.lines();
    assert_eq!(
        lines.next(),
        Some("error E10219: 'missingOne' is not declared"),
        "the first line is `error CODE: message`"
    );
    let location = lines.next().unwrap_or_default();
    assert!(
        location.starts_with("  --> ") && location.ends_with(":4:18"),
        "the second line is `  --> path:line:col`, got {location:?}"
    );
    assert_eq!(run.status.code(), Some(1), "a failing compile exits 1");
}

/// The standard-library slice compiles with zero diagnostics under the host
/// profile.
///
/// The library has no command-line mode of its own, so "compiles the slice
/// alone" is observed through the smallest program that imports a declaration
/// from each host module and uses it. The host profile provides `std.mem`,
/// `std.text`, `std.fmt`, and `std.io`; the two Amiga modules are rejected on
/// this profile. Zero diagnostics includes warnings: a warning here would mean
/// the library is not clean. `Buffer` is a move-only handle, so lending it to a
/// `borrow` parameter is required rather than needless.
#[test]
fn stdlib_slice_compiles_with_no_diagnostics_under_host() {
    let source = r#"module spec.stage.stdlib_host;

import { Buffer, bufferNew, bufferBytes, bufferLen } from std.mem;
import { String, stringOf } from std.text;
import { toHex } from std.fmt;
import { print, println } from std.io;

func loadTag(borrow buf: Buffer): void {
    let bytes: u8[] = bufferBytes(buf);
    bytes[0] = 65;
}

func main(): i32 {
    let buf: Buffer? = bufferNew(4);
    if (buf == null) {
        return 1;
    }
    loadTag(buf);
    let s: String = stringOf("ok");
    print(toHex(u32(bufferLen(buf))));
    println(s);
    return 0;
}
"#;
    assert_no_diagnostics("stdlib-host", run_source("stdlib-host", source));
}

/// The standard-library slice compiles with zero diagnostics under the
/// hosted-amiga profile.
///
/// This profile provides all six modules, so the program adds `std.amiga.dos`
/// and `std.amiga.interrupt` to the host set and uses them: it waits one tick
/// and registers then removes a capture-less handler. Every imported name is
/// used, so an unused-import warning would fail the case.
#[test]
fn stdlib_slice_compiles_with_no_diagnostics_under_hosted_amiga() {
    let source = r#"module spec.stage.stdlib_hosted_amiga;

import { Buffer, bufferNew, bufferBytes, bufferLen } from std.mem;
import { String, stringOf } from std.text;
import { toHex } from std.fmt;
import { print, println } from std.io;
import { delayTicks } from std.amiga.dos;
import { HandlerToken, addHandler, removeHandler } from std.amiga.interrupt;

func loadTag(borrow buf: Buffer): void {
    let bytes: u8[] = bufferBytes(buf);
    bytes[0] = 65;
}

func onTick(): void {
}

func main(): i32 {
    let buf: Buffer? = bufferNew(4);
    if (buf == null) {
        return 1;
    }
    loadTag(buf);
    let s: String = stringOf("ok");
    print(toHex(u32(bufferLen(buf))));
    println(s);
    delayTicks(0);
    let token: HandlerToken = addHandler(onTick);
    removeHandler(token);
    return 0;
}
"#;
    let (run, _emitted) = run_source_emitting_hosted_amiga("stdlib-hosted", source);
    assert_no_diagnostics("stdlib-hosted", run);
}

/// A non-duplicable element type in an `Array` instantiation is rejected.
///
/// The growable sequence accepts duplicable element types only, because v1
/// cannot generate the drop glue a move-only element would need. Instantiating
/// it with the move-only `Buffer` handle is therefore a staged-feature error
/// rather than a type error.
#[test]
fn a_non_duplicable_array_element_reports_its_code() {
    let source = r#"module spec.stage.stdlib_array_buffer;

import { Buffer, Array, arrayNew } from std.mem;

func main(): i32 {
    let xs: Array<Buffer> = arrayNew<Buffer>();
    return 0;
}
"#;
    assert_reports_code("stdlib-array-buffer", source, "E90000");
}

/// A local that is declared and never used reports W10000.
#[test]
fn an_unused_local_reports_a_warning() {
    let source = r#"module spec.stage.warn_unused_local;

func main(): i32 {
    let unused = 42;
    return 0;
}
"#;
    assert_reports_warning(
        "warn-unused-local",
        source,
        "warning W10000: unused local 'unused'",
    );
}

/// A writable lend of a duplicable value reports W10001.
///
/// A duplicable value is passed by copy, so a `borrow` parameter would let the
/// callee write only the copy. The case pins the rendered first line.
#[test]
fn a_needless_lend_reports_a_warning() {
    let source = r#"module spec.stage.warn_needless_lend;

struct Point {
    x: i32;
    y: i32;
}

func bump(borrow p: Point): void {
    p.x = p.x + 1;
}

func main(): i32 {
    let p = Point { x: 1, y: 2 };
    bump(p);
    return p.x;
}
"#;
    assert_reports_warning(
        "warn-needless-lend",
        source,
        "warning W10001: unnecessary lend of duplicable 'p'",
    );
}

/// A `switch` case body that falls through to the next label reports W10002.
#[test]
fn a_switch_fallthrough_reports_a_warning() {
    let source = r#"module spec.stage.warn_fallthrough;

func start(): void { }
func run(): void { }

func main(): i32 {
    let state = 0;
    switch (state) {
        case 0:
            start();
        case 1:
            run();
            break;
        default:
            break;
    }
    return 0;
}
"#;
    assert_reports_warning(
        "warn-fallthrough",
        source,
        "warning W10002: case falls through to the next label",
    );
}

/// A statement after an unconditional return reports W10003.
#[test]
fn unreachable_code_reports_a_warning() {
    let source = r#"module spec.stage.warn_unreachable;

func tail(): void { }

func f(): i32 {
    return 1;
    tail();
}

func main(): i32 {
    return f();
}
"#;
    assert_reports_warning(
        "warn-unreachable",
        source,
        "warning W10003: unreachable code",
    );
}

/// The large-frame warning honors each profile's threshold.
///
/// A frame below the host limit draws no warning. A frame above the host limit
/// draws W10004, and the same frame also draws W10004 under hosted-amiga, whose
/// limit is smaller. The first line names the function and its frame size in
/// bytes, so both warned cases pin the exact size.
#[test]
fn a_large_stack_frame_reports_a_warning_at_each_profile_threshold() {
    let host_below = r#"module spec.stage.warn_frame_host_below;

func small(): void {
    let page: u8[4096] = [; 0];
    page[0] = 1;
}

func main(): i32 {
    small();
    return 0;
}
"#;
    assert_omits_warning(
        "warn-frame-host-below",
        run_source("warn-frame-host-below", host_below),
        "W10004",
    );

    let host_above = r#"module spec.stage.warn_frame_host_above;

func large(): void {
    let page: u8[20000] = [; 0];
    page[0] = 1;
}

func main(): i32 {
    large();
    return 0;
}
"#;
    assert_reports_warning(
        "warn-frame-host-above",
        host_above,
        "warning W10004: stack frame of 'large' is 20000 bytes",
    );

    let hosted = r#"module spec.stage.warn_frame_hosted;

func large(): void {
    let page: u8[4096] = [; 0];
    page[0] = 1;
}

func main(): i32 {
    large();
    return 0;
}
"#;
    let (run, _emitted) = run_source_emitting_hosted_amiga("warn-frame-hosted", hosted);
    assert_warning_first_line(
        "warn-frame-hosted",
        run,
        "warning W10004: stack frame of 'large' is 4096 bytes",
    );
}

/// An imported name that is never used reports W10006.
#[test]
fn an_unused_import_reports_a_warning() {
    let source = r#"module spec.stage.warn_unused_import;

import { Buffer } from std.mem;

func main(): i32 {
    return 0;
}
"#;
    assert_reports_warning(
        "warn-unused-import",
        source,
        "warning W10006: unused import 'Buffer'",
    );
}
