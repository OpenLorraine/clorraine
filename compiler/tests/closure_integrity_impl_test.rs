//! Implementation tests for the closure-integrity batch.
//!
//! These cases pin the observable internals of the capture-list validity
//! judgments: the exact diagnostic sequence and message name per shape (the
//! relayed entry, the module-level entries, the dotted write and dotted
//! move), the single report for a deep relay, the accepted deep relay
//! actually building and running, and the byte-identical re-emission of the
//! accepted relay. They complement the specification suite, which pins the
//! accepted and rejected program behavior.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one program may run before the case fails as a hang.
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
            "clc-closureimpl-{}-{tag}-{sequence}",
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

/// Compiles `source` and returns the scratch directory, output, and C path.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
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
    (scratch, output, c_path)
}

/// Emits one host-profile program and asserts the front end accepted it.
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

/// Emits the same program twice into one directory and returns both units.
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

/// Emits, builds, and runs one host program, returning its output.
fn run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
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

/// The `error E…:` codes the compiler reported on stderr, in order.
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

/// Asserts the program was rejected with exactly the given error codes, that
/// the message contains every fragment, and that no translation unit exists.
fn assert_rejected(tag: &str, source: &str, expected: &[&str], fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    let codes = error_codes(&output);
    assert_eq!(
        codes,
        expected,
        "{tag}: expected {expected:?}, saw {codes:?}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// Every rejected capture-validity shape reports exactly one named code.
///
/// The relayed entry reports `E10405` on the entry's line and names the
/// missing outer capture; the module-level entries (a `let` and a `const`)
/// report `E10406`; the dotted write reports `E10402` and the dotted move
/// `E10401`, both naming the root. A second code anywhere would mean a site
/// is judged by two rules, and a wrong name would mean the wrong side.
#[test]
fn should_report_one_named_error_per_capture_validity_shape() {
    assert_rejected(
        "relay-entry",
        r#"module spec.ci.impl.relaymissing;

func main(): i32 {
    let n: i32 = 2;
    let m: i32 = 3;
    return ([n] () =>
        ([m] () => m)())();
}
"#,
        &["E10405"],
        &["lambda reads 'm' without listing it", "main.clo:7"],
    );
    assert_rejected(
        "module-let-entry",
        r#"module spec.ci.impl.modulelet;

let g: i32 = 5;

func main(): i32 {
    return ([g] () => g)();
}
"#,
        &["E10406"],
        &["module-level 'g' needs no capture"],
    );
    assert_rejected(
        "module-const-entry",
        r#"module spec.ci.impl.moduleconst;

const base: i32 = 7;

func main(): i32 {
    return ([base] () => base)();
}
"#,
        &["E10406"],
        &["module-level 'base' needs no capture"],
    );
    assert_rejected(
        "module-func-entry",
        r#"module spec.ci.impl.modulefunc;

func helper(): i32 {
    return 4;
}

func main(): i32 {
    return ([helper] () => helper())();
}
"#,
        &["E10406"],
        &["module-level 'helper' needs no capture"],
    );
    assert_rejected(
        "module-type-entry",
        r#"module spec.ci.impl.moduletype;

struct Point {
    x: i32;
}

func main(): i32 {
    return ([Point] () => 1)();
}
"#,
        &["E10406"],
        &["module-level 'Point' needs no capture"],
    );
    assert_rejected(
        "import-entry",
        r#"module spec.ci.impl.importentry;

import { stringOf } from std.text;

func main(): i32 {
    return ([stringOf] () => 0)();
}
"#,
        &["E10406"],
        &["module-level 'stringOf' needs no capture"],
    );
    assert_rejected(
        "dotted-write",
        r#"module spec.ci.impl.dottedwrite;

struct P {
    x: i32;
}

func main(): i32 {
    let p: P = P { x: 1 };
    ([p] () => { p.x = 2; })();
    return p.x;
}
"#,
        &["E10402"],
        &["cannot write captured 'p'"],
    );
    assert_rejected(
        "dotted-move",
        r#"module spec.ci.impl.dottedmove;

import { String, stringOf } from std.text;

struct P {
    v: String;
}

func eat(take s: String): i32 {
    return 0;
}

func main(): i32 {
    let p: P = P { v: stringOf("ab") };
    return ([p] () => eat(take p.v))();
}
"#,
        &["E10401"],
        &["cannot move captured 'p'"],
    );
}

/// A deep relay reports exactly once, and the fully listed deep relay runs.
///
/// The three-level offending shape reports one `E10405` for the innermost
/// entry — its enclosing lambda cannot carry the name — while the same shape
/// with every level listed builds and runs, so the accepted relay keeps its
/// behavior at depth.
#[test]
fn should_report_a_deep_relay_once_and_run_the_listed_relay() {
    assert_rejected(
        "deep-relay-missing",
        r#"module spec.ci.impl.deeprelaymissing;

func main(): i32 {
    let a: i32 = 1;
    let b: i32 = 2;
    let m: i32 = 3;
    return ([a, b] () => ([b] () => ([m] () => m)())())();
}
"#,
        &["E10405"],
        &["'m'"],
    );
    let listed = r#"module spec.ci.impl.deeprelaylisted;

func main(): i32 {
    let a: i32 = 1;
    let b: i32 = 2;
    let m: i32 = 3;
    return ([a, b, m] () => ([b, m] () => ([m] () => m)())())();
}
"#;
    let output = run_host("deep-relay-listed", listed);
    assert_eq!(
        output.status.code(),
        Some(3),
        "deep-relay-listed: expected exit 3; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The accepted relay re-emits byte-identically.
#[test]
fn should_reemit_the_listed_relay_byte_identically() {
    let source = r#"module spec.ci.impl.relaylisted;

func main(): i32 {
    let n: i32 = 2;
    let m: i32 = 3;
    return ([n, m] () => ([m] () => m)())();
}
"#;
    let (first, second) = emit_twice("reemission-listed-relay", source);
    assert_eq!(
        first, second,
        "reemission-listed-relay: the two emissions differ"
    );
}
