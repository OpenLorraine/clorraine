//! Runnable specification tests for the v1.1 language surfaces.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary.
//! Accepted programs are built with the host C compiler under the fixed
//! warning flags and run; rejected programs assert the diagnostic code and
//! that no translation unit was written. The dedicated memory suite owns the
//! AddressSanitizer proof of the drop paths.
//!
//! The suite grows with the execution plan: this first group pins the `Box<T>`
//! type model and its boundary diagnostics.

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
            "clc-v11spec-{}-{tag}-{sequence}",
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

/// Compiles `source` for the given profile and returns the scratch directory,
/// the compiler output, and the emitted-C path.
fn compile_only(tag: &str, profile: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    (scratch, output, c_path)
}

/// The diagnostic codes in the compiler's stderr, in order.
fn error_codes(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("error ")?;
            let code = rest.split(':').next()?;
            code.starts_with('E').then(|| code.to_owned())
        })
        .collect()
}

/// Asserts a source is rejected with `code` and writes no C.
fn assert_rejected(tag: &str, source: &str, code: &str) {
    let (scratch, output, c_path) = compile_only(tag, "host", source);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected rejection, but the compiler accepted it"
    );
    let codes = error_codes(&output);
    assert!(
        codes.iter().any(|found| found == code),
        "{tag}: expected {code}, got {codes:?}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not write C at {}",
        c_path.display()
    );
    drop(scratch);
}

/// Asserts a source is accepted and writes a C translation unit that builds
/// under the fixed strict host flags.
fn assert_accepted(tag: &str, source: &str) {
    let (scratch, output, c_path) = compile_only(tag, "host", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected acceptance; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        c_path.exists(),
        "{tag}: an accepted program must write the translation unit"
    );
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
        ])
        .arg(&program)
        .arg(&c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the strict host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    drop(scratch);
}

/// Compiles, builds, and runs an accepted program under the strict host
/// flags. The dedicated memory suite owns the AddressSanitizer child build.
fn run_host(tag: &str, source: &str) -> Output {
    let (scratch, output, c_path) = compile_only(tag, "host", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected acceptance; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
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
        ])
        .arg(&program)
        .arg(&c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the strict host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let result = Command::new(&program)
        .output()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"));
    drop(scratch);
    result
}

