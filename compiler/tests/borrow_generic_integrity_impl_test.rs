//! Implementation tests for the borrow/generic integrity batch.
//!
//! These cases pin the observable internals of the fixes: the emitted C shape
//! of an accepted nullable out-slot store and the diagnostic counts of a
//! store site that the checker visits on more than one pass. They complement
//! the specification suite, which pins the accepted and rejected behavior.

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
            "clc-borrowgenericimpl-{}-{tag}-{sequence}",
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

/// Compiles `source` and returns the scratch directory, output, and C path.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
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
    (scratch, output, c_path)
}

/// Emits one host-profile program and asserts the front end accepted it.
///
/// Returns the emitted translation unit.
fn emit_host(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
    );
    emitted
}

/// Asserts the text contains a fragment.
fn assert_contains(tag: &str, text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "{tag}: the emitted C does not contain {needle:?}"
    );
}

/// Asserts the first fragment appears before the second one in the text.
fn assert_before(tag: &str, text: &str, first: &str, second: &str) {
    let first_at = text
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: the text does not contain {first:?}"));
    let second_at = text
        .find(second)
        .unwrap_or_else(|| panic!("{tag}: the text does not contain {second:?}"));
    assert!(
        first_at < second_at,
        "{tag}: expected {first:?} before {second:?}"
    );
}

/// Counts the occurrences of a fragment.
fn count_occurrences(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

/// A nullable out-slot store is emitted as a pointer write with one release.
///
/// The callee receives the address of the caller's slot, so the definition
/// declares a pointer parameter and the call site passes the slot's address.
/// The store releases the callee-owned incoming value, zeroes the slot, stores
/// the new value through the pointer, and empties the moved source, in that
/// order. A store that skipped the release would leak the incoming value; one
/// that released after the store would free the value it just wrote.
#[test]
fn should_emit_a_pointer_store_when_a_nullable_out_slot_receives_a_value() {
    let source = r#"module spec.bgi.impl.outslot;

import { String, stringOf } from std.text;

func fillString(take v: String, borrow slot: String?): void {
    slot = v;
}

func main(): i32 {
    let slot: String? = null;
    fillString(stringOf("xyz"), borrow slot);
    if (slot == null) {
        return 9;
    }
    return i32(len(slot));
}
"#;
    let emitted = emit_host("out-slot-store", source);
    assert_contains("out-slot-store", &emitted, "ClString *slot)");
    assert_contains("out-slot-store", &emitted, "(*slot) = v;");
    assert_contains("out-slot-store", &emitted, "&(slot)");
    assert_before(
        "out-slot-store",
        &emitted,
        "FreeString((*slot))",
        "(*slot) = v;",
    );
    assert_before("out-slot-store", &emitted, "(*slot) = v;", "v = NULL;");
}

/// A parameter-typed store inside a loop reports each diagnostic once.
///
/// The checker walks a loop body on a speculative pass and on the reporting
/// pass, so a duty recorded at the store would be judged twice if the site did
/// not dedupe. The program is rejected with exactly one `E10034` for the store
/// and exactly one `E10020` for the source the loop's second iteration may
/// have moved; a duplicated count means the site was judged once per pass.
#[test]
fn should_report_one_slot_error_when_a_loop_body_stores_into_a_parameter_typed_slot() {
    let source = r#"module spec.bgi.impl.loopstore;

import { String, stringOf } from std.text;

func fill<T>(take v: T, borrow slot: T, flag: bool): void {
    let i: i32 = 0;
    while (i < 1) {
        i = i + 1;
        if (flag) {
            slot = v;
        }
    }
}

func main(): i32 {
    let s: String = stringOf("ab");
    fill<String>(stringOf("xyz"), borrow s, true);
    return 0;
}
"#;
    let (_scratch, output, c_path) = compile_only("loop-store", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "loop-store: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        count_occurrences(&stderr, "error E10034:"),
        1,
        "loop-store: expected exactly one E10034; stderr:\n{stderr}"
    );
    assert_eq!(
        count_occurrences(&stderr, "error E10020:"),
        1,
        "loop-store: expected exactly one E10020; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "loop-store: a rejected program must not produce a translation unit"
    );
}
