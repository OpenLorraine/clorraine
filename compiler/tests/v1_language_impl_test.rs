//! Implementation tests for the v1 language completion internals.
//!
//! These tests cover the emitter and checker edges the runnable specification
//! suite does not pin directly: that a generic struct instantiation dedupes to
//! one emitted C struct, that nested arguments compose into one symbol, and
//! that the generics diagnostics report through the real compiler binary. They
//! are not the specification oracle; a defect here is an implementation bug.

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
            "clc-v1lang-impl-{}-{tag}-{sequence}",
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

/// Compiles one source for the host profile and returns the emitted C.
///
/// The front end must accept the program; a rejection is a test failure, not a
/// result to assert.
fn emit(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("out.c");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("cannot write the snippet");
    let compile = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        compile.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Compiles one source and returns the compiler's stderr.
fn diagnostics(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("out.c");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("cannot write the snippet");
    let compile = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    String::from_utf8_lossy(&compile.stderr).into_owned()
}

/// The number of times `needle` appears in `haystack`.
fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

#[test]
fn one_c_struct_per_distinct_instantiation_is_emitted() {
    let source = r#"module tests.v1.impl.dedupe;

struct Pair<T> {
    first: T;
    second: T;
}

func main(): i32 {
    let a = Pair<i32> { first: 1, second: 2 };
    let b = Pair<i32> { first: 3, second: 4 };
    return a.first + b.second;
}
"#;
    let emitted = emit("dedupe", source);
    assert_eq!(
        count(&emitted, "struct ClPair__i32 {"),
        1,
        "two uses of Pair<i32> must share one emitted struct"
    );
    assert!(
        !emitted.contains("struct ClPair {"),
        "the bare generic template must not be emitted"
    );
}

#[test]
fn nested_arguments_compose_into_a_single_symbol() {
    let source = r#"module tests.v1.impl.nested;

struct Pair<T> {
    first: T;
    second: T;
}

func main(): i32 {
    let inner = Pair<i32> { first: 1, second: 2 };
    let outer = Pair<Pair<i32>> { first: inner, second: inner };
    return outer.first.first;
}
"#;
    let emitted = emit("nested", source);
    assert_eq!(
        count(&emitted, "struct ClPair__Pair__i32 {"),
        1,
        "the nested key must compose from the inner key"
    );
}

#[test]
fn a_type_argument_violating_a_bound_reports_e10303() {
    let source = r#"module tests.v1.impl.bound;

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func main(): i32 {
    return apply<i32, i32>(5, 3);
}
"#;
    let stderr = diagnostics("bound", source);
    assert!(
        stderr.contains("error E10303"),
        "expected E10303, got:\n{stderr}"
    );
}

#[test]
fn returning_a_callable_only_value_reports_e10304() {
    let source = r#"module tests.v1.impl.callable;

func keep<T, F: func(T): T>(x: T, f: F): F {
    return f;
}

func main(): i32 {
    return 0;
}
"#;
    let stderr = diagnostics("callable", source);
    assert!(
        stderr.contains("error E10304"),
        "expected E10304, got:\n{stderr}"
    );
}

#[test]
fn a_struct_that_embeds_itself_reports_e10305() {
    let source = r#"module tests.v1.impl.recursive;

struct Wrapper<T> {
    inner: T;
}

struct Nest<T> {
    next: Nest<Wrapper<T>>;
}

func main(): i32 {
    return 0;
}
"#;
    let stderr = diagnostics("recursive", source);
    assert!(
        stderr.contains("error E10305"),
        "expected E10305, got:\n{stderr}"
    );
}
