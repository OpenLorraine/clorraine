//! Specification tests for the strict-build and type-identity batch.
//!
//! Each case compiles small Clorraine programs with the built `clc` binary. A
//! rejection case asserts the exact code count, the message fragments, and that
//! no translation unit is written; an accepted control builds the emitted C
//! under the strict warning flags and runs it. The batch pins the explicit
//! type-argument rejection on every non-generic callee, the parse-only stage
//! mode, the `embed` constant-context rule, and the cross-module same-name type
//! collision.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one control program may run before the case fails as a hang.
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
            "clc-strictspec-{}-{tag}-{sequence}",
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

/// Writes the named source files and compiles the first one as the entry.
///
/// The extra files are passed to `clc` in order; the loader uses each file's
/// module declaration, exactly as a multi-file program does.
fn compile_files(
    tag: &str,
    sources: &[(&str, &str)],
    flags: &[&str],
) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let mut paths = Vec::new();
    for (name, source) in sources {
        let path = scratch.path().join(name);
        std::fs::write(&path, source)
            .unwrap_or_else(|err| panic!("{tag}: cannot write {name}: {err}"));
        paths.push(path);
    }
    let c_path = scratch.path().join("out.c");
    let entry = paths
        .first()
        .unwrap_or_else(|| panic!("{tag}: no source file"))
        .display()
        .to_string();
    let mut args = vec![
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
    ];
    args.extend(flags.iter().map(|flag| flag.to_string()));
    args.push(entry);
    args.extend(paths[1..].iter().map(|path| path.display().to_string()));
    let output = run_clc(&args);
    (scratch, output, c_path)
}

/// Compiles one source and returns the compiler output and the C path.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    compile_files(tag, &[("main.clo", source)], &[])
}

