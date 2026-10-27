//! Emitted-shape and internal-boundary tests for the strict-build batch.
//!
//! The specification suite states the observable behavior; these cases pin the
//! diagnostic code and count, the reported span, the absence of a translation
//! unit, the re-enabled E10213 golden, the collision report, the parse-only
//! contract, and byte-identical re-emission.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one accepted control may run before the case fails as a hang.
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
            "clc-strictimpl-{}-{tag}-{sequence}",
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

/// Writes the named sources and compiles the first one as the entry.
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
        .unwrap_or_else(|| panic!("{tag}: no source"))
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
    (scratch, run_clc(&args), c_path)
}

/// Compiles one source.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    compile_files(tag, &[("main.clo", source)], &[])
}

/// Asserts one `code` rejection and returns the rendered stderr.
fn rejection(tag: &str, sources: &[(&str, &str)], code: &str) -> String {
    let (_scratch, output, c_path) = compile_files(tag, sources, &[]);
    assert_ne!(output.status.code(), Some(0), "{tag}: expected a rejection");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(
        stderr.matches(&format!("error {code}:")).count(),
        1,
        "{tag}: expected one {code}; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not write a translation unit"
    );
    stderr
}

/// Emits one accepted source.
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

/// Emits one accepted source twice, returning both translation units.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the source");
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

/// Emits, builds, and runs one accepted control.
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
    let mut child = Command::new(&program)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("{tag}: cannot run: {err}"));
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
                panic!("{tag}: the program did not finish");
            }
            Err(err) => panic!("{tag}: cannot wait: {err}"),
        }
    }
    child
        .wait_with_output()
        .unwrap_or_else(|err| panic!("{tag}: cannot collect output: {err}"))
}

// ---------------------------------------------------------------------------
// DEF-167: explicit type arguments on every non-generic callee
// ---------------------------------------------------------------------------

/// One E10300 at the call, no C; the parse-only mode keeps the stage parse.
#[test]
fn should_pin_the_type_argument_diagnostic_shape() {
    let stage = r#"module impl.strict.stage;

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
    let stderr = rejection("st32stage", &[("main.clo", stage)], "E10300");
    assert!(
        stderr.contains("wrong number of type arguments"),
        "st32stage: unexpected message; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("main.clo:6:"),
        "st32stage: the report must land on the call; stderr:\n{stderr}"
    );

    let (_scratch, parsed, c_path) = compile_files(
        "st32parse",
        &[("main.clo", stage)],
        &["--parse-only"],
    );
    assert_eq!(
        parsed.status.code(),
        Some(0),
        "st32parse: the parse-only mode rejected the stage parse; stderr:\n{}",
        String::from_utf8_lossy(&parsed.stderr)
    );
    assert!(!c_path.exists(), "st32parse: parse-only must not write C");

    let func_value = r#"module impl.strict.funcvalue;

func main(): i32 {
    let g: func(i32): i32 = (x: i32) => x;
    return g<i32>(1);
}
"#;
    rejection("st32funcvalue", &[("main.clo", func_value)], "E10300");
}

// ---------------------------------------------------------------------------
// DEF-171: the embed constant context and the re-enabled golden
// ---------------------------------------------------------------------------

/// The runtime positions report one E10213 with their span; the module const
/// keeps E10224; the golden file is rejected.
#[test]
fn should_pin_the_embed_context_diagnostics() {
    let local_let = r#"module impl.strict.embedlocal;

func checksum(): u32 {
    let data: u8[16] = embed("assets/logo.bin");
    return u32(len(data));
}

func main(): i32 {
    return 0;
}
"#;
    let stderr = rejection("st33let", &[("main.clo", local_let)], "E10213");
    assert!(
        stderr.contains("main.clo:4:"),
        "st33let: the report must land on the embed; stderr:\n{stderr}"
    );

    let module_const = r#"module impl.strict.embedconst;

const LEVEL: u8[16] = embed("assets/logo.bin");

func main(): i32 {
    return 0;
}
"#;
    let (_scratch, output, c_path) = compile_only("st33const", module_const);
    assert_ne!(output.status.code(), Some(0), "st33const: expected a rejection");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10224:").count(),
        1,
        "st33const: expected one E10224; stderr:\n{stderr}"
    );
    assert_eq!(
        stderr.matches("error E10213:").count(),
        0,
        "st33const: a module const must not report E10213; stderr:\n{stderr}"
    );
    assert!(!c_path.exists(), "st33const: no translation unit");

    let golden = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../spec/tests/compile-fail/E10213_embed_outside_const.clo");
    let golden_source = std::fs::read_to_string(&golden)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", golden.display()));
    rejection("st33golden", &[("main.clo", &golden_source)], "E10213");
}

// ---------------------------------------------------------------------------
// DEF-416: the cross-module collision and its same-module control
// ---------------------------------------------------------------------------

/// One E11101 at the second module's declaration, no C; the same-module type
/// name accepts and runs.
#[test]
fn should_pin_the_cross_module_collision() {
    let a = r#"module impl.coll.a;

export struct Thing { v: u8; }
"#;
    let b = r#"module impl.coll.b;

export struct Thing { v: i32; }
"#;
    let main = r#"module impl.coll.main;

import { Thing as AThing } from impl.coll.a;
import { Thing as BThing } from impl.coll.b;

func main(): i32 {
    let a: AThing = AThing { v: 1 };
    let b: BThing = BThing { v: 2 };
    return i32(a.v) + b.v - 3;
}
"#;
    let stderr = rejection(
        "st34collision",
        &[("main.clo", main), ("a.clo", a), ("b.clo", b)],
        "E11101",
    );
    assert!(
        stderr.contains("duplicate declaration of 'Thing'"),
        "st34collision: unexpected message; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("b.clo:3:"),
        "st34collision: the report must land on the second declaration; stderr:\n{stderr}"
    );

    let control = r#"module impl.coll.one;

struct Thing { v: i32; }

func main(): i32 {
    let t: Thing = Thing { v: 1 };
    return t.v - 1;
}
"#;
    let output = run_host("st34control", control);
    assert_eq!(
        output.status.code(),
        Some(0),
        "st34control: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Emission and determinism
// ---------------------------------------------------------------------------

/// The accepted constant control emits stable C.
#[test]
fn should_reemit_the_strict_control_identically() {
    let control = r#"module impl.strict.determinism;

const N: i32 = 3;

func main(): i32 {
    return N - 3;
}
"#;
    let (first, second) = emit_twice("determinism", control);
    assert_eq!(first, second, "the emitted translation unit is not stable");
}
