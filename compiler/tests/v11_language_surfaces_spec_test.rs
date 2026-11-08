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

/// Asserts a source compiles and writes C, without linking the unit.
///
/// A hook case names an `ffi` function whose C implementation belongs to a
/// later fixture, so the compiler stage is the whole judgment here.
fn assert_accepts_source(tag: &str, profile: &str, source: &str) {
    let (scratch, output, c_path) = compile_only(tag, profile, source);
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
    drop(scratch);
}

/// Asserts a source is rejected with `code` on the given profile.
fn assert_rejected_profile(tag: &str, profile: &str, source: &str, code: &str) {
    let (scratch, output, c_path) = compile_only(tag, profile, source);
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

/// Compiles two source files and returns the scratch directory, output, and C.
fn compile_two(tag: &str, first: &str, second: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let first_path = scratch.path().join("a.clo");
    let second_path = scratch.path().join("b.clo");
    std::fs::write(&first_path, first).expect("write the first module");
    std::fs::write(&second_path, second).expect("write the second module");
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        first_path.display().to_string(),
        second_path.display().to_string(),
    ]);
    (scratch, output, c_path)
}

/// Asserts two modules are accepted and their emitted C builds strictly.
fn assert_accepted_two(tag: &str, first: &str, second: &str) {
    let (scratch, output, c_path) = compile_two(tag, first, second);
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
    drop(scratch);
}