/// Asserts the front end rejects every source set with exactly one `code`
/// diagnostic, the given message fragments, and no translation unit.
fn assert_rejected_once(tag: &str, sources: &[(&str, &str)], code: &str, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_files(tag, sources, &[]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let marker = format!("error {code}:");
    assert_eq!(
        stderr.matches(&marker).count(),
        1,
        "{tag}: expected exactly one {code}; stderr:\n{stderr}"
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: the diagnostic does not contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not write a translation unit"
    );
}


/// Emits one host program and asserts the front end accepted it.
fn emit_host(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Emits, builds, and runs one host program, returning its output.
fn run_host(tag: &str, source: &str) -> Output {
    let emitted = emit_host(tag, source);
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, &emitted).expect("write the emitted C");
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
    run_program(tag, &program, &[])
}

/// Runs a built program with a time limit.
fn run_program(tag: &str, program: &Path, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(program);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command
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

/// Asserts the program exit code.
fn assert_exit(tag: &str, output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{tag}: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// DEF-167: explicit type arguments on a non-generic callee
// ---------------------------------------------------------------------------

/// The stage-spec source `<i32>` locals full-compiles to one E10300; the
/// same source parses cleanly through the parse-only mode; the non-generic
/// free, qualified, dot, func-typed, and local forms each report one E10300.
#[test]
fn should_reject_explicit_type_arguments_on_non_generic_callees() {
    let stage_shape = r#"module spec.strict.stage;

func shapes(a: i32, b: i32, c: i32): i32 {
    let comparison = a < b;
    let explicit_args = identity<i32>(1);
    let call_shaped = a < b > (c);
    return 0;
}

func identity<T>(x: T): T {
    return x;
}

func main(): i32 {
    return shapes(1, 2, 3);
}
"#;
    assert_rejected_once("st32stage", &[("main.clo", stage_shape)], "E10300", &[]);

    let parsed = compile_files(
        "st32parse",
        &[("main.clo", stage_shape)],
        &["--parse-only"],
    );
    assert_eq!(
        parsed.1.status.code(),
        Some(0),
        "st32parse: the parse-only mode rejected the stage parse; stderr:\n{}",
        String::from_utf8_lossy(&parsed.1.stderr)
    );

    let non_generic_function = r#"module spec.strict.nongeneric;

func id(x: i32): i32 {
    return x;
}

func main(): i32 {
    return id<i32>(1);
}
"#;
    assert_rejected_once(
        "st32func",
        &[("main.clo", non_generic_function)],
        "E10300",
        &[],
    );

    let qualified_helper = r#"module spec.strict.lib;

export func id(x: i32): i32 {
    return x;
}
"#;
    let qualified_main = r#"module spec.strict.qualified;

func main(): i32 {
    return spec.strict.lib.id<i32>(1);
}
"#;
    assert_rejected_once(
        "st32qualified",
        &[("main.clo", qualified_main), ("lib.clo", qualified_helper)],
        "E10300",
        &[],
    );

    let dot = r#"module spec.strict.dot;

struct H { v: i32; }

func call(borrow h: H, x: i32): i32 {
    return 0;
}

func main(): i32 {
    let h: H = H { v: 1 };
    return h.call<i32>(1);
}
"#;
    assert_rejected_once("st32dot", &[("main.clo", dot)], "E10300", &[]);

    let func_value = r#"module spec.strict.funcvalue;

func main(): i32 {
    let g: func(i32): i32 = (x: i32) => x;
    return g<i32>(1);
}
"#;
    assert_rejected_once("st32funcvalue", &[("main.clo", func_value)], "E10300", &[]);

    let local = r#"module spec.strict.local;

func main(): i32 {
    let a: i32 = 1;
    let b: i32 = 2;
    let c: i32 = 3;
    let r = a < b > (c);
    return r;
}
"#;
    assert_rejected_once("st32local", &[("main.clo", local)], "E10300", &[]);
}

// ---------------------------------------------------------------------------
// DEF-171: embed outside a constant context
// ---------------------------------------------------------------------------

/// A function-body local `let`/`const` and a module-level `let` report one
/// E10213; a module `const` keeps only the staged E10224; the non-embed
/// constant control builds.
#[test]
fn should_reject_embed_outside_a_constant_context() {
    let local_let = r#"module spec.strict.embedlocal;

func checksum(): u32 {
    let data: u8[16] = embed("assets/logo.bin");
    return u32(len(data));
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once("st33let", &[("main.clo", local_let)], "E10213", &["embed"]);

    let local_const = r#"module spec.strict.embedconstlocal;

func checksum(): u32 {
    const DATA: u8[16] = embed("assets/logo.bin");
    return u32(len(DATA));
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once("st33const", &[("main.clo", local_const)], "E10213", &["embed"]);

    let module_let = r#"module spec.strict.embedglobal;

let palette: u8[64] = embed("assets/logo.bin");

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once("st33global", &[("main.clo", module_let)], "E10213", &["embed"]);
}

/// A module `const` keeps only the staged E10224 and no E10213.
#[test]
fn should_keep_the_module_const_embed_at_e10224() {
    let module_const = r#"module spec.strict.embedmoduleconst;

const LEVEL: u8[16] = embed("assets/logo.bin");

func main(): i32 {
    return 0;
}
"#;
    let (_scratch, output, c_path) = compile_only("st33moduleconst", module_const);
    assert_ne!(output.status.code(), Some(0), "st33moduleconst: expected a rejection");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10224:").count(),
        1,
        "st33moduleconst: expected one E10224; stderr:\n{stderr}"
    );
    assert_eq!(
        stderr.matches("error E10213:").count(),
        0,
        "st33moduleconst: a module const must not report E10213; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "st33moduleconst: a rejected program must not write C"
    );

    let control = r#"module spec.strict.embedcontrol;

const N: i32 = 3;

func main(): i32 {
    return N - 3;
}
"#;
    let output = run_host("st33control", control);
    assert_exit("st33control", &output, 0);
}

// ---------------------------------------------------------------------------
// DEF-416: cross-module same-simple-name collision
// ---------------------------------------------------------------------------

/// Two modules declaring the same simple type name report one E11101 at the
/// second declaration and write no C.
#[test]
fn should_reject_a_cross_module_simple_name_collision() {
    let a = r#"module spec.coll2.a;

export struct Thing { v: u8; }
"#;
    let b = r#"module spec.coll2.b;

export struct Thing { v: i32; }
"#;
    let main = r#"module spec.coll2.main;

import { Thing as AThing } from spec.coll2.a;
import { Thing as BThing } from spec.coll2.b;

func main(): i32 {
    let a: AThing = AThing { v: 1 };
    let b: BThing = BThing { v: 2 };
    return i32(a.v) + b.v - 3;
}
"#;
    assert_rejected_once(
        "st34collision",
        &[("main.clo", main), ("a.clo", a), ("b.clo", b)],
        "E11101",
        &["Thing"],
    );
}

/// A same-module bare type name resolves normally; the collision check must not
/// reject one module's own declaration.
#[test]
fn should_accept_a_same_module_bare_type_name() {
    let source = r#"module spec.coll2.one;

struct Thing { v: i32; }

func main(): i32 {
    let t: Thing = Thing { v: 1 };
    return t.v - 1;
}
"#;
    let output = run_host("st34control", source);
    assert_exit("st34control", &output, 0);
}
