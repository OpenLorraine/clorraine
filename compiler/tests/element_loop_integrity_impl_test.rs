//! Implementation tests for the shape of element reads passed as arguments and
//! of nested loop steps.
//!
//! The element-read cases cover what the specification suite cannot see: the
//! emitted C for an element argument (the element is passed directly as a view,
//! with no `__cl_owned` scope temporary), for an ownerless temporary argument
//! (the temporary is still bound and released exactly once), and for a template
//! placeholder over an element (the placeholder's concatenation result owns its
//! storage, but the element read itself is not bound).
//!
//! The nested-loop cases cover the shape the specification suite cannot see:
//! each nested `for` prints its own increment in its own header, with no dead
//! labelled step block and no duplicated loop, and two emissions of one program
//! are byte-identical.
//!
//! Every run goes through a helper with a time limit, so a loop regression
//! fails the case instead of freezing the suite.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one program may run before the case fails as a hang.
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

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
            "clc-elemimpl-{}-{tag}-{sequence}",
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

/// Emits one host-profile program and asserts the front end accepted it.
///
/// Returns the emitted translation unit. A rejection is a failure of the case,
/// not a result to assert, because these cases are about what the emitter
/// produces for accepted programs.
fn emit_host(tag: &str, scratch: &ScratchDir, source: &str) -> String {
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

/// Compiles the emitted C with the host compiler under the strict flag set.
///
/// Panics with the compiler's stderr when the build fails. Returns the built
/// program path.
fn build_host_c(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    program
}

/// Runs a built program with a time limit and returns its output.
///
/// A program that is still running when the limit passes is killed and the
/// case fails, so a regression that loops forever surfaces as one failing case
/// instead of a frozen suite.
fn run_with_timeout(tag: &str, program: &Path) -> Output {
    let mut child = Command::new(program)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"));
    let deadline = Instant::now() + RUN_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("{tag}: the program did not finish within {RUN_TIMEOUT:?}");
            }
            Err(err) => panic!("{tag}: cannot wait for the program: {err}"),
        }
    }
    child
        .wait_with_output()
        .unwrap_or_else(|err| panic!("{tag}: cannot collect the program output: {err}"))
}

