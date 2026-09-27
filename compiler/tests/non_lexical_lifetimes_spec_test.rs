//! Specification tests for non-lexical lifetimes.
//!
//! The v2 borrow model makes a loan live from its creation to its last use on
//! any path, not to the end of the scope that owns the derived view
//! (`spec/16-borrow-rules.md` section 8). Every case compiles a small Clorraine
//! program with the built `clc` binary and asserts the behavior the frozen
//! specification requires: a move, a mutation, or a new exclusive lend is
//! rejected exactly when a use of the loan is still reachable, and accepted
//! when the last use precedes it. The cases drive the published command-line
//! behavior, so a regression is visible as a wrong diagnostic or an unexpected
//! acceptance.
//!
//! The accepted programs are built and run by
//! `non_lexical_lifetimes_impl_test.rs`; these cases stop at the front end.

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
            "clc-nll-{}-{tag}-{sequence}",
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

/// Compiles one source snippet for the host profile and returns the output.
fn run_source(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("cannot write snippet: {err}"));
    let output_path = scratch.path().join("out.c");
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            output_path.display().to_string(),
            source_path.display().to_string(),
        ])
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Asserts that the compiler accepts a snippet and emits C.
fn assert_accepts(tag: &str, source: &str) {
    let run = run_source(tag, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: expected acceptance; stderr:\n{stderr}"
    );
    assert!(
        !stderr.lines().any(|line| line.starts_with("error ")),
        "{tag}: unexpected error diagnostic; stderr:\n{stderr}"
    );
}

/// Asserts that compiling one snippet reports each expected line exactly the
/// given number of times, and fails with exit code 1.
///
/// The exact count is the point: a loan conflict must be reported once even
/// when several accesses are reachable from it.
fn assert_reports_counted(tag: &str, source: &str, expected: &[(&str, usize)]) {
    let run = run_source(tag, source);
    let stderr = String::from_utf8_lossy(&run.stderr);
    for (line, count) in expected {
        let seen = stderr.lines().filter(|candidate| candidate == line).count();
        assert_eq!(
            seen, *count,
            "{tag}: expected {count} occurrence(s) of `{line}`, saw {seen}; stderr:\n{stderr}"
        );
    }
    assert_eq!(run.status.code(), Some(1), "{tag}: failing compiles exit 1");
}

// -- Accepted shapes -------------------------------------------------------

/// `spec/16-borrow-rules.md` section 7: a move after the view's last use is
/// accepted because the loan's region ends at that use.
#[test]
fn should_accept_a_move_after_the_views_last_use() {
    let source = r#"module tests.nll.move_after_last_use;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(): void {
    let s: String = stringOf("x");
    let v: u8[] = bytes(s);
    observe(v);
    sink(s);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("st1", source);
}

