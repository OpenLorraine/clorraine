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

/// Asserts a source compiles and its emitted C builds strictly (no link).
fn assert_accepted_c(tag: &str, source: &str) {
    let (scratch, output, c_path) = compile_only(tag, "host", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected acceptance; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let object = scratch.path().join("out.o");
    let built = Command::new("cc")
        .args(["-std=c99", "-O0", "-Wall", "-Wextra", "-Werror", "-c"])
        .arg(&c_path)
        .arg("-o")
        .arg(&object)
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
}/// A wrapper whose entry calls `demo`, for behavior cases.
fn run_program(items: &str) -> String {
    format!("module spec.v11;\n{items}\nfunc main(): i32 {{ return demo(); }}\n")
}

/// The synchronous C hook fixture linked by the host behavior cases.
const HOOK_FIXTURE: &str = include_str!("fixtures/hook_fixture_host.c");

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

/// A generic cast is judged at each explicit instantiation, so a Box argument
/// reports the reserved pointee-pointer rule.
#[test]
fn an_explicit_generic_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-explicit",
        &program(
            "func leak<T>(x: T): raw i32 {\n\
                 unsafe { return raw i32(x); }\n\
             }\n\
             func demo(b: Box<i32>): raw i32 { return leak<Box<i32>>(b); }",
        ),
        "E10201",
    );
}

/// An inferred generic argument is judged the same way as an explicit one.
#[test]
fn an_inferred_generic_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-inferred",
        &program(
            "func leak<T>(x: T): raw i32 {\n\
                 unsafe { return raw i32(x); }\n\
             }\n\
             func demo(b: Box<i32>): raw i32 { return leak(b); }",
        ),
        "E10201",
    );
}

/// A forwarding generic call records its own instance, so it cannot hide the
/// Box argument from the cast audit.
#[test]
fn a_forwarding_generic_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-forwarding",
        &program(
            "func leak<T>(x: T): raw i32 {\n\
                 unsafe { return raw i32(x); }\n\
             }\n\
             func forward<T>(x: T): raw i32 { return leak<T>(x); }\n\
             func demo(b: Box<i32>): raw i32 { return forward(b); }",
        ),
        "E10201",
    );
}

/// A Box reached through an unresolved member chain is still a Box operand.
#[test]
fn a_member_chain_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-member-chain",
        &program(
            "struct Inner { n: Box<i32>; }\n\
             struct Holder { inner: Inner; }\n\
             func leak<T>(x: T): raw i32 {\n\
                 unsafe { return raw i32(x.inner.n); }\n\
             }\n\
             func demo(h: Holder): raw i32 { return leak<Holder>(h); }",
        ),
        "E10201",
    );
}

/// A conditional whose arms resolve to a Box cannot hide the cast operand.
#[test]
fn a_conditional_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-conditional",
        &program(
            "struct Holder { b: Box<i32>; }\n\
             func leak<T>(x: T, c: bool): raw i32 {\n\
                 unsafe { return raw i32(c ? x.b : x.b); }\n\
             }\n\
             func demo(h: Holder): raw i32 { return leak(h, true); }",
        ),
        "E10201",
    );
}

/// An index into a substituted row of Box elements is still a Box operand.
#[test]
fn an_index_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-index",
        &program(
            "struct Holder { items: Box<i32>[1]; }\n\
             func leak<T>(x: T): raw i32 {\n\
                 unsafe { return raw i32(x.items[0]); }\n\
             }\n\
             func demo(h: Holder): raw i32 { return leak(h); }",
        ),
        "E10201",
    );
}

/// A raw dereference of a substituted pointer to a Box is still a Box operand.
#[test]
fn a_dereference_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-deref",
        &program(
            "struct Holder { p: raw Box<i32>; }\n\
             func leak<T>(x: T): raw i32 {\n\
                 unsafe { return raw i32(*(x.p)); }\n\
             }\n\
             func demo(h: Holder): raw i32 { return leak(h); }",
        ),
        "E10201",
    );
}

/// A local alias of an unresolved member read keeps the operand chain.
#[test]
fn an_alias_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-alias",
        &program(
            "struct Inner { n: Box<i32>; }\n\
             struct Holder { inner: Inner; }\n\
             func leak<T>(x: T): raw i32 {\n\
                 unsafe { let y = x.inner; return raw i32(y.n); }\n\
             }\n\
             func demo(h: Holder): raw i32 { return leak(h); }",
        ),
        "E10201",
    );
}

