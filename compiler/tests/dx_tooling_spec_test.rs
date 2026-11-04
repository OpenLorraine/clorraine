//! End-to-end specification tests for the developer-experience tooling.
//!
//! Every case spawns a built binary in a private scratch directory and asserts
//! the observable contract: stdout, stderr, file effects, and the exit code.
//! The cases derive from the requirements and the resolved design decisions;
//! they are the immutable oracle for the implementation.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

mod dx_support;

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
            "clorraine-dx-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
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

/// Writes a source fixture into the scratch directory and returns its path.
fn write_fixture(scratch: &ScratchDir, name: &str, source: &str) -> PathBuf {
    let path = scratch.path().join(name);
    std::fs::write(&path, source)
        .unwrap_or_else(|error| panic!("cannot write fixture {name}: {error}"));
    path
}

/// Runs a binary with `args`, using `cwd` as its working directory.
fn run_binary(binary: &str, cwd: &Path, args: &[&str]) -> Output {
    Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|error| panic!("failed to spawn {binary}: {error}"))
}

/// Runs the built `clc` compiler.
fn run_clc(cwd: &Path, args: &[&str]) -> Output {
    run_binary(env!("CARGO_BIN_EXE_clc"), cwd, args)
}

/// Runs the built `clorraine` driver.
fn run_clorraine(cwd: &Path, args: &[&str]) -> Output {
    run_binary(env!("CARGO_BIN_EXE_clorraine"), cwd, args)
}

/// Runs the built `clorraine` driver with an explicit `PATH` value.
fn run_clorraine_with_path(cwd: &Path, args: &[&str], path: &OsStr) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clorraine"))
        .args(args)
        .current_dir(cwd)
        .env("PATH", path)
        .output()
        .expect("failed to spawn the clorraine driver")
}

/// Marks a file executable so a test can stand in for a real tool.
#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)
        .expect("the tool file exists")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("set the execute bit");
}

/// The captured standard output as text.
fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The captured standard error as text.
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Asserts `output` exited with `expected`, showing both streams on mismatch.
fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: expected exit {expected}; stdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

/// The exact registry entry rendered by `--explain E10034`.
const EXPLAIN_E10034: &str = "error E10034: cannot release or store borrowed parameter '{name}'\n  trigger: Borrowed parameter released or stored by the callee\n  section: Ownership, moves, initialization, nullability, borrows (E100xx)\n  help: declare the parameter `take` or by value; a `borrow T?` slot may hold its own incoming value, a static view, or a newly owned value\n";

/// The exact registry entry rendered by `--explain W10000`.
const EXPLAIN_W10000: &str = "warning W10000: unused local '{name}'\n  trigger: Local declared and never used\n  section: Warnings (W100xx)\n  help: remove the binding or use it\n";

// `clc --explain E10034` renders the full registry entry.
#[test]
fn explain_renders_the_registry_entry_on_clc() {
    let scratch = ScratchDir::new("st1");
    let output = run_clc(scratch.path(), &["--explain", "E10034"]);
    assert_exit("explain on clc", &output, 0);
    assert_eq!(stdout(&output), EXPLAIN_E10034);
    assert_eq!(stderr(&output), "");
}

// `clorraine --explain W10000` renders a warning entry.
#[test]
fn explain_renders_a_warning_entry_on_clorraine() {
    let scratch = ScratchDir::new("st2");
    let output = run_clorraine(scratch.path(), &["--explain", "W10000"]);
    assert_exit("explain on clorraine", &output, 0);
    assert_eq!(stdout(&output), EXPLAIN_W10000);
    assert_eq!(stderr(&output), "");
}

// the joined `--explain=<CODE>` spelling is identical.
#[test]
fn explain_accepts_the_joined_spelling() {
    let scratch = ScratchDir::new("st3");
    let output = run_clc(scratch.path(), &["--explain=E10034"]);
    assert_exit("joined explain", &output, 0);
    assert_eq!(stdout(&output), EXPLAIN_E10034);
    assert_eq!(stderr(&output), "");
}

// an unknown code is a usage error.
#[test]
fn explain_rejects_an_unknown_code() {
    let scratch = ScratchDir::new("st4");
    let output = run_clc(scratch.path(), &["--explain", "NOPE"]);
    assert_exit("unknown code", &output, 2);
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "usage error: unknown diagnostic code 'NOPE'\n");
}

