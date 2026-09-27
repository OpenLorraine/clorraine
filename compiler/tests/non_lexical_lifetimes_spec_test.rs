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

/// Asserts that compiling one snippet reports exactly the expected errors, each
/// the given number of times, and fails with exit code 1.
///
/// The exact count is the point: a loan conflict must be reported once even
/// when several accesses are reachable from it. An error that is not in
/// `expected` fails the case, so a fix cannot pass by adding an unrelated
/// diagnostic. Warnings are not counted.
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
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error "))
        .collect();
    let expected_total: usize = expected.iter().map(|(_, count)| count).sum();
    for line in &errors {
        assert!(
            expected.iter().any(|(candidate, _)| candidate == line),
            "{tag}: unexpected diagnostic `{line}`; stderr:\n{stderr}"
        );
    }
    assert_eq!(
        errors.len(),
        expected_total,
        "{tag}: expected {expected_total} error(s), saw {}; stderr:\n{stderr}",
        errors.len()
    );
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

// -- Branch-assigned aliases and exit-edge precision -----------------------

/// `spec/16-borrow-rules.md` section 1: a view rebound on one branch still
/// aliases its first owner on the other path, so a move after the join is
/// rejected even though the other branch re-pointed the view.
#[test]
fn should_report_e10022_when_a_view_is_rebound_on_one_branch() {
    let source = r#"module tests.nll.branch_rebind_move;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(c: bool): void {
    let s: String = stringOf("x");
    let t: String = stringOf("y");
    let a: u8[] = bytes(s);
    if (c) {
        a = bytes(t);
    }
    sink(s);
    observe(a);
}