/// Asserts two modules are rejected with `code`.
fn assert_rejected_two(tag: &str, first: &str, second: &str, code: &str) {
    let (scratch, output, c_path) = compile_two(tag, first, second);
    let codes = error_codes(&output);
    assert!(
        codes.iter().any(|found| found == code),
        "{tag}: expected {code}, got {codes:?}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!c_path.exists(), "a rejected program must not write C");
    drop(scratch);
}

/// Compiles, builds with a C fixture, and runs a program.
fn run_host_fixture(tag: &str, source: &str, fixture: &str) -> Output {
    let (scratch, output, c_path) = compile_only(tag, "host", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected acceptance; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let fixture_path = scratch.path().join("fixture.c");
    std::fs::write(&fixture_path, fixture).expect("write the C fixture");
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
        .arg(&fixture_path)
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

// ---------------------------------------------------------------------------
// X1: ordinary Clorraine freedBy deleters.
// ---------------------------------------------------------------------------

/// A same-module ordinary `take T -> void` function is a valid deleter and
/// releases the handle exactly once at scope exit.
#[test]
fn a_same_module_ordinary_deleter_is_accepted() {
    let output = run_host_fixture(
        "x1-same-module",
        r#"module spec.v11;
import { String, stringOf } from std.text;
handle Widget freedBy freeWidget;
ffi lib {
    func cl_widget_new(): Widget;
    func cl_widget_free(take w: Widget): void;
}
func freeWidget(take w: Widget): void {
    let s = stringOf("x");
    cl_widget_free(w);
}
func demo(): i32 {
    let w = cl_widget_new();
    return 0;
}
func main(): i32 { return demo(); }
"#,
        r#"#include <stdlib.h>
void *cl_widget_new(void) { return malloc(16); }
void cl_widget_free(void *p) { free(p); }
"#,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the forwarding deleter exited {:?}; stderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// An exported cross-module ordinary deleter resolves through its module.
#[test]
fn a_cross_module_exported_deleter_is_accepted() {
    let (scratch, output, c_path) = compile_two(
        "x1-cross-module",
        "module a;\nexport handle Widget freedBy b.freeWidget;\nfunc main(): i32 { return 0; }\n",
        "module b;\nffi lib { func cl_free_widget(take w: a.Widget): void; }\nexport func freeWidget(take w: a.Widget): void { cl_free_widget(w); }\n",
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the cross-module deleter was rejected; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let c = std::fs::read_to_string(&c_path).expect("read the emitted C");
    assert!(c.contains("cl_b_freeWidget"), "the deleter symbol is missing; emitted:\n{c}");
    drop(scratch);
}

/// A simple imported deleter name resolves in the handle's module scope.
#[test]
fn a_cross_module_imported_deleter_is_accepted() {
    let (scratch, output, c_path) = compile_two(
        "x1-imported",
        "module a;\nimport { freeWidget } from b;\nexport handle Widget freedBy freeWidget;\nfunc main(): i32 { return 0; }\n",
        "module b;\nffi lib { func cl_free_widget(take w: a.Widget): void; }\nexport func freeWidget(take w: a.Widget): void { cl_free_widget(w); }\n",
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the imported deleter was rejected; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let c = std::fs::read_to_string(&c_path).expect("read the emitted C");
    assert!(c.contains("cl_b_freeWidget"), "the deleter symbol is missing; emitted:\n{c}");
    drop(scratch);
}

/// A wrong mode is E10102.
#[test]
fn a_deleter_with_the_wrong_mode_is_rejected() {
    assert_rejected(
        "x1-wrong-mode",
        &program(
            "handle Widget freedBy freeWidget;\n\
             func freeWidget(w: Widget): void { }",
        ),
        "E10102",
    );
}

/// A non-void return is E10102.
#[test]
fn a_deleter_with_a_non_void_return_is_rejected() {
    assert_rejected(
        "x1-return",
        &program(
            "handle Widget freedBy freeWidget;\n\
             func freeWidget(take w: Widget): i32 { return 0; }",
        ),
        "E10102",
    );
}

/// A wrong parameter type is E10102.
#[test]
fn a_deleter_with_the_wrong_parameter_type_is_rejected() {
    assert_rejected(
        "x1-param",
        &program(
            "handle Widget freedBy freeWidget;\n\
             func freeWidget(take w: i32): void { }",
        ),
        "E10102",
    );
}

/// An unresolved deleter name is E10102.
#[test]
fn a_missing_deleter_is_rejected() {
    assert_rejected(
        "x1-missing",
        &program("handle Widget freedBy missing;"),
        "E10102",
    );
}

/// An intrinsic is not a user deleter.
#[test]
fn an_intrinsic_deleter_is_rejected() {
    assert_rejected(
        "x1-intrinsic",
        &program("handle Widget freedBy rawStr;"),
        "E10102",
    );
}

/// A generic function is not a deleter in v1.1.
#[test]
fn a_generic_deleter_is_rejected() {
    assert_rejected(
        "x1-generic",
        &program(
            "handle Widget freedBy freeWidget;\n\
             func freeWidget<T>(take w: Widget): void { }",
        ),
        "E10102",
    );
}

/// Two modules may not declare the same handle.
#[test]
fn a_cross_module_handle_duplicate_is_rejected() {
    assert_rejected_two(
        "x1-duplicate",
        "module a;\nexport handle Widget freedBy b.freeWidget;\nfunc main(): i32 { return 0; }\n",
        "module b;\nexport handle Widget freedBy b.freeWidget;\nexport func freeWidget(take w: a.Widget): void { }\n",
        "E11101",
    );
}

/// A deleter body may drop an owned field through the opaque handle.
#[test]
fn an_owned_field_deleter_releases_once() {
    let output = run_host_fixture(
        "x1-owned-field",
        r#"module spec.v11;
import { String, stringOf } from std.text;
handle Widget freedBy freeWidget;
ffi lib {
    func cl_widget_new(): Widget;
    func cl_widget_free(take w: Widget): void;
}
struct Payload { s: String; }
func freeWidget(take w: Widget): void {
    unsafe {
        let payload: Payload = *raw Payload(w);
    }
    cl_widget_free(w);
}
func demo(): i32 {
    let w = cl_widget_new();
    unsafe {
        *raw Payload(w) = Payload { s: stringOf("owned") };
    }
    return 0;
}
func main(): i32 { return demo(); }
"#,
        r#"#include <stdlib.h>
#include <string.h>
void *cl_widget_new(void) { return calloc(1, 64); }
void cl_widget_free(void *p) { free(p); }
"#,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the owned-field deleter exited {:?}; stderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A deleter body that never releases its `take` parameter is rejected.
#[test]
fn a_non_consuming_deleter_is_rejected() {
    assert_rejected(
        "x1-non-consuming",
        &program(
            "handle Widget freedBy freeWidget;\n\
             func freeWidget(take w: Widget): void { }",
        ),
        "E10102",
    );
}

/// A private cross-module function is not a deleter.
#[test]
fn a_private_cross_module_deleter_is_rejected() {
    let (scratch, output, _c_path) = compile_two(
        "x1-private",
        "module a;\nexport handle Widget freedBy b.freeWidget;\nfunc main(): i32 { return 0; }\n",
        "module b;\nfunc freeWidget(take w: a.Widget): void { }\n",
    );
    let codes = error_codes(&output);
    assert!(
        codes.iter().any(|code| code == "E10102"),
        "expected E10102, got {codes:?}"
    );
    drop(scratch);
}

/// A same-module declaration of another kind shadows a program-wide function.
#[test]
fn a_non_function_declaration_shadows_the_deleter() {
    assert_rejected(
        "x1-struct-shadow",
        &program(
            "struct freeWidget { x: i32; }\n\
             handle Widget freedBy freeWidget;",
        ),
        "E10102",
    );
}

/// A user `handle String` is a duplicate of the predeclared handle.
#[test]
fn a_user_string_handle_is_rejected() {
    assert_rejected(
        "x1-string-duplicate",
        &program(
            "ffi lib { func freeS(take s: String): void; }\n\
             handle String freedBy freeS;",
        ),
        "E11101",
    );
}

/// A handle remains move-only under an ordinary deleter.
#[test]
fn an_ordinary_deleter_keeps_the_handle_move_only() {
    assert_rejected(
        "x1-move-only",
        &program(
            "handle Widget freedBy freeWidget;\n\
             ffi lib { func cl_free_widget(take w: Widget): void; }\n\
             func freeWidget(take w: Widget): void { cl_free_widget(w); }\n\
             func demo(take w: Widget): void { let a = w; let b = w; }",
        ),
        "E10020",
    );
}

// ---------------------------------------------------------------------------
// A synchronous C hook is a `borrow`-mode `func(...)` FFI parameter.
// ---------------------------------------------------------------------------

/// A capture-less lambda is a plain function pointer and crosses the hook.
#[test]
fn a_captureless_lambda_crosses_a_borrow_hook() {
    assert_accepts_source(
        "hk-captureless",
        "host",
        &program(
            "ffi lib { func apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 { return apply((x: i32) => x + 1, 1); }",
        ),
    );
}

/// A capturing lambda is rejected until the trampoline path exists.
#[test]
fn a_capturing_hook_lambda_is_rejected_until_trampolines() {
    for (tag, marker) in [
        ("unmarked", ""),
        ("const", "const "),
        ("take", "take "),
        ("borrow", "borrow "),
    ] {
        let source = program(&format!(
            "ffi lib {{ func apply({marker}cb: func(i32): i32, value: i32): i32; }}\n\
             func demo(): i32 {{ let n: i32 = 1; return apply([n] (x: i32) => x + n, 1); }}"
        ));
        assert_rejected(&format!("hk-capture-{tag}"), &source, "E10400");
    }
}

/// A user `func(...)` parameter keeps rejecting a capturing lambda.
#[test]
fn a_capturing_lambda_at_a_user_func_parameter_is_rejected() {
    assert_rejected(
        "hk-user-func",
        &program(
            "func invoke(f: func(i32): i32): i32 { return f(1); }\n\
             func demo(): i32 { let n: i32 = 1; return invoke([n] (x: i32) => x + n); }",
        ),
        "E10400",
    );
}

/// A standard-library API that stores the callback rejects a capturing lambda.
#[test]
fn a_stored_callback_api_rejects_a_capturing_lambda() {
    assert_rejected_profile(
        "hk-stored-newhook",
        "hosted-amiga",
        &program(
            "import { newHook } from std.amiga.utility;\n\
             func demo(): i32 { let n: i32 = 1; let h = newHook([n] () => { if (n == 0) { } }, 7); return 0; }",
        ),
        "E10400",
    );
    assert_rejected_profile(
        "hk-stored-addhandler",
        "hosted-amiga",
        &program(
            "import { addHandler } from std.amiga.interrupt;\n\
             func demo(): i32 { let n: i32 = 1; let t = addHandler([n] () => { if (n == 0) { } }); return 0; }",
        ),
        "E10400",
    );
    assert_rejected_profile(
        "hk-stored-createtask",
        "hosted-amiga",
        &program(
            "import { createTask } from std.amiga.exec;\n\
             func demo(): i32 { let n: i32 = 1; let t = createTask([n] () => { if (n == 0) { } }, 4096); return 0; }",
        ),
        "E10400",
    );
}

/// A callback whose written signature differs from the hook is rejected.
#[test]
fn a_hook_signature_mismatch_is_rejected() {
    assert_rejected(
        "hk-mismatch",
        &program(
            "ffi lib { func apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 { return apply((x: u8) => 0, 1); }",
        ),
        "E10210",
    );
}

/// A value that is not a function is rejected at a callback parameter.
#[test]
fn a_non_callable_hook_argument_is_rejected() {
    assert_rejected(
        "hk-non-callable",
        &program(
            "ffi lib { func apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 { return apply(1, 1); }",
        ),
        "E10200",
    );
}

/// A hook call in a short-circuit operand remains an ordinary call.
#[test]
fn a_captureless_hook_in_a_short_circuit_is_accepted() {
    assert_accepts_source(
        "hk-short-circuit",
        "host",
        &program(
            "ffi lib { func check(borrow cb: func(): bool): bool; }\n\
             func demo(): i32 { if (check(() => true) && check(() => false)) { return 1; } return 0; }",
        ),
    );
}

/// A capture-less hook inside a generic body is a plain pointer.
#[test]
fn a_captureless_hook_in_a_generic_body_is_accepted() {
    assert_accepts_source(
        "hk-generic-captureless",
        "host",
        &program(
            "ffi lib { func apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func run<T>(x: T): i32 { return apply((v: i32) => v + 1, 1); }",
        ),
    );
}

/// A capturing hook inside a generic body needs a per-instance trampoline.
#[test]
fn a_capturing_hook_in_a_generic_body_is_rejected() {
    assert_rejected(
        "hk-generic-capture",
        &program(
            "ffi lib { func apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func run<T>(x: T): i32 { let n: i32 = 1; return apply([n] (v: i32) => v + n, 1); }",
        ),
        "E10400",
    );
}

/// A view parameter is not ABI-identical to the lifted lambda.
#[test]
fn a_hook_callback_with_a_view_parameter_is_rejected() {
    assert_rejected(
        "hk-view-param",
        &program(
            "ffi lib { func apply(borrow cb: func(str): i32): i32; }\n\
             func demo(): i32 { return apply((s: str) => 0); }",
        ),
        "E10210",
    );
}

/// A slice parameter is not ABI-identical to the lifted lambda.
#[test]
fn a_hook_callback_with_a_slice_parameter_is_rejected() {
    assert_rejected(
        "hk-slice-param",
        &program(
            "ffi lib { func apply(borrow cb: func(i32[]): i32): i32; }\n\
             func demo(): i32 { return apply((xs: i32[]) => 0); }",
        ),
        "E10210",
    );
}

/// A fixed-array parameter is not ABI-identical to the lifted lambda.
#[test]
fn a_hook_callback_with_a_fixed_array_parameter_is_rejected() {
    assert_rejected(
        "hk-array-param",
        &program(
            "ffi lib { func apply(borrow cb: func(i32[2]): i32): i32; }\n\
             func demo(): i32 { return apply((xs: i32[2]) => 0); }",
        ),
        "E10210",
    );
}

/// A fixed-array return is not ABI-identical to the lifted lambda.
#[test]
fn a_hook_callback_with_a_fixed_array_return_is_rejected() {
    assert_rejected(
        "hk-array-return",
        &program(
            "ffi lib { func apply(borrow cb: func(): i32[2]): i32; }\n\
             func demo(): i32 { return apply(() => [1, 2]); }",
        ),
        "E10210",
    );
}

/// A borrowed move-only struct is a pointer in the language and a value in C.
#[test]
fn a_hook_callback_borrowing_a_move_only_struct_is_rejected() {
    assert_rejected(
        "hk-borrow-owned",
        &program(
            "struct Owned { s: String; }\n\
             ffi lib { func apply(borrow cb: func(borrow Owned): i32): i32; }\n\
             func peek(borrow o: Owned): i32 { return 0; }\n\
             func demo(): i32 { return apply(peek); }",
        ),
        "E10210",
    );
}

/// The ABI-identical scalar, enum, struct, and function-pointer set is accepted.
#[test]
fn a_hook_callback_with_abi_identical_types_is_accepted() {
    assert_accepts_source(
        "hk-abi-set",
        "host",
        &program(
            "struct Point { x: i32; y: i32; }\n\
             enum Color: u8 { Red, Green }\n\
             ffi lib { func apply(borrow cb: func(i32, bool, cstring, raw u8, Point, Color, func(i32): i32): i32): i32; }\n\
             func demo(): i32 { return apply((a: i32, b: bool, c: cstring, d: raw u8, p: Point, col: Color, f: func(i32): i32) => 0); }",
        ),
    );
}

/// A `[borrow x]` capture at a hook is a capturing hook, not a plain pointer.
#[test]
fn a_borrow_capture_at_a_hook_is_rejected_until_trampolines() {
    assert_rejected(
        "hk-borrow-capture",
        &program(
            "ffi lib { func apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 {\n\
                 let n: i32 = 1;\n\
                 return apply([borrow n] (x: i32) => { n = n + 1; return x + n; }, 1);\n\
             }",
        ),
        "E10400",
    );
}

/// A capturing lambda at a callable-bound generic parameter is not a C hook.
#[test]
fn a_capturing_lambda_at_a_callable_bound_is_still_accepted() {
    assert_run_ok(
        "hk-callable-bound",
        &run_program(
            "func invoke<T, F: func(T): T>(x: T, f: F): T { return f(x); }\n\
             func demo(): i32 {\n\
                 let n: i32 = 10;\n\
                 if (invoke(2, [n] (x: i32) => x + n) == 12) { return 0; }\n\
                 return 1;\n\
             }",
        ),
    );
}