// a missing value is a usage error.
#[test]
fn explain_rejects_a_missing_value() {
    let scratch = ScratchDir::new("st5");
    let output = run_clc(scratch.path(), &["--explain"]);
    assert_exit("missing value", &output, 2);
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "usage error: --explain needs a value\n");
}

// explain is honoured wherever it appears on the driver.
#[test]
fn explain_is_honoured_without_or_after_a_command() {
    let scratch = ScratchDir::new("st6");
    for args in [&["--explain", "E10034"][..], &["run", "--explain", "E10034"][..]] {
        let output = run_clorraine(scratch.path(), args);
        assert_exit("explain anywhere", &output, 0);
        assert_eq!(stdout(&output), EXPLAIN_E10034);
        assert_eq!(stderr(&output), "");
    }
}

// valid extra arguments are ignored, so explain needs no profile or file.
#[test]
fn explain_wins_over_other_valid_arguments() {
    let scratch = ScratchDir::new("st7");
    // The file is deliberately invalid: explain must never read it.
    write_fixture(&scratch, "main.clo", "this is not a program\n");
    let output = run_clc(
        scratch.path(),
        &["--explain", "E10034", "main.clo", "--profile", "host"],
    );
    assert_exit("extra arguments", &output, 0);
    assert_eq!(stdout(&output), EXPLAIN_E10034);
    assert_eq!(stderr(&output), "");
}

/// A source whose `null` initializer is used, so E10012 reports on the `let`.
const BAD_SOURCE: &str = "module tests.dx.bad;\n\nfunc demo(): i32 {\n    let x: i32 = null;\n    return x;\n}\n\nfunc main(): i32 {\n    return 0;\n}\n";

/// A source with one unused local and a valid `main`.
const UNUSED_SOURCE: &str = "module tests.dx.unused;\n\nfunc main(): i32 {\n    let unused: i32 = 1;\n    return 0;\n}\n";

/// A minimal valid program that reaches the C compiler.
const GOOD_SOURCE: &str = "module tests.dx.good;\n\nfunc main(): i32 {\n    return 0;\n}\n";

/// The exact JSON line for the `E10012` fixture.
const JSON_E10012: &str = "{\"code\":\"E10012\",\"severity\":\"error\",\"message\":\"'null' is not allowed here\",\"help\":\"declare the type `T?` or avoid the empty state\",\"file\":\"bad.clo\",\"line\":4,\"column\":5,\"endLine\":4,\"endColumn\":22}\n";

/// The exact JSON line for the `W10000` fixture.
///
/// The compiler's warning carries no help line (the human render has none), so
/// JSON reports `help:null`; the registry help is reachable through
/// `--explain W10000`.
const JSON_W10000: &str = "{\"code\":\"W10000\",\"severity\":\"warning\",\"message\":\"unused local 'unused'\",\"help\":null,\"file\":\"unused.clo\",\"line\":4,\"column\":9,\"endLine\":4,\"endColumn\":15}\n";

/// The exact null-code JSON line for a usage error with no input files.
const JSON_NO_FILES: &str = "{\"code\":null,\"severity\":\"error\",\"message\":\"usage error: at least one input file is required\",\"help\":null,\"file\":null,\"line\":null,\"column\":null,\"endLine\":null,\"endColumn\":null}\n";

// a compile error becomes one JSON object on stderr.
#[test]
fn json_renders_a_compile_error() {
    let scratch = ScratchDir::new("st8");
    write_fixture(&scratch, "bad.clo", BAD_SOURCE);
    let output = run_clc(
        scratch.path(),
        &[
            "--diagnostics=json",
            "--profile",
            "host",
            "-o",
            "out.c",
            "bad.clo",
        ],
    );
    assert_exit("json compile error", &output, 1);
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), JSON_E10012);
}

// a warning becomes one JSON object and does not fail the run.
#[test]
fn json_renders_a_warning() {
    let scratch = ScratchDir::new("st9");
    write_fixture(&scratch, "unused.clo", UNUSED_SOURCE);
    let output = run_clc(
        scratch.path(),
        &[
            "--diagnostics=json",
            "--profile",
            "host",
            "-o",
            "out.c",
            "unused.clo",
        ],
    );
    assert_exit("json warning", &output, 0);
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), JSON_W10000);
}

