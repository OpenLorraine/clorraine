//! Implementation tests for the module initializer integrity batch.
//!
//! The specification suite proves what an initializer computes; these cases
//! prove how the compiler emits its resolved callables: a lifted lambda with
//! its declaration and definition, a func-typed global called through its own
//! storage, a generic call reaching its declared instance, and a callable-field
//! call passing its receiver.

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
            "clc-criimpl-{}-{tag}-{sequence}",
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
/// Returns the emitted translation unit. These cases are about what the
/// emitter produces for accepted programs, so a rejection is a failure.
fn emit_host(tag: &str, source: &str) -> String {
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
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty"
    );
    emitted
}

/// Asserts the emitted C contains `needle`, reporting the full text otherwise.
fn assert_contains(tag: &str, emitted: &str, needle: &str) {
    assert!(
        emitted.contains(needle),
        "{tag}: expected the emitted C to contain {needle:?}; emitted C:\n{emitted}"
    );
}

/// Counts non-overlapping occurrences of `needle`.
fn count(emitted: &str, needle: &str) -> usize {
    emitted.matches(needle).count()
}

/// Asserts the first `s = NULL;` after `call_at` follows the call directly.
///
/// The emitted C may carry `#line` markers between the call and the next
/// statement, so the lines after the call's own line may consist of those
/// markers and blank lines; any other text means a statement intervenes before
/// the move-out.
fn assert_move_out_follows(tag: &str, emitted: &str, call_at: usize) {
    let after = &emitted[call_at..];
    let line_end = after.find('\n').map_or(after.len(), |index| index + 1);
    let rest = &after[line_end..];
    let null_at = rest
        .find("s = NULL;")
        .unwrap_or_else(|| panic!("{tag}: no move-out follows the call; emitted C:\n{emitted}"));
    assert!(
        rest[..null_at]
            .lines()
            .all(|line| line.trim().is_empty() || line.trim_start().starts_with("#line")),
        "{tag}: a statement intervenes before the move-out; emitted C:\n{emitted}"
    );
}

/// Emits one program twice from the same source path, returning both texts.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let mut emitted = Vec::new();
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
            "{tag}: the front end rejected the program; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        emitted.push(std::fs::read_to_string(&c_path).unwrap_or_default());
    }
    (emitted[0].clone(), emitted[1].clone())
}


/// An initializer lambda lifts to a module-level function.
///
/// The initializer's call reaches the lifted identity, and the lifted function
/// is declared and defined exactly as a body lambda's is.
#[test]
fn should_lift_an_initializer_lambda() {
    let source = r#"module spec.mii.impl.lambdacallee;

let g: i32 = (() => 3)();

func main(): i32 {
    return g - 3;
}
"#;
    let emitted = emit_host("lambdacallee", source);
    assert_contains("lambdacallee", &emitted, "static i32 cl_lambda_0(void);");
    assert_contains("lambdacallee", &emitted, "static i32 cl_lambda_0(void)\n{");
    assert_contains(
        "lambdacallee",
        &emitted,
        "cl_spec_mii_impl_lambdacallee_g = cl_lambda_0();",
    );
}

/// A func-typed global call in an initializer spells the global's symbol.
///
/// The call goes through the global's own storage, so the written name never
/// reaches the C file as a free function.
#[test]
fn should_spell_a_func_typed_global_call_in_an_initializer() {
    let source = r#"module spec.mii.impl.funcglobal;

func ready(): i32 {
    return 7;
}

let f: func(): i32 = ready;
let g: i32 = f();

func main(): i32 {
    return g - 7;
}
"#;
    let emitted = emit_host("funcglobal", source);
    assert_contains(
        "funcglobal",
        &emitted,
        "cl_spec_mii_impl_funcglobal_f = cl_spec_mii_impl_funcglobal_ready;",
    );
    assert_contains(
        "funcglobal",
        &emitted,
        "cl_spec_mii_impl_funcglobal_g = (cl_spec_mii_impl_funcglobal_f)();",
    );
}

/// A generic initializer call reaches its declared instance.
///
/// The instance machinery walks initializer expressions, so the call and the
/// definition share one key and no implicit declaration remains.
#[test]
fn should_declare_a_generic_initializer_call_instance() {
    let source = r#"module spec.mii.impl.genericcall;

func pick<T>(a: T, b: T): T {
    return a;
}

let g: i32 = pick<i32>(1, 2);

func main(): i32 {
    return g - 1;
}
"#;
    let emitted = emit_host("genericcall", source);
    assert_contains("genericcall", &emitted, "static i32 cl_pick__i32(i32 a, i32 b);");
    assert_contains("genericcall", &emitted, "static i32 cl_pick__i32(i32 a, i32 b)\n{");
    assert_contains(
        "genericcall",
        &emitted,
        "cl_spec_mii_impl_genericcall_g = cl_pick__i32(1, 2);",
    );
}

/// A callable-field call in an initializer passes the field read.
///
/// The receiver's global is spelled and the field read is the callable, so the
/// call renders through the receiver exactly as the body's field-call path
/// renders it.
#[test]
fn should_pass_a_field_receiver_in_an_initializer() {
    let source = r#"module spec.mii.impl.fieldreceiver;

func ready(): i32 {
    return 7;
}

struct Box { get: func(): i32; }

let b: Box = Box { get: ready };
let g: i32 = b.get();

func main(): i32 {
    return g - 7;
}
"#;
    let emitted = emit_host("fieldreceiver", source);
    assert_contains(
        "fieldreceiver",
        &emitted,
        "cl_spec_mii_impl_fieldreceiver_g = (cl_spec_mii_impl_fieldreceiver_b.get)();",
    );
}

/// A lambda-valued func-typed global binds the lifted identity.
///
/// The global's initializer is a lambda value, so the lifted function is its
/// C value exactly as a body lambda's would be, and the later call reaches it
/// through the global.
#[test]
fn should_bind_a_lambda_valued_global_to_the_lifted_identity() {
    let source = r#"module spec.mii.impl.lambdavalue;

import { String, stringOf } from std.text;

let f: func(String): i32 = (s: String) => i32(len(s));
let g: i32 = f(stringOf("abc"));

func main(): i32 {
    return g - 3;
}
"#;
    let emitted = emit_host("lambdavalue", source);
    assert_contains(
        "lambdavalue",
        &emitted,
        "cl_spec_mii_impl_lambdavalue_f = cl_lambda_0;",
    );
    assert_contains(
        "lambdavalue",
        &emitted,
        "cl_spec_mii_impl_lambdavalue_g = (cl_spec_mii_impl_lambdavalue_f)(cl_std_text_stringOf(cl_str(\"abc\")));",
    );
}
