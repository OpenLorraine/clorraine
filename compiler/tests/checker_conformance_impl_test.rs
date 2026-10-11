//! Implementation tests for the checker conformance batch.
//!
//! The specification suite proves what the programs compute; these cases prove
//! how the compiler emits the fixed shapes: the initializer's single-evaluation
//! stores for an impure dividend and an impure slice index base, the slice
//! typedefs an inline argument, a lifted specialization, and a generic
//! struct-instance element need (with the instance declared first), and the
//! owning index base's unchanged release.

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



/// The initializer stores an impure dividend once.
///
/// The guard and the division read the stored local, so the call appears
/// exactly once (red at the phase base: the guard re-rendered the call).
#[test]
fn should_store_an_impure_dividend_once_in_an_initializer() {
    let source = r#"module spec.cc.impl.initdividend;

let calls: i32 = 0;

func d(): i32 {
    calls = calls + 1;
    return 2;
}

let g: i32 = d() / 1;

func main(): i32 {
    return calls * 10 + g;
}
"#;
    let emitted = emit_host("initdividend", source);
    assert_contains("initdividend",
        &emitted,
        "i32 __cl_itmp0 = cl_spec_cc_impl_initdividend_d();",
    );
    assert_contains("initdividend", &emitted, "(__cl_itmp0 == -2147483648");
    assert_contains("initdividend",
        &emitted,
        "cl_spec_cc_impl_initdividend_g = (__cl_itmp0 / 1);",
    );
    assert_eq!(count(&emitted, "cl_spec_cc_impl_initdividend_d();"), 1);
}

/// The initializer stores an impure slice index base once.
///
/// The bounds guard and the element access read the stored local, so the call
/// behind the base appears exactly once (red at the phase base: the guard
/// re-rendered the call).
#[test]
fn should_store_an_impure_index_base_once_in_an_initializer() {
    let source = r#"module spec.cc.impl.initbase;

import { slice } from std.mem;

let calls: i32 = 0;
let xs: i32[4] = [1, 2, 3, 4];

func v(s: i32[]): i32[] {
    calls = calls + 1;
    return s;
}

let sl: i32[] = slice(xs, 0, 4);
let e: i32 = v(sl)[1];

func main(): i32 {
    return calls * 10 + e;
}
"#;
    let emitted = emit_host("initbase", source);
    assert_contains("initbase",
        &emitted,
        "ClSlice_i32 __cl_itmp0 = cl_spec_cc_impl_initbase_v(cl_spec_cc_impl_initbase_sl);",
    );
    assert_contains("initbase", &emitted, "((usize)(1) >= (usize)((__cl_itmp0).len))");
    assert_contains("initbase",
        &emitted,
        "cl_spec_cc_impl_initbase_e = (__cl_itmp0).ptr[1];",
    );
    assert_eq!(
        count(
            &emitted,
            "cl_spec_cc_impl_initbase_v(cl_spec_cc_impl_initbase_sl)"
        ),
        1
    );
}

/// An inline slice argument defines its typedef and use.
///
/// The builder's compound literal names the view type, so the typedef is
/// declared before the function that builds it.
#[test]
fn should_define_an_inline_argument_slice_typedef() {
    let source = r#"module spec.cc.impl.inlinearg;

import { slice } from std.mem;
import { stringOf } from std.text;

func mkS(): String[2] {
    return [stringOf("ab"), stringOf("cd")];
}

func main(): i32 {
    return i32(len(slice(mkS(), 0, 2)));
}
"#;
    let emitted = emit_host("inlinearg", source);
    assert_contains("inlinearg",
        &emitted,
        "typedef struct { ClString *ptr; usize len; } ClSlice_String;",
    );
    assert_contains("inlinearg",
        &emitted,
        "((ClSlice_String){ (__cl_tmp0) + (0), (usize)((2) - (0)) })",
    );
}

/// A lifted-lambda specialization defines its written slice typedef.
///
/// The specialization's body measures the view, so the typedef precedes the
/// lifted function that uses it.
#[test]
fn should_define_a_lifted_specialization_slice_typedef() {
    let source = r#"module spec.cc.impl.lambdaspec;

func makeSize<T>(): func(): usize {
    return () => sizeof(T[]);
}

func main(): i32 {
    let size: func(): usize = makeSize<i32>();
    if (size() >= 8) {
        return 0;
    }
    return 1;
}
"#;
    let emitted = emit_host("lambdaspec", source);
    assert_contains("lambdaspec",
        &emitted,
        "typedef struct { i32 *ptr; usize len; } ClSlice_i32;",
    );
    assert_contains("lambdaspec", &emitted, "static usize cl_lambda_0__i32(void)");
    assert_contains("lambdaspec", &emitted, "return sizeof(ClSlice_i32);");
}

/// A generic struct-instance element declares its instance first.
///
/// The slice typedef names the instance type, so the instance's own typedef
/// precedes the view typedef that references it.
#[test]
fn should_declare_a_generic_struct_instance_before_its_slice_typedef() {
    let source = r#"module spec.cc.impl.structelement;

struct Wrap<T> {
    value: T;
}

func sizeOf<T>(): usize {
    return sizeof(T[]);
}

func main(): i32 {
    if (sizeOf<Wrap<i32>>() >= 8) {
        return 0;
    }
    return 1;
}
"#;
    let emitted = emit_host("structelement", source);
    let instance = emitted
        .find("typedef struct ClWrap__i32 ClWrap__i32;")
        .expect("structelement: the instance typedef is missing");
    let slice = emitted
        .find("typedef struct { ClWrap__i32 *ptr; usize len; } ClSlice_Wrap__i32;")
        .expect("structelement: the slice typedef is missing");
    assert!(
        instance < slice,
        "structelement: the instance typedef must precede the slice typedef"
    );
    assert_contains("structelement", &emitted, "return sizeof(ClSlice_Wrap__i32);");
}

/// An owning index base keeps its single call and release.
///
/// The call-result fixed array behind the run-time index stores once and its
/// elements release at the init function's end, exactly as before the fix.
#[test]
fn should_release_an_owning_index_base_once() {
    let source = r#"module spec.cc.impl.initownbase;

import { stringOf } from std.text;

let position: usize = 0;
let calls: i32 = 0;

func mk(): String[2] {
    calls = calls + 1;
    return [stringOf("ab"), stringOf("cd")];
}

let n: usize = len(mk()[position]);

func main(): i32 {
    return calls * 10 + i32(n);
}
"#;
    let emitted = emit_host("initownbase", source);
    assert_eq!(
        count(&emitted, "cl_spec_cc_impl_initownbase_mk(__cl_itmp0)"),
        1
    );
    assert_contains("initownbase", &emitted, "FreeString(__cl_itmp0[0])");
    assert_contains("initownbase", &emitted, "FreeString(__cl_itmp0[1])");
}