// a usage error is machine-readable on both binaries.
#[test]
fn json_renders_a_usage_error() {
    let scratch = ScratchDir::new("st10");
    let clc = run_clc(scratch.path(), &["--diagnostics=json", "--profile", "host"]);
    assert_exit("json usage error on clc", &clc, 2);
    assert_eq!(stdout(&clc), "");
    assert_eq!(stderr(&clc), JSON_NO_FILES);

    let driver = run_clorraine(scratch.path(), &["run", "--diagnostics=json"]);
    assert_exit("json usage error on clorraine", &driver, 2);
    assert_eq!(stdout(&driver), "");
    assert_eq!(stderr(&driver), JSON_NO_FILES);
}

// quotes and newlines in an echoed argument stay on one JSON line.
#[test]
fn json_escapes_argument_content() {
    let scratch = ScratchDir::new("st11");
    let quoted = run_clc(
        scratch.path(),
        &[
            "--diagnostics=json",
            "--profile",
            "host",
            "--max-errors",
            "3\"z",
        ],
    );
    assert_exit("json quote escaping", &quoted, 2);
    assert_eq!(
        stderr(&quoted),
        "{\"code\":null,\"severity\":\"error\",\"message\":\"usage error: --max-errors expects a number, got '3\\\"z'\",\"help\":null,\"file\":null,\"line\":null,\"column\":null,\"endLine\":null,\"endColumn\":null}\n"
    );

    let newline = run_clc(
        scratch.path(),
        &["--diagnostics=json", "--profile", "host", "--nope\nx"],
    );
    assert_exit("json newline escaping", &newline, 2);
    assert_eq!(
        stderr(&newline),
        "{\"code\":null,\"severity\":\"error\",\"message\":\"usage error: unknown option '--nope\\nx'\",\"help\":null,\"file\":null,\"line\":null,\"column\":null,\"endLine\":null,\"endColumn\":null}\n"
    );
    assert_eq!(stderr(&newline).lines().count(), 1);
}

// without the flag, the human output is unchanged.
#[test]
fn human_output_is_unchanged() {
    let scratch = ScratchDir::new("st12");
    write_fixture(&scratch, "bad.clo", BAD_SOURCE);
    let output = run_clc(
        scratch.path(),
        &["--profile", "host", "-o", "out.c", "bad.clo"],
    );
    assert_exit("human output", &output, 1);
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "error E10012: 'null' is not allowed here\n  --> bad.clo:4:5\n  help: declare the type `T?` or avoid the empty state\n"
    );
}

// an unknown format value is a human usage error.
#[test]
fn an_unknown_diagnostics_value_is_a_human_usage_error() {
    let scratch = ScratchDir::new("st13");
    write_fixture(&scratch, "bad.clo", BAD_SOURCE);
    let output = run_clc(
        scratch.path(),
        &["--diagnostics=xml", "--profile", "host", "bad.clo"],
    );
    assert_exit("unknown format", &output, 2);
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "usage error: unknown diagnostics format 'xml' (expected json)\n"
    );
}

// deny-warnings keeps the warning severity and fails the run.
#[test]
fn json_keeps_warning_severity_under_deny_warnings() {
    let scratch = ScratchDir::new("st14");
    write_fixture(&scratch, "unused.clo", UNUSED_SOURCE);
    let output = run_clc(
        scratch.path(),
        &[
            "--diagnostics=json",
            "--deny-warnings",
            "--profile",
            "host",
            "-o",
            "out.c",
            "unused.clo",
        ],
    );
    assert_exit("json deny warnings", &output, 1);
    assert_eq!(stderr(&output), JSON_W10000);
}

// the driver forwards the format to the in-process compiler.
#[test]
fn the_driver_forwards_json_to_the_compiler() {
    let scratch = ScratchDir::new("st15");
    write_fixture(&scratch, "bad.clo", BAD_SOURCE);
    let output = run_clorraine(
        scratch.path(),
        &["build", "--diagnostics=json", "bad.clo"],
    );
    assert_exit("json forwarding", &output, 1);
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), JSON_E10012);
}

// a missing environment tool is one machine-readable error.
#[test]
fn json_renders_an_environment_error() {
    let scratch = ScratchDir::new("st20");
    write_fixture(&scratch, "good.clo", GOOD_SOURCE);
    let output = run_clorraine_with_path(
        scratch.path(),
        &["build", "--diagnostics=json", "good.clo"],
        OsStr::new(""),
    );
    assert_exit("json environment error", &output, 1);
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "{\"code\":null,\"severity\":\"error\",\"message\":\"clorraine: required tool 'cc' was not found on PATH\\ninstall the SDK toolchain or run inside the container (make -C docker shell)\",\"help\":null,\"file\":null,\"line\":null,\"column\":null,\"endLine\":null,\"endColumn\":null}\n"
    );
}