/// A loop binding over an unresolved row keeps the iterable's element shape.
#[test]
fn a_loop_element_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-loop",
        &program(
            "struct Holder { items: Box<i32>[1]; }\n\
             func leak<T>(x: T): raw i32 {\n\
                 unsafe {\n\
                     for (v in x.items) { return raw i32(v); }\n\
                     return raw i32(0);\n\
                 }\n\
             }\n\
             func demo(h: Holder): raw i32 { return leak(h); }",
        ),
        "E10201",
    );
}

/// A lambda capture of an unresolved local keeps the captured shape.
#[test]
fn a_lambda_capture_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-capture",
        &program(
            "struct Holder { b: Box<i32>; }\n\
             func apply<F: func(): raw i32>(f: F): raw i32 { return f(); }\n\
             func leak<T>(x: T): raw i32 {\n\
                 unsafe { let y = x.b; return apply([y] () => raw i32(y)); }\n\
             }\n\
             func demo(h: Holder): raw i32 { return leak(h); }",
        ),
        "E10201",
    );
}

/// A dereference of an address-of local keeps the pointee's shape.
#[test]
fn an_address_of_deref_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-address-of",
        &program(
            "struct Inner { n: Box<i32>; }\n\
             struct Holder { inner: Inner; }\n\
             func leak<T>(x: T): raw i32 {\n\
                 unsafe {\n\
                     let p = &x.inner.n;\n\
                     return raw i32(*p);\n\
                 }\n\
             }\n\
             func demo(h: Holder): raw i32 { return leak(h); }",
        ),
        "E10201",
    );
}

/// A conditional of address-of pointers keeps both pointee shapes.
#[test]
fn a_conditional_address_of_deref_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-conditional-address-of",
        &program(
            "struct Holder { b: Box<i32>; }\n\
             func leak<T>(x: T, c: bool): raw i32 {\n\
                 unsafe {\n\
                     let q = c ? &x.b : &x.b;\n\
                     return raw i32(*q);\n\
                 }\n\
             }\n\
             func demo(h: Holder): raw i32 { return leak(h, true); }",
        ),
        "E10201",
    );
}

/// The prefix `unsafe` wrapper is transparent to the address-of shape.
#[test]
fn an_unsafe_address_of_deref_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-unsafe-address-of",
        &program(
            "struct Inner { n: Box<i32>; }\n\
             struct Holder { inner: Inner; }\n\
             func leak<T>(x: T): raw i32 {\n\
                 unsafe {\n\
                     let p = unsafe &x.inner.n;\n\
                     return raw i32(*p);\n\
                 }\n\
             }\n\
             func demo(h: Holder): raw i32 { return leak(h); }",
        ),
        "E10201",
    );
}

/// A store of a Box into an unresolved local replaces its recorded shape.
#[test]
fn a_store_raw_cast_of_a_box_is_rejected() {
    assert_rejected(
        "box-raw-cast-store",
        &program(
            "struct InnerInt { n: i32; }\n\
             struct Holder { inner: InnerInt; }\n\
             func leak<T>(x: T, box: Box<i32>): raw i32 {\n\
                 unsafe { let y = x.inner; y = box; return raw i32(y); }\n\
             }\n\
             func demo(h: Holder, b: Box<i32>): raw i32 { return leak(h, b); }",
        ),
        "E10201",
    );
}

/// A later non-Box store replaces an earlier Box-shaped origin, so the cast
/// follows the value the binding actually holds.
#[test]
fn a_non_box_store_after_a_box_shaped_initializer_is_accepted() {
    assert_accepts_source(
        "box-raw-cast-store-non-box",
        "host",
        &program(
            "struct InnerBox { n: Box<i32>; }\n\
             struct InnerInt { n: i32; }\n\
             struct Holder { inner: InnerBox; }\n\
             func leak<T>(x: T, other: InnerInt): raw i32 {\n\
                 unsafe { let y = x.inner; y = other; return raw i32(y.n); }\n\
             }\n\
             func demo(h: Holder, o: InnerInt): raw i32 { return leak(h, o); }",
        ),
    );
}

