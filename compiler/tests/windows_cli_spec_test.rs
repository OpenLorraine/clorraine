//! Windows CLI specification tests.
//!
//! These cases pin the behaviour the Windows command line depends on and that
//! can be proven on any host: source paths inside `#line` directives are valid
//! C string literals, the freestanding runtime only uses raw Linux system
//! calls on Linux, and the driver's Windows rules are pure functions that take
//! the host family as a parameter, so a Linux test run exercises the Windows
//! branches too.

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
            "clc-windows-cli-{}-{tag}-{sequence}",
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
fn run_clc(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// A complete program on one line, so its statements map to source line 1.
const ONE_LINE_PROGRAM: &str = "module demo.Paths; func main(): i32 { return 0; }\n";

/// Writes the one-line program under `file_name`, emits it for `profile`, and
/// returns the source path as passed to the compiler and the emitted C.
fn emit_named(scratch: &ScratchDir, file_name: &str, profile: &str) -> (String, String) {
    let source = scratch.path().join(file_name);
    std::fs::write(&source, ONE_LINE_PROGRAM).expect("write the source");
    let c_path = scratch.path().join("out.c");
    let source_text = source.to_str().expect("UTF-8 scratch path").to_owned();
    let output = run_clc(&[
        "--profile",
        profile,
        "-o",
        c_path.to_str().expect("UTF-8 scratch path"),
        &source_text,
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the compiler rejected {file_name:?}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).expect("read the emitted C");
    (source_text, emitted)
}

/// Builds host-profile C with the strict flags and asserts the build passes.
fn assert_strict_host_build(scratch: &ScratchDir, emitted: &str) {
    let c_path = scratch.path().join("strict.c");
    std::fs::write(&c_path, emitted).expect("write the C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-o"])
        .arg(&program)
        .arg(&c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

/// The `#line` directives of emitted C, in order.
fn line_directives(emitted: &str) -> Vec<&str> {
    emitted.lines().filter(|line| line.starts_with("#line ")).collect()
}

/// The source directory of a scratch file, as the compiler was given it.
fn directory_of(source: &str) -> &str {
    &source[..source.rfind('/').expect("an absolute scratch path")]
}

#[test]
fn should_escape_a_backslash_in_the_line_directive_path_when_the_file_name_contains_one() {
    let scratch = ScratchDir::new("backslash");
    let (source, emitted) = emit_named(&scratch, "back\\slash.clo", "host");
    let expected = format!("#line 1 \"{}/back\\\\slash.clo\"", directory_of(&source));
    assert!(
        line_directives(&emitted).contains(&expected.as_str()),
        "expected {expected:?} among the directives {:?}",
        line_directives(&emitted)
    );
    assert_strict_host_build(&scratch, &emitted);
}

#[test]
fn should_escape_a_double_quote_in_the_line_directive_path_when_the_file_name_contains_one() {
    let scratch = ScratchDir::new("quote");
    let (source, emitted) = emit_named(&scratch, "quo\"te.clo", "host");
    let expected = format!("#line 1 \"{}/quo\\\"te.clo\"", directory_of(&source));
    assert!(
        line_directives(&emitted).contains(&expected.as_str()),
        "expected {expected:?} among the directives {:?}",
        line_directives(&emitted)
    );
    assert_strict_host_build(&scratch, &emitted);
}

#[test]
fn should_write_a_tab_as_an_octal_escape_when_the_file_name_contains_one() {
    let scratch = ScratchDir::new("tab");
    let (source, emitted) = emit_named(&scratch, "tab\there.clo", "host");
    let expected = format!("#line 1 \"{}/tab\\011here.clo\"", directory_of(&source));
    assert!(
        line_directives(&emitted).contains(&expected.as_str()),
        "expected {expected:?} among the directives {:?}",
        line_directives(&emitted)
    );
    assert_strict_host_build(&scratch, &emitted);
}

#[test]
fn should_keep_every_line_directive_unchanged_when_the_path_has_no_special_characters() {
    // A plain path is written exactly as the compiler was given it, so the
    // emitted directives are the same bytes they were before escaping existed.
    let hello = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/hello/hello.clo");
    let hello_text = hello.to_str().expect("UTF-8 repository path");
    assert!(
        !hello_text.contains(['\\', '"']) && hello_text.bytes().all(|b| b >= 0x20 && b != 0x7f),
        "the repository path must be plain for this case: {hello_text:?}"
    );
    let scratch = ScratchDir::new("plain");
    let c_path = scratch.path().join("hello.c");
    let output = run_clc(&[
        "--profile",
        "host",
        "-o",
        c_path.to_str().expect("UTF-8 scratch path"),
        hello_text,
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    let emitted = std::fs::read_to_string(&c_path).expect("read the emitted C");
    let directives = line_directives(&emitted);
    assert!(!directives.is_empty(), "hello emits no #line directives");
    let mut names_hello = false;
    for directive in directives {
        let rest = directive.strip_prefix("#line ").expect("a #line directive");
        let (number, quoted) = rest.split_once(' ').expect("a line number and a path");
        assert!(number.bytes().all(|b| b.is_ascii_digit()), "{directive:?}");
        let path = quoted
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or_else(|| panic!("an unquoted path in {directive:?}"));
        // Hello's own lines name the path as given; the imported standard
        // library keeps its plain logical path. Neither gains an escape.
        assert!(
            path == hello_text || (path.starts_with("stdlib/std.") && path.ends_with(".clo")),
            "{directive:?}"
        );
        assert!(!path.contains(['\\', '"']), "{directive:?}");
        names_hello |= path == hello_text;
    }
    assert!(names_hello, "no directive names hello.clo itself");
}

/// Emits a small freestanding program and returns its C text.
fn emit_freestanding(scratch: &ScratchDir) -> PathBuf {
    let source = scratch.path().join("main.clo");
    std::fs::write(&source, ONE_LINE_PROGRAM).expect("write the source");
    let c_path = scratch.path().join("free.c");
    let output = run_clc(&[
        "--profile",
        "freestanding",
        "-o",
        c_path.to_str().expect("UTF-8 scratch path"),
        source.to_str().expect("UTF-8 scratch path"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    c_path
}

/// Preprocesses freestanding C as the host compiler would see it with the
/// given extra macro flags, and returns the preprocessed text.
fn preprocess(c_path: &Path, flags: &[&str]) -> String {
    let output = Command::new("cc")
        .args(["-std=c99", "-ffreestanding", "-E", "-P"])
        .args(flags)
        .arg(c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).expect("UTF-8 preprocessor output")
}

/// Macros that tell C code it is being compiled for Linux.
const NOT_LINUX: [&str; 4] = ["-U__linux__", "-U__linux", "-Ulinux", "-U__gnu_linux__"];

/// The raw system-call instruction the shim emits for this host's CPU.
const SYSCALL_MARKER: &str = if cfg!(target_arch = "aarch64") { "\"svc #0\"" } else { "\"syscall\"" };

#[test]
fn should_use_raw_system_calls_only_on_linux_when_building_the_freestanding_runtime() {
    let scratch = ScratchDir::new("free-syscalls");
    let c_path = emit_freestanding(&scratch);

    let linux = preprocess(&c_path, &[]);
    assert!(linux.contains(SYSCALL_MARKER), "Linux keeps the raw system calls");

    let mut windows = NOT_LINUX.to_vec();
    windows.push("-D_WIN64");
    let other = preprocess(&c_path, &windows);
    assert!(
        !other.contains(SYSCALL_MARKER),
        "a non-Linux host must take the C-runtime fallback, not raw Linux system calls"
    );
    let plain_other = preprocess(&c_path, &NOT_LINUX);
    assert!(!plain_other.contains(SYSCALL_MARKER), "any non-Linux host takes the fallback");
}

#[test]
fn should_make_the_pointer_sized_integer_64_bit_only_under_win64_when_building_freestanding() {
    let scratch = ScratchDir::new("free-uintptr");
    let c_path = emit_freestanding(&scratch);

    let linux = preprocess(&c_path, &[]);
    assert!(linux.contains("typedef unsigned long uintptr_t;"), "Linux keeps unsigned long");
    assert!(!linux.contains("typedef unsigned long long uintptr_t;"));

    let mut windows = NOT_LINUX.to_vec();
    windows.push("-D_WIN64");
    let win64 = preprocess(&c_path, &windows);
    assert!(
        win64.contains("typedef unsigned long long uintptr_t;"),
        "64-bit Windows has a 32-bit long, so the pointer-sized integer is unsigned long long"
    );
    assert!(!win64.contains("typedef unsigned long uintptr_t;"));
}

#[test]
fn should_select_native_system_calls_only_for_linux_x86_64_and_aarch64() {
    use clc::clorraine::native_syscalls_for;
    assert!(native_syscalls_for("linux", "x86_64"));
    assert!(native_syscalls_for("linux", "aarch64"));
    assert!(!native_syscalls_for("windows", "x86_64"));
    assert!(!native_syscalls_for("macos", "aarch64"));
    assert!(!native_syscalls_for("linux", "m68k"));
}