/// The repository root, one level above the compiler package.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Reads the canonical grammar JSON.
fn canonical_grammar() -> String {
    let path = repo_root().join("editors/clorraine.tmLanguage.json");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

// every EBNF keyword and reserved built-in appears in the grammar.
#[test]
fn grammar_covers_every_ebnf_term() {
    let grammar = std::fs::read_to_string(repo_root().join("spec/grammar.ebnf.md"))
        .expect("the EBNF is readable");
    let patterns = dx_support::pattern_strings(&canonical_grammar());
    for (production, expected) in [
        ("keyword", 50_usize),
        ("contextual_keyword", 1),
        ("reserved_builtin", 12),
    ] {
        let terms = dx_support::production_terms(&grammar, production);
        assert_eq!(terms.len(), expected, "{production} term count");
        for term in terms {
            assert!(
                patterns
                    .iter()
                    .any(|pattern| dx_support::word_bounded(pattern, term)),
                "{production} term '{term}' is missing from the grammar"
            );
        }
    }
}

// the canonical path exists, the old copy is gone, and the demo uses it.
#[test]
fn the_canonical_grammar_path_is_used() {
    let root = repo_root();
    assert!(
        root.join("editors/clorraine.tmLanguage.json").is_file(),
        "the canonical grammar exists"
    );
    assert!(
        !root
            .join("tools/demo/grammars/clorraine.tmLanguage.json")
            .exists(),
        "the demo-owned copy is removed"
    );
    let shiki = std::fs::read_to_string(root.join("tools/demo/src/util/shiki.ts"))
        .expect("the demo highlighter is readable");
    assert!(shiki.contains("editors"), "shiki.ts points at editors/");
    assert!(shiki.contains("clorraine.tmLanguage.json"));
}

// the grammar keeps its language metadata.
#[test]
fn the_grammar_keeps_its_metadata() {
    let strings = dx_support::json_strings(&canonical_grammar());
    assert!(
        strings.iter().any(|value| value == "source.clorraine"),
        "scopeName is source.clorraine"
    );
    assert!(
        strings.iter().any(|value| value == "clo"),
        "fileTypes lists clo"
    );
}

// the v1 surfaces are scoped and the stale spelling is gone.
#[test]
fn the_grammar_covers_the_v1_surfaces() {
    let patterns = dx_support::pattern_strings(&canonical_grammar());
    assert!(
        patterns.iter().any(|pattern| pattern.contains("=>")),
        "the arrow operator is scoped"
    );
    assert!(
        patterns
            .iter()
            .any(|pattern| dx_support::word_bounded(pattern, "freedBy")),
        "freedBy is scoped"
    );
    assert!(
        !patterns
            .iter()
            .any(|pattern| dx_support::word_bounded(pattern, "deleter")),
        "the stale deleter spelling is absent"
    );
}

// a multi-line internal compiler error is escaped onto one JSON line.
#[test]
fn json_escapes_a_multiline_internal_compiler_error() {
    let scratch = ScratchDir::new("st21");
    write_fixture(&scratch, "good.clo", GOOD_SOURCE);
    let bin = scratch.path().join("bin");
    std::fs::create_dir_all(&bin).expect("create the tool directory");
    let cc = bin.join("cc");
    std::fs::write(&cc, "#!/bin/sh\necho 'fake compiler error' >&2\nexit 1\n")
        .expect("write the stand-in compiler");
    #[cfg(unix)]
    make_executable(&cc);

    let output = run_clorraine_with_path(
        scratch.path(),
        &["build", "--diagnostics=json", "good.clo"],
        bin.as_os_str(),
    );
    assert_exit("json internal compiler error", &output, 1);
    assert_eq!(stdout(&output), "");
    let text = stderr(&output);
    assert_eq!(text.lines().count(), 1, "the ICE must be one JSON line:\n{text}");
    assert!(
        text.starts_with(
            "{\"code\":null,\"severity\":\"error\",\"message\":\"error: internal compiler error: the emitted C failed to compile\\n  note: emitted C kept at "
        ),
        "unexpected ICE prefix:\n{text}"
    );
    assert!(
        text.contains("\\n  first error: fake compiler error\""),
        "the first C error must be escaped inside the message:\n{text}"
    );
    assert!(text.contains("\"help\":null,\"file\":null"));
}
