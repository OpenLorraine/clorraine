//! Implementation tests for the reporting shape of the borrow-view move rule.
//!
//! The specification suite proves that a move through a `for (borrow e in xs)`
//! view is refused with the element-move diagnostic and that a move out of a
//! `borrow` parameter keeps its own diagnostic. These cases pin the reporting
//! shape: one transfer produces one report, and the parameter rule reports its
//! own code rather than the element-move code. A duplicate report would give
//! the user two conflicting remedies for one transfer, so the count is part of
//! the contract.
//!
//! Every case here is a compile-time rejection, so the helper only compiles and
//! reads the diagnostics; no C is built or run.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

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
            "clc-bvsimp-{}-{tag}-{sequence}",
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

/// Compiles `source` in the host profile and returns the compiler output.
///
/// The scratch directory travels with the output so the caller keeps the
/// program file alive while it reads the diagnostics.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    (scratch, output)
}

/// Counts the reports of one error code in the compiler's stderr.
fn count_reports(stderr: &str, code: &str) -> usize {
    stderr.matches(&format!("error {code}:")).count()
}

/// One transfer through a view reports the element-move limit exactly once.
///
/// The `take` call is the only move in the program, so a second report would
/// mean the same transfer is diagnosed twice.
#[test]
fn should_report_a_single_element_move_when_a_take_argument_reads_a_borrow_view() {
    let source = r#"module codegen.bvs.imp1;

import { String, stringOf } from std.text;

func eat(take s: String): void { }

func main(): i32 {
    let xs: String[2] = [stringOf("a"), stringOf("bb")];
    for (borrow e in xs) {
        eat(e);
    }
    return 0;
}
"#;
    let (_scratch, output) = compile_only("bvsimp1", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "bvsimp1: the front end accepted a move through a borrow view"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_reports(&stderr, "E10026"),
        1,
        "bvsimp1: expected exactly one E10026 report; stderr:\n{stderr}"
    );
}

/// A field path out of a view also reports the element-move limit once.
///
/// The projected path is a different route into the move rule than a bare
/// name, and it must not add a report on top of the element-move one.
#[test]
fn should_report_a_single_element_move_when_a_field_path_leaves_a_borrow_view() {
    let source = r#"module codegen.bvs.imp2;

import { String, stringOf } from std.text;

struct P { name: String; n: i32; }

func main(): i32 {
    let ps: P[2] = [P { name: stringOf("a"), n: 1 }, P { name: stringOf("b"), n: 2 }];
    for (borrow p in ps) {
        let s: String = p.name;
        if (len(s) == 0) { return 1; }
    }
    return 0;
}
"#;
    let (_scratch, output) = compile_only("bvsimp2", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "bvsimp2: the front end accepted a field move through a borrow view"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_reports(&stderr, "E10026"),
        1,
        "bvsimp2: expected exactly one E10026 report; stderr:\n{stderr}"
    );
}

/// A move out of a `borrow` parameter keeps the parameter-specific report.
///
/// The view guard must not swallow the parameter rule: this program still
/// reports the borrow-move diagnostic and never the element-move one.
#[test]
fn should_keep_the_borrow_parameter_move_when_a_take_argument_reads_a_borrow_parameter() {
    let source = r#"module codegen.bvs.imp3;

import { String, stringOf } from std.text;

func eat(take s: String): void { }

func demo(borrow s: String): void {
    eat(s);
}

func main(): i32 { return 0; }
"#;
    let (_scratch, output) = compile_only("bvsimp3", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "bvsimp3: the front end accepted a move out of a borrow parameter"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_reports(&stderr, "E10034"),
        1,
        "bvsimp3: expected exactly one E10034 report; stderr:\n{stderr}"
    );
    assert_eq!(
        count_reports(&stderr, "E10026"),
        0,
        "bvsimp3: the parameter rule must not report the element-move code; stderr:\n{stderr}"
    );
}
