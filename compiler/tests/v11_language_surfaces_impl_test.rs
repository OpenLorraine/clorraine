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

// ---------------------------------------------------------------------------
// L5: synchronous C hook trampolines
// ---------------------------------------------------------------------------

/// A capture-less callback stays a plain function pointer with no wrapper.
#[test]
fn a_captureless_hook_stays_a_plain_function_pointer() {
    let (_scratch, c) = compile(
        "hk-captureless-impl",
        "host",
        &program(
            "ffi lib { func apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 { return apply((x: i32) => x + 1, 1); }",
        ),
    );
    assert!(
        c.contains("apply((LONG (*)(LONG))(void (*)(void))(cl_lambda_0), 1)"),
        "a capture-less callback must be passed directly through the ABI cast; emitted:\n{c}"
    );
    for absent in [
        "cl_hooktramp_",
        "cl_hookwrap_",
        "ClHookEnvNode",
        "ClClosure_",
    ] {
        assert!(
            !c.contains(absent),
            "a capture-less hook must not emit '{absent}'; emitted:\n{c}"
        );
    }
}

/// A capturing callback emits one trampoline, one wrapper, and a stack node.
#[test]
fn a_capturing_hook_emits_a_trampoline_and_a_stack_environment() {
    let (_scratch, c) = compile(
        "hk-capturing-impl",
        "host",
        &program(
            "ffi lib { func apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 { let n: i32 = 1; return apply([n] (x: i32) => x + n, 1); }",
        ),
    );
    assert_eq!(
        c.matches("struct ClHookEnvNode {").count(),
        1,
        "the environment stack must be emitted once; emitted:\n{c}"
    );
    assert!(
        c.contains("static LONG cl_hooktramp_0(LONG a0)"),
        "the trampoline must use the declared callback signature; emitted:\n{c}"
    );
    assert!(
        c.contains("cl_hook_env_find(0)"),
        "the trampoline must find its environment by id; emitted:\n{c}"
    );
    assert!(
        c.contains("cl_hook_env_push(&n0, 0, env0)"),
        "the wrapper must push the callback node; emitted:\n{c}"
    );
    assert!(
        c.contains("cl_hook_env_pop(&n0)"),
        "the wrapper must pop the callback node; emitted:\n{c}"
    );
    assert!(
        c.contains("cl_hookwrap_0((LONG (*)(LONG))(void (*)(void))(cl_hooktramp_0), 1, (__cl_tmp0).env)"),
        "the wrapper call must pass the trampoline and the closure environment; emitted:\n{c}"
    );
    assert!(
        !c.contains("cl_hook_env_alloc"),
        "the hook environment must not allocate; emitted:\n{c}"
    );
    assert!(
        c.contains("&(((struct ClEnv_0){ n }))"),
        "the environment must be a caller-frame temporary; emitted:\n{c}"
    );
    let temp = c
        .find("__cl_tmp0 = ((ClClosure_0)")
        .expect("the closure temporary must be assigned");
    let call = c
        .find("cl_hookwrap_0((")
        .expect("the wrapper call must be emitted");
    assert!(
        temp < call,
        "the closure must be materialized before the call; emitted:\n{c}"
    );
}

/// Two capturing callbacks push in argument order and pop in reverse.
#[test]
fn two_hook_callbacks_emit_reverse_pops() {
    let (_scratch, c) = compile(
        "hk-two-impl",
        "host",
        &program(
            "ffi lib { func apply2(borrow a: func(i32): i32, borrow b: func(i32): i32, value: i32): i32; }\n\
             func demo(): i32 {\n\
                 let x: i32 = 1;\n\
                 let y: i32 = 2;\n\
                 return apply2([x] (v: i32) => v + x, [y] (v: i32) => v + y, 10);\n\
             }",
        ),
    );
    assert_eq!(
        c.matches("static LONG cl_hooktramp_").count(),
        4,
        "two callbacks need two prototypes and two definitions; emitted:\n{c}"
    );
    assert_eq!(
        c.matches("static LONG cl_hookwrap_").count(),
        2,
        "one wrapper prototype and one definition are due; emitted:\n{c}"
    );
    let push0 = c.find("cl_hook_env_push(&n0, 0, env0)").expect("push n0");
    let push1 = c.find("cl_hook_env_push(&n1, 1, env1)").expect("push n1");
    let pop1 = c.find("cl_hook_env_pop(&n1)").expect("pop n1");
    let pop0 = c.find("cl_hook_env_pop(&n0)").expect("pop n0");
    assert!(push0 < push1, "pushes follow argument order; emitted:\n{c}");
    assert!(push1 < pop1 && pop1 < pop0, "pops reverse the pushes; emitted:\n{c}");
}

