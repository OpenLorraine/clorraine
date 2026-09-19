//! Command-line specification tests for front-end stage behavior.
//!
//! The golden suites state one expected diagnostic per file; these cases cover
//! the lexical and parser forms the specification calls out explicitly (the
//! literal and template spellings, the statement-level disambiguations, and
//! the diagnostic-only declaration starts) plus the name and module rules that
//! no single golden states on its own: shadowing, declaration order, merged
//! modules, and import bindings. Every case drives the built `clc` binary, so
//! the published command-line behavior is the oracle.

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

func identity<T>(x: T): T {
    return x;
}

func shapes(a: i32, b: i32, c: i32): i32 {
    let comparison = (a < b) > (c);
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
