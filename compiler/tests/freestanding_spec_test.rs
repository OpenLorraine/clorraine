//! Freestanding profile specification tests.
//!
//! The `freestanding` profile has no operating system to run on, so its proof
//! is split across two legs. The m68k leg is compiled and linked with no C
//! runtime (`-ffreestanding -nostdlib -lg`) and the case asserts the link
//! exits zero, which the linker refuses to do while any symbol is unresolved.
//! The host leg compiles the same translation unit with a raw-syscall layer
//! and runs it, so the program's stdout and exit code prove the semantics.
//!
//! The m68k toolchain lives only in the pinned SDK container, so a case that
//! needs it skips itself with a note on a host without it. The host cases run
//! anywhere a C compiler is present.

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
            "clc-freestanding-{}-{tag}-{sequence}",
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

/// Root of the frozen specification, resolved from the crate location.
fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec")
}

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Runs the built `clorraine` driver with `args` in `cwd`.
fn run_driver(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clorraine"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to spawn the clorraine driver")
}

/// True when `tool` can be spawned, so the m68k cases skip on a host without it.
fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// The first non-empty stderr line of a process result.
fn first_stderr_line(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_owned()
}

/// Writes `source` and emits it under the freestanding profile.
///
/// Returns the emitted C path and the compiler's process output; the caller
/// decides whether a rejection is expected.
fn emit_freestanding(scratch: &ScratchDir, source: &str) -> (PathBuf, Output) {
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the source snippet");
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "freestanding".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    (c_path, output)
}

/// Emits `source` and asserts the compiler accepted it with non-empty C.
fn emit_ok(tag: &str, scratch: &ScratchDir, source: &str) -> PathBuf {
    let (c_path, output) = emit_freestanding(scratch, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected a valid freestanding program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
    );
    c_path
}

/// Compiles emitted C on the host with the no-libc stance and returns the
/// build result. The flags mirror the profile's host run leg.
fn host_build(tag: &str, c_path: &Path, bin_path: &Path, release: bool) -> Output {
    let mut args = vec![
        "-std=c99".to_owned(),
        "-ffreestanding".to_owned(),
        "-fno-builtin".to_owned(),
        "-nostdlib".to_owned(),
    ];
    if release {
        args.push("-O2".to_owned());
    } else {
        args.extend(["-O0".to_owned(), "-g".to_owned()]);
    }
    args.extend([
        "-Wall".to_owned(),
        "-Wextra".to_owned(),
        "-Werror".to_owned(),
        "-o".to_owned(),
        bin_path.display().to_string(),
        c_path.display().to_string(),
    ]);
    Command::new("cc")
        .args(&args)
        .output()
        .unwrap_or_else(|err| panic!("{tag}: failed to spawn the host C compiler: {err}"))
}

/// Compiles and links the m68k artifact. Returns `None` when the toolchain is
/// absent, so a bare-host run reports a skip rather than a false failure.
fn m68k_build(tag: &str, c_path: &Path, bin_path: &Path, release: bool) -> Option<Output> {
    if !tool_available("m68k-amigaos-gcc") {
        println!("{tag}: skipped (m68k-amigaos-gcc is not installed)");
        return None;
    }
    let mut args = vec![
        "-std=c99".to_owned(),
        "-ffreestanding".to_owned(),
        "-fno-builtin".to_owned(),
        "-nostdlib".to_owned(),
        "-m68000".to_owned(),
        "-fwrapv".to_owned(),
    ];
    if release {
        args.push("-O2".to_owned());
    } else {
        args.extend(["-O0".to_owned(), "-g".to_owned()]);
    }
    args.extend([
        "-Wall".to_owned(),
        "-Wextra".to_owned(),
        "-Werror".to_owned(),
        "-o".to_owned(),
        bin_path.display().to_string(),
        c_path.display().to_string(),
        "-lg".to_owned(),
    ]);
    Some(
        Command::new("m68k-amigaos-gcc")
            .args(&args)
            .output()
            .unwrap_or_else(|err| panic!("{tag}: failed to spawn m68k-amigaos-gcc: {err}")),
    )
}

