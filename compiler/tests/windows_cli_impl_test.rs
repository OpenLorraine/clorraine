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

// --- The driver's host-family rules -----------------------------------------

use std::ffi::{OsStr, OsString};

use clc::clorraine::launcher_for;
use clc::clorraine::platform::{
    config_file, display_path, executable_name, pe_file_version, shell_word, HostOs,
};

/// A minimal PE32+ image with a fixed version block at `info_at`.
fn pe_with_info(info_at: usize, structure_version: u32) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x400];
    bytes[0..2].copy_from_slice(b"MZ");
    bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
    let info = [0xFEEF_04BDu32, structure_version, (3 << 16) | 2, 35 << 16];
    for (index, word) in info.iter().enumerate() {
        let at = info_at + index * 4;
        bytes[at..at + 4].copy_from_slice(&word.to_le_bytes());
    }
    bytes
}

#[test]
fn should_find_no_version_when_the_pe_header_offset_points_past_the_end_or_overflows() {
    for offset in [0x3fe_u32, 0x400, u32::MAX] {
        let mut bytes = pe_with_info(0x200, 0x0001_0000);
        bytes[0x3c..0x40].copy_from_slice(&offset.to_le_bytes());
        assert_eq!(pe_file_version(&bytes), None, "offset {offset:#x}");
    }
    assert_eq!(pe_file_version(b"MZ"), None);
    assert_eq!(pe_file_version(&[]), None);
}

#[test]
fn should_ignore_a_version_block_that_is_unaligned_has_another_structure_version_or_is_cut_off() {
    assert_eq!(pe_file_version(&pe_with_info(0x202, 0x0001_0000)), None);
    assert_eq!(pe_file_version(&pe_with_info(0x200, 0x0002_0000)), None);
    // The block needs all 52 bytes; one that starts 16 bytes before the end
    // of the file is not read.
    let mut short = pe_with_info(0x3f0, 0x0001_0000);
    short.truncate(0x400);
    assert_eq!(pe_file_version(&short), None);
    assert_eq!(pe_file_version(&pe_with_info(0x200, 0x0001_0000)), Some((3, 2, 35)));
}

#[test]
fn should_keep_windows_paths_without_a_drive_verbatim_prefix_and_plain_paths_unchanged() {
    let volume = r"\\?\Volume{0b1c}\x";
    assert_eq!(display_path(HostOs::Windows, volume), volume);
    assert_eq!(display_path(HostOs::Windows, r"\\?\d:\x"), r"d:\x");
    assert_eq!(display_path(HostOs::Windows, r"C:\x"), r"C:\x");
    assert_eq!(display_path(HostOs::Windows, r"\\srv\share"), r"\\srv\share");
}

#[test]
fn should_treat_any_case_of_the_exe_suffix_as_present() {
    assert_eq!(executable_name(HostOs::Windows, "fs-uae.Exe"), "fs-uae.Exe");
    assert_eq!(executable_name(HostOs::Windows, "exe"), "exe.exe");
    assert_eq!(executable_name(HostOs::Windows, ".exe"), ".exe");
}

#[test]
fn should_accept_a_unc_or_slash_terminated_appdata_and_reject_a_drive_without_a_separator() {
    let config = |value: &str| {
        let value = value.to_owned();
        config_file(HostOs::Windows, move |name| (name == "APPDATA").then(|| OsString::from(&value)))
    };
    assert_eq!(
        config(r"C:\Users\u\AppData\Roaming\").unwrap().to_str(),
        Some(r"C:\Users\u\AppData\Roaming\clorraine\config")
    );
    assert_eq!(
        config(r"\\srv\home\u").unwrap().to_str(),
        Some(r"\\srv\home\u\clorraine\config")
    );
    assert!(config("C:relative").is_err());
    assert!(config("").is_err());
}

#[test]
fn should_quote_an_empty_word_and_make_control_characters_visible() {
    assert_eq!(shell_word(HostOs::Unix, ""), "''");
    assert_eq!(shell_word(HostOs::Windows, ""), "\"\"");
    let shown = shell_word(HostOs::Windows, "a\u{1b}b");
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
}

#[test]
fn should_fall_back_to_the_bare_launcher_name_when_path_has_no_clorraine() {
    let scratch = ScratchDir::new("launcher-miss");
    assert_eq!(
        launcher_for(HostOs::Windows, OsStr::new("clorraine"), scratch.path().as_os_str()),
        "clorraine"
    );
    // A Unix file without the .exe suffix does not satisfy the Windows lookup.
    let plain = scratch.path().join("clorraine");
    std::fs::write(&plain, "").expect("write the file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    assert_eq!(
        launcher_for(HostOs::Windows, OsStr::new("clorraine"), scratch.path().as_os_str()),
        "clorraine"
    );
    assert_eq!(
        launcher_for(HostOs::Unix, OsStr::new("clorraine"), scratch.path().as_os_str()),
        plain.to_str().expect("UTF-8")
    );
}

#[test]
fn should_make_a_relative_launcher_path_absolute_without_resolving_it() {
    let launcher = launcher_for(HostOs::Unix, OsStr::new("./bin/clorraine"), OsStr::new(""));
    let cwd = std::env::current_dir().expect("cwd");
    assert_eq!(launcher, cwd.join("./bin/clorraine").to_str().expect("UTF-8"));
    let windows = launcher_for(HostOs::Windows, OsStr::new(r"bin\clorraine.exe"), OsStr::new(""));
    assert!(windows.ends_with(r"bin\clorraine.exe") && windows.len() > r"bin\clorraine.exe".len());
}