/// A capturing hook crosses the m68k Amiga ABI and runs under `vamos`.
///
/// The SDK container provides the cross compiler and the runner; a host
/// without them skips the leg with a note, exactly as the other Amiga suites.
#[test]
fn a_capturing_hook_crosses_the_m68k_abi() {
    if !tool_available("m68k-amigaos-gcc") || !tool_available("vamos") {
        eprintln!("skip: m68k-amigaos-gcc or vamos is not on PATH");
        return;
    }
    let (scratch, _c) = compile(
        "hk-m68k",
        "hosted-amiga",
        "module tests.hookm68k;\n\
         import { println } from std.io;\n\
         ffi lib { func cl_hook_fixture_apply(borrow cb: func(i32): i32, value: i32): i32; }\n\
         func demo(): i32 {\n\
             let n: i32 = 40;\n\
             return cl_hook_fixture_apply([n] (x: i32) => x + n, 2);\n\
         }\n\
         func main(): i32 {\n\
             if (demo() == 42) { println(\"hook: ok 42\"); } else { println(\"hook: bad\"); }\n\
             return 0;\n\
         }\n",
    );
    let fixture = scratch.path().join("hook_fixture_m68k.c");
    std::fs::write(
        &fixture,
        include_str!("fixtures/hook_fixture_m68k.c"),
    )
    .expect("write the m68k fixture");
    let binary = scratch.path().join("hook-m68k");
    let build = Command::new("m68k-amigaos-gcc")
        .args([
            "-std=c99",
            "-mcrt=nix20",
            "-m68000",
            "-fwrapv",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
        ])
        .arg(&binary)
        .arg(scratch.path().join("out.c"))
        .arg(&fixture)
        .args(["-lamiga", "-lm"])
        .output()
        .expect("failed to spawn m68k-amigaos-gcc");
    assert!(
        build.status.success(),
        "the m68k hook fixture must build strictly; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new("vamos")
        .arg(&binary)
        .output()
        .expect("failed to spawn vamos");
    assert!(
        String::from_utf8_lossy(&run.stdout).replace('\r', "").contains("hook: ok 42"),
        "the m68k hook must return its captured value; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

/// True when a tool runs on this host.
fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// The accepted callback set compiles under the strict m68k ABI spelling.
#[test]
fn the_hook_abi_set_builds_on_m68k() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    let (scratch, _c) = compile(
        "hk-m68k-abi",
        "hosted-amiga",
        "module tests.hookm68k.abi;\n\
         enum Color: u8 { Red, Green }\n\
         ffi lib {\n\
             func cl_fixture_mix(borrow cb: func(bool, raw u8, Color, func(i32): i32): i32): i32;\n\
         }\n\
         func demo(): i32 {\n\
             return cl_fixture_mix((b: bool, p: raw u8, c: Color, f: func(i32): i32) => 0);\n\
         }\n\
         func main(): i32 { return demo(); }\n",
    );
    let object = scratch.path().join("hook-abi.o");
    let build = Command::new("m68k-amigaos-gcc")
        .args([
            "-std=c99",
            "-mcrt=nix20",
            "-m68000",
            "-fwrapv",
            "-O0",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-c",
        ])
        .arg(scratch.path().join("out.c"))
        .arg("-o")
        .arg(&object)
        .output()
        .expect("failed to spawn m68k-amigaos-gcc");
    assert!(
        build.status.success(),
        "the accepted callback set must build on m68k; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}