/// The same unresolved operand shapes with non-Box members stay accepted.
#[test]
fn unresolved_operand_shapes_with_non_box_fields_are_accepted() {
    assert_accepts_source(
        "box-raw-cast-shapes-non-box",
        "host",
        &program(
            "struct Inner { n: i32; }\n\
             struct Holder { b: i32; inner: Inner; items: i32[1]; p: raw i32; }\n\
             func conditional<T>(x: T, c: bool): raw i32 {\n\
                 unsafe { return raw i32(c ? x.b : x.b); }\n\
             }\n\
             func indexed<T>(x: T): raw i32 {\n\
                 unsafe { return raw i32(x.items[0]); }\n\
             }\n\
             func dereferenced<T>(x: T): raw i32 {\n\
                 unsafe { return raw i32(*(x.p)); }\n\
             }\n\
             func aliased<T>(x: T): raw i32 {\n\
                 unsafe { let y = x.inner; return raw i32(y.n); }\n\
             }\n\
             func addressed<T>(x: T): raw i32 {\n\
                 unsafe { let p = &x.inner.n; return raw i32(*p); }\n\
             }\n\
             func conditionalAddressed<T>(x: T, c: bool): raw i32 {\n\
                 unsafe { let q = c ? &x.b : &x.b; return raw i32(*q); }\n\
             }\n\
             func demo(h: Holder): i32 {\n\
                 let a: raw i32 = conditional(h, true);\n\
                 let b: raw i32 = indexed(h);\n\
                 let c: raw i32 = dereferenced(h);\n\
                 let d: raw i32 = aliased(h);\n\
                 let e: raw i32 = addressed(h);\n\
                 let f: raw i32 = conditionalAddressed(h, true);\n\
                 return 0;\n\
             }",
        ),
    );
}

/// The same generic instantiated with a non-Box type stays accepted.
///
/// The judgment is the compiler stage: a run-time integer-to-`raw` cast is a
/// separate, pre-existing emission question, so this case does not build the
/// unit.
#[test]
fn a_generic_raw_cast_of_a_non_box_is_accepted() {
    assert_accepts_source(
        "box-raw-cast-non-box",
        "host",
        &program(
            "func leak<T>(x: T): raw i32 {\n\
                 unsafe { return raw i32(x); }\n\
             }\n\
             func demo(): raw i32 { return leak<i32>(7); }",
        ),
    );
}