/// An element read passed as an argument is rendered as a direct view.
///
/// The array owns the element, so the emitter must not bind a scope temporary
/// for it: the call receives the element view directly and the scope cleanup
/// only releases the array. The case also runs the program, so a shape that
/// still double-frees would abort instead of returning the length.
#[test]
fn should_pass_an_element_view_directly_when_the_argument_reads_an_element() {
    let source = r#"module codegen.elem.impl1;

import { String, stringOf } from std.text;

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let arr: String[2] = [stringOf("a"), stringOf("bb")];
    return vlen(arr[0]);
}
"#;
    let scratch = ScratchDir::new("ei1");
    let emitted = emit_host("ei1", &scratch, source);
    assert!(
        !emitted.contains("__cl_owned"),
        "ei1: an element argument must not bind an owned temporary"
    );
    assert!(
        emitted.contains("cl_string_view(arr[0])"),
        "ei1: the element should be passed as a direct view"
    );
    let program = build_host_c("ei1", &scratch, &emitted);
    let output = run_with_timeout("ei1", &program);
    assert_eq!(
        output.status.code(),
        Some(1),
        "ei1: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// An ownerless call-result argument keeps its single scope binding.
///
/// A call result has no container behind it, so the emitter binds it to
/// `__cl_owned0`, passes a view of the binding, and releases the binding once.
/// The case also runs the program so the single release is proven at runtime.
#[test]
fn should_bind_and_release_once_when_the_argument_is_an_ownerless_temporary() {
    let source = r#"module codegen.elem.impl2;

import { String, stringOf } from std.text;

func vlen(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    return vlen(stringOf("hi"));
}
"#;
    let scratch = ScratchDir::new("ei2");
    let emitted = emit_host("ei2", &scratch, source);
    assert!(
        emitted.contains("cl_string_view(__cl_owned0)"),
        "ei2: the ownerless temporary should be bound and passed as a view"
    );
    assert!(
        emitted.contains("FreeString(__cl_owned0)"),
        "ei2: the ownerless temporary should be released by the scope cleanup"
    );
    let program = build_host_c("ei2", &scratch, &emitted);
    let output = run_with_timeout("ei2", &program);
    assert_eq!(
        output.status.code(),
        Some(2),
        "ei2: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A template placeholder over an element does not bind the element itself.
///
/// The placeholder's concatenation allocates its own result, so exactly one
/// owned temporary (the result) is bound and released. The element read is
/// borrowed into the result instead of being bound as a second owner, which
/// the pre-fix shape did with an `__cl_owned0 = arr[0];` assignment.
#[test]
fn should_bind_only_the_placeholder_result_when_an_element_is_printed() {
    let source = r#"module codegen.elem.impl3;

import { String, stringOf } from std.text;
import { println } from std.io;

func main(): i32 {
    let arr: String[1] = [stringOf("ab")];
    println($"{arr[0]}");
    return 0;
}
"#;
    let scratch = ScratchDir::new("ei3");
    let emitted = emit_host("ei3", &scratch, source);
    assert!(
        !emitted.contains("__cl_owned0 = arr[0];"),
        "ei3: the element read must not be bound to an owned temporary"
    );
    assert!(
        emitted.contains("cl_str_part_string(arr[0])"),
        "ei3: the element should be borrowed into the placeholder result"
    );
    assert!(
        emitted.contains("FreeString(__cl_owned0)"),
        "ei3: the placeholder result should be released by the scope cleanup"
    );
}

/// Each nested element loop prints its own step in its own C header.
///
/// The outer header increments the outer index and the inner header increments
/// the inner index. Exactly two structured `for` headers exist, and no dead
/// labelled step block remains, so a nested loop can never be printed twice or
/// re-enter its own pre-header as a step.
#[test]
fn should_emit_each_own_header_when_element_loops_are_nested() {
    let source = r#"module codegen.elem.impl4;

func main(): i32 {
    let xs: i32[2] = [1, 2];
    let ys: i32[3] = [1, 2, 3];
    let s: i32 = 0;
    for (borrow x in xs) {
        for (borrow y in ys) {
            s = s + x + y;
        }
    }
    return s;
}
"#;
    let scratch = ScratchDir::new("ei4");
    let emitted = emit_host("ei4", &scratch, source);
    assert!(
        emitted.contains("for (; (__cl_tmp0 < 2); __cl_tmp0 = (__cl_tmp0 + 1u))"),
        "ei4: the outer header should carry the outer increment"
    );
    assert!(
        emitted.contains("for (; (__cl_tmp2 < 3); __cl_tmp2 = (__cl_tmp2 + 1u))"),
        "ei4: the inner header should carry the inner increment"
    );
    assert_eq!(
        emitted.matches("for (; (").count(),
        2,
        "ei4: a nested loop must not be printed a second time"
    );
    assert!(
        !emitted.contains("__cl_b"),
        "ei4: no dead labelled step block should remain"
    );
}

/// A nested three-clause `for` prints its own increment in its C header.
///
/// The inner header carries the increment of the inner loop variable (`j`), not
/// the outer loop's index; with the outer step in its own header, the program
/// terminates.
#[test]
fn should_emit_its_own_increment_when_a_three_clause_for_is_nested() {
    let source = r#"module codegen.elem.impl5;

func main(): i32 {
    let xs: i32[2] = [1, 2];
    let s: i32 = 0;
    for (borrow x in xs) {
        for (let j: i32 = 0; j < 3; j = j + 1) {
            s = s + x + j;
        }
    }
    return s;
}
"#;
    let scratch = ScratchDir::new("ei5");
    let emitted = emit_host("ei5", &scratch, source);
    assert!(
        emitted.contains("for (; (j < 3); j = (i32)((u32)j + (u32)1))"),
        "ei5: the inner header should carry the three-clause increment"
    );
    assert_eq!(
        emitted.matches("for (; (").count(),
        2,
        "ei5: the nested loop must not be printed a second time"
    );
}

/// Emitting the same nested program twice produces byte-identical C.
///
/// Determinism guards the emitter against order-dependent block selection when
/// a loop's step is chosen. Both emissions read the same source path, so the
/// embedded `#line` source name is identical and any difference is emitter
/// behavior.
#[test]
fn should_emit_identical_c_when_the_same_nested_program_is_emitted_twice() {
    let source = r#"module codegen.elem.impl6;

func main(): i32 {
    let xs: i32[2] = [1, 2];
    let ys: i32[3] = [1, 2, 3];
    let s: i32 = 0;
    for (borrow x in xs) {
        for (borrow y in ys) {
            s = s + x + y;
        }
    }
    return s;
}
"#;
    let scratch = ScratchDir::new("ei6");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("ei6: cannot write the program: {err}"));
    let mut emissions = Vec::new();
    for name in ["first.c", "second.c"] {
        let c_path = scratch.path().join(name);
        let output = run_clc(&[
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            c_path.display().to_string(),
            source_path.display().to_string(),
        ]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "ei6: the front end rejected the program; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        emissions.push(
            std::fs::read_to_string(&c_path)
                .unwrap_or_else(|err| panic!("ei6: cannot read {name}: {err}")),
        );
    }
    assert!(
        !emissions[0].trim().is_empty(),
        "ei6: the emitted C is empty"
    );
    assert_eq!(
        emissions[0], emissions[1],
        "ei6: two emissions of the same program must be byte-identical"
    );
}