func main(): i32 { demo(true); return 0; }
"#;
    assert_reports_counted(
        "branch-rebind-move",
        source,
        &[("error E10022: cannot move 's' while it is borrowed", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 1: the mirror shape, where the other
/// branch carries the rebind, must reject the move the same way.
#[test]
fn should_report_e10022_when_the_other_branch_carries_the_rebind() {
    let source = r#"module tests.nll.branch_rebind_else_move;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(c: bool): void {
    let s: String = stringOf("x");
    let t: String = stringOf("y");
    let a: u8[] = bytes(s);
    if (c) {
    } else {
        a = bytes(t);
    }
    sink(s);
    observe(a);
}

func main(): i32 { demo(true); return 0; }
"#;
    assert_reports_counted(
        "branch-rebind-else-move",
        source,
        &[("error E10022: cannot move 's' while it is borrowed", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 1: a view reassigned on one branch still
/// aliases the first owner, so mutating that owner after the join is rejected.
#[test]
fn should_report_e10031_when_a_widened_view_is_rebound_on_one_branch() {
    let source = r#"module tests.nll.branch_rebind_write;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(c: bool): void {
    let s: String = stringOf("x");
    let t: String = stringOf("y");
    let a: str = s;
    if (c) {
        a = t;
    }
    s = stringOf("z");
    observeStr(a);
}

func main(): i32 { demo(true); return 0; }
"#;
    assert_reports_counted(
        "branch-rebind-write",
        source,
        &[("error E10031: cannot modify 's' while it is shared-borrowed", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 7: a view and its owner declared and used
/// together in a loop body are accepted; the next iteration's use belongs to a
/// fresh owner on the exit edge, not to the freed one.
#[test]
fn should_accept_a_loop_local_owner_used_within_each_iteration() {
    let source = r#"module tests.nll.loop_local_owner;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let i: i32 = 0;
    while (i < 2) {
        let s: String = stringOf("x");
        let v: str = s;
        observeStr(v);
        i = i + 1;
    }
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("loop-local-owner", source);
}

/// `spec/03-ownership.md`: a straight-line defer that binds an owner, derives a
/// view, and uses it is accepted; the defer's own release happens after its
/// use.
#[test]
fn should_accept_a_defer_that_owns_and_uses_its_view_before_release() {
    let source = r#"module tests.nll.defer_local_owner;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    defer {
        let s: String = stringOf("x");
        let v: str = s;
        observeStr(v);
    }
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("defer-local-owner", source);
}

// -- Loop-exit precision ---------------------------------------------------

/// `spec/16-borrow-rules.md` section 6: a view bound inside a loop body and
/// used after the loop aliases a per-iteration owner, so its use follows the
/// owner's release and is E10030.
#[test]
fn should_report_e10030_when_a_loop_local_view_escapes_after_the_loop() {
    let source = r#"module tests.nll.loop_local_escapes;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let i: i32 = 0;
    let held: str = "";
    while (i < 1) {
        let s: String = stringOf("x");
        held = s;
        i = i + 1;
    }
    observeStr(held);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "loop-local-escapes",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 1: a loan created in a loop body still
/// keeps the outer owner borrowed after the loop, so moving it is E10022.
#[test]
fn should_report_e10022_when_a_loop_borrow_precedes_a_move_after_the_loop() {
    let source = r#"module tests.nll.loop_borrow_then_move;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}
func sink(take s: String): void {}

func demo(): void {
    let s: String = stringOf("x");
    let held: str = "";
    let i: i32 = 0;
    while (i < 1) {
        held = s;
        i = i + 1;
    }
    sink(s);
    observeStr(held);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "loop-borrow-then-move",
        source,
        &[("error E10022: cannot move 's' while it is borrowed", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 1: the same loan blocks a mutation after
/// the loop, so writing the owner is E10031.
#[test]
fn should_report_e10031_when_a_loop_borrow_precedes_a_write_after_the_loop() {
    let source = r#"module tests.nll.loop_borrow_then_write;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let s: String = stringOf("x");
    let held: str = "";
    let i: i32 = 0;
    while (i < 1) {
        held = s;
        i = i + 1;
    }
    s = stringOf("y");
    observeStr(held);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "loop-borrow-then-write",
        source,
        &[("error E10031: cannot modify 's' while it is shared-borrowed", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: the `do`/`while` exit carries the same
/// rule as the `while` exit.
#[test]
fn should_report_e10030_when_a_do_while_local_view_escapes() {
    let source = r#"module tests.nll.do_while_local_escapes;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let i: i32 = 0;
    let held: str = "";
    do {
        let s: String = stringOf("x");
        held = s;
        i = i + 1;
    } while (i < 1);
    observeStr(held);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "do-while-local-escapes",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: the three-clause `for` exit carries the
/// same rule.
#[test]
fn should_report_e10030_when_a_for_c_local_view_escapes() {
    let source = r#"module tests.nll.for_c_local_escapes;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let held: str = "";
    for (let i: i32 = 0; i < 1; i++) {
        let s: String = stringOf("x");
        held = s;
    }
    observeStr(held);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "for-c-local-escapes",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: the element-loop exit carries the same
/// rule.
#[test]
fn should_report_e10030_when_an_element_loop_local_view_escapes() {
    let source = r#"module tests.nll.for_in_local_escapes;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let xs: u32[1] = [1];
    let held: str = "";
    for (x in xs) {
        let s: String = stringOf("x");
        held = s;
    }
    observeStr(held);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "for-in-local-escapes",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

// -- Back-edge and deferred loans ------------------------------------------

/// `spec/16-borrow-rules.md` section 6: a view bound on the loop back edge is
/// live at the head, so a use at the top of the body reads the storage freed at
/// the end of the prior iteration and is E10030.
#[test]
fn should_report_e10030_when_a_back_edge_view_is_used_at_the_body_top() {
    let source = r#"module tests.nll.back_edge_body_top;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let i: i32 = 0;
    let held: u8[] = bytes(stringOf("z"));
    while (i < 2) {
        firstByte(held);
        let s: String = stringOf("x");
        held = bytes(s);
        i = i + 1;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "back-edge-body-top",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: the loop condition is evaluated at the
/// head on every iteration, so a view bound on the back edge is live there and
/// its use in the condition is E10030.
#[test]
fn should_report_e10030_when_a_back_edge_view_is_used_in_the_condition() {
    let source = r#"module tests.nll.back_edge_condition;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let i: i32 = 0;
    let held: u8[] = bytes(stringOf("z"));
    while (i32(firstByte(held)) >= 0) {
        let s: String = stringOf("x");
        held = bytes(s);
        i = i + 1;
        if (i >= 2) {
            break;
        }
    }
    return i;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "back-edge-condition",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: the `do`/`while` body top carries the
/// same back-edge rule as the `while` body top.
#[test]
fn should_report_e10030_when_a_do_while_back_edge_view_is_used_at_the_body_top() {
    let source = r#"module tests.nll.do_while_body_top;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let i: i32 = 0;
    do {
        firstByte(held);
        let s: String = stringOf("x");
        held = bytes(s);
        i = i + 1;
    } while (i < 2);
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "do-while-body-top",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: the three-clause `for` body top carries
/// the same back-edge rule.
#[test]
fn should_report_e10030_when_a_for_c_back_edge_view_is_used_at_the_body_top() {
    let source = r#"module tests.nll.for_c_body_top;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    for (let i: i32 = 0; i < 2; i++) {
        firstByte(held);
        let s: String = stringOf("x");
        held = bytes(s);
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "for-c-body-top",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: a three-clause `for` step runs after
/// the body, so a view bound on the back edge is live there and its use in the
/// step is E10030.
#[test]
fn should_report_e10030_when_a_back_edge_view_is_used_in_the_step() {
    let source = r#"module tests.nll.back_edge_step;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let k: i32 = 0;
    for (let i: i32 = 0; i < 2; k = k + i32(firstByte(held))) {
        let s: String = stringOf("x");
        held = bytes(s);
    }
    return k;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "back-edge-step",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/03-ownership.md`: a defer runs on the scope's exit edge and its rebind
/// of an outer view is visible after the scope, so a later use of the view is
/// E10030.
#[test]
fn should_report_e10030_when_a_defer_rebinds_an_outer_view_to_a_scope_owner() {
    let source = r#"module tests.nll.defer_rebind_escapes;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(c: bool): void {
    let held: str = "";
    if (c) {
        let s: String = stringOf("x");
        defer {
            held = s;
        }
    }
    observeStr(held);
}

func main(): i32 { demo(true); return 0; }
"#;
    assert_reports_counted(
        "defer-rebind-escapes",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/14-diagnostics.md` E10030: the owner name is still printed when the
/// owner was declared inside a defer and that scope has already closed.
#[test]
fn should_name_a_defer_local_owner_in_e10030() {
    let source = r#"module tests.nll.defer_local_owner_name;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let held: str = "";
    defer {
        let i: i32 = 0;
        while (i < 1) {
            let s: String = stringOf("x");
            held = s;
            i = i + 1;
        }
        observeStr(held);
    }
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "defer-local-owner-name",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 7: an outer owner borrowed in a loop into
/// a loop-local view whose only use is inside the loop is accepted; the move
/// after the loop follows the loan's last use.
#[test]
fn should_accept_a_move_after_an_in_loop_borrow_whose_use_stays_in_the_loop() {
    let source = r#"module tests.nll.in_loop_borrow_then_move;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}
func sink(take s: String): void {}

func demo(): void {
    let s: String = stringOf("x");
    let i: i32 = 0;
    while (i < 2) {
        let v: str = s;
        observeStr(v);
        i = i + 1;
    }
    sink(s);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("in-loop-borrow-then-move", source);
}

/// `spec/03-ownership.md`: a defer runs on the `break` edge too, so a view it
/// rebinds there still escorts the owner's release; a later use is E10030.
#[test]
fn should_report_e10030_when_a_break_edge_defers_a_rebind() {
    let source = r#"module tests.nll.break_defer_rebind;

import { String, stringOf } from std.text;

func observeStr(v: str): void {}

func demo(): void {
    let held: str = "";
    let i: i32 = 0;
    while (i < 1) {
        let s: String = stringOf("x");
        defer {
            held = s;
        }
        break;
    }
    observeStr(held);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_reports_counted(
        "break-defer-rebind",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

// -- Continue edges and the for-condition back edge ------------------------

/// `spec/16-borrow-rules.md` section 6: an explicit `continue` returns to the
/// head, so a loan the body created is live at the body top on the next
/// iteration and the use is E10030.
#[test]
fn should_report_e10030_when_a_continue_edge_carries_a_loan_to_the_body_top() {
    let source = r#"module tests.nll.continue_body_top;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let i: i32 = 0;
    while (i < 2) {
        firstByte(held);
        let s: String = stringOf("x");
        held = bytes(s);
        i = i + 1;
        continue;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "continue-body-top",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: a conditional `continue` still returns
/// to the head, so the loan it created is live there on the next iteration.
#[test]
fn should_report_e10030_when_a_conditional_continue_carries_a_loan_to_the_head() {
    let source = r#"module tests.nll.continue_branch_loan;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let i: i32 = 0;
    while (i < 2) {
        firstByte(held);
        if (i < 1) {
            let s: String = stringOf("x");
            held = bytes(s);
            continue;
        }
        i = i + 1;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "continue-branch-loan",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: the `do`/`while` continue edge carries
/// the same back-edge rule as the `while` continue edge.
#[test]
fn should_report_e10030_when_a_do_while_continue_edge_carries_a_loan() {
    let source = r#"module tests.nll.do_while_continue;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let i: i32 = 0;
    do {
        firstByte(held);
        let s: String = stringOf("x");
        held = bytes(s);
        i = i + 1;
        continue;
    } while (i < 2);
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "do-while-continue",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: the three-clause `for` condition is
/// re-evaluated at the head, so a back-edge view is live there and its use is
/// E10030.
#[test]
fn should_report_e10030_when_a_for_c_condition_use_reaches_the_back_edge() {
    let source = r#"module tests.nll.for_c_condition;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    for (let i: i32 = 0; i32(firstByte(held)) >= 0; i++) {
        let s: String = stringOf("x");
        held = bytes(s);
        if (i >= 1) {
            break;
        }
        continue;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "for-c-condition",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: an element loop's continue edge carries
/// the same rule.
#[test]
fn should_report_e10030_when_a_for_in_continue_edge_carries_a_loan() {
    let source = r#"module tests.nll.for_in_continue;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let xs: u32[2] = [1, 2];
    for (x in xs) {
        firstByte(held);
        let s: String = stringOf("y");
        held = bytes(s);
        continue;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "for-in-continue",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: a value bound to the loop-local owner
/// on a `continue` path is still live at the use after the loop, so it is
/// E10030.
#[test]
fn should_report_e10030_when_a_continue_bound_view_is_used_after_the_loop() {
    let source = r#"module tests.nll.continue_exit;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(c: bool): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let i: i32 = 0;
    while (i < 2) {
        if (c) {
            let s: String = stringOf("x");
            held = bytes(s);
            i = i + 1;
            continue;
        }
        i = i + 1;
    }
    return i32(firstByte(held));
}

func main(): i32 { return demo(true); }
"#;
    assert_reports_counted(
        "continue-exit",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

// -- Defer and drop registration order -------------------------------------

/// `spec/03-ownership.md`: a scope's drops and defers run in one reverse
/// registration order. A defer registered before the local it observes runs
/// after that local's release, so its use of the view is E10030.
#[test]
fn should_report_e10030_when_a_defer_registered_before_a_local_uses_its_view() {
    let source = r#"module tests.nll.defer_before_local;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    {
        defer {
            firstByte(held);
        }
        let s: String = stringOf("x");
        held = bytes(s);
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "defer-before-local",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/03-ownership.md`: the mirror order is legal. A defer registered after
/// the owner it observes runs before the owner's release, so the use is
/// accepted.
#[test]
fn should_accept_a_defer_registered_after_the_owner_it_observes() {
    let source = r#"module tests.nll.defer_after_local;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    {
        let s: String = stringOf("x");
        let held: u8[] = bytes(s);
        defer {
            firstByte(held);
        }
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_accepts("defer-after-local", source);
}

/// `spec/03-ownership.md`: the reverse registration order holds on the `return`
/// cleanup ladder too, so the defer runs after the earlier-registered owner's
/// release.
#[test]
fn should_report_e10030_when_a_return_orders_a_defer_before_a_local() {
    let source = r#"module tests.nll.return_defer_before_local;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    defer {
        firstByte(held);
    }
    let s: String = stringOf("x");
    held = bytes(s);
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "return-defer-before-local",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/03-ownership.md`: the reverse registration order holds on the `break`
/// edge too.
#[test]
fn should_report_e10030_when_a_break_orders_a_defer_before_a_local() {
    let source = r#"module tests.nll.break_defer_before_local;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let i: i32 = 0;
    while (i < 1) {
        defer {
            firstByte(held);
        }
        let s: String = stringOf("x");
        held = bytes(s);
        break;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "break-defer-before-local",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/03-ownership.md`: the reverse registration order holds on the
/// `continue` edge too.
#[test]
fn should_report_e10030_when_a_continue_orders_a_defer_before_a_local() {
    let source = r#"module tests.nll.continue_defer_before_local;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let i: i32 = 0;
    while (i < 1) {
        defer {
            firstByte(held);
        }
        let s: String = stringOf("x");
        held = bytes(s);
        i = i + 1;
        continue;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "continue-defer-before-local",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

// -- Control flow inside a defer -------------------------------------------

/// `spec/03-ownership.md`: a `return` inside a defer is part of the defer's own
/// path, so it must not wipe the enclosing loop's state; the program is
/// accepted.
#[test]
fn should_accept_a_return_inside_a_defer() {
    let source = r#"module tests.nll.defer_return;

func demo(): i32 {
    let i: i32 = 0;
    while (i < 1) {
        defer {
            return 7;
        }
        i = i + 1;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_accepts("defer-return", source);
}

/// `spec/03-ownership.md`: a `continue` inside a defer is part of the defer's
/// own path; the owner used after the defer registration stays live and the
/// program is accepted.
#[test]
fn should_accept_a_continue_inside_a_defer() {
    let source = r#"module tests.nll.defer_continue;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let i: i32 = 0;
    while (i < 2) {
        let s: String = stringOf("x");
        {
            defer {
                continue;
            }
        }
        firstByte(bytes(s));
        i = i + 1;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_accepts("defer-continue", source);
}

/// `spec/03-ownership.md`: a `break` inside a defer is part of the defer's own
/// path; the owner used after the defer registration stays live and the program
/// is accepted.
#[test]
fn should_accept_a_break_inside_a_defer() {
    let source = r#"module tests.nll.defer_break;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let i: i32 = 0;
    while (i < 2) {
        let s: String = stringOf("x");
        {
            defer {
                break;
            }
        }
        firstByte(bytes(s));
        i = i + 1;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_accepts("defer-break", source);
}

// -- Loop joins across continue paths --------------------------------------

/// `spec/16-borrow-rules.md` section 6: the three-clause `for` step runs on
/// every continue edge, so a loan a continue path created is live at the step
/// use and is E10030.
#[test]
fn should_report_e10030_when_a_for_step_use_reaches_a_continue_paths_loan() {
    let source = r#"module tests.nll.for_step_continue_loan;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let q: String = stringOf("q");
    let i: i32 = 0;
    for (let j: i32 = 0; i < 2; firstByte(held), j++) {
        if (i < 1) {
            let s: String = stringOf("x");
            held = bytes(s);
            i = i + 1;
            continue;
        }
        held = bytes(q);
        i = i + 1;
        continue;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "for-step-continue-loan",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: a move in the `for` step reaches the
/// head on the back edge, so the next iteration's condition read and step read
/// of the moved owner are both `E10020`.
#[test]
fn should_report_e10020_when_a_for_step_move_reaches_the_condition() {
    let source = r#"module tests.nll.for_step_move;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func sink(take s: String): void {}

func demo(): i32 {
    let s: String = stringOf("x");
    for (let i: i32 = 0; i < 2 && firstByte(bytes(s)) >= 0; sink(s)) {
        i = i + 1;
        continue;
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "for-step-move",
        source,
        &[("error E10020: 's' was moved and cannot be used", 2)],
    );
}

/// `spec/16-borrow-rules.md` section 6: the `do`/`while` condition runs on
/// every continue edge, so a continue-path loan is live at the condition use
/// and is E10030.
#[test]
fn should_report_e10030_when_a_do_while_condition_use_reaches_a_continue_loan() {
    let source = r#"module tests.nll.do_while_condition_continue_loan;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let q: String = stringOf("q");
    let i: i32 = 0;
    do {
        if (i < 1) {
            let s: String = stringOf("x");
            held = bytes(s);
            i = i + 1;
            continue;
        }
        held = bytes(q);
        i = i + 1;
        continue;
    } while (i32(firstByte(held)) >= 0);
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "do-while-condition-continue-loan",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: the `do`/`while` exit joins every
/// continue path, so a loan created only on a continue path is live after the
/// loop and is E10030.
#[test]
fn should_report_e10030_when_a_do_while_exit_reaches_a_continue_paths_loan() {
    let source = r#"module tests.nll.do_while_exit_continue_loan;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    let q: String = stringOf("q");
    let i: i32 = 0;
    do {
        if (i < 1) {
            let s: String = stringOf("x");
            held = bytes(s);
            i = i + 1;
            continue;
        }
        held = bytes(q);
        i = i + 1;
        continue;
    } while (i < 1);
    return i32(firstByte(held));
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "do-while-exit-continue-loan",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 6: when a `while (true)` body always
/// terminates, the code after the loop joins every `break` edge, so an earlier
/// break path's loan is live there and is E10030.
#[test]
fn should_report_e10030_when_a_break_only_loop_exit_joins_every_break() {
    let source = r#"module tests.nll.exit_two_break;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func demo(c: bool): i32 {
    let held: u8[] = bytes(stringOf("z"));
    while (true) {
        if (c) {
            let s: String = stringOf("x");
            held = bytes(s);
            break;
        }
        break;
    }
    return i32(firstByte(held));
}

func main(): i32 { return demo(true); }
"#;
    assert_reports_counted(
        "exit-two-break",
        source,
        &[("error E10030: borrow of 's' does not live long enough", 1)],
    );
}

/// `spec/03-ownership.md`: a defer whose body owns a temporary is a legal
/// program and must not panic the checker.
#[test]
fn should_accept_a_defer_that_owns_a_temporary() {
    let source = r#"module tests.nll.defer_owned_temp;

import { stringOf, bytes } from std.text;

func demo(): i32 {
    let held: u8[] = bytes(stringOf("z"));
    defer {
        held = bytes(stringOf("q"));
    }
    return 0;
}

func main(): i32 { return demo(); }
"#;
    assert_accepts("defer-owned-temp", source);
}

// -- Condition effects reach the loop exit ---------------------------------

/// `spec/16-borrow-rules.md` section 7: a move the `while` condition makes
/// reaches the code after the loop on the false path, so reusing the owner is
/// `E10020`.
#[test]
fn should_report_e10020_when_a_while_condition_moves_the_owner() {
    let source = r#"module tests.nll.while_condition_move;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func consume(take s: String): bool {
    return false;
}

func demo(): i32 {
    let s: String = stringOf("x");
    while (consume(s)) {
    }
    let v: u8[] = bytes(s);
    return i32(firstByte(v));
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "while-condition-move",
        source,
        &[("error E10020: 's' was moved and cannot be used", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 7: a move the `do`/`while` condition makes
/// reaches the code after the loop even when the body always `continue`s.
#[test]
fn should_report_e10020_when_a_do_while_continue_body_moves_in_the_condition() {
    let source = r#"module tests.nll.do_while_condition_move;

import { String, stringOf, bytes } from std.text;

func firstByte(v: u8[]): u8 {
    return v[0];
}

func consume(take s: String): bool {
    return false;
}

func demo(): i32 {
    let s: String = stringOf("x");
    do {
        continue;
    } while (consume(s));
    let v: u8[] = bytes(s);
    return i32(firstByte(v));
}

func main(): i32 { return demo(); }
"#;
    assert_reports_counted(
        "do-while-condition-move",
        source,
        &[("error E10020: 's' was moved and cannot be used", 1)],
    );
}

/// `spec/14-diagnostics.md` E10021: a temporary a deferred block materialized
/// does not leak its binding state into the local that reuses the id, so an
/// uninitialized use is still reported.
#[test]
fn should_report_e10021_when_a_later_local_reuses_a_defer_temporary_id() {
    let source = r#"module tests.nll.defer_temp_state;

import { String, stringOf, bytes } from std.text;

func observeStr(v: str): void {}

func demo(c: bool): i32 {
    let held: u8[] = bytes(stringOf("z"));
    {
        defer {
            observeStr(stringOf("q"));
        }
    }
    let x: String;
    if (c) {
        x = stringOf("w");
    }
    observeStr(x);
    return 0;
}

func main(): i32 { return demo(false); }
"#;
    assert_reports_counted(
        "defer-temp-state",
        source,
        &[("error E10021: 'x' may be used before it is initialized", 1)],
    );
}

/// `spec/16-borrow-rules.md` section 7: a view used in the `for` step on every
/// iteration is accepted when the owner is moved only after the loop, so the
/// step join does not over-reject.
#[test]
fn should_accept_a_for_step_view_use_before_the_owners_move() {
    let source = r#"module tests.nll.for_step_view_use;

import { String, stringOf, bytes } from std.text;

func observe(v: u8[]): void {}
func sink(take s: String): void {}

func demo(): void {
    let s: String = stringOf("x");
    let v: u8[] = bytes(s);
    for (let i: i32 = 0; i < 2; observe(v), i++) {
    }
    sink(s);
}

func main(): i32 { demo(); return 0; }
"#;
    assert_accepts("for-step-view-use", source);
}
