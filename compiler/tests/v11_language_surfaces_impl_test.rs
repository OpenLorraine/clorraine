//! Emitted-C pins for the v1.1 language surfaces.
//!
//! These tests compile a program with the built `clc` binary and assert the
//! shape of the emitted translation unit. They are implementation pins: a
//! shape that changes for an unintended reason is a failure. Behavior lives in
//! the specification suite.

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
            "clc-v11impl-{}-{tag}-{sequence}",
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

/// Compiles `source` and returns the scratch directory and the emitted C.
fn compile(tag: &str, profile: &str, source: &str) -> (ScratchDir, String) {
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
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected acceptance; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let c = std::fs::read_to_string(&c_path)
        .unwrap_or_else(|err| panic!("{tag}: cannot read {}: {err}", c_path.display()));
    (scratch, c)
}

/// Compiles two modules and returns the scratch directory and emitted C.
fn compile_two(tag: &str, first: &str, second: &str) -> (ScratchDir, String) {
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
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected acceptance; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let c = std::fs::read_to_string(&c_path).expect("read the emitted C");
    (scratch, c)
}

/// A minimal program wrapper.
fn program(items: &str) -> String {
    format!("module spec.v11.impl;\n{items}\nfunc main(): i32 {{ return 0; }}\n")
}

// ---------------------------------------------------------------------------
// Box type model
// ---------------------------------------------------------------------------

/// `Box<i32>` is a `void *` value and the type key names the element.
#[test]
fn a_box_uses_the_opaque_pointer_representation() {
    let (_scratch, c) = compile(
        "box-type",
        "host",
        &program(
            "struct Holder { b: Box<i32>; n: i32; }\n\
             func keep(b: Box<i32>): void { }",
        ),
    );
    assert!(
        c.contains("void * b;"),
        "a Box field must be an opaque pointer; emitted:\n{c}"
    );
}

/// A struct containing a Box folds its size and offset through the layout.
#[test]
fn a_struct_with_a_box_folds_its_layout() {
    let (_scratch, c) = compile(
        "box-layout",
        "host",
        &program(
            "const N: usize = sizeof(Box<i32>);\n\
             struct Holder { b: Box<i32>; n: i32; }\n\
             const M: usize = sizeof(Holder);\n\
             func demo(): i32 {\n\
                 let xs: u8[N];\n\
                 let ys: u8[M];\n\
                 return 0;\n\
             }",
        ),
    );
    assert!(
        c.contains("u8 xs[8]"),
        "sizeof(Box<i32>) must fold to the host pointer size; emitted:\n{c}"
    );
    assert!(
        c.contains("u8 ys[16]"),
        "sizeof(StructWithBox) must fold through the struct layout; emitted:\n{c}"
    );
}

/// A `Box<Wrap<i32>>` records the nested generic instance.
#[test]
fn a_boxed_generic_instance_records_the_inner_typedef() {
    let (_scratch, c) = compile(
        "box-generic-instance",
        "host",
        &program(
            "export struct Wrap<T> { v: T; }\n\
             struct Holder { b: Box<Wrap<i32>>; }",
        ),
    );
    assert!(
        c.contains("struct ClWrap__i32") || c.contains("ClWrap__i32"),
        "the inner Wrap<i32> typedef must be emitted; emitted:\n{c}"
    );
}

/// A generic aggregate is not emitted as an FFI prototype (`E10105` rejects it
/// first); this control checks the normal non-FFI path stays valid.
#[test]
fn a_generic_aggregate_instance_is_emitted_for_a_value() {
    let (_scratch, c) = compile(
        "wrap-value",
        "host",
        &program(
            "export struct Wrap<T> { v: T; }\n\
             func keep(take w: Wrap<i32>): void { }",
        ),
    );
    assert!(
        c.contains("ClWrap__i32"),
        "a value use of Wrap<i32> must name the instance typedef; emitted:\n{c}"
    );
}

// ---------------------------------------------------------------------------
// Box allocation and drop helpers
// ---------------------------------------------------------------------------

/// A boxed scalar has a prototype, a definition, and a zoned allocation.
#[test]
fn a_boxed_scalar_emits_the_helpers() {
    let (_scratch, c) = compile(
        "box-helpers",
        "host",
        &program(
            "import { boxNew } from std.mem;\n\
             func demo(): i32 {\n\
                 let b = boxNew<i32>(7);\n\
                 return 0;\n\
             }",
        ),
    );
    assert!(
        c.contains("static inline void *cl_box_alloc__i32(void);"),
        "the allocation helper prototype must be emitted; emitted:\n{c}"
    );
    assert!(
        c.contains("static inline void cl_box_drop__i32(void *box);"),
        "the drop helper prototype must be emitted; emitted:\n{c}"
    );
    assert!(
        c.contains("return cl_box_alloc(sizeof(i32));"),
        "the allocation helper must ask for the element size; emitted:\n{c}"
    );
    assert!(
        c.contains("if (b) { cl_box_drop__i32(b); }"),
        "the scope drop must call the helper; emitted:\n{c}"
    );
}

/// A generic `boxNew<T>` instance names the concrete helper, not `p1_T`.
#[test]
fn a_generic_box_new_names_the_concrete_helper() {
    let (_scratch, c) = compile(
        "box-generic-helper",
        "host",
        &program(
            "import { boxNew } from std.mem;\n\
             import { String, stringOf } from std.text;\n\
             func demo(): i32 {\n\
                 let b = boxNew<String>(stringOf(\"x\"));\n\
                 return 0;\n\
             }",
        ),
    );
    assert!(
        !c.contains("cl_box_alloc__p1_T"),
        "an unsubstituted parameter helper name must not be emitted; emitted:\n{c}"
    );
    assert!(
        c.contains("cl_box_alloc__String"),
        "the concrete String helper must be emitted; emitted:\n{c}"
    );
}

// ---------------------------------------------------------------------------
// X1: ordinary freedBy deleters
// ---------------------------------------------------------------------------

/// A same-module ordinary deleter uses its mangled module symbol.
#[test]
fn an_ordinary_deleter_uses_its_module_symbol() {
    let (_scratch, c) = compile(
        "x1-symbol",
        "host",
        &program(
            "handle Widget freedBy freeWidget;\n\
             ffi lib { func cl_free_widget(take w: Widget): void; }\n\
             func freeWidget(take w: Widget): void { cl_free_widget(w); }\n\
             func demo(take w: Widget): void { }",
        ),
    );
    assert!(
        c.contains("cl_spec_v11_impl_freeWidget(w)"),
        "the ordinary deleter must use its module symbol; emitted:\n{c}"
    );
}

/// A cross-module qualified deleter is mangled once, not twice.
#[test]
fn a_cross_module_deleter_is_not_double_mangled() {
    let (_scratch, c) = compile_two(
        "x1-cross-symbol",
        "module a;\nexport handle Widget freedBy b.freeWidget;\nfunc demo(take w: Widget): void { }\nfunc main(): i32 { return 0; }\n",
        "module b;\nffi lib { func cl_free_widget(take w: a.Widget): void; }\nexport func freeWidget(take w: a.Widget): void { cl_free_widget(w); }\n",
    );
    assert!(
        c.contains("cl_b_freeWidget(w)"),
        "the qualified deleter must mangle once; emitted:\n{c}"
    );
    assert!(
        !c.contains("cl_b_b_freeWidget"),
        "the qualified deleter must not double-mangle; emitted:\n{c}"
    );
}