/// `spec/16-borrow-rules.md` section 2: an implicit `String` -> `str` widening
/// is a loan whose region ends at the view's last use, so a later mutation of
/// the owner is accepted.
#[test]
fn should_accept_a_mutation_after_a_widened_views_last_use() {
    let source = r#"module tests.nll.widening_last_use;

import { String, stringOf } from std.text;

func demo(): void {
    let s: String = stringOf("x");
    let v: str = s;
    if (len(v) == 0) {
        return;
    }
    s = stringOf("y");
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("st2", source);
}

/// `spec/16-borrow-rules.md` section 7: an exclusive lend after the shared
/// loan's last use is accepted; the "unused later" gap is exactly what the
/// last-use regions close.
#[test]
fn should_accept_an_exclusive_lend_after_a_shared_loans_last_use() {
    let source = r#"module tests.nll.exclusive_after_last_use;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}
func reset(borrow s: String): void {}
func viewOf(s: str): str { return s; }

func demo(): void {
    let s: String = stringOf("x");
    let v: str = viewOf(s);
    observeStr(v);
    reset(s);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("st3", source);
}

/// `spec/16-borrow-rules.md` section 7: after a branch that only uses the view
/// on one path, no use is reachable past the branch, so the move is accepted.
#[test]
fn should_accept_a_move_after_a_branch_only_use() {
    let source = r#"module tests.nll.branch_only_use;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(c: bool): void {
    let s: String = stringOf("x");
    let v: u8[] = bytes(s);
    if (c) {
        observe(v);
    }
    sink(s);
}

func main(): i32 { demo(true); return 0; }
"#;
    assert_accepts("st8", source);
}

/// `spec/16-borrow-rules.md` section 7: a view used inside a shorter scope is
/// accepted as long as every use precedes the owner's release.
#[test]
fn should_accept_a_widened_view_used_inside_a_shorter_scope() {
    let source = r#"module tests.nll.view_inside_shorter_scope;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let held: str;
    {
        let s: String = stringOf("x");
        held = s;
        observeStr(held);
    }
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("st10", source);
}

/// `spec/16-borrow-rules.md` section 1: copies of a view share one loan, and
/// the loan's region ends after both alias uses, so the move is accepted.
#[test]
fn should_accept_a_move_after_every_alias_use() {
    let source = r#"module tests.nll.alias_uses_before_move;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(): void {
    let s: String = stringOf("x");
    let a: u8[] = bytes(s);
    let b: u8[] = a;
    observe(a);
    observe(b);
    sink(s);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("st12", source);
}

/// `spec/16-borrow-rules.md` section 7: events are ordered by evaluation, so
/// the read of the view on the right-hand side happens before the store; the
/// store is accepted.
#[test]
fn should_accept_a_rebind_that_reads_the_view_in_the_same_statement() {
    let source = r#"module tests.nll.rebind_reads_view;

import { String, stringOf } from std.text;

func demo(): void {
    let s: String = stringOf("x");
    let v: str = s;
    s = stringOf(v);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("st14", source);
}

/// `spec/03-ownership.md`: a `defer` block runs on the scope's exit edge before
/// the owner's release, so a use inside it is accepted.
#[test]
fn should_accept_a_defer_that_uses_the_view_on_the_exit_edge() {
    let source = r#"module tests.nll.defer_use_on_exit;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let s: String = stringOf("x");
    let v: str = s;
    defer {
        observeStr(v);
    }
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("st16", source);
}

// -- Rejected shapes -------------------------------------------------------

/// `spec/14-diagnostics.md` E10022: moving the owner while a use of the view is
/// still reachable is rejected exactly once.
#[test]
fn should_report_e10022_once_when_a_use_follows_the_move() {
    let source = r#"module tests.nll.move_before_last_use;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(): void {
    let s: String = stringOf("x");
    let v: u8[] = bytes(s);
    sink(s);
    observe(v);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "st4",
        source,
        &[("error E10022: cannot move 's' while it is borrowed", 1)],
    );
}

/// `spec/14-diagnostics.md` E10031: mutating the owner while a use of the
/// shared loan is reachable is rejected exactly once.
#[test]
fn should_report_e10031_once_when_a_use_follows_the_mutation() {
    let source = r#"module tests.nll.mutation_before_last_use;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}

func demo(): void {
    let s: String = stringOf("x");
    let v: u8[] = bytes(s);
    s = stringOf("y");
    observe(v);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "st5",
        source,
        &[("error E10031: cannot modify 's' while it is shared-borrowed", 1)],
    );
}

/// `spec/14-diagnostics.md` E10032: a second exclusive lend while the first is
/// still live is rejected exactly once.
#[test]
fn should_report_e10032_once_when_a_second_exclusive_lend_is_live() {
    let source = r#"module tests.nll.second_exclusive_lend;

import { Buffer, bufferNew, bufferBytes } from std.mem;

func observe(v: u8[]): void {}

func demo(): void {
    let b: Buffer? = bufferNew(16);
    if (b == null) {
        return;
    }
    let v: u8[] = bufferBytes(b);
    let w: u8[] = bufferBytes(b);
    observe(v);
    observe(w);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "st6",
        source,
        &[("error E10032: 'b' is already borrowed", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 1: a use on the next loop iteration is
/// reachable through the back edge, so mutating the owner in the loop body is
/// E10031.
#[test]
fn should_report_e10031_when_a_loop_back_edge_reaches_the_use() {
    let source = r#"module tests.nll.loop_back_edge;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}

func demo(): void {
    let s: String = stringOf("x");
    let v: u8[] = bytes(s);
    let i: i32 = 0;
    while (i < 2) {
        observe(v);
        if (i == 0) {
            s = stringOf("y");
        }
        i = i + 1;
    }
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "st7",
        source,
        &[("error E10031: cannot modify 's' while it is shared-borrowed", 1)],
    );
}

/// `spec/14-diagnostics.md` E10022: a use after the branch is reachable from
/// the move taken on the other path, so the move is rejected.
#[test]
fn should_report_e10022_when_a_use_after_the_branch_is_reachable_from_the_move() {
    let source = r#"module tests.nll.branch_move_before_use;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(c: bool): void {
    let s: String = stringOf("x");
    let v: u8[] = bytes(s);
    if (c) {
        sink(s);
    }
    observe(v);
}

func main(): i32 { demo(true); return 0; }
"#;
    assert_reports_counted(
        "st9",
        source,
        &[("error E10022: cannot move 's' while it is borrowed", 1)],
    );
}

/// `spec/14-diagnostics.md` E10030: a use after the owner's scope release is
/// reported once at the loan's creation.
#[test]
fn should_report_e10030_when_a_use_follows_the_owners_scope() {
    let source = r#"module tests.nll.view_after_owner_scope;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let held: str;
    {
        let s: String = stringOf("x");
        held = s;
    }
    observeStr(held);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "st11",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 1: aliases share one loan, so a move that
/// one alias still uses after is E10022.
#[test]
fn should_report_e10022_when_an_alias_use_follows_the_move() {
    let source = r#"module tests.nll.alias_use_after_move;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(): void {
    let s: String = stringOf("x");
    let a: u8[] = bytes(s);
    let b: u8[] = a;
    sink(s);
    observe(b);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "st13",
        source,
        &[("error E10022: cannot move 's' while it is borrowed", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 7: the store is ordered after the read in
/// the same statement, so a later use of the view is E10031.
#[test]
fn should_report_e10031_when_the_view_is_used_after_a_same_statement_rebind() {
    let source = r#"module tests.nll.view_use_after_rebind;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let s: String = stringOf("x");
    let v: str = s;
    s = stringOf(v);
    observeStr(v);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "st15",
        source,
        &[("error E10031: cannot modify 's' while it is shared-borrowed", 1)],
    );
}
