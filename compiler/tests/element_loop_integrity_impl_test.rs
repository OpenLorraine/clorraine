//! Implementation tests for the shape of element reads passed as arguments.
//!
//! These cases cover what the specification suite cannot see: the emitted C for
//! an element argument (the element is passed directly as a view, with no
//! `__cl_owned` scope temporary), for an ownerless temporary argument (the
//! temporary is still bound and released exactly once), and for a template
//! placeholder over an element (the placeholder's concatenation result owns its
//! storage, but the element read itself is not bound).
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
/// owned temporary (the result) is bound and released. The element read feeds
/// a copy into the result instead of being bound as a second owner, which the
/// pre-fix shape did with an `__cl_owned0 = arr[0];` assignment.
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
        emitted.contains("cl_string_copy(arr[0])"),
        "ei3: the element should feed the placeholder result by copy"
    );
    assert!(
        emitted.contains("FreeString(__cl_owned0)"),
        "ei3: the placeholder result should be released by the scope cleanup"
    );
}
