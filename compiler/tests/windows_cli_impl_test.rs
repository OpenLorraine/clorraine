//! Windows CLI implementation tests.
//!
//! Edge cases behind the Windows specification: the escaping of every byte
//! class a source path can carry into a `#line` directive, the boundaries of
//! the native-syscall predicate, and a MinGW build of freestanding C that
//! uses `offsetof`, the one shim macro that converts a pointer to an integer.

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
            "clc-windows-cli-impl-{}-{tag}-{sequence}",
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

/// Emits the one-line program saved as `file_name` and returns the quoted
/// path of its first `#line 1` directive, without the quotes, after checking
/// that the C builds with the strict host flags.
fn line_one_path(tag: &str, file_name: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source = scratch.path().join(file_name);
    std::fs::write(&source, ONE_LINE_PROGRAM).expect("write the source");
    let c_path = scratch.path().join("out.c");
    let source_text = source.to_str().expect("UTF-8 scratch path").to_owned();
    let output = run_clc(&["--profile", "host", "-o", c_path.to_str().expect("UTF-8"), &source_text]);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    let emitted = std::fs::read_to_string(&c_path).expect("read the emitted C");
    let directive = emitted
        .lines()
        .find(|line| line.starts_with("#line 1 \""))
        .unwrap_or_else(|| panic!("{tag}: no #line 1 directive"))
        .to_owned();
    let built = Command::new("cc")
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-o"])
        .arg(scratch.path().join("prog"))
        .arg(&c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(built.status.success(), "{tag}: {}", String::from_utf8_lossy(&built.stderr));
    let dir = source_text[..source_text.rfind('/').expect("absolute")].to_owned();
    let body = directive["#line 1 \"".len()..directive.len() - 1].to_owned();
    (dir, body)
}

#[test]
fn should_octal_escape_every_control_byte_and_delete_in_a_line_directive_path() {
    let name: String = (1u8..0x20).chain([0x7f]).map(char::from).collect::<String>() + ".clo";
    let (dir, body) = line_one_path("controls", &name);
    let expected: String = (1u32..0x20).chain([0x7f]).map(|b| format!("\\{b:03o}")).collect();
    assert_eq!(body, format!("{dir}/{expected}.clo"));
}

#[test]
fn should_keep_a_digit_after_an_octal_escape_as_its_own_character() {
    // A C octal escape takes at most three digits, so `\0017` is byte 1 and '7'.
    let (dir, body) = line_one_path("octal-digit", "a\u{1}7.clo");
    assert_eq!(body, format!("{dir}/a\\0017.clo"));
}

#[test]
fn should_keep_spaces_and_utf8_letters_unescaped_in_a_line_directive_path() {
    let (dir, body) = line_one_path("utf8", "caf\u{e9} au lait.clo");
    assert_eq!(body, format!("{dir}/caf\u{e9} au lait.clo"));
}

#[test]
fn should_escape_backslashes_and_quotes_together_in_a_line_directive_path() {
    let (dir, body) = line_one_path("mixed", "a\\\"b\\\\.clo");
    assert_eq!(body, format!("{dir}/a\\\\\\\"b\\\\\\\\.clo"));
}

#[test]
fn should_break_every_trigraph_in_a_line_directive_path() {
    // The C compilers run with -std=c99, which turns trigraphs on; the strict
    // build inside the helper fails on any trigraph that survives.
    let (dir, body) = line_one_path("trigraph", "tri??=x.clo");
    assert_eq!(body, format!("{dir}/tri?\\?=x.clo"));
    let (dir, body) = line_one_path("trigraph-run", "a???=b??-c.clo");
    assert_eq!(body, format!("{dir}/a?\\?\\?=b?\\?-c.clo"));
}

#[test]
fn should_keep_a_single_question_mark_unescaped_in_a_line_directive_path() {
    let (dir, body) = line_one_path("question", "why?.clo");
    assert_eq!(body, format!("{dir}/why?.clo"));
}

#[test]
fn should_reject_native_system_calls_for_other_linux_architectures_and_other_systems() {
    use clc::clorraine::native_syscalls_for;
    for (os, arch) in [
        ("linux", "x86"),
        ("linux", "arm"),
        ("linux", "riscv64"),
        ("android", "aarch64"),
        ("freebsd", "x86_64"),
        ("macos", "x86_64"),
        ("", "x86_64"),
        ("Linux", "x86_64"),
    ] {
        assert!(!native_syscalls_for(os, arch), "{os}/{arch}");
    }
}

#[test]
fn should_build_freestanding_offsetof_without_narrowing_a_pointer_when_compiled_with_mingw() {
    if Command::new("x86_64-w64-mingw32-gcc").arg("--version").output().is_err() {
        println!("skipped: x86_64-w64-mingw32-gcc is not installed");
        return;
    }
    let scratch = ScratchDir::new("mingw-offsetof");
    let source = scratch.path().join("off.clo");
    std::fs::write(
        &source,
        "module demo.Off;\n\nstruct Pair { a: i32; b: u32; }\n\n\
         const OFF: usize = offsetof(Pair, b);\n\n\
         func main(): i32 {\n    if (OFF == 4) {\n        return 0;\n    }\n    return 1;\n}\n",
    )
    .expect("write the source");
    let c_path = scratch.path().join("off.c");
    let output = run_clc(&[
        "--profile",
        "freestanding",
        "-o",
        c_path.to_str().expect("UTF-8"),
        source.to_str().expect("UTF-8"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    // The driver's flags for a host without the raw syscall layer.
    let built = Command::new("x86_64-w64-mingw32-gcc")
        .args(["-std=c99", "-ffreestanding", "-fno-builtin", "-O0", "-g"])
        .args(["-Wall", "-Wextra", "-Werror", "-o"])
        .arg(scratch.path().join("off.exe"))
        .arg(&c_path)
        .output()
        .expect("failed to spawn MinGW");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
}