/// Emits `source`, builds the host no-libc artifact, and runs it.
fn emit_build_run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let c_path = emit_ok(tag, &scratch, source);
    let program = scratch.path().join("prog");
    let built = host_build(tag, &c_path, &program, false);
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    Command::new(&program)
        .output()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"))
}

/// One program that returns a fixed code, used by several cases.
const EXIT7_SOURCE: &str = r#"module spec.freestanding.exit7;

func main(): i32 {
    return 7;
}
"#;

/// A console program that prints one line.
const HELLO_SOURCE: &str = r#"module spec.freestanding.hello;

import { println } from std.io;

func main(): i32 {
    println("hello");
    return 0;
}
"#;

/// A float program whose result is six.
const FLOAT_SOURCE: &str = r#"module spec.freestanding.float;

func main(): i32 {
    let x: f64 = 1.5;
    return i32(x * 4.0);
}
"#;

/// ST-1: emitting for freestanding writes the runtime entry point and helpers.
#[test]
fn emitting_for_freestanding_embeds_the_runtime_entry_point() {
    let scratch = ScratchDir::new("entry");
    let c_path = emit_ok(
        "entry",
        &scratch,
        "module spec.freestanding.entry;\n\nfunc main(): i32 {\n    return 0;\n}\n",
    );
    let c = std::fs::read_to_string(&c_path).expect("read the emitted C");
    for needle in [
        "void _start(void)",
        "typedef unsigned int usize;",
        "cl_trap",
        "memcpy",
    ] {
        assert!(c.contains(needle), "the freestanding C must contain {needle:?}");
    }
}

/// ST-3..ST-6: a module outside the freestanding surface is a compile-time
/// E11200 naming the module and the profile.
#[test]
fn rejected_modules_report_e11200_on_freestanding() {
    let cases: [(&str, &str); 4] = [
        (
            "std.mem",
            "module spec.freestanding.reject_mem;\n\nimport { bufferNew } from std.mem;\n\nfunc main(): i32 {\n    return 0;\n}\n",
        ),
        (
            "std.text",
            "module spec.freestanding.reject_text;\n\nimport { String } from std.text;\n\nfunc main(): i32 {\n    return 0;\n}\n",
        ),
        (
            "std.fmt",
            "module spec.freestanding.reject_fmt;\n\nimport { toHex } from std.fmt;\n\nfunc main(): i32 {\n    return 0;\n}\n",
        ),
        (
            "std.amiga.dos",
            "module spec.freestanding.reject_amiga;\n\nimport { delayTicks } from std.amiga.dos;\n\nfunc main(): i32 {\n    return 0;\n}\n",
        ),
    ];
    for (module, source) in cases {
        let scratch = ScratchDir::new("reject");
        let (_, output) = emit_freestanding(&scratch, source);
        assert_eq!(
            output.status.code(),
            Some(1),
            "rejecting {module} must exit 1; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            first_stderr_line(&output),
            format!("error E11200: '{module}' is not available on profile 'freestanding'"),
            "rejecting {module} must name the module and the profile"
        );
    }
}

/// ST-7: floats are allowed on freestanding; the program emits with no E11200.
#[test]
fn float_types_are_allowed_on_freestanding() {
    let scratch = ScratchDir::new("float_emit");
    emit_ok("float_emit", &scratch, FLOAT_SOURCE);
}

/// A template placeholder lowers to the heap-backed formatter, which the
/// freestanding runtime does not provide, so it is rejected at compile time.
#[test]
fn a_template_placeholder_is_e11200_on_freestanding() {
    let source = r#"module spec.freestanding.template;

import { println } from std.io;

func main(): i32 {
    let n: i32 = 3;
    println($"n={n}");
    return 0;
}
"#;
    let scratch = ScratchDir::new("template");
    let (_, output) = emit_freestanding(&scratch, source);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a template placeholder must be rejected; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        first_stderr_line(&output),
        "error E11200: 'string formatting' is not available on profile 'freestanding'"
    );
}