/// Asserts a program runs and exits zero.
fn assert_run_ok(tag: &str, source: &str) {
    let output = run_host(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A minimal program wrapper around a `Box` declaration.
fn program(items: &str) -> String {
    format!("module spec.v11;\n{items}\nfunc main(): i32 {{ return 0; }}\n")
}

/// A wrapper whose entry calls `demo`, for behavior cases.
fn run_program(items: &str) -> String {
    format!("module spec.v11;\n{items}\nfunc main(): i32 {{ return demo(); }}\n")
}

// ---------------------------------------------------------------------------
// The Box type is spellable as a type-only built-in.
// ---------------------------------------------------------------------------

/// A Box field, a `sizeof`, and an `offsetof` resolve without a declaration.
#[test]
fn a_box_type_is_a_builtin_type() {
    assert_accepted(
        "box-builtin",
        &program(
            "struct Holder { b: Box<i32>; n: i32; }\n\
             const WIDTH: usize = sizeof(Box<i32>);\n\
             const OFF: usize = offsetof(Holder, b);\n\
             const N: usize = sizeof(Holder);",
        ),
    );
}

/// A nullable Box slot is accepted.
#[test]
fn a_nullable_box_is_spellable() {
    assert_accepted(
        "box-nullable",
        &program("func accept(b: Box<i32>?): i32 { return 0; }"),
    );
}

/// `Box` with no type argument is the ordinary arity error.
#[test]
fn box_without_a_type_argument_is_rejected() {
    assert_rejected(
        "box-arity-zero",
        &program("struct Holder { b: Box; }"),
        "E10300",
    );
}

/// `Box` with two type arguments is the ordinary arity error.
#[test]
fn box_with_two_type_arguments_is_rejected() {
    assert_rejected(
        "box-arity-two",
        &program("struct Holder { b: Box<i32, u8>; }"),
        "E10300",
    );
}

/// A user declaration named `Box` shadows the built-in, like `String`.
#[test]
fn a_user_box_declaration_shadows_the_builtin() {
    assert_accepted(
        "box-shadow",
        &program(
            "struct Box<T> { value: T; }\n\
             func demo(): i32 {\n\
                 let b = Box<i32> { value: 1 };\n\
                 return b.value - 1;\n\
             }",
        ),
    );
}

/// A Box value is not fabricated by a struct-literal spelling.
#[test]
fn a_box_struct_literal_is_rejected() {
    assert_rejected(
        "box-literal",
        &program("func demo(): i32 { let b: Box<i32> = Box<i32> {}; return 0; }"),
        "E10203",
    );
}

/// A Box value is not fabricated by a call-shaped conversion.
#[test]
fn a_box_call_conversion_is_rejected() {
    assert_rejected(
        "box-call",
        &program("func demo(p: raw i32): i32 { let b: Box<i32> = Box<i32>(p); return 0; }"),
        "E10201",
    );
}

/// The pointee pointer is reserved to the standard library.
#[test]
fn a_raw_cast_of_a_box_outside_the_library_is_rejected() {
    assert_rejected(
        "box-raw-cast",
        &program(
            "func demo(b: Box<i32>): i32 { unsafe { let p: raw i32 = raw i32(b); return *p; } }",
        ),
        "E10201",
    );
}

/// A non-static view cannot be boxed.
#[test]
fn a_view_typed_box_element_is_rejected() {
    assert_rejected(
        "box-view",
        &program("struct Holder { b: Box<str>; }"),
        "E10227",
    );
}

/// A container of views cannot be boxed either.
#[test]
fn a_boxed_container_of_views_is_rejected() {
    assert_rejected(
        "box-array-view",
        &program(
            "import { Array } from std.mem;\n\
             struct Holder { b: Box<Array<str>>; }",
        ),
        "E10227",
    );
}

/// A static literal view is safe to box and stays accepted.
#[test]
fn a_static_str_box_element_is_accepted() {
    assert_accepted(
        "box-static-str",
        &program("struct Holder { b: Box<static str>; }"),
    );
}

/// An overlay cannot be boxed.
#[test]
fn a_boxed_overlay_is_rejected() {
    assert_rejected(
        "box-overlay",
        &program(
            "struct Regs { dmacon: volatile u16; }\n\
             struct Holder { b: Box<Regs>; }",
        ),
        "E10207",
    );
}

/// A Box cannot cross the FFI boundary.
#[test]
fn a_box_in_an_ffi_signature_is_rejected() {
    assert_rejected(
        "box-ffi",
        &program("ffi lib { func takeBox(take b: Box<i32>): void; }"),
        "E10105",
    );
}

/// A generic aggregate still cannot cross the FFI boundary.
#[test]
fn a_generic_aggregate_in_an_ffi_signature_is_rejected() {
    assert_rejected(
        "wrap-ffi",
        &program(
            "export struct Wrap<T> { v: T; }\n\
             ffi lib { func takeWrap(take w: Wrap<i32>): void; }",
        ),
        "E10105",
    );
}

/// A Box nested in a callback type is also an FFI crossing.
#[test]
fn a_box_inside_an_ffi_callback_is_rejected() {
    assert_rejected(
        "box-ffi-callback",
        &program("ffi lib { func each(cb: func(Box<i32>): void): void; }"),
        "E10105",
    );
}

/// A freestanding program cannot use Box: the module and allocator are absent.
#[test]
fn a_freestanding_box_is_rejected() {
    let (scratch, output, c_path) = compile_only(
        "box-freestanding",
        "freestanding",
        &program("struct Holder { b: Box<i32>; }"),
    );
    let codes = error_codes(&output);
    assert!(
        codes.iter().any(|code| code == "E11200"),
        "expected E11200, got {codes:?}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!c_path.exists(), "a rejected program must not write C");
    drop(scratch);
}

/// A Box is move-only: two uses of a moved parameter report E10020.
#[test]
fn a_box_is_move_only() {
    assert_rejected(
        "box-move",
        &program(
            "func demo(take b: Box<i32>): i32 {\n\
                 let c = b;\n\
                 let d = b;\n\
                 return 0;\n\
             }",
        ),
        "E10020",
    );
}

/// A member read on a Box is the ordinary unknown-member error.
#[test]
fn a_box_member_read_is_rejected() {
    assert_rejected(
        "box-member",
        &program("func demo(b: Box<i32>): i32 { return b.value; }"),
        "E10212",
    );
}

/// Indexing a Box is the ordinary not-indexable error.
#[test]
fn a_box_index_is_rejected() {
    assert_rejected(
        "box-index",
        &program("func demo(b: Box<i32>): i32 { return b[0]; }"),
        "E10209",
    );
}

/// Calling a Box is the ordinary not-callable error.
#[test]
fn a_box_call_is_rejected() {
    assert_rejected(
        "box-callable",
        &program("func demo(b: Box<i32>): i32 { return b(); }"),
        "E10309",
    );
}

/// `null` in a non-nullable Box slot is E10012.
#[test]
fn a_null_in_a_non_nullable_box_is_rejected() {
    assert_rejected(
        "box-null",
        &program("func demo(): i32 { let b: Box<i32> = null; return 0; }"),
        "E10012",
    );
}

/// A possibly-null Box used without narrowing is E10010.
#[test]
fn a_possibly_null_box_needs_narrowing() {
    assert_rejected(
        "box-narrow",
        &program(
            "func demo(b: Box<i32>?): i32 {\n\
                 let c: Box<i32> = b;\n\
                 return 0;\n\
             }",
        ),
        "E10010",
    );
}

// ---------------------------------------------------------------------------
// Review-fix cases: the built-in is a type, shadowing is real, and an element
// rule survives generic substitution.
// ---------------------------------------------------------------------------

/// A bare built-in `Box` is a type, not a value.
#[test]
fn a_bare_box_value_is_rejected() {
    assert_rejected(
        "box-bare-value",
        &program("func demo(): i32 { return Box; }"),
        "E10219",
    );
}

/// A slice is a non-static view and cannot be boxed.
#[test]
fn a_slice_box_element_is_rejected() {
    assert_rejected(
        "box-slice",
        &program("struct Holder { b: Box<u8[]>; }"),
        "E10227",
    );
}

/// A generic struct instance that substitutes a view into `Box<T>` is caught.
#[test]
fn a_boxed_generic_instance_with_a_view_is_rejected() {
    assert_rejected(
        "box-generic-view",
        &program(
            "export struct Holder<T> { b: Box<T>; }\n\
             struct Top { h: Holder<str>; }",
        ),
        "E10227",
    );
}

/// A user enum named `Box` shadows the built-in, so no arity check fires.
#[test]
fn a_user_enum_box_shadows_the_builtin() {
    assert_accepted(
        "box-enum-shadow",
        &program(
            "enum Box: i32 { A }\n\
             func demo(): i32 { let b: Box = Box.A; return 0; }",
        ),
    );
}

/// A user handle named `Box` shadows the built-in, so no arity check fires.
#[test]
fn a_user_handle_box_shadows_the_builtin() {
    assert_accepted(
        "box-handle-shadow",
        &program(
            "handle Box freedBy FreeBox;\n\
             ffi lib { func FreeBox(take b: Box): void; }\n\
             func demo(b: Box?): i32 { return 0; }",
        ),
    );
}

/// A nullable element reports exactly one E10011, not one per walk.
#[test]
fn a_nullable_box_element_reports_once() {
    let (scratch, output, _c_path) = compile_only(
        "box-nullable-element",
        "host",
        &program("struct Holder { b: Box<i32?>; }"),
    );
    let count = error_codes(&output)
        .iter()
        .filter(|code| code.as_str() == "E10011")
        .count();
    assert_eq!(count, 1, "E10011 must be reported exactly once");
    drop(scratch);
}

/// A call-shaped `Box` with the wrong arity reports E10300 as well.
#[test]
fn a_box_call_with_two_type_arguments_reports_arity() {
    let (scratch, output, _c_path) = compile_only(
        "box-call-arity",
        "host",
        &program("func demo(p: raw i32): i32 { let b: Box<i32, u8> = Box<i32, u8>(p); return 0; }"),
    );
    let codes = error_codes(&output);
    assert!(
        codes.iter().any(|code| code == "E10300"),
        "expected E10300, got {codes:?}"
    );
    drop(scratch);
}

/// A function with a Box parameter builds under the strict host flags.
#[test]
fn a_box_parameter_builds_strictly() {
    assert_accepted(
        "box-param-build",
        &program("func demo(take b: Box<i32>): void { }"),
    );
}

/// A nullable Box compares with `null` and narrows like a handle.
#[test]
fn a_nullable_box_compares_with_null() {
    assert_accepted(
        "box-null-compare",
        &program(
            "func demo(b: Box<i32>?): i32 {\n\
                 if (b == null) { return 1; }\n\
                 return 0;\n\
             }",
        ),
    );
}

// ---------------------------------------------------------------------------
// Box allocation, access, and recursive drop.
// ---------------------------------------------------------------------------

/// A scalar box is allocated, read through `boxWith`, and dropped.
#[test]
fn a_box_holds_and_reads_a_scalar() {
    assert_run_ok(
        "box-scalar",
        &run_program(
            "import { boxNew, boxWith } from std.mem;\n\
             func demo(): i32 {\n\
                 let b = boxNew<i32>(7);\n\
                 let n: i32 = 0;\n\
                 boxWith(b, [borrow n] (v: i32) => { n = v; });\n\
                 return n - 7;\n\
             }",
        ),
    );
}

/// A boxed String is read through the callback and released on exit.
#[test]
fn a_boxed_string_reads_through_the_callback() {
    assert_run_ok(
        "box-string",
        &run_program(
            "import { boxNew, boxWith } from std.mem;\n\
             import { String, stringOf, bytes } from std.text;\n\
             func demo(): i32 {\n\
                 let b = boxNew<String>(stringOf(\"hi\"));\n\
                 let n: i32 = 0;\n\
                 boxWith(b, [borrow n] (v: String) => { n = i32(len(bytes(v))); });\n\
                 return n - 2;\n\
             }",
        ),
    );
}

/// `boxSet` drops the old String and installs the new one.
#[test]
fn a_box_set_replaces_a_string() {
    assert_run_ok(
        "box-set",
        &run_program(
            "import { boxNew, boxSet, boxWith } from std.mem;\n\
             import { String, stringOf, bytes } from std.text;\n\
             func demo(): i32 {\n\
                 let b = boxNew<String>(stringOf(\"ab\"));\n\
                 boxSet(b, stringOf(\"cde\"));\n\
                 let n: i32 = 0;\n\
                 boxWith(b, [borrow n] (v: String) => { n = i32(len(bytes(v))); });\n\
                 return n - 3;\n\
             }",
        ),
    );
}

/// Reassigning a box releases the old block and its pointee.
#[test]
fn a_box_reassignment_releases_the_old_value() {
    assert_run_ok(
        "box-reassign",
        &run_program(
            "import { boxNew } from std.mem;\n\
             import { String, stringOf } from std.text;\n\
             func demo(): i32 {\n\
                 let b = boxNew<String>(stringOf(\"aa\"));\n\
                 b = boxNew<String>(stringOf(\"bb\"));\n\
                 return 0;\n\
             }",
        ),
    );
}

/// A box of a box drops the inner block and its String.
#[test]
fn a_nested_box_drops_recursively() {
    assert_run_ok(
        "box-nested",
        &run_program(
            "import { boxNew } from std.mem;\n\
             import { String, stringOf } from std.text;\n\
             func demo(): i32 {\n\
                 let inner = boxNew<String>(stringOf(\"x\"));\n\
                 let outer = boxNew<Box<String>>(inner);\n\
                 return 0;\n\
             }",
        ),
    );
}

/// A box of a growable array releases its elements before the buffer.
#[test]
fn a_boxed_array_drops_its_elements() {
    assert_run_ok(
        "box-array",
        &run_program(
            "import { boxNew, arrayNew, push, Array } from std.mem;\n\
             import { String, stringOf } from std.text;\n\
             func demo(): i32 {\n\
                 let xs = arrayNew<String>();\n\
                 push<String>(xs, stringOf(\"a\"));\n\
                 push<String>(xs, stringOf(\"b\"));\n\
                 let b = boxNew<Array<String>>(xs);\n\
                 return 0;\n\
             }",
        ),
    );
}

/// A recursive linked list of boxes drops from the head through every link.
#[test]
fn a_recursive_box_list_drops() {
    assert_run_ok(
        "box-list",
        &run_program(
            "import { boxNew } from std.mem;\n\
             struct Node { value: i32; next: Box<Node>?; }\n\
             func demo(): i32 {\n\
                 let c = Node { value: 3, next: null };\n\
                 let b = boxNew<Node>(c);\n\
                 let a = boxNew<Node>(Node { value: 2, next: b });\n\
                 let root = boxNew<Node>(Node { value: 1, next: a });\n\
                 return 0;\n\
             }",
        ),
    );
}

/// A box created in a loop and a defer drops each iteration's value.
#[test]
fn a_box_in_a_loop_and_a_defer_drops() {
    assert_run_ok(
        "box-loop-defer",
        &run_program(
            "import { boxNew } from std.mem;\n\
             func demo(): i32 {\n\
                 let i: i32 = 0;\n\
                 while (i < 3) {\n\
                     let b = boxNew<i32>(i);\n\
                     i = i + 1;\n\
                 }\n\
                 defer {\n\
                     let d = boxNew<i32>(9);\n\
                 }\n\
                 return 0;\n\
             }",
        ),
    );
}

/// A null box drops as a no-op.
#[test]
fn a_null_box_drops_as_a_noop() {
    assert_run_ok(
        "box-null-drop",
        &run_program(
            "func demo(): i32 {\n\
                 let b: Box<i32>? = null;\n\
                 return 0;\n\
             }",
        ),
    );
}

/// A possibly-null box cannot be passed to `boxWith` without narrowing.
#[test]
fn a_nullable_box_argument_needs_narrowing() {
    assert_rejected(
        "box-nullable-call",
        &program(
            "import { boxWith } from std.mem;\n\
             func demo(b: Box<i32>?): i32 {\n\
                 let n: i32 = 0;\n\
                 boxWith(b, [borrow n] (v: i32) => { n = v; });\n\
                 return n;\n\
             }",
        ),
        "E10010",
    );
}