/// The embedded `std.mem` `boxPtr` helper keeps its pointee-pointer cast.
#[test]
fn the_standard_library_box_pointer_helper_is_accepted() {
    assert_accepted(
        "box-raw-cast-stdmem",
        &program(
            "import { boxNew, boxWith } from std.mem;\n\
             func demo(): i32 {\n\
                 let b: Box<i32> = boxNew<i32>(7);\n\
                 boxWith(b, [] (v: i32) => { });\n\
                 return 0;\n\
             }",
        ),
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

/// A capturing lambda without a synchronous `borrow` contract is rejected.
#[test]
fn a_capturing_hook_without_a_borrow_contract_is_rejected() {
    for (tag, marker) in [
        ("unmarked", ""),
        ("const", "const "),
        ("take", "take "),
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
    assert_accepted_c(
        "hk-abi-set",
        &program(
            "struct Point { x: i32; y: i32; }\n\
             enum Color: u8 { Red, Green }\n\
             ffi lib { func apply(borrow cb: func(i32, bool, cstring, raw u8, Point, Color, func(i32): i32): i32): i32; }\n\
             func demo(): i32 { return apply((a: i32, b: bool, c: cstring, d: raw u8, p: Point, col: Color, f: func(i32): i32) => 0); }",
        ),
    );
}

/// A `[borrow x]` capture writes through the callback's environment.
#[test]
fn a_borrow_capture_at_a_hook_writes_through() {
    let output = run_host_fixture(
        "hk-borrow-capture",
        &run_program(
            "ffi lib { func cl_fixture_apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 {\n\
                 let n: i32 = 1;\n\
                 let r = cl_fixture_apply([borrow n] (x: i32) => { n = n + 1; return x + n; }, 1);\n\
                 if (n == 2 && r == 3) { return 0; }\n\
                 return 1;\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the borrow capture must write through; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
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

// ---------------------------------------------------------------------------
// A capturing lambda crosses a synchronous C hook through a trampoline.
// ---------------------------------------------------------------------------

/// A capturing lambda returns the value its environment holds.
#[test]
fn a_capturing_lambda_crosses_a_borrow_hook() {
    let output = run_host_fixture(
        "hk-capturing",
        &run_program(
            "ffi lib { func cl_fixture_apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 {\n\
                 let n: i32 = 40;\n\
                 if (cl_fixture_apply([n] (x: i32) => x + n, 2) == 42) { return 0; }\n\
                 return 1;\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the capturing hook must return its captured value; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Two capturing callbacks in one call each find their own environment.
#[test]
fn two_capturing_callbacks_find_their_environments() {
    let output = run_host_fixture(
        "hk-two-callbacks",
        &run_program(
            "ffi lib { func cl_fixture_apply2(borrow a: func(i32): i32, borrow b: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 {\n\
                 let x: i32 = 1;\n\
                 let y: i32 = 2;\n\
                 if (cl_fixture_apply2([x] (v: i32) => v + x, [y] (v: i32) => v + y, 10) == 23) { return 0; }\n\
                 return 1;\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "both callbacks must find their own environment; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A callback that calls a Clorraine function which calls another hook nests.
#[test]
fn nested_hook_calls_find_their_environments() {
    let output = run_host_fixture(
        "hk-nested",
        &run_program(
            "ffi lib {\n\
                 func cl_fixture_reenter(borrow cb: func(i32): i32, value: i32): i32;\n\
                 func cl_fixture_apply(borrow cb: func(i32): i32, value: i32): i32;\n\
             }\n\
             func inner(x: i32): i32 {\n\
                 let n: i32 = 100;\n\
                 return cl_fixture_apply([n] (v: i32) => v + n, x);\n\
             }\n\
             func demo(): i32 {\n\
                 let n: i32 = 1;\n\
                 if (cl_fixture_reenter([n] (v: i32) => inner(v) + n, 5) == 106) { return 0; }\n\
                 return 1;\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the nested hook must find its own environment; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A recursive hook reuses the same lambda site with a new environment.
#[test]
fn a_reentrant_hook_finds_its_own_environment() {
    let output = run_host_fixture(
        "hk-reentrant",
        &run_program(
            "ffi lib { func cl_fixture_apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func step(depth: i32): i32 {\n\
                 if (depth == 0) { return 0; }\n\
                 let n: i32 = depth;\n\
                 return cl_fixture_apply([n, depth] (x: i32) => x + n + step(depth - 1), 1);\n\
             }\n\
             func demo(): i32 { if (step(2) == 5) { return 0; } return 1; }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the reentrant hook must find the innermost environment; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A C hook that calls one callback twice reads the same environment.
#[test]
fn a_repeated_callback_reads_the_same_environment() {
    let output = run_host_fixture(
        "hk-repeat",
        &run_program(
            "ffi lib { func cl_fixture_repeat(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 {\n\
                 let n: i32 = 5;\n\
                 if (cl_fixture_repeat([n] (x: i32) => x + n, 3) == 16) { return 0; }\n\
                 return 1;\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "a repeated callback must read the same environment; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A sibling effect after the callback runs cannot change a copy capture.
#[test]
fn a_sibling_effect_does_not_change_a_captured_value() {
    let output = run_host_fixture(
        "hk-order",
        &run_program(
            "ffi lib {\n\
                 func cl_fixture_ptr_a(): raw i32;\n\
                 func cl_fixture_ptr_b(): raw i32;\n\
                 func cl_fixture_read(p: raw i32): i32;\n\
                 func cl_fixture_apply0(borrow cb: func(): i32, value: i32): i32;\n\
             }\n\
             func bump(borrow p: raw i32?): i32 { p = cl_fixture_ptr_b(); return 0; }\n\
             func demo(): i32 {\n\
                 let p: raw i32? = cl_fixture_ptr_a();\n\
                 let r = cl_fixture_apply0(\n\
                     [p] () => { if (p == null) { return -1; } return cl_fixture_read(p); },\n\
                     bump(p));\n\
                 if (p == null) { return 1; }\n\
                 if (cl_fixture_read(p) != 9) { return 2; }\n\
                 if (r != 7) { return 3; }\n\
                 return 0;\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the capture must be read at its own argument position; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A capturing hook in a short-circuit operand does not run when skipped.
#[test]
fn a_capturing_hook_in_a_short_circuit_does_not_run() {
    let output = run_host_fixture(
        "hk-short-circuit-capturing",
        &run_program(
            "ffi lib {\n\
                 func cl_fixture_apply(borrow cb: func(i32): i32, value: i32): i32;\n\
                 func cl_fixture_count(): i32;\n\
             }\n\
             func demo(): i32 {\n\
                 let n: i32 = 1;\n\
                 if (false && cl_fixture_apply([n] (x: i32) => x + n, 2) == 3) { return 1; }\n\
                 return cl_fixture_count();\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the skipped hook must not run; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A callback invoked after its C hook returns faults with E11215.
#[test]
fn a_callback_invoked_after_return_faults() {
    let output = run_host_fixture(
        "hk-after-return",
        &run_program(
            "ffi lib {\n\
                 func cl_fixture_defer(borrow cb: func(i32): i32): void;\n\
                 func cl_fixture_run_deferred(value: i32): i32;\n\
             }\n\
             func demo(): i32 {\n\
                 let n: i32 = 1;\n\
                 cl_fixture_defer([n] (x: i32) => x + n);\n\
                 return cl_fixture_run_deferred(10);\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_ne!(
        output.status.code(),
        Some(0),
        "an after-return callback must fault"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("E11215"),
        "the fault must report E11215; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A borrowing capture of a binding the same call moves is rejected.
#[test]
fn a_hook_capture_of_a_moved_binding_is_rejected() {
    assert_rejected(
        "hk-capture-take",
        &program(
            "import { String, stringOf, bytes } from std.text;\n\
             ffi lib { func apply(borrow cb: func(i32): i32, take s: String): i32; }\n\
             func demo(): i32 {\n\
                 let s = stringOf(\"x\");\n\
                 return apply([s] (x: i32) => x + i32(len(bytes(s))), s);\n\
             }",
        ),
        "E10030",
    );
}

/// A guarded hook whose operand is evaluated fills the environment and runs.
#[test]
fn a_capturing_hook_in_an_evaluated_guard_runs() {
    let output = run_host_fixture(
        "hk-guard-true",
        &run_program(
            "ffi lib { func cl_fixture_apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 {\n\
                 let n: i32 = 1;\n\
                 if (true && cl_fixture_apply([n] (x: i32) => x + n, 2) == 3) { return 0; }\n\
                 return 1;\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the evaluated guarded hook must run; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A capturing hook in a loop body emits one wrapper and runs each iteration.
#[test]
fn a_capturing_hook_in_a_loop_body_builds() {
    let output = run_host_fixture(
        "hk-loop-body",
        &run_program(
            "ffi lib { func cl_fixture_apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 {\n\
                 let n: i32 = 1;\n\
                 let total: i32 = 0;\n\
                 let i: i32 = 0;\n\
                 while (i < 2) {\n\
                     total = total + cl_fixture_apply([n] (x: i32) => x + n, 1);\n\
                     i = i + 1;\n\
                 }\n\
                 if (total == 4) { return 0; }\n\
                 return 1;\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the loop-body hook must emit one wrapper; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A capturing hook in a defer emits one wrapper and runs on the exit edge.
#[test]
fn a_capturing_hook_in_a_defer_builds() {
    let output = run_host_fixture(
        "hk-defer",
        "module spec.v11;\n\
         ffi lib {\n\
             func cl_fixture_apply(borrow cb: func(i32): i32, value: i32): i32;\n\
             func cl_fixture_count(): i32;\n\
         }\n\
         func demo(): i32 {\n\
             let n: i32 = 1;\n\
             defer { let r = cl_fixture_apply([n] (x: i32) => x + n, 1); }\n\
             return 0;\n\
         }\n\
         func main(): i32 { let r = demo(); return cl_fixture_count() - 1; }\n",
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the defer hook must emit one wrapper and run once; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// An earlier effectful sibling runs before the callback environment is built.
#[test]
fn a_hook_capture_reads_after_an_earlier_sibling_effect() {
    let output = run_host_fixture(
        "hk-order-reverse",
        &run_program(
            "ffi lib {\n\
                 func cl_fixture_ptr_a(): raw i32;\n\
                 func cl_fixture_ptr_b(): raw i32;\n\
                 func cl_fixture_read(p: raw i32): i32;\n\
                 func cl_fixture_apply1(value: i32, borrow cb: func(): i32): i32;\n\
             }\n\
             func bump(borrow p: raw i32?): i32 { p = cl_fixture_ptr_b(); return 0; }\n\
             func demo(): i32 {\n\
                 let p: raw i32? = cl_fixture_ptr_a();\n\
                 let r = cl_fixture_apply1(\n\
                     bump(p),\n\
                     [p] () => { if (p == null) { return -1; } return cl_fixture_read(p); });\n\
                 if (r == 9) { return 0; }\n\
                 return 1;\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the earlier sibling effect must run before the capture; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A named callback's own ABI spelling is judged as well as the hook's.
#[test]
fn a_hook_callback_with_a_borrowed_value_is_rejected() {
    assert_rejected(
        "hk-found-mode",
        &program(
            "struct Owned { s: String; }\n\
             ffi lib { func apply(borrow cb: func(Owned): i32): i32; }\n\
             func peek(borrow o: Owned): i32 { return 0; }\n\
             func demo(): i32 { return apply(peek); }",
        ),
        "E10210",
    );
}

/// A generic body can forward an opaque function field to a hook.
#[test]
fn a_generic_function_field_reaches_a_hook() {
    let output = run_host_fixture(
        "hk-generic-field",
        &run_program(
            "struct Holder { cb: func(i32): i32; }\n\
             ffi lib { func cl_fixture_apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func run<T>(h: T): i32 { return cl_fixture_apply(h.cb, 1); }\n\
             func demo(): i32 {\n\
                 let r = run(Holder { cb: (x: i32) => x + 1 });\n\
                 if (r == 2) { return 0; }\n\
                 return 1;\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the generic function field must reach the hook; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A deleter that only borrows its parameter is rejected.
#[test]
fn a_deleter_that_only_borrows_is_rejected() {
    assert_rejected(
        "x1-borrow-only",
        &program(
            "handle Widget freedBy freeWidget;\n\
             ffi lib { func cl_widget_free(take w: Widget): void; func cl_peek(borrow w: Widget): i32; }\n\
             func freeWidget(take w: Widget): void { let v = cl_peek(w); }",
        ),
        "E10102",
    );
}

/// A deleter that releases only on one branch is rejected.
#[test]
fn a_deleter_that_frees_on_one_branch_is_rejected() {
    assert_rejected(
        "x1-branch-free",
        &program(
            "handle Widget freedBy freeWidget;\n\
             ffi lib { func cl_widget_free(take w: Widget): void; func maybe(): bool; }\n\
             func freeWidget(take w: Widget): void { if (maybe()) { cl_widget_free(w); } }",
        ),
        "E10102",
    );
}

/// An indirect deleter release cycle is rejected.
#[test]
fn an_indirect_deleter_cycle_is_rejected() {
    assert_rejected(
        "x1-cycle",
        &program(
            "handle Widget freedBy freeWidget;\n\
             ffi lib { func cl_widget_free(take w: Widget): void; }\n\
             func relay(take w: Widget): void { freeWidget(w); }\n\
             func freeWidget(take w: Widget): void { relay(w); }",
        ),
        "E10102",
    );
}

/// A take argument that forwards the captured binding through a call is caught.
#[test]
fn a_hook_capture_of_a_binding_moved_through_a_call_is_rejected() {
    assert_rejected(
        "hk-capture-take-call",
        &program(
            "handle Widget freedBy freeWidget;\n\
             ffi widget { func cl_widget_new(): Widget; func cl_widget_free(take w: Widget): void; }\n\
             ffi lib { func apply(borrow cb: func(i32): i32, take w: Widget): i32; }\n\
             func freeWidget(take w: Widget): void { cl_widget_free(w); }\n\
             func pass(take w: Widget): Widget { return w; }\n\
             func demo(): i32 {\n\
                 let w = cl_widget_new();\n\
                 return apply([w] (v: i32) => v, pass(w));\n\
             }",
        ),
        "E10030",
    );
}

/// A null view argument to a capturing hook uses the ABI null, not a zero view.
#[test]
fn a_null_view_argument_in_a_capturing_hook_builds() {
    let output = run_host_fixture(
        "hk-null-view",
        &run_program(
            "ffi lib { func cl_fixture_view(borrow cb: func(i32): i32, s: str?): i32; }\n\
             func demo(): i32 {\n\
                 let n: i32 = 1;\n\
                 if (cl_fixture_view([n] (x: i32) => x + n, null) == 2) { return 0; }\n\
                 return 1;\n\
             }",
        ),
        HOOK_FIXTURE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the null view must cross as the ABI null; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