/// The heap-backed `String` handle is not part of the freestanding runtime, so
/// a `String` type is rejected before it can emit undefined helpers.
#[test]
fn the_string_type_is_e11200_on_freestanding() {
    let source = r#"module spec.freestanding.string;

import { println } from std.io;

func show(s: String): void {
    println(s);
}

func main(): i32 {
    return 0;
}
"#;
    let scratch = ScratchDir::new("string");
    let (_, output) = emit_freestanding(&scratch, source);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a String type must be rejected; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        first_stderr_line(&output),
        "error E11200: 'String' is not available on profile 'freestanding'"
    );
}

/// ST-11: the m68k artifact compiles and links against the soft-float archive.
#[test]
fn the_m68k_artifact_links_with_the_soft_float_archive() {
    let scratch = ScratchDir::new("m68k_link");
    let c_path = emit_ok("m68k_link", &scratch, HELLO_SOURCE);
    let program = scratch.path().join("prog");
    let Some(built) = m68k_build("m68k_link", &c_path, &program, false) else {
        return;
    };
    assert!(
        built.status.success(),
        "m68k_link: the m68k link must resolve every symbol; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

/// ST-12: the same link at `-O2` still resolves the runtime's own memcpy.
#[test]
fn the_m68k_artifact_links_at_o2() {
    let scratch = ScratchDir::new("m68k_o2");
    let c_path = emit_ok("m68k_o2", &scratch, HELLO_SOURCE);
    let program = scratch.path().join("prog");
    let Some(built) = m68k_build("m68k_o2", &c_path, &program, true) else {
        return;
    };
    assert!(
        built.status.success(),
        "m68k_o2: the optimized m68k link must resolve every symbol; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

/// ST-14: a console program prints through the host no-libc run.
#[test]
fn println_runs_on_the_host_no_libc_leg() {
    let output = emit_build_run_host("print", HELLO_SOURCE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "print: a clean program exits zero; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}

/// ST-15: the program's own exit code is preserved through `_start`.
#[test]
fn the_program_exit_code_is_preserved_on_the_host_leg() {
    let output = emit_build_run_host("exit7", EXIT7_SOURCE);
    assert_eq!(output.status.code(), Some(7), "exit7: expected exit 7");
}

/// ST-16: by-value aggregate copies resolve the runtime's memcpy/memset.
#[test]
fn aggregate_copies_resolve_the_memory_helpers() {
    let source = r#"module spec.freestanding.aggregate;

struct Pair { a: i32; b: i32; }

func sumArray(xs: i32[3]): i32 {
    return xs[0] + xs[1] + xs[2];
}

func sumPair(p: Pair): i32 {
    return p.a + p.b;
}

func main(): i32 {
    let xs: i32[3] = [1, 2, 3];
    let p: Pair = Pair{ a: 40, b: 2 };
    return sumArray(xs) + sumPair(p);
}
"#;
    let output = emit_build_run_host("aggregate", source);
    assert_eq!(
        output.status.code(),
        Some(48),
        "aggregate: 6 from the array plus 42 from the struct"
    );
}

/// ST-17: the float program returns six through the host no-libc run.
#[test]
fn a_float_program_runs_on_the_host_no_libc_leg() {
    let output = emit_build_run_host("float_run", FLOAT_SOURCE);
    assert_eq!(output.status.code(), Some(6), "float_run: expected 6");
}

/// ST-18: the file functions compile but trap at runtime with the new code.
#[test]
fn file_functions_trap_with_e11214() {
    let source = r#"module spec.freestanding.file_trap;

import { File, openFile } from std.io;

func main(): i32 {
    let f: File? = openFile("x", 1005);
    if (f == null) {
        return 1;
    }
    return 0;
}
"#;
    let output = emit_build_run_host("file_trap", source);
    assert!(
        !output.status.success(),
        "file_trap: a file function must trap, not return"
    );
    assert_eq!(
        first_stderr_line(&output),
        "error E11214: file I/O is not available on the freestanding profile"
    );
}

/// ST-19: an existing runtime fault still traps through the freestanding shim.
#[test]
fn division_by_zero_traps_with_e11210_on_the_host_leg() {
    let source = r#"module spec.freestanding.divide;

func main(): i32 {
    let zero: i32 = 0;
    let n: i32 = 10 / zero;
    return n;
}
"#;
    let output = emit_build_run_host("divide", source);
    assert!(
        !output.status.success(),
        "divide: a runtime fault must exit nonzero"
    );
    assert_eq!(
        first_stderr_line(&output),
        "error E11210: integer division by zero"
    );
}

/// The console corpus program prints its recorded output through the host leg.
#[test]
fn the_console_corpus_matches_its_recorded_output() {
    let scratch = ScratchDir::new("corpus");
    let source_path = spec_root().join("tests/corpus/freestanding/program.clo");
    let source = std::fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", source_path.display()));
    let (c_path, output) = emit_freestanding(&scratch, &source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "corpus: the compile must succeed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let program = scratch.path().join("prog");
    let built = host_build("corpus", &c_path, &program, false);
    assert!(
        built.status.success(),
        "corpus: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(&program)
        .output()
        .expect("run the corpus program");
    assert_eq!(ran.status.code(), Some(0), "corpus: expected a clean exit");
    let expected_path = spec_root().join("tests/corpus/freestanding/expected-output.txt");
    let expected = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", expected_path.display()));
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        expected,
        "corpus: the output differs from the recorded output"
    );
}

/// ST-20: `clorraine build --profile freestanding` writes the m68k artifact.
#[test]
fn the_driver_builds_the_m68k_artifact() {
    if !tool_available("m68k-amigaos-gcc") {
        println!("driver_build: skipped (m68k-amigaos-gcc is not installed)");
        return;
    }
    let scratch = ScratchDir::new("driver_build");
    std::fs::write(scratch.path().join("main.clo"), HELLO_SOURCE).expect("write the source");
    let output = run_driver(
        scratch.path(),
        &["build", "--profile", "freestanding", "main.clo"],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "driver_build: build must succeed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact = scratch
        .path()
        .join("build/clorraine/main/freestanding/main");
    assert!(
        artifact.exists(),
        "driver_build: the m68k artifact must exist at {}",
        artifact.display()
    );
}

/// ST-21: `clorraine run --profile freestanding` runs the host no-libc artifact.
#[test]
fn the_driver_runs_the_host_no_libc_artifact() {
    if !tool_available("m68k-amigaos-gcc") {
        println!("driver_run: skipped (m68k-amigaos-gcc is not installed)");
        return;
    }
    let scratch = ScratchDir::new("driver_run");
    std::fs::write(scratch.path().join("main.clo"), HELLO_SOURCE).expect("write the source");
    let output = run_driver(
        scratch.path(),
        &["run", "--profile", "freestanding", "main.clo"],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "driver_run: run must exit with the program's code; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "hello\n",
        "driver_run: the program output must reach stdout"
    );
}

/// A local fixed array of views initializes and reads back on the host leg.
///
/// The `str[2]` local is a 16-byte-aligned value aggregate, so `_start` must
/// place `main` on the alignment the host ABI's call sequence expects; the
/// bytes read back through both views and the exit code carries their sum
/// (97 + 98). The m68k leg pins that the entry change stays clean for the
/// other target's placement (red at the phase base: the host no-libc build
/// passed and the run segfaulted before returning).
#[test]
fn should_run_a_local_fixed_array_of_views_on_the_host_leg() {
    let source = r#"module spec.freestanding.fsentry;

func main(): i32 {
    let a: str[2] = [; "ab"];
    return i32(a[0][0]) + i32(a[1][1]);
}
"#;
    let scratch = ScratchDir::new("fsentry");
    let c_path = emit_ok("fsentry", &scratch, source);
    let program = scratch.path().join("prog");
    let built = host_build("fsentry", &c_path, &program, false);
    assert!(
        built.status.success(),
        "fsentry: the host no-libc build must accept the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(&program)
        .output()
        .unwrap_or_else(|err| panic!("fsentry: cannot run the program: {err}"));
    assert_eq!(
        ran.status.code(),
        Some(195),
        "fsentry: the run must return the two bytes read back; stderr:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    let m68k_program = scratch.path().join("prog-m68k");
    let Some(built) = m68k_build("fsentry", &c_path, &m68k_program, false) else {
        return;
    };
    assert!(
        built.status.success(),
        "fsentry: the m68k link must stay clean; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}
